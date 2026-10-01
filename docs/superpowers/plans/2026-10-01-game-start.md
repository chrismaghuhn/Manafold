# Game Start Implementation Plan

**Status:** DRAFT — executed overnight by the agent from a spec that the owner
has not yet approved; the owner reviews both in the morning.

**Spec:** `docs/superpowers/specs/2026-10-01-game-start-design.md` (binding; its
§6 phasing is this plan's task order).

## Global Constraints

- AGENTS.md: correctness → determinism → information safety → decision
  completeness → replayability; formats change in place; tests exercise the
  production runtime; every commit keeps `cargo test --workspace --locked`,
  clippy `-D warnings` and the release smoke test green.
- A commit that changes the rules closure re-pins both smoke fingerprints and
  shows in its message that only digests changed (a digest-free trajectory
  comparison before/after).
- No card-name dispatch; no project labels in identifiers.

## Tasks

1. **Chain validation of zone moves** — done (`e1356df4`).
2. **Start a game from two deck lists** (spec G1–G7, G11–G13; mulligan answers
   fail closed):
   1. `rules/game-start` capability (registry, generated projection,
      `MAGIC_GAME_RULE_ROOTS`, closure lists in tests, capability doc); re-pin
      fingerprints with a digest-free equality check.
   2. RNG kinds `GameStartChooser` = 2 (global only), `LibraryShuffle` = 3
      (player-scoped only); RNG_CONTRACT.md.
   3. State: `ContinuationPayload::GameStart`, `GameStartStage`; validation
      (turn 0 ⇔ continuation, stage/set consistency, request matching); digest
      arm `game_start`; STATE_HASHING.md.
   4. Decisions: `StartingPlayer`, `MulliganDeclaration` (Rust, digest,
      Python, schemas).
   5. Events: `StartingPlayerChosen`, `MulliganDeclared`, `LibraryShuffled`
      (audit) with operations, parity, coverage, chain replay of shuffles;
      observed `starting_player_chosen`, `mulligan_declared` (Rust, Python,
      schema, projection lists).
   6. Rules: `game_start.rs` — `start_game`, pregame dispatch before
      `validate_slice`, installer/validator, shuffle + seven draws,
      `begin_first_turn`, turn-1 draw-step skip.
   7. Evidence: rules unit tests and an endpoint test file `game_start.rs`.
3. **London mulligans** (spec G6.3–G6.4, G8–G10): `HandToOwnerLibraryBottom`,
   redraw, `MulliganBottom`, rounds, knowledge rule, `reveals_new` fix.
4. **Smoke games from deck lists** (spec G14).
