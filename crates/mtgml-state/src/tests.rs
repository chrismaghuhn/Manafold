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

fn assembly_continuation(
    stage: AssemblyStageV2,
    selected_count: Option<u32>,
    selected_piece_keys: Vec<u32>,
    ordered_piece_keys: Vec<u32>,
) -> ContinuationRecordV2 {
    ContinuationRecordV2 {
        id: ContinuationId(1),
        actor: PlayerId(1),
        created_at_revision: StateRevision(0),
        stage_index: stage.stage_index(),
        payload: ContinuationPayloadV2::SyntheticM2Assembly {
            stage,
            selected_count,
            selected_piece_keys,
            ordered_piece_keys,
        },
    }
}

/// Builds the pending request that expresses exactly the given stage's
/// program, so continuation + request stay one authoritative unit.
fn matching_pending(record: &ContinuationRecordV2) -> PendingDecisionRecordV2 {
    use mtgml_decision::{AuthoritativeCandidateV2, DecisionDomainV2};
    let ContinuationPayloadV2::SyntheticM2Assembly {
        stage,
        selected_count,
        selected_piece_keys,
        ..
    } = &record.payload;
    let actor = record.actor;
    let pieces: Vec<u32> = match stage {
        AssemblyStageV2::ChooseCount => Vec::new(),
        AssemblyStageV2::ChooseMembers => (0..selected_count.unwrap_or(0)).collect(),
        AssemblyStageV2::OrderMembers => selected_piece_keys.clone(),
    };
    let candidates: Vec<AuthoritativeCandidateV2> = pieces
        .iter()
        .enumerate()
        .map(|(index, piece)| AuthoritativeCandidateV2 {
            candidate_id: mtgml_model::CandidateIdV1(index as u32),
            visible_intent: mtgml_decision::CandidateIntent::SelectMode { mode_index: *piece },
            trusted_binding: mtgml_decision::EngineCandidateBinding::SelectMode {
                mode_index: *piece,
            },
        })
        .collect();
    let decision = match stage {
        AssemblyStageV2::ChooseCount => DecisionDomainV2::ChooseNumber {
            minimum: 0,
            maximum: 3,
        },
        AssemblyStageV2::ChooseMembers => DecisionDomainV2::ChooseMany {
            minimum: selected_count.unwrap_or(0),
            maximum: selected_count.unwrap_or(0),
        },
        AssemblyStageV2::OrderMembers => {
            let count = u32::try_from(selected_piece_keys.len()).unwrap_or(0);
            DecisionDomainV2::Order {
                minimum: count,
                maximum: count,
            }
        }
    };
    PendingDecisionRecordV2 {
        request: mtgml_decision::AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(9),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(9),
            state_revision: StateRevision(0),
            actor,
            visibility: mtgml_decision::DecisionVisibility::Public,
            decision,
            candidates,
            continuation_id: Some(record.id),
        },
    }
}

fn state_with_continuation(record: ContinuationRecordV2) -> EngineState {
    let mut state = empty_shell();
    // The perspective-local player-decision allocator must cover the issued
    // visible identity of the attached pending request.
    let identity = state
        .perspective_identities
        .players
        .get_mut(&record.actor)
        .unwrap();
    identity.next_player_decision_id =
        mtgml_model::PlayerDecisionIdV1(identity.next_player_decision_id.0.max(10));
    state.allocators.next_decision_id = DecisionId(state.allocators.next_decision_id.0.max(10));
    state.execution.continuations.insert(record.id, record);
    let pending_request = {
        let record = state
            .execution
            .continuations
            .get(&ContinuationId(1))
            .unwrap();
        matching_pending(record)
    };
    if let Some(pending) = state.execution.pending_decision.as_mut() {
        *pending = pending_request;
    } else {
        state.execution.pending_decision = Some(pending_request);
    }
    state.allocators.next_continuation_id = ContinuationId(2);
    state
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
include!("tests/zones_allocators.rs");
