# M1.2 Accepted Transaction Design

## Scope

Implement one accepted synthetic player-response transaction on top of the
M1.1 canonical `EngineState`. The only supported accepted action is selecting
the existing public `select_public_object` candidate from the pending
`ChooseOne` request. The action has no Magic meaning; it only clears the
pending decision and proves the atomic state/event/delta boundary.

The implementation starts from master
`796dbc00a9ead81ede9c0e76f08446db8da85882` on a dedicated `chris/` branch.
M1.3 rejection completeness, M1.4 multi-event composition, M1.5 RNG and
allocator consumption, PlayerEndpoint coexistence, and real Magic semantics
remain out of scope.

## Boundary and error semantics

The existing `RulesKernel` boundary is corrected to carry the trusted
submitting actor explicitly:

```rust
fn apply(
    &mut self,
    state: &EngineState,
    trusted_actor: PlayerId,
    response: &DecisionResponse,
) -> Result<TransitionResult, KernelExecutionError>;
```

`TransitionResult.accepted` remains part of the product contract.

- `Ok(TransitionResult { accepted: true, .. })` is a valid accepted player
  transition.
- `Ok(TransitionResult { accepted: false, .. })` is a fail-closed player
  submission rejection. It uses the existing exact nonmutation contract:
  unchanged state, revision, digests, RNG, allocators, knowledge, identities,
  current pending decision, empty events/audit, and `Running` status.
- `Err(KernelExecutionError)` is reserved for trusted/internal failures such as
  invalid authoritative input state, checked revision/event-ID overflow,
  digest construction failure, invalid resulting state, or transition/event/
  delta contract failure.

The kernel does not expose a new player-facing rejection mapping. M1.3 owns the
complete rejection matrix and safe endpoint error surface.

## Components and data flow

`SyntheticM1RulesKernel` is the minimal concrete `RulesKernel` implementation
in `mtgml-rules`. It executes the following ordered flow:

1. Validate the complete before-state. An invalid authoritative state is an
   internal execution error.
2. Validate the trusted actor, response schema, pending decision identity,
   expected revision, exact `ChooseOne` cardinality, candidate membership, and
   exact visible-to-authoritative candidate binding. Any failed player
   precondition returns an exact rejected `TransitionResult` without creating
   an accepted workspace.
3. Clone the complete `EngineState` into a local transition workspace.
4. Checked-increment the revision, clear the pending decision, and checked-
   increment only `next_rule_event_id`. All RNG, zones, core state, knowledge,
   perspective identities, and other allocators remain unchanged.
5. Emit exactly one event:

   ```text
   RuleEventId(1)
   state_revision = 1
   DecisionCleared { decision: DecisionId(1) }
   ```

6. Build `StateDelta::between(before, after, audit)` with exactly one matching
   `SemanticDeltaOperation::DecisionCleared` audit entry.
7. Derive `next_decision = None` and `status = EpisodeStatus::Running`.
8. Validate the resulting state and the existing transition/event/delta
   contract before returning the accepted product.

No authoritative object is mutated; the caller receives the new value and is
responsible for any later environment commit integration.

## Exact evidence

The focused M1.2 test constructs the state through
`construct_synthetic_engine_state(...)` and compares the complete before and
after products. It asserts the pending request/candidate/binding, actor and
response, revision and event-ID changes, unchanged RNG and unrelated
allocators, unchanged zones/core/knowledge/identity/format data, one exact
event, one exact audit entry, `accepted == true`, no next decision, and
`Running` status. It also applies the emitted delta to the before-state and
compares the resulting state and digest with the authoritative next-state
product; the before and after V2 digests must differ.

Minimal negative evidence covers only the preconditions needed to protect the
accepted path (for example, wrong trusted actor and a binding mismatch). These
cases assert the exact rejected product but do not claim
`REJECTED_RESPONSE_COMPLETE_NONMUTATION = PASS`.

## Verification and delivery

Run focused Rust tests first, then the requested locked workspace tests,
format/check/clippy commands, repository checks, and the exact-head PR Fast,
Integration, and Nightly Certification Smoke workflows. Record every gate as
`PASS`, `FAIL`, `NOT_RUN`, or `BLOCKED`; only the two M1.2 gates may become
`PASS`, and only after their executable evidence succeeds. Open one Draft PR
targeting `master` and do not merge it.
