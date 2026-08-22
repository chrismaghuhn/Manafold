# AGENTS.md — Manafold

Manafold is a greenfield, headless, deterministic, ML-native Magic: The Gathering rules and simulation engine.

This file tells coding agents **how to work in this repository**.

It is deliberately not a second architecture specification.

For system semantics, read the normative documents and accepted ADRs referenced below.

---

# 1. Decision compass

When priorities conflict, use this order:

**correctness → determinism → information safety → decision completeness → replayability → maintainability → performance → ML scale**

Do not trade an earlier property for a later one without an explicit architectural decision.

Manafold exists to produce **training data that can be trusted**.

Everything else depends on that.

---

# 2. Read before changing code

Before nontrivial work:

1. Read `AGENTS.md`.
2. Read `README.md`.
3. Read `docs/NORMATIVE_HIERARCHY.md`.
4. Read the current project/milestone status.
5. Use the trigger table below to locate the relevant normative documents.
6. Inspect accepted ADRs affecting the subsystem.
7. Inspect the implementation and tests you are about to change.

Do not rely on summaries, old reviews, changelogs, issue text, or this file when a normative source exists.

If code, schemas, fixtures, documentation, tests, or accepted ADRs contradict each other:

**do not silently choose one interpretation.**

Either:

* resolve every affected representation coherently, or
* report the contradiction as a blocker.

---

# 3. Project boundary

Manafold is independent.

Do not introduce runtime or architectural dependencies on Forge, XMage, Argentum, Cockatrice, or another MTG engine.

External engines may later be useful as:

* research references;
* differential-testing references;
* behavioral comparison points.

They are not Manafold's semantic authority.

---

# 4. Maturity and evidence discipline

Repository structure, documentation, or plausible-looking code does not prove support.

Never infer:

```text
parsed      = supported
compiled    = certified
implemented = covered
covered     = certified
candidate   = frozen
```

Likewise:

```text
Python tests PASS
≠
Rust workspace PASS
```

and:

```text
documentation says a gate exists
≠
that gate was executed
```

Use explicit statuses:

* `PASS`
* `FAIL`
* `NOT_RUN`
* `BLOCKED`
* `EXPERIMENTAL`

## Non-negotiable rule

**Never report `PASS` for a command, test, benchmark, build, lint, type-check, conformance run, or verification gate that was not actually executed successfully.**

If a required tool is unavailable, report `NOT_RUN`.

If another failing condition prevents execution, report `BLOCKED`.

If behavior has not reached its required evidence level, do not upgrade its lifecycle status.

Before claiming a milestone or contract is complete, inspect the current generated verification/status artifacts rather than relying on prose from an older session.

---

# 5. Semantic trigger map

`AGENTS.md` intentionally does not restate these contracts.

When changing a subsystem, read its authoritative sources first.

| If you change…                                 | Read first                                                                                                       |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| architecture or component ownership            | `docs/ARCHITECTURE.md`, `docs/NORMATIVE_HIERARCHY.md`, relevant ADRs                                             |
| domain objects, IDs, zones, object incarnation | `docs/DOMAIN_MODEL.md`, `docs/contracts/ENGINE_STATE_CLOSURE.md`                                                 |
| authoritative state                            | `docs/contracts/ENGINE_STATE_CLOSURE.md`, `docs/DOMAIN_MODEL.md`                                                 |
| transitions, continuations, forced progress    | `docs/EXECUTION_MODEL.md`, `docs/RULES_SEMANTICS.md`                                                             |
| authoritative or observed events               | `docs/RULES_SEMANTICS.md`, `docs/INFORMATION_MODEL.md`                                                           |
| decisions or legal candidates                  | `docs/DECISION_PROTOCOL.md`, `docs/DECISION_INVENTORY.md`                                                        |
| hidden information or player knowledge         | `docs/INFORMATION_MODEL.md`, `docs/THREAT_MODEL.md`                                                              |
| observations or player-facing APIs             | `docs/INFORMATION_MODEL.md`, `docs/ML_ENVIRONMENT.md`                                                            |
| digests or canonical identity                  | `docs/STATE_HASHING.md`, `docs/REPLAY_AND_DETERMINISM.md`                                                        |
| checkpoints, forks, restore, replay            | `docs/REPLAY_AND_DETERMINISM.md`, `docs/STATE_HASHING.md`                                                        |
| Rust/Python/wire contracts                     | `docs/contracts/WIRE_CONTRACT.md`, `docs/contracts/ML_CONTRACT.md`                                               |
| generated contract vocabulary                  | `contracts/catalog/`, relevant generator docs and ADRs                                                           |
| trajectories or ML-facing data                 | `docs/ML_TRAJECTORIES.md`, `docs/ML_ENVIRONMENT.md`                                                              |
| Commander or another format                    | `docs/FORMAT_MODULES.md`, relevant format specifications                                                         |
| Magic rules or mechanics                       | `docs/rules/AUTHORITY_POLICY.md`, `docs/rules/ADDING_RULES_AND_MECHANICS.md`, `docs/rules/MECHANIC_LIFECYCLE.md` |
| cards or Card IR                               | `docs/cards/ADDING_CARDS.md`, `docs/CARD_IR.md`                                                                  |
| capabilities or dependency closure             | `docs/cards/CAPABILITY_MODEL.md`                                                                                 |
| support or certification claims                | `docs/cards/CERTIFICATION.md`, `docs/contracts/ACCEPTANCE_GATES.md`                                              |
| native executors                               | `docs/cards/NATIVE_EXECUTOR_POLICY.md`                                                                           |
| conformance or correctness evidence            | `docs/TESTING_AND_CONFORMANCE.md`, `docs/testing/CONFORMANCE_AUTHORING.md`                                       |
| noninterference                                | `docs/testing/NONINTERFERENCE_TESTING.md`, `docs/THREAT_MODEL.md`                                                |
| property tests or fuzzing                      | `docs/testing/PROPERTY_AND_FUZZING.md`                                                                           |
| API stability                                  | `docs/maintenance/API_LIFECYCLE.md`                                                                              |
| schema evolution                               | `docs/maintenance/SCHEMA_EVOLUTION.md`                                                                           |
| release/freeze behavior                        | `docs/contracts/ACCEPTANCE_GATES.md`, current milestone specification and status                                 |

When an accepted ADR covers the change, read it before implementation.

Do not copy normative prose from these documents back into `AGENTS.md`.

---

# 6. Cross-layer changes

Public contracts often span multiple representations.

Before changing one, search for all affected representations, including where applicable:

```text
Rust
Python
contract catalog
generated vocabulary
JSON Schema
Golden fixtures
negative fixtures
conformance cases
replay formats
trajectory formats
documentation
ADRs
```

Do not fix only the first representation you encounter.

Where the repository has a normative generator, change the authoritative source rather than generated output.

After changing generated contract vocabulary, run the repository's generation and drift checks.

Mechanical duplication should be generated where practical.

Magic semantics, information-flow rules, state invariants, and cross-field semantic validation must remain explicit reviewed logic rather than generated pseudo-rules.

---

# 7. Scope discipline

Prefer the smallest coherent change that satisfies the requested task.

Do not:

* perform unrelated refactors;
* rewrite architecture without demonstrated need;
* add speculative abstraction for distant features;
* optimize before profiling;
* add card-specific hacks for behavior that belongs to a reusable mechanic;
* weaken verification merely to make a gate pass;
* silently widen support claims.

If you discover an unrelated problem:

* fix it only if it blocks the requested work or creates immediate correctness risk;
* otherwise record/report it separately.

---

# 8. Maintainer workflows

Use the cheapest verification level appropriate for the current work.

## Diagnose environment

```bash
just doctor
```

Use for toolchain/setup problems.

## Bootstrap development environment

```bash
just bootstrap
```

Bootstrap must not silently rewrite frozen contracts.

## Fast development loop

```bash
just check-fast
```

Use repeatedly during implementation.

It should provide quick feedback for common structural, generated-contract, language, and schema errors.

## Integration

```bash
just check
```

Run before presenting implementation or contract work as ready for review.

## Full verification

```bash
just check-all
```

Use for broad semantic changes, cross-layer contract changes, replay/information-boundary changes, and release preparation.

## Release candidate

```bash
just release-candidate
```

Use only when preparing a candidate for freeze/release.

Do not force release-level ceremony into every local edit.

---

# 9. Verification evidence

Verification must not mutate the source state being verified.

Generated logs and reports belong in the repository-defined verification output location, currently outside the reproducible source archive.

The final reproducibility/archive check must happen after source-changing operations.

Do not modify archived source after the final archive gate and still claim that archive was the tested state.

---

# 10. Working procedure for agents

For nontrivial tasks:

1. Resolve the exact requested scope.
2. Read the relevant normative sources using the trigger map.
3. Inspect current implementation and tests.
4. Identify affected contracts and layers.
5. State internally which invariants must remain true.
6. Make the smallest coherent implementation.
7. Update all affected representations.
8. Add or update tests.
9. Run `check-fast` or narrower tests early.
10. Run the required broader profile before completion.
11. Inspect the resulting diff for accidental scope growth.
12. Report exactly what changed.
13. Report exactly which gates ran.
14. Report every required gate that remains `FAIL`, `BLOCKED`, or `NOT_RUN`.

Never hide uncertainty behind confident wording.

---

# 11. Review procedure

When reviewing Manafold, prioritize findings by impact on trustworthy simulation.

Use severity when useful:

* `BLOCKER`
* `MAJOR`
* `MINOR`
* `NIT`

Use precise categories where useful:

* architectural contradiction
* cross-layer contract drift
* information leak
* determinism/replay risk
* missing invariant
* validation gap
* implementation bug
* conformance gap
* documentation contradiction
* maintainer ergonomics issue
* performance concern

Prefer:

> `PlayerEndpoint exposes X through Y, violating Z contract.`

over:

> `This feels unsafe.`

Distinguish between:

* an architecture problem;
* an incomplete implementation;
* an incomplete validator;
* a stale generated artifact;
* a documentation problem;
* an unexecuted gate.

These require different fixes.

---

# 12. Definition of done

Code being written is not sufficient.

For ordinary implementation work, completion normally requires:

* requested behavior implemented;
* relevant invariants preserved;
* appropriate tests added or updated;
* affected docs/contracts updated where necessary;
* generated artifacts synchronized;
* applicable checks executed successfully;
* no known blocker hidden.

For cross-language/public contract changes also verify:

* Rust/Python/schema parity where applicable;
* fixtures;
* compatibility;
* replay implications;
* information-boundary implications;
* trajectory implications.

For support/certification changes also verify:

* reachable capability closure;
* required evidence;
* native-executor policy;
* certification outcome.

Do not call a task complete when a required gate remains unknown.

---

# 13. Freeze rule

A design that looks complete is not a frozen contract.

A freeze requires the repository-defined freeze gates to **actually execute and pass**.

In particular:

```text
Freeze Candidate ≠ Frozen Contract
```

Do not begin the next major milestone merely because documentation, scaffolding, or non-native checks look good.

If required Rust, Python, schema, conformance, information-safety, replay, or reproducibility gates have not run successfully:

**the freeze remains blocked.**

---

# 14. Agent-specific prohibitions

Do not:

* invent test results;
* infer compilation success from syntax inspection;
* infer runtime correctness from compilation;
* infer card support from parser success;
* infer certification from implementation;
* hand-edit generated contract vocabulary when an authoritative generator exists;
* silently choose between contradictory contracts;
* turn Python into a second Magic rules engine;
* introduce hidden semantic state for convenience;
* bypass information boundaries for search or debugging;
* replace missing decisions with heuristic or random choices;
* weaken fail-closed behavior to make progress appear faster;
* change accepted architectural decisions casually;
* create support claims unsupported by evidence.

---

# 15. Final agent rules

When forced to choose between **quick progress** and **trustworthy semantics**, preserve trustworthy semantics.

When forced to choose between **guessing** and **failing closed with a precise diagnostic**, fail closed.

When a behavior is genuinely reusable across cards, prefer a reusable rule capability over duplicated card-specific logic.

When an existing normative contract answers the question, use it rather than reproducing your own version of the rule.

When evidence has not been produced, say so.

**Manafold's output may become ML training data. Treat semantic correctness and provenance accordingly.**

---

## Agent skills

### Issue tracker

Issues are tracked as GitHub issues on `github.com/chrismaghuhn/Manafold`, via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role triage vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.
