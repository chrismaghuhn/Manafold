// Event to semantic-delta mapping of the turn events the V3 progression emits.

use mtgml_state::{BeginningStep, TurnPosition};

fn turn_position_beginning() -> TurnPosition {
    TurnPosition::Beginning {
        step: BeginningStep::Untap,
    }
}

fn event_turn_number_changed(from: u64, to: u64) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::TurnNumberChanged { from, to },
    }
}

fn event_turn_position_changed(from: TurnPosition, to: TurnPosition) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::TurnPositionChanged { from, to },
    }
}

fn event_active_player_changed(from: PlayerId, to: PlayerId) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::ActivePlayerChanged { from, to },
    }
}

fn event_untap_completed(objects: Vec<GameObjectId>) -> AuthoritativeRuleEvent {
    AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::UntapCompleted { affected_objects: objects },
    }
}

// --- Event→delta mapping ---

#[test]
fn turn_number_changed_maps_to_turn_number_changed_delta() {
    let event = event_turn_number_changed(1, 2);
    assert!(matches!(
        event.event.semantic_delta(),
        SemanticDeltaOperation::TurnNumberChanged { from: 1, to: 2 }
    ));
}

#[test]
fn turn_position_changed_maps_to_turn_position_changed_delta() {
    let from = turn_position_beginning();
    let to = TurnPosition::PrecombatMain;
    let event = event_turn_position_changed(from, to);
    assert!(matches!(
        event.event.semantic_delta(),
        SemanticDeltaOperation::TurnPositionChanged { from: f, to: t }
        if f == from && t == to
    ));
}

#[test]
fn active_player_changed_maps_to_active_player_changed_delta() {
    let event = event_active_player_changed(PlayerId(1), PlayerId(2));
    assert!(matches!(
        event.event.semantic_delta(),
        SemanticDeltaOperation::ActivePlayerChanged { from: p1, to: p2 }
        if p1 == PlayerId(1) && p2 == PlayerId(2)
    ));
}

#[test]
fn untap_completed_maps_to_untap_completed_delta() {
    let objects = vec![GameObjectId(1), GameObjectId(2)];
    let event = event_untap_completed(objects.clone());
    assert!(matches!(
        event.event.semantic_delta(),
        SemanticDeltaOperation::UntapCompleted { affected_objects: a }
        if a == objects
    ));
}
