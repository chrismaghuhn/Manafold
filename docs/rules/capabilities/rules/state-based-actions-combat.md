# State-Based Actions After Combat Damage

**Capability key/version:** `rules/state-based-actions-combat@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `state_based_actions`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 104.2a, 104.4a, 510.3, 704.3, 704.5a. Design: `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md` sections 3 and 4.

Covered for the loss of a player at 0 or less life in two-player games of
admitted lands and vanilla creatures by the listed rules and production-endpoint
cases. Every other state-based action is specified and not implemented. No
certification, card, deck, format, or playability support is claimed.

## Supported scope

After combat damage, before the active player receives priority (CR 704.3,
510.3), a player at 0 or less life loses the game (CR 704.5a). In a two-player
game the other player wins (CR 104.2a). The episode status is
`Terminal { reason: RulesLoss }` with a `Loss` for that player and a `Win` for
the other. The player's `has_lost` flag is set, no decision is pending, and no
player holds priority. Later responses are rejected with `EpisodeClosed`.

A state in which a player who has not lost is at 0 or less life is not a
decision state: it is rejected where a request would be validated or answered,
because the loss applies before anyone could decide.

## Explicit exclusions

The following are not supported and fail closed:
- both players at 0 or less life at once, which would be a draw (CR 104.4a);
- a creature with lethal damage, a creature with toughness 0 or less, and every
  other state-based action;
- the owner's choice of graveyard order when several cards go to one graveyard;
- games with more than two players.

## State and identity model

`has_lost` on the losing player. Nothing else changes.

## Events and replacement points

`StateBasedActionsApplied { actions: [PlayerLoses { player }] }`. Its semantic
operation covers the `has_lost` change. There are no replacement points.

## Decisions and ordering

No decision is involved. The loss is a rules consequence, not a choice.

## Information and opaque identities

The outcome is public.

## Transition/continuation behavior

The transition that opens the combat damage step deals the damage, finds the
player at 0 or less life and ends the episode.

## Conformance, property, replay, and performance evidence

- Rules cases in `crates/mtgml-rules/src/turn_progression.rs`:
  `lethal_combat_damage_ends_the_game_before_anyone_receives_priority` and
  `a_decision_state_with_a_player_at_zero_life_who_has_not_lost_is_rejected`.
- Production-endpoint cases in `crates/mtgml-environment/tests/creature_game.rs`:
  `zero_life_ends_the_game` (status, `EpisodeClosed`, replay equality) and
  `overkill_damage_shows_negative_life_and_ends_the_game`.
- Random smoke games in `crates/mtgml-environment/tests/random_smoke.rs`:
  `asymmetric_games_cast_attack_and_end_at_zero_life` asserts that, among ten
  seeded games of a creature deck against a land deck, at least one ends in a
  rules loss with the land player at 0 life or less, and that it replays to the
  same checkpoint.
