# M1.4 Sequential Event/Delta Composition Implementation Plan

**Status:** accepted

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Extend the accepted synthetic M1 response to produce the exact ordered 40 -> 39 -> 38 plus DecisionCleared event trace inside one atomic revision, with executable sequential-cursor and full-delta parity evidence.

**Architecture:** Keep EngineState authoritative and SyntheticM1RulesKernel as the single synthetic implementation. After existing M1.3 validation succeeds, mutate a cloned workspace in semantic order, assign all three events to one revision, advance only the rule-event allocator from 1 to 4, build a complete replacement StateDelta, and validate the complete product before returning.

**Tech Stack:** Rust 1.85.1, mtgml-rules, mtgml-state, FullStateDigestV2, AuthoritativeRuleEvent, SemanticDeltaOperation, locked Cargo tests, repository Python checks, just profiles, and GitHub Actions Draft PR checks.

---

**Starting master:** 476db61152990eda7f74198ef853b924746fac39
**Branch:** chris/m1-4-sequential-event-delta

## Files and responsibilities

- Create: docs/superpowers/specs/2026-08-21-m1-4-sequential-event-delta-design.md — accepted M1.4 scope and evidence.
- Create: docs/superpowers/plans/2026-08-21-m1-4-sequential-event-delta.md — this plan.
- Modify: docs/normative-document-register.v1.json — register both process artifacts.
- Modify: crates/mtgml-rules/src/tests.rs — exact accepted product and sequential-cursor contract evidence.
- Modify: crates/mtgml-rules/src/synthetic.rs — three-event accepted workspace only.
- Do not modify: contract.rs, semantic_cursor.rs, events.rs, validation.rs, StateDelta, environment/replay ownership, RNG services, allocator services, schemas, or endpoint types.

## Task 1: Commit the process artifacts

**Files:** the two M1.4 documents and docs/normative-document-register.v1.json

- [ ] Verify the branch and source identity:

~~~
git branch --show-current
git rev-parse HEAD
git rev-parse origin/master
~~~

Expected:

~~~
chris/m1-4-sequential-event-delta
476db61152990eda7f74198ef853b924746fac39
476db61152990eda7f74198ef853b924746fac39
~~~

- [ ] Validate and commit only the process artifacts:

~~~
python scripts/check_documentation.py
git diff --check
git add -- docs/superpowers/specs/2026-08-21-m1-4-sequential-event-delta-design.md docs/superpowers/plans/2026-08-21-m1-4-sequential-event-delta.md docs/normative-document-register.v1.json
git commit -m "docs: add M1.4 sequential event delta plan"
~~~

Expected: documentation validation succeeds and the commit contains only the two M1.4 artifacts plus their register entries.

## Task 2: Add the failing exact accepted-product test

**File:** crates/mtgml-rules/src/tests.rs

- [ ] Before production code, update synthetic_m1_acceptance_returns_exact_transition_product so expected_after changes only revision, P1 life, pending decision, and rule-event allocator:

~~~
let mut expected_after = before.clone();
expected_after.revision = StateRevision(1);
expected_after.core.players.get_mut(&PlayerId(1)).unwrap().life = 38;
expected_after.execution.pending_decision = None;
expected_after.allocators.next_rule_event_id = RuleEventId(4);
~~~

- [ ] Replace the one expected event with this exact ordered vector:

~~~
let expected_events = vec![
    AuthoritativeRuleEvent {
        event_id: RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::LifeChanged {
            player: PlayerId(1), from: 40, to: 39,
        },
    },
    AuthoritativeRuleEvent {
        event_id: RuleEventId(2),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::LifeChanged {
            player: PlayerId(1), from: 39, to: 38,
        },
    },
    AuthoritativeRuleEvent {
        event_id: RuleEventId(3),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::DecisionCleared {
            decision: DecisionId(1),
        },
    },
];
let expected_audit = expected_events
    .iter()
    .map(|event| event.event.semantic_delta())
    .collect::<Vec<_>>();
let expected_delta =
    StateDelta::between(&before, &expected_after, expected_audit.clone()).unwrap();
~~~

- [ ] Assert accepted, the complete next_state, complete event vector, exact audit, next_decision == None, EpisodeStatus::Running, revision 0 -> 1, allocator 1 -> 4, distinct digests, and full StateDelta reapplication/digest equality. Explicitly assert unrelated components remain unchanged:

~~~
assert_eq!(result.next_state.core.players[&PlayerId(2)], before.core.players[&PlayerId(2)]);
assert_eq!(result.next_state.zones, before.zones);
assert_eq!(result.next_state.random, before.random);
assert_eq!(result.next_state.knowledge, before.knowledge);
assert_eq!(result.next_state.perspective_identities, before.perspective_identities);
assert_eq!(result.next_state.format, before.format);
assert_eq!(result.next_state.allocators.next_object_id, before.allocators.next_object_id);
assert_eq!(result.next_state.allocators.next_decision_id, before.allocators.next_decision_id);
assert_eq!(result.next_state.allocators.next_continuation_id, before.allocators.next_continuation_id);
~~~

- [ ] Run the red test:

~~~
cargo test -p mtgml-rules --locked synthetic_m1_acceptance_returns_exact_transition_product
~~~

Expected: FAIL because the current kernel returns one DecisionCleared, leaves P1 at 40, and advances the rule-event allocator only to 2.

## Task 3: Add sequential-cursor negative evidence

**File:** crates/mtgml-rules/src/tests.rs

- [ ] Add second_life_event_must_use_cursor_life_after_first_event. Build state() -> after P1 life 38, revision 1, next event ID 3, then events 40 -> 39 and 40 -> 38. Assert:

~~~
assert!(matches!(
    validate_transition_contract(&before, &transition),
    Err(TransitionViolation::LifeChange)
));
~~~

- [ ] Add reversed_dependent_life_events_fail. Build the same after-state with events 39 -> 38 followed by 40 -> 39; assert Err(TransitionViolation::LifeChange) because event 1 fails against the before cursor.

- [ ] Add incomplete_life_trace_fails_final_projection. Build state() -> after P1 life 38, revision 1, next event ID 2, with only LifeChanged { from: 40, to: 39 }; assert Err(TransitionViolation::LifeChange) from final cursor projection mismatch.

- [ ] Add event_and_delta_audit_disagreement_fails. Build a valid two-life-event transition with result(), replace transition.delta.audit[1] with SemanticDeltaOperation::LifeChanged { player: PlayerId(1), from: 39, to: 37 }, and assert Err(TransitionViolation::EventDeltaMismatch).

- [ ] Run the focused contract evidence:

~~~
cargo test -p mtgml-rules --locked life_event
cargo test -p mtgml-rules --locked event_delta
~~~

Expected: these generic cursor tests pass and remain green after the kernel change; Task 2 remains red until Task 4.

## Task 4: Implement the exact three-event workspace

**File:** crates/mtgml-rules/src/synthetic.rs

- [ ] After existing M1.3 binding validation, derive checked IDs for the three events and the next allocator value:

~~~
let first_event_id = state.allocators.next_rule_event_id;
let second_event_id = RuleEventId(first_event_id.0.checked_add(1)
    .ok_or(KernelExecutionError::RuleEventIdOverflow)?);
let third_event_id = RuleEventId(second_event_id.0.checked_add(1)
    .ok_or(KernelExecutionError::RuleEventIdOverflow)?);
let next_event_id = RuleEventId(third_event_id.0.checked_add(1)
    .ok_or(KernelExecutionError::RuleEventIdOverflow)?);
~~~

- [ ] On the cloned workspace, set P1 life to 39, append LifeChanged(P1, 40, 39), set P1 life to 38, append LifeChanged(P1, 39, 38), clear the pending decision, and append DecisionCleared(request.decision_id). Every event uses next_state.revision; the outer revision is incremented once.

- [ ] Set only next_state.allocators.next_rule_event_id = next_event_id. Construct audit from the ordered event vector, create StateDelta::between(state, &next_state, audit), derive next_decision = None and EpisodeStatus::Running, validate the after-state and validate_transition_contract, then return. Do not mutate the input state, call RNG, or consume any allocator other than rule-event IDs.

- [ ] Run the green focused test and the rules crate:

~~~
cargo test -p mtgml-rules --locked synthetic_m1_acceptance_returns_exact_transition_product
cargo test -p mtgml-rules --locked
~~~

Expected: both PASS, including the existing M1.3 rejection matrix and generic composition tests.

## Task 5: Inspect, format, and commit implementation

**Files:** crates/mtgml-rules/src/synthetic.rs, crates/mtgml-rules/src/tests.rs

- [ ] Inspect scope:

~~~
git diff --check
git diff -- crates/mtgml-rules/src/synthetic.rs crates/mtgml-rules/src/tests.rs
~~~

Confirm no RNG draw, non-rule allocator change, decision creation, environment/replay, endpoint, or Magic-specific behavior was added.

- [ ] Format and commit:

~~~
cargo fmt --all
cargo fmt --all -- --check
git add -- crates/mtgml-rules/src/synthetic.rs crates/mtgml-rules/src/tests.rs
git commit -m "feat: compose sequential synthetic rule events"
~~~

## Task 6: Run required local verification

- [ ] Run each locked Rust command separately:

~~~
cargo test -p mtgml-rules --locked
cargo test -p mtgml-state --locked
cargo test --workspace --all-features --locked
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
~~~

- [ ] Run repository checks and profiles:

~~~
python scripts/run_checks.py fast
python scripts/check_documentation.py
python scripts/verify_repository.py
just check-fast
just check
~~~

If either just command is blocked by unavailable WSL2/Hyper-V, record BLOCKED; do not infer hosted success from local success.

- [ ] Read generated verification reports when present. Preserve:

~~~
ENGINE_STATE_CONSTRUCTION_AND_INVARIANTS = PASS only with executed evidence
ACCEPTED_TRANSITION_EXACT_PRODUCT         = PASS only with executed evidence
STATE_DELTA_FULL_REAPPLICATION            = PASS only with executed evidence
SEQUENTIAL_EVENT_DELTA_PARITY             = PASS only with exact executed M1.4 evidence
REJECTED_RESPONSE_COMPLETE_NONMUTATION    = BLOCKED pending #25
DETERMINISTIC_RNG_AND_ALLOCATORS          = NOT_RUN
CHECKPOINT_RESTORE_COMPLETE_IDENTITY      = NOT_RUN
FORK_PARITY                               = NOT_RUN
REPLAY_PARITY                             = NOT_RUN
MULTI_PLAYER_ENDPOINT_BINDING             = NOT_RUN
~~~

Do not hand-edit generated status files.

- [ ] Audit the final local tree:

~~~
git diff --check origin/master...HEAD
git diff --stat origin/master...HEAD
git diff --name-only origin/master...HEAD
git status --short --branch
~~~

Changed files must be limited to the two M1.4 process artifacts, the register, synthetic.rs, and tests.rs.

## Task 7: Push and open one Draft PR

- [ ] Record final head and push:

~~~
git rev-parse HEAD
git push --set-upstream origin chris/m1-4-sequential-event-delta
~~~

- [ ] Create one Draft PR:

~~~
gh pr create --draft --base master --head chris/m1-4-sequential-event-delta
~~~

The body must include the starting SHA, final head SHA, changed files, exact three-event sequence, final state mutations, repeated/mixed/negative evidence, actual local statuses, exact-head hosted statuses if observed, REJECTED_RESPONSE_COMPLETE_NONMUTATION = BLOCKED, all M1.5+ gates NOT_RUN, and explicit confirmation that M1.5+ was not implemented. Add Closes #23 only if the executed evidence supports SEQUENTIAL_EVENT_DELTA_PARITY = PASS. Do not merge.

## Plan self-review

- Exact event order, one outer revision, state mutations, audit order, full delta reapplication, digest parity, and unrelated-state identity are covered by Tasks 2 and 4.
- Repeated dependent events, reversed events, incomplete final projection, audit disagreement, and the exact mixed sequence are covered by Tasks 2 and 3.
- The existing M1.3 rejection matrix is rerun unchanged.
- Existing generic semantic_cursor.rs, contract.rs, events.rs, and StateDelta contracts are reused; no parallel kernel or speculative abstraction is planned.
- RNG, non-rule allocators, decision creation, checkpoint/restore, fork, replay, endpoint binding, and real Magic semantics remain excluded.
