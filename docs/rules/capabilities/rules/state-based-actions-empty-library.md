# Empty-Library Draw Loss

**Capability key/version:** `rules/state-based-actions-empty-library@0.1.0`
**Lifecycle:** `covered`
**Owner role:** `state_based_actions`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 104.2a, 117.5, 121.4, 504.1, 704.3, 704.5b.

## Supported scope

The draw-step draw finds the active player's library empty (CR 504.1, 121.4). The state-based actions are checked before any player receives priority in that step (CR 117.5, 704.3), and that player loses the game (CR 704.5b). In a two-player game the other player wins (CR 104.2a).

The resulting episode status is `Terminal { reason: RulesLoss }`, with a `Loss` for the player who attempted the draw and a `Win` for the other player. The losing player's `has_lost` flag is set. No decision is pending, and no player holds priority.

## Explicit exclusions

The following are not supported:
- draws outside the draw step;
- multiple draws;
- draw replacement;
- games with more than two players;
- simultaneous losses;
- every other state-based action.

Inputs that would need any of these fail closed.

## State and identity model

The attempted draw and the state-based-action check happen in the same transition, so no attempted-draw marker is stored. No card moves. No object or opaque identity changes.

## Events and replacement points

The transition emits `StateBasedActionsApplied { actions: [PlayerLoses { player }] }`. Its semantic operation covers the `has_lost` change in the state delta.

## Decisions, actor, cardinality, ordering

No decision is involved. The loss is a rules consequence, not a choice.

## Information/knowledge/opaque identities

The outcome is public. No hidden information changes, because the library was already empty.

## Transition and continuation behavior

The step that ends the upkeep performs the whole sequence in one transition:
1. It moves to the draw step.
2. It finds the empty library.
3. It applies the loss.
4. It ends the episode.

Later responses are rejected with `EpisodeClosed`.

## Conformance evidence

- `turn_progression::tests::drawing_from_an_empty_library_loses_the_game` (rules)
- `production_turn::drawing_from_an_empty_library_ends_the_game` (production endpoints, replay)
