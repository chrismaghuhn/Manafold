# M1.3 Rejection Atomicity Design

**Status:** accepted for implementation
**Stability:** provisional
**Owner:** maintainer

## Scope

M1.3 extends the existing one-event `SyntheticM1RulesKernel` evidence from
minimal negative cases to a table-driven rejection matrix. Normal malformed,
stale, wrong-actor, unsupported, and semantically invalid responses must return
`Ok(TransitionResult { accepted: false, .. })` before any accepted-workspace
mutation. The existing `RulesKernel::apply` boundary and M1.2 accepted path do
not change.

The only production behavior correction is to reject `CandidateAssignment`
values with `ordinal: Some(...)` on the M1 `ChooseOne` path. This field has no
meaning for the supported synthetic decision and must not be silently ignored.

## Ownership and boundary

`EngineState` is the complete authoritative nonmutation surface. Rejection
tests compare the complete state and explicitly name revision, V2 digest,
pending request/bindings/continuation, random root/streams/cursors, all
allocator cursors, zones/stack/core, knowledge/history lengths, perspective
identity maps, and format state. The returned product must contain no events or
audit entries, an identity delta, the unchanged current decision, `Running`
status, and a delta that reapplies exactly to the before-state.

`mtgml-rules` owns this executable transition product and the distinction
between player rejection and `KernelExecutionError`. A malformed trusted
before-state is rejected by `validate_engine_state` as
`Err(KernelExecutionError::BeforeState(...))`; in particular, a forged trusted
candidate binding cannot be supplied by `DecisionResponse`, and a binding
mismatch that makes the authoritative before-state invalid is an internal
state error rather than a player rejection.

`EnvironmentCheckpointV2`, `EnvironmentLimitCounters`, episode status, and the
closed `PlayerApiError` enum remain owned by `mtgml-environment`. The current
repository has no executable environment transaction owner that invokes the
synthetic kernel and commits these fields, so M1.3 will not invent one. The
same applies to accepted replay-step count and replay history: `mtgml-replay`
currently supplies V2 DTOs and validation, not a runtime recorder. These
surfaces are recorded as `BLOCKED` or `NOT_RUN`, not promoted to `PASS`.

Observation and information-state types remain projections/DTOs. Their
sequence fields and `KnowledgeState` remain untouched; no new history owner is
introduced in this milestone.

## Rejection matrix

The matrix uses one reusable exact-product assertion and covers:

| Class | Cases |
| --- | --- |
| Actor/identity | wrong trusted actor, stale state revision, wrong decision ID |
| Response shape | schema mismatch, empty candidate ID, duplicate assignment, zero assignments, more than one assignment for `ChooseOne` |
| Assignment semantics | unknown candidate, unsupported ordinal, valid `Confirm` candidate/binding, multiple valid candidates, valid continuation, other valid unsupported decision family, no pending decision |
| Trusted/internal distinction | invalid authoritative allocator state, visible/authoritative binding mismatch |

Every normal rejection is required to preserve the complete currently-owned
authoritative surface and to pass `validate_transition_contract`. The internal
cases must return `BeforeState` errors and must not be converted to player
rejections.

## Player-safe surface

No authoritative IDs, bindings, seeds, RNG cursors, allocator values, hidden
identities, knowledge internals, trusted diagnostics, or kernel error text are
added to `DecisionResponse` or player errors. The existing closed
`PlayerApiError` variants are checked for sanitized display output. Because no
real environment submit mapping exists yet, the test documents sanitization of
the existing public error surface without fabricating an endpoint transaction.

## Out of scope

This design does not add environment commit/checkpoint/replay execution, RNG
draws, allocator consumption, multiple dependent rule events, new semantic
cursor families, endpoint binding, real Magic semantics, or M1.4+ behavior.

The resulting M1.3 gate is `PASS` only if every required owner has executable
evidence. With the current repository ownership, the rules-level evidence can
be complete while the overall `REJECTED_RESPONSE_COMPLETE_NONMUTATION` gate
remains `BLOCKED` on the missing environment/replay execution path.
