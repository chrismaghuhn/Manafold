# Combat Damage

**Capability key/version:** `rules/combat-damage@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `combat`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 506.2, 508.1, 508.1f, 508.8, 509.1a, 509.2, 510.1a, 510.2, 510.3, 511.3. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` sections 3 to 5.

Covered for unblocked attackers in two-player games of admitted lands and
vanilla creatures by the listed rules and production-endpoint cases. Blockers
and damage to creatures are specified and fail closed. No certification, card,
deck, format, or playability support is claimed.

## Supported scope

The active player declares attackers (CR 508.1; see `rules/summoning-sickness`
for which creatures can attack), and they attack the defending player
(CR 506.2). With attackers declared the combat continues (CR 508.8):
1. The declare blockers step opens when the defending player controls no
   untapped creature, so there is nothing to declare. The active player
   receives priority (CR 509.2).
2. At the start of the combat damage step every attacking creature deals
   damage to the defending player equal to its power, simultaneously
   (CR 510.1a, 510.2). A creature that would assign 0 or less damage assigns
   none. The player loses that much life (see `rules/damage-and-life`).
3. The state-based actions are checked (see `rules/state-based-actions-combat`),
   and the active player receives priority (CR 510.3).
4. The creatures stop attacking when the end of combat step ends (CR 511.3).

## Explicit exclusions

The following are not supported and fail closed:
- a defending player who controls an untapped creature: the declare blockers
  step would need a block declaration, so the pass that opens it is refused
  with `TurnProgressUnsupported`;
- damage to creatures, damage division and blocked attackers;
- first strike, double strike, trample and every other keyword;
- damage prevention and replacement;
- more than two players.

## State and identity model

The combat state holds the defending player, the attackers (any number, sorted
by object), one `None` blocker per attacker, and `damage_step_completed`, which
the damage step sets. The combat state is removed when the end of combat step
ends. Attackers are tapped by the declaration (CR 508.1f).

## Events and replacement points

The combat damage step emits, in this order, `CombatDamageDealt` (one
assignment per attacker with positive power: source, the defending player as
recipient, amount), `LifeChanged` and `CombatDamageStepCompleted`. Every
assignment names a declared attacker and the defending player, and the amounts
add up to the life the player lost. `DamageApplied` is not emitted: it overlaps
these events and goes when damage to creatures is added. There are no
replacement points.

## Decisions and ordering

No decision is involved in the damage step. Whether a creature attacks is the
attacker declaration's decision.

## Information and opaque identities

The attack is public. `AttackersDeclared` (every declaration, including an
empty one) and the life change are observed by both players, with attackers
listed by the perspective's ascending opaque ids. `CombatDamageDealt` is not
observed yet.

## Transition/continuation behavior

Passing priority in the declare attackers step with both players passing in
succession opens the declare blockers step; the same in that step opens the
combat damage step, which deals the damage in the same transition. A
transition that cannot continue changes nothing.

A restored or committed state is accepted only if its combat is one this slice
could have produced: the combat exists from the declaration to the end of
combat, its attackers are creatures on the battlefield that the active player
controls, it attacks the other player, and from the declare blockers step on
the defending player controls no untapped creature. Anything else fails closed,
for priority requests too.

## Conformance, property, replay, and performance evidence

- Rules cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `unblocked_attackers_deal_combat_damage_to_the_defending_player`,
  `a_defender_with_an_untapped_creature_fails_closed`.
- Event projection cases in `crates/mtgml-rules/src/events.rs`:
  `combat_damage_to_a_player_must_match_the_life_change`,
  `combat_damage_to_a_creature_fails_closed`, `the_damage_step_completes_once`.
- Restore cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `a_restored_combat_with_a_possible_blocker_is_refused`,
  `a_restored_combat_cannot_have_the_active_player_as_its_defender`,
  `a_restored_combat_needs_attackers_the_active_player_controls`,
  `a_restored_combat_is_in_a_combat_step_it_could_be_in`,
  `every_boundary_of_an_attack_validates`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_game.rs`:
  `an_unblocked_attack_lowers_the_defenders_life`,
  `a_defender_with_an_untapped_creature_fails_closed`,
  `a_restored_combat_the_game_could_not_reach_is_refused`.
