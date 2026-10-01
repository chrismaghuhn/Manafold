# Game Start

**Capability key/version:** `rules/game-start@0.1.0`
**Lifecycle:** `specified`
**Owner role:** `turn`
**Authority snapshots:** WotC Comprehensive Rules snapshot `wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, rules 103.1, 103.3, 103.4, 103.5, 103.8, 103.8a, 500.11.

## Supported scope

The first game of a two-player match starts from two deck lists of admitted
cards:

1. One player is chosen at random to decide who takes the first turn (CR 103.1).
2. That player chooses the starting player.
3. Each library is shuffled (CR 103.3); each player has 20 life (CR 103.4).
4. Each player draws seven cards and may take London mulligans (CR 103.5).
5. The starting player takes turn 1, whose draw step is skipped
   (CR 103.8a, 500.11).

## Explicit exclusions

The following are not supported and fail closed:
- sideboards, companions, commanders, sticker sheets and conspiracies (CR 103.2);
- opening-hand actions (CR 103.6): only definitions whose profile has no
  pregame semantics are accepted;
- Planechase (CR 103.7);
- later games of a match, where the previous loser chooses (CR 103.1);
- multiplayer rules, including the free mulligan (CR 103.5c);
- decks with fewer than seven cards.

## State and identity model

Until turn 1 begins the turn number is 0 and exactly one `GameStart`
continuation records the chooser, the starting player, the stage, each
player's mulligan count, who has kept and who declared a mulligan in the
current round. Libraries are face-down ordered zones; every move to a hand or
a library makes a new object (CR 400.7).

## Events and replacement points

`StartingPlayerChosen` and `MulliganDeclared` are public. `LibraryShuffled` is
a trusted audit of one shuffle: stream, cursors, consumed words and the
resulting order. Draws are ordinary zone transitions.

## Decisions, actor, cardinality, ordering

- `StartingPlayer`: the chooser picks one player (`ChooseOne`, public).
- `MulliganDeclaration`: keep or take a mulligan (`ChooseOne`, public); the
  starting player declares first, then the other player.
- `MulliganBottom`: the cards to put on the bottom and their order
  (`Order { n, n }`, acting player only).

## Information/knowledge/opaque identities

The library order, the RNG streams and the cards in an opponent's hand stay
hidden. A player learns the cards they draw. A card returned to the library
loses its owner's live mapping; the owner does not retain the positions of
bottomed cards.

## Transition and continuation behavior

One response is one transition: the answer plus every forced step up to the
next decision (shuffles, the draws, the start of turn 1).

## Conformance evidence

Listed when the lifecycle advances to `covered`.
