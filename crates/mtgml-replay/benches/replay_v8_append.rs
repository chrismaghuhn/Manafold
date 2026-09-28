use std::hint::black_box;
use std::time::Instant;

use mtgml_decision::{DecisionAnswerV2, DecisionResponseV3, DECISION_RESPONSE_V3_SCHEMA};
use mtgml_model::{FullStateDigestV7, StateRevision};
use mtgml_replay::{
    InitialEnvironmentIdentityV8, ReplayManifestV8, ReplayRecorderV8, ReplayStepV8,
};

fn run_append_count(count: u64) -> std::time::Duration {
    let manifest: ReplayManifestV8 = serde_json::from_str(include_str!(
        "../../../schemas/examples/replay-manifest-v8.json"
    ))
    .unwrap();
    let mut recorder = ReplayRecorderV8::new(manifest.clone()).unwrap();
    let mut current = manifest.initial_identity.clone();
    let actor = manifest.decks[0].player;
    let started = Instant::now();
    for index in 0..count {
        let mut counters = current.environment_limit_counters.clone();
        counters.decisions_submitted += 1;
        counters.accepted_transitions += 1;
        let revision = StateRevision(current.state_revision.0 + 1);
        let digest = FullStateDigestV7::from_digest_bytes([index as u8; 32]);
        let checkpoint_digest =
            mtgml_persistence::checkpoint_digest::calculate_checkpoint_digest_v8(
                &digest.as_digest_reference(),
                &current.episode_status,
                &counters,
                &current.checkpoint_codec_identity,
                &current.execution_identity,
            )
            .unwrap();
        let step = ReplayStepV8 {
            step_index: index,
            actor,
            checkpoint_digest_before: current.checkpoint_digest.clone(),
            state_revision_before: current.state_revision,
            response: DecisionResponseV3 {
                schema_version: DECISION_RESPONSE_V3_SCHEMA.to_owned(),
                player_decision_id: mtgml_model::PlayerDecisionIdV1(index + 1),
                view_sequence: mtgml_model::VisibleSequence(index + 1),
                answer: DecisionAnswerV2::SelectOne {
                    candidate_id: mtgml_model::CandidateIdV1(0),
                },
            },
            accepted: true,
            state_revision_after: revision,
            full_state_digest_after: digest.clone(),
            episode_status_after: current.episode_status.clone(),
            environment_limit_counters_after: counters.clone(),
            checkpoint_digest_after: checkpoint_digest.clone(),
        };
        recorder.append(black_box(step)).unwrap();
        current = InitialEnvironmentIdentityV8 {
            state_revision: revision,
            full_state_digest: digest,
            episode_status: current.episode_status,
            environment_limit_counters: counters,
            checkpoint_codec_identity: current.checkpoint_codec_identity,
            checkpoint_digest,
            execution_identity: current.execution_identity,
        };
    }
    black_box(recorder);
    started.elapsed()
}

fn main() {
    for count in [100, 1_000, 10_000] {
        let elapsed = run_append_count(count);
        println!("append_count={count} elapsed_ns={}", elapsed.as_nanos());
    }
}
