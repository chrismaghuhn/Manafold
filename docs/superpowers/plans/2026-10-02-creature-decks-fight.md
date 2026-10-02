# Creature Decks Fight Implementation Plan

**Status:** DRAFT for owner review (2026-10-02).
- Execution: subagent-driven, with Sonnet implementers and Opus reviewers.
- This is step 3b of the approved spec. Step 3a (blocks and deaths) is merged
  as #272 (`master` 6165cd25), and its references were refreshed there.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** two vanilla-creature decks play to a winner. The attacking player
divides a blocked attacker's damage among its blockers, and both players see
blocks, combat damage and marked damage.

**Architecture:**
- **509.1g.** A blocker whose attacker left combat stays a blocking creature.
  The combat state's blocker map gains an optional attacker.
- **Damage division (510.1c).** It is asked per blocker, like the block
  declaration:
  - one `ChooseOne` per blocker except the last;
  - the partial division lives in a continuation that only the attacking
    player sees;
  - one `CombatDamageDealt` is emitted when the division is complete.
- **Observation.** The observation gains marked damage per permanent, the
  blocked attackers and the blocks. `BlockersDeclared` and `CombatDamageDealt`
  become public observed events. A player sees their own partial block and
  damage answers, and the opponent does not.
- **Smoke games.** The asymmetric smoke games become creature decks on both
  sides. They assert coverage of blocks, multi-blocks, divisions, deaths and
  graveyard orders.

**Tech Stack:**
- Rust workspace: `mtgml-state`, `-rules`, `-decision`, `-observation`,
  `-environment`;
- the Python client mirror;
- JSON schemas and wire goldens.

**Spec:** `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md`
(revision 2, approved; amended in place by the owner decisions). This plan
covers:
- §3: combat damage and 510.1c;
- §5: damage division;
- §6: observation;
- §7: step 3b, including the 509.1g bullet.

## Global Constraints

- **AGENTS.md is binding:**
  - correctness → determinism → information safety → decision completeness;
  - every player choice is an explicit decision;
  - unsupported rules fail closed with a precise error;
  - formats change in place (delete replaced code in the same change);
  - tests go through the production runtime;
  - every rules question is answered from the pinned rules text
    `C:\Dev\src\Manafold\.rules\MagicCompRules 20260925.txt` (SHA-256
    `8d860e45…`; never commit it, and never search the filesystem for it).
- **Every commit keeps green:**
  - fmt and clippy `-D warnings`;
  - `cargo test --workspace --locked` and the release `random_smoke`;
  - the Python suite, ruff, mypy and the fast-gate scripts.
- **A commit that changes the smoke pins re-pins them.** If the games'
  decisions did not change, its message proves that with a decision-only
  scratch fingerprint (actor, decision id, answer; length-prefixed) at base and
  head.
  - The current pins in `crates/mtgml-environment/tests/random_smoke.rs` are
    `SHORT_FINGERPRINT` `ce5a35d3…`, `LONG_FINGERPRINT` `0a48dc3c…` and
    `ASYMMETRIC_SHORT_FINGERPRINT` `9c28330d…`.
  - Until Task 5 the smoke games never block: the land player has no
    creatures.
- **No card-name dispatch.** Power, toughness and card types come from the S1
  base characteristics.
- **Ordering.** Requests, candidates and every list a player sees are ordered
  by **that player's opaque ids** (`assign_dense`), never by `GameObjectId`.
- **Witnesses:**
  - Savannah Lions {W} 2/1;
  - Gray Ogre {2}{R} 2/2;
  - Hill Giant {3}{R} 3/3.
- **Owner decisions that stand:**
  - an SBA batch with a player loss asks no graveyard order;
  - the CR 103.3 shuffle/cut stays out;
  - first strike, trample and every other keyword stay out (they fail closed
    at admission).

## Review Focus

1. **A division that runs out before the last blocker.** Once the remaining
   damage is 0, the remaining blockers get 0 and no request is made.
   - Task 2: `a_division_that_runs_out_asks_for_no_more`.
2. **Two divided attackers in one combat.** Each is asked attacker by
   attacker, in the attacking player's opaque order, and one
   `CombatDamageDealt` is emitted for the whole combat.
   - Task 2: `two_divided_attackers_are_asked_one_after_the_other`.
3. **A restored or forged partial division.** A restored partial division
   continues identically. A forged one is refused:
   - an amount above the remaining damage;
   - another revision;
   - another blocker order.

   Task 2: `a_restored_partial_division_continues_identically`,
   `a_forged_partial_division_is_refused`.
4. **A half-made choice is seen only by the player who makes it.**
   - Task 2: `a_half_divided_damage_is_invisible_to_the_defender`.
   - Task 4: `the_defender_sees_its_own_partial_blocks`,
     `the_attacking_player_sees_its_own_partial_division`.
5. **A blocker whose attacker died is still blocking until combat ends**, and
   both players see it blocking nothing.
   - Task 1: `a_blocker_whose_attacker_died_is_still_blocking_until_combat_ends`.
   - Task 3: `both_players_see_a_blocker_whose_attacker_died_blocking_nothing`.

---

### Task 1: A blocker whose attacker died is still blocking (CR 509.1g)

**Files:**
- `crates/mtgml-state/src/core.rs` (`CombatState.blockers`)
- `crates/mtgml-state/src/validation/core.rs` (~:72-97)
- `crates/mtgml-state/src/digest.rs` (`combat_value`, ~:1270)
- `crates/mtgml-rules/src/combat.rs`:
  - `remove_from_combat`;
  - `deal_combat_damage`;
  - `validate_reachable_combat`.
- `crates/mtgml-rules/src/events.rs`:
  - the damage projection, ~:1234;
  - the `StateBasedActionsApplied` projection, ~:1461;
  - the `BlockersDeclared` projection, ~:1521.
- `docs/STATE_HASHING.md`
- `docs/rules/capabilities/rules/declare-blockers.md`: delete the "Known
  divergence" section, and say what the state now holds.

**Interfaces:**
- **Produces:** `CombatState.blockers: BTreeMap<GameObjectId, Option<GameObjectId>>`.
  - `Some(attacker)` is the attacker the blocker blocks.
  - `None` is a blocking creature whose attacker was removed from combat
    (CR 509.1g, 506.4). `None` exists only once `damage_step_completed` is set.
- **Produces, combat digest:** the blockers element becomes
  `[[blocker, attacker | null], …]`. A state with no `None` blocker has the
  same bytes as today, so the smoke pins do not move.
- `CombatBlockerAssignmentV1 { blocker, attacker }` and the `BlockersDeclared`
  event keep a plain attacker, since a declaration always names one.

- [ ] **Step 1: Failing tests.**
  - `a_blocker_whose_attacker_died_is_still_blocking_until_combat_ends`
    (endpoint, `creature_combat.rs`).
    - P1's Savannah Lions attacks, and P2's Hill Giant blocks.
    - After damage, the Lions is in P1's graveyard, and
      `combat.blockers == {giant: None}` at CombatDamage and at EndOfCombat.
    - The Giant has 2 marked.
    - After end of combat, `combat` is `None`.
    - The EndOfCombat checkpoint restores and continues identically.
  - Rewrite 3a's `a_creature_that_dies_is_removed_from_combat_but_its_attacker_stays_blocked`
    (`turn_progression.rs`):
    - the blocker of the dead attacker maps to `None`;
    - the dead blocker's attacker stays in `blocked_attackers`.
  - `a_blocker_without_an_attacker_needs_the_damage_dealt`
    (`mtgml-state` validation):
    - `None` is refused before `damage_step_completed`;
    - `None` is accepted after;
    - `Some(a)` with `a` not blocked is refused.
  - `the_combat_digest_binds_a_blocker_without_an_attacker`: `None` and
    `Some(x)` give different digests.
  - **SBA projection (events.rs):**
    - a projection that drops the blocker of a destroyed attacker is
      refused;
    - one that keeps the blocker with `None` is accepted;
    - a `None` given to a blocker whose attacker survived is refused.
- [ ] **Step 2: Run them.** Expected: FAIL.
- [ ] **Step 3: Implement.**
  - **`remove_from_combat(object)`:**
    - a dying attacker leaves `attackers` and `blocked_attackers`, and its
      blockers' values become `None`;
    - a dying blocker leaves `blockers`.
  - **`deal_combat_damage`:** a `None` blocker deals no damage (510.1d).
  - **`validate_reachable_combat`:** before damage, every value is `Some`.
    The final-review predicate `blocked_attackers == {values}` becomes
    "equals the set of `Some` values".
- [ ] **Step 4: Run** everything. Expected: PASS, with the smoke pins
  unchanged.
- [ ] **Step 5: Commit** `fix: a blocker whose attacker died is still blocking`.

### Task 2: The attacking player divides combat damage among blockers (CR 510.1c)

**Files:**
- `crates/mtgml-decision/src/v4.rs` (purpose, intent);
  `python/src/mtgml/decision_v4.py`;
  `schemas/player-decision-request.v4.schema.json` and the player-step schema,
  plus one example and two negatives.
- `crates/mtgml-state/src/shared_execution.rs` (continuation), its digest in
  `digest.rs`, `engine.rs` validation of the continuation, and
  `docs/STATE_HASHING.md`.
- `crates/mtgml-rules/src/combat.rs` (division), `turn_progression.rs`
  (request, answer dispatch, restore).
- Tests: `crates/mtgml-environment/tests/creature_combat.rs`, rules unit
  tests in `turn_progression.rs`.

**Interfaces:**
- **Consumes:** Task 1's `blockers: BTreeMap<GameObjectId, Option<GameObjectId>>`.
- **Produces:**
  - `DecisionPurposeV4::CombatDamageAssignment` (doc: CR 510.1c), domain
    `ChooseOne`, visibility `ActingPlayerOnly`, actor = the active player.
  - `CandidateIntent::AssignCombatDamage { attacker: OpaqueObjectId, recipient: OpaqueObjectId, amount: u64 }`.
    - `amount` is serialized as a canonical decimal string, like the
      observation's `controlled_since_turn`.
    - The candidates for the first pending blocker are
      `amount = 0..=remaining`.
  - The continuation:
    `ContinuationPayload::CombatDamageAssignment { player: PlayerId, pending_attackers: Vec<GameObjectId>, pending_blockers: Vec<GameObjectId>, assigned: BTreeMap<GameObjectId, u64> }`.
    - `pending_attackers` holds the divided attackers not finished yet, in
      `player`'s opaque order; the first is being divided.
    - `pending_blockers` holds the first pending attacker's blockers without an
      amount, in `player`'s opaque order; the first is asked next.
    - `assigned` holds every **answered** amount, by blocker. Each blocker
      blocks one attacker.
    - Forced amounts are never stored: the last blocker's remainder, and the
      zeros after the damage runs out.
    - `created_at_revision == revision − assigned.len()`, pinned exactly as for
      the block declaration.
  - **Who is asked:** a request is made only while the current attacker has
    two or more blockers without an amount and `remaining ≥ 1`. Otherwise the
    rest is forced and the next attacker starts.
    - Divided attackers are the blocked attackers with two or more blockers
      and power ≥ 1 (510.1a).
  - `deal_combat_damage(admission, next, facts, divided: &BTreeMap<GameObjectId, u64>)`.
    - `divided` gives every blocker of a divided attacker its amount, forced
      ones included.
    - Single-blocker, unblocked and blocker damage stay as today.
    - **Delete** the multi-block fail-closed and its test
      `two_blockers_on_one_attacker_fail_closed_until_damage_can_be_divided`.
- **Do not add a fifth copy of the request-validator idiom.**
  - The four copies are `validate_payment_request`, `validate_block_request`,
    `validate_graveyard_order_request` and `game_start.rs` ~:452-466.
  - Extract their shared body into one function that takes the rule string
    and the re-derived `RequestShape`, and use it for all five.
- `validate_reachable_combat`: at CombatDamage with attackers and
  `!damage_step_completed`, accept the state only while the
  `CombatDamageAssignment` continuation is pending and re-derives exactly.

- [ ] **Step 1: Failing tests.** Endpoint tests run through the real controller.
  - `a_hill_giant_blocked_by_lions_and_ogre_can_assign_every_division`:
    - P1's Hill Giant attacks, and P2 blocks with Savannah Lions and Gray Ogre.
    - P1 gets exactly one `CombatDamageAssignment` request.
    - Its recipient is the first of the two blockers in P1's opaque order.
    - Its amounts are exactly `{0, 1, 2, 3}`.
  - `each_division_gives_the_expected_deaths`: one game per division, written
    as (to Lions, to Ogre).

    | Division | Lions | Ogre | Hill Giant (dealt 4) |
    |---|---|---|---|
    | (0, 3) | on the battlefield, 0 marked | dies | dies |
    | (1, 2) | dies | dies | dies |
    | (2, 1) | dies | survives, 1 marked | dies |
    | (3, 0) | dies | survives, 0 marked | dies |

    In the (1, 2) game, P2 gets a graveyard-order request for its two cards
    and P1 does not, because P1 loses only the Giant.
  - `a_division_that_runs_out_asks_for_no_more`:
    - Hill Giant is blocked by two Savannah Lions and a Gray Ogre.
    - Answering 3 for the first blocker asks nothing more.
    - The other two get 0.
  - `two_divided_attackers_are_asked_one_after_the_other` (rules unit test,
    synthetic state):
    - attacker by attacker, in P1's opaque order;
    - one `CombatDamageDealt` holding every assignment.
  - `a_restored_partial_division_continues_identically`: restore after the
    first answer, then compare the next checkpoints and the replay.
  - `a_forged_partial_division_is_refused`: restore refuses each of these:
    - an amount above the remaining damage;
    - another `created_at_revision`;
    - swapped `pending_blockers`;
    - a pending division with `damage_step_completed` set.
  - `a_half_divided_damage_is_invisible_to_the_defender`: P2's information
    bytes, observation and visible decision are equal before and after P1's
    first answer, with a positive control.
- [ ] **Step 2: Run them.** Expected: FAIL. Today the multi-block fails
  closed.
- [ ] **Step 3: Implement** as in Interfaces. `CombatDamageDealt` stays
  unobserved in this task; Task 3 makes it public.
- [ ] **Step 4: Run** everything, including the Python suite, the schemas and
  the goldens. Expected: PASS, with the smoke pins unchanged (the smoke games
  never block).
- [ ] **Step 5: Commit** `feat: the attacking player divides combat damage among blockers`.

### Task 3: Players observe blocks and combat damage

**Files:**
- `crates/mtgml-observation/src/magic_observation.rs` and
  `magic_shared_execution_observation_v1.rs` (fields, validation);
  `observed_event_v4.rs` (event kinds).
- `crates/mtgml-environment/src/player_projection.rs`,
  `successor_projection.rs` (projection; next to `attacking` and
  `AttackersDeclared`).
- `crates/mtgml-rules/src/combat.rs`: `BlockersDeclared` and
  `CombatDamageDealt` go through `observe_public` instead of
  `record_unobserved`.
- Python: `_magic_basic_land_observation_v1.py`,
  `magic_shared_execution_observation_v1.py`, `_events_v4.py`.
- Schemas: `magic-basic-land-observation.v1`,
  `magic-shared-execution-observation.v1` and `observed-event-envelope.v4`,
  with their examples, negatives and wire goldens (`wire/golden/manifest.json`).

**Interfaces:**
- **Consumes:** Task 1's `Option` blockers, and Task 2's division and its
  single `CombatDamageDealt`.
- **Produces, observation (both observation types):**
  - `PermanentObservationV1.marked_damage: u64`, a canonical decimal string;
    `"0"` for every non-creature.
  - `blocked: Vec<OpaqueObjectId>`: the blocked attackers (509.1h), ascending.
    Each one is also in `attacking`.
  - `blocking: Vec<BlockObservationV1 { blocker: OpaqueObjectId, attacker: Option<OpaqueObjectId> }>`,
    ascending by blocker. `attacker: null` is a blocker whose attacker left
    combat (509.1g). A `Some` attacker is in `attacking` and in `blocked`.
  - `pending_blocks: Option<Vec<DeclaredBlockObservationV1 { blocker, attacker: Option<OpaqueObjectId> }>>`
    and
    `pending_damage_assignment: Option<Vec<AssignedDamageObservationV1 { attacker, blocker, amount: u64 }>>`.
    - The key is required, and its value is always `null` in this task.
      Task 4 fills them.
    - In `DeclaredBlockObservationV1`, `attacker: null` means "does not
      block".
- **Produces, observed events (`ObservedEventKindV4`):**
  - `BlockersDeclared { defending_player: PlayerId, blocks: Vec<ObservedBlockV1 { blocker, attacker }> }`,
    ascending by blocker.
  - `CombatDamageDealt { assignments: Vec<ObservedDamageV1 { source: OpaqueObjectId, recipient: ObservedDamageRecipientV1, amount: u64 }> }`.
    - The recipient is `Player { player }` or `Object { object }`.
    - The assignments are sorted by (source, recipient) in the observer's
      opaque ids.
  - A creature's death stays the existing `ObjectMoved`.
  - `MarkedDamageChanged` stays unobserved, since marks are in the
    observation. Cleanup's removal is seen as `marked_damage` going to `"0"`.
- **Decision (resolves a 3a deferred minor).** `BlockersDeclared` is emitted
  whenever the declare blockers step's turn-based action happens, that is,
  whenever there are attackers (508.8). This includes no blocks, and a
  defender with no untapped creature. "No blocks" then has one event-log
  shape.

- [ ] **Step 1: Failing tests** (endpoint, through the real controller).
  - `both_players_see_the_blocks_once_declared`:
    - The 3a test `a_half_declared_block_is_invisible_to_the_attacker` still
      passes.
    - After the last answer, both players get `BlockersDeclared`, and
      `blocking`/`blocked` list the blocks in their own opaque ids.
  - `both_players_see_combat_damage_and_marks`, with Hill Giant vs Savannah
    Lions:
    - both players get `CombatDamageDealt` with both assignments;
    - the Giant's `marked_damage` is `"2"` for both;
    - the Lions' move is observed as before.
  - `both_players_see_a_blocker_whose_attacker_died_blocking_nothing`: the
    Task 1 scenario shows a `blocking` entry with `attacker: null` for both
    players.
  - `an_attack_nobody_can_block_shows_an_empty_block_declaration`: in a
    creature deck vs a land deck, the attacker gets `BlockersDeclared` with
    empty `blocks`.
  - Cleanup:
    - rewrite the 3a assertion that cleanup leaves the observation unchanged;
    - now `marked_damage` reads `"0"` after cleanup, with no observed event
      for the removal.
  - The deck-order noninterference test covers the new fields.
  - Rust and Python wire tests, with both languages rejecting the same cases:
    - unsorted or duplicate `blocking`;
    - a `blocking` attacker that is not attacking;
    - a `blocked` id not in `attacking`;
    - a missing `pending_blocks` key;
    - nonzero `marked_damage` on a land;
    - an observed `CombatDamageDealt` with a zero amount.
- [ ] **Step 2: Run them.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** everything. Expected: PASS.
  - The smoke pins change, because of the observation format and the empty
    `BlockersDeclared` in the asymmetric games.
  - Re-pin them with decision-only evidence that the decisions are unchanged.
- [ ] **Step 5: Commit** `feat: players observe blocks and combat damage`.

### Task 4: A player sees their own partial block and damage answers

**Files:** `crates/mtgml-environment/src/player_projection.rs` and
`successor_projection.rs`, plus the tests. No format change, since Task 3
added the keys.

**Interfaces:**
- **Consumes:**
  - Task 3's `pending_blocks` and `pending_damage_assignment`;
  - 3a's `BlockDeclaration` continuation;
  - Task 2's `CombatDamageAssignment` continuation.
- **Produces:**
  - `pending_blocks` is `Some(declared so far)` for the defender only, while
    the block declaration is pending. It is listed ascending by the
    defender's opaque ids.
  - `pending_damage_assignment` is `Some(assigned so far)` for the attacking
    player only, while the division is pending.
  - Both are `null` for the other player, and `null` when nothing is pending.

- [ ] **Step 1: Failing tests** (endpoint).
  - `the_defender_sees_its_own_partial_blocks`: after the first block answer,
    the defender's observation lists it, and the attacker's is `null`. The 3a
    byte-equality test for the attacker still passes.
  - `the_attacking_player_sees_its_own_partial_division`: after the first
    division answer, P1 sees it, and P2's bytes are unchanged (Task 2's test
    still passes).
  - A restored partial block or division gives the same observation as the
    uninterrupted game.
- [ ] **Step 2: Run them.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** everything. Expected: PASS, with the smoke pins
  unchanged (the smoke games still never block).
- [ ] **Step 5: Commit** `feat: a player sees their own partial combat answers`.

### Task 5: Two creature decks play to a winner (symmetric smoke games)

**Files:** `crates/mtgml-environment/tests/random_smoke.rs`, and
`tests/common/` if a helper must change.

**Interfaces:**
- **Consumes:** everything above. `random_answer` already answers `ChooseOne`.
- **Produces:** the asymmetric game kind is replaced by creature decks on both
  sides.
  - Constants: `CREATURE_GAMES` (start at 10), `CREATURE_LANDS = 17`,
    `CREATURE_CREATURES = 10`.
  - Short pin: `CREATURE_SHORT_SEED` and `CREATURE_SHORT_LAST_TURN`, documented
    like today's `ASYMMETRIC_SHORT_SEED`. Use the first seed whose game
    declares a block before that turn begins.
  - Pin: `CREATURE_SHORT_FINGERPRINT`.
  - Delete `ASYMMETRIC_*`, `land_player` and `play_asymmetric`. Update the
    module docs.
  - `Tally` gains:
    - `blocks`: block answers with an attacker;
    - `multi_blocks`: observed `BlockersDeclared` with an attacker blocked
      twice or more;
    - `divisions`: `CombatDamageAssignment` answers;
    - `deaths`: observed `ObjectMoved` from the battlefield to a graveyard;
    - `graveyard_orders`: `SbaGraveyardOrder` answers.

- [ ] **Step 1: Write the test.** `creature_games_fight_to_a_winner` runs the
  games side by side, as today, and checks:
  - determinism and replay per game;
  - each game ends `Running` at turn 31, or `Terminal { RulesLoss }` with the
    loser at ≤ 0 life;
  - over all games, each of `casts`, `payments`, `attacks`, `blocks`,
    `multi_blocks`, `divisions`, `deaths` and `graveyard_orders` is > 0;
  - at least one game ends with a winner.
- [ ] **Step 2: Run it** (release). Expected: FAIL, because the new test and
  constants do not exist yet. Then make it pass.
  - If a coverage count stays 0, raise `CREATURE_GAMES` in steps of 5, up to
    40.
  - Past 40, report it rather than changing the decks.
  - Record the final counts in the commit message.
  - If a game reaches `MAX_DECISIONS` before turn 31, report it with its
    seed; do not raise the cap silently.
- [ ] **Step 3: Pin** `CREATURE_SHORT_FINGERPRINT`.
  - The land pins must not change in this task.
  - The decisions change by design, so there is no decision-only evidence.
    The commit message says so and lists the coverage counts.
- [ ] **Step 4: Run** everything. Expected: PASS.
- [ ] **Step 5: Commit** `test: two creature decks play to a winner`.

### Task 6: Capabilities and documents

**Files:**
- `cards/capabilities/registry.json`:
  - `rules/combat-damage` covers the division; add its conformance cases;
  - `rules/declare-blockers` drops the divergence note.
- Capability docs: `combat-damage.md`, `declare-blockers.md`,
  `damage-and-life.md`.
- `docs/INFORMATION_MODEL.md`: blocks, damage and marks are public; partial
  answers are private to their actor.
- `docs/DECISION_PROTOCOL.md`: `CombatDamageAssignment`, and when
  `BlockersDeclared` is emitted.
- `docs/STATE_HASHING.md` (check against Tasks 1 and 2).
- `README.md`: two creature decks play to a winner.
- The spec header and §7: step 3b is done.
- Regenerate the projection.

- [ ] **Step 1: Update.** Every test name the docs and registry cite must exist.
- [ ] **Step 2: Run** `scripts/validate_maintainer_artifacts.py`,
  `scripts/generate_card_ir_capability_projection.py --check` and
  `scripts/check_documentation.py`. Commit, then run
  `scripts/run_checks.py integration` on the clean tree. Expected: PASS.
- [ ] **Step 3: Commit** `docs: creature decks fight`, before running the gate.
