# M1.5 Deterministic Services Design

**Status:** accepted for implementation
**Stability:** provisional
**Owner:** maintainer
**Starting master:** `da563135cebfc17efff6f2b6692950a0360f23ed`

## Goal

Extend the accepted M1 synthetic `select_public_object` transition with one
authoritative draw from the existing `SyntheticM1/Global` stream and one
authoritative `EffectInstanceId` allocation. The complete transition product
must remain deterministic, checkpointable through the existing state contract,
exactly auditable, and atomically nonmutating on rejection or trusted service
failure.

The request's design is treated as approved: M1.5 is the only implementation
scope. Environment transaction ownership, restore, fork, replay execution,
endpoint submission, and M2 information-safety work remain outside this
change.

## Reconciled constraints

- `origin/master` was fetched at task start and is exactly
  `da563135cebfc17efff6f2b6692950a0360f23ed`.
- The current M1.4 kernel already derives the acting player from
  `pending.request.actor`; the non-default `[PlayerId(7), PlayerId(9)]`
  regression remains mandatory.
- `mtgml.rng.v1`, `RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1)`,
  and `uniform_below_u64` are reused without reinterpretation or version bump.
- The root seed, typed stream key, cursor, raw-word count, and internal effect
  identity remain trusted data. No player-facing type or wire schema is
  extended.

## Considered approaches

1. **Typed authoritative event plus typed allocator operation — selected.**
   Add `RandomValueSampled` and the matching `SemanticDeltaOperation`, extend
   the existing semantic cursor with the before-state root seed, and add one
   `IdentityAllocatorState::allocate_effect_id` operation. This proves causal
   RNG evidence while keeping allocator-only bookkeeping out of the rule
   event stream.
2. **Encode both services as public outcome events — rejected.**
   This would make internal RNG provenance and an internal effect identity look
   like rule semantics and would create an unsafe projection pressure.
3. **Advance state counters without a typed RNG event — rejected.**
   The final random cursor would change without a sequential causal event, so
   `SemanticValidationCursor` could not prove the transition rather than merely
   compare its final projection.

## Authoritative data flow

After complete response, actor, decision, candidate, and binding validation,
the kernel verifies that the required stream exists. A missing
`SyntheticM1/Global` stream returns the existing exact rejected product and
consumes nothing. Otherwise it clones `EngineState` and performs this local,
checked order:

```text
A. allocate EffectInstanceId from next_effect_id
B. mutate acting-player life 40 -> 39; append LifeChanged
C. mutate acting-player life 39 -> 38; append LifeChanged
D. call EngineState::uniform_below_u64(SyntheticM1/Global, 10)
E. append RandomValueSampled with exact cursor/audit fields
F. clear DecisionId(1); append DecisionCleared
G. set one outer revision and the contiguous rule-event allocator
H. build the complete StateDelta and validate the complete product
```

The canonical `"11".repeat(32)` fixture must independently assert:

```text
stream              = SyntheticM1 / Global
bound               = 10
value               = 1
raw_words_consumed  = 1
cursor_before       = 0
cursor_after        = 1
effect_id           = EffectInstanceId(1)
next_effect_id     = EffectInstanceId(2)
```

The value is derived from the accepted `mtgml.rng.v1` primitive, not read from
the generated event. The root seed remains present in both before and after
authoritative state.

## Event and delta contract

The accepted canonical trace uses one revision (`0 -> 1`) and four contiguous
rule-event IDs (`1 -> 5`):

```text
1. LifeChanged(actor, 40 -> 39)
2. LifeChanged(actor, 39 -> 38)
3. RandomValueSampled(stream, 10, 1, 1, 0, 1)
4. DecisionCleared(DecisionId(1))
```

`AuthoritativeRuleEventKind::RandomValueSampled` and
`SemanticDeltaOperation::RandomValueSampled` carry the same typed fields:

```rust
RandomValueSampled {
    stream: RandomStreamKeyV1,
    bound: u64,
    value: u64,
    raw_words_consumed: u64,
    cursor_before: u64,
    cursor_after: u64,
}
```

The semantic cursor retains the before-state root seed and per-stream cursors.
For each random event it requires the exact current `cursor_before`, reruns the
authoritative `uniform_below_u64` primitive, checks bound/value/consumption/
`cursor_after`, and advances its local cursor. Final cursor-map equality with
the after-state remains mandatory, as does root-seed equality. Event-to-audit
equality remains mandatory.

The effect allocator advances only in the complete replacement state. No fake
effect record or semantic event is created. `StateDelta::apply(before)` must
equal `next_state`, including the random cursor and effect allocator, and the
reapplied digest must equal the next-state digest.

## Failure and atomicity

`IdentityAllocatorState::allocate_effect_id` returns the current typed ID and
checked-increments it, but rejects `u64::MAX` with a typed
`IdentityAllocationError`. `KernelExecutionError` wraps this error and
`RandomValidationError` for trusted deterministic-service failures.

Allocator exhaustion is checked before the RNG draw. RNG exhaustion is a
typed internal error from the authoritative sampler. Neither error is a
normal player rejection. Since all service consumption occurs on a cloned
workspace, every failure leaves the input `EngineState` unchanged and emits
no externally returned events or delta.

Normal rejected responses, including a structurally valid synthetic state with
the required stream absent, return the existing exact rejected product: no
state, digest, revision, pending-decision, event, audit, RNG cursor, or
allocator mutation. The repository-level
`REJECTED_RESPONSE_COMPLETE_NONMUTATION` gate remains `BLOCKED` because #25
still owns environment/replay closure.

## Checkpoint and information-safety evidence

No environment dependency is added to `mtgml-rules`. The existing
`EnvironmentCheckpointV2` test ownership receives a focused DTO round-trip
test using a valid state with cursor `1` and `next_effect_id = 2`; it validates
the checkpoint/state digests and exact preservation of both continuation
values. This is capture evidence only and does not promote restore/fork/replay
gates.

The change adds no fields to `PlayerEndpoint`, `PlayerStep`, observations,
information state, observed events, player errors, trajectories, or schemas.
The authoritative RNG event is trusted kernel data and is never mapped to a
player projection. No root seed, stream key/cursor, raw-word count, or
`EffectInstanceId` is exposed through a player-facing type.

## Verification obligations

Focused Rust tests cover the exact canonical event/state product, repeated
complete `TransitionResult` equality, non-default actor identity, stream
isolation, all RNG-event negative cases, rejection nonmutation, both service
exhaustion paths, full delta/digest parity, and checkpoint capture. The
existing M1.3 rejection matrix is rerun unchanged.

The final report records only executed statuses. It preserves
`REJECTED_RESPONSE_COMPLETE_NONMUTATION = BLOCKED` and leaves checkpoint
restore, fork, replay, and endpoint-binding gates `NOT_RUN`. No M1.6+ runtime
implementation is included.
