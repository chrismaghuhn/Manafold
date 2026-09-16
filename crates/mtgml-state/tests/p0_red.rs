use std::collections::BTreeMap;

use mtgml_model::{FullStateDigestV4, GameObjectId, PlayerId};
use mtgml_random::RootSeed256;
use mtgml_state::{
    construct_synthetic_engine_state, BaseCharacteristics, BeginningStep, CombatState,
    ControlHistory, FoundationCreatureSource, FoundationSourceKind, PriorityState,
    SyntheticResetInputs, SyntheticV4Setup, TurnPosition,
};

fn current_digest_is_v4(state: &mtgml_state::EngineState) -> FullStateDigestV4 {
    state.digest().unwrap()
}

#[test]
fn p0_synthetic_reset_receives_v4_facts_as_explicit_setup_input() {
    let setup = SyntheticV4Setup {
        position: TurnPosition::Beginning(BeginningStep::Untap),
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
        TurnPosition::Beginning(BeginningStep::Untap)
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
