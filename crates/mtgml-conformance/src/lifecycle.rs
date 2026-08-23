//! M2.E synthetic lifecycle fixture program (Issue #52).
//!
//! Drives the production lifecycle primitives through the feature-gated
//! rules fixture support and asserts complete transition products without
//! fabricating a `DecisionResponseV2`. Every scenario here is authoritative
//! evidence for the three owned M2.E gates.

use mtgml_model::{
    CardDefinitionId, EpisodeStatus, FullStateDigestV3, GameObjectId, OpaqueObjectId,
    PhysicalCardId, PlayerId, VisibleSequence, ZoneKind,
};
use mtgml_rules::fixture_support::{FixtureTransition, PlannedOccurrence};
use mtgml_rules::{AuthoritativeRuleEvent, TransitionResult};
use mtgml_state::{
    construct_synthetic_engine_state, EngineState, IdentityMutationV1, KnowledgeAcquisitionCause,
    KnowledgeAcquisitionReason, KnowledgeHistoryChannel, KnowledgeInvalidationReason,
    KnowledgeMutationV1, KnownLocationFactV2, PerspectiveLifecycleAuditV1,
    PerspectiveLifecycleMutationV1, SyntheticResetInputs, VisibilityPartition, ZoneLocation,
    ZonePosition,
};

use crate::ConformanceFailure;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn seed() -> mtgml_random::RootSeed256 {
    mtgml_random::RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap()
}

/// Two-player fixture state: GO3/GO4 wait unseen in Exile; P1 initially
/// knows only opaque 1 -> GO1 (cursor 1, allocator 2).
pub fn lifecycle_fixture() -> EngineState {
    let mut state = construct_synthetic_engine_state(SyntheticResetInputs {
        players: [P1, P2],
        root_seed: seed(),
    })
    .unwrap();
    for index in 3..=4u64 {
        let object = GameObjectId(index);
        state.zones.objects.insert(
            object,
            mtgml_state::GameObject {
                id: object,
                physical_card: Some(PhysicalCardId(index)),
                card_definition: CardDefinitionId(index),
                owner: P1,
                controller: P1,
                tapped: false,
                face_down: false,
            },
        );
        state.zones.locations.insert(object, exile_location());
    }
    state.allocators.next_object_id = GameObjectId(5);
    // The lifecycle program is decision-free by construction: no pending
    // entry decision, no continuation chain can go stale across revisions.
    state.execution.pending_decision = None;
    state.execution.continuations.clear();
    state
}

fn exile_location() -> ZoneLocation {
    ZoneLocation {
        zone: ZoneKind::Exile,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    }
}

fn battlefield() -> ZoneLocation {
    ZoneLocation {
        zone: ZoneKind::Battlefield,
        player: None,
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::Public,
        partition: None,
    }
}

fn hidden_hand(player: PlayerId) -> ZoneLocation {
    ZoneLocation {
        zone: ZoneKind::Hand,
        player: Some(player),
        position: ZonePosition::Unordered,
        visibility: VisibilityPartition::OwnerOnly,
        partition: None,
    }
}

fn observed(
    sequence: u64,
    channel: KnowledgeHistoryChannel,
    cause: KnowledgeAcquisitionCause,
) -> KnowledgeAcquisitionReason {
    KnowledgeAcquisitionReason::Observed {
        channel,
        sequence: VisibleSequence(sequence),
        cause,
    }
}

fn occurrence(
    perspective: PlayerId,
    sequence: u64,
    identity: IdentityMutationV1,
    knowledge: Option<KnowledgeMutationV1>,
    observation: mtgml_rules::PerspectiveObservationPolicyV1,
) -> PlannedOccurrence {
    PlannedOccurrence {
        lifecycle: PerspectiveLifecycleAuditV1 {
            perspective,
            sequence: VisibleSequence(sequence),
            mutation: PerspectiveLifecycleMutationV1 {
                identity,
                knowledge,
            },
        },
        observation,
    }
}

/// S1+S2: reveal of an unknown object followed by a tracked incarnation
/// change into a hidden zone. Same opaque identity persists, the allocator
/// does not advance for the incarnation change.
pub fn scenario_reveal_then_tracked_incarnation(
    before: &EngineState,
) -> Result<TransitionResult, ConformanceFailure> {
    let mut transition = FixtureTransition::start(before).map_err(contract)?;
    let revealed = transition
        .move_object_incarnation(GameObjectId(3), battlefield())
        .map_err(contract)?;
    transition
        .apply_occurrence(occurrence(
            P1,
            1,
            IdentityMutationV1::Allocate {
                opaque: OpaqueObjectId(2),
                object: revealed,
            },
            Some(KnowledgeMutationV1::Acquire {
                opaque: OpaqueObjectId(2),
                definition: Some(CardDefinitionId(3)),
                location: Some(battlefield()),
                acquisition: observed(
                    1,
                    KnowledgeHistoryChannel::Public,
                    KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::Appeared {
                from_zone: ZoneKind::Exile,
                to_zone: ZoneKind::Battlefield,
                new_object: revealed,
            },
        ))
        .map_err(contract)?;
    let hidden = transition
        .move_object_incarnation(revealed, hidden_hand(P2))
        .map_err(contract)?;
    transition
        .apply_occurrence(occurrence(
            P1,
            2,
            IdentityMutationV1::Remap {
                opaque: OpaqueObjectId(2),
                from_object: revealed,
                to_object: hidden,
            },
            Some(KnowledgeMutationV1::CurrentToHistory {
                opaque: OpaqueObjectId(2),
                observed_definition: Some(CardDefinitionId(3)),
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::MovedInSight {
                from_zone: ZoneKind::Battlefield,
                to_zone: ZoneKind::Hand,
                old_object: revealed,
                new_object: hidden,
                reveals_old: true,
                reveals_new: false,
            },
        ))
        .map_err(contract)?;
    transition.finish().map_err(contract)
}

/// S5: explicit forget retires the live mapping and the knowledge record in
/// one occurrence. Deliberately public provenance (public/public_event).
pub fn scenario_explicit_forget(
    before: &EngineState,
) -> Result<TransitionResult, ConformanceFailure> {
    let mut transition = FixtureTransition::start(before).map_err(contract)?;
    transition
        .apply_occurrence(occurrence(
            P1,
            1,
            IdentityMutationV1::Retire {
                opaque: OpaqueObjectId(1),
                object: GameObjectId(1),
            },
            Some(KnowledgeMutationV1::Invalidate {
                opaque: OpaqueObjectId(1),
                reason: KnowledgeInvalidationReason::ExplicitForget,
                invalidation_provenance: observed(
                    1,
                    KnowledgeHistoryChannel::Public,
                    KnowledgeAcquisitionCause::PublicEvent,
                ),
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::NoEnvelope,
        ))
        .map_err(contract)?;
    transition.finish().map_err(contract)
}

/// S6 twin scenarios: randomization / shuffle destroy distinguishability of
/// every previously known object. Separate semantic occurrences, never two
/// reasons inside one occurrence.
pub fn scenario_indistinguishability(
    before: &EngineState,
    reason: KnowledgeInvalidationReason,
) -> Result<TransitionResult, ConformanceFailure> {
    let mut transition = FixtureTransition::start(before).map_err(contract)?;
    // Hidden randomization consumes authoritative randomness before every
    // known correspondence is destroyed by the shared hidden-set move.
    transition
        .record_hidden_random_sample(1000)
        .map_err(contract)?;
    let mut cursor = before.knowledge.players[&P1].next_visible_sequence.0;
    let identity_before = &before.perspective_identities.players[&P1];
    let known_pairs: Vec<(OpaqueObjectId, GameObjectId)> = identity_before
        .opaque_to_object
        .iter()
        .map(|(opaque, object)| (*opaque, *object))
        .collect();
    let mut p2_cursor = before.knowledge.players[&P2].next_visible_sequence.0;
    let p2_identity_before = &before.perspective_identities.players[&P2];
    for (opaque, object) in known_pairs {
        // Physical truth: objects not yet inside the shared hidden set leave
        // public sight into it; afterwards members are indistinguishable.
        if before.zones.locations.get(&object) != Some(&hidden_hand(P2)) {
            transition
                .move_object_incarnation(object, hidden_hand(P2))
                .map_err(contract)?;
        }
        // Every perspective that still retains the object loses it too.
        let p2_opaque = p2_identity_before.object_to_opaque.get(&object).copied();
        let p2_knows = p2_opquate_is_known(before, p2_opaque.as_ref());
        if p2_knows {
            transition
                .apply_occurrence(occurrence(
                    P2,
                    p2_cursor,
                    IdentityMutationV1::Retire {
                        opaque: p2_opaque.unwrap(),
                        object,
                    },
                    Some(KnowledgeMutationV1::Invalidate {
                        opaque: p2_opaque.unwrap(),
                        reason,
                        invalidation_provenance: observed(
                            p2_cursor,
                            KnowledgeHistoryChannel::Public,
                            KnowledgeAcquisitionCause::PublicEvent,
                        ),
                    }),
                    mtgml_rules::PerspectiveObservationPolicyV1::NoEnvelope,
                ))
                .map_err(contract)?;
            p2_cursor += 1;
        }
        transition
            .apply_occurrence(occurrence(
                P1,
                cursor,
                IdentityMutationV1::Retire { opaque, object },
                Some(KnowledgeMutationV1::Invalidate {
                    opaque,
                    reason,
                    invalidation_provenance: observed(
                        cursor,
                        KnowledgeHistoryChannel::Public,
                        KnowledgeAcquisitionCause::PublicEvent,
                    ),
                }),
                mtgml_rules::PerspectiveObservationPolicyV1::NoEnvelope,
            ))
            .map_err(contract)?;
        cursor += 1;
    }
    transition.finish().map_err(contract)
}

/// Later re-identification after retirement allocates the next deterministic
/// opaque id, which is never one of the retired ids.
pub fn scenario_reidentification(
    before: &EngineState,
) -> Result<TransitionResult, ConformanceFailure> {
    let mut transition = FixtureTransition::start(before).map_err(contract)?;
    let next_opaque = before.perspective_identities.players[&P1].next_opaque_object_id;
    let seen = transition
        .move_object_incarnation(GameObjectId(4), battlefield())
        .map_err(contract)?;
    transition
        .apply_occurrence(occurrence(
            P1,
            before.knowledge.players[&P1].next_visible_sequence.0,
            IdentityMutationV1::Allocate {
                opaque: next_opaque,
                object: seen,
            },
            Some(KnowledgeMutationV1::Acquire {
                opaque: next_opaque,
                definition: Some(CardDefinitionId(4)),
                location: Some(battlefield()),
                acquisition: observed(
                    before.knowledge.players[&P1].next_visible_sequence.0,
                    KnowledgeHistoryChannel::Public,
                    KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::Appeared {
                from_zone: ZoneKind::Exile,
                to_zone: ZoneKind::Battlefield,
                new_object: seen,
            },
        ))
        .map_err(contract)?;
    transition.finish().map_err(contract)
}

/// Private look plus an accepted public return: exercises UpdateLocation
/// transitions and ordered multi-update history on one retained record.
pub fn scenario_private_look_and_history(
    before: &EngineState,
) -> Result<TransitionResult, ConformanceFailure> {
    use mtgml_state::KnownLocationFactV2;
    let mut transition = FixtureTransition::start(before).map_err(contract)?;
    let revealed = transition
        .move_object_incarnation(GameObjectId(3), battlefield())
        .map_err(contract)?;
    transition
        .apply_occurrence(occurrence(
            P1,
            1,
            IdentityMutationV1::Allocate {
                opaque: OpaqueObjectId(2),
                object: revealed,
            },
            Some(KnowledgeMutationV1::Acquire {
                opaque: OpaqueObjectId(2),
                definition: Some(CardDefinitionId(3)),
                location: Some(battlefield()),
                acquisition: observed(
                    1,
                    KnowledgeHistoryChannel::Public,
                    KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::Appeared {
                from_zone: ZoneKind::Exile,
                to_zone: ZoneKind::Battlefield,
                new_object: revealed,
            },
        ))
        .map_err(contract)?;
    let hidden = transition
        .move_object_incarnation(revealed, hidden_hand(P2))
        .map_err(contract)?;
    transition
        .apply_occurrence(occurrence(
            P1,
            2,
            IdentityMutationV1::Remap {
                opaque: OpaqueObjectId(2),
                from_object: revealed,
                to_object: hidden,
            },
            Some(KnowledgeMutationV1::CurrentToHistory {
                opaque: OpaqueObjectId(2),
                observed_definition: None,
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::MovedInSight {
                from_zone: ZoneKind::Battlefield,
                to_zone: ZoneKind::Hand,
                old_object: revealed,
                new_object: hidden,
                reveals_old: true,
                reveals_new: false,
            },
        ))
        .map_err(contract)?;
    // Accepted UpdateLocation: the perspective looks at the object inside the
    // hidden zone (private/private_look) without any envelope.
    transition
        .apply_occurrence(occurrence(
            P1,
            3,
            IdentityMutationV1::None,
            Some(KnowledgeMutationV1::UpdateLocation {
                opaque: OpaqueObjectId(2),
                fact: KnownLocationFactV2 {
                    location: hidden_hand(P2),
                    provenance: observed(
                        3,
                        KnowledgeHistoryChannel::Private,
                        KnowledgeAcquisitionCause::PrivateLook,
                    ),
                },
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::NoEnvelope,
        ))
        .map_err(contract)?;
    let returned = transition
        .move_object_incarnation(hidden, battlefield())
        .map_err(contract)?;
    transition
        .apply_occurrence(occurrence(
            P1,
            4,
            IdentityMutationV1::Remap {
                opaque: OpaqueObjectId(2),
                from_object: hidden,
                to_object: returned,
            },
            Some(KnowledgeMutationV1::UpdateLocation {
                opaque: OpaqueObjectId(2),
                fact: KnownLocationFactV2 {
                    location: battlefield(),
                    provenance: observed(
                        4,
                        KnowledgeHistoryChannel::Public,
                        KnowledgeAcquisitionCause::PublicEvent,
                    ),
                },
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::MovedInSight {
                from_zone: ZoneKind::Hand,
                to_zone: ZoneKind::Battlefield,
                old_object: hidden,
                new_object: returned,
                reveals_old: true,
                reveals_new: true,
            },
        ))
        .map_err(contract)?;
    transition.finish().map_err(contract)
}
/// Public fan-out: one physical public occurrence authorized for both
/// perspectives produces one occurrence record per perspective with that
/// perspective's own next visible sequences (N/N+1 style independence).
pub fn scenario_public_fanout_two_reveals(
    before: &EngineState,
) -> Result<TransitionResult, ConformanceFailure> {
    let mut transition = FixtureTransition::start(before).map_err(contract)?;
    let first = transition
        .move_object_incarnation(GameObjectId(3), battlefield())
        .map_err(contract)?;
    let second = transition
        .move_object_incarnation(GameObjectId(4), battlefield())
        .map_err(contract)?;
    for perspective in [P1, P2] {
        let record = &before.perspective_identities.players[&perspective];
        let knowledge = &before.knowledge.players[&perspective];
        let mut cursor = knowledge.next_visible_sequence.0;
        let mut allocator = record.next_opaque_object_id;
        for object in [first, second] {
            transition
                .apply_occurrence(occurrence(
                    perspective,
                    cursor,
                    IdentityMutationV1::Allocate {
                        opaque: allocator,
                        object,
                    },
                    Some(KnowledgeMutationV1::Acquire {
                        opaque: allocator,
                        definition: None,
                        location: Some(battlefield()),
                        acquisition: observed(
                            cursor,
                            KnowledgeHistoryChannel::Public,
                            KnowledgeAcquisitionCause::ExplicitReveal,
                        ),
                    }),
                    mtgml_rules::PerspectiveObservationPolicyV1::Appeared {
                        from_zone: ZoneKind::Exile,
                        to_zone: ZoneKind::Battlefield,
                        new_object: object,
                    },
                ))
                .map_err(contract)?;
            cursor += 1;
            allocator = OpaqueObjectId(allocator.0 + 1);
        }
    }
    transition.finish().map_err(contract)
}

/// own-private acquisition through the generic no-envelope policy.
pub fn scenario_own_private_identity(
    before: &EngineState,
) -> Result<TransitionResult, ConformanceFailure> {
    let mut transition = FixtureTransition::start(before).map_err(contract)?;
    let received = transition
        .move_object_incarnation(GameObjectId(3), hidden_hand(P1))
        .map_err(contract)?;
    transition
        .apply_occurrence(occurrence(
            P1,
            1,
            IdentityMutationV1::Allocate {
                opaque: OpaqueObjectId(2),
                object: received,
            },
            Some(KnowledgeMutationV1::Acquire {
                opaque: OpaqueObjectId(2),
                definition: Some(CardDefinitionId(3)),
                location: Some(hidden_hand(P1)),
                acquisition: observed(
                    1,
                    KnowledgeHistoryChannel::Private,
                    KnowledgeAcquisitionCause::OwnPrivateIdentity,
                ),
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::NoEnvelope,
        ))
        .map_err(contract)?;
    transition.finish().map_err(contract)
}

/// Invalidating hidden transition, distinct from tracked hiding: the move is
/// publicly observed but correspondence becomes untrackable afterwards.
pub fn scenario_conceal_untrackable(
    before: &EngineState,
) -> Result<TransitionResult, ConformanceFailure> {
    let mut transition = FixtureTransition::start(before).map_err(contract)?;
    // Prepare: P1 knows GO3 as opaque 2 (public reveal, sequence 1).
    let revealed = transition
        .move_object_incarnation(GameObjectId(3), battlefield())
        .map_err(contract)?;
    transition
        .apply_occurrence(occurrence(
            P1,
            1,
            IdentityMutationV1::Allocate {
                opaque: OpaqueObjectId(2),
                object: revealed,
            },
            Some(KnowledgeMutationV1::Acquire {
                opaque: OpaqueObjectId(2),
                definition: Some(CardDefinitionId(3)),
                location: Some(battlefield()),
                acquisition: observed(
                    1,
                    KnowledgeHistoryChannel::Public,
                    KnowledgeAcquisitionCause::ExplicitReveal,
                ),
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::Appeared {
                from_zone: ZoneKind::Exile,
                to_zone: ZoneKind::Battlefield,
                new_object: revealed,
            },
        ))
        .map_err(contract)?;
    // Publicly seen move into a hidden region destroys trackability.
    let hidden = transition
        .move_object_incarnation(revealed, hidden_hand(P2))
        .map_err(contract)?;
    transition
        .apply_occurrence(occurrence(
            P1,
            2,
            IdentityMutationV1::Retire {
                opaque: OpaqueObjectId(2),
                object: revealed,
            },
            Some(KnowledgeMutationV1::Invalidate {
                opaque: OpaqueObjectId(2),
                reason: KnowledgeInvalidationReason::HiddenTransition,
                invalidation_provenance: observed(
                    2,
                    KnowledgeHistoryChannel::Public,
                    KnowledgeAcquisitionCause::PublicEvent,
                ),
            }),
            mtgml_rules::PerspectiveObservationPolicyV1::MovedInSight {
                from_zone: ZoneKind::Battlefield,
                to_zone: ZoneKind::Hand,
                old_object: revealed,
                new_object: hidden,
                reveals_old: true,
                reveals_new: false,
            },
        ))
        .map_err(contract)?;
    transition.finish().map_err(contract)
}

fn contract(error: mtgml_rules::KernelExecutionError) -> ConformanceFailure {
    ConformanceFailure::Contract(error.to_string())
}

/// Generic exact-product assertion: no decision response involved.
pub fn assert_exact_transition_product(
    before: &EngineState,
    result: &TransitionResult,
    expected_events: &[AuthoritativeRuleEvent],
    expected_digest: &FullStateDigestV3,
) -> Result<(), ConformanceFailure> {
    mtgml_rules::validate_transition_contract(before, result)
        .map_err(|error| ConformanceFailure::Contract(error.to_string()))?;
    if result.events != expected_events {
        return Err(ConformanceFailure::Events);
    }
    if result
        .next_state
        .digest()
        .map_err(|_| ConformanceFailure::StateDigest)?
        != *expected_digest
    {
        return Err(ConformanceFailure::StateDigest);
    }
    if !matches!(result.status, EpisodeStatus::Running) {
        return Err(ConformanceFailure::Status);
    }
    Ok(())
}

fn p2_opquate_is_known(before: &EngineState, opaque: Option<&OpaqueObjectId>) -> bool {
    match opaque {
        Some(opaque) => before.knowledge.players[&P2].active.contains_key(opaque),
        None => false,
    }
}

#[cfg(test)]
mod gate_evidence {
    use super::*;

    #[test]
    fn tracked_incarnation_persists_opaque_without_allocator_advance() {
        let before = lifecycle_fixture();
        let result = scenario_reveal_then_tracked_incarnation(&before).unwrap();
        let identity = &result.next_state.perspective_identities.players[&P1];
        // GO3(5) -> hidden incarnation: same opaque 2, remapped to GO6.
        assert_eq!(
            identity.opaque_to_object.get(&OpaqueObjectId(2)),
            Some(&GameObjectId(6))
        );
        assert_eq!(identity.next_opaque_object_id, OpaqueObjectId(3));
        let knowledge = &result.next_state.knowledge.players[&P1];
        assert_eq!(knowledge.next_visible_sequence, VisibleSequence(3));
        let record = &knowledge.active[&OpaqueObjectId(2)];
        assert!(record.known_location.is_none());
        assert_eq!(record.historical_locations.len(), 1);
    }

    #[test]
    fn explicit_forget_retires_mapping_and_knowledge_together() {
        let before = lifecycle_fixture();
        let result = scenario_explicit_forget(&before).unwrap();
        let identity = &result.next_state.perspective_identities.players[&P1];
        assert!(!identity.opaque_to_object.contains_key(&OpaqueObjectId(1)));
        assert!(identity.retired_object_ids.contains(&OpaqueObjectId(1)));
        assert_eq!(identity.next_opaque_object_id, OpaqueObjectId(2));
        let retired = &result.next_state.knowledge.players[&P1].retired[&OpaqueObjectId(1)];
        assert_eq!(
            retired.invalidation.reason,
            KnowledgeInvalidationReason::ExplicitForget
        );
    }

    #[test]
    fn randomization_and_shuffle_are_distinct_retirement_scenarios() {
        for reason in [
            KnowledgeInvalidationReason::Randomization,
            KnowledgeInvalidationReason::Shuffle,
        ] {
            let mut state = lifecycle_fixture();
            let revealed = scenario_reveal_then_tracked_incarnation(&state).unwrap();
            state = revealed.next_state;
            let before_cursor = state.knowledge.players[&P1].next_visible_sequence;
            let result = scenario_indistinguishability(&state, reason).unwrap();
            let identity = &result.next_state.perspective_identities.players[&P1];
            assert!(identity.retired_object_ids.contains(&OpaqueObjectId(1)));
            assert!(identity.retired_object_ids.contains(&OpaqueObjectId(2)));
            assert!(identity.opaque_to_object.is_empty());
            let retired = &result.next_state.knowledge.players[&P1].retired;
            assert_eq!(retired[&OpaqueObjectId(1)].invalidation.reason, reason);
            assert_eq!(retired[&OpaqueObjectId(2)].invalidation.reason, reason);
            assert_eq!(
                result.next_state.knowledge.players[&P1].next_visible_sequence,
                VisibleSequence(before_cursor.0 + 2)
            );
        }
    }

    #[test]
    fn reidentification_after_randomization_allocates_next_unused_never_reused() {
        let mut state = lifecycle_fixture();
        state = scenario_reveal_then_tracked_incarnation(&state)
            .unwrap()
            .next_state;
        state = scenario_indistinguishability(&state, KnowledgeInvalidationReason::Randomization)
            .unwrap()
            .next_state;
        let next_before = state.perspective_identities.players[&P1].next_opaque_object_id;
        let result = scenario_reidentification(&state).unwrap();
        let identity = &result.next_state.perspective_identities.players[&P1];
        assert!(identity.retired_object_ids.contains(&OpaqueObjectId(1)));
        assert!(identity.retired_object_ids.contains(&OpaqueObjectId(2)));
        assert!(!identity.retired_object_ids.contains(&next_before));
        assert!(identity.opaque_to_object.contains_key(&next_before));
        assert_eq!(
            identity.next_opaque_object_id,
            OpaqueObjectId(next_before.0 + 1)
        );
    }

    #[test]
    fn public_fanout_assigns_independent_perspective_sequences() {
        let before = lifecycle_fixture();
        let result = scenario_public_fanout_two_reveals(&before).unwrap();
        for (player, first_opaque) in [(P1, OpaqueObjectId(2)), (P2, OpaqueObjectId(3))] {
            let identity = &result.next_state.perspective_identities.players[&player];
            let knowledge = &result.next_state.knowledge.players[&player];
            assert_eq!(knowledge.next_visible_sequence, VisibleSequence(3));
            assert_eq!(
                identity.opaque_to_object.get(&first_opaque),
                Some(&GameObjectId(5))
            );
            assert_eq!(
                identity
                    .opaque_to_object
                    .get(&OpaqueObjectId(first_opaque.0 + 1)),
                Some(&GameObjectId(6))
            );
        }
        // Perspectives allocate independent opaque ids for the same objects.
        assert_ne!(
            result.next_state.perspective_identities.players[&P1]
                .opaque_to_object
                .get(&OpaqueObjectId(2)),
            result.next_state.perspective_identities.players[&P2]
                .opaque_to_object
                .get(&OpaqueObjectId(2))
        );
    }

    #[test]
    fn own_private_identity_acquisition_uses_no_envelope_policy() {
        let before = lifecycle_fixture();
        let result = scenario_own_private_identity(&before).unwrap();
        let record = &result.next_state.knowledge.players[&P1].active[&OpaqueObjectId(2)];
        match record.acquisition {
            KnowledgeAcquisitionReason::Observed {
                cause: KnowledgeAcquisitionCause::OwnPrivateIdentity,
                channel: KnowledgeHistoryChannel::Private,
                ..
            } => {}
            other => panic!("unexpected acquisition {other:?}"),
        }
    }

    #[test]
    fn conceal_untrackable_retires_with_hidden_transition_reason() {
        let before = lifecycle_fixture();
        let result = scenario_conceal_untrackable(&before).unwrap();
        let retired = &result.next_state.knowledge.players[&P1].retired[&OpaqueObjectId(2)];
        assert_eq!(
            retired.invalidation.reason,
            KnowledgeInvalidationReason::HiddenTransition
        );
        assert!(result.next_state.perspective_identities.players[&P1]
            .retired_object_ids
            .contains(&OpaqueObjectId(2)));
    }
}

#[cfg(test)]
mod gate_evidence_history {
    use super::*;

    #[test]
    fn private_look_and_public_return_order_history_strictly() {
        let before = lifecycle_fixture();
        let result = scenario_private_look_and_history(&before).unwrap();
        let record = &result.next_state.knowledge.players[&P1].active[&OpaqueObjectId(2)];
        assert_eq!(record.historical_locations.len(), 2);
        let sequences: Vec<u64> = record
            .historical_locations
            .iter()
            .filter_map(|fact| fact.provenance.observed_sequence())
            .map(|sequence| sequence.0)
            .collect();
        assert_eq!(sequences, vec![1, 3]);
        assert_eq!(
            record
                .known_location
                .as_ref()
                .unwrap()
                .provenance
                .observed_sequence(),
            Some(VisibleSequence(4))
        );
        assert_eq!(
            result.next_state.knowledge.players[&P1].next_visible_sequence,
            VisibleSequence(5)
        );
    }

    #[test]
    fn own_private_identity_acquisition_is_owned_gate_evidence() {
        let before = lifecycle_fixture();
        scenario_own_private_identity(&before).unwrap();
    }

    #[test]
    fn public_fanout_is_owned_gate_evidence() {
        let before = lifecycle_fixture();
        let result = scenario_public_fanout_two_reveals(&before).unwrap();
        assert_eq!(
            result.next_state.perspective_identities.players[&P1]
                .opaque_to_object
                .len(),
            3
        );
    }
}
