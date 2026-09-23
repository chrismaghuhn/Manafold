use super::*;
use crate::semantic_catalog_generated::synthetic_legacy_default_semantic_contract_id;

use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};

use mtgml_model::{
    CandidateIdV1, CheckpointDigestV6, ContentDigest, ContinuationId, EpisodeStatus,
    ExecutionIdentityV1, ExecutionProgramV1, FullStateDigestV5, PlayerDecisionIdV1, PlayerId,
    PlayerOutcome, PlayerResult, StateRevision, TerminalReason, TruncationReason,
};

use mtgml_observation::{
    PlayerStepV2, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};

use mtgml_random::RootSeed256;

use mtgml_replay::{
    AuthoritativeReplayV6, DeckIdentityV1, KernelIdentityV1, ReplaySchemaVersionsV6,
};

fn config(players: [PlayerId; 2]) -> SyntheticM1EnvironmentConfig {
    SyntheticM1EnvironmentConfig {
        codec: CheckpointCodecIdentity {
            codec_id: "in-memory-reference".into(),
            semantic_version: "6".into(),
        },
        setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
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
            schemas: ReplaySchemaVersionsV6 {
                observation: OBSERVATION_SCHEMA.into(),
                observation_payload_codec: "synthetic-m3-observation.v1".into(),
                information_state: INFORMATION_STATE_SCHEMA_V2.into(),
                decision: "player-decision-request.v2".into(),
                decision_response: DECISION_RESPONSE_V2_SCHEMA.into(),
                observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
                player_step: PLAYER_STEP_SCHEMA_V2.into(),
                replay_step: "replay-step.v6".into(),
            },
            decks: players
                .into_iter()
                .enumerate()
                .map(|(index, player)| DeckIdentityV1 {
                    player,
                    deck_id: format!("synthetic-deck-{}", index + 1),
                    digest: ContentDigest::from_canonical_bytes(
                        format!("synthetic-deck-{}", index + 1).as_bytes(),
                    ),
                })
                .collect(),
        },
    }
}

fn seed() -> RootSeed256 {
    RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap()
}

fn response(candidate_id: u32, revision: u64) -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(revision),
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(candidate_id),
        },
    }
}

fn synthetic_identity() -> ExecutionIdentityV1 {
    ExecutionIdentityV1 {
        program_kind: ExecutionProgramV1::SyntheticRulesCompat,
        semantic_contract_id: synthetic_legacy_default_semantic_contract_id(),
    }
}

fn backend() -> SyntheticM1EnvironmentBackend {
    let players = [PlayerId(1), PlayerId(2)];
    SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap()
}

fn rich_provenance_state() -> mtgml_state::EngineState {
    use mtgml_model::VisibleSequence;
    use mtgml_state::{
        KnowledgeAcquisitionCause, KnowledgeAcquisitionReason, KnowledgeHistoryChannel,
        KnowledgeInvalidationReason, KnowledgeInvalidationV2, KnownLocationFactV2,
        RetiredKnowledgeRecordV2,
    };
    let observed = |channel, sequence: u64, cause| KnowledgeAcquisitionReason::Observed {
        channel,
        sequence: VisibleSequence(sequence),
        cause,
    };
    let mut state =
        mtgml_state::construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: seed(),
            setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
        })
        .unwrap();

    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity
        .opaque_to_object
        .insert(mtgml_model::OpaqueObjectId(3), mtgml_model::GameObjectId(2));
    identity
        .object_to_opaque
        .insert(mtgml_model::GameObjectId(2), mtgml_model::OpaqueObjectId(3));
    identity.next_opaque_object_id = mtgml_model::OpaqueObjectId(4);
    identity
        .retired_object_ids
        .insert(mtgml_model::OpaqueObjectId(2));

    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let hidden_location = mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Library,
        player: Some(PlayerId(2)),
        position: mtgml_state::ZonePosition::Top { offset: 0 },
        visibility: mtgml_state::VisibilityPartition::FaceDown,
        partition: None,
    };

    // Retired record: private_look acquisition, own_private_identity history,
    // explicit_reveal invalidation.
    let mut retired = RetiredKnowledgeRecordV2 {
        opaque_object: mtgml_model::OpaqueObjectId(2),
        physical_card: None,
        card_definition: None,
        last_known_location: Some(KnownLocationFactV2 {
            location: hidden_location.clone(),
            provenance: observed(
                KnowledgeHistoryChannel::Private,
                2,
                KnowledgeAcquisitionCause::PrivateLook,
            ),
        }),
        historical_locations: vec![KnownLocationFactV2 {
            location: hidden_location.clone(),
            provenance: observed(
                KnowledgeHistoryChannel::Private,
                1,
                KnowledgeAcquisitionCause::OwnPrivateIdentity,
            ),
        }],
        acquisition: observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
        invalidation: KnowledgeInvalidationV2 {
            provenance: observed(
                KnowledgeHistoryChannel::Public,
                3,
                KnowledgeAcquisitionCause::ExplicitReveal,
            ),
            reason: KnowledgeInvalidationReason::Shuffle,
        },
    };
    retired.last_known_location = Some(KnownLocationFactV2 {
        location: hidden_location.clone(),
        provenance: observed(
            KnowledgeHistoryChannel::Private,
            2,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
    });
    knowledge
        .retired
        .insert(mtgml_model::OpaqueObjectId(2), retired);
    knowledge.active.remove(&mtgml_model::OpaqueObjectId(2));
    knowledge.next_visible_sequence = VisibleSequence(4);

    // Active record with explicit_reveal current-fact provenance.
    knowledge.active.insert(
        mtgml_model::OpaqueObjectId(3),
        mtgml_state::KnowledgeRecordV2 {
            opaque_object: mtgml_model::OpaqueObjectId(3),
            physical_card: None,
            card_definition: Some(mtgml_model::CardDefinitionId(2)),
            known_location: Some(KnownLocationFactV2 {
                location: hidden_location,
                provenance: observed(
                    KnowledgeHistoryChannel::Public,
                    0,
                    KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
            acquisition: observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::ExplicitReveal,
            ),
            historical_locations: Vec::new(),
        },
    );
    state
}

fn expected_retained_knowledge() -> Vec<mtgml_observation::PlayerKnownObjectV1> {
    use mtgml_model::{CardDefinitionId, OpaqueObjectId, PlayerId, VisibleSequence, ZoneKind};
    use mtgml_observation::{
        PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1, PlayerKnowledgeInvalidationReasonV1,
        PlayerKnowledgeInvalidationV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
        PlayerKnownLocationV1, PlayerKnownObjectV1,
    };

    let initial = || PlayerKnowledgeProvenanceV1::InitialConfiguration;
    let observed = |channel, sequence, cause| PlayerKnowledgeProvenanceV1::Observed {
        channel,
        sequence: VisibleSequence(sequence),
        cause,
    };
    let battlefield = || PlayerKnownLocationV1 {
        zone: ZoneKind::Battlefield,
        player: None,
    };
    let hidden_library = || PlayerKnownLocationV1 {
        zone: ZoneKind::Library,
        player: Some(PlayerId(2)),
    };
    let fact = |location, provenance| PlayerKnownLocationFactV1 {
        location,
        provenance,
    };

    vec![
        PlayerKnownObjectV1::Active {
            opaque_object_id: OpaqueObjectId(1),
            known_definition: Some(CardDefinitionId(1)),
            current_known_location_fact: Some(fact(battlefield(), initial())),
            historical_locations: Vec::new(),
            acquisition: initial(),
        },
        PlayerKnownObjectV1::Retired {
            opaque_object_id: OpaqueObjectId(2),
            known_definition: None,
            last_known_location_fact: Some(fact(
                hidden_library(),
                observed(
                    PlayerKnowledgeChannelV1::Private,
                    2,
                    PlayerKnowledgeCauseV1::PrivateLook,
                ),
            )),
            historical_locations: vec![fact(
                hidden_library(),
                observed(
                    PlayerKnowledgeChannelV1::Private,
                    1,
                    PlayerKnowledgeCauseV1::OwnPrivateIdentity,
                ),
            )],
            acquisition: observed(
                PlayerKnowledgeChannelV1::Private,
                0,
                PlayerKnowledgeCauseV1::PrivateLook,
            ),
            invalidation: PlayerKnowledgeInvalidationV1 {
                provenance: observed(
                    PlayerKnowledgeChannelV1::Public,
                    3,
                    PlayerKnowledgeCauseV1::ExplicitReveal,
                ),
                reason: PlayerKnowledgeInvalidationReasonV1::Shuffle,
            },
        },
        PlayerKnownObjectV1::Active {
            opaque_object_id: OpaqueObjectId(3),
            known_definition: Some(CardDefinitionId(2)),
            current_known_location_fact: Some(fact(
                hidden_library(),
                observed(
                    PlayerKnowledgeChannelV1::Public,
                    0,
                    PlayerKnowledgeCauseV1::ExplicitReveal,
                ),
            )),
            historical_locations: Vec::new(),
            acquisition: observed(
                PlayerKnowledgeChannelV1::Public,
                0,
                PlayerKnowledgeCauseV1::ExplicitReveal,
            ),
        },
    ]
}

fn submit_answer(
    endpoint: &PlayerEndpointHandle,
    answer: mtgml_decision::DecisionAnswerV2,
) -> PlayerStepV2 {
    let request = endpoint
        .visible_decision()
        .unwrap()
        .expect("a stage decision is visible");
    let step = endpoint
        .submit(mtgml_decision::DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: request.player_decision_id,
            state_revision: request.state_revision,
            answer,
        })
        .unwrap();
    step.validate().unwrap();
    step
}

fn number_answer(value: i64) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::ChooseNumber { value }
}

fn members_answer(ids: &[u32]) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::SelectMany {
        candidate_ids: ids.iter().copied().map(CandidateIdV1).collect(),
    }
}

fn order_answer(ids: &[u32]) -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::Order {
        candidate_ids: ids.iter().copied().map(CandidateIdV1).collect(),
    }
}

/// Drives entry + ChooseCount(2) so the environment sits at the nonterminal
/// ChooseMembers stage of continuation C(1).
fn environment_at_members_stage() -> TrustedEnvironmentController {
    let controller = TrustedEnvironmentController::new(backend());
    let p1 = controller.bind_player(PlayerId(1)).unwrap();
    let _ = submit_answer(&p1, order_entry_answer());
    let _ = submit_answer(&p1, number_answer(2));
    controller
}

fn order_entry_answer() -> mtgml_decision::DecisionAnswerV2 {
    mtgml_decision::DecisionAnswerV2::SelectOne {
        candidate_id: CandidateIdV1(0),
    }
}

fn public_fingerprint(controller: &TrustedEnvironmentController) -> Vec<u8> {
    let checkpoint = controller.checkpoint().unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&checkpoint.state_digest.raw_bytes());
    bytes.extend_from_slice(&checkpoint.checkpoint_digest.raw_bytes());
    bytes.extend(serde_json::to_vec(&controller.export_replay().unwrap()).unwrap());
    bytes
}

use mtgml_model::{GameObjectId, OpaqueObjectId, VisibleSequence};

use mtgml_rules::TransitionResult;

use mtgml_state::{construct_synthetic_engine_state, EngineState};

fn m2e_fixture() -> EngineState {
    use mtgml_state::{GameObject, VisibilityPartition, ZoneLocation, ZonePosition};
    let mut state = construct_synthetic_engine_state(mtgml_state::SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: seed(),
        setup: mtgml_state::SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap();
    let exile = ZoneLocation {
        zone: mtgml_model::ZoneKind::Exile,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    for index in 3..=4u64 {
        let object = GameObjectId(index);
        state.zones.objects.insert(
            object,
            GameObject {
                id: object,
                physical_card: Some(mtgml_model::PhysicalCardId(index)),
                card_definition: mtgml_model::CardDefinitionId(index),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: false,
                face_down: false,
            },
        );
        state.zones.locations.insert(object, exile.clone());
    }
    state.allocators.next_object_id = GameObjectId(5);
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    state
}

fn battlefield_location() -> mtgml_state::ZoneLocation {
    mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Battlefield,
        player: None,
        position: mtgml_state::ZonePosition::Unordered,
        visibility: mtgml_state::VisibilityPartition::Public,
        partition: None,
    }
}

fn hidden_hand(player: PlayerId) -> mtgml_state::ZoneLocation {
    mtgml_state::ZoneLocation {
        zone: mtgml_model::ZoneKind::Hand,
        player: Some(player),
        position: mtgml_state::ZonePosition::Unordered,
        visibility: mtgml_state::VisibilityPartition::OwnerOnly,
        partition: None,
    }
}

/// Reveal GO3 to P1 (opaque 2) and then track it through an incarnation
/// change into a hidden zone. Returns the product of the single transition.
fn tracked_incarnation_product() -> Result<(EngineState, TransitionResult), ()> {
    use mtgml_rules::fixture_support::{FixtureTransition, PlannedOccurrence};
    let before = m2e_fixture();
    let mut transition = FixtureTransition::start(&before).map_err(|_| ())?;
    let revealed = transition
        .move_object_incarnation(GameObjectId(3), battlefield_location())
        .map_err(|_| ())?;
    transition
        .apply_occurrence(PlannedOccurrence {
            lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                perspective: PlayerId(1),
                sequence: VisibleSequence(1),
                mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                    identity: mtgml_state::IdentityMutationV1::Allocate {
                        opaque: OpaqueObjectId(2),
                        object: revealed,
                    },
                    knowledge: Some(mtgml_state::KnowledgeMutationV1::Acquire {
                        opaque: OpaqueObjectId(2),
                        definition: Some(mtgml_model::CardDefinitionId(3)),
                        location: Some(battlefield_location()),
                        acquisition: mtgml_state::KnowledgeAcquisitionReason::Observed {
                            channel: mtgml_state::KnowledgeHistoryChannel::Public,
                            sequence: VisibleSequence(1),
                            cause: mtgml_state::KnowledgeAcquisitionCause::ExplicitReveal,
                        },
                    }),
                },
            },
            observation: mtgml_rules::PerspectiveObservationPolicyV1::Appeared {
                from_zone: mtgml_model::ZoneKind::Exile,
                to_zone: mtgml_model::ZoneKind::Battlefield,
                new_object: revealed,
            },
        })
        .map_err(|_| ())?;
    let hidden = transition
        .move_object_incarnation(revealed, hidden_hand(PlayerId(2)))
        .map_err(|_| ())?;
    transition
        .apply_occurrence(PlannedOccurrence {
            lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                perspective: PlayerId(1),
                sequence: VisibleSequence(2),
                mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                    identity: mtgml_state::IdentityMutationV1::Remap {
                        opaque: OpaqueObjectId(2),
                        from_object: revealed,
                        to_object: hidden,
                    },
                    knowledge: Some(mtgml_state::KnowledgeMutationV1::CurrentToHistory {
                        opaque: OpaqueObjectId(2),
                        observed_definition: Some(mtgml_model::CardDefinitionId(3)),
                    }),
                },
            },
            observation: mtgml_rules::PerspectiveObservationPolicyV1::MovedInSight {
                from_zone: mtgml_model::ZoneKind::Battlefield,
                to_zone: mtgml_model::ZoneKind::Hand,
                old_object: revealed,
                new_object: hidden,
                reveals_old: true,
                reveals_new: false,
            },
        })
        .map_err(|_| ())?;
    let result = transition.finish().map_err(|_| ())?;
    Ok((before, result))
}

fn two_perspective_outcome_product() -> (EngineState, TransitionResult) {
    use mtgml_rules::fixture_support::{FixtureTransition, PlannedOccurrence};

    let before = m2e_fixture();
    let mut transition = FixtureTransition::start(&before).unwrap();
    for (perspective, code) in [(PlayerId(1), "p1-outcome"), (PlayerId(2), "p2-outcome")] {
        transition
            .apply_occurrence(PlannedOccurrence {
                lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                    perspective,
                    sequence: VisibleSequence(1),
                    mutation: mtgml_state::PerspectiveLifecycleMutationV1::default(),
                },
                observation: mtgml_rules::PerspectiveObservationPolicyV1::AnnouncedOutcome {
                    code: code.into(),
                },
            })
            .unwrap();
    }
    (before, transition.finish().unwrap())
}

#[test]
fn global_hidden_allocator_history_cannot_move_opaque_assignment() {
    use mtgml_rules::fixture_support::{FixtureTransition, PlannedOccurrence};
    let base = m2e_fixture();
    let mut variant = base.clone();
    // Hidden global allocation history differs wildly between the pair,
    // including the risky global OBJECT allocator itself.
    variant.allocators.next_effect_id = mtgml_model::EffectInstanceId(900);
    variant.allocators.next_trigger_id = mtgml_model::TriggerInstanceId(700);
    variant.allocators.next_object_id = GameObjectId(500);

    let mut previous: Option<Vec<u8>> = None;
    let mut collected_seen: Vec<u32> = Vec::new();
    for state in [base, variant] {
        let mut transition = FixtureTransition::start(&state).unwrap();
        let seen = transition
            .move_object_incarnation(GameObjectId(3), battlefield_location())
            .unwrap();
        transition
            .apply_occurrence(PlannedOccurrence {
                lifecycle: mtgml_state::PerspectiveLifecycleAuditV1 {
                    perspective: PlayerId(1),
                    sequence: VisibleSequence(1),
                    mutation: mtgml_state::PerspectiveLifecycleMutationV1 {
                        identity: mtgml_state::IdentityMutationV1::Allocate {
                            opaque: OpaqueObjectId(2),
                            object: seen,
                        },
                        knowledge: Some(mtgml_state::KnowledgeMutationV1::Acquire {
                            opaque: OpaqueObjectId(2),
                            definition: None,
                            location: Some(battlefield_location()),
                            acquisition: mtgml_state::KnowledgeAcquisitionReason::Observed {
                                channel: mtgml_state::KnowledgeHistoryChannel::Public,
                                sequence: VisibleSequence(1),
                                cause: mtgml_state::KnowledgeAcquisitionCause::ExplicitReveal,
                            },
                        }),
                    },
                },
                observation: mtgml_rules::PerspectiveObservationPolicyV1::Appeared {
                    from_zone: mtgml_model::ZoneKind::Exile,
                    to_zone: mtgml_model::ZoneKind::Battlefield,
                    new_object: seen,
                },
            })
            .unwrap();
        let result = transition.finish().unwrap();
        let identity = &result.next_state.perspective_identities.players[&PlayerId(1)];
        assert_eq!(
            identity.opaque_to_object.get(&OpaqueObjectId(2)),
            Some(&GameObjectId(seen.0))
        );
        collected_seen.push(u32::try_from(seen.0).unwrap());
        let knowledge_bytes =
            serde_json::to_vec(&result.next_state.knowledge.players[&PlayerId(1)]).unwrap();
        if let Some(previous_bytes) = previous.as_ref() {
            assert_eq!(previous_bytes, &knowledge_bytes);
        }
        previous = Some(knowledge_bytes);
    }
    // The trusted incarnations must differ (hidden object-allocator history)
    assert_ne!(collected_seen[0], collected_seen[1]);
}

// Lexical fragments: physical discoverability without changing any
// tests::<name> identity addressed by the M1/M2 gate runners.
include!("tests/forced_progress.rs");
include!("tests/checkpoint_replay.rs");
include!("tests/player_endpoint.rs");
include!("tests/continuation.rs");
include!("tests/information_projection.rs");
include!("tests/error_nonmutation.rs");
mod response_transaction {
    use super::*;
    include!("tests/response_transaction.rs");
}
include!("tests/batch_d.rs");
include!("tests/batch_e.rs");
include!("tests/batch_f.rs");
include!("tests/batch_g.rs");
include!("tests/turn_structure.rs");

mod semantic_catalog {
    #![allow(unused_imports)]
    use super::*;
    use crate::semantic_catalog_generated::synthetic_legacy_default_semantic_contract_id;
    include!("tests/semantic_catalog.rs");
}

mod restore_admission {
    use super::*;
    use crate::semantic_catalog_generated::synthetic_legacy_default_semantic_contract_id;
    include!("tests/restore_admission.rs");
}
