//! Command dispatch: envelope-shape validation, capability gating, and the
//! shared endpoint-operation executor behind both routes.
//!
//! Gates stay uniform and closed: unknown command names and failed
//! trusted-key presentation answer identically (`unknown_command`, so no
//! existence oracle exists), and every unknown, expired, or foreign token
//! answers `unknown_token`. Player commands never surface anything beyond
//! the frozen wire/service failure classes.

use crate::protocol::{self, decode_wire_bytes};
use crate::session::Session;
use mtgml_environment::{PlayerBoundaryError, PlayerEndpoint};
use mtgml_model::PlayerId;
use mtgml_random::RootSeed256;
use mtgml_wire::WireContract;
use serde::Serialize;
use serde_json::{json, Map, Value};

pub const TRUSTED_COMMANDS: &[&str] =
    &["reset_synthetic", "bind_player", "direct_call", "shutdown"];

pub const PLAYER_COMMANDS: &[&str] = &[
    "observation",
    "information_state",
    "visible_decision",
    "submit",
];

pub fn is_trusted_command(cmd: &str) -> bool {
    TRUSTED_COMMANDS.contains(&cmd)
}

/// What the session loop should do after one handled request.
pub enum Action {
    /// Emit the response line, then keep serving.
    Respond(String),
    /// Emit the response line, then end the process cleanly.
    Shutdown(String),
}

pub fn handle(session: &mut Session, raw: &Value) -> Action {
    let id = protocol::extract_id(raw);
    let Some(cmd) = raw.get("cmd").and_then(Value::as_str) else {
        return reject(id, protocol::EnvelopeErrorCode::InvalidParams);
    };
    if raw.get("v").and_then(Value::as_u64) != Some(protocol::PROTOCOL_VERSION) {
        return reject(id, protocol::EnvelopeErrorCode::InvalidParams);
    }
    let Some(params) = raw.get("params").and_then(Value::as_object) else {
        return reject(id, protocol::EnvelopeErrorCode::InvalidParams);
    };

    if is_trusted_command(cmd) {
        let presented = params.get("trusted_key").and_then(Value::as_str);
        if !session.authorize_trusted(presented) {
            return reject(id, protocol::EnvelopeErrorCode::UnknownCommand);
        }
        trusted_command(session, cmd, params, id)
    } else if PLAYER_COMMANDS.contains(&cmd) {
        let resolved = params
            .get("token")
            .and_then(Value::as_str)
            .and_then(|token| session.resolve_token(token));
        match resolved {
            Some(binding) => execute_op(binding.endpoint.as_ref(), cmd, params, id),
            None => reject(id, protocol::EnvelopeErrorCode::UnknownToken),
        }
    } else {
        reject(id, protocol::EnvelopeErrorCode::UnknownCommand)
    }
}

fn trusted_command(
    session: &mut Session,
    cmd: &str,
    params: &Map<String, Value>,
    id: Option<u64>,
) -> Action {
    match cmd {
        "reset_synthetic" => match parse_reset_params(params) {
            Some((players, root_seed)) => match session.reset_synthetic(players, root_seed) {
                Ok(()) => Action::Respond(protocol::ok_envelope(id, json!({}))),
                Err(_) => reject(id, protocol::EnvelopeErrorCode::ServiceUnavailable),
            },
            None => reject(id, protocol::EnvelopeErrorCode::InvalidParams),
        },
        "bind_player" => match param_player(params, "player") {
            Some(player) => match session.bind_player(player) {
                Ok(token) => Action::Respond(protocol::ok_envelope(id, json!({"token": token}))),
                Err(_) => reject(id, protocol::EnvelopeErrorCode::ServiceUnavailable),
            },
            None => reject(id, protocol::EnvelopeErrorCode::InvalidParams),
        },
        "direct_call" => {
            let op = params
                .get("op")
                .and_then(Value::as_str)
                .filter(|op| PLAYER_COMMANDS.contains(op));
            match (op, param_player(params, "player")) {
                (Some(op), Some(player)) => match session.route(player) {
                    Some(endpoint) => execute_op(endpoint, op, params, id),
                    None => reject(id, protocol::EnvelopeErrorCode::ServiceUnavailable),
                },
                _ => reject(id, protocol::EnvelopeErrorCode::InvalidParams),
            }
        }
        "shutdown" => Action::Shutdown(protocol::ok_envelope(id, json!({}))),
        _ => reject(id, protocol::EnvelopeErrorCode::UnknownCommand),
    }
}

/// The single executor behind both routes: token-scoped player commands
/// and trusted direct calls invoke the identical endpoint operations on
/// the resolved [`PlayerEndpoint`].
pub fn execute_op(
    endpoint: &dyn PlayerEndpoint,
    op: &str,
    params: &Map<String, Value>,
    id: Option<u64>,
) -> Action {
    match op {
        "observation" => match endpoint.observation() {
            Ok(observation) => payload_action(id, "observation_wire_b64", &observation),
            Err(_) => reject(id, protocol::EnvelopeErrorCode::ServiceUnavailable),
        },
        "information_state" => match endpoint.information_state() {
            Ok(information_state) => {
                payload_action(id, "information_state_wire_b64", &information_state)
            }
            Err(_) => reject(id, protocol::EnvelopeErrorCode::ServiceUnavailable),
        },
        "visible_decision" => match endpoint.visible_decision() {
            Ok(Some(request)) => payload_action(id, "visible_decision_wire_b64", &request),
            Ok(None) => Action::Respond(protocol::ok_envelope(
                id,
                json!({"visible_decision_wire_b64": null}),
            )),
            Err(_) => reject(id, protocol::EnvelopeErrorCode::ServiceUnavailable),
        },
        "submit" => match decode_wire_bytes(params, "response_wire_b64") {
            None => reject(id, protocol::EnvelopeErrorCode::InvalidParams),
            Some(response_bytes) => {
                match mtgml_environment::submit_response_bytes(endpoint, &response_bytes) {
                    Ok(step) => payload_action(id, "step_wire_b64", &step),
                    Err(PlayerBoundaryError::Wire(_)) => {
                        reject(id, protocol::EnvelopeErrorCode::MalformedResponse)
                    }
                    Err(PlayerBoundaryError::Service(_)) => {
                        reject(id, protocol::EnvelopeErrorCode::ServiceUnavailable)
                    }
                }
            }
        },
        _ => reject(id, protocol::EnvelopeErrorCode::UnknownCommand),
    }
}

fn payload_action<T>(id: Option<u64>, field: &str, value: &T) -> Action
where
    T: Serialize + WireContract,
{
    match protocol::encode_payload(value) {
        Some(payload) => Action::Respond(protocol::ok_envelope(id, json!({ (field): payload }))),
        None => reject(id, protocol::EnvelopeErrorCode::ServiceUnavailable),
    }
}

fn parse_reset_params(params: &Map<String, Value>) -> Option<([PlayerId; 2], RootSeed256)> {
    let players = params.get("players")?.as_array()?;
    if players.len() != 2 {
        return None;
    }
    let first = player_value(players.first()?);
    let second = player_value(players.get(1)?);
    let root_seed = RootSeed256::from_lower_hex(params.get("root_seed_hex")?.as_str()?).ok()?;
    match (first, second) {
        (Some(first), Some(second)) if first != second => Some(([first, second], root_seed)),
        _ => None,
    }
}

fn param_player(params: &Map<String, Value>, key: &str) -> Option<PlayerId> {
    player_value(params.get(key)?)
}

fn player_value(value: &Value) -> Option<PlayerId> {
    let raw = value.as_str()?.parse::<u32>().ok()?;
    Some(PlayerId(u64::from(raw)))
}

fn reject(id: Option<u64>, code: protocol::EnvelopeErrorCode) -> Action {
    Action::Respond(protocol::error_envelope(id, code))
}
