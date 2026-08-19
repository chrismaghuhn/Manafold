# Execution and Transaction Model

**Status:** accepted execution ownership; exact Magic ordering deferred to pinned M3 specifications  
**Stability:** normative transaction boundary

## One semantic transaction

A submitted player response is handled as one atomic transaction:

```text
1. locate perspective-bound pending request
2. decode and structurally validate response
3. verify actor, decision ID and expected revision
4. validate assignment/cardinality/numeric constraints
5. resolve visible candidates to exact authoritative bindings
6. create a transition workspace from EngineState
7. execute the currently specified rule program/continuation
8. accumulate semantic events and an exact state replacement delta
9. perform all mandatory forced progress required before the next choice
10. validate EngineState and event/delta parity
11. derive next decision and EpisodeStatus
12. commit state, replay step and perspective projections together
```

Steps 7–9 are placeholders for M3’s pinned cost, replacement, trigger, state-based-action, priority, and resolution ordering. M0.2 fixes where those semantics live; it does not invent them.

## Rejection

Malformed, stale, wrong-actor, illegal, unsupported, or mismatched submissions reject before commit. Rejection preserves exactly:

- full-state digest and revision;
- current decision and bindings;
- RNG streams/counters;
- identity allocators;
- knowledge and opaque identity maps;
- replay accepted-step count;
- visible history and observed-event sequence.

Only a sanitized player error may be returned through the endpoint. Trusted diagnostics may contain more detail but cannot alter player-visible bytes.

## Forced progress

Rules that require no player choice execute inside the authoritative kernel until one of these boundaries:

- a player decision is required;
- the game reaches a terminal outcome;
- a technical safety limit produces truncation;
- unsupported semantics or an invariant failure abort trusted execution.

Forced progress never guesses an optional choice.

## State/event/delta product

An accepted transition produces:

```text
next EngineState
ordered AuthoritativeRuleEvent[]
exact StateDelta
next AuthoritativeDecisionRequest?
EpisodeStatus
```

The exact delta must reproduce the entire next state. The semantic audit operations explain rule-relevant mutations but are not the sole reconstruction mechanism.

## Deterministic ordering

Whenever the rules do not permit implementation-dependent ordering, the engine follows pinned rules semantics. Whenever multiple representations are semantically equivalent but ordering is exposed to an API, M0.2 requires an explicit canonical ordering policy and a conformance test; container/hash-map iteration is never acceptable.

## Continuations

A resolution that pauses for player input stores a serializable continuation in `EngineState`. The pending decision references it. Resuming requires the same decision identity and state revision. Threads, closures, stack frames, and controller callbacks cannot be authoritative continuation state.

## V0.2.1 semantic validation cursor

An accepted response remains one atomic revision, but authoritative events are validated in sequence against an internal semantic cursor. The cursor starts from the before-state projection, each event validates and advances it, and the final cursor must equal the corresponding projection of the after-state.

Directly comparing every event with only the global before and after states is forbidden because it rejects valid sequences such as `40 -> 39 -> 38`, two uses of one RNG stream, or `DecisionCleared(old)` followed by `DecisionCreated(next)`.

Each new semantic event family must define:

- its cursor precondition;
- its cursor mutation;
- its exact `SemanticDeltaOperation` representation;
- its final-state projection equality;
- repeated-event and mixed-event conformance cases.

Bookkeeping changes remain in the exact full `StateDelta` even when they have no standalone rule event.

## Complete checkpoints

The controller checkpoint contract is `EnvironmentCheckpointV1`, not bare `EngineState`. It includes the full-state digest, episode status, declared decision/transition/event/resource/wall-clock counters, and a codec identity. Restore validates the complete checkpoint before backend mutation.
