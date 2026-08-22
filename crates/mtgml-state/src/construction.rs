use std::collections::BTreeMap;

use mtgml_decision::{
    AuthoritativeCandidateV2, AuthoritativeDecisionRequestV2, CandidateIntent, DecisionDomainV2,
    DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::{
    AbilityInstanceId, CardDefinitionId, ContinuationId, DecisionId, EffectInstanceId,
    GameObjectId, OpaqueAbilityId, OpaqueObjectId, PhysicalCardId, PlayerDecisionIdV1, PlayerId,
    RuleEventId, StackObjectId, StateRevision, TriggerInstanceId, VisibleSequence, ZoneKind,
};
use mtgml_random::{
    CanonicalRandomStreamEntryV1, RandomStateV1, RandomStreamCursorV1, RandomStreamKeyV1,
    RandomStreamKindV1, RandomValidationError, RootSeed256,
};
use thiserror::Error;

use crate::core::{CoreRulesState, PlayerState};
use crate::engine::EngineState;
use crate::execution::ExecutionState;
use crate::format::FormatState;
use crate::identity::IdentityAllocatorState;
use crate::knowledge::KnowledgeAcquisitionReason;
use crate::m2_shape::{
    KnowledgeRecordV2, KnownLocationFactV2, PendingDecisionRecordV2, PerspectiveIdentityRecordV2,
    PerspectiveIdentityStateV2, PlayerKnowledgeStateV2,
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
    let player_one_identity = PerspectiveIdentityRecordV2 {
        opaque_to_object: BTreeMap::from([(public_opaque_id, public_object_id)]),
        opaque_to_ability: BTreeMap::new(),
        object_to_opaque: BTreeMap::from([(public_object_id, public_opaque_id)]),
        ability_to_opaque: BTreeMap::new(),
        next_opaque_object_id: OpaqueObjectId(2),
        next_opaque_ability_id: OpaqueAbilityId(1),
        next_player_decision_id: PlayerDecisionIdV1(2),
        retired_object_ids: Default::default(),
        retired_ability_ids: Default::default(),
    };
    let player_two_identity = PerspectiveIdentityRecordV2 {
        opaque_to_object: BTreeMap::from([
            (public_opaque_id, public_object_id),
            (hidden_opaque_id, hidden_object_id),
        ]),
        opaque_to_ability: BTreeMap::new(),
        object_to_opaque: BTreeMap::from([
            (public_object_id, public_opaque_id),
            (hidden_object_id, hidden_opaque_id),
        ]),
        ability_to_opaque: BTreeMap::new(),
        next_opaque_object_id: OpaqueObjectId(3),
        next_opaque_ability_id: OpaqueAbilityId(1),
        next_player_decision_id: PlayerDecisionIdV1(2),
        retired_object_ids: Default::default(),
        retired_ability_ids: Default::default(),
    };
    let perspective_identities = PerspectiveIdentityStateV2 {
        players: BTreeMap::from([
            (player_one, player_one_identity),
            (player_two, player_two_identity),
        ]),
    };

    let public_knowledge = KnowledgeRecordV2 {
        opaque_object: public_opaque_id,
        physical_card: Some(PhysicalCardId(1)),
        card_definition: Some(CardDefinitionId(1)),
        known_location: Some(KnownLocationFactV2 {
            location: public_location.clone(),
            provenance: KnowledgeAcquisitionReason::InitialConfiguration,
        }),
        acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
        historical_locations: Vec::new(),
    };
    let hidden_knowledge = KnowledgeRecordV2 {
        opaque_object: hidden_opaque_id,
        physical_card: Some(PhysicalCardId(2)),
        card_definition: Some(CardDefinitionId(2)),
        known_location: Some(KnownLocationFactV2 {
            location: hidden_location.clone(),
            provenance: KnowledgeAcquisitionReason::InitialConfiguration,
        }),
        acquisition: KnowledgeAcquisitionReason::InitialConfiguration,
        historical_locations: Vec::new(),
    };
    let knowledge = KnowledgeState {
        players: BTreeMap::from([
            (
                player_one,
                PlayerKnowledgeStateV2 {
                    active: BTreeMap::from([(public_opaque_id, public_knowledge.clone())]),
                    next_visible_sequence: VisibleSequence(1),
                    ..Default::default()
                },
            ),
            (
                player_two,
                PlayerKnowledgeStateV2 {
                    active: BTreeMap::from([
                        (public_opaque_id, public_knowledge),
                        (hidden_opaque_id, hidden_knowledge),
                    ]),
                    next_visible_sequence: VisibleSequence(1),
                    ..Default::default()
                },
            ),
        ]),
    };

    let request = AuthoritativeDecisionRequestV2 {
        decision_id: DecisionId(1),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(0),
        actor: player_one,
        visibility: DecisionVisibility::Public,
        decision: DecisionDomainV2::ChooseOne,
        candidates: vec![AuthoritativeCandidateV2 {
            candidate_id: mtgml_model::CandidateIdV1(0),
            visible_intent: CandidateIntent::SelectObject {
                object: public_opaque_id,
            },
            trusted_binding: EngineCandidateBinding::SelectObject {
                object: public_object_id,
            },
        }],
        continuation_id: None,
    };
    let execution = ExecutionState {
        pending_decision: Some(PendingDecisionRecordV2 { request }),
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

// Keep the type name used by the current EngineState field explicit at the
// module boundary; the implementation is the M2 V2 semantic shape.
type KnowledgeState = crate::m2_shape::KnowledgeStateV2;
