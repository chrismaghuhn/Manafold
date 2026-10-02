# Combat Damage

**Capability key/version:** `rules/combat-damage@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `combat`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 506.2, 506.4, 508.1, 508.1f, 508.8, 509.1a, 509.1h, 509.2, 510.1a, 510.1b, 510.1c, 510.1d, 510.2, 510.3, 511.3. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` sections 3 to 5.

Covered for unblocked attackers and attackers blocked by one or more creatures,
in two-player games of admitted lands and vanilla creatures, by the listed rules
and production-endpoint cases. An attacker blocked by two or more creatures
divides its damage among them as its controller chooses (CR 510.1c): the
controller is asked, one blocker at a time. No certification, card, deck, format,
or playability support is claimed.

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
   blocker (CR 510.1c). An attacker with two or more blockers divides its damage
   among them as its controller chooses (CR 510.1c): see "Decisions and
   ordering".
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
- first strike, double strike, trample and every other keyword;
- damage prevention and replacement;
- more than two players.

## State and identity model

The combat state holds the defending player, the attackers (any number, sorted
by object), the blockers (each blocking creature with the attacker it blocks;
none, once the attacker has died, for a blocker that survives it, CR 509.1g),
the attackers that became blocked, and `damage_step_completed`, which the damage
step sets. The combat state is removed when the end of combat step ends.
Attackers are tapped by the declaration (CR 508.1f). A creature that leaves the
battlefield leaves combat first (CR 506.4; see
`rules/state-based-actions-combat`). The damage marked on creatures is the
`marked_damage` of their permanent record (see `rules/damage-and-life`).

## Events and replacement points

The combat damage step emits, in this order, `CombatDamageDealt` (one
assignment for each source and recipient that receives damage: its source, its
recipient, which is the defending player or a creature, and the amount, which is
1 or more; an attacker whose damage is divided among its blockers has one
assignment for each blocker with a share above 0), `LifeChanged` when the
player lost life, one `MarkedDamageChanged` for each creature that was dealt
damage, in object order, and `CombatDamageStepCompleted`. The events of the
state-based actions that follow are in `rules/state-based-actions-combat`. Every
assignment names a declared attacker or blocker and the recipient the combat
sends its damage to, and the amounts add up to the player's life loss and to the
damage marked on each creature. The event projection knows no powers, so it does
not check that the assignments of a divided attacker add up to its power. That
sum is enforced where the damage is dealt: `deal_combat_damage` fails closed on
a division that does not add up (CR 510.1c). A creature destroyed in the same
transition is gone from the state it ends in, so its `MarkedDamageChanged` is
checked as damage added to what it had before. There is no `DamageApplied`
event: it overlapped these events and is deleted. There are no replacement
points.

## Decisions and ordering

With no blocker or one blocker, no decision is involved in the damage step: the
damage goes to the defending player, or all of it to the blocker (CR 510.1b,
510.1c). An attacker with power 1 or more and two or more blockers has its
controller divide its damage: a `CombatDamageAssignment` request (`ChooseOne`,
the active player only) is asked for each such attacker in turn, in the order of
the active player's opaque ids, and within an attacker for each blocker in the
same order, with one `AssignCombatDamage { attacker, recipient, amount }`
candidate for each amount from 0 to the damage left. A blocker is asked only
while two or more blockers of the attacker have no amount and damage is left;
otherwise the last blocker is assigned what is left, and when nothing is left
each blocker still without an amount is assigned none, and nobody is asked. The
answers wait in a `CombatDamageAssignment` continuation (answered amounts only,
`created_at_revision` the state's revision less the answers); the damage of every
creature is dealt at once with the last answer, in one `CombatDamageDealt`
(CR 510.2), and the state-based actions follow. Whether a creature attacks is the
attacker declaration's decision, and whether it blocks is the block
declaration's.

## Information and opaque identities

The attack, the blocks and the damage are public. `AttackersDeclared` (every
declaration, including an empty one), `BlockersDeclared` (see
`rules/declare-blockers`), `CombatDamageDealt` and the life change are observed
by both players, with the creatures listed by the perspective's ascending opaque
ids. `CombatDamageDealt` is one event for the whole combat (CR 510.2): each
assignment has its source, its recipient (the defending player or a creature)
and an amount of 1 or more, sorted by the perspective's opaque ids, and it is
not emitted when no creature assigns damage. It comes before the `LifeChanged`
and before the deaths. The damage dealt to a creature is also shown as the
`marked_damage` of its permanent in the observation (see `rules/damage-and-life`),
so `MarkedDamageChanged` and `CombatDamageStepCompleted` are observed by neither
player. The deaths the damage causes are public: both players see the creature go
from the battlefield to its owner's graveyard.

The division is private to the attacking player while it is made: its answers so
far are in the `pending_damage_assignment` of that player's observation (one row
for each blocker answered, with its attacker and amount, ascending by attacker
and then blocker; an amount the rules force is not listed), and in no other
player's. Nothing else the other player sees changes with an answer. See
`docs/INFORMATION_MODEL.md`.

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
action runs on entering the step, so no game rests there before it, and the end
of combat step is reached only through it (CR 508.8, 510.1; with no attackers
the step is skipped). The one exception is a pending division, which is accepted
only if its continuation and request are exactly those the battlefield calls for:
the divided attackers and the blockers still to ask in the active player's opaque
order, answers the division could have been given (none above the damage left,
none for a blocker the rules force), the revision it was created at, no priority
and the damage undealt. Anything else fails closed, for priority requests too.

## Conformance, property, replay, and performance evidence

- Rules cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `unblocked_attackers_deal_combat_damage_to_the_defending_player`,
  `a_blocked_attacker_and_its_blocker_damage_each_other`,
  `an_unblocked_attacker_still_hits_the_player_while_another_is_blocked`,
  `an_attacker_whose_blocker_is_gone_deals_no_damage`,
  `two_divided_attackers_are_asked_one_after_the_other`,
  `the_damage_dealt_is_the_division_made`,
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
  `every_boundary_of_an_attack_validates`; in
  `crates/mtgml-state/src/tests/validation.rs`:
  `an_end_of_combat_with_attackers_has_dealt_their_damage`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_game.rs`:
  `an_unblocked_attack_lowers_the_defenders_life` and
  `a_restored_combat_the_game_could_not_reach_is_refused`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_combat.rs`:
  `declaring_no_block_declares_an_empty_block_and_the_attack_stays_unblocked`
  (two unblocked attackers lower the defender's life by their powers),
  `a_3_3_blocked_by_a_2_1_kills_it_and_survives`,
  `a_2_2_and_a_2_1_that_block_each_other_both_die_one_to_each_graveyard`,
  `a_restored_combat_after_a_blocker_died_continues_identically`,
  `a_restored_end_of_combat_has_dealt_the_damage_of_its_attackers`,
  `a_hill_giant_blocked_by_lions_and_ogre_can_assign_every_division`,
  `each_division_gives_the_expected_deaths`,
  `a_division_that_runs_out_asks_for_no_more`,
  `a_restored_partial_division_continues_identically`,
  `a_forged_partial_division_is_refused`,
  `a_half_divided_damage_is_invisible_to_the_defender`,
  `both_players_see_combat_damage_and_marks`,
  `the_attacking_player_sees_its_own_partial_division`,
  `a_restored_partial_division_gives_the_same_observation` and
  `the_partial_division_is_listed_in_the_order_of_the_attackers_opaque_ids`.
- State and decision cases for the division:
  `the_combat_damage_assignment_digest_binds_every_field` and
  `an_amount_of_combat_damage_has_its_own_purpose_and_candidate_forms` in
  `crates/mtgml-state/src/digest.rs`;
  `a_combat_damage_assignment_names_the_attacking_player_and_the_blockers_of_its_attacker`
  and `a_combat_damage_request_asks_about_the_next_blocker_and_an_amount_each` in
  `crates/mtgml-state/tests/g0d_state_authority.rs`;
  `combat_damage_assignment_requires_one_answer_an_amount_intent_and_its_actor_only`
  in `crates/mtgml-decision/src/v4.rs`.
- Observation and wire cases:
  `combat_damage_dealt_round_trips_as_a_public_wire_value` and
  `a_malformed_combat_damage_is_rejected` in
  `crates/mtgml-observation/src/observed_event_v4.rs`;
  `basic_land_observation_v1_shows_blocks_and_marked_damage` in
  `crates/mtgml-observation/src/tests.rs`; and in Python,
  `test_combat_damage_assignment_offers_each_amount_for_one_blocker_to_its_actor_only`
  in `python/tests/test_decision_v4.py` and `test_combat_damage_dealt_round_trips`
  in `python/tests/test_g0c_observed_event_v4.py`.
- Random smoke games in `crates/mtgml-environment/tests/random_smoke.rs`:
  `creature_games_fight_to_a_winner` plays fifteen seeded games of two creature
  decks, which cast creatures, attack, block (several creatures may block one
  attacker), divide combat damage among blockers, lose creatures to damage and
  can end at 0 life. It asserts that across the games creatures blocked, an
  attacker was blocked by two creatures, combat damage was divided, a creature
  died, and at least one game ended with a winner; every game repeats and replays
  to the same checkpoint.
  `creature_short_game_matches_its_pinned_fingerprint` pins a short game that
  declares a block. The same file also plays two basic-land decks.
