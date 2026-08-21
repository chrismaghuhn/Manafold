# ADR 0039: Perspective-local decision identity and typed staged choices

- **Status:** proposed
- **Date:** 2026-08-21
- **Supersedes:** none; M1 Decision V1 retains its historical meaning
- **Unblocks:** M2 structural Decision V2 implementation after ADR 0040/V3 persistence contract are also accepted

## Context

M1 intentionally exercises one narrow synthetic `ChooseOne` path. Its public request exposes the same internal `DecisionId` used by trusted state, candidates use arbitrary string IDs and mandatory semantic-key strings, responses use an assignment list with optional ordinal, authoritative bindings are stored separately from visible candidates, and continuations carry only an ID plus label.

Those shapes proved the deterministic transaction shell but are not sufficient for M2:

- global internal decision allocation history can become player-visible identity history;
- assignment-plus-ordinal overloads unordered, ordered and numeric answer semantics;
- visible candidate and trusted binding storage can drift;
- mandatory semantic keys would freeze dataset/action semantics before OD-011;
- a string label is not executable checkpointable continuation state.

## Decision

### Identity separation

Use distinct meanings:

```text
DecisionId           trusted authoritative pending-decision identity
PlayerDecisionIdV1   perspective-local visible request identity
CandidateIdV1        request-local candidate identity
ContinuationId       trusted staged-execution identity
```

`PlayerDecisionIdV1` is allocated from the acting perspective's checkpointed identity state and must not depend on hidden/global allocation history.

`CandidateIdV1` is dense after canonical public ordering and valid for one request only. It is never a dataset label.

### Authoritative request owns exact candidate pairing

`EngineState` stores `AuthoritativeDecisionRequestV2`.

Every authoritative candidate record contains:

```text
candidate_id
visible_intent
trusted_binding
```

together. A separately maintained binding map is not the M2 authority.

Player projection produces `PlayerDecisionRequestV2` and strips:

- trusted `DecisionId`;
- `ContinuationId`;
- bindings/internal IDs;
- hidden context;
- allocation history.

### Closed decision/answer families

M2 decision domains:

```text
ChooseOne
ChooseMany { minimum, maximum }
ChooseNumber { minimum, maximum }
Order { minimum, maximum }
```

M2 responses use the corresponding answer union:

```text
SelectOne { candidate_id }
SelectMany { candidate_ids }
ChooseNumber { value }
Order { candidate_ids }
```

`SelectMany` has one canonical set representation: unique candidate IDs in ascending request-local order. `Order` preserves semantic sequence order. `ChooseNumber` is a direct numeric value, not fake candidates.

### Candidate ordering

Production candidate generation orders choices by player-authorized material only:

```text
visible intent variant rank
canonical visible payload bytes
```

Trusted IDs, candidate bindings, hidden definitions, insertion/hash-map order, RNG state, allocator history and continuation internals cannot break ties.

If two distinct non-equivalent bindings are publicly indistinguishable under the declared player choice, generation fails closed rather than sorting by hidden identity.

### Semantic keys remain deferred

`PlayerDecisionRequestV2` does not contain a mandatory semantic action key. OD-011 remains open.

Future trajectory/action abstraction is independently versioned and may derive safe semantic keys only after information-safety review.

### Typed serialized continuation

M2 uses a closed typed continuation payload inside `EngineState`.

For the bounded synthetic chain:

- one trusted `ContinuationId` persists through all stages;
- every stage receives a fresh `DecisionId`, `PlayerDecisionIdV1`, and current state revision;
- actor, stage index and partial semantic values are serialized state;
- rejected responses preserve the entire continuation exactly;
- completion removes the continuation.

Closures, controller callbacks, threads, native stack frames, label interpreters and free-form scripts cannot be authoritative continuation state.

M2 does not freeze nested/recursive continuation composition. M3 may add typed frame composition when real rules evidence requires it, preserving the same explicit state/replay boundary.

### Validation and rejection

Wire decoding precedes semantic submission. Once `DecisionResponseV2` exists, validation proceeds through perspective-bound request identity/revision, answer-domain match, candidate membership/uniqueness/canonical form, cardinality/range, binding integrity, contextual legality and continuation-stage support before commit.

A typed semantic rejection mutates nothing.

Malformed/noncanonical bytes are not a semantic response and produce no semantic PlayerStep/replay step.

## Compatibility

This is a semantic break from Decision V1.

M2 allocates new player request/response schema versions. V1 values/fixtures retain their original M1 meaning and are not rewritten.

The new trusted request/continuation state contributes to `FullStateDigestV3`; V2 full-state/checkpoint/replay identities are not reinterpreted.

## Consequences

Positive:

- player-visible request identity cannot leak hidden global decision history;
- answer semantics are unambiguous;
- visible/trusted candidate pairing has one authority;
- staged actions survive checkpoint/fork/replay;
- dataset-key stability is not accidentally frozen in M2;
- the same protocol can later represent modes, targets, ordering, numbers and staged costs.

Costs:

- cross-layer Decision V2 migration;
- perspective-local request allocator state;
- current M1 fixtures require historical/current-version separation;
- M2 soundness/completeness proof must explore staged paths.

## Rejected alternatives

- keep internal `DecisionId` as the player-visible ID;
- keep arbitrary string candidate IDs;
- use optional ordinals for both sets and ordered choices;
- represent numbers as candidates;
- keep a separate candidate-binding map as authority;
- retain mandatory semantic keys before OD-011;
- resume staged execution from strings/callbacks/controller state;
- auto-complete unsupported or optional choices.

## Evidence required

This ADR does not mark M2 behavior `PASS`.

Executable M2 evidence must prove:

- all four closed families;
- canonical insertion-order-independent candidate IDs;
- global-allocation-history noninterference;
- exact binding validation;
- continuation creation/advance/reject/complete;
- checkpoint/fork/replay parity;
- legal-choice soundness and completeness;
- complete rejection nonmutation.

## Review trigger

Revisit if a locked M3 capability demonstrates that the closed domains cannot represent a required meaningful player choice without loss, or if nested/simultaneous decision semantics require a new explicitly versioned protocol.
