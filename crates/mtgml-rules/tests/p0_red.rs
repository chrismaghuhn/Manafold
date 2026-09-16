use std::collections::BTreeMap;

use mtgml_decision::{DecisionAnswerV2, DecisionResponseV2, DECISION_RESPONSE_V2_SCHEMA};
use mtgml_model::{CandidateIdV1, PlayerDecisionIdV1, PlayerId, StateRevision};
use mtgml_random::RootSeed256;
use mtgml_rules::{
    validate_transition_contract, RulesKernel, SyntheticM1RulesKernel, TransitionViolation,
};
use mtgml_state::{
    construct_synthetic_engine_state, BaseCharacteristics, CombatState, ControlHistory,
    FoundationCreatureSource, FoundationSourceKind, PriorityState, StateDelta,
    SyntheticResetInputs, SyntheticV4Setup, TurnPosition,
};

fn state() -> mtgml_state::EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap()
}

fn response() -> DecisionResponseV2 {
    DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(0),
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(0),
        },
    }
}

fn foundation_source() -> FoundationCreatureSource {
    FoundationCreatureSource {
        source_kind: FoundationSourceKind::Creature,
        base_characteristics: BaseCharacteristics::Simple {
            power: 3,
            toughness: 3,
        },
        marked_damage: 0,
        control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
    }
}

fn assert_v4_fact_mutation_is_rejected(mutate: impl FnOnce(&mut mtgml_state::EngineState)) {
    let before = state();
    let mut kernel = SyntheticM1RulesKernel;
    let mut transition = kernel.apply(&before, PlayerId(1), &response()).unwrap();
    mutate(&mut transition.next_state);
    transition.delta = StateDelta::between(
        &before,
        &transition.next_state,
        transition.delta.audit.clone(),
    )
    .unwrap();

    let error = validate_transition_contract(&before, &transition).unwrap_err();
    assert!(matches!(error, TransitionViolation::UnexplainedMutation));
}

#[test]
fn p0_normal_transition_rejects_unexplained_turn_position_mutation() {
    assert_v4_fact_mutation_is_rejected(|state| {
        state.core.position = TurnPosition::PrecombatMain;
    });
}

#[test]
fn p0_normal_transition_rejects_unexplained_priority_mutation() {
    assert_v4_fact_mutation_is_rejected(|state| {
        state.core.priority = PriorityState::HeldBy {
            player: PlayerId(1),
            consecutive_passes: 0,
        };
    });
}

#[test]
fn p0_normal_transition_rejects_unexplained_combat_mutation() {
    assert_v4_fact_mutation_is_rejected(|state| {
        state.combat = Some(CombatState {
            defending_player: PlayerId(2),
            attackers: vec![mtgml_model::GameObjectId(1)],
            blockers: BTreeMap::from([(mtgml_model::GameObjectId(1), None)]),
        });
    });
}

#[test]
fn p0_normal_transition_rejects_unexplained_foundation_source_mutation() {
    assert_v4_fact_mutation_is_rejected(|state| {
        state
            .foundation_sources
            .insert(mtgml_model::GameObjectId(1), foundation_source());
    });
}
