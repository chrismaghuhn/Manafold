# AGENTS.md — Manafold

Manafold is a greenfield, headless, deterministic, ML-native Magic: The Gathering rules and simulation engine.

This file tells coding agents **how to work in this repository**. It is deliberately not a second architecture specification. For system semantics, read the normative documents and accepted ADRs referenced below.

## 1. Decision compass

When priorities conflict, use this order:

**correctness → determinism → information safety → decision completeness → replayability → maintainability → performance → ML scale**

Do not trade an earlier property for a later one without an explicit architectural decision. Manafold exists to produce **training data that can be trusted**.

## 2. Read before changing code

Before nontrivial work:

1. Read `AGENTS.md`.
2. Read `README.md` or the project charter.
3. Read `docs/NORMATIVE_HIERARCHY.md`.
4. Read the current project/milestone status.
5. Use the trigger table below to locate the relevant normative documents.
6. Inspect accepted ADRs affecting the subsystem.
7. Inspect the implementation and tests you are about to change.

Do not rely on summaries, old reviews, changelogs, issue text, or this file when a normative source exists.

If code, schemas, fixtures, documentation, tests, or accepted ADRs contradict each other, **do not silently choose one interpretation**. Resolve every affected representation coherently or report the contradiction as a blocker.

## 3. Project boundary and evidence discipline

Manafold is independent. Do not introduce runtime or architectural dependencies on Forge, XMage, Argentum, Cockatrice, or another MTG engine. External engines may be research or differential-testing references, not semantic authority.

Repository structure, documentation, or plausible-looking code does not prove support.

Never infer:

```text
parsed      = supported
compiled    = certified
implemented = covered
covered     = certified
candidate   = frozen
```

Use explicit statuses: `PASS`, `FAIL`, `NOT_RUN`, `BLOCKED`, `EXPERIMENTAL`.

**Never report `PASS` for a command, test, benchmark, build, lint, type-check, conformance run, or verification gate that was not actually executed successfully.** If a required tool is unavailable, report `NOT_RUN`. If another failing condition prevents execution, report `BLOCKED`.

Before claiming a milestone or contract is complete, inspect the current generated verification/status artifacts rather than relying on prose from an older session.

## 4. Semantic trigger map

`AGENTS.md` intentionally does not restate these contracts.

| If you change… | Read first |
|---|---|
| architecture or ownership | `06_ARCHITECTURE.md`, `03_NORMATIVE_HIERARCHY.md`, `34_ADR_BUNDLE.md` |
| domain objects, IDs, zones, incarnation | `07_DOMAIN_MODEL.md`, `27_ENGINE_STATE_CLOSURE.md` |
| authoritative state | `27_ENGINE_STATE_CLOSURE.md`, `07_DOMAIN_MODEL.md` |
| transitions, continuations, forced progress | `08_EXECUTION_MODEL.md`, `09_RULES_SEMANTICS.md` |
| events | `09_RULES_SEMANTICS.md`, `11_INFORMATION_MODEL.md` |
| decisions or legal candidates | `10_DECISION_PROTOCOL.md` |
| hidden information or knowledge | `11_INFORMATION_MODEL.md`, `17_THREAT_MODEL.md` |
| observations or player APIs | `11_INFORMATION_MODEL.md`, `14_ML_ENVIRONMENT.md` |
| digests | `13_STATE_HASHING.md`, `12_REPLAY_AND_DETERMINISM.md` |
| checkpoints, forks, replay | `12_REPLAY_AND_DETERMINISM.md`, `13_STATE_HASHING.md` |
| Rust/Python/wire contracts | `26_WIRE_CONTRACT.md`, `25_ML_CONTRACT.md` |
| trajectories or ML-facing data | `15_ML_TRAJECTORIES.md`, `14_ML_ENVIRONMENT.md` |
| Magic rules/mechanics | `23_AUTHORITY_POLICY.md`, `22_ADDING_RULES_AND_MECHANICS.md` |
| cards | `19_ADDING_CARDS.md` |
| capabilities | `20_CAPABILITY_MODEL.md` |
| support/certification | `21_CERTIFICATION.md`, `28_ACCEPTANCE_GATES.md` |
| conformance/correctness | `16_TESTING_AND_CONFORMANCE.md` |
| API stability | `30_API_LIFECYCLE.md` |
| schema evolution | `31_SCHEMA_EVOLUTION.md` |
| freeze/release status | `28_ACCEPTANCE_GATES.md`, `33_CURRENT_PROJECT_STATE.md`, current milestone |

When an accepted ADR covers the change, read it before implementation. Do not copy normative prose from these documents back into `AGENTS.md`.

## 5. Cross-layer and scope discipline

Public contracts can span Rust, Python, contract catalogs, generated vocabulary, JSON Schema, Golden fixtures, negative fixtures, conformance cases, replay formats, trajectory formats, documentation, and ADRs. Do not fix only the first representation you encounter.

Where the repository has a normative generator, change the authoritative source rather than generated output. Mechanical duplication should be generated where practical; Magic semantics, information-flow rules, state invariants, and cross-field semantic validation remain explicit reviewed logic.

Prefer the smallest coherent change. Do not perform unrelated refactors, add speculative abstraction, optimize before profiling, weaken verification to make a gate pass, or silently widen support claims.

## 6. Maintainer workflows

Use the cheapest verification level appropriate for the work:

```bash
just doctor
just bootstrap
just check-fast
just check
just check-all
just release-candidate
```

Use `check-fast` during normal implementation, `check` before review, `check-all` for broad semantic/cross-layer changes, and `release-candidate` only for freeze/release preparation.

Verification must not mutate the source state being verified. Generated logs/reports belong outside the reproducible source archive. The final archive check runs after source-changing operations.

## 7. Working procedure for agents

For nontrivial tasks:

1. Resolve the exact scope.
2. Read the relevant normative sources.
3. Inspect current implementation and tests.
4. Identify affected contracts/layers and invariants.
5. Make the smallest coherent implementation.
6. Update all affected representations.
7. Add/update tests.
8. Run narrow checks early.
9. Run the required broader profile before completion.
10. Inspect the diff for accidental scope growth.
11. Report exactly what changed and which gates ran.
12. Report every required gate that remains `FAIL`, `BLOCKED`, or `NOT_RUN`.

Never hide uncertainty behind confident wording.

## 8. Review classification

Use severity when useful: `BLOCKER`, `MAJOR`, `MINOR`, `NIT`.

Use precise categories: architectural contradiction, cross-layer contract drift, information leak, determinism/replay risk, missing invariant, validation gap, implementation bug, conformance gap, documentation contradiction, maintainer ergonomics issue, performance concern.

Distinguish architecture problems from incomplete implementations, validators, stale generated artifacts, documentation issues, and unexecuted gates.

## 9. Definition of done

Code being written is not sufficient. Completion normally requires requested behavior implemented, relevant invariants preserved, appropriate tests, affected docs/contracts updated, generated artifacts synchronized, applicable checks executed successfully, and no known blocker hidden.

For cross-language/public contract changes also verify parity, fixtures, compatibility, replay implications, information-boundary implications, and trajectory implications.

For support/certification changes also verify reachable capability closure, required evidence, native-executor policy, and certification outcome.

## 10. Freeze rule

A design that looks complete is not a frozen contract.

```text
Freeze Candidate ≠ Frozen Contract
```

A freeze requires the repository-defined gates to actually execute and pass. Do not begin the next major milestone because documentation, scaffolding, or non-native checks merely look good.

## 11. Agent-specific prohibitions

Do not invent test results; infer compilation from syntax inspection; infer runtime correctness from compilation; infer card support from parsing; infer certification from implementation; hand-edit generated vocabulary when an authoritative generator exists; silently choose between contradictory contracts; turn Python into a second Magic rules engine; introduce hidden semantic state for convenience; bypass information boundaries for debugging/search; replace missing decisions with heuristic/random choices; weaken fail-closed behavior; casually change accepted architecture; or make support claims without evidence.

## 12. Final rules

When forced to choose between **quick progress** and **trustworthy semantics**, preserve trustworthy semantics.

When forced to choose between **guessing** and **failing closed with a precise diagnostic**, fail closed.

When an existing normative contract answers the question, use it rather than reproducing your own version.

When evidence has not been produced, say so.
