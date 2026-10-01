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

use std::collections::{BTreeMap, BTreeSet};

use crate::engine_state_shape::EngineStateShapeViolation;

use mtgml_persistence::cbor::Value;

fn synthetic_state() -> EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
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
            turn_number: 1,
            position: TurnPosition::Beginning {
                step: BeginningStep::Untap,
            },
            priority: PriorityState::None,
        },
        combat: None,
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
        card_rules: CardRulesAuthoritativeStateV1::default(),
    };
    state.card_rules.turn_history.turn_number = 1;
    for player in players {
        state
            .card_rules
            .mana
            .pools
            .insert(player, Default::default());
        state
            .card_rules
            .turn_history
            .players
            .insert(player, Default::default());
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
        acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
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
fn deterministic_structural_identity_repeats_exactly() {
    let state = synthetic_state();
    let rebuilt = synthetic_state();
    assert_eq!(state, rebuilt);
    assert_eq!(v7_digest(&state), v7_digest(&rebuilt));
}

#[test]
fn synthetic_reset_rejects_duplicate_players() {
    let result = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(1)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
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
        setup: SyntheticV4Setup::synthetic_compatibility(),
    };
    assert_eq!(
        construct_synthetic_engine_state(inputs.clone()).unwrap(),
        construct_synthetic_engine_state(inputs).unwrap()
    );
}

fn v7_digest(state: &EngineState) -> mtgml_model::FullStateDigest {
    calculate_full_state_digest(state).unwrap()
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
    let payload = canonical_state_bytes(state).unwrap();
    let decoded = mtgml_persistence::cbor::decode_canonical(&payload).unwrap();
    let mut texts = Vec::new();
    walk(&decoded, &mut texts);
    texts
}

fn lifecycle_fixture() -> EngineState {
    let mut state = synthetic_state();
    let exile_location = crate::zones::ZoneLocation {
        zone: ZoneKind::Exile,
        player: None,
        position: crate::zones::ZonePosition::Unordered,
        visibility: crate::zones::VisibilityPartition::Public,
        partition: None,
    };
    for index in 3..=4u64 {
        let object = GameObjectId(index);
        state.zones.objects.insert(
            object,
            crate::zones::GameObject {
                id: object,
                physical_card: Some(PhysicalCardId(index)),
                card_definition: CardDefinitionId(index),
                owner: PlayerId(1),
                controller: PlayerId(1),
                tapped: false,
                face_down: false,
            },
        );
        state.zones.locations.insert(object, exile_location.clone());
    }
    state.allocators.next_object_id = GameObjectId(5);
    state
}

fn observed_at(
    sequence: u64,
    channel: crate::knowledge::KnowledgeHistoryChannel,
    cause: crate::knowledge::KnowledgeAcquisitionCause,
) -> crate::knowledge::KnowledgeAcquisitionReason {
    crate::knowledge::KnowledgeAcquisitionReason::Observed {
        channel,
        sequence: VisibleSequence(sequence),
        cause,
    }
}

// Lexical fragments: physical discoverability without changing any
// tests::<name> identity addressed by the M1/M2 gate runners.
include!("tests/digest.rs");
include!("tests/validation.rs");
include!("tests/continuation.rs");
include!("tests/knowledge_identity.rs");
include!("tests/lifecycle.rs");
include!("tests/batch_d.rs");
include!("tests/batch_e.rs");
include!("tests/zones_allocators.rs");
