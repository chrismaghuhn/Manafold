# Execution and Transaction Model

**Status:** accepted transaction ownership; M2 decision/information refinements are freeze candidates; exact Magic ordering deferred to M3  
**Stability:** normative transaction boundary

## One semantic transaction

A typed submitted player response is handled as one atomic transaction:

```text
1. locate perspective-bound visible/authoritative pending request
2. validate typed response schema/local shape
3. verify perspective-local request identity and expected revision
4. verify answer-domain match, membership, uniqueness and canonical form
5. validate cardinality/numeric/order constraints
6. validate exact visible-to-authoritative candidate binding
7. validate context-dependent legality and supported continuation stage
8. create a transition workspace from EngineState
9. execute the currently specified rule program/continuation
10. accumulate semantic events and exact state replacement delta
11. perform mandatory forced progress until next choice/outcome/stop
12. validate EngineState and sequential event/delta parity
13. derive next authoritative decision and EpisodeStatus
14. derive and validate every required perspective projection
15. commit state, status/counters, accepted replay step and projections atomically
```

Malformed/noncanonical wire bytes fail before step 1 because no typed semantic `DecisionResponse` exists. They do not create a semantic PlayerStep/replay step.

M3 will pin exact cost, replacement, trigger, SBA, priority, combat and resolution ordering. M2 fixes the generic transaction/continuation/information boundary without inventing those rules.

## Rejection

A typed semantic rejection preserves exactly:

- complete authoritative state and full-state digest/revision;
- current authoritative/player request and exact bindings;
- continuation payload/stage;
- RNG streams/cursors;
- trusted and perspective-local identity allocators;
- knowledge, opaque mappings and retired IDs;
- perspective-visible sequence state;
- episode status and environment counters;
- accepted replay history;
- player-visible bytes except the closed semantic rejection code.

A wire decode failure additionally proves zero mutation but is a wire-layer result, not semantic rejection.

Internal invariant/binding/projection/digest failure discards the workspace and returns only the closed endpoint service failure; trusted diagnostics remain private.

## Forced progress

Rules requiring no player choice execute inside the authoritative kernel until:

- a player decision is required;
- the game reaches a terminal outcome;
- a technical safety limit produces truncation;
- unsupported semantics or an invariant failure abort trusted execution.

Forced progress never guesses an optional choice.

## State/event/delta product

An accepted transition produces atomically:

```text
next EngineState
ordered AuthoritativeRuleEvent[]
exact StateDelta
next AuthoritativeDecisionRequest?
EpisodeStatus
```

The exact delta reproduces the entire next state. Semantic audit operations explain rule-relevant mutations but are not the sole reconstruction mechanism.

## Decision identity

M2 distinguishes:

```text
DecisionId           trusted authoritative identity
PlayerDecisionIdV1   perspective-local request identity
CandidateIdV1        request-local identity
ContinuationId       trusted staged-action identity
```

A player response is bound to the endpoint perspective, `PlayerDecisionIdV1` and expected revision. The player does not submit an actor or trusted `DecisionId`.

## Deterministic ordering

Whenever ordering is exposed to a player API, its canonical policy uses only authorized/public material.

Container/hash-map iteration, trusted IDs, allocation history and hidden values never define candidate/event/player DTO order.

For unordered choice semantics, one canonical wire representation is required. Semantic order choices preserve their declared sequence.

## Continuations

A resolution that pauses for player input stores a typed serializable continuation in `EngineState`.

For M2's bounded linear chain:

- one trusted `ContinuationId` persists;
- every stage gets fresh trusted/player-visible decision identities;
- stage index and partial values are explicit state;
- rejection mutates nothing;
- completion removes the continuation.

Threads, closures, stack frames, controller callbacks, free-form labels and interpreter state cannot be authoritative continuation state.

M3 may extend continuation composition through new typed state after evidence; M2 does not freeze a generic VM.

## Sequential semantic validation cursor

An accepted response remains one atomic revision, but authoritative events are validated in order against an internal semantic cursor.

The cursor starts from the before-state projection; each event validates and advances it; the final cursor equals the corresponding after-state projection.

Each semantic event family defines:

- cursor precondition;
- cursor mutation;
- exact `SemanticDeltaOperation`;
- final-state projection equality;
- repeated/mixed-event cases.

M2 information projection additionally uses a per-perspective visible-sequence cursor. Hidden events advance no sequence for that perspective; the final projection cursor must equal the next sequence stored in after-state.

## Complete checkpoints

M1's current checkpoint contract is `EnvironmentCheckpointV2`.

M2 changes execution/knowledge/perspective-identity state and therefore plans `EnvironmentCheckpointV3` bound to `FullStateDigestV3`/`CheckpointDigestV3`.

When the M2 runtime state cut occurs:

- V2 is not reinterpreted against the new `EngineState`;
- no duplicate legacy `EngineStateV2` is introduced merely to keep the V2 runtime checkpoint executable;
- historical V2 support is classified explicitly;
- V3 restore validates complete state, episode status, limit counters and codec identity before backend mutation.

The ADR-0038 V3 persisted digest bytes are specified in [`STATE_HASHING.md`](STATE_HASHING.md).
