# M1.4 Sequential Event/Delta Composition Design

**Status:** accepted for implementation
**Stability:** provisional
**Owner:** maintainer
**Starting master:** `476db61152990eda7f74198ef853b924746fac39`

## Scope

M1.4 extends the existing accepted synthetic `select_public_object`
transaction in `SyntheticM1RulesKernel`. The response remains one atomic
outer transition, but its local workspace performs three ordered semantic
mutations:

1. `PlayerId(1)` life `40 -> 39`;
2. `PlayerId(1)` life `39 -> 38`;
3. clear `DecisionId(1)`.

The selected object remains semantically inert. The life changes are synthetic
composition evidence and are not combat, damage, payment, loss of life, spell
resolution, or another Magic mechanic.

## Ownership and boundaries

`mtgml-rules` remains the owner of the accepted transition product and its
sequential event/delta contract. `EngineState` remains authoritative. The
kernel clones a local workspace only after all M1.3 response and binding checks
pass, mutates that workspace in event order, advances the outer revision once,
and validates the complete product before returning it.

No environment transaction owner, checkpoint/restore, fork, replay recorder,
endpoint binding, RNG draw, non-rule allocator consumption, new decision, or
real Magic semantics is introduced. The pre-existing rule-event allocator
advances from
`RuleEventId(1)` to `RuleEventId(4)` solely to identify the three emitted
events. The repository-level `REJECTED_RESPONSE_COMPLETE_NONMUTATION` gate
remains `BLOCKED`; its environment/replay closure belongs to #25.

## Exact accepted product

The accepted result has:

```text
revision: 0 -> 1
P1 life: 40 -> 38
P2 life: unchanged at 40
pending decision: DecisionId(1) -> None
next_rule_event_id: 1 -> 4
```

All other authoritative state, including RNG root/streams/cursors, every
non-event allocator, zones, objects, stack, turn/core fields, knowledge,
perspective identities, and format state, is unchanged. The exact ordered
events are:

```text
RuleEventId(1), state_revision 1, LifeChanged(P1, 40, 39)
RuleEventId(2), state_revision 1, LifeChanged(P1, 39, 38)
RuleEventId(3), state_revision 1, DecisionCleared(DecisionId(1))
```

`StateDelta.audit` is exactly the three corresponding
`SemanticDeltaOperation` values in the same order. Its complete replacement,
not the audit, remains the reconstruction mechanism; applying it to the
before-state must produce the authoritative after-state and equal digest.

## Sequential validation evidence

The existing `SemanticValidationCursor` and
`validate_transition_contract()` remain the only event validation path. The
cursor starts from the before-state, applies each event in vector order, and
must equal the after-state projection. Focused contract tests cover:

- event 2 claiming `40 -> 38` after event 1 moved the cursor to `39`;
- reversed dependent events `39 -> 38`, then `40 -> 39`;
- an after-state at life `38` with only the first life event;
- an audit vector that disagrees with the ordered event vector;
- the exact mixed `40 -> 39`, `39 -> 38`, `DecisionCleared` sequence.

The first four cases must reject with the relevant transition violation; the
last case must pass and its final cursor must match the committed state.

## Implementation files

- `crates/mtgml-rules/src/synthetic.rs`: extend only the accepted synthetic
  workspace and event accumulation.
- `crates/mtgml-rules/src/tests.rs`: update the exact accepted product and add
  sequential-cursor negative/positive contract evidence.
- `docs/superpowers/specs/2026-08-21-m1-4-sequential-event-delta-design.md`:
  this design and ownership record.
- `docs/superpowers/plans/2026-08-21-m1-4-sequential-event-delta.md`: the
  executable implementation plan.
- `docs/normative-document-register.v1.json`: register the two provisional
  process artifacts.

No generated vocabulary, schema, state/delta type, event family, RNG service,
allocator service, environment/replay owner, endpoint, or status artifact is
changed by this design.

## Verification and delivery

Run the focused M1.4 rules tests first, then the locked Rust, formatting,
repository, and `just` checks requested by issue #23. Classify every command
from its actual result as `PASS`, `FAIL`, `BLOCKED`, or `NOT_RUN`. On the exact
final head, inspect hosted PR Fast, Integration, Nightly Certification Smoke,
and CodeQL results if available. Push one dedicated `chris/` branch and open
one Draft PR against `master`; do not merge.

The only M1 gate this issue may promote is
`SEQUENTIAL_EVENT_DELTA_PARITY`, and only after its executable evidence passes.
`DETERMINISTIC_RNG_AND_ALLOCATORS`, checkpoint/restore, fork, replay, and
multi-player endpoint gates remain `NOT_RUN`.
