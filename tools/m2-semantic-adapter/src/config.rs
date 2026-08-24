//! Adapter-local synthetic environment configuration identity, copied
//! verbatim (values and structure) from the established M2.G isolation
//! fixture constructor
//! (`crates/mtgml-conformance/src/isolation/paired.rs::synthetic_environment_config`)
//! so this tool never depends on a harness crate. Duplication-per-consumer
//! is that constructor's own declared pattern.
//!
//! Only configuration identity types are referenced here; no replay or
//! privileged operation is performed by this tool.

use mtgml_environment::{
    CheckpointCodecIdentity, SyntheticM1EnvironmentConfig, SyntheticM1ReplayConfig,
};
use mtgml_model::{ContentDigest, PlayerId};
use mtgml_observation::{
    INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};
use mtgml_replay::{DeckIdentityV1, KernelIdentityV1, ReplaySchemaVersionsV1};

fn codec_identity() -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    }
}

pub fn synthetic_environment_config(players: [PlayerId; 2]) -> SyntheticM1EnvironmentConfig {
    SyntheticM1EnvironmentConfig {
        codec: codec_identity(),
        replay: SyntheticM1ReplayConfig {
            engine_build: "synthetic-build".into(),
            kernel: KernelIdentityV1 {
                implementation_id: "synthetic-m2".into(),
                semantic_version: "0.2.2".into(),
                build_profile: "test".into(),
            },
            rules_snapshot: "synthetic-rules".into(),
            format_policy_snapshot: "synthetic-format".into(),
            oracle_snapshot: "synthetic-oracle".into(),
            card_bundle: "synthetic-bundle".into(),
            randomness_contract_id: "mtgml.rng.v1".into(),
            schemas: ReplaySchemaVersionsV1 {
                observation: OBSERVATION_SCHEMA.into(),
                information_state: INFORMATION_STATE_SCHEMA_V2.into(),
                decision: "player-decision-request.v2".into(),
                decision_response: "decision-response.v2".into(),
                observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
                player_step: PLAYER_STEP_SCHEMA_V2.into(),
                replay_step: "replay-step.v3".into(),
            },
            decks: players
                .into_iter()
                .enumerate()
                .map(|(index, player)| DeckIdentityV1 {
                    player,
                    deck_id: format!("synthetic-deck-{index}"),
                    digest: ContentDigest::from_canonical_bytes(
                        format!("synthetic-deck-{index}").as_bytes(),
                    ),
                })
                .collect(),
        },
    }
}
