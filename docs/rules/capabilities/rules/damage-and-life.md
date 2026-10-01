# Damage and Life

**Capability key/version:** `rules/damage-and-life@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `damage`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 119.3, 120.1, 120.3a, 510.1a. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` sections 3 and 4.

Covered for combat damage dealt to a player in two-player games of admitted
lands and vanilla creatures by the listed rules and production-endpoint cases.
Damage marked on a creature is specified and not implemented. No certification,
card, deck, format, or playability support is claimed.

## Supported scope

Combat damage dealt to a player makes that player lose that much life
(CR 120.3a, 119.3). The life total is a signed number: damage past 0 leaves it
negative, and observations show the negative total. The turn history records
that the player lost life this turn.

## Explicit exclusions

The following are not supported and fail closed:
- damage marked on a creature, and damage that destroys it;
- damage that is not combat damage, and damage to a planeswalker or a battle;
- damage prevention and replacement, infect and every other result of damage;
- life gain and life payment.

## State and identity model

The player's life total and the `lost_life_this_turn` flag of the turn history.
No identity changes.

## Events and replacement points

`CombatDamageDealt` carries the assignments and `LifeChanged { player, from, to }`
carries the result. The event projection requires the assignments to the player
to add up to the player's life loss. There are no replacement points.

## Decisions and ordering

No decision is involved.

## Information and opaque identities

Life totals are public. Both players observe `LifeChanged`.

## Transition/continuation behavior

The life loss happens in the transition that opens the combat damage step. The
state-based actions are checked in the same transition.

## Conformance, property, replay, and performance evidence

- Rules case in `crates/mtgml-rules/src/turn_progression.rs`:
  `unblocked_attackers_deal_combat_damage_to_the_defending_player`.
- Event projection case in `crates/mtgml-rules/src/events.rs`:
  `combat_damage_to_a_player_must_match_the_life_change`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_game.rs`:
  `an_unblocked_attack_lowers_the_defenders_life` (both players observe
  `life_changed` and the new totals) and
  `overkill_damage_shows_negative_life_and_ends_the_game`.
