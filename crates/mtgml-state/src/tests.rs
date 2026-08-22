use super::*;
use mtgml_model::{
    AbilityInstanceId, CardDefinitionId, ContinuationId, DecisionId, GameObjectId, OpaqueAbilityId,
    OpaqueObjectId, PhysicalCardId, PlayerId, StackObjectId, StateRevision, TriggerInstanceId,
    VisibleSequence, ZoneKind,
};
use mtgml_random::{
    CanonicalRandomStreamEntryV1, RandomStateV1, RandomStreamCursorV1, RandomStreamKeyV1,
    RandomStreamKindV1, RootSeed256,
};
use std::collections::BTreeMap;

use crate::m2_shape::M2ShapeViolation;
use mtgml_persistence::cbor::Value;

fn synthetic_state() -> EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
    })
    .unwrap()
}

fn empty_shell() -> EngineState {
    let players = [PlayerId(1), PlayerId(2)];
    let mut state = EngineState {
        revision: StateRevision(0),
        core: CoreRulesState {
            players: players
                .into_iter()
                .map(|player| {
                    (
                        player,
                        PlayerState {
                            life: 20,
                            has_lost: false,
                        },
                    )
                })
                .collect(),
            active_player: PlayerId(1),
            priority_player: PlayerId(1),
            turn_number: 1,
        },
        zones: ZoneState::default(),
        allocators: IdentityAllocatorState::default(),
        execution: ExecutionState::default(),
        random: RandomStateV1::from_entries(
            RootSeed256::from_lower_hex(&"22".repeat(32)).unwrap(),
            vec![CanonicalRandomStreamEntryV1 {
                key: RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
                next_raw_u64: RandomStreamCursorV1::default().next_raw_u64,
            }],
        )
        .unwrap(),
        knowledge: KnowledgeStateV2::default(),
        perspective_identities: PerspectiveIdentityStateV2::default(),
        format: FormatState::None,
    };
    for player in players {
        state.knowledge.players.insert(player, Default::default());
        state.perspective_identities.players.insert(
            player,
            PerspectiveIdentityRecordV2 {
                opaque_to_object: BTreeMap::new(),
                opaque_to_ability: BTreeMap::new(),
                object_to_opaque: BTreeMap::new(),
                ability_to_opaque: BTreeMap::new(),
                next_opaque_object_id: OpaqueObjectId(1),
                next_opaque_ability_id: OpaqueAbilityId(1),
                next_player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
                retired_object_ids: Default::default(),
                retired_ability_ids: Default::default(),
            },
        );
    }
    state
}

fn public_location() -> ZoneLocation {
    ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    }
}

fn observed(
    channel: KnowledgeHistoryChannel,
    sequence: u64,
    cause: KnowledgeAcquisitionCause,
) -> KnowledgeAcquisitionReason {
    KnowledgeAcquisitionReason::Observed {
        channel,
        sequence: VisibleSequence(sequence),
        cause,
    }
}

fn fact(location: ZoneLocation, provenance: KnowledgeAcquisitionReason) -> KnownLocationFactV2 {
    KnownLocationFactV2 {
        location,
        provenance,
    }
}

fn retired_record(opaque: OpaqueObjectId) -> RetiredKnowledgeRecordV2 {
    RetiredKnowledgeRecordV2 {
        opaque_object: opaque,
        physical_card: None,
        card_definition: Some(CardDefinitionId(3)),
        last_known_location: None,
        historical_locations: Vec::new(),
        acquisition: observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::PublicEvent,
        ),
        invalidation: KnowledgeInvalidationV2 {
            provenance: observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::ExplicitReveal,
            ),
            reason: KnowledgeInvalidationReason::Shuffle,
        },
    }
}

#[test]
fn synthetic_state_is_the_current_m2_shape() {
    let state = synthetic_state();
    validate_engine_state(&state).unwrap();
    assert_eq!(state.revision, StateRevision(0));
    assert!(state.execution.pending_decision.is_some());
    assert!(state.execution.effects.is_empty());
    assert!(state.execution.waiting_triggers.is_empty());
    assert!(state.execution.delayed_effects.is_empty());
    assert_eq!(state.knowledge.players.len(), 2);
    assert_eq!(state.perspective_identities.players.len(), 2);
}

#[test]
fn valid_empty_shell_passes_cross_component_validation() {
    let state = empty_shell();
    validate_engine_state(&state).unwrap();
    assert!(state.digest().is_ok());
}

/// Frozen known answer for the canonical synthetic reset state. The payload is
/// the complete `full-state-digest-input.v3` canonical CBOR array and the
/// digest is SHA-256 of the `mtgml.digest-envelope.v1` framing around it.
#[test]
fn full_state_digest_v3_known_answer() {
    let state = synthetic_state();
    let payload = state.canonical_digest_bytes().unwrap();
    const EXPECTED_PAYLOAD_HEX: &str = concat!(
        "8b781a66756c6c2d73746174652d6469676573742d696e7075742e7633781a6d74676d6c2e66756c6c2d73746174652d",
        "6469676573742e763300848283011828f483021828f40101018582870101010101f4f4870202020202f4f5828201856b",
        "626174746c656669656c64f68269756e6f726465726564f6667075626c6963f6820285676c696272617279028263746f",
        "700069666163655f646f776ef6818284676c6962726172790269666163655f646f776ef6810280808803010101010201",
        "01858801010001667075626c6963826a63686f6f73655f6f6e65f6818300826d73656c6563745f6f626a65637401826d",
        "73656c6563745f6f626a65637401f680808080836c6d74676d6c2e726e672e7631582011111111111111111111111111",
        "11111111111111111111111111111111111111818244010001000082840101818601010182856b626174746c65666965",
        "6c64f68269756e6f726465726564f6667075626c6963f68275696e697469616c5f636f6e66696775726174696f6ef680",
        "8275696e697469616c5f636f6e66696775726174696f6ef680840201828601010182856b626174746c656669656c64f6",
        "8269756e6f726465726564f6667075626c6963f68275696e697469616c5f636f6e66696775726174696f6ef680827569",
        "6e697469616c5f636f6e66696775726174696f6ef6860202028285676c696272617279028263746f700069666163655f",
        "646f776ef68275696e697469616c5f636f6e66696775726174696f6ef6808275696e697469616c5f636f6e6669677572",
        "6174696f6ef6808288018182010180020102808088028282010182020280030102808082646e6f6e65f6",
    );
    const EXPECTED_DIGEST_HEX: &str =
        "680120895f69a0cea14399e53a80cc6bf3b10f167d7f9b21c5e2d38ebddf164a";
    assert_eq!(hex(&payload), EXPECTED_PAYLOAD_HEX);
    let digest = state.digest().unwrap();
    assert_eq!(digest.to_string(), EXPECTED_DIGEST_HEX);
    assert_eq!(digest.raw_bytes().len(), 32);
    assert_eq!(digest, state.clone().digest().unwrap());

    // The payload is exactly the eleven declared top-level fields, and each
    // knowledge entry is the fixed four-element per-player record.
    let decoded = mtgml_persistence::cbor::decode_canonical(&payload).unwrap();
    let Value::Array(fields) = &decoded else {
        panic!("V3 payload must be an array");
    };
    assert_eq!(fields.len(), 11);
    assert_eq!(fields[0], Value::Text("full-state-digest-input.v3".into()));
    assert_eq!(fields[1], Value::Text("mtgml.full-state-digest.v3".into()));
    let Value::Array(knowledge_players) = &fields[8] else {
        panic!("knowledge_v2 must be an array");
    };
    for player_entry in knowledge_players {
        let Value::Array(entry) = player_entry else {
            panic!("knowledge_v2 entries must be arrays");
        };
        assert_eq!(entry.len(), 4, "knowledge_v2 per-player layout changed");
    }
}

#[test]
fn m2_b_full_state_digest_v3_mutation_matrix() {
    type Mutation = (&'static str, fn(&mut EngineState));
    let mutations: Vec<Mutation> = vec![
        ("revision_and_pending_revision", |state| {
            state.revision = StateRevision(1);
            if let Some(pending) = state.execution.pending_decision.as_mut() {
                pending.request.state_revision = StateRevision(1);
            }
        }),
        ("core_life", |state| {
            state.core.players.get_mut(&PlayerId(1)).unwrap().life = 39;
        }),
        ("core_has_lost", |state| {
            state.core.players.get_mut(&PlayerId(2)).unwrap().has_lost = true;
        }),
        ("core_active_player", |state| {
            state.core.active_player = PlayerId(2);
        }),
        ("core_priority_player", |state| {
            state.core.priority_player = PlayerId(2);
        }),
        ("core_turn_number", |state| {
            state.core.turn_number += 1;
        }),
        ("zone_object_tapped", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .tapped = true;
        }),
        ("zone_object_face_down", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .face_down = true;
        }),
        ("zone_object_controller", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .controller = PlayerId(2);
        }),
        ("zone_object_owner", |state| {
            state.zones.objects.get_mut(&GameObjectId(1)).unwrap().owner = PlayerId(2);
        }),
        ("zone_object_physical_card", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .physical_card = None;
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    record.physical_card = None;
                }
            }
        }),
        ("zone_object_card_definition", |state| {
            state
                .zones
                .objects
                .get_mut(&GameObjectId(1))
                .unwrap()
                .card_definition = CardDefinitionId(9);
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    record.card_definition = Some(CardDefinitionId(9));
                }
            }
        }),
        ("zone_location_zone", |state| {
            let graveyard = ZoneLocation {
                zone: ZoneKind::Graveyard,
                ..public_location()
            };
            state
                .zones
                .locations
                .insert(GameObjectId(1), graveyard.clone());
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    if let Some(current) = record.known_location.as_mut() {
                        current.location = graveyard.clone();
                    }
                }
            }
        }),
        ("zone_location_position", |state| {
            let location = state.zones.locations.get_mut(&GameObjectId(2)).unwrap();
            location.position = ZonePosition::Bottom { offset: 0 };
            let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
            knowledge
                .active
                .get_mut(&OpaqueObjectId(2))
                .unwrap()
                .known_location
                .as_mut()
                .unwrap()
                .location
                .position = ZonePosition::Bottom { offset: 0 };
        }),
        ("zone_stack_records", |state| {
            state.zones.stack_records.insert(
                StackObjectId(1),
                StackRecord {
                    id: StackObjectId(1),
                    controller: PlayerId(1),
                    source_object: None,
                    source_ability: None,
                },
            );
            state.zones.stack_order.push(StackObjectId(1));
            state.allocators.next_stack_object_id = StackObjectId(2);
        }),
        ("allocator_next_object_id", |state| {
            state.allocators.next_object_id = GameObjectId(4);
        }),
        ("allocator_next_ability_id", |state| {
            state.allocators.next_ability_id = AbilityInstanceId(2);
        }),
        ("allocator_next_stack_object_id", |state| {
            state.allocators.next_stack_object_id = StackObjectId(2);
        }),
        ("allocator_next_effect_id", |state| {
            state.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
        }),
        ("allocator_next_trigger_id", |state| {
            state.allocators.next_trigger_id = TriggerInstanceId(2);
        }),
        ("allocator_next_decision_id", |state| {
            state.allocators.next_decision_id = DecisionId(3);
        }),
        ("allocator_next_continuation_id", |state| {
            state.allocators.next_continuation_id = ContinuationId(2);
        }),
        ("allocator_next_rule_event_id", |state| {
            state.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
        }),
        ("pending_decision_visibility", |state| {
            let pending = state.execution.pending_decision.as_mut().unwrap();
            pending.request.visibility = mtgml_decision::DecisionVisibility::ActingPlayerOnly;
        }),
        ("pending_decision_trusted_id", |state| {
            let pending = state.execution.pending_decision.as_mut().unwrap();
            pending.request.decision_id = DecisionId(2);
            state.allocators.next_decision_id = DecisionId(3);
        }),
        ("pending_decision_actor", |state| {
            let pending = state.execution.pending_decision.as_mut().unwrap();
            pending.request.actor = PlayerId(2);
        }),
        ("pending_candidate_binding", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity
                .opaque_to_object
                .insert(OpaqueObjectId(2), GameObjectId(2));
            identity
                .object_to_opaque
                .insert(GameObjectId(2), OpaqueObjectId(2));
            identity.next_opaque_object_id = OpaqueObjectId(3);
            let pending = state.execution.pending_decision.as_mut().unwrap();
            let candidate = &mut pending.request.candidates[0];
            candidate.visible_intent = mtgml_decision::CandidateIntent::SelectObject {
                object: OpaqueObjectId(2),
            };
            candidate.trusted_binding = mtgml_decision::EngineCandidateBinding::SelectObject {
                object: GameObjectId(2),
            };
        }),
        ("execution_continuation", |state| {
            state.execution.continuations.insert(
                ContinuationId(1),
                ContinuationRecordV2 {
                    id: ContinuationId(1),
                    actor: PlayerId(1),
                    created_at_revision: StateRevision(0),
                    stage_index: 0,
                    payload: ContinuationPayloadV2::SyntheticM2Assembly {
                        stage: AssemblyStageV2::ChooseCount,
                        selected_count: None,
                        selected_piece_keys: Vec::new(),
                        ordered_piece_keys: Vec::new(),
                    },
                },
            );
            state.allocators.next_continuation_id = ContinuationId(2);
        }),
        ("pending_continuation_reference", |state| {
            state.execution.continuations.insert(
                ContinuationId(1),
                ContinuationRecordV2 {
                    id: ContinuationId(1),
                    actor: PlayerId(1),
                    created_at_revision: StateRevision(0),
                    stage_index: 0,
                    payload: ContinuationPayloadV2::SyntheticM2Assembly {
                        stage: AssemblyStageV2::ChooseCount,
                        selected_count: None,
                        selected_piece_keys: Vec::new(),
                        ordered_piece_keys: Vec::new(),
                    },
                },
            );
            state.allocators.next_continuation_id = ContinuationId(2);
            let pending = state.execution.pending_decision.as_mut().unwrap();
            pending.request.continuation_id = Some(ContinuationId(1));
        }),
        ("random_root_seed", |state| {
            let seed = state.random.root_seed.as_bytes();
            let mut hex = String::with_capacity(64);
            for byte in seed {
                std::fmt::Write::write_fmt(&mut hex, format_args!("{byte:02x}")).unwrap();
            }
            let last = hex.pop().unwrap();
            hex.push(if last == '1' { '2' } else { '1' });
            state.random.root_seed = RootSeed256::from_lower_hex(&hex).unwrap();
        }),
        ("random_stream_cursor", |state| {
            let key = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
            let next = state.random.lookup_stream(&key).unwrap().next_raw_u64 + 1;
            state
                .random
                .set_cursor(&key, RandomStreamCursorV1 { next_raw_u64: next })
                .unwrap();
        }),
        ("random_additional_stream", |state| {
            state
                .random
                .streams
                .entry(RandomStreamKeyV1::player_scoped(
                    RandomStreamKindV1::SyntheticM1,
                    1,
                ))
                .or_default();
        }),
        ("knowledge_acquisition_provenance", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
            record.acquisition = observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::PublicEvent,
            );
        }),
        ("knowledge_provenance_cause_only", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
            record.acquisition = observed(
                KnowledgeHistoryChannel::Public,
                0,
                KnowledgeAcquisitionCause::ExplicitReveal,
            );
        }),
        ("knowledge_known_location", |state| {
            let graveyard = ZoneLocation {
                zone: ZoneKind::Graveyard,
                ..public_location()
            };
            state
                .zones
                .locations
                .insert(GameObjectId(1), graveyard.clone());
            for knowledge in state.knowledge.players.values_mut() {
                if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
                    if let Some(current) = record.known_location.as_mut() {
                        current.location = graveyard.clone();
                    }
                }
            }
        }),
        ("knowledge_private_acquisition", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
            let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
            record.acquisition = observed(
                KnowledgeHistoryChannel::Private,
                0,
                KnowledgeAcquisitionCause::PrivateLook,
            );
        }),
        ("knowledge_historical_location", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            knowledge
                .active
                .get_mut(&OpaqueObjectId(1))
                .unwrap()
                .historical_locations
                .push(fact(
                    public_location(),
                    observed(
                        KnowledgeHistoryChannel::Public,
                        0,
                        KnowledgeAcquisitionCause::PublicEvent,
                    ),
                ));
        }),
        ("knowledge_retired_record", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity.next_opaque_object_id = OpaqueObjectId(6);
            identity.retired_object_ids.insert(OpaqueObjectId(5));
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            knowledge
                .retired
                .insert(OpaqueObjectId(5), retired_record(OpaqueObjectId(5)));
        }),
        ("knowledge_next_visible_sequence", |state| {
            let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
            knowledge.next_visible_sequence = VisibleSequence(2);
        }),
        ("identity_object_mapping", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity
                .opaque_to_object
                .insert(OpaqueObjectId(2), GameObjectId(2));
            identity
                .object_to_opaque
                .insert(GameObjectId(2), OpaqueObjectId(2));
            identity.next_opaque_object_id = OpaqueObjectId(3);
        }),
        ("identity_ability_mapping", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity
                .opaque_to_ability
                .insert(OpaqueAbilityId(1), AbilityInstanceId(1));
            identity
                .ability_to_opaque
                .insert(AbilityInstanceId(1), OpaqueAbilityId(1));
            identity.next_opaque_ability_id = OpaqueAbilityId(2);
            state.allocators.next_ability_id = AbilityInstanceId(2);
        }),
        ("identity_next_player_decision_id", |state| {
            let identity = state
                .perspective_identities
                .players
                .get_mut(&PlayerId(1))
                .unwrap();
            identity.next_player_decision_id = mtgml_model::PlayerDecisionIdV1(3);
        }),
        ("format_commander", |state| {
            state.format = FormatState::Commander {
                state: CommanderState {
                    designations: BTreeMap::from([(PlayerId(1), vec![PhysicalCardId(1)])]),
                    cast_counts: BTreeMap::new(),
                    damage: BTreeMap::new(),
                },
            };
        }),
    ];

    let baseline = synthetic_state();
    let baseline_digest = baseline.digest().unwrap();
    assert!(!mutations.is_empty());
    for (name, mutate) in mutations {
        let mut changed = synthetic_state();
        mutate(&mut changed);
        validate_engine_state(&changed)
            .unwrap_or_else(|error| panic!("mutation {name} must stay valid: {error}"));
        let changed_digest = changed.digest().unwrap();
        assert_ne!(
            baseline_digest, changed_digest,
            "mutation {name} must change the V3 digest"
        );
    }
}

#[test]
fn knowledge_history_is_digested_without_a_player_level_aggregate() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
    ));
    validate_engine_state(&state).unwrap();
    let with_history = state.digest().unwrap();
    let mut stripped = state.clone();
    stripped
        .knowledge
        .players
        .get_mut(&PlayerId(2))
        .unwrap()
        .active
        .get_mut(&OpaqueObjectId(2))
        .unwrap()
        .historical_locations
        .clear();
    validate_engine_state(&stripped).unwrap();
    assert_ne!(with_history, stripped.digest().unwrap());
}

#[test]
fn deterministic_structural_identity_repeats_exactly() {
    let state = synthetic_state();
    let rebuilt = synthetic_state();
    assert_eq!(state, rebuilt);
    assert_eq!(state.digest().unwrap(), rebuilt.digest().unwrap());
    assert_eq!(
        state.canonical_digest_bytes().unwrap(),
        rebuilt.canonical_digest_bytes().unwrap()
    );
}

#[test]
fn state_delta_uses_full_state_digest_v3() {
    let before = synthetic_state();
    let mut after = before.clone();
    after.core.players.get_mut(&PlayerId(1)).unwrap().life = 39;
    let graveyard = ZoneLocation {
        zone: ZoneKind::Graveyard,
        ..public_location()
    };
    after
        .zones
        .locations
        .insert(GameObjectId(1), graveyard.clone());
    for knowledge in after.knowledge.players.values_mut() {
        if let Some(record) = knowledge.active.get_mut(&OpaqueObjectId(1)) {
            if let Some(current) = record.known_location.as_mut() {
                current.location = graveyard.clone();
            }
        }
    }
    let knowledge = after.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge.next_visible_sequence = VisibleSequence(2);
    after
        .random
        .set_cursor(
            &RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
            RandomStreamCursorV1 { next_raw_u64: 1 },
        )
        .unwrap();
    after.allocators.next_object_id = GameObjectId(4);

    let delta = StateDelta::between(&before, &after, vec![]).unwrap();
    assert_eq!(delta.before_digest, before.digest().unwrap());
    assert_eq!(delta.after_digest, after.digest().unwrap());
    let reapplied = delta.apply(&before).unwrap();
    assert_eq!(reapplied, after);
    assert_eq!(reapplied.digest().unwrap(), delta.after_digest);

    let unrelated = empty_shell();
    assert!(matches!(
        delta.apply(&unrelated),
        Err(DeltaApplicationError::BeforeMismatch)
    ));
}

#[test]
fn v3_digest_payload_is_nonempty_canonical_cbor() {
    let state = synthetic_state();
    let payload = state.canonical_digest_bytes().unwrap();
    assert!(!payload.is_empty());
    assert_eq!(payload[0] & 0xe0, 0x80, "root must be a CBOR array");
}

#[test]
fn synthetic_reset_rejects_duplicate_players() {
    let result = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(1)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
    });
    assert!(matches!(
        result,
        Err(SyntheticStateConstructionError::DuplicatePlayers)
    ));
}

#[test]
fn synthetic_reset_is_exactly_deterministic_for_identical_inputs() {
    let inputs = SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"33".repeat(32)).unwrap(),
    };
    assert_eq!(
        construct_synthetic_engine_state(inputs).unwrap(),
        construct_synthetic_engine_state(inputs).unwrap()
    );
}

#[test]
fn pending_decision_must_match_state_revision() {
    let mut state = synthetic_state();
    state.revision = StateRevision(7);
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::PendingDecision
        ))
    ));
}

#[test]
fn unordered_object_must_not_appear_in_ordered_zones() {
    let mut state = synthetic_state();
    let key = state.zones.locations.get(&GameObjectId(2)).unwrap().key();
    state
        .zones
        .ordered_zones
        .get_mut(&key)
        .unwrap()
        .push(GameObjectId(1));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::OrderedZoneMismatch)
    ));
}

#[test]
fn ordered_object_must_appear_exactly_once() {
    let mut state = synthetic_state();
    let key = state.zones.locations.get(&GameObjectId(2)).unwrap().key();
    let ordered = state.zones.ordered_zones.get_mut(&key).unwrap();
    ordered.push(GameObjectId(2));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::OrderedZoneMismatch)
    ));
}

#[test]
fn ordered_object_must_not_be_missing() {
    let mut state = synthetic_state();
    state
        .zones
        .locations
        .get_mut(&GameObjectId(2))
        .unwrap()
        .position = ZonePosition::Unordered;
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::OrderedZoneMismatch)
    ));
}

#[test]
fn duplicate_live_physical_card_incarnation_rejected() {
    let mut state = synthetic_state();
    state
        .zones
        .objects
        .get_mut(&GameObjectId(1))
        .unwrap()
        .physical_card = Some(PhysicalCardId(2));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::DuplicatePhysicalCard)
    ));
}

#[test]
fn pending_candidate_binding_must_match_authoritative_binding() {
    let mut state = synthetic_state();
    let pending = state.execution.pending_decision.as_mut().unwrap();
    pending.request.candidates[0].trusted_binding =
        mtgml_decision::EngineCandidateBinding::SelectObject {
            object: GameObjectId(2),
        };
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::PendingDecisionMismatch)
    ));
}

#[test]
fn opaque_allocator_must_reference_declared_player() {
    let mut state = empty_shell();
    state.random.streams.insert(
        RandomStreamKeyV1::player_scoped(RandomStreamKindV1::SyntheticM1, 9),
        RandomStreamCursorV1::default(),
    );
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::RandomState)
    ));
}

#[test]
fn commander_designation_must_reference_owned_live_physical_card() {
    let mut state = synthetic_state();
    state.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(PlayerId(2), vec![PhysicalCardId(1)])]),
            cast_counts: BTreeMap::new(),
            damage: BTreeMap::new(),
        },
    };
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::FormatMismatch)
    ));
}

#[test]
fn valid_commander_structural_references_are_accepted() {
    let mut state = synthetic_state();
    state.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(PlayerId(1), vec![PhysicalCardId(1)])]),
            cast_counts: BTreeMap::from([(PhysicalCardId(1), 2)]),
            damage: BTreeMap::from([(PhysicalCardId(1), BTreeMap::from([(PlayerId(2), 5)]))]),
        },
    };
    validate_engine_state(&state).unwrap();
}

#[test]
fn commander_ledger_must_reference_a_designated_physical_card() {
    let mut state = synthetic_state();
    state.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(PlayerId(1), vec![PhysicalCardId(1)])]),
            cast_counts: BTreeMap::from([(PhysicalCardId(2), 1)]),
            damage: BTreeMap::new(),
        },
    };
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::FormatMismatch)
    ));
}

#[test]
fn commander_damage_ledger_must_reference_a_declared_player() {
    let mut state = synthetic_state();
    state.format = FormatState::Commander {
        state: CommanderState {
            designations: BTreeMap::from([(PlayerId(1), vec![PhysicalCardId(1)])]),
            cast_counts: BTreeMap::new(),
            damage: BTreeMap::from([(PhysicalCardId(1), BTreeMap::from([(PlayerId(9), 3)]))]),
        },
    };
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::FormatMismatch)
    ));
}

#[test]
fn active_knowledge_requires_exactly_one_live_mapping() {
    let mut state = synthetic_state();
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .opaque_to_object
        .remove(&OpaqueObjectId(1));
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .object_to_opaque
        .remove(&GameObjectId(1));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::KnowledgeMismatch)
    ));
}

#[test]
fn reverse_identity_maps_must_agree() {
    let mut state = synthetic_state();
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .object_to_opaque
        .insert(GameObjectId(1), OpaqueObjectId(9));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::IdentityMapping
        ))
    ));
}

#[test]
fn retired_opaque_identity_must_not_stay_active() {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.retired_object_ids.insert(OpaqueObjectId(1));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::RetiredIdentity
        ))
    ));
}

#[test]
fn opaque_allocator_must_stay_ahead_of_issued_ids() {
    let mut state = synthetic_state();
    state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap()
        .next_opaque_object_id = OpaqueObjectId(1);
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(M2ShapeViolation::Allocator))
    ));
}

#[test]
fn retired_knowledge_must_not_keep_a_live_mapping() {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    identity.retired_object_ids.insert(OpaqueObjectId(5));
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge
        .retired
        .insert(OpaqueObjectId(5), retired_record(OpaqueObjectId(5)));
    validate_engine_state(&state).unwrap();

    // The same opaque identity cannot simultaneously be retired and active.
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity
        .opaque_to_object
        .insert(OpaqueObjectId(5), GameObjectId(2));
    identity
        .object_to_opaque
        .insert(GameObjectId(2), OpaqueObjectId(5));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::RetiredIdentity
        ))
    ));
}

#[test]
fn known_location_must_match_the_live_association() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge
        .active
        .get_mut(&OpaqueObjectId(1))
        .unwrap()
        .known_location = Some(KnownLocationFactV2 {
        location: ZoneLocation {
            zone: ZoneKind::Exile,
            ..public_location()
        },
        provenance: KnowledgeAcquisitionReason::InitialConfiguration,
    });
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::KnowledgeMismatch)
    ));
}

#[test]
fn historical_location_sequences_must_increase() {
    let mut state = synthetic_state();
    let fact_at = |sequence: u64| {
        fact(
            public_location(),
            observed(
                KnowledgeHistoryChannel::Public,
                sequence,
                KnowledgeAcquisitionCause::PublicEvent,
            ),
        )
    };
    {
        let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
        let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
        record.historical_locations = vec![fact_at(0), fact_at(0)];
    }
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    ));
    {
        let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
        let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
        record.historical_locations = vec![fact_at(1), fact_at(0)];
    }
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    ));
}

#[test]
fn knowledge_provenance_must_not_be_future_dated() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
    record.acquisition = observed(
        KnowledgeHistoryChannel::Public,
        5,
        KnowledgeAcquisitionCause::PublicEvent,
    );
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    ));
}

#[test]
fn invalid_knowledge_cause_channel_combination_is_rejected() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
    record.acquisition = observed(
        KnowledgeHistoryChannel::Private,
        0,
        KnowledgeAcquisitionCause::PublicEvent,
    );
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    );
}

#[test]
fn retired_knowledge_provenance_is_validated() {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    identity.retired_object_ids.insert(OpaqueObjectId(5));
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let mut record = retired_record(OpaqueObjectId(5));
    record.acquisition = observed(
        KnowledgeHistoryChannel::Public,
        9,
        KnowledgeAcquisitionCause::PublicEvent,
    );
    knowledge.retired.insert(OpaqueObjectId(5), record);
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    ));
}

#[test]
fn pending_decision_must_reference_an_existing_continuation() {
    let mut state = synthetic_state();
    let pending = state.execution.pending_decision.as_mut().unwrap();
    pending.request.continuation_id = Some(ContinuationId(42));
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationReference
        ))
    ));
}

#[test]
fn continuation_stage_must_match_its_payload() {
    let mut state = empty_shell();
    state.execution.continuations.insert(
        ContinuationId(1),
        ContinuationRecordV2 {
            id: ContinuationId(1),
            actor: PlayerId(1),
            created_at_revision: StateRevision(0),
            stage_index: 2,
            payload: ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseCount,
                selected_count: None,
                selected_piece_keys: Vec::new(),
                ordered_piece_keys: Vec::new(),
            },
        },
    );
    state.allocators.next_continuation_id = ContinuationId(2);
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationStage
        ))
    ));
}

#[test]
fn continuation_revision_must_not_be_future_dated() {
    let mut state = empty_shell();
    state.execution.continuations.insert(
        ContinuationId(1),
        ContinuationRecordV2 {
            id: ContinuationId(1),
            actor: PlayerId(1),
            created_at_revision: StateRevision(3),
            stage_index: 0,
            payload: ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseCount,
                selected_count: None,
                selected_piece_keys: Vec::new(),
                ordered_piece_keys: Vec::new(),
            },
        },
    );
    state.allocators.next_continuation_id = ContinuationId(2);
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationRevision
        ))
    ));
}

#[test]
fn unsupported_effect_machinery_is_rejected() {
    let mut state = empty_shell();
    state.execution.effects.insert(
        mtgml_model::EffectInstanceId(1),
        EffectRecord {
            id: mtgml_model::EffectInstanceId(1),
            label: "unsupported".into(),
        },
    );
    state.allocators.next_effect_id = mtgml_model::EffectInstanceId(2);
    assert!(matches!(
        validate_engine_state(&state),
        Err(EngineStateViolation::ExecutionMismatch)
    ));
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        std::fmt::Write::write_fmt(&mut out, format_args!("{byte:02x}")).unwrap();
        out
    })
}

fn digest_payload_texts(state: &EngineState) -> Vec<String> {
    fn walk(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Text(text) => out.push(text.clone()),
            Value::Array(items) => {
                for item in items {
                    walk(item, out);
                }
            }
            _ => {}
        }
    }
    let payload = state.canonical_digest_bytes().unwrap();
    let decoded = mtgml_persistence::cbor::decode_canonical(&payload).unwrap();
    let mut texts = Vec::new();
    walk(&decoded, &mut texts);
    texts
}

#[test]
fn historical_private_look_provenance_is_bound_into_the_digest() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::PrivateLook,
        ),
    ));
    validate_engine_state(&state).unwrap();
    assert!(digest_payload_texts(&state).contains(&"private_look".to_string()));

    // Changing only the retained cause changes the V3 digest.
    let baseline_digest = state.digest().unwrap();
    let mut changed = state.clone();
    let knowledge = changed.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    record.historical_locations[0].provenance = observed(
        KnowledgeHistoryChannel::Private,
        0,
        KnowledgeAcquisitionCause::OwnPrivateIdentity,
    );
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, changed.digest().unwrap());
}

#[test]
fn explicit_reveal_is_not_collapsed_to_public_event() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::ExplicitReveal,
        ),
    ));
    validate_engine_state(&state).unwrap();
    let texts = digest_payload_texts(&state);
    assert!(texts.contains(&"explicit_reveal".to_string()));
}

#[test]
fn own_private_identity_is_not_collapsed_to_private_look() {
    let mut state = synthetic_state();
    let knowledge = state.knowledge.players.get_mut(&PlayerId(2)).unwrap();
    let record = knowledge.active.get_mut(&OpaqueObjectId(2)).unwrap();
    let location = record.known_location.clone().unwrap().location;
    record.historical_locations.push(fact(
        location,
        observed(
            KnowledgeHistoryChannel::Private,
            0,
            KnowledgeAcquisitionCause::OwnPrivateIdentity,
        ),
    ));
    validate_engine_state(&state).unwrap();
    assert!(digest_payload_texts(&state).contains(&"own_private_identity".to_string()));
}

#[test]
fn invalidation_provenance_is_preserved_exactly() {
    let mut state = synthetic_state();
    let identity = state
        .perspective_identities
        .players
        .get_mut(&PlayerId(1))
        .unwrap();
    identity.next_opaque_object_id = OpaqueObjectId(6);
    identity.retired_object_ids.insert(OpaqueObjectId(5));
    let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    knowledge
        .retired
        .insert(OpaqueObjectId(5), retired_record(OpaqueObjectId(5)));
    validate_engine_state(&state).unwrap();

    let texts = digest_payload_texts(&state);
    assert!(texts.contains(&"explicit_reveal".to_string()));
    assert!(texts.contains(&"shuffle".to_string()));

    // Mutating only the invalidation provenance changes the digest.
    let baseline_digest = state.digest().unwrap();
    let mut changed = state.clone();
    let knowledge = changed.knowledge.players.get_mut(&PlayerId(1)).unwrap();
    let record = knowledge.retired.get_mut(&OpaqueObjectId(5)).unwrap();
    record.invalidation.provenance = observed(
        KnowledgeHistoryChannel::Public,
        0,
        KnowledgeAcquisitionCause::PublicEvent,
    );
    validate_engine_state(&changed).unwrap();
    assert_ne!(baseline_digest, changed.digest().unwrap());
}

#[test]
fn global_decision_allocator_must_exceed_every_issued_identity() {
    let build = |next: u64| {
        let mut state = synthetic_state();
        state.allocators.next_decision_id = DecisionId(next);
        state
    };
    // next == issued must fail closed.
    assert_eq!(
        validate_engine_state(&build(1)),
        Err(EngineStateViolation::AllocatorBehind)
    );
    // next < issued must fail closed.
    assert_eq!(
        validate_engine_state(&build(0)),
        Err(EngineStateViolation::AllocatorBehind)
    );
    // A strictly greater cursor is accepted.
    validate_engine_state(&build(2)).unwrap();
    assert!(build(2).digest().is_ok());
}

#[test]
fn global_ability_allocator_must_exceed_every_issued_identity() {
    let issue = |state: &mut EngineState| {
        let identity = state
            .perspective_identities
            .players
            .get_mut(&PlayerId(1))
            .unwrap();
        identity
            .opaque_to_ability
            .insert(OpaqueAbilityId(1), AbilityInstanceId(5));
        identity
            .ability_to_opaque
            .insert(AbilityInstanceId(5), OpaqueAbilityId(1));
        identity.next_opaque_ability_id = OpaqueAbilityId(2);
    };
    let build = |next: u64| {
        let mut state = synthetic_state();
        issue(&mut state);
        state.allocators.next_ability_id = AbilityInstanceId(next);
        state
    };
    // next == issued must fail closed.
    assert_eq!(
        validate_engine_state(&build(5)),
        Err(EngineStateViolation::AllocatorBehind)
    );
    // next < issued must fail closed.
    assert_eq!(
        validate_engine_state(&build(4)),
        Err(EngineStateViolation::AllocatorBehind)
    );
    // A strictly greater cursor is accepted.
    validate_engine_state(&build(6)).unwrap();
}

#[test]
fn continuation_actor_must_own_the_referenced_request() {
    let mut state = empty_shell();
    state.execution.continuations.insert(
        ContinuationId(1),
        ContinuationRecordV2 {
            id: ContinuationId(1),
            actor: PlayerId(1),
            created_at_revision: StateRevision(0),
            stage_index: 0,
            payload: ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseCount,
                selected_count: None,
                selected_piece_keys: Vec::new(),
                ordered_piece_keys: Vec::new(),
            },
        },
    );
    state.allocators.next_continuation_id = ContinuationId(2);
    state.allocators.next_decision_id = DecisionId(2);
    // A valid pending request owned by a different player.
    state.execution.pending_decision = Some(PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(1),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: PlayerId(2),
            visibility: mtgml_decision::DecisionVisibility::Public,
            decision: mtgml_decision::DecisionDomainV2::ChooseOne,
            candidates: vec![mtgml_decision::AuthoritativeCandidateV2 {
                candidate_id: mtgml_model::CandidateIdV1(0),
                visible_intent: mtgml_decision::CandidateIntent::PassPriority,
                trusted_binding: mtgml_decision::EngineCandidateBinding::PassPriority,
            }],
            continuation_id: Some(ContinuationId(1)),
        },
    });
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::ContinuationActor
        ))
    );
}

#[test]
fn historical_monotonicity_ignores_unsequenced_provenance() {
    let build = |provenances: Vec<KnowledgeAcquisitionReason>| {
        let mut state = synthetic_state();
        let knowledge = state.knowledge.players.get_mut(&PlayerId(1)).unwrap();
        let record = knowledge.active.get_mut(&OpaqueObjectId(1)).unwrap();
        let location = record.known_location.clone().unwrap().location;
        record.historical_locations = provenances
            .into_iter()
            .map(|provenance| fact(location.clone(), provenance))
            .collect();
        state
    };

    // An unsequenced initial fact followed by an observed fact is valid.
    let valid = build(vec![
        KnowledgeAcquisitionReason::InitialConfiguration,
        observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::PublicEvent,
        ),
    ]);
    validate_engine_state(&valid).unwrap();

    // Two observed facts at the same sequence remain invalid.
    let invalid = build(vec![
        observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::PublicEvent,
        ),
        KnowledgeAcquisitionReason::InitialConfiguration,
        observed(
            KnowledgeHistoryChannel::Public,
            0,
            KnowledgeAcquisitionCause::PublicEvent,
        ),
    ]);
    assert_eq!(
        validate_engine_state(&invalid),
        Err(EngineStateViolation::M2Shape(
            M2ShapeViolation::VisibleSequence
        ))
    );
}
