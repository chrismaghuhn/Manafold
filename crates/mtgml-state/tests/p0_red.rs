use std::collections::BTreeMap;

use mtgml_model::{GameObjectId, PlayerId};
use mtgml_random::RootSeed256;
use mtgml_state::{
    construct_synthetic_engine_state, BeginningStep, CombatState, PriorityState,
    SyntheticResetInputs, SyntheticV4Setup, TurnPosition,
};

#[test]
fn p0_synthetic_reset_receives_v4_facts_as_explicit_setup_input() {
    let setup = SyntheticV4Setup {
        position: TurnPosition::Beginning {
            step: BeginningStep::Untap,
        },
        priority: PriorityState::None,
        combat: None,
    };
    let state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup,
    })
    .unwrap();

    assert_eq!(
        state.core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Untap,
        }
    );
    assert_eq!(state.core.priority, PriorityState::None);
    assert_eq!(state.combat, None);
}

#[test]
fn p0_combat_facts_are_typed_state_values() {
    let combat = CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(1)],
        damage_step_completed: false,
        blocked_attackers: std::collections::BTreeSet::from([GameObjectId(1)]),
        blockers: BTreeMap::new(),
    };

    assert_eq!(combat.attackers, vec![GameObjectId(1)]);
}

fn compatibility_state() -> mtgml_state::EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::synthetic_compatibility(),
    })
    .unwrap()
}

fn absent_priority_holder(state: &mut mtgml_state::EngineState) {
    state.core.priority = PriorityState::HeldBy {
        player: PlayerId(99),
        consecutive_passes: 0,
    };
}

fn excessive_consecutive_passes(state: &mut mtgml_state::EngineState) {
    state.core.priority = PriorityState::HeldBy {
        player: PlayerId(1),
        consecutive_passes: 2,
    };
}

fn dangling_attacker(state: &mut mtgml_state::EngineState) {
    state.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(99)],
        damage_step_completed: false,
        blocked_attackers: std::collections::BTreeSet::new(),
        blockers: BTreeMap::new(),
    });
}

fn dangling_blocker(state: &mut mtgml_state::EngineState) {
    state.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(1)],
        damage_step_completed: false,
        blocked_attackers: std::collections::BTreeSet::from([GameObjectId(1)]),
        blockers: BTreeMap::from([(GameObjectId(99), GameObjectId(1))]),
    });
}

fn duplicate_attackers(state: &mut mtgml_state::EngineState) {
    state.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(1), GameObjectId(1)],
        damage_step_completed: false,
        blocked_attackers: std::collections::BTreeSet::new(),
        blockers: BTreeMap::new(),
    });
}

fn noncanonical_attackers(state: &mut mtgml_state::EngineState) {
    state.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(2), GameObjectId(1)],
        damage_step_completed: false,
        blocked_attackers: std::collections::BTreeSet::new(),
        blockers: BTreeMap::new(),
    });
}

type InvalidStateCase = (&'static str, fn(&mut mtgml_state::EngineState));

#[test]
fn p0_v4_state_validation_rejects_each_new_structural_invalidity() {
    let cases: [InvalidStateCase; 6] = [
        ("absent priority holder", absent_priority_holder),
        ("consecutive passes above one", excessive_consecutive_passes),
        ("dangling attacker", dangling_attacker),
        ("dangling blocker", dangling_blocker),
        ("duplicate attackers", duplicate_attackers),
        ("noncanonical attackers", noncanonical_attackers),
    ];

    for (label, mutate) in cases {
        let mut state = compatibility_state();
        mutate(&mut state);
        assert!(
            mtgml_state::validate_engine_state(&state).is_err(),
            "V4 structural invalidity must be rejected: {label}"
        );
    }
}
