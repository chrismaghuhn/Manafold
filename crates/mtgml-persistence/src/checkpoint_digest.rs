use crate::{cbor, envelope, PersistenceDecodeErrorV1};
use mtgml_model::{
    CheckpointCodecIdentity, CheckpointDigestV3, CheckpointDigestV4, DigestReferenceV1,
    EnvironmentLimitCounters, EpisodeStatus, FullStateDigestV3, FullStateDigestV4, PlayerOutcome,
    PlayerResult, TerminalReason, TruncationReason,
};

pub const CHECKPOINT_DOMAIN: &str = "mtgml.checkpoint-digest.v3";
pub const CHECKPOINT_INPUT_SCHEMA: &str = "environment-checkpoint-digest-input.v3";
pub const CHECKPOINT_DOMAIN_V4: &str = "mtgml.checkpoint-digest.v4";
pub const CHECKPOINT_INPUT_SCHEMA_V4: &str = "environment-checkpoint-digest-input.v4";

fn validate_full_state_reference(
    reference: &DigestReferenceV1,
    domain: &str,
    input_schema: &str,
) -> Result<(), PersistenceDecodeErrorV1> {
    if reference.envelope_version != envelope::DIGEST_ENVELOPE_ID
        || reference.algorithm_id != envelope::SHA256_ID
        || reference.semantic_domain != domain
        || reference.payload_codec_id != envelope::CANONICAL_CBOR_ID
        || reference.input_schema_id != input_schema
    {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    Ok(())
}

fn validate_checkpoint_digest_inputs(
    full_state_digest: &DigestReferenceV1,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
    domain: &str,
    input_schema: &str,
) -> Result<(), PersistenceDecodeErrorV1> {
    validate_full_state_reference(full_state_digest, domain, input_schema)?;
    status
        .validate()
        .map_err(|_| PersistenceDecodeErrorV1::SemanticValidation)?;
    counters
        .validate()
        .map_err(|_| PersistenceDecodeErrorV1::SemanticValidation)?;
    if codec.codec_id.is_empty() || codec.semantic_version.is_empty() {
        return Err(PersistenceDecodeErrorV1::SemanticValidation);
    }
    Ok(())
}

pub fn calculate_checkpoint_digest_v3(
    full_state_digest: &DigestReferenceV1,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
) -> Result<CheckpointDigestV3, PersistenceDecodeErrorV1> {
    validate_checkpoint_digest_inputs(
        full_state_digest,
        status,
        counters,
        codec,
        FullStateDigestV3::DOMAIN,
        "full-state-digest-input.v3",
    )?;
    let payload = checkpoint_payload(
        full_state_digest,
        status,
        counters,
        codec,
        CHECKPOINT_DOMAIN,
        CHECKPOINT_INPUT_SCHEMA,
    )?;
    let bytes = cbor::encode_canonical(&payload)?;
    let envelope = envelope::encode_envelope(CHECKPOINT_DOMAIN, CHECKPOINT_INPUT_SCHEMA, &bytes)?;
    Ok(CheckpointDigestV3::from_digest_bytes(
        envelope::hash_envelope(&envelope),
    ))
}

pub fn calculate_checkpoint_digest_v4(
    full_state_digest: &DigestReferenceV1,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
) -> Result<CheckpointDigestV4, PersistenceDecodeErrorV1> {
    validate_checkpoint_digest_inputs(
        full_state_digest,
        status,
        counters,
        codec,
        FullStateDigestV4::DOMAIN,
        "full-state-digest-input.v4",
    )?;
    let payload = checkpoint_payload(
        full_state_digest,
        status,
        counters,
        codec,
        CHECKPOINT_DOMAIN_V4,
        CHECKPOINT_INPUT_SCHEMA_V4,
    )?;
    let bytes = cbor::encode_canonical(&payload)?;
    let envelope =
        envelope::encode_envelope(CHECKPOINT_DOMAIN_V4, CHECKPOINT_INPUT_SCHEMA_V4, &bytes)?;
    Ok(CheckpointDigestV4::from_digest_bytes(
        envelope::hash_envelope(&envelope),
    ))
}

fn checkpoint_payload(
    full_state_digest: &DigestReferenceV1,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
    domain: &str,
    input_schema: &str,
) -> Result<cbor::Value, PersistenceDecodeErrorV1> {
    Ok(cbor::Value::Array(vec![
        cbor::Value::Text(input_schema.to_owned()),
        cbor::Value::Text(domain.to_owned()),
        envelope::digest_reference_value(full_state_digest),
        episode_status_value(status)?,
        counters_value(counters),
        cbor::Value::Array(vec![
            cbor::Value::Text(codec.codec_id.clone()),
            cbor::Value::Text(codec.semantic_version.clone()),
        ]),
    ]))
}

fn episode_status_value(status: &EpisodeStatus) -> Result<cbor::Value, PersistenceDecodeErrorV1> {
    match status {
        EpisodeStatus::Running => Ok(variant("running", cbor::Value::Null)),
        EpisodeStatus::Terminal { reason, players } => Ok(variant(
            "terminal",
            cbor::Value::Array(vec![
                cbor::Value::Text(terminal_reason(*reason).to_owned()),
                player_outcomes_value(players)?,
            ]),
        )),
        EpisodeStatus::Truncated { reason, players } => Ok(variant(
            "truncated",
            cbor::Value::Array(vec![
                cbor::Value::Text(truncation_reason(*reason).to_owned()),
                player_outcomes_value(players)?,
            ]),
        )),
    }
}

fn player_outcomes_value(
    players: &[PlayerOutcome],
) -> Result<cbor::Value, PersistenceDecodeErrorV1> {
    let mut players = players.to_vec();
    players.sort_by_key(|outcome| outcome.player);
    let values = players
        .into_iter()
        .map(|outcome| {
            cbor::Value::Array(vec![
                cbor::Value::Unsigned(outcome.player.0),
                cbor::Value::Text(player_result(outcome.result).to_owned()),
            ])
        })
        .collect();
    Ok(cbor::Value::Array(values))
}

fn counters_value(counters: &EnvironmentLimitCounters) -> cbor::Value {
    cbor::Value::Array(vec![
        cbor::Value::Unsigned(counters.decisions_submitted),
        cbor::Value::Unsigned(counters.accepted_transitions),
        cbor::Value::Unsigned(counters.rule_events_emitted),
        cbor::Value::Unsigned(counters.resource_units_consumed),
        cbor::Value::Unsigned(counters.wall_clock_elapsed_millis),
    ])
}

fn variant(name: &str, payload: cbor::Value) -> cbor::Value {
    cbor::Value::Array(vec![cbor::Value::Text(name.to_owned()), payload])
}

fn terminal_reason(reason: TerminalReason) -> &'static str {
    match reason {
        TerminalReason::RulesLoss => "rules_loss",
        TerminalReason::Concession => "concession",
        TerminalReason::SimultaneousOutcome => "simultaneous_outcome",
        TerminalReason::RulesDraw => "rules_draw",
        TerminalReason::SpecifiedLoop => "specified_loop",
    }
}

fn truncation_reason(reason: TruncationReason) -> &'static str {
    match reason {
        TruncationReason::DecisionLimit => "decision_limit",
        TruncationReason::RuleEventLimit => "rule_event_limit",
        TruncationReason::WallClockLimit => "wall_clock_limit",
        TruncationReason::ResourceLimit => "resource_limit",
        TruncationReason::ExternalStop => "external_stop",
    }
}

fn player_result(result: PlayerResult) -> &'static str {
    match result {
        PlayerResult::Win => "win",
        PlayerResult::Loss => "loss",
        PlayerResult::Draw => "draw",
        PlayerResult::Eliminated => "eliminated",
        PlayerResult::Unresolved => "unresolved",
    }
}
