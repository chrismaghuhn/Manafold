use std::collections::BTreeMap;

use mtgml_decision::{
    ActionCandidate, CandidateIntent, DecisionKind, DecisionVisibility, EngineCandidateBinding,
    PlayerDecisionRequest, PLAYER_DECISION_REQUEST_SCHEMA,
};
use mtgml_model::{
    AbilityInstanceId, CardDefinitionId, ContinuationId, DecisionId, EffectInstanceId,
    EventSequence, GameObjectId, OpaqueAbilityId, OpaqueObjectId, PhysicalCardId, PlayerId,
    RuleEventId, StackObjectId, StateRevision, TriggerInstanceId, ZoneKind,
};
use mtgml_random::{
    CanonicalRandomStreamEntryV1, RandomStateV1, RandomStreamCursorV1, RandomStreamKeyV1,
    RandomStreamKindV1, RandomValidationError, RootSeed256,
};
use thiserror::Error;

use crate::core::{CoreRulesState, PlayerState};
use crate::engine::EngineState;
use crate::execution::{ExecutionState, PendingDecisionRecord};
use crate::format::FormatState;
use crate::identity::{IdentityAllocatorState, PerspectiveIdentityMap, PerspectiveIdentityState};
use crate::knowledge::{
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgePoint, KnowledgeState,
    KnownObjectIdentity, PlayerKnowledgeState,
};
use crate::validation::{validate_engine_state, EngineStateViolation};
use crate::zones::{GameObject, VisibilityPartition, ZoneLocation, ZonePosition, ZoneState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntheticResetInputs {
    pub players: [PlayerId; 2],
    pub root_seed: RootSeed256,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SyntheticStateConstructionError {
    #[error("synthetic reset requires two distinct player identities")]
    DuplicatePlayers,
    #[error("synthetic RNG state could not be initialized: {0}")]
    Random(#[from] RandomValidationError),
    #[error("synthetic state failed cross-component validation: {0}")]
    Validation(#[from] EngineStateViolation),
}

pub fn construct_synthetic_engine_state(
    inputs: SyntheticResetInputs,
) -> Result<EngineState, SyntheticStateConstructionError> {
    let [player_one, player_two] = inputs.players;
    if player_one == player_two {
        return Err(SyntheticStateConstructionError::DuplicatePlayers);
    }

    let public_object_id = GameObjectId(1);
    let hidden_object_id = GameObjectId(2);
    let public_location = ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    };
    let hidden_location = ZoneLocation {
        zone: ZoneKind::Library,
        player: Some(player_two),
        position: ZonePosition::Top { offset: 0 },
        visibility: VisibilityPartition::FaceDown,
        partition: None,
    };
    let public_object = GameObject {
        id: public_object_id,
        physical_card: Some(PhysicalCardId(1)),
        card_definition: CardDefinitionId(1),
        owner: player_one,
        controller: player_one,
        tapped: false,
        face_down: false,
    };
    let hidden_object = GameObject {
        id: hidden_object_id,
        physical_card: Some(PhysicalCardId(2)),
        card_definition: CardDefinitionId(2),
        owner: player_two,
        controller: player_two,
        tapped: false,
        face_down: true,
    };

    let zones = ZoneState {
        objects: BTreeMap::from([
            (public_object_id, public_object),
            (hidden_object_id, hidden_object),
        ]),
        locations: BTreeMap::from([
            (public_object_id, public_location.clone()),
            (hidden_object_id, hidden_location.clone()),
        ]),
        ordered_zones: BTreeMap::from([(hidden_location.key(), vec![hidden_object_id])]),
        stack_records: BTreeMap::new(),
        stack_order: Vec::new(),
    };

    let public_opaque_id = OpaqueObjectId(1);
    let hidden_opaque_id = OpaqueObjectId(2);
    let player_one_identities = PerspectiveIdentityMap {
        object_to_opaque: BTreeMap::from([(public_object_id, public_opaque_id)]),
        opaque_to_object: BTreeMap::from([(public_opaque_id, public_object_id)]),
        ability_to_opaque: BTreeMap::new(),
        opaque_to_ability: BTreeMap::new(),
    };
    let player_two_identities = PerspectiveIdentityMap {
        object_to_opaque: BTreeMap::from([
            (public_object_id, public_opaque_id),
            (hidden_object_id, hidden_opaque_id),
        ]),
        opaque_to_object: BTreeMap::from([
            (public_opaque_id, public_object_id),
            (hidden_opaque_id, hidden_object_id),
        ]),
        ability_to_opaque: BTreeMap::new(),
        opaque_to_ability: BTreeMap::new(),
    };
    let perspective_identities = PerspectiveIdentityState {
        players: BTreeMap::from([
            (player_one, player_one_identities),
            (player_two, player_two_identities),
        ]),
    };

    let public_knowledge = KnownObjectIdentity {
        object: public_object_id,
        physical_card: Some(PhysicalCardId(1)),
        card_definition: Some(CardDefinitionId(1)),
        known_location: Some(public_location.clone()),
        learned_at: KnowledgePoint {
            channel: KnowledgeHistoryChannel::Public,
            sequence: EventSequence(0),
        },
        learned_via: KnowledgeAcquisitionReason::ExplicitReveal,
    };
    let hidden_knowledge = KnownObjectIdentity {
        object: hidden_object_id,
        physical_card: Some(PhysicalCardId(2)),
        card_definition: Some(CardDefinitionId(2)),
        known_location: Some(hidden_location),
        learned_at: KnowledgePoint {
            channel: KnowledgeHistoryChannel::Private,
            sequence: EventSequence(0),
        },
        learned_via: KnowledgeAcquisitionReason::OwnZoneIdentity,
    };
    let knowledge = KnowledgeState {
        players: BTreeMap::from([
            (
                player_one,
                PlayerKnowledgeState {
                    known_objects: BTreeMap::from([(public_object_id, public_knowledge.clone())]),
                    ..PlayerKnowledgeState::default()
                },
            ),
            (
                player_two,
                PlayerKnowledgeState {
                    known_objects: BTreeMap::from([
                        (public_object_id, public_knowledge),
                        (hidden_object_id, hidden_knowledge),
                    ]),
                    ..PlayerKnowledgeState::default()
                },
            ),
        ]),
    };

    let request = PlayerDecisionRequest {
        schema_version: PLAYER_DECISION_REQUEST_SCHEMA.to_owned(),
        decision_id: DecisionId(1),
        state_revision: StateRevision(0),
        actor: player_one,
        visibility: DecisionVisibility::Public,
        decision: DecisionKind::ChooseOne,
        candidates: vec![ActionCandidate {
            candidate_id: "select_public_object".to_owned(),
            semantic_key: "synthetic.select_public_object".to_owned(),
            intent: CandidateIntent::SelectObject {
                object: public_opaque_id,
            },
        }],
    };
    let execution = ExecutionState {
        pending_decision: Some(PendingDecisionRecord {
            request,
            candidate_bindings: BTreeMap::from([(
                "select_public_object".to_owned(),
                EngineCandidateBinding::SelectObject {
                    object: public_object_id,
                },
            )]),
            continuation: None,
        }),
        ..ExecutionState::default()
    };

    let allocators = IdentityAllocatorState {
        next_object_id: GameObjectId(3),
        next_ability_id: AbilityInstanceId(1),
        next_stack_object_id: StackObjectId(1),
        next_effect_id: EffectInstanceId(1),
        next_trigger_id: TriggerInstanceId(1),
        next_decision_id: DecisionId(2),
        next_continuation_id: ContinuationId(1),
        next_rule_event_id: RuleEventId(1),
        next_opaque_object_id: BTreeMap::from([
            (player_one, OpaqueObjectId(2)),
            (player_two, OpaqueObjectId(3)),
        ]),
        next_opaque_ability_id: BTreeMap::from([
            (player_one, OpaqueAbilityId(1)),
            (player_two, OpaqueAbilityId(1)),
        ]),
    };

    let random = RandomStateV1::from_entries(
        inputs.root_seed,
        vec![CanonicalRandomStreamEntryV1 {
            key: RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
            next_raw_u64: RandomStreamCursorV1::default().next_raw_u64,
        }],
    )?;

    let state = EngineState {
        revision: StateRevision(0),
        core: CoreRulesState {
            players: BTreeMap::from([
                (
                    player_one,
                    PlayerState {
                        life: 40,
                        has_lost: false,
                    },
                ),
                (
                    player_two,
                    PlayerState {
                        life: 40,
                        has_lost: false,
                    },
                ),
            ]),
            active_player: player_one,
            priority_player: player_one,
            turn_number: 1,
        },
        zones,
        allocators,
        execution,
        random,
        knowledge,
        perspective_identities,
        format: FormatState::None,
    };

    validate_engine_state(&state)?;
    Ok(state)
}
