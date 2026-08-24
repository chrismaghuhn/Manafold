//! H.2-ii adapter evidence suite.
//!
//! Everything here drives the REAL synthetic environment through the
//! session/handler layer (JSON requests, never raw endpoint calls for the
//! property under test) and proves, below the JSONL envelope:
//!
//! - the single dynamic counter `endpoint_submit_calls` via a counting
//!   wrapper around the real handle (malformed = 0; every typed submission
//!   including wrong-actor = exactly 1);
//! - malformed raw-byte classes fail closed at layer A with a healthy,
//!   reusable session afterwards;
//! - handler transparency: handler-emitted payload bytes are exactly
//!   `encode_canonical` of what the endpoint produced for every operation,
//!   with accepted states never consumed twice (single-shot spy);
//! - the closed panic-classification policy and its redaction;
//! - controlled service-failure redaction with continued service;
//! - base64 fidelity of the protocol helpers;
//! - promoted session/protocol negatives (EOF after shutdown, uniform
//!   unknown-token shape, token non-echo);
//! - in-crate guards: the frozen direct-dependency allowlist and the
//!   tool-source privileged-operation scan.

use crate::handlers::{self, Action};
use crate::protocol;
use crate::session::Session;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_decision::{
    DecisionAnswerV2, DecisionResponseV2, PlayerDecisionRequestV2, DECISION_RESPONSE_V2_SCHEMA,
};
use mtgml_environment::{PlayerEndpoint, PlayerEndpointError, PlayerEndpointHandle};
use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, PlayerId, StateRevision};
use mtgml_observation::{
    ObservationEnvelope, PlayerInformationStateV2, PlayerStepSubmissionV1, PlayerStepV2,
    PlayerSubmissionCodeV1,
};
use mtgml_wire::{decode_canonical, encode_canonical};
use serde_json::{json, Map, Value};
use std::io::Cursor;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const TRUSTED_KEY: &str = "evidence-suite-trusted-key";
/// Distinct from the smoke-test seed on purpose: any accidental coupling
/// between suites would show up as cross-test drift instead of hiding.
const ROOT_SEED_HEX: &str = "0aa90aa90aa90aa90aa90aa90aa90aa90aa90aa90aa90aa90aa90aa90aa90aa9";

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

// ---------------------------------------------------------------------------
// Request/response helpers
// ---------------------------------------------------------------------------

fn request(id: Option<u64>, cmd: &str, params: Value) -> Value {
    let mut object = Map::new();
    object.insert("v".into(), json!(protocol::PROTOCOL_VERSION));
    if let Some(id) = id {
        object.insert("id".into(), json!(id));
    }
    object.insert("cmd".into(), json!(cmd));
    object.insert("params".into(), params);
    Value::Object(object)
}

/// Drives one request through the handler and returns both the exact
/// emitted line and its parsed envelope. The raw line is what redaction
/// scans observe — parsing alone could hide detail smuggled into unused
/// fields.
fn handle_line(
    session: &mut Session,
    id: Option<u64>,
    cmd: &str,
    params: Value,
) -> (String, Value) {
    let line = match handlers::handle(session, &request(id, cmd, params)) {
        Action::Respond(line) | Action::Shutdown(line) => line,
    };
    let parsed: Value = serde_json::from_str(&line).unwrap();
    (line, parsed)
}

fn handle(session: &mut Session, id: Option<u64>, cmd: &str, params: Value) -> Value {
    handle_line(session, id, cmd, params).1
}

fn error_code(response: &Value) -> &str {
    response["error"]["code"].as_str().unwrap()
}

fn b64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

fn unb64(encoded: &str) -> Vec<u8> {
    STANDARD.decode(encoded).unwrap()
}

fn reset_ok(session: &mut Session, id: u64) {
    let response = handle(
        session,
        Some(id),
        "reset_synthetic",
        json!({
            "trusted_key": TRUSTED_KEY,
            "players": ["1", "2"],
            "root_seed_hex": ROOT_SEED_HEX,
        }),
    );
    assert_eq!(response["ok"], true, "reset must succeed");
}

fn bind_plain(session: &mut Session, id: u64, player: &str) -> String {
    let response = handle(
        session,
        Some(id),
        "bind_player",
        json!({"trusted_key": TRUSTED_KEY, "player": player}),
    );
    assert_eq!(response["ok"], true, "binding {player} must succeed");
    response["result"]["token"].as_str().unwrap().to_string()
}

// ---------------------------------------------------------------------------
// Test-only endpoint wrappers
// ---------------------------------------------------------------------------

/// Counting decorator around a real handle (the established M2.G seam
/// pattern): forwards reads verbatim, counts every semantic submit it is
/// asked to perform, and captures the canonical bytes of the last step it
/// returned — the single dynamic counter `endpoint_submit_calls`.
struct SubmitSpy {
    inner: Arc<dyn PlayerEndpoint>,
    submits: AtomicUsize,
    last_step_canonical: Mutex<Option<Vec<u8>>>,
}

impl SubmitSpy {
    fn new(inner: Arc<dyn PlayerEndpoint>) -> Self {
        Self {
            inner,
            submits: AtomicUsize::new(0),
            last_step_canonical: Mutex::new(None),
        }
    }

    fn submit_calls(&self) -> usize {
        self.submits.load(Ordering::SeqCst)
    }

    fn captured_canonical(&self) -> Option<Vec<u8>> {
        self.last_step_canonical.lock().unwrap().clone()
    }
}

impl PlayerEndpoint for SubmitSpy {
    fn perspective(&self) -> PlayerId {
        self.inner.perspective()
    }

    fn observation(&self) -> Result<ObservationEnvelope, PlayerEndpointError> {
        self.inner.observation()
    }

    fn information_state(&self) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        self.inner.information_state()
    }

    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError> {
        self.inner.visible_decision()
    }

    fn submit(&self, response: DecisionResponseV2) -> Result<PlayerStepV2, PlayerEndpointError> {
        self.submits.fetch_add(1, Ordering::SeqCst);
        let step = self.inner.submit(response)?;
        *self.last_step_canonical.lock().unwrap() = encode_canonical(&step).ok();
        Ok(step)
    }
}

/// Controlled failing seam: a player-bound endpoint whose every operation
/// fails with exactly the closed layer-C error. Never a production type.
struct FailingEndpoint {
    perspective: PlayerId,
}

impl PlayerEndpoint for FailingEndpoint {
    fn perspective(&self) -> PlayerId {
        self.perspective
    }

    fn observation(&self) -> Result<ObservationEnvelope, PlayerEndpointError> {
        Err(PlayerEndpointError::ServiceUnavailable)
    }

    fn information_state(&self) -> Result<PlayerInformationStateV2, PlayerEndpointError> {
        Err(PlayerEndpointError::ServiceUnavailable)
    }

    fn visible_decision(&self) -> Result<Option<PlayerDecisionRequestV2>, PlayerEndpointError> {
        Err(PlayerEndpointError::ServiceUnavailable)
    }

    fn submit(&self, _response: DecisionResponseV2) -> Result<PlayerStepV2, PlayerEndpointError> {
        Err(PlayerEndpointError::ServiceUnavailable)
    }
}

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

/// Reset + spy-wrapped P1, driven through the production-equivalent
/// registration seam; the real P1 handle stays available for direct
/// comparison below the envelope.
fn spawned_with_spy() -> (Session, String, Arc<SubmitSpy>, PlayerEndpointHandle) {
    let mut session = Session::new(Some(TRUSTED_KEY.to_string()));
    reset_ok(&mut session, 0);
    let real_p1 = session.live_handle_for_test(P1).expect("P1 resolvable");
    let spy = Arc::new(SubmitSpy::new(Arc::new(real_p1.clone())));
    let token_p1 = session
        .bind_endpoint_for_test(P1, Arc::clone(&spy) as Arc<dyn PlayerEndpoint>)
        .unwrap();
    (session, token_p1, spy, real_p1)
}

/// Reads P1's entry decision through the handler and decodes it.
fn visible_request(session: &mut Session, token: &str) -> PlayerDecisionRequestV2 {
    let response = handle(session, None, "visible_decision", json!({"token": token}));
    assert_eq!(response["ok"], true);
    let encoded = response["result"]["visible_decision_wire_b64"]
        .as_str()
        .expect("entry decision must be visible");
    decode_canonical::<PlayerDecisionRequestV2>(&unb64(encoded)).unwrap()
}

fn answered(request: &PlayerDecisionRequestV2, answer: DecisionAnswerV2) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: request.player_decision_id,
        state_revision: request.state_revision,
        answer,
    }
}

fn first_candidate(request: &PlayerDecisionRequestV2) -> CandidateIdV1 {
    request
        .candidates
        .first()
        .expect("dense candidate")
        .candidate_id
}

fn stale_answered(request: &PlayerDecisionRequestV2) -> DecisionResponseV2 {
    let mut response = answered(
        request,
        DecisionAnswerV2::SelectOne {
            candidate_id: first_candidate(request),
        },
    );
    response.player_decision_id = PlayerDecisionIdV1(response.player_decision_id.0 + 1);
    response
}

/// Canonical bytes of a plausible-but-unjudged entry response. Malformed
/// classes fail at layer A before any semantics, so exact identity values
/// are irrelevant; only the canonical document shape matters (the M2.G
/// wire_boundary precedent).
fn plausible_canonical_bytes() -> Vec<u8> {
    encode_canonical(&DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(0),
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
    })
    .unwrap()
}

// ---------------------------------------------------------------------------
// Malformed raw-byte corruption classes
// ---------------------------------------------------------------------------

type Corrupt = fn(&[u8]) -> Vec<u8>;

/// Shape sanity guard shared by the splicing corruptors: they assume
/// canonical JSON-object bytes so a codec drift fails loudly here instead
/// of silently producing meaningless classes.
fn assert_object_shape(canonical: &[u8]) {
    assert!(
        canonical.first() == Some(&b'{') && canonical.last() == Some(&b'}'),
        "canonical submission bytes are no longer a JSON object"
    );
}

fn leading_whitespace(canonical: &[u8]) -> Vec<u8> {
    assert_object_shape(canonical);
    let mut bytes = Vec::with_capacity(canonical.len() + 1);
    bytes.push(b' ');
    bytes.extend_from_slice(canonical);
    bytes
}

/// Canonical bytes sort keys; a fixed non-sorted top-level order breaks the
/// byte comparison even though the JSON is semantically identical.
fn wrong_key_order(canonical: &[u8]) -> Vec<u8> {
    assert_object_shape(canonical);
    let fields = serde_json::from_slice::<Value>(canonical)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    let order = [
        "schema_version",
        "player_decision_id",
        "state_revision",
        "answer",
    ];
    let pieces: Vec<String> = order
        .iter()
        .map(|key| format!("\"{key}\":{}", fields[*key]))
        .collect();
    format!("{{{}}}", pieces.join(",")).into_bytes()
}

fn unknown_field_added(canonical: &[u8]) -> Vec<u8> {
    assert_object_shape(canonical);
    let mut bytes = canonical[..canonical.len() - 1].to_vec();
    bytes.extend_from_slice(b",\"adapter_unknown_field\":1}");
    bytes
}

fn wrong_schema_version(canonical: &[u8]) -> Vec<u8> {
    assert_object_shape(canonical);
    std::str::from_utf8(canonical)
        .unwrap()
        .replace(DECISION_RESPONSE_V2_SCHEMA, "decision-response.v9")
        .into_bytes()
}

fn truncated_json(canonical: &[u8]) -> Vec<u8> {
    assert_object_shape(canonical);
    canonical[..canonical.len() - 8].to_vec()
}

fn candidate_id_overflow(canonical: &[u8]) -> Vec<u8> {
    assert_object_shape(canonical);
    std::str::from_utf8(canonical)
        .unwrap()
        .replace("\"candidate_id\":0", "\"candidate_id\":4294967296")
        .into_bytes()
}

/// Invalid UTF-8 spliced mid-document: unrepresentable through any JSON
/// string carrier, lossless through the base64 envelope.
fn invalid_utf8_splice(canonical: &[u8]) -> Vec<u8> {
    assert_object_shape(canonical);
    let split = canonical.len() / 2;
    let mut bytes = canonical[..split].to_vec();
    bytes.extend_from_slice(&[0xFF, 0xFE]);
    bytes.extend_from_slice(&canonical[split..]);
    bytes
}

fn embedded_nul(canonical: &[u8]) -> Vec<u8> {
    assert_object_shape(canonical);
    let split = canonical.len() / 2;
    let mut bytes = canonical[..split].to_vec();
    bytes.push(0x00);
    bytes.extend_from_slice(&canonical[split..]);
    bytes
}

/// Trailing truncated multibyte sequence: invalid UTF-8 AND trailing garbage.
fn truncated_multibyte_tail(canonical: &[u8]) -> Vec<u8> {
    assert_object_shape(canonical);
    let mut bytes = canonical.to_vec();
    bytes.extend_from_slice(&[0xE2, 0x82]);
    bytes
}

fn arbitrary_garbage(_canonical: &[u8]) -> Vec<u8> {
    [0xDE, 0xAD, 0xBE, 0xEF].repeat(16)
}

const DOCUMENT_CLASSES: &[(&str, Corrupt)] = &[
    ("leading_whitespace", leading_whitespace),
    ("wrong_key_order", wrong_key_order),
    ("unknown_field_added", unknown_field_added),
    ("wrong_schema_version", wrong_schema_version),
    ("truncated_json", truncated_json),
    ("candidate_id_u32_overflow", candidate_id_overflow),
];

const RAW_BYTE_CLASSES: &[(&str, Corrupt)] = &[
    ("invalid_utf8_sequence", invalid_utf8_splice),
    ("embedded_nul", embedded_nul),
    ("truncated_multibyte", truncated_multibyte_tail),
    ("arbitrary_garbage_bytes", arbitrary_garbage),
];

// ---------------------------------------------------------------------------
// A+B: submit-counter proofs and the malformed matrix, through the handler
// ---------------------------------------------------------------------------

#[test]
fn malformed_wire_classes_fail_closed_zero_submits_session_healthy() {
    let canonical = plausible_canonical_bytes();
    for (group, classes) in [
        ("document_level", DOCUMENT_CLASSES),
        ("raw_byte_level", RAW_BYTE_CLASSES),
    ] {
        for (name, corrupt) in classes {
            let mut session = Session::new(Some(TRUSTED_KEY.to_string()));
            reset_ok(&mut session, 0);
            let real = session.live_handle_for_test(P1).unwrap();
            let spy = Arc::new(SubmitSpy::new(Arc::new(real)));
            let token = session
                .bind_endpoint_for_test(P1, Arc::clone(&spy) as Arc<dyn PlayerEndpoint>)
                .unwrap();

            let corrupted = corrupt(&canonical);
            let (line, response) = handle_line(
                &mut session,
                None,
                "submit",
                json!({"token": token, "response_wire_b64": b64(&corrupted)}),
            );

            assert_eq!(
                response["ok"], false,
                "class {group}/{name} must fail closed"
            );
            assert_eq!(
                error_code(&response),
                "malformed_response",
                "class {group}/{name} code"
            );
            assert_eq!(
                spy.submit_calls(),
                0,
                "class {group}/{name} must never reach the semantic submit"
            );
            assert!(
                spy.captured_canonical().is_none(),
                "class {group}/{name} must produce no step"
            );

            // Session remains healthy: a subsequent authorized read succeeds.
            let health = handle(&mut session, None, "observation", json!({"token": token}));
            assert_eq!(
                health["ok"], true,
                "class {group}/{name} left an unhealthy session"
            );
            assert!(health["result"]["observation_wire_b64"].is_string());

            // Redaction: nothing beyond the closed envelope was emitted.
            assert_closed_error_envelope(&line, None, "malformed_response");
        }
    }
}

/// Typed stale response: layer-A decode succeeds, the endpoint IS called
/// exactly once, and the step carries the rejected/stale_decision outcome.
#[test]
fn stale_typed_submission_reaches_endpoint_exactly_once() {
    let (mut session, token, spy, _real_p1) = spawned_with_spy();

    let request = visible_request(&mut session, &token);
    let stale = stale_answered(&request);
    let response = handle(
        &mut session,
        None,
        "submit",
        json!({"token": token, "response_wire_b64": b64(&encode_canonical(&stale).unwrap())}),
    );

    assert_eq!(
        response["ok"], true,
        "typed rejections ride inside ok steps"
    );
    let step: PlayerStepV2 = decode_canonical(&unb64(
        response["result"]["step_wire_b64"].as_str().unwrap(),
    ))
    .unwrap();
    assert_eq!(
        step.submission,
        PlayerStepSubmissionV1::Rejected {
            code: PlayerSubmissionCodeV1::StaleDecision,
        }
    );
    assert_eq!(spy.submit_calls(), 1, "exactly one endpoint submit");
}

/// Foreign-actor submission via P2's token while P1 holds the request:
/// exactly one submit of P2's own endpoint and the uniform
/// unavailable_decision rejection.
#[test]
fn foreign_actor_submission_reaches_its_endpoint_exactly_once() {
    let mut session = Session::new(Some(TRUSTED_KEY.to_string()));
    reset_ok(&mut session, 0);

    // P1 plain-bound: fixture source for a well-formed response.
    let token_p1 = bind_plain(&mut session, 1, "1");
    let request = visible_request(&mut session, &token_p1);
    let answer = answered(
        &request,
        DecisionAnswerV2::SelectOne {
            candidate_id: first_candidate(&request),
        },
    );

    // P2 decorated: the submitting actor's endpoint is the counted one.
    let real_p2 = session.live_handle_for_test(P2).unwrap();
    let spy_p2 = Arc::new(SubmitSpy::new(Arc::new(real_p2)));
    let token_p2 = session
        .bind_endpoint_for_test(P2, Arc::clone(&spy_p2) as Arc<dyn PlayerEndpoint>)
        .unwrap();

    let response = handle(
        &mut session,
        None,
        "submit",
        json!({"token": token_p2, "response_wire_b64": b64(&encode_canonical(&answer).unwrap())}),
    );

    assert_eq!(response["ok"], true);
    let step: PlayerStepV2 = decode_canonical(&unb64(
        response["result"]["step_wire_b64"].as_str().unwrap(),
    ))
    .unwrap();
    assert_eq!(
        step.submission,
        PlayerStepSubmissionV1::Rejected {
            code: PlayerSubmissionCodeV1::UnavailableDecision,
        }
    );
    assert_eq!(
        spy_p2.submit_calls(),
        1,
        "the actor's endpoint IS called once"
    );
}

/// Accepted response: exactly one submit, step accepted, state advanced.
#[test]
fn accepted_submission_reaches_endpoint_exactly_once() {
    let (mut session, token, spy, _real_p1) = spawned_with_spy();

    let request = visible_request(&mut session, &token);
    let answer = answered(
        &request,
        DecisionAnswerV2::SelectOne {
            candidate_id: first_candidate(&request),
        },
    );
    let response = handle(
        &mut session,
        None,
        "submit",
        json!({"token": token, "response_wire_b64": b64(&encode_canonical(&answer).unwrap())}),
    );

    assert_eq!(response["ok"], true);
    let step: PlayerStepV2 = decode_canonical(&unb64(
        response["result"]["step_wire_b64"].as_str().unwrap(),
    ))
    .unwrap();
    assert_eq!(step.submission, PlayerStepSubmissionV1::Accepted);
    assert_eq!(spy.submit_calls(), 1, "exactly one endpoint submit");
}

// ---------------------------------------------------------------------------
// C: handler transparency below JSONL for every op
// ---------------------------------------------------------------------------

#[test]
fn read_operations_are_transparent_below_the_envelope() {
    let (mut session, token_p1, _spy, real_p1) = spawned_with_spy();
    let token_p2 = bind_plain(&mut session, 2, "2");
    let real_p2 = session.live_handle_for_test(P2).unwrap();

    // observation
    let (_, response) = handle_line(
        &mut session,
        None,
        "observation",
        json!({"token": token_p1}),
    );
    assert_eq!(response["ok"], true);
    let emitted = unb64(response["result"]["observation_wire_b64"].as_str().unwrap());
    assert_eq!(
        emitted,
        encode_canonical(&real_p1.observation().unwrap()).unwrap()
    );

    // information_state
    let (_, response) = handle_line(
        &mut session,
        None,
        "information_state",
        json!({"token": token_p1}),
    );
    assert_eq!(response["ok"], true);
    let emitted = unb64(
        response["result"]["information_state_wire_b64"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(
        emitted,
        encode_canonical(&real_p1.information_state().unwrap()).unwrap()
    );

    // visible_decision Some (P1 holds the entry decision)
    let (_, response) = handle_line(
        &mut session,
        None,
        "visible_decision",
        json!({"token": token_p1}),
    );
    assert_eq!(response["ok"], true);
    let emitted = unb64(
        response["result"]["visible_decision_wire_b64"]
            .as_str()
            .unwrap(),
    );
    let direct = real_p1
        .visible_decision()
        .unwrap()
        .expect("entry decision visible");
    assert_eq!(emitted, encode_canonical(&direct).unwrap());

    // visible_decision None (P2 has no pending request)
    let (_, response) = handle_line(
        &mut session,
        None,
        "visible_decision",
        json!({"token": token_p2}),
    );
    assert_eq!(response["ok"], true);
    assert_eq!(
        response["result"]["visible_decision_wire_b64"],
        Value::Null,
        "handler must emit explicit null for an absent decision"
    );
    assert!(real_p2.visible_decision().unwrap().is_none());
}

/// ACCEPTED submits advance state, so the accepted instance is consumed
/// EXACTLY once: the spy captures the step returned by that one call and
/// its canonical bytes must equal the payload emitted by the same request.
#[test]
fn accepted_submit_single_shot_spy_matches_emitted_bytes() {
    let (mut session, token, spy, _real_p1) = spawned_with_spy();

    let request = visible_request(&mut session, &token);
    let answer = answered(
        &request,
        DecisionAnswerV2::SelectOne {
            candidate_id: first_candidate(&request),
        },
    );
    let (_line, response) = handle_line(
        &mut session,
        None,
        "submit",
        json!({"token": token, "response_wire_b64": b64(&encode_canonical(&answer).unwrap())}),
    );
    assert_eq!(response["ok"], true);
    assert_eq!(
        spy.submit_calls(),
        1,
        "the accepted state is consumed exactly once"
    );
    let emitted = unb64(response["result"]["step_wire_b64"].as_str().unwrap());
    assert_eq!(
        spy.captured_canonical().as_deref(),
        Some(emitted.as_slice()),
        "emitted step bytes must be the canonical encoding of the returned step"
    );
}

/// Rejected submits are defined nonmutating, so same-instance transparency
/// is additionally allowed: the handler-driven rejection bytes equal a
/// direct trait-call rejection on the same backend afterwards.
#[test]
fn rejected_submit_same_instance_transparency() {
    let (mut session, token, spy, real_p1) = spawned_with_spy();

    let request = visible_request(&mut session, &token);
    let stale = stale_answered(&request);
    let stale_bytes = encode_canonical(&stale).unwrap();

    let (_, response) = handle_line(
        &mut session,
        None,
        "submit",
        json!({"token": token, "response_wire_b64": b64(&stale_bytes)}),
    );
    assert_eq!(response["ok"], true);
    assert_eq!(spy.submit_calls(), 1);
    let handler_bytes = unb64(response["result"]["step_wire_b64"].as_str().unwrap());
    assert_eq!(
        spy.captured_canonical().as_deref(),
        Some(handler_bytes.as_slice())
    );

    // Same-instance direct replay of the identical typed rejection.
    let step_direct = real_p1.submit(stale.clone()).unwrap();
    assert_eq!(encode_canonical(&step_direct).unwrap(), handler_bytes);
    assert_eq!(
        spy.submit_calls(),
        1,
        "direct calls bypass the installed wrapper"
    );
}

// ---------------------------------------------------------------------------
// D: panic classification policy
// ---------------------------------------------------------------------------

fn assert_closed_error_envelope(line: &str, id: Option<u64>, code: &str) {
    let parsed: Value = serde_json::from_str(line).expect("envelope lines are JSON");
    assert_eq!(
        parsed,
        json!({"v": protocol::PROTOCOL_VERSION, "id": id, "ok": false, "error": {"code": code}}),
        "emitted line must be EXACTLY the closed error envelope"
    );
}

#[test]
fn panic_classification_policy_is_closed_and_detail_free() {
    for id in [None, Some(u64::MAX)] {
        // PLAYER command panic: best-effort frozen layer-C surface + terminate.
        let (player_line, player_exit) = crate::panic_envelope_and_exit(false, id);
        assert_eq!(player_exit, protocol::EXIT_FATAL, "terminate intent");
        assert_closed_error_envelope(&player_line, id, "service_unavailable");

        // TRUSTED command panic: adapter-internal classification + terminate.
        let (trusted_line, trusted_exit) = crate::panic_envelope_and_exit(true, id);
        assert_eq!(trusted_exit, protocol::EXIT_FATAL, "terminate intent");
        assert_closed_error_envelope(&trusted_line, id, "internal_error");

        // Neither classification can carry panic detail: the envelopes are
        // byte-closed shapes whose text contains no diagnostic vocabulary.
        for marker in [
            "panic",
            "detail",
            "poison",
            "backend",
            "unwind",
            TRUSTED_KEY,
        ] {
            assert!(
                !player_line.contains(marker),
                "leaked {marker:?} to player surface"
            );
            assert!(
                !trusted_line.contains(marker),
                "leaked {marker:?} to trusted surface"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E: controlled failing seam — redaction with continue
// ---------------------------------------------------------------------------

#[test]
fn failing_seam_redacts_and_service_continues() {
    let mut session = Session::new(Some(TRUSTED_KEY.to_string()));
    reset_ok(&mut session, 1);

    let failing = Arc::new(FailingEndpoint { perspective: P1 });
    let token = session
        .bind_endpoint_for_test(P1, failing as Arc<dyn PlayerEndpoint>)
        .unwrap();

    let mut emitted_lines: Vec<String> = Vec::new();
    for op in ["observation", "information_state", "visible_decision"] {
        let (line, response) = handle_line(&mut session, Some(10), op, json!({"token": token}));
        // Exact closed shape: the failing seam can emit NOTHING beyond the
        // frozen envelope — this is the redaction property itself.
        assert_closed_error_envelope(&line, Some(10), "service_unavailable");
        emitted_lines.push(line);
        assert_eq!(response["ok"], false, "{op} must fail closed");
        assert_eq!(error_code(&response), "service_unavailable");
    }

    // Submit also fails at the service layer (bytes are perfectly valid).
    let (line, response) = handle_line(
        &mut session,
        Some(11),
        "submit",
        json!({"token": token, "response_wire_b64": b64(&plausible_canonical_bytes())}),
    );
    assert_closed_error_envelope(&line, Some(11), "service_unavailable");
    emitted_lines.push(line);
    assert_eq!(error_code(&response), "service_unavailable");

    // Loop continues: the next valid command succeeds on the healthy route.
    let token_p2 = bind_plain(&mut session, 12, "2");
    let recovery = handle(
        &mut session,
        Some(13),
        "observation",
        json!({"token": token_p2}),
    );
    assert_eq!(
        recovery["ok"], true,
        "session must remain servable after failures"
    );

    // NO trusted detail anywhere in ANY emitted output.
    for line in &emitted_lines {
        for secret in [TRUSTED_KEY, ROOT_SEED_HEX] {
            assert!(!line.contains(secret), "trusted detail leaked: {secret:?}");
        }
    }
}

#[test]
fn run_level_service_failure_then_recovery_continues_the_loop() {
    let script = format!(
        "{{\"v\":1,\"id\":1,\"cmd\":\"reset_synthetic\",\"params\":{{\"trusted_key\":\"{TRUSTED_KEY}\",\"players\":[\"1\",\"2\"],\"root_seed_hex\":\"{ROOT_SEED_HEX}\"}}}}\n\
         {{\"v\":1,\"id\":2,\"cmd\":\"direct_call\",\"params\":{{\"trusted_key\":\"{TRUSTED_KEY}\",\"op\":\"observation\",\"player\":\"1\"}}}}\n\
         {{\"v\":1,\"id\":3,\"cmd\":\"bind_player\",\"params\":{{\"trusted_key\":\"{TRUSTED_KEY}\",\"player\":\"1\"}}}}\n\
         {{\"v\":1,\"id\":4,\"cmd\":\"direct_call\",\"params\":{{\"trusted_key\":\"{TRUSTED_KEY}\",\"op\":\"observation\",\"player\":\"1\"}}}}\n\
         {{\"v\":1,\"id\":5,\"cmd\":\"shutdown\",\"params\":{{\"trusted_key\":\"{TRUSTED_KEY}\"}}}}\n"
    );
    let mut input = Cursor::new(script);
    let mut output = Vec::new();
    let code = crate::run(&mut input, &mut output, Some(TRUSTED_KEY.to_string()));

    assert_eq!(
        code,
        protocol::EXIT_OK,
        "controlled failure must not kill the loop"
    );
    let text = String::from_utf8(output).unwrap();
    let lines: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 5, "one response per scripted command");

    // Controlled failure BEFORE any binding: closed service class.
    assert_eq!(lines[1]["id"], 2);
    assert_eq!(error_code(&lines[1]), "service_unavailable");

    // The loop continued: bind succeeded, then the SAME direct route works.
    assert_eq!(lines[2]["ok"], true);
    assert_eq!(lines[3]["id"], 4);
    assert_eq!(lines[3]["ok"], true, "recovery proves continued service");
    assert!(lines[3]["result"]["observation_wire_b64"].is_string());

    // NO trusted detail anywhere in the process output.
    for secret in [TRUSTED_KEY, ROOT_SEED_HEX] {
        assert!(!text.contains(secret), "trusted detail leaked into stdout");
    }
}

// ---------------------------------------------------------------------------
// F: base64 fidelity through the protocol helpers
// ---------------------------------------------------------------------------

struct Xorshift64(u64);

impl Xorshift64 {
    fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            chunk.copy_from_slice(&x.to_le_bytes()[..chunk.len()]);
        }
    }
}

#[test]
fn base64_round_trip_fidelity_through_protocol_helpers() {
    let mut corpus: Vec<Vec<u8>> = vec![
        Vec::new(),
        vec![0x00],
        vec![0xFF],
        vec![b'A'],
        (0u8..=255).collect(),
        vec![0x00; 256],
        vec![0xFF; 64],
        b"plain ascii payload".to_vec(),
    ];
    let mut rng = Xorshift64(0x243F_6A88_85A3_08D3);
    for size in [1usize, 2, 3, 5, 8, 63, 64, 65, 1024, 4096, 65_536] {
        let mut buffer = vec![0u8; size];
        rng.fill(&mut buffer);
        corpus.push(buffer);
    }

    for bytes in &corpus {
        let encoded = STANDARD.encode(bytes);
        // Standard alphabet with `=` padding, deterministic length.
        assert!(
            encoded.bytes().all(|byte| byte.is_ascii_alphanumeric()
                || byte == b'+'
                || byte == b'/'
                || byte == b'='),
            "encoded form left the standard base64 alphabet"
        );
        assert_eq!(encoded.len() % 4, 0, "padded output length invariant");
        assert_eq!(encoded.len(), encoded_len(bytes.len()));

        // Round-trip identity through the engine...
        assert_eq!(STANDARD.decode(&encoded).unwrap(), *bytes);
        // ...and through the exact helper the submit path uses.
        let mut params = Map::new();
        params.insert("payload".into(), Value::String(encoded.clone()));
        assert_eq!(
            protocol::decode_wire_bytes(&params, "payload").as_deref(),
            Some(bytes.as_slice()),
        );
    }
}

fn encoded_len(raw: usize) -> usize {
    (raw.div_ceil(3)) * 4
}

// ---------------------------------------------------------------------------
// G: promoted named negatives
// ---------------------------------------------------------------------------

#[test]
fn eof_after_shutdown_processes_nothing_more_and_exits_cleanly() {
    let shutdown = json!({
        "v": 1, "id": 7, "cmd": "shutdown",
        "params": {"trusted_key": TRUSTED_KEY}
    })
    .to_string();
    let later_reset = json!({
        "v": 1, "id": 8, "cmd": "reset_synthetic",
        "params": {"trusted_key": TRUSTED_KEY, "players": ["1", "2"], "root_seed_hex": ROOT_SEED_HEX}
    })
    .to_string();
    let script = format!("{shutdown}\n{later_reset}\n");

    let mut input = Cursor::new(script);
    let mut output = Vec::new();
    let code = crate::run(&mut input, &mut output, Some(TRUSTED_KEY.to_string()));

    assert_eq!(code, protocol::EXIT_OK, "shutdown terminates cleanly");
    let text = String::from_utf8(output).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "nothing after the shutdown ack may be emitted"
    );
    let ack: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(ack["id"], 7);
    assert_eq!(ack["ok"], true);
}

#[test]
fn unknown_token_error_envelopes_are_uniform_modulo_id() {
    let mut session = Session::new(Some(TRUSTED_KEY.to_string()));
    reset_ok(&mut session, 1);
    let invalidated = bind_plain(&mut session, 2, "1");
    reset_ok(&mut session, 3); // epoch replacement: every issued token dies

    // Same request shape, two different failure origins, ids deliberately
    // omitted so byte equality can be total.
    let (_, after_reset) = handle_line(
        &mut session,
        None,
        "observation",
        json!({"token": invalidated}),
    );
    let (_, never_minted) = handle_line(
        &mut session,
        None,
        "observation",
        json!({"token": "f".repeat(32)}),
    );

    let expected = json!({
        "v": protocol::PROTOCOL_VERSION, "id": null,
        "ok": false, "error": {"code": "unknown_token"}
    });
    assert_eq!(after_reset, expected);
    assert_eq!(never_minted, expected, "uniform unknown-token shape");

    // With ids present, the envelopes differ ONLY by the echoed id.
    fn strip_id(mut value: Value) -> Value {
        value
            .as_object_mut()
            .expect("envelope object")
            .remove("id")
            .expect("id field present");
        value
    }
    let with_id_a = handle(
        &mut session,
        Some(100),
        "observation",
        json!({"token": invalidated}),
    );
    let with_id_b = handle(
        &mut session,
        Some(200),
        "observation",
        json!({"token": "f".repeat(32)}),
    );
    let stripped_a = strip_id(with_id_a.clone());
    assert_eq!(with_id_a["id"], 100);
    assert_eq!(with_id_b["id"], 200);
    assert_eq!(stripped_a, strip_id(with_id_b));
}

#[test]
fn tokens_are_never_echoed_in_responses_after_binding_results() {
    let mut session = Session::new(Some(TRUSTED_KEY.to_string()));
    let mut emitted: Vec<String> = Vec::new();

    let push =
        |session: &mut Session, emitted: &mut Vec<String>, id: u64, cmd: &str, params: Value| {
            let (line, _) = handle_line(session, Some(id), cmd, params);
            emitted.push(line);
        };

    push(
        &mut session,
        &mut emitted,
        1,
        "reset_synthetic",
        json!({
            "trusted_key": TRUSTED_KEY,
            "players": ["1", "2"],
            "root_seed_hex": ROOT_SEED_HEX,
        }),
    );
    push(
        &mut session,
        &mut emitted,
        2,
        "bind_player",
        json!({"trusted_key": TRUSTED_KEY, "player": "1"}),
    );
    push(
        &mut session,
        &mut emitted,
        3,
        "bind_player",
        json!({"trusted_key": TRUSTED_KEY, "player": "2"}),
    );

    let token_one = serde_json::from_str::<Value>(&emitted[1]).unwrap()["result"]["token"]
        .as_str()
        .unwrap()
        .to_string();
    let token_two = serde_json::from_str::<Value>(&emitted[2]).unwrap()["result"]["token"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(token_one, token_two);
    // Sanity: issuance itself necessarily carries the minted token.
    assert!(emitted[1].contains(&token_one));
    assert!(emitted[2].contains(&token_two));

    push(
        &mut session,
        &mut emitted,
        4,
        "observation",
        json!({"token": token_one}),
    );
    push(
        &mut session,
        &mut emitted,
        5,
        "information_state",
        json!({"token": token_one}),
    );
    push(
        &mut session,
        &mut emitted,
        6,
        "visible_decision",
        json!({"token": token_one}),
    );
    push(
        &mut session,
        &mut emitted,
        7,
        "visible_decision",
        json!({"token": token_two}),
    );

    // A typed (stale) submission closes the sequence.
    let mut preflight = Session::new(Some(TRUSTED_KEY.to_string()));
    reset_ok(&mut preflight, 20);
    let preflight_token = bind_plain(&mut preflight, 21, "1");
    let request = visible_request(&mut preflight, &preflight_token);
    let stale = stale_answered(&request);
    push(
        &mut session,
        &mut emitted,
        8,
        "submit",
        json!({"token": token_one, "response_wire_b64": b64(&encode_canonical(&stale).unwrap())}),
    );
    let submitted: Value = serde_json::from_str(&emitted[7]).unwrap();
    assert_eq!(
        submitted["ok"], true,
        "typed rejections ride inside ok steps"
    );
    let step: PlayerStepV2 = decode_canonical(&unb64(
        submitted["result"]["step_wire_b64"].as_str().unwrap(),
    ))
    .unwrap();
    assert!(matches!(
        step.submission,
        PlayerStepSubmissionV1::Rejected { .. }
    ));

    // THE assertion: no response after the bind results repeats either token.
    for line in &emitted[3..] {
        assert!(!line.contains(&token_one), "token one echoed in: {line}");
        assert!(!line.contains(&token_two), "token two echoed in: {line}");
    }
}

// ---------------------------------------------------------------------------
// H: in-crate guards
// ---------------------------------------------------------------------------

/// The frozen DIRECT-dependency allowlist for this tool (plan §D/§L).
///
/// `MTGML_ALLOWLIST` is the exact set of workspace (mtgml-*) dependencies:
/// configuration identity types are why mtgml-replay appears; the forbidden
/// set (state/rules/persistence/conformance) is implied by exact-set
/// equality and asserted explicitly for clarity. `INFRASTRUCTURE` pins the
/// mechanical envelope deps; ANY new dependency — including a new
/// infrastructure crate — fails this guard until consciously re-reviewed.
#[test]
fn tool_direct_dependencies_match_frozen_allowlist() {
    const TOOL: &str = "m2-semantic-adapter";
    const MTGML_ALLOWLIST: [&str; 7] = [
        "mtgml-environment",
        "mtgml-wire",
        "mtgml-decision",
        "mtgml-observation",
        "mtgml-model",
        "mtgml-random",
        "mtgml-replay",
    ];
    const INFRASTRUCTURE: [&str; 4] = ["base64", "getrandom", "serde", "serde_json"];
    const FORBIDDEN_DIRECT: [&str; 4] = [
        "mtgml-state",
        "mtgml-rules",
        "mtgml-persistence",
        "mtgml-conformance",
    ];

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("..").join("..");

    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let metadata_output = Command::new(cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(&workspace_root)
        .output()
        .expect("cargo metadata must be executable from the test");
    assert!(
        metadata_output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&metadata_output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&metadata_output.stdout).unwrap();
    let packages = metadata["packages"].as_array().expect("packages array");

    let tool_package = packages
        .iter()
        .find(|package| package["name"] == TOOL)
        .expect("tool package must be a workspace member");
    let mut direct: Vec<String> = tool_package["dependencies"]
        .as_array()
        .expect("dependency list")
        .iter()
        .filter(|dep| dep["kind"].is_null())
        .filter(|dep| dep["optional"].as_bool() != Some(true))
        .map(|dep| dep["name"].as_str().expect("dep name").to_string())
        .collect();
    direct.sort();

    let mtgml_deps: Vec<String> = direct
        .iter()
        .filter(|name| name.starts_with("mtgml-"))
        .cloned()
        .collect();
    let mut expected_mtgml: Vec<String> = MTGML_ALLOWLIST
        .iter()
        .map(|name| name.to_string())
        .collect();
    expected_mtgml.sort();
    assert_eq!(
        mtgml_deps, expected_mtgml,
        "tool mtgml-* direct dependencies drifted from the frozen allowlist"
    );

    let mut frozen_total: Vec<String> = MTGML_ALLOWLIST
        .iter()
        .chain(INFRASTRUCTURE.iter())
        .map(|name| name.to_string())
        .collect();
    frozen_total.sort();
    assert_eq!(
        direct, frozen_total,
        "tool direct dependency SET drifted; update this pinned guard consciously"
    );

    for forbidden in FORBIDDEN_DIRECT {
        assert!(
            !direct.contains(&forbidden.to_string()),
            "forbidden direct dependency {forbidden}"
        );
    }

    // Nothing in the workspace may depend ON the unpublished tool.
    for package in packages {
        let name = package["name"].as_str().expect("package name");
        for dep in package["dependencies"].as_array().expect("deps") {
            assert_ne!(
                dep["name"].as_str().expect("dep name"),
                TOOL,
                "workspace package {name} depends on the tool"
            );
        }
    }
}

/// Occurrence finder, case-sensitive. Two modes:
///
/// - [`MatchMode::WholeIdentifier`] — a hit requires identifier boundaries
///   on BOTH sides, so identifier-extended spellings never fire.
/// - [`MatchMode::IdentifierPrefix`] — the token must START an identifier
///   (non-identifier on the left only), so CamelCase type families with
///   version suffixes are still caught while lookalikes that merely
///   CONTAIN similar words never fire.
#[derive(Clone, Copy)]
enum MatchMode {
    WholeIdentifier,
    IdentifierPrefix,
}

fn token_hits(text: &str, token: &str, mode: MatchMode) -> Vec<usize> {
    let ident_char = |character: char| character.is_ascii_alphanumeric() || character == '_';
    let mut hits = Vec::new();
    let mut cursor = 0;
    while let Some(found) = text[cursor..].find(token) {
        let start = cursor + found;
        let end = start + token.len();
        let preceded = text[..start].chars().next_back().is_some_and(ident_char);
        let followed = text[end..].chars().next().is_some_and(ident_char);
        let is_hit = match mode {
            MatchMode::WholeIdentifier => !preceded && !followed,
            MatchMode::IdentifierPrefix => !preceded,
        };
        if is_hit {
            hits.push(start);
        }
        cursor = end;
    }
    hits
}

fn collect_rs_files(directory: &PathBuf, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("src directory readable") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

#[test]
fn tool_source_contains_no_privileged_operations() {
    // Assembled at RUNTIME from fragments on purpose: this scan's own source
    // lives inside the scanned tree, so pinning any forbidden literal here
    // would make the guard self-trip. Fragment concatenation keeps this file
    // clean while producing the exact tokens below.
    let dot_call = |method: &str| (dot_call_method(method), MatchMode::WholeIdentifier);
    fn dot_call_method(method: &str) -> String {
        format!(".{method}(")
    }
    let whole = |parts: &[&str]| (parts.concat(), MatchMode::WholeIdentifier);
    let prefix = |parts: &[&str]| (parts.concat(), MatchMode::IdentifierPrefix);
    let tokens: Vec<(&str, (String, MatchMode))> = vec![
        ("controller capability method call", dot_call("checkpoint")),
        ("controller capability method call", dot_call("restore")),
        ("controller capability method call", dot_call("fork")),
        ("privileged export operation", whole(&["export_", "replay"])),
        (
            "privileged trusted execution",
            whole(&["execute_", "trusted_", "response"]),
        ),
        (
            "privileged execution entry",
            whole(&["execute_", "replay_", "from_", "checkpoint"]),
        ),
        (
            "authoritative replay type family",
            prefix(&["Authoritative", "Replay"]),
        ),
        ("recorder type family", prefix(&["Replay", "Recorder"])),
        ("replay step type family", prefix(&["Replay", "Step"])),
    ];

    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_dir, &mut files);
    assert!(
        files.len() >= 7,
        "source walk found implausibly few files ({})",
        files.len()
    );

    let mut violations = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("source file readable");
        for (reason, (token, mode)) in &tokens {
            for position in token_hits(&text, token, *mode) {
                let line = text[..position]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    + 1;
                violations.push(format!(
                    "{}:{line}: forbidden {reason}: {token}",
                    file.display()
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "privileged operations found in tool source:\n{}",
        violations.join("\n")
    );
}

#[test]
fn privilege_scan_detector_rejects_hits_and_allows_lookalikes() {
    let replay_export = concat!("export_", "replay");
    let authoritative = concat!("Authoritative", "Replay");
    let recorder = concat!("Replay", "Recorder");
    let step = concat!("Replay", "Step");

    // Positive demonstrations are assembled at runtime for the same
    // self-trip reason the guard itself assembles its tokens: this test's
    // source sits inside the scanned tree.
    let demo_method_call = format!(".{replay_export}()");
    let demo_suffixed_type = format!("use crate::{authoritative}V3;");
    let demo_step_type = format!("{step}V2");
    let demo_recorder_type = format!("{recorder}Handle");
    let extended = format!("let x = c.{replay_export}_now();");

    // Whole-identifier mode: identifier-extended spellings do NOT fire.
    assert_eq!(
        token_hits(&extended, replay_export, MatchMode::WholeIdentifier).len(),
        0,
        "identifier-extended occurrences are not whole-token matches"
    );
    assert_eq!(
        token_hits(&demo_method_call, replay_export, MatchMode::WholeIdentifier).len(),
        1
    );

    // Prefix mode: suffixed type families ARE caught.
    assert_eq!(
        token_hits(
            &demo_suffixed_type,
            authoritative,
            MatchMode::IdentifierPrefix
        )
        .len(),
        1
    );
    assert_eq!(
        token_hits(&demo_step_type, step, MatchMode::IdentifierPrefix).len(),
        1
    );
    assert_eq!(
        token_hits(&demo_recorder_type, recorder, MatchMode::IdentifierPrefix).len(),
        1
    );

    // Legitimate configuration identities never fire in either mode:
    // none of them contains a forbidden token as a matching substring.
    for identity in [
        "CheckpointCodecIdentity",
        "SyntheticM1ReplayConfig",
        "DeckIdentityV1",
        "KernelIdentityV1",
        "ReplaySchemaVersionsV1",
        concat!("replay", "-step.v3"),
        "synthetic-m2-memory",
    ] {
        for (token, mode) in [
            (replay_export, MatchMode::WholeIdentifier),
            (authoritative, MatchMode::IdentifierPrefix),
            (recorder, MatchMode::IdentifierPrefix),
            (step, MatchMode::IdentifierPrefix),
        ] {
            assert_eq!(
                token_hits(identity, token, mode).len(),
                0,
                "config identity {identity} must not match forbidden token {token}"
            );
        }
    }

    // Case sensitivity protects the hyphenated schema-version literal even
    // when probed with the CamelCase family stem.
    let hyphen_literal = format!("{lower_step}.v3", lower_step = concat!("replay", "-step"));
    assert_eq!(
        token_hits(&hyphen_literal, step, MatchMode::IdentifierPrefix).len(),
        0
    );
}
