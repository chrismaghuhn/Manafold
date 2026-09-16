use std::collections::BTreeMap;

use mtgml_model::{FullStateDigestV4, GameObjectId, PlayerId};
use mtgml_random::RootSeed256;
use mtgml_state::{
    construct_synthetic_engine_state, BaseCharacteristics, BeginningStep, CombatState, CombatStep,
    ControlHistory, FoundationCreatureSource, FoundationSourceKind, PriorityState,
    SyntheticResetInputs, SyntheticV4Setup, TurnPosition,
};

fn current_digest_is_v4(state: &mtgml_state::EngineState) -> FullStateDigestV4 {
    state.digest().unwrap()
}

#[test]
fn p0_synthetic_reset_receives_v4_facts_as_explicit_setup_input() {
    let setup = SyntheticV4Setup {
        position: TurnPosition::Beginning {
            step: BeginningStep::Untap,
        },
        priority: PriorityState::None,
        combat: None,
        foundation_sources: BTreeMap::new(),
    };
    let state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup,
    })
    .unwrap();

    let _digest = current_digest_is_v4(&state);
    assert_eq!(
        state.core.position,
        TurnPosition::Beginning {
            step: BeginningStep::Untap,
        }
    );
    assert_eq!(state.core.priority, PriorityState::None);
    assert_eq!(state.combat, None);
    assert!(state.foundation_sources.is_empty());
}

#[test]
fn p0_foundation_and_combat_facts_are_typed_state_values() {
    let source = FoundationCreatureSource {
        source_kind: FoundationSourceKind::Creature,
        base_characteristics: BaseCharacteristics::Simple {
            power: 3,
            toughness: 3,
        },
        marked_damage: 0,
        control_history: ControlHistory::BeforeTurnStart { turn_number: 1 },
    };
    let combat = CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(1)],
        blockers: BTreeMap::from([(GameObjectId(1), None)]),
    };

    assert_eq!(source.source_kind, FoundationSourceKind::Creature);
    assert_eq!(combat.attackers, vec![GameObjectId(1)]);
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

fn compatibility_state() -> mtgml_state::EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
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
        blockers: BTreeMap::from([(GameObjectId(99), None)]),
    });
}

fn dangling_blocker(state: &mut mtgml_state::EngineState) {
    state.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(1)],
        blockers: BTreeMap::from([(GameObjectId(1), Some(GameObjectId(99)))]),
    });
}

fn duplicate_attackers(state: &mut mtgml_state::EngineState) {
    state.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(1), GameObjectId(1)],
        blockers: BTreeMap::from([(GameObjectId(1), None)]),
    });
}

fn noncanonical_attackers(state: &mut mtgml_state::EngineState) {
    state.combat = Some(CombatState {
        defending_player: PlayerId(2),
        attackers: vec![GameObjectId(2), GameObjectId(1)],
        blockers: BTreeMap::from([(GameObjectId(1), None), (GameObjectId(2), None)]),
    });
}

fn dangling_foundation_source(state: &mut mtgml_state::EngineState) {
    state
        .foundation_sources
        .insert(GameObjectId(99), foundation_source());
}

fn future_same_turn_control_boundary(state: &mut mtgml_state::EngineState) {
    state.core.position = TurnPosition::Beginning {
        step: BeginningStep::Upkeep,
    };
    state.foundation_sources.insert(
        GameObjectId(1),
        FoundationCreatureSource {
            control_history: ControlHistory::DuringTurn {
                turn_number: state.core.turn_number,
                boundary: TurnPosition::Combat {
                    step: CombatStep::DeclareAttackers,
                },
            },
            ..foundation_source()
        },
    );
}

type InvalidStateCase = (&'static str, fn(&mut mtgml_state::EngineState));

#[test]
fn p0_v4_state_validation_rejects_each_new_structural_invalidity() {
    let cases: [InvalidStateCase; 8] = [
        ("absent priority holder", absent_priority_holder),
        ("consecutive passes above one", excessive_consecutive_passes),
        ("dangling attacker", dangling_attacker),
        ("dangling blocker", dangling_blocker),
        ("duplicate attackers", duplicate_attackers),
        ("noncanonical attackers", noncanonical_attackers),
        ("dangling foundation source", dangling_foundation_source),
        (
            "future same-turn control boundary",
            future_same_turn_control_boundary,
        ),
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
