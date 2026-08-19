# Testing and Conformance

**Status:** accepted proof architecture

## Layers

1. local value/type validation;
2. cross-component `EngineState` invariants;
3. exact transition contract tests;
4. primitive rules/mechanic conformance;
5. card and interaction cases;
6. soundness/completeness generation tests;
7. information noninterference;
8. replay/checkpoint/fork determinism;
9. wire/schema/cross-language fixtures;
10. property, fuzz, soak, and performance evidence;
11. differential comparison where useful.

## Exact case structure

A case begins from complete state and specifies current decision/candidates, response, acceptance/rejection, exact events/delta/digests, next decision/status, and per-player projections. Minimum event count is diagnostic only.

## Test status

A source test that has never executed in the pinned toolchain is `NOT_RUN`, not `PASS`. Generated verification reports are the only release-status authority.

## Coverage meaning

Line/branch coverage can reveal untested code but does not prove semantic capability coverage. Capability evidence is tied to declared authority cases, interactions, information risks, and recursive bundle closure.
