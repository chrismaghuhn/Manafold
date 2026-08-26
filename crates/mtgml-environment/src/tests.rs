use super::*;

use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};

use mtgml_model::{
    CandidateIdV1, CheckpointDigestV3, ContentDigest, ContinuationId, EpisodeStatus,
    FullStateDigestV3, PlayerDecisionIdV1, PlayerId, StateRevision, TerminalReason,
    TruncationReason,
};

use mtgml_observation::{
    PlayerStepV2, INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};

use mtgml_random::RootSeed256;

use mtgml_replay::{
    AuthoritativeReplayV3, DeckIdentityV1, KernelIdentityV1, ReplaySchemaVersionsV1,
};

fn config(players: [PlayerId; 2]) -> SyntheticM1EnvironmentConfig {
    SyntheticM1EnvironmentConfig {
        codec: CheckpointCodecIdentity {
            codec_id: "synthetic-m2-memory".into(),
            semantic_version: "3".into(),
        },
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
                decision_response: DECISION_RESPONSE_V2_SCHEMA.into(),
                observed_event: OBSERVED_EVENT_SCHEMA_V2.into(),
                player_step: PLAYER_STEP_SCHEMA_V2.into(),
                replay_step: "replay-step.v3".into(),
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
                0,
                KnowledgeAcquisitionCause::PrivateLook,
            ),
        }),
        historical_locations: vec![KnownLocationFactV2 {
            location: hidden_location.clone(),
            provenance: observed(
                KnowledgeHistoryChannel::Private,
                0,
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
                0,
                KnowledgeAcquisitionCause::ExplicitReveal,
            ),
            reason: KnowledgeInvalidationReason::Shuffle,
        },
    };
    retired.last_known_location = Some(KnownLocationFactV2 {
        location: hidden_location.clone(),
        provenance: observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
    });
    knowledge
        .retired
        .insert(mtgml_model::OpaqueObjectId(2), retired);
    knowledge.active.remove(&mtgml_model::OpaqueObjectId(2));

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

fn projected_provenance(
    information: &mtgml_observation::PlayerInformationStateV2,
) -> Vec<(u64, String)> {
    use mtgml_observation::{PlayerKnowledgeProvenanceV1, PlayerKnownObjectV1};
    fn render(provenance: &PlayerKnowledgeProvenanceV1) -> String {
        match provenance {
            PlayerKnowledgeProvenanceV1::InitialConfiguration => "initial_configuration".into(),
            PlayerKnowledgeProvenanceV1::Observed {
                channel,
                sequence,
                cause,
            } => format!("observed/{channel:?}/{}/{cause:?}", sequence.0),
        }
    }
    let mut rendered = Vec::new();
    for record in &information.retained_knowledge {
        match record {
            PlayerKnownObjectV1::Active {
                opaque_object_id,
                current_known_location_fact,
                historical_locations,
                acquisition,
                ..
            } => {
                if let Some(current) = current_known_location_fact {
                    rendered.push((
                        opaque_object_id.0,
                        format!("current/{}", render(&current.provenance)),
                    ));
                }
                for historical in historical_locations {
                    rendered.push((
                        opaque_object_id.0,
                        format!("historical/{}", render(&historical.provenance)),
                    ));
                }
                rendered.push((
                    opaque_object_id.0,
                    format!("acquisition/{}", render(acquisition)),
                ));
            }
            PlayerKnownObjectV1::Retired {
                opaque_object_id,
                last_known_location_fact,
                historical_locations,
                acquisition,
                invalidation,
                ..
            } => {
                if let Some(last) = last_known_location_fact {
                    rendered.push((
                        opaque_object_id.0,
                        format!("last/{}", render(&last.provenance)),
                    ));
                }
                for historical in historical_locations {
                    rendered.push((
                        opaque_object_id.0,
                        format!("historical/{}", render(&historical.provenance)),
                    ));
                }
                rendered.push((
                    opaque_object_id.0,
                    format!("acquisition/{}", render(acquisition)),
                ));
                let reason_text = format!("{:?}", invalidation.reason);
                rendered.push((
                    opaque_object_id.0,
                    format!(
                        "invalidation/{}/{}",
                        render(&invalidation.provenance),
                        reason_text
                    ),
                ));
            }
        }
    }
    rendered.sort();
    rendered
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
include!("tests/checkpoint_replay.rs");
include!("tests/player_endpoint.rs");
include!("tests/continuation.rs");
include!("tests/information_projection.rs");
include!("tests/error_nonmutation.rs");
