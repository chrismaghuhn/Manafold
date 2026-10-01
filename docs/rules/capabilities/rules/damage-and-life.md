# Damage and Life

**Capability key/version:** `rules/damage-and-life@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `damage`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 119.3, 120.1, 120.3a, 120.3e, 120.5, 120.6, 510.1a, 514.1, 514.2. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` sections 3 and 4.

Covered for combat damage dealt to a player or to a creature in two-player
games of admitted lands and vanilla creatures, by the listed rules and
production-endpoint cases. Damage marked on a creature stays until the cleanup
step removes it. What lethal damage does is `rules/state-based-actions-combat`.
No certification, card, deck, format, or playability support is claimed.

## Supported scope

- Combat damage dealt to a player makes that player lose that much life
  (CR 120.3a, 119.3). The life total is a signed number: damage past 0 leaves
  it negative, and observations show the negative total. The turn history
  records that the player lost life this turn.
- Combat damage dealt to a creature is marked on it (CR 120.3e). Damage dealt
  to a creature that already has some adds to it. Damage does not destroy a
  creature itself (CR 120.5): the state-based actions do, when the damage marked
  is at least its toughness (CR 120.6, 704.5g).
- Damage marked on a creature remains until the cleanup step (CR 120.6). The
  cleanup step first has the active player discard to their maximum hand size
  (CR 514.1), and then all damage marked on permanents is removed at once
  (CR 514.2), in the same transition as the discard, or, when there is no
  discard, in the transition that ends the end step. So a creature that survives a
  block keeps its damage through the end of combat, the postcombat main phase
  and the end step, and the next turn begins without damage on any creature.

## Explicit exclusions

The following are not supported and fail closed:
- damage that is not combat damage, and damage to a planeswalker or a battle;
- damage prevention and replacement, infect, wither, lifelink and every other
  result of damage;
- a creature with a toughness of 0 or less (CR 704.5f), which no card of the
  slice has;
- life gain and life payment.

## State and identity model

The player's life total and the `lost_life_this_turn` flag of the turn history,
and, for every permanent on the battlefield, `marked_damage` of its permanent
record (`PermanentState`, a number of damage that starts at 0). Only a creature
is ever damaged. A permanent that leaves the battlefield loses its record, so a
new incarnation starts without damage (CR 400.7).

A restored or committed state is accepted only if the damage marked in it is
damage this slice can have made: it is on a creature only; it exists only from
the combat damage step, once its damage is dealt, through the end of combat, the
postcombat main phase and the end step, and in the cleanup step only while the
discard of CR 514.1 is pending, before the damage is removed; and no creature has
lethal damage marked, except the creatures that die with a graveyard order that
is being asked (see `rules/state-based-actions-combat`), because the
state-based actions destroy it before any player has priority (CR 704.3).

## Events and replacement points

`CombatDamageDealt` carries the assignments, `LifeChanged { player, from, to }`
carries the result for a player, and `MarkedDamageChanged { creature, from, to }`
carries it for a creature. The event projection requires the assignments to the
player to add up to the player's life loss, and the assignments to a creature to
add up to the damage added to it. The damage marked rises only as combat damage
is dealt, and falls only to 0, in the transition that ends the cleanup step,
with one event for each creature that had some, in object order. There are no
replacement points.

## Decisions and ordering

No decision is involved.

## Information and opaque identities

Life totals are public. Both players observe `LifeChanged`. No player observes
marked damage: `CombatDamageDealt` and `MarkedDamageChanged` are observed by
neither, and the observation has no field for it, so two states that differ only
in non-lethal damage marked give both players the same observation, events and
decisions. A creature's damage becomes observable with the observation of combat
(owner decision 2026-10-01); until then players see only what damage causes: the
life change, and a creature going to its owner's graveyard.

## Transition/continuation behavior

The life loss and the marking happen in the transition that opens the combat
damage step. The state-based actions are checked in the same transition. The
damage is removed in the transition that ends the cleanup step. If a
state-based action would be performed or a trigger waits right after that
(CR 514.3a), a player would have to receive priority in the cleanup step, which
this slice does not support: the transition fails closed.

## Conformance, property, replay, and performance evidence

- Rules cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `unblocked_attackers_deal_combat_damage_to_the_defending_player`,
  `a_blocked_attacker_and_its_blocker_damage_each_other`,
  `marked_damage_is_removed_at_cleanup_with_and_without_a_discard`,
  `cleanup_removes_the_damage_of_every_creature_that_had_any_in_object_order`,
  `a_state_based_action_in_cleanup_fails_closed_instead_of_granting_priority`.
- Restore cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `a_restored_creature_with_lethal_damage_is_refused`,
  `a_restored_land_with_marked_damage_is_refused`,
  `a_restored_combat_that_dealt_no_damage_has_no_marked_damage`,
  `a_restored_cleanup_accepts_marked_damage_only_while_the_discard_is_pending`.
- Event projection cases in `crates/mtgml-rules/src/events.rs`:
  `combat_damage_to_a_player_must_match_the_life_change`,
  `combat_damage_to_creatures_must_match_the_blocks_and_the_marked_damage`,
  `marked_damage_changed_must_match_the_states`,
  `marked_damage_falls_only_when_the_cleanup_step_ends`.
- State cases in `crates/mtgml-state/src/tests/permanents.rs`:
  `the_digest_binds_the_marked_damage_of_each_permanent`,
  `marking_damage_adds_to_the_damage_marked_and_fails_without_a_permanent`,
  `removing_marked_damage_clears_every_permanent_and_reports_those_that_had_some`,
  `delta_accepts_marked_damage_with_its_event`,
  `delta_rejects_marked_damage_without_the_event`,
  `delta_rejects_marked_damage_that_its_event_does_not_name`,
  `delta_rejects_a_permanent_that_enters_damaged`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_game.rs`:
  `an_unblocked_attack_lowers_the_defenders_life` (both players observe
  `life_changed` and the new totals) and
  `overkill_damage_shows_negative_life_and_ends_the_game`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_combat.rs`:
  `a_3_3_blocked_by_a_2_1_kills_it_and_survives` (the survivor has 2 damage
  marked and no player observes it),
  `a_creature_that_survived_a_block_has_no_damage_marked_in_the_next_turn` and
  `a_discard_in_the_cleanup_step_comes_before_the_damage_is_removed`.
- Random smoke games in `crates/mtgml-environment/tests/random_smoke.rs`:
  `asymmetric_games_cast_attack_and_end_at_zero_life` plays ten seeded games of a
  creature deck against a land deck, in which unblocked attackers lower the
  land player's life.
