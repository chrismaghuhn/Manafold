use mtgml_decision::{CandidateAssignment, DecisionResponse, DECISION_RESPONSE_SCHEMA};
use mtgml_model::{
    ContentDigest, DecisionId, FullStateDigest, FullStateDigestV2, PlayerId, StateRevision,
};

use crate::{
    AuthoritativeReplayV1, DeckIdentityV1, KernelIdentityV1, RandomnessIdentityV1,
    RandomnessIdentityV2, ReplayManifestV1, ReplayManifestV2,
    ReplayRecorderV2, ReplaySchemaVersionsV1, ReplayStepV1, ReplayStepV2,
    ReplayValidationError, REPLAY_FILE_SCHEMA, REPLAY_FILE_SCHEMA_V2, REPLAY_MANIFEST_SCHEMA,
    REPLAY_MANIFEST_SCHEMA_V2,
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

fn digest_v2(text: char) -> FullStateDigestV2 {
    FullStateDigestV2::parse(text.to_string().repeat(64)).unwrap()
}

fn manifest_v2() -> ReplayManifestV2 {
    ReplayManifestV2 {
        schema_version: REPLAY_MANIFEST_SCHEMA_V2.into(),
        engine_build: "synthetic-build".into(),
        kernel: KernelIdentityV1 {
            implementation_id: "synthetic-m1".into(),
            semantic_version: "0.2.2".into(),
            build_profile: "test".into(),
        },
        rules_snapshot: "synthetic-rules".into(),
        format_policy_snapshot: "synthetic-format".into(),
        oracle_snapshot: "synthetic-oracle".into(),
        card_bundle: "synthetic-bundle".into(),
        schemas: ReplaySchemaVersionsV1 {
            observation: "observation-envelope.v1".into(),
            information_state: "information-state-envelope.v1".into(),
            decision: "player-decision-request.v1".into(),
            decision_response: DECISION_RESPONSE_SCHEMA.into(),
            observed_event: "observed-event-envelope.v1".into(),
            player_step: "player-step.v1".into(),
            replay_step: "replay-step.v2".into(),
        },
        randomness: RandomnessIdentityV2 {
            contract_id: "mtgml.rng.v1".into(),
            root_seed_hex: "00".repeat(32),
        },
        decks: vec![DeckIdentityV1 {
            player: PlayerId(1),
            deck_id: "synthetic-deck-1".into(),
            digest: ContentDigest::parse("11".repeat(32)).unwrap(),
        }],
        initial_state_revision: StateRevision(0),
        initial_state_digest: digest_v2('0'),
    }
}

fn response_v2(revision: u64) -> DecisionResponse {
    DecisionResponse {
        schema_version: DECISION_RESPONSE_SCHEMA.into(),
        decision_id: DecisionId(1),
        state_revision: StateRevision(revision),
        assignments: vec![CandidateAssignment {
            candidate_id: "select_public_object".into(),
            ordinal: None,
        }],
    }
}

fn accepted_step_v2() -> ReplayStepV2 {
    ReplayStepV2 {
        step_index: 0,
        state_revision_before: StateRevision(0),
        response: response_v2(0),
        accepted: true,
        state_revision_after: StateRevision(1),
        state_digest_after: digest_v2('1'),
    }
}

#[test]
fn replay_recorder_starts_empty_segment_at_manifest_identity() {
    let manifest = manifest_v2();
    let recorder = ReplayRecorderV2::new(manifest.clone()).unwrap();

    assert_eq!(recorder.step_count(), 0);
    assert_eq!(recorder.manifest(), &manifest);
    let replay = recorder.export().unwrap();

    assert_eq!(replay.schema_version, REPLAY_FILE_SCHEMA_V2);
    assert_eq!(replay.manifest, manifest);
    assert!(replay.steps.is_empty());
    assert_eq!(
        replay.final_state_revision,
        StateRevision(0),
        "an empty segment remains at its checkpoint revision"
    );
    assert_eq!(replay.final_state_digest, digest_v2('0'));
    replay.validate().unwrap();
}

#[test]
fn replay_recorder_appends_exact_accepted_step() {
    let mut recorder = ReplayRecorderV2::new(manifest_v2()).unwrap();
    recorder.append(accepted_step_v2()).unwrap();

    let replay = recorder.export().unwrap();
    assert_eq!(recorder.step_count(), 1);
    assert_eq!(replay.steps, vec![accepted_step_v2()]);
    assert_eq!(replay.final_state_revision, StateRevision(1));
    assert_eq!(replay.final_state_digest, digest_v2('1'));
    replay.validate().unwrap();
}

#[test]
fn replay_recorder_rejects_invalid_append_without_mutation() {
    let mut recorder = ReplayRecorderV2::new(manifest_v2()).unwrap();
    let before = recorder.export().unwrap();
    let mut invalid = accepted_step_v2();
    invalid.step_index = 1;

    assert_eq!(
        recorder.append(invalid),
        Err(ReplayValidationError::RevisionDiscontinuity)
    );
    assert_eq!(recorder.export().unwrap(), before);
}

#[test]
fn replay_recorder_keeps_rejected_diagnostic_identity() {
    let mut recorder = ReplayRecorderV2::new(manifest_v2()).unwrap();
    let mut step = accepted_step_v2();
    step.accepted = false;
    step.state_revision_after = StateRevision(0);
    step.state_digest_after = digest_v2('0');

    recorder.append(step.clone()).unwrap();
    let replay = recorder.export().unwrap();

    assert_eq!(replay.steps, vec![step]);
    assert_eq!(replay.final_state_revision, StateRevision(0));
    assert_eq!(replay.final_state_digest, digest_v2('0'));
    replay.validate().unwrap();
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
