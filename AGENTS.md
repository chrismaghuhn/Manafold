# AGENTS.md — Manafold

Manafold is a headless, deterministic Magic: The Gathering rules engine. Its
games are meant to become ML training data. It is a solo student project.

## 1. What matters, in order

**correctness → determinism → information safety → decision completeness →
replayability → maintainability → performance**

In practice:

- Same seed + same decisions = same game, byte for byte.
- A player-facing API never reveals hidden information (other hands, library
  order, internal IDs, RNG state).
- Every player choice is an explicit decision. Never fill in a choice
  heuristically or randomly.
- Unsupported rules fail with a precise error. Never guess.

## 2. Progress = what the engine can play

Every task ends with an answer to:

> **What can the engine do now that it could not do before?**

New contract versions, documents, gates, and refactors are not progress on
their own. If a task would add ceremony without adding or protecting playable
behavior, say so and propose something smaller.

**Current focus:** reconnect the full turn (untap → cleanup, incl. combat) to
the production runtime; then spells and the stack.

## 3. Keep changes small

- Smallest change that achieves the task. One feature per branch/PR.
- No speculative abstractions, no unrelated refactors. Report unrelated
  problems instead of fixing them.
- Mechanics are reusable rules, never card-name special cases.
- Python is a client only: no Magic rules, legality, or state authority.
- No runtime dependency on other MTG engines (Forge, XMage, …). Their data may
  be used as reference or import source.
- No project-management labels (`M3`, `Phase 4`, `Block 7`, `G0j`, …) in code
  identifiers, modules, or types. Use semantic names (`draw-card`,
  `combat-damage`).

## 4. Versions: one current format, changed in place

Manafold has no external users and no persisted data that must stay readable.

- Change state, decision, event, checkpoint, and replay formats **in place**.
  Do not add a `FooV9` next to `FooV8`.
- When something is replaced, delete the old code path in the same change.
- There is exactly one runtime. Tests must not compile different runtime
  modules than production (no `#[cfg(test)]` / feature swaps of runtime
  modules). *Currently violated in `mtgml-environment`; being removed.*
- Where older docs or ADRs demand a new version identity for every change,
  this section takes precedence until those docs are updated.

## 5. Tests must exercise the real engine

- Test through the same code path production uses. Feature-gated historical
  or testkit runtimes are not evidence that the engine works.
- Cargo unifies features across the workspace: because `mtgml-conformance`
  enables `historical-conformance-runtime`, even `cargo test --workspace`
  without `--all-features` runs the old runtime. Until those features are
  removed, production behavior is only proven by
  `cargo test -p mtgml-environment --locked` (its `tests/` integration tests).
- For rule changes: first write a failing test that shows the behavior in a
  real game flow, then implement.
- Some gate scripts fail on a dirty checkout. Commit (or stash) before running
  checks, and never report a failure caused only by uncommitted files as a
  code failure.

## 6. Honest reporting

- Report a command as passing only if you ran it and it passed. Otherwise say
  `NOT_RUN` (not executed) or `FAIL` (with the relevant output).
- Compiles ≠ correct. Parsed card ≠ supported card.
- If code, tests, and docs disagree, say so. Don't silently pick one.

## 7. Commands

```bash
just check-fast                          # quick loop while working
cargo test -p mtgml-environment --locked # production runtime path
just check                               # before opening a PR
just doctor                              # toolchain problems
```

## 8. Where to look

Read the doc for the subsystem you change, not everything. Docs describe
intent; code and tests are what actually runs.

| Topic | Doc |
|---|---|
| Architecture overview | `docs/ARCHITECTURE.md` |
| Objects, zones, IDs | `docs/DOMAIN_MODEL.md` |
| Transitions, forced progress | `docs/EXECUTION_MODEL.md` |
| Decisions, legal candidates | `docs/DECISION_PROTOCOL.md` |
| Hidden information | `docs/INFORMATION_MODEL.md` |
| Replay, determinism | `docs/REPLAY_AND_DETERMINISM.md` |
| Which Magic rules text is authoritative | `docs/rules/AUTHORITY_POLICY.md` |

## 9. Workflow

- Issues live on GitHub (`chrismaghuhn/Manafold`), used via `gh`. See
  `docs/agents/issue-tracker.md` and `docs/agents/triage-labels.md`; domain
  docs: `docs/agents/domain.md`.
- Work on a branch off `master`. Don't commit or push unless asked.
- After a context compaction: re-read this file, the active plan, and
  `git status` / `git diff` before editing again.
