# Conformance Case Authoring

**Status:** accepted process

## One claim per minimal case

A case should isolate one rule/capability claim while retaining every state component required for realism. Large end-to-end games supplement, not replace, minimal cases.

## Required fields

- case ID and capability version;
- authority snapshot and cited rule/ruling identifiers;
- complete initial `EngineState` or deterministic builder input;
- current decision and candidate-set expectation;
- submitted response;
- accepted/rejected expectation;
- exact semantic events and delta expectations;
- next full digest, decision, and status;
- per-player observation, information, and observed-event expectations;
- checkpoint/fork/replay requirements;
- pre-fix failure explanation for regressions.

## Assertion strategies

Use exact equality for stable semantic values. Structural matchers are allowed only where the contract explicitly permits unconstrained identifiers and must still prove cardinality, ownership, visibility, and relationships.

## Rejection cases

Every decision family includes malformed, stale, wrong actor, unknown candidate, duplicate, cardinality, and mismatched-binding cases as applicable. All assert complete nonmutation.

## Differential notes

Record independent-engine behavior as evidence, not authority. A disagreement remains visible in the case history.

## Input assertions

A conformance runner must receive and compare the actual visible decision and the actual submitted response. Merely storing `expected_current_decision` and `response` in a case without asserting them is a failed harness contract.

## Compositional event traces

Cases that can emit repeated or chained events include explicit intermediate values. At minimum, synthetic coverage includes:

- two life changes to the same player in one accepted revision;
- clearing one pending decision and creating the next;
- two consumptions of the same RNG stream;
- consecutive zone transitions with complete old/new snapshots and LKI;
- mixed tap, zone, decision, and public-outcome sequences where reachable.

The final full-state digest and exact `StateDelta` remain outer atomic assertions.
