//! M3.S2 direct-request RED cases and substrate characterization.
//!
//! Positive cases deliberately stop at the private production-owned seam.
//! Their independently authored expected products remain here as the Task 2
//! oracle; FIX-01 must not calculate any part of those products.

use mtgml_model::{CardDefinitionId, GameObjectId, PhysicalCardId, PlayerId, ZoneKind};
use mtgml_rules::{
    execute_selected_zone_transition_for_conformance, ConformanceZoneTransitionKind,
    KernelExecutionError,
};
use mtgml_state::{
    construct_synthetic_engine_state, validate_engine_state, BaseCharacteristics, ControlHistory,
    EngineState, FoundationCreatureSource, FoundationSourceKind, GameObject, ObjectSnapshot,
    SyntheticResetInputs, SyntheticV4Setup, VisibilityPartition, ZoneKey, ZoneLocation,
    ZonePosition,
};
use std::collections::BTreeMap;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const OLD_BATTLEFIELD: GameObjectId = GameObjectId(1);
const OLD_LIBRARY_TOP: GameObjectId = GameObjectId(2);
const CARD_BATTLEFIELD: PhysicalCardId = PhysicalCardId(1);
const CARD_LIBRARY: PhysicalCardId = PhysicalCardId(2);

fn base_state() -> EngineState {
    let mut state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [P1, P2],
        root_seed: mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap();
    // These direct primitive cases do not carry or fabricate a player response.
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    state
}

fn location(
    zone: ZoneKind,
    player: Option<PlayerId>,
    position: ZonePosition,
    visibility: VisibilityPartition,
) -> ZoneLocation {
    ZoneLocation {
        zone,
        player,
        position,
        visibility,
        partition: None,
    }
}

fn battlefield_case_state() -> EngineState {
    let mut state = base_state();
    let graveyard = location(
        ZoneKind::Graveyard,
        Some(P1),
        ZonePosition::Top { offset: 0 },
        VisibilityPartition::Public,
    );
    let second = location(
        ZoneKind::Graveyard,
        Some(P1),
        ZonePosition::Top { offset: 1 },
        VisibilityPartition::Public,
    );
    for (id, definition, physical, zone_location) in [
        (GameObjectId(3), 3, 3, graveyard.clone()),
        (GameObjectId(4), 4, 4, second),
    ] {
        state.zones.objects.insert(
            id,
            GameObject {
                id,
                physical_card: Some(PhysicalCardId(physical)),
                card_definition: CardDefinitionId(definition),
                owner: P1,
                controller: P1,
                tapped: false,
                face_down: false,
            },
        );
        state.zones.locations.insert(id, zone_location);
    }
    state.zones.ordered_zones.insert(
        ZoneKey {
            zone: ZoneKind::Graveyard,
            player: Some(P1),
            visibility: VisibilityPartition::Public,
            partition: None,
        },
        vec![GameObjectId(3), GameObjectId(4)],
    );
    state.allocators.next_object_id = GameObjectId(5);
    state.foundation_sources.insert(
        OLD_BATTLEFIELD,
        FoundationCreatureSource {
            source_kind: FoundationSourceKind::Creature,
            base_characteristics: BaseCharacteristics::Simple {
                power: 2,
                toughness: 2,
            },
            marked_damage: 1,
            control_history: ControlHistory::BeforeTurnStart { turn_number: 0 },
        },
    );
    validate_engine_state(&state).expect("authored battlefield case state is valid");
    state
}

fn library_case_state() -> EngineState {
    let mut state = base_state();
    // The zone represents hidden status; the card itself is not face-down.
    state
        .zones
        .objects
        .get_mut(&OLD_LIBRARY_TOP)
        .unwrap()
        .face_down = false;
    let second = GameObjectId(3);
    let library_top_one = location(
        ZoneKind::Library,
        Some(P2),
        ZonePosition::Top { offset: 1 },
        VisibilityPartition::FaceDown,
    );
    state.zones.objects.insert(
        second,
        GameObject {
            id: second,
            physical_card: Some(PhysicalCardId(3)),
            card_definition: CardDefinitionId(3),
            owner: P2,
            controller: P2,
            tapped: false,
            face_down: false,
        },
    );
    state.zones.locations.insert(second, library_top_one);
    state.zones.ordered_zones.insert(
        ZoneKey {
            zone: ZoneKind::Library,
            player: Some(P2),
            visibility: VisibilityPartition::FaceDown,
            partition: None,
        },
        vec![OLD_LIBRARY_TOP, second],
    );
    state.allocators.next_object_id = GameObjectId(4);
    validate_engine_state(&state).expect("authored library case state is valid");
    state
}

fn battlefield_request() -> Result<mtgml_rules::TransitionResult, KernelExecutionError> {
    execute_selected_zone_transition_for_conformance(
        &battlefield_case_state(),
        OLD_BATTLEFIELD,
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    )
}

fn library_request() -> Result<mtgml_rules::TransitionResult, KernelExecutionError> {
    execute_selected_zone_transition_for_conformance(
        &library_case_state(),
        OLD_LIBRARY_TOP,
        ConformanceZoneTransitionKind::LibraryTopToOwnerHand,
    )
}

fn assert_s2_red<T>(case_id: &str, actual: Result<T, KernelExecutionError>) -> T {
    actual.unwrap_or_else(|error| {
        panic!("RED {case_id}: expected selected S2 product; authoritative seam returned {error}")
    })
}

#[test]
fn s2_zone_battlefield_graveyard() {
    let before = battlefield_case_state();
    assert_eq!(before.allocators.next_object_id, GameObjectId(5));
    // Independent expected vector and redundant position witnesses:
    // [NEW(5), G0(3), G1(4)] at offsets 0, 1, 2.
    let expected_order = vec![GameObjectId(5), GameObjectId(3), GameObjectId(4)];
    let expected_positions = [
        (GameObjectId(5), ZonePosition::Top { offset: 0 }),
        (GameObjectId(3), ZonePosition::Top { offset: 1 }),
        (GameObjectId(4), ZonePosition::Top { offset: 2 }),
    ];
    assert_eq!(
        expected_order,
        [GameObjectId(5), GameObjectId(3), GameObjectId(4)]
    );
    assert_eq!(expected_positions[2].1, ZonePosition::Top { offset: 2 });
    let _ = assert_s2_red(
        "s2.zone.battlefield_graveyard",
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
    );
}

#[test]
fn s2_zone_library_hand_top() {
    let before = library_case_state();
    assert_eq!(
        before.zones.ordered_zones.values().next().unwrap(),
        &[OLD_LIBRARY_TOP, GameObjectId(3)]
    );
    // Independent expected source remainder and destination identity.
    let expected_remaining_library = vec![GameObjectId(3)];
    let expected_new_object = GameObjectId(4);
    assert_eq!(expected_remaining_library, [GameObjectId(3)]);
    assert_eq!(expected_new_object, GameObjectId(4));
    let _ = assert_s2_red("s2.zone.library_hand_top", library_request());
}

#[test]
fn s2_zone_graveyard_order() {
    let before = battlefield_case_state();
    let key = before.zones.locations.get(&GameObjectId(3)).unwrap().key();
    assert_eq!(
        before.zones.ordered_zones[&key],
        [GameObjectId(3), GameObjectId(4)]
    );
    // Literal expected post-state; do not call production ordering helpers.
    let expected = [
        (GameObjectId(5), ZonePosition::Top { offset: 0 }),
        (GameObjectId(3), ZonePosition::Top { offset: 1 }),
        (GameObjectId(4), ZonePosition::Top { offset: 2 }),
    ];
    assert_eq!(expected[0].1, ZonePosition::Top { offset: 0 });
    assert_eq!(expected[1].1, ZonePosition::Top { offset: 1 });
    assert_eq!(expected[2].1, ZonePosition::Top { offset: 2 });
    let _ = assert_s2_red(
        "s2.zone.graveyard_order",
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
    );
}

#[test]
fn s2_zone_event_delta_cursor_expectations() {
    let before = battlefield_case_state();
    let old_snapshot = ObjectSnapshot {
        object: OLD_BATTLEFIELD,
        physical_card: Some(CARD_BATTLEFIELD),
        card_definition: CardDefinitionId(1),
        owner: P1,
        controller: P1,
        tapped: false,
        face_down: false,
        location: location(
            ZoneKind::Battlefield,
            None,
            ZonePosition::Unordered,
            VisibilityPartition::Public,
        ),
    };
    let expected_transition = mtgml_state::ZoneTransition {
        old_object: OLD_BATTLEFIELD,
        new_object: GameObjectId(5),
        physical_card: Some(CARD_BATTLEFIELD),
        from: location(
            ZoneKind::Battlefield,
            None,
            ZonePosition::Unordered,
            VisibilityPartition::Public,
        ),
        to: location(
            ZoneKind::Graveyard,
            Some(P1),
            ZonePosition::Top { offset: 0 },
            VisibilityPartition::Public,
        ),
        last_known: old_snapshot,
        new_snapshot: ObjectSnapshot {
            object: GameObjectId(5),
            physical_card: Some(CARD_BATTLEFIELD),
            card_definition: CardDefinitionId(1),
            owner: P1,
            controller: P1,
            tapped: false,
            face_down: false,
            location: location(
                ZoneKind::Graveyard,
                Some(P1),
                ZonePosition::Top { offset: 0 },
                VisibilityPartition::Public,
            ),
        },
    };
    let expected_event = mtgml_rules::AuthoritativeRuleEventKind::ZoneTransition {
        transition: Box::new(expected_transition.clone()),
    };
    let expected_delta = mtgml_state::SemanticDeltaOperation::ZoneTransition {
        transition: Box::new(expected_transition),
    };
    assert_eq!(expected_event.semantic_delta(), expected_delta);
    assert_eq!(before.revision.0.checked_add(1), Some(1));
    let _ = assert_s2_red(
        "s2.zone.event_delta_cursor_expectations",
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
    );
}

#[test]
fn s2_identity_destination_canonical_state() {
    // `controller = owner` is GameObject storage normalization for non-
    // battlefield rows, not a Magic rule assigning those cards a controller.
    let battlefield_expected = GameObject {
        id: GameObjectId(5),
        physical_card: Some(CARD_BATTLEFIELD),
        card_definition: CardDefinitionId(1),
        owner: P1,
        controller: P1,
        tapped: false,
        face_down: false,
    };
    let library_expected = GameObject {
        id: GameObjectId(4),
        physical_card: Some(CARD_LIBRARY),
        card_definition: CardDefinitionId(2),
        owner: P2,
        controller: P2,
        tapped: false,
        face_down: false,
    };
    let expected_battlefield_snapshot = ObjectSnapshot {
        object: GameObjectId(5),
        physical_card: Some(CARD_BATTLEFIELD),
        card_definition: CardDefinitionId(1),
        owner: P1,
        controller: P1,
        tapped: false,
        face_down: false,
        location: location(
            ZoneKind::Graveyard,
            Some(P1),
            ZonePosition::Top { offset: 0 },
            VisibilityPartition::Public,
        ),
    };
    let expected_library_snapshot = ObjectSnapshot {
        object: GameObjectId(4),
        physical_card: Some(CARD_LIBRARY),
        card_definition: CardDefinitionId(2),
        owner: P2,
        controller: P2,
        tapped: false,
        face_down: false,
        location: location(
            ZoneKind::Hand,
            Some(P2),
            ZonePosition::Unordered,
            VisibilityPartition::OwnerOnly,
        ),
    };
    assert_eq!(battlefield_expected.controller, battlefield_expected.owner);
    assert_eq!(library_expected.controller, library_expected.owner);
    assert_eq!(expected_battlefield_snapshot.object, GameObjectId(5));
    assert_eq!(
        expected_battlefield_snapshot.location.zone,
        ZoneKind::Graveyard
    );
    assert_eq!(expected_library_snapshot.object, GameObjectId(4));
    assert_eq!(expected_library_snapshot.location.zone, ZoneKind::Hand);
    let _ = assert_s2_red(
        "s2.identity.destination_canonical_state/battlefield",
        battlefield_request(),
    );
}

#[test]
fn s2_identity_destination_canonical_state_library() {
    let expected = ObjectSnapshot {
        object: GameObjectId(4),
        physical_card: Some(CARD_LIBRARY),
        card_definition: CardDefinitionId(2),
        owner: P2,
        controller: P2,
        tapped: false,
        face_down: false,
        location: location(
            ZoneKind::Hand,
            Some(P2),
            ZonePosition::Unordered,
            VisibilityPartition::OwnerOnly,
        ),
    };
    assert_eq!(expected.physical_card, Some(CARD_LIBRARY));
    assert_eq!(expected.owner, P2);
    assert_eq!(expected.controller, P2);
    assert!(!expected.tapped);
    assert!(!expected.face_down);
    let _ = assert_s2_red(
        "s2.identity.destination_canonical_state/library",
        library_request(),
    );
}

#[test]
fn s2_identity_old_reference_closure_characterization() {
    const CASE_ID: &str = "s2.identity.old_reference_closure";
    let state = battlefield_case_state();
    let old = OLD_BATTLEFIELD;
    // Pin each current authoritative reference category in this validated
    // state. S2 admission requires all non-source sites to be absent.
    let refs = BTreeMap::from([
        ("zones.objects", state.zones.objects.contains_key(&old)),
        ("zones.locations", state.zones.locations.contains_key(&old)),
        (
            "ordered_zones",
            state
                .zones
                .ordered_zones
                .values()
                .any(|ids| ids.contains(&old)),
        ),
        (
            "foundation_sources",
            state.foundation_sources.contains_key(&old),
        ),
        (
            "combat.attackers",
            state
                .combat
                .as_ref()
                .is_some_and(|c| c.attackers.contains(&old)),
        ),
        (
            "combat.blocker_keys_or_values",
            state.combat.as_ref().is_some_and(|c| {
                c.blockers
                    .iter()
                    .any(|(attacker, blocker)| *attacker == old || *blocker == Some(old))
            }),
        ),
        (
            "stack_records",
            state
                .zones
                .stack_records
                .values()
                .any(|r| r.source_object == Some(old)),
        ),
        (
            "pending_trusted_bindings",
            state.execution.pending_decision.as_ref().is_some_and(|p| {
                p.request
                    .candidates
                    .iter()
                    .any(|c| match &c.trusted_binding {
                        mtgml_decision::EngineCandidateBinding::CastSpell { object }
                        | mtgml_decision::EngineCandidateBinding::SelectObject { object } => {
                            *object == old
                        }
                        _ => false,
                    })
            }),
        ),
        (
            "perspective_live_mappings",
            state
                .perspective_identities
                .players
                .values()
                .any(|identity| identity.object_to_opaque.contains_key(&old)),
        ),
    ]);
    assert_eq!(refs["zones.objects"], true, "{CASE_ID}");
    assert_eq!(refs["zones.locations"], true, "{CASE_ID}");
    assert_eq!(refs["ordered_zones"], false, "{CASE_ID}");
    assert_eq!(refs["foundation_sources"], true, "{CASE_ID}");
    assert_eq!(refs["combat.attackers"], false, "{CASE_ID}");
    assert_eq!(refs["combat.blocker_keys_or_values"], false, "{CASE_ID}");
    assert_eq!(refs["stack_records"], false, "{CASE_ID}");
    assert_eq!(refs["pending_trusted_bindings"], false, "{CASE_ID}");
    assert_eq!(refs["perspective_live_mappings"], true, "{CASE_ID}");
    // These current shapes contain no GameObjectId fields: effects, waiting
    // triggers, delayed effects, continuations, and FormatState.
    assert!(state.execution.effects.is_empty());
    assert!(state.execution.waiting_triggers.is_empty());
    assert!(state.execution.delayed_effects.is_empty());
    assert!(state.execution.continuations.is_empty());
    assert!(matches!(state.format, mtgml_state::FormatState::None));
}

#[test]
fn s2_identity_foundation_source_cessation() {
    let before = battlefield_case_state();
    assert!(before.foundation_sources.contains_key(&OLD_BATTLEFIELD));
    // Independently authored expected destination has no source record.
    let expected_new = GameObjectId(5);
    assert!(!before.foundation_sources.contains_key(&expected_new));
    let _ = assert_s2_red(
        "s2.identity.foundation_source_cessation",
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
    );
}

#[test]
fn s2_rejection_stale_old_incarnation_historical_witness() {
    let before = battlefield_case_state();
    // Step 1 must be a real accepted production-owned transition. The intended
    // Step 2 is already explicit below; FIX-01 RED stops at Step 1.
    let first = assert_s2_red(
        "s2.rejection.stale_old_incarnation/step_1",
        execute_selected_zone_transition_for_conformance(
            &before,
            OLD_BATTLEFIELD,
            ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
        ),
    );
    let after_first = first.next_state.clone();
    let before_retry = DirectProductFingerprint::capture(&first);
    let retry = execute_selected_zone_transition_for_conformance(
        &after_first,
        OLD_BATTLEFIELD,
        ConformanceZoneTransitionKind::BattlefieldToOwnerGraveyard,
    );
    assert!(retry.is_err(), "stale OLD must be rejected");
    assert_eq!(DirectProductFingerprint::capture(&first), before_retry);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectProductFingerprint {
    state: EngineState,
    digest: mtgml_model::FullStateDigestV4,
    events: Vec<mtgml_rules::AuthoritativeRuleEvent>,
    delta: mtgml_state::StateDelta,
    next_decision: Option<mtgml_decision::AuthoritativeDecisionRequestV2>,
    status: mtgml_model::EpisodeStatus,
}

impl DirectProductFingerprint {
    fn capture(result: &mtgml_rules::TransitionResult) -> Self {
        Self {
            state: result.next_state.clone(),
            digest: result.next_state.digest().unwrap(),
            events: result.events.clone(),
            delta: result.delta.clone(),
            next_decision: result.next_decision.clone(),
            status: result.status.clone(),
        }
    }
}

#[test]
fn s2_mutant_stale_old_lifecycle_occurrence_is_validator_only() {
    // This is an internal candidate-product mutant, never a typed S2 request.
    // FixtureTransition supplies only a valid base ZoneTransition product;
    // this test mutates its trusted audit and calls the real validator.
    use mtgml_model::{RuleEventId, StateRevision, VisibleSequence};
    use mtgml_rules::fixture_support::FixtureTransition;
    use mtgml_rules::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
    use mtgml_state::{
        IdentityMutationV1, PerspectiveLifecycleAuditV1, PerspectiveLifecycleMutationV1,
    };

    let mut before = base_state();
    let old = GameObjectId(3);
    before.zones.objects.insert(
        old,
        GameObject {
            id: old,
            physical_card: Some(PhysicalCardId(3)),
            card_definition: CardDefinitionId(3),
            owner: P1,
            controller: P1,
            tapped: false,
            face_down: false,
        },
    );
    before.zones.locations.insert(
        old,
        location(
            ZoneKind::Exile,
            None,
            ZonePosition::Unordered,
            VisibilityPartition::Public,
        ),
    );
    before.allocators.next_object_id = GameObjectId(4);
    validate_engine_state(&before).unwrap();
    let mut candidate = FixtureTransition::start(&before).unwrap();
    candidate
        .move_object_incarnation(
            old,
            location(
                ZoneKind::Battlefield,
                None,
                ZonePosition::Unordered,
                VisibilityPartition::Public,
            ),
        )
        .unwrap();
    let mut result = candidate.finish().unwrap();

    let stale_lifecycle = PerspectiveLifecycleAuditV1 {
        perspective: P1,
        sequence: VisibleSequence(1),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::Allocate {
                opaque: mtgml_model::OpaqueObjectId(2),
                object: old,
            },
            knowledge: None,
        },
    };
    result.events.push(AuthoritativeRuleEvent {
        event_id: RuleEventId(2),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::PerspectiveOccurrence {
            lifecycle: stale_lifecycle.clone(),
            observation: mtgml_rules::PerspectiveObservationPolicyV1::NoEnvelope,
        },
    });
    result
        .delta
        .audit
        .push(mtgml_state::SemanticDeltaOperation::PerspectiveLifecycle {
            lifecycle: stale_lifecycle,
        });
    let error = mtgml_rules::validate_transition_contract(&before, &result).unwrap_err();
    assert!(
        matches!(error, mtgml_rules::TransitionViolation::OccurrencePairing),
        "RED s2.mutant.stale_old_lifecycle_occurrence: expected semantic cursor rejection for OLD after ZoneTransition"
    );
}

#[test]
fn s2_requests_fail_closed_without_mutation() {
    let battlefield = battlefield_case_state();
    let library = library_case_state();
    let battlefield_digest = battlefield.digest().unwrap();
    let library_digest = library.digest().unwrap();
    let battlefield_rng = battlefield.random.clone();
    let library_rng = library.random.clone();
    assert!(matches!(
        battlefield_request(),
        Err(KernelExecutionError::ZoneIncarnationUnavailable)
    ));
    assert!(matches!(
        library_request(),
        Err(KernelExecutionError::ZoneIncarnationUnavailable)
    ));
    // FIX-01 authorization requires all requests to remain fail-closed and
    // read-only; these checks pass independently of the RED product tests.
    assert_eq!(battlefield.digest().unwrap(), battlefield_digest);
    assert_eq!(library.digest().unwrap(), library_digest);
    assert_eq!(battlefield.random, battlefield_rng);
    assert_eq!(library.random, library_rng);
}
