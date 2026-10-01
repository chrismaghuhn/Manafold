# Public Player State in the Observation

**Status:** DRAFT — written overnight by the agent; NOT approved by the owner.
Branch `feat/public-player-state`, stacked on `feat/game-start`.

## 1. What the engine can do afterwards

A player's observation shows:
- every player's life total;
- every player's hand size and library size;
- which permanents on the battlefield are tapped.

Today none of these are in the observation. An agent cannot see that it is at 3
life, how many cards its opponent holds, or which of its lands are already
tapped, except by replaying the event history. Both the game-start review and
the vanilla-creature review name this gap, and combat needs life and tapped
status from its first step.

## 2. Rules basis

All of these values are public game information. The texts of 401.3, 402.3
and 110.5 were checked overnight through the Academy Ruins rules API (current
CR), but not against the hash of the pinned snapshot.
- **Hand size:** a player can't look at the cards in another player's hand but
  may count them at any time (CR 402.3).
- **Library size:** any player may count the cards in any library (CR 401.3).
- **Tapped:** this is a permanent's status (CR 110.5), and the battlefield is a
  public zone (CR 400.2). Face-down permanents show their status too.
- **Life:** CR 119 governs life totals; no rule hides them.

## 3. Format (changed in place)

`MagicBasicLandObservationV1` and `MagicSharedExecutionObservationV1` both gain
two fields:

- `players`: one entry per player in the state, strictly ascending by player:
  ```
  { player, life: i64, hand_count: u32, library_count: u32 }
  ```
  `life` uses the same integer encoding as the observed `life_changed` event.
  The `active_player` must be one of the listed players.
- `tapped`: the perspective's opaque ids of the tapped permanents on the
  battlefield (public or face down), strictly ascending.

Schema strings stay the same (AGENTS.md §4). The JSON schemas, Python mirrors,
schema examples and negative cases change in place.
- The observation wire golden (`magic-shared-execution-observation.v1.json`)
  and the five shared-observation wire negatives gain the fields.
- The envelope goldens carry an opaque `{}` payload and do not change.
- One new wire negative, on the basic-land contract, pins the new semantic
  check in both languages: an active player missing from `players`.

## 4. Projection

- **Counts** come from `zones.locations`: zone `Hand` or `Library` with
  `player = Some(p)`. A count that does not fit `u32` fails closed
  (`ServiceUnavailable`).
- **Tapped** objects are the battlefield objects with `tapped = true`. Each is
  mapped through the perspective's identity; a missing opaque id fails closed.
- Projection stays read-only: no allocation and no knowledge change
  (INFORMATION_MODEL §Allocation rule).

**Information safety.**
- Both perspectives see the same values; only the opaque ids differ.
- No value depends on library order, hidden card identity or trusted ids.
- The existing deck-order noninterference test
  (`a_player_learns_nothing_about_the_opponents_deck_order`) compares a
  player's complete bytes, observation included. It stops after the mulligans,
  so it covers `players` but never a non-empty `tapped`. `tapped` reads only
  public battlefield objects through the perspective's own identity, and the
  endpoint test pins that identity.

## 5. Evidence

1. **Observation crate.**
   - The validation rejects:
     - unordered or duplicate players;
     - an active player missing from `players`;
     - unordered or duplicate `tapped` ids.
   - The Rust DTO matches the schema example byte for byte.
2. **Production endpoints** (`crates/mtgml-environment/tests/public_player_state.rs`):
   - After the game start, both players see 20 life and seven-card hands for
     both players, and library sizes of deck size minus seven.
   - The opponent's hand count is visible to a player during the mulligans.
   - A land tapped for mana appears in `tapped` for both perspectives, under
     each one's own opaque id, and leaves it at its controller's next untap
     step.
   - Playing a land lowers the hand count; the draw on turn 2 lowers the
     library count.
3. **Python:** the mirror round-trips the example and rejects the same bad
   cases. The schema negatives are registered.
4. **Smoke:** both fingerprints are re-pinned. The commit message records a
   comparison made with a scratch test, which is not committed: the responses
   and checkpoint digests of the pinned games are identical before and after,
   so only what the players see changes. The final reviewer reproduced it
   independently.

## 6. Out of scope

- **Graveyard and exile contents:** they are public objects and already appear
  in the retained knowledge with their location.
- **Poison counters:** not in the state.
- **Creature fields** (power/toughness, damage, can-attack, combat): the
  creature work adds them.
- **Who has lost:** the episode status carries it.

## 7. Tasks

1. Observation types, validation, schemas, examples, negatives, Python mirror
   (red tests first).
2. Projection and the endpoint tests.
3. Re-pin the smoke fingerprints with the before/after decision comparison;
   update `INFORMATION_MODEL.md` §Current player products.

## 8. Open questions for the owner

1. Is the current-state observation the right home for these values? The
   alternative is observed events only. Proposed: the observation carries the
   state, the events carry the history.
2. Should graveyard and exile counts be listed explicitly for ML convenience,
   although the knowledge already holds them?
