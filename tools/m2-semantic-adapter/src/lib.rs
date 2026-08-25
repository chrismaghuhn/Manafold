//! Temporary M2.H semantic adapter: a JSON Lines subprocess shell around
//! the authoritative player boundary.
//!
//! The adapter owns routing state only (the token registry and
//! bound-handle routes). It forwards exact canonical DTO bytes between a
//! test orchestrator and the real player endpoints; it never pre-validates
//! or synthesizes semantic payloads — submission bytes reach
//! [`mtgml_environment::submit_response_bytes`] raw. This is unpublished
//! test infrastructure (`publish = false`), never a production transport;
//! OD-009 remains open.
//!
//! Process model: one process serves one live synthetic environment at a
//! time (sequential re-resets allowed); stdin EOF or the trusted `shutdown`
//! command ends it cleanly. stdout carries protocol lines only; stderr
//! carries trusted diagnostics only.

pub mod config;
pub mod handlers;
pub mod protocol;
pub mod session;
pub mod tokens;

#[cfg(test)]
mod evidence_tests;

use handlers::{is_trusted_command, Action};
use protocol::EnvelopeErrorCode;
use protocol::FramedLine;
use session::Session;
use std::any::Any;
use std::io::{BufRead, Write};
use std::panic::{self, AssertUnwindSafe};

/// The closed panic-classification policy for one serviced command whose
/// handling panicked (§D failure policy): a panicked PLAYER command maps
/// best-effort to the frozen layer-C `service_unavailable` surface; a
/// panicked TRUSTED command maps to the adapter-internal `internal_error`
/// class. The exit intent is always fatal termination. Deliberately pure
/// so the policy is directly testable without inducing endpoint panics,
/// and structurally incapable of carrying panic detail: the emitted line
/// is exactly the closed error envelope.
fn panic_envelope_and_exit(trusted: bool, id: Option<u64>) -> (String, i32) {
    let code = if trusted {
        EnvelopeErrorCode::InternalError
    } else {
        EnvelopeErrorCode::ServiceUnavailable
    };
    (protocol::error_envelope(id, code), protocol::EXIT_FATAL)
}

/// Serves one adapter session to completion and returns the process exit
/// code: 0 for clean EOF/shutdown termination, nonzero fail-closed
/// otherwise. A panic while servicing any command terminates the loop; the
/// frozen error surface (internal_error for trusted commands,
/// service_unavailable for everything else) is emitted best-effort first.
pub fn run(input: &mut dyn BufRead, output: &mut dyn Write, trusted_key: Option<String>) -> i32 {
    let mut session = Session::new(trusted_key);
    loop {
        match protocol::read_line_capped(input) {
            Ok(FramedLine::Eof) => return protocol::EXIT_OK,
            Ok(FramedLine::Oversized) => {
                let response = protocol::error_envelope(None, EnvelopeErrorCode::OversizedInput);
                let _ = emit(output, &response);
                return protocol::EXIT_FATAL;
            }
            Ok(FramedLine::Line(line)) => {
                let raw: serde_json::Value = match serde_json::from_slice(&line) {
                    Ok(raw) => raw,
                    Err(_) => {
                        let response =
                            protocol::error_envelope(None, EnvelopeErrorCode::ParseError);
                        if emit(output, &response).is_err() {
                            return protocol::EXIT_FATAL;
                        }
                        continue;
                    }
                };
                let id = protocol::extract_id(&raw);
                let trusted = raw
                    .get("cmd")
                    .and_then(serde_json::Value::as_str)
                    .map(is_trusted_command)
                    .unwrap_or(false);
                let outcome =
                    panic::catch_unwind(AssertUnwindSafe(|| handlers::handle(&mut session, &raw)));
                match outcome {
                    Ok(Action::Respond(response)) => {
                        if emit(output, &response).is_err() {
                            return protocol::EXIT_FATAL;
                        }
                    }
                    // Shutdown-ack emission is deliberately best-effort; all other fatal paths return EXIT_FATAL.
                    Ok(Action::Shutdown(response)) => {
                        let _ = emit(output, &response);
                        return protocol::EXIT_OK;
                    }
                    Err(payload) => {
                        report_panic(&payload);
                        let (response, exit_code) = panic_envelope_and_exit(trusted, id);
                        // Best-effort emission: a failed emit cannot change
                        // the outcome, so termination is unconditional.
                        let _ = emit(output, &response);
                        return exit_code;
                    }
                }
            }
            Err(_) => return protocol::EXIT_FATAL,
        }
    }
}

fn emit(output: &mut dyn Write, line: &str) -> std::io::Result<()> {
    output.write_all(line.as_bytes())?;
    output.write_all(b"\n")?;
    output.flush()
}

fn report_panic(payload: &Box<dyn Any + Send>) {
    let detail = payload
        .downcast_ref::<&str>()
        .map(|detail| (*detail).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned());
    match detail {
        Some(detail) => eprintln!("m2-semantic-adapter: handler panicked: {detail}"),
        None => eprintln!("m2-semantic-adapter: handler panicked"),
    }
}

#[cfg(test)]
mod smoke_tests {
    use super::run;
    use crate::handlers::{self, Action};
    use crate::protocol;
    use crate::session::Session;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use mtgml_decision::PlayerDecisionRequestV2;
    use mtgml_model::PlayerId;
    use mtgml_observation::ObservationEnvelope;
    use mtgml_wire::decode_canonical;
    use serde_json::{json, Value};
    use std::io::Cursor;

    const TRUSTED_KEY: &str = "smoke-trusted-key";
    const ROOT_SEED_HEX: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

    fn request(id: u64, cmd: &str, params: Value) -> Value {
        json!({"v": 1, "id": id, "cmd": cmd, "params": params})
    }

    fn request_line(id: u64, cmd: &str, params: Value) -> String {
        request(id, cmd, params).to_string()
    }

    fn expect_respond(action: Action) -> Value {
        match action {
            Action::Respond(line) | Action::Shutdown(line) => serde_json::from_str(&line).unwrap(),
        }
    }

    fn error_code(response: &Value) -> String {
        response["error"]["code"].as_str().unwrap().to_string()
    }

    #[test]
    fn framing_reset_and_shutdown_round_trip() {
        let script = format!(
            "not json\n{}\n{}\n",
            request_line(
                1,
                "reset_synthetic",
                json!({
                    "trusted_key": TRUSTED_KEY,
                    "players": ["1", "2"],
                    "root_seed_hex": ROOT_SEED_HEX,
                })
            ),
            request_line(2, "shutdown", json!({"trusted_key": TRUSTED_KEY}),),
        );
        let mut input = Cursor::new(script);
        let mut output = Vec::new();
        let code = run(&mut input, &mut output, Some(TRUSTED_KEY.to_string()));
        assert_eq!(code, protocol::EXIT_OK);
        let text = String::from_utf8(output).unwrap();
        let lines: Vec<Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(error_code(&lines[0]), "parse_error");
        assert_eq!(lines[1]["id"], 1);
        assert_eq!(lines[1]["ok"], true);
        assert_eq!(lines[1]["result"], json!({}));
        assert_eq!(lines[2]["id"], 2);
        assert_eq!(lines[2]["ok"], true);
    }

    #[test]
    fn oversized_line_fails_closed() {
        let script = format!("\"{}\"", "x".repeat(protocol::MAX_INPUT_LINE_BYTES + 1));
        let mut input = Cursor::new(script);
        let mut output = Vec::new();
        let code = run(&mut input, &mut output, Some(TRUSTED_KEY.to_string()));
        assert_eq!(code, protocol::EXIT_FATAL);
        let text = String::from_utf8(output).unwrap();
        let response: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(error_code(&response), "oversized_input");
    }

    #[test]
    fn reset_bind_and_player_views() {
        let mut session = Session::new(Some(TRUSTED_KEY.to_string()));

        let foreign_key = json!({
            "trusted_key": "wrong-key",
            "players": ["1", "2"],
            "root_seed_hex": ROOT_SEED_HEX,
        });
        let response = expect_respond(handlers::handle(
            &mut session,
            &request(1, "reset_synthetic", foreign_key),
        ));
        assert_eq!(error_code(&response), "unknown_command");

        let reset = json!({
            "trusted_key": TRUSTED_KEY,
            "players": ["1", "2"],
            "root_seed_hex": ROOT_SEED_HEX,
        });
        let response = expect_respond(handlers::handle(
            &mut session,
            &request(2, "reset_synthetic", reset),
        ));
        assert_eq!(response["ok"], true);

        let token_one = bind(&mut session, 3, "1");
        let token_two = bind(&mut session, 4, "2");
        assert_ne!(token_one, token_two);
        for token in [&token_one, &token_two] {
            assert_eq!(token.len(), 32);
            assert!(
                token
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
                "tokens are lowercase hex"
            );
        }

        observation(&mut session, 5, &token_one, PlayerId(1));
        let observation_two = observation(&mut session, 6, &token_two, PlayerId(2));

        let direct_route = expect_respond(handlers::handle(
            &mut session,
            &request(
                9,
                "direct_call",
                json!({
                    "trusted_key": TRUSTED_KEY,
                    "op": "observation",
                    "player": "2",
                }),
            ),
        ));
        assert_eq!(direct_route["result"], observation_two);

        let visible_request = expect_respond(handlers::handle(
            &mut session,
            &request(10, "visible_decision", json!({"token": token_one})),
        ));
        let encoded = visible_request["result"]["visible_decision_wire_b64"]
            .as_str()
            .unwrap();
        let bytes = STANDARD.decode(encoded).unwrap();
        let decoded: PlayerDecisionRequestV2 = decode_canonical(&bytes).unwrap();

        let other_visible = expect_respond(handlers::handle(
            &mut session,
            &request(11, "visible_decision", json!({"token": token_two})),
        ));
        assert!(other_visible["result"]["visible_decision_wire_b64"].is_null());

        assert_eq!(decoded.schema_version, "player-decision-request.v2");

        let reset_again = json!({
            "trusted_key": TRUSTED_KEY,
            "players": ["1", "2"],
            "root_seed_hex": ROOT_SEED_HEX,
        });
        let response = expect_respond(handlers::handle(
            &mut session,
            &request(12, "reset_synthetic", reset_again),
        ));
        assert_eq!(response["ok"], true);
        let response = expect_respond(handlers::handle(
            &mut session,
            &request(13, "information_state", json!({"token": token_two})),
        ));
        assert_eq!(error_code(&response), "unknown_token");

        let shutdown = expect_respond(handlers::handle(
            &mut session,
            &request(14, "shutdown", json!({"trusted_key": TRUSTED_KEY})),
        ));
        assert_eq!(shutdown["ok"], true);
    }

    fn bind(session: &mut Session, id: u64, player: &str) -> String {
        let response = expect_respond(handlers::handle(
            session,
            &request(
                id,
                "bind_player",
                json!({"trusted_key": TRUSTED_KEY, "player": player}),
            ),
        ));
        assert_eq!(response["ok"], true);
        response["result"]["token"].as_str().unwrap().to_string()
    }

    fn observation(session: &mut Session, id: u64, token: &str, expected: PlayerId) -> Value {
        let response = expect_respond(handlers::handle(
            session,
            &request(id, "observation", json!({ "token": token })),
        ));
        assert_eq!(response["ok"], true);
        let encoded = response["result"]["observation_wire_b64"].as_str().unwrap();
        let bytes = STANDARD.decode(encoded).unwrap();
        let envelope: ObservationEnvelope = serde_json::from_slice(&bytes).unwrap();
        envelope.validate().unwrap();
        assert_eq!(envelope.perspective, expected);
        response["result"].clone()
    }
}
