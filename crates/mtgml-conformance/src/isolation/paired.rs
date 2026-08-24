//! Runtime-acceptance pair builder for paired-state isolation evidence.
//!
//! Every constructed state passes `validate_engine_state` and is then
//! accepted into a live synthetic environment before it can appear inside a
//! `PairedCase`; a state the runtime cannot accept never becomes evidence.

use mtgml_environment::{
    EnvironmentCheckpointV3, PlayerEndpointHandle, SyntheticM1EnvironmentBackend,
    SyntheticM1EnvironmentConfig, SyntheticM1ReplayConfig, TrustedEnvironmentController,
};
use mtgml_model::{
    CheckpointCodecIdentity, ContentDigest, EnvironmentLimitCounters, EpisodeStatus, GameObjectId,
    PlayerId,
};
use mtgml_observation::{
    INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};
use mtgml_random::RootSeed256;
use mtgml_replay::{DeckIdentityV1, KernelIdentityV1, ReplaySchemaVersionsV1};
use mtgml_state::{
    construct_synthetic_engine_state, validate_engine_state, EngineState, SyntheticResetInputs,
};

use super::witnesses::PairWitness;
use super::HarnessError;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

/// The established harness fixture identity of the one hidden object that
/// the shared rename transform relocates.
const RENAMED_FROM: GameObjectId = GameObjectId(2);
const RENAMED_TO: GameObjectId = GameObjectId(9);

/// The declared information-isolation axes of milestone M2.G.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisKind {
    OpponentHiddenDefinition,
    HiddenConcealedOrdering,
    ForeignPrivateLook,
    FaceDownIdentity,
    RootSeedPreAuth,
    HiddenRngCursor,
    ObjectRenaming,
    AbilityRenaming,
    GlobalAllocatorHistory,
    ForeignKnowledgeHistory,
}

/// One paired-state case: two runtime-accepted states plus the witness that
/// authorizes their differences from `perspective`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedCase {
    pub name: &'static str,
    pub axis: AxisKind,
    pub perspective: PlayerId,
    pub state_a: EngineState,
    pub state_b: EngineState,
    pub witness: PairWitness,
}

/// Declared mutation report of a conformance-only transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransformReport {
    /// The declared mutated top-level field list.
    pub mutated_fields: &'static [&'static str],
}

/// Conformance-only mutation helper over a cloned authoritative state.
///
/// Transforms fail closed: a declared fixture object that is absent aborts
/// the case build instead of panicking, so an out-of-date enrichment can
/// never silently produce evidence from an unintended state.
pub type TransformFn = fn(&mut EngineState) -> Result<TransformReport, HarnessError>;

fn codec_identity() -> CheckpointCodecIdentity {
    CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    }
}

/// The established per-consumer synthetic environment configuration
/// (duplication-per-consumer, as in `legal_space`).
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

/// Accepts one authoritative state into a live synthetic environment with
/// both players bound. Digests are computed by checkpoint creation and the
/// full backend validation runs before any endpoint exists.
pub fn spawn_environment(
    state: EngineState,
    config: &SyntheticM1EnvironmentConfig,
) -> Result<(TrustedEnvironmentController, [PlayerEndpointHandle; 2]), HarnessError> {
    let counters = EnvironmentLimitCounters::default();
    let checkpoint =
        EnvironmentCheckpointV3::new(state, EpisodeStatus::Running, counters, codec_identity())
            .map_err(|_| HarnessError::CheckpointInvalid)?;
    checkpoint
        .validate()
        .map_err(|_| HarnessError::CheckpointInvalid)?;
    let backend = SyntheticM1EnvironmentBackend::from_checkpoint(checkpoint, config.clone())
        .map_err(|_| HarnessError::SyntheticBackendRejected)?;
    let controller = TrustedEnvironmentController::new(backend);
    let p1 = controller
        .bind_player(P1)
        .map_err(|_| HarnessError::BindFailed)?;
    let p2 = controller
        .bind_player(P2)
        .map_err(|_| HarnessError::BindFailed)?;
    Ok((controller, [p1, p2]))
}

/// Builds a paired case: clone base, apply each transform, validate both
/// sides, require runtime acceptance of both sides, and attach the witness.
pub fn build_case(
    name: &'static str,
    axis: AxisKind,
    base: &EngineState,
    transform_a: TransformFn,
    transform_b: TransformFn,
    witness: PairWitness,
) -> Result<PairedCase, HarnessError> {
    let mut state_a = base.clone();
    transform_a(&mut state_a)?;
    validate_engine_state(&state_a).map_err(HarnessError::StateValidation)?;
    let mut state_b = base.clone();
    transform_b(&mut state_b)?;
    validate_engine_state(&state_b).map_err(HarnessError::StateValidation)?;
    let config = synthetic_environment_config([P1, P2]);
    spawn_environment(state_a.clone(), &config)?;
    spawn_environment(state_b.clone(), &config)?;
    Ok(PairedCase {
        name,
        axis,
        perspective: witness.perspective,
        state_a,
        state_b,
        witness,
    })
}

/// Deterministic two-player base state from a lowercase hex root seed.
pub fn base_pair_state(seed_hex: &str) -> Result<EngineState, HarnessError> {
    let root_seed = RootSeed256::from_lower_hex(seed_hex).map_err(|_| HarnessError::SeedFormat)?;
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [P1, P2],
        root_seed,
    })
    .map_err(|_| HarnessError::SyntheticConstruction)
}

/// The single shared hidden-object rename transform: renames the one hidden
/// fixture object consistently across zones, ordered zones, and every
/// perspective identity record, raising the global object allocator to cover
/// the fresh identity. Fails closed when the declared fixture object is
/// absent or an allocator head would overflow; a failed application leaves no
/// half-applied rename behind.
pub(crate) fn rename_hidden_object(
    state: &mut EngineState,
) -> Result<TransformReport, HarnessError> {
    let Some(mut object) = state.zones.objects.remove(&RENAMED_FROM) else {
        return Err(HarnessError::TransformFixtureAbsent);
    };
    object.id = RENAMED_TO;
    state.zones.objects.insert(RENAMED_TO, object);
    let Some(location) = state.zones.locations.remove(&RENAMED_FROM) else {
        // Restore the removed object so the failure leaves no half-applied
        // transform behind.
        let Some(mut restored) = state.zones.objects.remove(&RENAMED_TO) else {
            return Err(HarnessError::TransformFixtureAbsent);
        };
        restored.id = RENAMED_FROM;
        state.zones.objects.insert(RENAMED_FROM, restored);
        return Err(HarnessError::TransformFixtureAbsent);
    };
    let key = location.key();
    state.zones.locations.insert(RENAMED_TO, location);
    if let Some(members) = state.zones.ordered_zones.get_mut(&key) {
        for member in members.iter_mut() {
            if *member == RENAMED_FROM {
                *member = RENAMED_TO;
            }
        }
    }
    for identity in state.perspective_identities.players.values_mut() {
        for target in identity.opaque_to_object.values_mut() {
            if *target == RENAMED_FROM {
                *target = RENAMED_TO;
            }
        }
        if let Some(opaque) = identity.object_to_opaque.remove(&RENAMED_FROM) {
            identity.object_to_opaque.insert(RENAMED_TO, opaque);
        }
    }
    state.allocators.next_object_id = GameObjectId(
        RENAMED_TO
            .0
            .checked_add(1)
            .ok_or(HarnessError::TransformPreconditionViolated)?,
    );
    Ok(TransformReport {
        mutated_fields: &["zones", "allocators", "perspective_identities"],
    })
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::isolation::witnesses::TrustedRenamingBijection;
    use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};
    use mtgml_environment::PlayerEndpoint;
    use mtgml_model::{
        InformationStateDigestV2, ObservationDigest, StateRevision, VisibleSequence,
    };
    use mtgml_observation::{
        ObservationEnvelope, PlayerInformationStateV2, PlayerStepSubmissionV1, PlayerStepV2,
        PlayerSubmissionCodeV1, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA,
        PLAYER_STEP_SCHEMA_V2,
    };
    use std::collections::BTreeMap;

    pub const RENAMED_FROM: GameObjectId = super::RENAMED_FROM;
    pub const RENAMED_TO: GameObjectId = super::RENAMED_TO;

    pub(crate) use super::rename_hidden_object;

    /// One spawned runtime-accepted environment pair of a `PairedCase`.
    pub type SpawnedPair = [(TrustedEnvironmentController, [PlayerEndpointHandle; 2]); 2];

    /// Spawns both sides of a paired case as independent live environments.
    pub fn spawn_pair(case: &PairedCase) -> Result<SpawnedPair, HarnessError> {
        let config = synthetic_environment_config([P1, P2]);
        Ok([
            spawn_environment(case.state_a.clone(), &config)?,
            spawn_environment(case.state_b.clone(), &config)?,
        ])
    }

    /// Finds the endpoint handle bound to `perspective`; fails closed when
    /// no such binding exists.
    pub fn endpoint_for(
        endpoints: &[PlayerEndpointHandle; 2],
        perspective: PlayerId,
    ) -> Result<&PlayerEndpointHandle, HarnessError> {
        endpoints
            .iter()
            .find(|endpoint| endpoint.perspective() == perspective)
            .ok_or(HarnessError::BindFailed)
    }

    /// Drives ONE accepted entry-stage submission through the real actor
    /// endpoint (candidate id read from the live `visible_decision()`).
    pub fn accepted_entry_submission(
        handle: &PlayerEndpointHandle,
    ) -> Result<PlayerStepV2, HarnessError> {
        let request = handle
            .visible_decision()
            .map_err(|_| HarnessError::EndpointService)?
            .ok_or(HarnessError::EndpointService)?;
        let candidate = request
            .candidates
            .first()
            .map(|candidate| candidate.candidate_id)
            .ok_or(HarnessError::EndpointService)?;
        let step = handle
            .submit(DecisionResponseV2 {
                schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
                player_decision_id: request.player_decision_id,
                state_revision: request.state_revision,
                answer: DecisionAnswerV2::SelectOne {
                    candidate_id: candidate,
                },
            })
            .map_err(|_| HarnessError::EndpointService)?;
        step.validate().map_err(|_| HarnessError::WireEncoding)?;
        Ok(step)
    }

    pub fn renaming_bijection() -> TrustedRenamingBijection {
        TrustedRenamingBijection {
            objects: BTreeMap::from([(RENAMED_FROM, RENAMED_TO)]),
            abilities: BTreeMap::new(),
        }
    }

    /// A typed-rejection submit result carrying a fully valid mirrored step.
    pub fn rejected_submit_result() -> Result<PlayerStepV2, mtgml_environment::PlayerEndpointError>
    {
        let observation = ObservationEnvelope {
            schema_version: OBSERVATION_SCHEMA.into(),
            perspective: P1,
            state_revision: StateRevision(0),
            payload_codec: "synthetic-m2-observation.v1".into(),
            payload_base64: "e30=".into(),
            digest: ObservationDigest::from_canonical_bytes(b"{}"),
        };
        let mut information_state = PlayerInformationStateV2 {
            schema_version: INFORMATION_STATE_SCHEMA_V2.into(),
            perspective: P1,
            state_revision: StateRevision(0),
            current_observation: observation,
            next_visible_sequence: VisibleSequence(1),
            retained_knowledge: Vec::new(),
            digest: InformationStateDigestV2::from_canonical_bytes(b"placeholder"),
        };
        let (_, digest) =
            mtgml_wire::compute_information_state_digest_v2(&information_state.digest_input())
                .unwrap();
        information_state.digest = digest;
        Ok(PlayerStepV2 {
            schema_version: PLAYER_STEP_SCHEMA_V2.into(),
            information_state,
            observed_events: Vec::new(),
            next_decision: None,
            status: EpisodeStatus::Running,
            submission: PlayerStepSubmissionV1::Rejected {
                code: PlayerSubmissionCodeV1::InvalidAnswer,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{rename_hidden_object, renaming_bijection, RENAMED_FROM, RENAMED_TO};
    use super::*;
    use crate::isolation::witnesses::{assert_witness, NonVacuityPredicate};

    #[test]
    fn runtime_acceptance_rejects_invalid_state() {
        let mut corrupted = base_pair_state(&"11".repeat(32)).unwrap();
        corrupted.allocators.next_object_id = mtgml_model::GameObjectId(1);
        let outcome = spawn_environment(corrupted, &synthetic_environment_config([P1, P2]));
        assert!(matches!(outcome, Err(HarnessError::CheckpointInvalid)));
    }

    #[test]
    fn build_case_runtime_accepts_both_sides_and_preserves_witness() {
        let base = base_pair_state(&"11".repeat(32)).unwrap();
        let mut expected_b = base.clone();
        rename_hidden_object(&mut expected_b).unwrap();
        let witness = crate::isolation::witnesses::PairWitness::new(
            P2,
            Some(renaming_bijection()),
            NonVacuityPredicate::Required,
        );

        fn unchanged(_state: &mut EngineState) -> Result<TransformReport, HarnessError> {
            Ok(TransformReport {
                mutated_fields: &[],
            })
        }

        let case = build_case(
            "hidden_object_renaming",
            AxisKind::ObjectRenaming,
            &base,
            unchanged,
            rename_hidden_object,
            witness,
        )
        .unwrap();
        assert_eq!(case.name, "hidden_object_renaming");
        assert_eq!(case.axis, AxisKind::ObjectRenaming);
        assert_eq!(case.perspective, P2);
        assert_eq!(case.state_b, expected_b);
        assert_witness(&case.state_a, &case.state_b, &case.witness).unwrap();
    }

    #[test]
    fn missing_fixture_object_fails_closed() {
        // A base state without the declared hidden fixture object must abort
        // the transform instead of panicking.
        let mut stripped = base_pair_state(&"11".repeat(32)).unwrap();
        stripped.zones.objects.remove(&RENAMED_FROM);
        assert_eq!(
            rename_hidden_object(&mut stripped),
            Err(HarnessError::TransformFixtureAbsent)
        );
        // The failed transform left no half-applied rename behind.
        assert!(!stripped.zones.objects.contains_key(&RENAMED_TO));
    }

    #[test]
    fn base_pair_state_is_deterministic_per_seed() {
        let seed_hex = "11".repeat(32);
        let first = base_pair_state(&seed_hex).unwrap();
        let second = base_pair_state(&seed_hex).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.digest().unwrap(), second.digest().unwrap());
    }
}
