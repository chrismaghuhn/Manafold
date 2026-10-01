# Game Start Design

**Status:** IMPLEMENTED on branch `feat/game-start`, 2026-10-01 — written and
implemented overnight by the agent on the owner's standing instruction ("work
on R1/W1, we will look at it"); revised after a review by a fresh review agent
(approve with changes; its five blocking points are folded in below). NOT yet
approved by the owner.

## 1. What the engine can do afterwards

Today every game begins from a state a test helper builds by hand: P1 is
already active in the first main phase with a dealt hand. After this change a
game begins from two deck lists exactly as CR 103 prescribes for the first
game of a two-player match:

- a randomly determined player chooses who takes the first turn;
- each deck is shuffled into a hidden library;
- each player starts at 20 life and draws seven cards;
- London mulligans, declared in turn order, redrawn simultaneously, with the
  mulligan count put on the bottom of the library in a chosen order;
- the starting player takes turn 1, whose draw step is skipped.

Every one of these choices is an explicit decision. The random-vs-random smoke
games start this way, so the random policy also plays the start.

This is one of the R1 × W1 closure packages ("game setup",
`research/M4_R1_W1_CAPABILITY_GAP_AUDIT.md`; rows PG-02 … PG-06 of
`research/2026-09-24-magic-rules-flow-inventory.md`). It needs no new card
content, so it is not blocked on the Oracle provenance that new creature
definitions need.

## 2. Rules authority

Comprehensive Rules effective 2026-09-25 (the pinned snapshot):

- 103.1 — the players determine which one of them chooses who takes the first
  turn (first game of a match: any mutually agreeable method).
- 103.3 — after the starting player is determined, each player shuffles their
  deck so that the cards are in a random order.
- 103.4 — each player's starting life total is 20.
- 103.5 — each player draws their starting hand (seven). The starting player
  declares keep or mulligan first, then each other player in turn order. Then
  every player who chose a mulligan does so at the same time: shuffles the hand
  into the library, draws a new hand of seven, and puts a number of those
  cards equal to the number of mulligans they have taken on the bottom of the
  library in any order. A player who keeps may take no further mulligans. The
  process repeats until no player takes a mulligan. 103.5c's free mulligan is
  multiplayer only.
- 103.8, 103.8a — the starting player takes the first turn; in a two-player
  game the starting player skips the draw step of that turn. 500.11: a skipped
  step is passed as though it did not exist. The current progression enters
  the turn-1 draw step and opens priority there without drawing; that is
  unreachable today (games start in turn 1's main phase) and becomes reachable
  with this change, so this change skips the step (G7).

Excluded, and not reachable with the admitted content: 103.2 (sideboards,
companions, commanders, sticker sheets, conspiracies), 103.6 (opening-hand
actions), 103.7 (Planechase), later games of a match (103.1: the previous
loser chooses), variants. `start_game` accepts only definitions whose admitted
profile has no pregame semantics (today: `basic-land@1.0.0`) and fails closed
on any other (`UnsupportedDefinition`), so a future card with an opening-hand
action cannot slip past 103.6.

## 3. Decisions

- **G1 — Entry point.** `mtgml_rules::start_game(admission, decks, root_seed)
  -> Result<EngineState, GameStartError>` with `decks: [(PlayerId,
  Vec<CardDefinitionId>); 2]` (deck-list order). It is a pure function of its
  inputs. It builds the complete pregame state and installs the first request.
  Object and physical-card ids are allocated in (`PlayerId`, deck index)
  order; every object gets its `faces` entry. Each deck must hold at least
  seven cards (`DeckTooSmall`: a shorter deck cannot draw its starting hand).
  No environment constructor is added: the only caller is the test helper,
  which builds `EnvironmentCheckpointV8` and the runtime from this state as it
  does today. When the ML side needs to create games, an environment
  constructor will bind real deck digests into `ReplayManifestV8.decks`
  (synthetic placeholders today); until then the initial checkpoint digest
  binds the libraries. The hand-built fixtures in `tests/common/mod.rs` stay
  for mid-game scenarios.
- **G2 — Pregame is turn 0.** Until turn 1 begins: `core.turn_number == 0` and
  `turn_history.turn_number == 0`; `core.position == Beginning{Untap}`;
  `core.priority == None`; no combat, stack, battlefield objects, mana,
  effects or triggers. `core.active_player` is the chooser until the starting
  player is chosen, then the starting player. Exactly one continuation exists,
  `ContinuationPayload::GameStart(GameStartContinuation)`, and it exists if and
  only if the turn number is 0. State validation (`validate_execution_records`)
  enforces all of this, including hand and library sizes consistent with the
  stage.
- **G3 — The continuation.**

  ```text
  GameStartContinuation {
      chooser: PlayerId,
      starting_player: Option<PlayerId>,     // None only in ChoosingStartingPlayer
      stage: GameStartStage,
      mulligans_taken: BTreeMap<PlayerId, u32>,   // every player
      kept: BTreeSet<PlayerId>,
      round_mulligans: BTreeSet<PlayerId>,   // declared "mulligan" this round
  }
  GameStartStage = ChoosingStartingPlayer
                 | Declaring { player }     // next declarer this round
                 | Bottoming { player }     // next player putting cards on the bottom
  ```

  `Declaring`'s and `Bottoming`'s player are derivable from the sets and the
  turn order; validation requires them to equal the derived player. Digest:
  payload `["game_start", chooser, starting_player_or_null, stage,
  mulligans_taken[[player, count]], kept[], round_mulligans[]]`, stage
  `["choosing_starting_player", null] | ["declaring", player] | ["bottoming",
  player]`. `continuation_request_actor` and `continuation_request_matches`
  (`engine.rs`) get `GameStart` arms.
- **G4 — Randomness.** Two new stream kinds, appended with codes 2 and 3:
  `GameStartChooser` (global only) samples the chooser with
  `uniform_below_u64(2)` over the players in `PlayerId` order inside
  `start_game`; `LibraryShuffle` (player-scoped only) drives every shuffle of
  that player's library with the existing `sampling::shuffle`. The chooser
  sample emits no event (construction emits none); it is pinned by the initial
  checkpoint and re-derivable because `start_game` is pure. RNG_CONTRACT.md's
  kind list and scope rules are updated.
- **G5 — Decisions** (three `DecisionPurposeV4` variants, appended; wire tags
  `starting_player`, `mulligan_declaration`, `mulligan_bottom`), each with its
  `allows_domain`, `allows_intent`, `visibility_is_valid`, digest arm, Python
  table entry and `player-decision-request.v4` / `player-step.v4` schema entry:
  - `StartingPlayer` — actor: the chooser; `ChooseOne`; candidates
    `SelectPlayer { player }` for both players; visibility `Public`.
  - `MulliganDeclaration` — actor: the declarer; `ChooseOne`; candidates
    `ChooseBoolean { value: false }` (keep) and `ChooseBoolean { value: true }`
    (take a mulligan); visibility `Public`.
  - `MulliganBottom` — actor: a player who just took a mulligan; `Order {
    minimum: n, maximum: n }`, n = min(mulligans taken, hand size) ≥ 1; one
    `SelectObject` candidate per card in that player's hand (the actor's own
    opaque ids); visibility `ActingPlayerOnly` (enforced). The answer order is
    top to bottom: the first chosen card ends highest of the group, the last
    one is the bottom card of the library.

  All three carry `continuation_id: Some(game start)`. Their actor need not be
  the active player, so the pregame installer takes an explicit actor and the
  continuation id, and the pregame validator re-derives the exact expected
  request from the continuation and the state (the pattern of
  `validate_discard_request`). Whether they belong in `is_profile_dependent`
  is decided against that function's contract during implementation and
  recorded.
- **G6 — Sequence.** One response is one transition (the answer plus every
  forced step up to the next decision).
  1. `start_game`: libraries in deck-list order (FaceDown partition, no
     knowledge; `GameObject.face_down` stays false), life 20, chooser sampled,
     `StartingPlayer` request to the chooser.
  2. Starting player chosen → both libraries shuffled (starting player
     first), both players draw seven (starting player first) → `Declaring
     {starting player}`.
  3. A declaration records keep (`kept`) or mulligan (`round_mulligans`). The
     next declarer is the next player in turn order who has not kept and has
     not declared this round. When nobody is left: every player in
     `round_mulligans`, in turn order, puts the whole hand into the library,
     the library is shuffled, the player draws seven and `mulligans_taken`
     increases; then `Bottoming {first of them}`.
  4. A bottom answer moves exactly the chosen cards from hand to the bottom of
     that player's library in the chosen order; the next player in
     `round_mulligans` bottoms next. After the last one, `round_mulligans` is
     cleared and a new round starts with the first player in turn order who
     has not kept; if every player has kept, turn 1 begins.
- **G7 — First turn.** A dedicated `begin_first_turn` (not `begin_turn`, which
  hands the turn to the other player): turn 0 → 1 with the active player
  unchanged, a fresh turn history, the continuation removed
  (`ContinuationChanged`), untap with an empty `UntapCompleted`, upkeep with
  priority to the starting player. When the upkeep ends, turn 1 passes from
  upkeep straight to the precombat main phase (103.8a, 500.11): no draw, no
  priority in a draw step.
- **G8 — Unlimited mulligans.** CR 103.5 sets no limit. With seven or more
  mulligans a player bottoms the whole seven-card hand. The engine does not
  cap mulligans; the smoke harness bounds a runaway policy with its decision
  budget.
- **G9 — Zone moves.** One new `SelectedZoneTransitionKind`,
  `HandToOwnerLibraryBottom`: a new incarnation appended at the bottom of the
  owner's library (CR 400.7). A mulligan appends the whole hand and then
  shuffles; bottoming appends the chosen cards in order. A shuffle is a
  trusted permutation of one library's ordered vector with the `Top{offset}`
  witnesses rewritten.
- **G10 — Knowledge.** Draws use the existing library→hand path. A card that
  leaves the hand for the library loses its owner's live mapping: reason
  `Shuffle` for a mulligan return (the shuffle follows in the same
  transition), `HiddenTransition` for a bottomed card. The owner does not
  retain knowledge of the bottomed cards' positions. This deviates from
  INFORMATION_MODEL.md's "persistence while distinguishable" and is recorded
  there and in the capability notes: retaining it would require every later
  draw to shift the known facts of tracked library cards (known facts must
  equal live positions, library members sit at `Top{offset}`), while players
  only ever see zone and player, not position, and the retired record keeps
  the card definition. A shuffle while a library card is tracked cannot occur
  under this rule and fails closed.
- **G11 — Events and observation.**
  - New authoritative events, each with its semantic operation, parity rule
    and perspective policy: `StartingPlayerChosen { chooser, starting_player
    }` (public), `MulliganDeclared { player, mulligan }` (public), and
    `LibraryShuffled { player, stream, cursor_before, cursor_after,
    raw_words_consumed, top_to_bottom }` (trusted audit per RNG_CONTRACT.md;
    accepted by the random-state and ordered-zone coverage; validated by
    re-deriving the permutation from seed, stream, cursor and input order).
  - Event validation handles several zone moves and shuffles in one
    transition: `ZoneTransition` and `LibraryShuffled` events are replayed in
    event order over a working copy of `before.zones` (remove and insert with
    ordered-zone offset shifting; the shuffle permutation); each
    `last_known` must match the working state and the final working zones must
    equal `after.zones`.
  - Observed events (Rust, Python, `observed-event-envelope.v4` schema):
    `starting_player_chosen`, `mulligan_declared`. No observed shuffle event:
    it follows from those two. The projection lists them in
    `is_projectable_public_source_event`, `project_v4_public_source_event` and
    `requires_all_player_audience`.
  - A hand→library move projects to its owner as `object_moved {old, new:
    null, hand → library}`: the projection derives `reveals_new` from the
    owner's after-identity instead of hard-coding it. The opponent sees no
    occurrence for hidden moves and the observation carries no zone counts, so
    the opponent learns only `mulligan_declared`. (The missing hand, library
    and life counts in observations are an existing gap, reported separately.)
- **G12 — Admission.** A new game rule root `rules/game-start` (version
  `0.1.0`) joins `MAGIC_GAME_RULE_ROOTS`; `start_game` and every pregame
  transition require it. Every fixture pinning the rules closure or ids derived
  from it is updated in the same change (precedent: commit `df8da30`).
- **G13 — Dispatch.** `execute_magic_response` and
  `validate_magic_pending_request` route the three pregame purposes to a
  pregame branch placed before `validate_slice` (which rejects any
  continuation). The branch validates its own turn-0 profile.
- **G14 — Smoke games.** `random_smoke` builds each game with `start_game`
  from two seed-chosen 27-card land decks (the 20 + 7 cards of today). The
  random policy answers `Order` domains with a uniformly random permutation of
  a uniformly random subset of the required size. The fingerprint pins are
  re-pinned in the same commit, with the reason in the commit message.

## 4. Fail-closed boundaries

`GameStartError`: `DeckTooSmall`, `UnknownDefinition`, `UnsupportedDefinition`,
`DuplicatePlayer`, `CapabilityMissing`, `IdentityExhausted`. A pregame
response that does not match the pending request is rejected like every other
response (state, RNG cursors, replay length and checkpoint unchanged).

## 5. Evidence

- Rules: starting-player choice both ways; the starting player acts first in
  turn 1, has no draw step, and the other player draws on turn 2; keep/keep;
  mulligan once (hand 6 after bottoming, the library's bottom card is the
  chosen one); both mulligan in the same round (simultaneous redraw,
  bottoming in turn order); a player who kept is never asked again; seven
  mulligans (hand 0).
- Validation: two draws in one transition validate; a shuffle plus draws
  validates; a stale second-draw location is rejected; a restore with a
  tampered continuation or tampered candidates is rejected.
- Information: games differing only in the opponent's deck order, hand, or
  bottom choices give the other player byte-identical steps through the whole
  pregame; bottom candidates are the actor's own opaque ids only.
- Determinism and replay: `start_game` is pure (same inputs, same initial
  checkpoint digest); the same answers give the same trajectory; replay and
  restore from any pregame checkpoint continue identically; a rejected pregame
  response leaves RNG cursors unchanged.
- Smoke: 30-turn random games from `start_game`, short and long pins.

## 6. Phasing (each commit green)

1. Event validation replays several zone moves in one transition (no new
   behavior; existing games unchanged).
2. `feat: start a game from two deck lists` — `start_game`, `rules/game-start`,
   the RNG kinds and audited shuffle, the continuation, `StartingPlayer` and
   `MulliganDeclaration`, the two observed events, `begin_first_turn` and the
   turn-1 draw-step skip. Answering "mulligan" fails closed with a precise
   error in this commit only.
3. `feat: London mulligans` — `HandToOwnerLibraryBottom`, the redraw,
   `MulliganBottom`, rounds, seven or more mulligans, knowledge rule G10, the
   projection fix.
4. `test: random smoke games start from deck lists` — decks, the random
   `Order` answer, re-pinned fingerprints, docs and registry.

## 7. Open questions for the owner

1. G10: forget bottomed cards now (draft) and retain them in a follow-up?
2. Observations carry no hand, library or life counts (existing gap): add them
   next?
