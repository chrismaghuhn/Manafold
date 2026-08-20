use mtgml_decision::DecisionResponse;
use mtgml_model::{ContentDigest, FullStateDigest, PlayerId, StateRevision};

use crate::{
    AuthoritativeReplayV1, DeckIdentityV1, KernelIdentityV1, RandomnessIdentityV1,
    ReplayManifestV1, ReplaySchemaVersionsV1, ReplayStepV1, ReplayValidationError,
    REPLAY_FILE_SCHEMA, REPLAY_MANIFEST_SCHEMA,
};

fn digest(text: char) -> FullStateDigest {
    FullStateDigest::parse(text.to_string().repeat(64)).unwrap()
}

fn manifest() -> ReplayManifestV1 {
    ReplayManifestV1 {
        schema_version: REPLAY_MANIFEST_SCHEMA.into(),
        engine_build: "build".into(),
        kernel: KernelIdentityV1 {
            implementation_id: "reference".into(),
            semantic_version: "0.2.2".into(),
            build_profile: "test".into(),
        },
        rules_snapshot: "rules".into(),
        format_policy_snapshot: "format".into(),
        oracle_snapshot: "oracle".into(),
        card_bundle: "bundle".into(),
        schemas: ReplaySchemaVersionsV1 {
            observation: "observation-envelope.v1".into(),
            information_state: "information-state-envelope.v1".into(),
            decision: "player-decision-request.v1".into(),
            decision_response: "decision-response.v1".into(),
            observed_event: "observed-event-envelope.v1".into(),
            player_step: "player-step.v1".into(),
            replay_step: "replay-step.v1".into(),
        },
        randomness: RandomnessIdentityV1 {
            algorithm_id: "counter".into(),
            derivation_version: "v1".into(),
            root_seed_hex: "00".repeat(32),
        },
        decks: vec![DeckIdentityV1 {
            player: PlayerId(1),
            deck_id: "deck".into(),
            digest: ContentDigest::parse("11".repeat(32)).unwrap(),
        }],
        initial_state_revision: StateRevision(0),
        initial_state_digest: digest('0'),
    }
}

#[test]
fn replay_schema_version_fields_must_all_be_non_empty() {
    let mut invalid = manifest();
    invalid.schemas.observed_event.clear();
    assert_eq!(
        invalid.validate(),
        Err(ReplayValidationError::EmptyIdentity)
    );
}

#[test]
fn rejected_replay_step_must_preserve_the_full_state_digest() {
    let mut replay = AuthoritativeReplayV1 {
        schema_version: REPLAY_FILE_SCHEMA.into(),
        manifest: manifest(),
        steps: vec![ReplayStepV1 {
            step_index: 0,
            state_revision_before: StateRevision(0),
            response: DecisionResponse {
                schema_version: "decision-response.v1".into(),
                decision_id: mtgml_model::DecisionId(1),
                state_revision: StateRevision(0),
                assignments: vec![],
            },
            accepted: false,
            state_revision_after: StateRevision(0),
            state_digest_after: digest('1'),
        }],
        final_state_revision: StateRevision(0),
        final_state_digest: digest('1'),
    };
    assert_eq!(
        replay.validate(),
        Err(ReplayValidationError::RejectedMutation)
    );
    replay.steps[0].state_digest_after = digest('0');
    replay.final_state_digest = digest('0');
    replay.validate().unwrap();
}
