//! State-based actions (CR 704) of the slice of lands and vanilla creatures.
//!
//! Whenever a player would get priority the game checks the state-based
//! actions, performs all that apply simultaneously as a single event, and
//! checks again until none applies (CR 704.3). This slice has two:
//! - a player with 0 or less life loses the game (CR 704.5a);
//! - a creature with toughness greater than 0 and at least that much damage
//!   marked on it is destroyed (CR 704.5g): it is removed from combat (CR 506.4)
//!   and put into its owner's graveyard (CR 701.8a) as a new object (CR 400.7).
//!   No card of this slice has a toughness of 0 or less (CR 704.5f).
//!
//! Two or more cards put into one graveyard at the same time are arranged by
//! their owner (CR 404.3). When such a batch has no player losing, the owners
//! are asked one after the other in APNAP order (CR 101.4), each for the
//! order of their own cards, top to bottom. The batch waits in a continuation,
//! with the creatures still on the battlefield, and applies with the last
//! answer. An owner of one card is not asked.
//!
//! A batch in which a player loses asks nobody. In a two-player game the loss
//! ends the game (CR 104.2a), so an order could never matter, and a decision
//! with no consequence is left out (owner decision 2026-10-01, as with the
//! cut of CR 103.3). Every player losing at once is a draw (CR 104.4a), which
//! fails closed.

use std::collections::BTreeMap;

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_decision::{
    AuthoritativeDecisionRequest, CandidateIntent, CandidateOrdering, DecisionDomainV2,
    DecisionPurposeV4, DecisionVisibility, EngineCandidateBinding,
};
use mtgml_model::{ContinuationId, GameObjectId, PlayerId, StateRevision, ZoneKind};
use mtgml_state::{
    CombatStep, ContinuationPayload, ContinuationRecord, EngineState, PriorityState,
    SbaGraveyardOwnerOrderV1, SbaObjectCauseV1, SbaSelectedActionV1, TurnPosition,
    VisibilityPartition, ZoneLocation, ZonePosition,
};

use crate::turn_progression::{
    admits, install_request, move_card, open_priority, record_unobserved, Facts, NextDecision,
    RequestShape,
};
use crate::zone_incarnation::SelectedZoneTransitionKind;
use crate::{AuthoritativeRuleEventKind, BasicLandTransitionError as Error};

/// The state-based actions that apply to `state`, in canonical order: the
/// players who lose by `PlayerId`, then the creatures destroyed by
/// `GameObjectId` (CR 704.5a, 704.5g).
fn applicable_actions(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<Vec<SbaSelectedActionV1>, Error> {
    let mut actions: Vec<_> = state
        .core
        .players
        .iter()
        .filter(|(_, player)| !player.has_lost && player.life <= 0)
        .map(|(player, _)| SbaSelectedActionV1::PlayerLoses { player: *player })
        .collect();
    for creature in crate::combat::battlefield_creatures(admission, state)? {
        let marked = state
            .card_rules
            .permanents
            .permanents
            .get(&creature.object)
            .ok_or(Error::InvalidResult)?
            .marked_damage;
        if creature.toughness > 0 && i128::from(marked) >= i128::from(creature.toughness) {
            actions.push(SbaSelectedActionV1::ObjectToOwnerGraveyard {
                object: creature.object,
                causes: vec![SbaObjectCauseV1::LethalDamage],
            });
        }
    }
    Ok(actions)
}

/// Whether any state-based action would be performed in `state` (CR 704.3).
pub(crate) fn any_apply(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<bool, Error> {
    Ok(!applicable_actions(admission, state)?.is_empty())
}

/// The players `actions` make lose.
fn losers_of(actions: &[SbaSelectedActionV1]) -> Vec<PlayerId> {
    actions
        .iter()
        .filter_map(|action| match action {
            SbaSelectedActionV1::PlayerLoses { player } => Some(*player),
            SbaSelectedActionV1::ObjectToOwnerGraveyard { .. } => None,
        })
        .collect()
}

/// The cards `actions` put into graveyards, each with its owner, in object
/// order.
fn destroyed_by(
    state: &EngineState,
    actions: &[SbaSelectedActionV1],
) -> Result<BTreeMap<GameObjectId, PlayerId>, Error> {
    let mut destroyed = BTreeMap::new();
    for action in actions {
        if let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action {
            let owner = state
                .zones
                .objects
                .get(object)
                .ok_or(Error::InvalidResult)?
                .owner;
            destroyed.insert(*object, owner);
        }
    }
    Ok(destroyed)
}

/// CR 404.3: the owners who arrange the cards `actions` put into their
/// graveyards, in APNAP order (CR 101.4): those with two or more.
fn owners_with_cards_to_arrange(
    state: &EngineState,
    actions: &[SbaSelectedActionV1],
) -> Result<Vec<PlayerId>, Error> {
    let mut per_owner = BTreeMap::<PlayerId, usize>::new();
    for owner in destroyed_by(state, actions)?.into_values() {
        *per_owner.entry(owner).or_default() += 1;
    }
    Ok(
        crate::game_start::turn_order(state, state.core.active_player)
            .into_iter()
            .filter(|owner| per_owner.get(owner).is_some_and(|count| *count > 1))
            .collect(),
    )
}

/// The top of `owner`'s public graveyard.
fn graveyard_top(owner: PlayerId) -> ZoneLocation {
    ZoneLocation {
        zone: ZoneKind::Graveyard,
        player: Some(owner),
        position: ZonePosition::Top { offset: 0 },
        visibility: VisibilityPartition::Public,
        partition: None,
    }
}

/// CR 704.3: checks the state-based actions, performs all that apply at once
/// as one event, and checks again until none applies. Returns what follows:
/// - the end of the game, when a player lost, whose opponent wins (CR 104.2a);
///   the game ends with the loss, so no check follows it;
/// - the graveyard order of the first owner who has to give one, when a batch
///   puts two or more cards of an owner into their graveyard; nothing of the
///   batch is performed until the last owner has answered (`order_graveyard`);
/// - otherwise, with nothing left to apply, priority for the active player.
///
/// A creature that is destroyed is removed from combat before it moves
/// (CR 506.4), and its owner has put a permanent card into their graveyard this
/// turn. Nothing of this slice makes one check cause another, so the second
/// check finds nothing; it is made because the rule makes it.
pub(crate) fn perform_state_based_actions(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    facts: &mut Facts,
) -> Result<NextDecision, Error> {
    loop {
        let actions = applicable_actions(admission, next)?;
        if actions.is_empty() {
            return Ok(open_priority(next));
        }
        admits(admission, "rules/state-based-actions-combat")?;
        let losers = losers_of(&actions);
        // CR 104.4a: all the players remaining in the game losing at once is a
        // draw, which is not modelled.
        if losers.len() > 1 {
            return Err(Error::TurnProgressUnsupported);
        }
        // A batch with a loss asks nobody: the game ends with it (owner decision
        // 2026-10-01, see the module documentation).
        let owners = if losers.is_empty() {
            owners_with_cards_to_arrange(next, &actions)?
        } else {
            Vec::new()
        };
        if !owners.is_empty() {
            begin_graveyard_order(next, actions, owners)?;
            return Ok(NextDecision::GraveyardOrder);
        }
        if let Some(loser) = apply(next, facts, actions, &[])? {
            return Ok(NextDecision::GameOver { loser });
        }
    }
}

/// CR 704.3: performs `actions` together, as one event: each destroyed
/// creature is removed from combat and then moves to the top of its owner's
/// graveyard, and each loser has lost. `orders` are the orders the owners of two
/// or more of the cards gave (CR 404.3). Returns the player who lost.
///
/// The cards move in object order, so of the cards of an owner who gave no
/// order, the one with the higher object id lies on top. That is also the order
/// of a batch with a loss, which asks nobody (owner decision 2026-10-01). A card
/// goes on top of its graveyard, so the cards of an owner who gave an order
/// move from the bottom of it to the top, in the places of that owner's cards
/// in the object order.
fn apply(
    next: &mut EngineState,
    facts: &mut Facts,
    actions: Vec<SbaSelectedActionV1>,
    orders: &[SbaGraveyardOwnerOrderV1],
) -> Result<Option<PlayerId>, Error> {
    let losers = losers_of(&actions);
    let destroyed = destroyed_by(next, &actions)?;
    for object in destroyed.keys() {
        crate::combat::remove_from_combat(next, *object);
    }
    let mut moves: Vec<GameObjectId> = destroyed.keys().copied().collect();
    for order in orders {
        let slots: Vec<usize> = (0..moves.len())
            .filter(|slot| destroyed.get(&moves[*slot]) == Some(&order.owner))
            .collect();
        if slots.len() != order.top_to_bottom.len() {
            return Err(Error::InvalidResult);
        }
        for (slot, object) in slots.into_iter().zip(order.top_to_bottom.iter().rev()) {
            moves[slot] = *object;
        }
    }
    for object in moves {
        let owner = *destroyed.get(&object).ok_or(Error::InvalidResult)?;
        move_card(
            next,
            object,
            SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard,
            graveyard_top(owner),
            facts,
        )?;
        next.card_rules
            .turn_history
            .record_permanent_card_to_graveyard(owner)
            .map_err(|_| Error::InvalidResult)?;
    }
    for loser in &losers {
        next.core
            .players
            .get_mut(loser)
            .ok_or(Error::InvalidResult)?
            .has_lost = true;
    }
    // The event is not shown to any player: the deaths are, as the zone
    // moves they are.
    record_unobserved(
        facts,
        AuthoritativeRuleEventKind::StateBasedActionsApplied { actions },
    );
    Ok(losers.first().copied())
}

/// The graveyard order the state waits on, as written in its one continuation.
struct PendingOrder<'a> {
    continuation: ContinuationId,
    batch: &'a [SbaSelectedActionV1],
    apnap_owners: &'a [PlayerId],
    next_owner_index: u32,
}

fn pending_order_of(state: &EngineState) -> Result<PendingOrder<'_>, Error> {
    let [(continuation, record)] = state.execution.continuations.iter().collect::<Vec<_>>()[..]
    else {
        return Err(Error::InvalidResult);
    };
    let ContinuationPayload::MagicSbaGraveyardOrderV1 {
        selected_sba_actions,
        apnap_owners,
        next_owner_index,
        ..
    } = &record.payload
    else {
        return Err(Error::InvalidResult);
    };
    Ok(PendingOrder {
        continuation: *continuation,
        batch: selected_sba_actions,
        apnap_owners,
        next_owner_index: *next_owner_index,
    })
}

/// CR 404.3: `batch` puts two or more cards of an owner into their graveyard.
/// It waits, in a continuation, for the order of the first of `owners` (in APNAP
/// order); the creatures stay on the battlefield until the last owner has
/// answered. The continuation is made in the transition that `next`'s revision
/// is the result of, which is the round the state validates it against.
fn begin_graveyard_order(
    next: &mut EngineState,
    batch: Vec<SbaSelectedActionV1>,
    owners: Vec<PlayerId>,
) -> Result<(), Error> {
    let round_start_revision =
        StateRevision(next.revision.0.checked_sub(1).ok_or(Error::InvalidResult)?);
    let continuation = next.allocators.next_continuation_id;
    next.allocators.next_continuation_id = ContinuationId(
        continuation
            .0
            .checked_add(1)
            .ok_or(Error::IdentityExhausted)?,
    );
    next.execution.continuations.insert(
        continuation,
        ContinuationRecord {
            id: continuation,
            created_at_revision: next.revision,
            payload: ContinuationPayload::MagicSbaGraveyardOrderV1 {
                round_start_revision,
                selected_sba_actions: batch,
                apnap_owners: owners,
                next_owner_index: 0,
                completed_owner_orders: Vec::new(),
            },
        },
    );
    Ok(())
}

/// The request that asks the next owner for the order of their cards (CR 404.3):
/// one candidate for each card of theirs in the batch, in the order of their
/// opaque identities, and an exact `Order` over all of them. It follows from the
/// continuation alone; `validate_pending_graveyard_order` is what says the
/// continuation is right. The actor is an owner, who need not be the active
/// player.
pub(crate) fn order_request_shape(state: &EngineState) -> Result<RequestShape, Error> {
    let pending = pending_order_of(state)?;
    let owner = *pending
        .apnap_owners
        .get(usize::try_from(pending.next_owner_index).map_err(|_| Error::InvalidResult)?)
        .ok_or(Error::InvalidResult)?;
    let identity = state
        .perspective_identities
        .players
        .get(&owner)
        .ok_or(Error::InvalidResult)?;
    let mut raw = Vec::new();
    for action in pending.batch {
        let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action else {
            continue;
        };
        let card_owner = state
            .zones
            .objects
            .get(object)
            .ok_or(Error::InvalidResult)?
            .owner;
        if card_owner != owner {
            continue;
        }
        let opaque = identity
            .object_to_opaque
            .get(object)
            .copied()
            .ok_or(Error::InvalidResult)?;
        raw.push((
            CandidateIntent::SelectObject { object: opaque },
            EngineCandidateBinding::SelectObject { object: *object },
        ));
    }
    let count = u32::try_from(raw.len()).map_err(|_| Error::InvalidResult)?;
    Ok(RequestShape {
        actor: owner,
        visibility: DecisionVisibility::ActingPlayerOnly,
        continuation_id: Some(pending.continuation),
        purpose: DecisionPurposeV4::SbaGraveyardOrder,
        decision_domain_v2: DecisionDomainV2::Order {
            minimum: count,
            maximum: count,
        },
        candidates: CandidateOrdering::assign_dense(raw).map_err(|_| Error::InvalidResult)?,
    })
}

/// Installs the request that asks the next owner for their order.
pub(crate) fn install_order_request(
    next: &mut EngineState,
) -> Result<AuthoritativeDecisionRequest, Error> {
    let shape = order_request_shape(next)?;
    install_request(next, shape)
}

/// CR 404.3: `owner`, whom the pending order asks, puts their cards into the
/// graveyard in the order `top_to_bottom`, the first being the topmost. The
/// order is recorded. With another owner to ask, that is all, and the next
/// owner is asked. After the last owner the whole batch applies (see `apply`),
/// and the state-based actions are checked again (CR 704.3). A batch with a
/// loss is never ordered (see the module documentation), so applying this one
/// cannot make a player lose; if it does, the state is not one the game makes,
/// and this fails closed.
pub(crate) fn order_graveyard(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    facts: &mut Facts,
    owner: PlayerId,
    top_to_bottom: Vec<GameObjectId>,
) -> Result<NextDecision, Error> {
    let continuation = pending_order_of(next)?.continuation;
    let record = next
        .execution
        .continuations
        .get_mut(&continuation)
        .ok_or(Error::InvalidResult)?;
    let ContinuationPayload::MagicSbaGraveyardOrderV1 {
        selected_sba_actions,
        apnap_owners,
        next_owner_index,
        completed_owner_orders,
        ..
    } = &mut record.payload
    else {
        return Err(Error::InvalidResult);
    };
    let asked = usize::try_from(*next_owner_index).map_err(|_| Error::InvalidResult)?;
    if apnap_owners.get(asked) != Some(&owner) {
        return Err(Error::InvalidResult);
    }
    completed_owner_orders.push(SbaGraveyardOwnerOrderV1 {
        owner,
        top_to_bottom: top_to_bottom.clone(),
    });
    *next_owner_index += 1;
    record_unobserved(
        facts,
        AuthoritativeRuleEventKind::SbaGraveyardOrderChosen {
            continuation,
            owner,
            top_to_bottom,
        },
    );
    if asked + 1 < apnap_owners.len() {
        return Ok(NextDecision::GraveyardOrder);
    }
    let batch = selected_sba_actions.clone();
    let orders = completed_owner_orders.clone();
    next.execution.continuations.remove(&continuation);
    match apply(next, facts, batch, &orders)? {
        None => perform_state_based_actions(admission, next, facts),
        Some(_) => Err(Error::InvalidResult),
    }
}

/// Whether the graveyard order `state` waits on is the one the state calls for:
/// exactly one such continuation, in the combat damage step after the damage
/// was dealt, while no player has priority (CR 510.3, 704.3). Its batch is the
/// state-based actions that apply to the state (CR 704.3), none of them a loss
/// (a batch with a loss asks nobody, because the game ends with it: owner
/// decision 2026-10-01), and the owners it asks are those with two or more of
/// its cards, in APNAP order. How far the orders have got is for the state to
/// validate.
pub(crate) fn validate_pending_graveyard_order(
    admission: &ExecutableProfileAdmissionV1,
    state: &EngineState,
) -> Result<(), Error> {
    let pending = pending_order_of(state)?;
    let batch = applicable_actions(admission, state)?;
    let combat = state.combat.as_ref().ok_or(Error::InvalidResult)?;
    if !losers_of(pending.batch).is_empty()
        || pending.batch != batch.as_slice()
        || pending.apnap_owners != owners_with_cards_to_arrange(state, &batch)?.as_slice()
        || state.core.position
            != (TurnPosition::Combat {
                step: CombatStep::CombatDamage,
            })
        || state.core.priority != PriorityState::None
        || !combat.damage_step_completed
    {
        return Err(Error::InvalidResult);
    }
    Ok(())
}

/// The creatures that die with the order the pending request asks for: those
/// of its batch. They carry lethal damage and are still on the battlefield,
/// which is the one case of a decision at which a creature has lethal damage
/// marked on it. Empty unless that request is pending.
pub(crate) fn creatures_awaiting_the_order(state: &EngineState) -> Vec<GameObjectId> {
    let ordering_is_pending = state
        .execution
        .pending_decision
        .as_ref()
        .is_some_and(|request| request.purpose == DecisionPurposeV4::SbaGraveyardOrder);
    let Some(pending) = ordering_is_pending
        .then(|| pending_order_of(state).ok())
        .flatten()
    else {
        return Vec::new();
    };
    pending
        .batch
        .iter()
        .filter_map(|action| match action {
            SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } => Some(*object),
            SbaSelectedActionV1::PlayerLoses { .. } => None,
        })
        .collect()
}
