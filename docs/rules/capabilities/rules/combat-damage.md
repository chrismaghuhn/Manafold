# Combat Damage

**Capability key/version:** `rules/combat-damage@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `combat`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 506.2, 506.4, 508.1, 508.1f, 508.8, 509.1a, 509.1h, 509.2, 510.1a, 510.1b, 510.1c, 510.1d, 510.2, 510.3, 511.3. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` sections 3 to 5.

Covered for attackers that are unblocked or blocked by at most one creature, in
two-player games of admitted lands and vanilla creatures, by the listed rules and
production-endpoint cases. An attacker blocked by two or more creatures has to
divide its damage among them (CR 510.1c); that division is not supported yet,
and the combat damage step of such an attacker fails closed. No certification,
card, deck, format, or playability support is claimed.

## Supported scope

The active player declares attackers (CR 508.1; see `rules/summoning-sickness`
for which creatures can attack), and they attack the defending player
(CR 506.2). With attackers declared the combat continues (CR 508.8): the
defending player declares blockers (see `rules/declare-blockers`), and the
active player receives priority (CR 509.2). At the start of the combat damage
step every attacking and blocking creature assigns combat damage equal to its
power, and all of it is dealt simultaneously (CR 510.1a, 510.2):
1. An unblocked attacker deals its damage to the defending player
   (CR 510.1b), who loses that much life (see `rules/damage-and-life`).
2. An attacker with exactly one blocker deals all of its damage to that
   blocker (CR 510.1c).
3. A blocked attacker with no blocker left deals no damage (CR 510.1c, 509.1h).
4. A blocker deals all of its damage to the attacker it blocks (CR 510.1d).
5. A creature that would assign 0 or less damage assigns none (CR 510.1a).
6. Damage dealt to a creature is marked on it (see `rules/damage-and-life`).

The state-based actions are then checked (see
`rules/state-based-actions-combat`): a creature dealt lethal damage is destroyed
and a player at 0 life or less loses. The active player receives priority
(CR 510.3). The creatures stop attacking and blocking when the end of combat
step ends (CR 511.3).

## Explicit exclusions

The following are not supported and fail closed:
- an attacker blocked by two or more creatures: the pass that opens the combat
  damage step is refused with `TurnProgressUnsupported`, and nothing changes. A
  game can reach that state, because declaring such blocks is legal (see
  `rules/declare-blockers`), but it cannot play on past it until damage can be
  divided. The division is a decision of the attacker's controller, one per
  blocker, and is not part of this slice;
- first strike, double strike, trample and every other keyword;
- damage prevention and replacement;
- more than two players.

## State and identity model

The combat state holds the defending player, the attackers (any number, sorted
by object), the blockers (each blocking creature with the attacker it blocks),
the attackers that became blocked, and `damage_step_completed`, which the damage
step sets. The combat state is removed when the end of combat step ends.
Attackers are tapped by the declaration (CR 508.1f). A creature that leaves the
battlefield leaves combat first (CR 506.4; see
`rules/state-based-actions-combat`). The damage marked on creatures is the
`marked_damage` of their permanent record (see `rules/damage-and-life`).

## Events and replacement points

The combat damage step emits, in this order, `CombatDamageDealt` (one
assignment per creature that assigns damage: its source, its recipient, which is
the defending player or a creature, and the amount), `LifeChanged` when the
player lost life, one `MarkedDamageChanged` for each creature that was dealt
damage, in object order, and `CombatDamageStepCompleted`. The events of the
state-based actions that follow are in `rules/state-based-actions-combat`. Every
assignment names a declared attacker or blocker and the recipient the combat
sends its damage to, and the amounts add up to the player's life loss and to the
damage marked on each creature. A creature destroyed in the same transition is
gone from the state it ends in, so its `MarkedDamageChanged` is checked as
damage added to what it had before. There is no `DamageApplied` event: it
overlapped these events and is deleted. There are no replacement points.

## Decisions and ordering

No decision is involved in the damage step: with one blocker, the division of
CR 510.1c is the only legal one. Whether a creature attacks is the attacker
declaration's decision, and whether it blocks is the block declaration's.

## Information and opaque identities

The attack is public. `AttackersDeclared` (every declaration, including an
empty one) and the life change are observed by both players, with attackers
listed by the perspective's ascending opaque ids. `CombatDamageDealt`,
`MarkedDamageChanged` and `CombatDamageStepCompleted` are observed by neither
player, and neither are the blocks that decide where the damage goes; they
become observable with the observation of combat (owner decision 2026-10-01).
The deaths the damage causes are public: both players see the creature go from
the battlefield to its owner's graveyard.

## Transition/continuation behavior

Passing priority in the declare attackers step with both players passing in
succession opens the declare blockers step; the same in that step opens the
combat damage step, which deals the damage and checks the state-based actions in
the same transition. When two or more cards of one owner die together, that
owner is asked to arrange them first, and the creatures die with the last
answer (see `rules/state-based-actions-combat`). A transition that cannot
continue changes nothing.

A restored or committed state is accepted only if its combat is one this slice
could have produced: the combat exists from the declaration to the end of
combat, its attackers are creatures on the battlefield that the active player
controls, it attacks the other player, and its blocks are ones the declare
blockers step could have made (see `rules/declare-blockers`). From the combat
damage step on, a combat with attackers has dealt its damage: the turn-based
action runs on entering the step, so no game rests there before it. Anything
else fails closed, for priority requests too.

## Conformance, property, replay, and performance evidence

- Rules cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `unblocked_attackers_deal_combat_damage_to_the_defending_player`,
  `a_blocked_attacker_and_its_blocker_damage_each_other`,
  `an_unblocked_attacker_still_hits_the_player_while_another_is_blocked`,
  `an_attacker_whose_blocker_is_gone_deals_no_damage`,
  `two_blockers_on_one_attacker_fail_closed_until_damage_can_be_divided`,
  `the_vanilla_creatures_have_the_powers_and_toughnesses_the_fights_assume`.
- Event projection cases in `crates/mtgml-rules/src/events.rs`:
  `combat_damage_to_a_player_must_match_the_life_change`,
  `combat_damage_to_creatures_must_match_the_blocks_and_the_marked_damage`,
  `combat_damage_goes_where_the_combat_sends_it`,
  `combat_damage_without_the_marked_damage_events_is_uncovered`,
  `combat_damage_to_a_creature_that_is_destroyed_in_the_same_transition_is_valid`,
  `the_damage_step_completes_once`.
- Restore cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `a_restored_damage_step_has_dealt_its_damage`,
  `a_restored_combat_cannot_have_the_active_player_as_its_defender`,
  `a_restored_combat_needs_attackers_the_active_player_controls`,
  `a_restored_combat_is_in_a_combat_step_it_could_be_in`,
  `a_restored_combat_without_attackers_may_have_a_defender_with_a_creature`,
  `every_boundary_of_an_attack_validates`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_game.rs`:
  `an_unblocked_attack_lowers_the_defenders_life` and
  `a_restored_combat_the_game_could_not_reach_is_refused`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_combat.rs`:
  `declaring_no_block_declares_an_empty_block_and_the_attack_stays_unblocked`
  (two unblocked attackers lower the defender's life by their powers),
  `a_3_3_blocked_by_a_2_1_kills_it_and_survives`,
  `a_2_2_and_a_2_1_that_block_each_other_both_die_one_to_each_graveyard` and
  `a_restored_combat_after_a_blocker_died_continues_identically`.
- Random smoke games in `crates/mtgml-environment/tests/random_smoke.rs`:
  `asymmetric_games_cast_attack_and_end_at_zero_life` plays ten seeded games of a
  creature deck against a land deck, in which creatures attack and unblocked
  combat damage is dealt; every game repeats and replays to the same
  checkpoint. No smoke game reaches a block: a game of two creature decks needs
  the division of damage.
