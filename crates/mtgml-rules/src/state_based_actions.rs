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
//! Two or more creatures put into one graveyard at the same time are arranged
//! by their owner (CR 404.3), a decision the game does not offer yet, so that
//! fails closed. So does every player losing at once, which is a draw
//! (CR 104.4a).

use std::collections::BTreeMap;

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_model::{GameObjectId, PlayerId, ZoneKind};
use mtgml_state::{
    EngineState, SbaObjectCauseV1, SbaSelectedActionV1, VisibilityPartition, ZoneLocation,
    ZonePosition,
};

use crate::turn_progression::{admits, move_card, record_unobserved, Facts};
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
/// as one event, and checks again until none applies. Returns the player who
/// lost, whose opponent wins (CR 104.2a); the game ends with the loss, so no
/// check follows it.
///
/// A creature that is destroyed is removed from combat before it moves
/// (CR 506.4), and its owner has put a permanent card into their graveyard this
/// turn. Nothing of this slice makes one check cause another, so the second
/// check finds nothing; it is made because the rule makes it.
pub(crate) fn perform_state_based_actions(
    admission: &ExecutableProfileAdmissionV1,
    next: &mut EngineState,
    facts: &mut Facts,
) -> Result<Option<PlayerId>, Error> {
    loop {
        let actions = applicable_actions(admission, next)?;
        if actions.is_empty() {
            return Ok(None);
        }
        admits(admission, "rules/state-based-actions-combat")?;
        let losers: Vec<PlayerId> = actions
            .iter()
            .filter_map(|action| match action {
                SbaSelectedActionV1::PlayerLoses { player } => Some(*player),
                SbaSelectedActionV1::ObjectToOwnerGraveyard { .. } => None,
            })
            .collect();
        // CR 104.4a: all the players remaining in the game losing at once is a
        // draw, which is not modelled.
        if losers.len() > 1 {
            return Err(Error::TurnProgressUnsupported);
        }
        let mut destroyed = BTreeMap::<GameObjectId, PlayerId>::new();
        let mut per_owner = BTreeMap::<PlayerId, usize>::new();
        for action in &actions {
            if let SbaSelectedActionV1::ObjectToOwnerGraveyard { object, .. } = action {
                let owner = next
                    .zones
                    .objects
                    .get(object)
                    .ok_or(Error::InvalidResult)?
                    .owner;
                destroyed.insert(*object, owner);
                *per_owner.entry(owner).or_default() += 1;
            }
        }
        // CR 404.3: the owner of two or more cards put into their graveyard at
        // the same time arranges them. The decision is not offered yet.
        if per_owner.values().any(|count| *count > 1) {
            return Err(Error::TurnProgressUnsupported);
        }
        for object in destroyed.keys() {
            crate::combat::remove_from_combat(next, *object);
        }
        for (object, owner) in &destroyed {
            move_card(
                next,
                *object,
                SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard,
                graveyard_top(*owner),
                facts,
            )?;
            next.card_rules
                .turn_history
                .record_permanent_card_to_graveyard(*owner)
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
        if let [loser] = losers[..] {
            return Ok(Some(loser));
        }
    }
}
