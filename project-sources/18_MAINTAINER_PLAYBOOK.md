# Maintainer Playbook

**Status:** accepted process index  
**Last reviewed:** 2026-08-18

## Before changing anything

1. Identify the semantic surface and its stability class.
2. Read the relevant normative docs and ADRs.
3. Check [`OPEN_DECISIONS.md`](../docs/OPEN_DECISIONS.md) and the design lock matrix.
4. Pin authority/input artifacts.
5. State the support claim and exact non-goals.
6. Add red evidence before changing behavior.

## Architecture or contract change

- create an ADR;
- update every Rust/Python/schema/fixture representation together;
- classify compatibility/migration;
- update the normative register if documents/surfaces change;
- run direct and transitive capability/leak checks;
- preserve old fixtures/artifacts where compatibility requires it.

## Adding a mechanic

Follow [`rules/ADDING_RULES_AND_MECHANICS.md`](../docs/rules/ADDING_RULES_AND_MECHANICS.md). Use `scripts/scaffold_capability.py`, specify authority/state/events/decisions/information/order, add red conformance cases, implement reusable primitives, then advance lifecycle only with evidence.

## Adding a card

Follow [`cards/ADDING_CARDS.md`](../docs/cards/ADDING_CARDS.md). Use `scripts/scaffold_card.py`, pin provenance, review generated IR, declare capabilities, test decisions/information/interactions, and certify only through a locked bundle.

## Changing a deck or bundle

Generate a scope-impact report. Recompute reachable definitions and recursive capability closure. Invalidate or supersede affected certification. Never replace a deck entry while retaining the previous bundle digest or claim.

## Hidden-information change

- enumerate each authorized perspective;
- specify knowledge gain/retention/invalidation;
- specify opaque identity lifecycle;
- add paired-state noninterference cases;
- review errors, events, candidate order, counts, and trajectory metadata for leaks.

## Replay/determinism change

- identify digest/schema/algorithm impact;
- add golden and negative fixtures;
- run repeated reset/step/checkpoint/fork/replay comparisons;
- prove no wall-clock/thread/map-order dependency;
- publish migration or new version rather than reinterpret old artifacts.

## Performance change

Preserve semantic parity first. Benchmark a pinned workload and report distributions, memory, and raw evidence. An optimization without reference-backend parity is not accepted.

## Release

Follow [`maintenance/RELEASE_PROCESS.md`](../docs/maintenance/RELEASE_PROCESS.md) and [`maintenance/FREEZE_LEVELS.md`](../docs/maintenance/FREEZE_LEVELS.md). Generate reports from executed commands; do not edit gate status manually.
