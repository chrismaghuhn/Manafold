# M1.3 Rejection Atomicity Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Prove complete nonmutation for every currently executable M1 synthetic player rejection while preserving the M1.2 accepted transaction and explicitly reporting unavailable environment/replay evidence.

**Architecture:** Keep RulesKernel::apply actor-aware and fallible. Add only the missing M1 ChooseOne ordinal rejection before accepted workspace mutation. Exercise the real mtgml-rules owner with a table-driven matrix and one explicit full-surface rejected-product assertion; do not add an environment transaction owner, replay recorder, endpoint binding, RNG consumption, allocator consumption, or multi-event behavior.

**Tech Stack:** Rust 1.85.1, mtgml-rules, mtgml-state, mtgml-decision, mtgml-environment, typed FullStateDigestV2, StateDelta, locked Cargo tests, repository verification scripts, and GitHub Actions Draft PR checks.

---

## Files and responsibilities

- Create: docs/superpowers/specs/2026-08-20-m1-3-rejection-atomicity-design.md — accepted M1.3 ownership, scope, matrix, and blocker design. Already committed before this plan.
- Create: docs/superpowers/plans/2026-08-20-m1-3-rejection-atomicity.md — this implementation plan.
- Modify: crates/mtgml-rules/src/synthetic.rs — reject an ordinal on the supported M1 ChooseOne path before any accepted workspace mutation.
- Modify: crates/mtgml-rules/src/tests.rs — table-driven normal-rejection and internal-error matrix plus the reusable complete nonmutation assertion.
- Modify: crates/mtgml-environment/src/tests.rs — verify that the existing closed PlayerApiError display surface contains no trusted/internal values; do not add a backend transaction path.
- Do not modify: EnvironmentCheckpointV2, EnvironmentLimitCounters, replay DTOs, replay validation, endpoint binding, RNG services, allocator consumption, semantic event families, or schemas.

## Task 1: Record the plan on the dedicated M1.3 branch

**Files:** docs/superpowers/plans/2026-08-20-m1-3-rejection-atomicity.md

- [ ] **Step 1: Verify the branch and source base**

Run:

~~~powershell
git branch --show-current
git rev-parse HEAD
git rev-parse origin/master
~~~

Expected:

~~~text
chris/m1-3-rejection-atomicity
bb5617e3d784dbf35a9821619cd8212b2b4cc67d
bb5617e3d784dbf35a9821619cd8212b2b4cc67d
~~~

- [ ] **Step 2: Commit the plan**

Run:

~~~powershell
git add -- docs/superpowers/plans/2026-08-20-m1-3-rejection-atomicity.md
git commit -m "docs: add M1.3 rejection atomicity plan"
~~~

## Task 2: Add the failing ordinal regression test

**Files:** crates/mtgml-rules/src/tests.rs

- [ ] **Step 1: Add a test that proves an unsupported ordinal is a normal rejection**

Add this test next to the existing synthetic rejection tests. It must use the real kernel and the existing exact-product helper:

~~~rust
#[test]
fn synthetic_kernel_rejects_unsupported_choose_one_ordinal() {
    let before = synthetic_state();
    let mut response = synthetic_response(&before);
    response.assignments[0].ordinal = Some(0);
    let mut kernel = SyntheticM1RulesKernel;

    let result = kernel.apply(&before, PlayerId(1), &response).unwrap();

    assert_exact_rejected_product(&before, result);
}
~~~

- [ ] **Step 2: Run the focused test and observe the expected failure**

Run:

~~~powershell
cargo test -p mtgml-rules --locked synthetic_kernel_rejects_unsupported_choose_one_ordinal
~~~

Expected: the test fails because the current production path validates the assignment candidate but does not reject ordinal Some(0), so it returns an accepted transition instead of accepted == false.

## Task 3: Implement the minimal ordinal guard

**Files:** crates/mtgml-rules/src/synthetic.rs, crates/mtgml-rules/src/tests.rs

- [ ] **Step 1: Add the guard before candidate binding and accepted workspace creation**

Immediately after selecting assignment and before accepting the candidate, add:

~~~rust
if assignment.ordinal.is_some() {
    return rejected(state);
}
~~~

Do not add an error variant, response field, rollback path, or generic validation framework. The existing rejected(state) helper remains the only normal player-rejection product.

- [ ] **Step 2: Run the focused test and the existing synthetic rules tests**

Run:

~~~powershell
cargo test -p mtgml-rules --locked synthetic_kernel_rejects_unsupported_choose_one_ordinal
cargo test -p mtgml-rules --locked synthetic_
~~~

Expected: both commands exit successfully; the first test proves the minimal production correction and the existing M1.2 accepted/rejected tests remain green.

## Task 4: Replace scattered negative evidence with one complete matrix

**Files:** crates/mtgml-rules/src/tests.rs

- [ ] **Step 1: Expand the reusable exact rejected-product assertion**

assert_exact_rejected_product must explicitly compare every currently-owned authoritative surface before calling validate_transition_contract. Keep the existing complete EngineState comparison, and add named assertions with this shape:

~~~rust
let before_digest = before.digest().unwrap();
let expected_next_decision = before
    .execution
    .pending_decision
    .as_ref()
    .map(|record| record.request.clone());

assert!(!result.accepted);
assert_eq!(result.next_state, *before);
assert_eq!(before.digest().unwrap(), result.next_state.digest().unwrap());
assert_eq!(result.next_state.revision, before.revision);
assert_eq!(
    result.next_state.execution.pending_decision,
    before.execution.pending_decision
);
assert_eq!(
    result.next_state.execution.continuations,
    before.execution.continuations
);
assert_eq!(result.next_state.random.root_seed, before.random.root_seed);
assert_eq!(result.next_state.random.streams, before.random.streams);
assert_eq!(result.next_state.allocators, before.allocators);
assert_eq!(result.next_state.zones, before.zones);
assert_eq!(result.next_state.core, before.core);
assert_eq!(result.next_state.knowledge, before.knowledge);
assert_eq!(
    result.next_state.perspective_identities,
    before.perspective_identities
);
assert_eq!(result.next_state.format, before.format);
assert!(result.events.is_empty());
assert!(result.delta.audit.is_empty());
assert_eq!(result.delta.replacement, before.parts());
assert_eq!(result.delta.before_revision, result.delta.after_revision);
assert_eq!(result.delta.before_revision, before.revision);
assert_eq!(result.delta.before_digest, before_digest);
assert_eq!(result.delta.after_digest, before_digest);
assert_eq!(result.next_decision, expected_next_decision);
assert_eq!(result.status, EpisodeStatus::Running);
assert_eq!(result.delta.apply(before).unwrap(), *before);
assert_eq!(before.digest().unwrap(), result.next_state.digest().unwrap());
validate_transition_contract(before, &result).unwrap();
~~~

The equality assertions are intentionally explicit even though EngineState equality and the V2 digest also exist. They prove the named RNG, allocator, knowledge/history, identity, continuation, core, zone, stack, and format surfaces rather than inferring them from one digest.

- [ ] **Step 2: Add table-driven case metadata and response/state mutators**

Define test-only function-pointer mutators and a case descriptor so every case records its name, before-state mutation, response mutation, trusted actor, and classification:

~~~rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum RejectionClassification {
    PlayerSubmission,
    TrustedBeforeState(EngineStateViolation),
}

#[derive(Clone)]
struct RejectionCase {
    name: &'static str,
    mutate_state: fn(&mut EngineState),
    mutate_response: fn(&EngineState, &mut DecisionResponse),
    trusted_actor: PlayerId,
    classification: RejectionClassification,
}
~~~

Use no-op mutators for unchanged state/response and define these exact normal cases:

~~~text
wrong trusted actor
stale state_revision
wrong decision_id
wrong schema_version
empty candidate ID
duplicate assignment
zero assignments
more than one assignment for ChooseOne
unknown candidate ID
unsupported ordinal
otherwise-valid Confirm candidate/binding
more than one valid candidate
pending decision with continuation
otherwise-valid ChooseMany decision
no pending decision
~~~

The Confirm, multiple-candidate, continuation, ChooseMany, and no-pending mutators must keep the resulting before-state valid whenever the case is a player submission. Build a response from the mutated pending request when one exists, and use a fixed schema-valid response for the no-pending case. Apply the response mutator after that base response is built.

- [ ] **Step 3: Add the internal cases to the same descriptor shape**

Add these trusted-state cases with exact expected errors:

~~~text
invalid next_rule_event_id zero -> TrustedBeforeState(AllocatorBehind)
visible/authoritative SelectObject binding mismatch -> TrustedBeforeState(PendingDecisionMismatch)
~~~

For the binding mismatch, mutate only the trusted pending.candidate_bindings["select_public_object"] to EngineCandidateBinding::SelectObject { object: GameObjectId(2) }. Do not add authoritative IDs or bindings to DecisionResponse; the response still carries only the visible candidate ID. validate_engine_state must reject the authoritative before-state before player-response validation.

- [ ] **Step 4: Execute every case through one table-driven test**

For each case:

1. construct synthetic_state();
2. apply mutate_state;
3. construct a valid response from the current pending request when possible;
4. apply mutate_response;
5. call SyntheticM1RulesKernel::apply with trusted_actor;
6. match the expected classification.

Player-submission cases must receive Ok(result) and call assert_exact_rejected_product. Trusted-state cases must receive Err(KernelExecutionError::BeforeState(expected)) and must not be accepted as player rejections. Include the case name in every assertion message.

- [ ] **Step 5: Run the matrix and complete rules crate**

Run:

~~~powershell
cargo test -p mtgml-rules --locked rejection
cargo test -p mtgml-rules --locked
~~~

Expected: the matrix and all existing event/delta/M1.2 tests pass. No new multi-event behavior is added; existing compositional validation tests are only retained as pre-existing M0.2/M1.2 coverage and do not change the M1.3 scope.

## Task 5: Prove the existing player-safe error surface without inventing an endpoint

**Files:** crates/mtgml-environment/src/tests.rs

- [ ] **Step 1: Add a closed-surface sanitization test**

Add a test over every existing PlayerApiError variant:

~~~rust
#[test]
fn player_api_errors_do_not_render_trusted_or_hidden_values() {
    let errors = [
        PlayerApiError::NoVisibleDecision,
        PlayerApiError::StaleResponse,
        PlayerApiError::InvalidSelection,
        PlayerApiError::EpisodeComplete,
        PlayerApiError::Unavailable,
    ];
    let forbidden = [
        "KernelExecutionError",
        "before state",
        "GameObjectId",
        "DecisionId",
        "OpaqueObjectId",
        "binding",
        "root seed",
        "next_raw_u64",
        "allocator",
        "knowledge",
    ];

    for error in errors {
        let rendered = error.to_string();
        for value in forbidden {
            assert!(
                !rendered.contains(value),
                "{error:?} exposed forbidden internal text {value:?}: {rendered:?}"
            );
        }
    }
}
~~~

This tests only the actual closed public enum. Do not add mapping logic to EnvironmentBackend, a synthetic controller, replay recording, or a new player error variant.

- [ ] **Step 2: Run the focused environment tests**

Run:

~~~powershell
cargo test -p mtgml-environment --locked player_api_errors_do_not_render_trusted_or_hidden_values
cargo test -p mtgml-environment --locked
~~~

Expected: both commands pass. The absence of an executable backend submission owner remains a documented BLOCKED evidence surface, not a hidden test gap.

## Task 6: Repository verification and exact status accounting

**Files:** no additional source files; verification output remains outside the reproducible source archive under the repository-defined dist/verification/ location.

- [ ] **Step 1: Run all required locked Rust checks separately**

Run:

~~~powershell
cargo test -p mtgml-rules --locked
cargo test -p mtgml-state --locked
cargo test -p mtgml-environment --locked
cargo test --workspace --all-features --locked
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
~~~

Record each command from its actual exit code. An unavailable tool is NOT_RUN; a blocked command is BLOCKED; no status is inferred from another command.

- [ ] **Step 2: Run repository checks for changed artifacts**

Run:

~~~powershell
python scripts/run_checks.py fast
python scripts/check_documentation.py
python scripts/verify_repository.py
~~~

If the repository uses a different documented invocation for one check, run that exact invocation and record it; do not hand-edit generated status.

- [ ] **Step 3: Run the requested local profiles and inspect evidence**

Run:

~~~powershell
just check-fast
just check
~~~

Read the generated reports. Preserve any WSL2/Hyper-V or tool-availability blockers exactly as reported. Do not claim hosted workflow success is local command success.

- [ ] **Step 4: Audit the exact diff and M1 boundary**

Run:

~~~powershell
git diff --check origin/master...HEAD
git diff --stat origin/master...HEAD
git diff --name-only origin/master...HEAD
git status --short --branch
~~~

Expected changed files are limited to the M1.3 design/plan, the synthetic ordinal guard and rules tests, and the player-error sanitization test. Confirm there are no changes to environment checkpoint ownership, replay execution, RNG/allocator consumption, endpoint binding, multiple semantic events, or real Magic behavior.

- [ ] **Step 5: Record gate statuses without overstating evidence**

Use these final statuses for the implemented evidence:

~~~text
ENGINE_STATE_CONSTRUCTION_AND_INVARIANTS = PASS only if its required command/evidence is rerun successfully
ACCEPTED_TRANSITION_EXACT_PRODUCT         = PASS only if its required command/evidence is rerun successfully
STATE_DELTA_FULL_REAPPLICATION            = PASS only if its required command/evidence is rerun successfully
REJECTED_RESPONSE_COMPLETE_NONMUTATION    = BLOCKED while the executable environment/replay owners are absent
SEQUENTIAL_EVENT_DELTA_PARITY             = NOT_RUN
DETERMINISTIC_RNG_AND_ALLOCATORS          = NOT_RUN
CHECKPOINT_RESTORE_COMPLETE_IDENTITY      = NOT_RUN
FORK_PARITY                               = NOT_RUN
REPLAY_PARITY                             = NOT_RUN
MULTI_PLAYER_ENDPOINT_BINDING             = NOT_RUN
~~~

The M1.3 report must separately list authoritative state, V2 digest, revision, pending decision/bindings, RNG, allocators, knowledge/history, opaque identities, rule events, delta, episode status, environment counters, accepted replay state/count, and player-safe errors as PASS, BLOCKED, or NOT_RUN based only on executed evidence.

- [ ] **Step 6: Create the one Draft PR only after verification**

Push the dedicated branch and create one Draft PR against master. The body must include starting SHA bb5617e3d784dbf35a9821619cd8212b2b4cc67d, final head SHA, changed files, exact matrix, internal/player classification, ownership/blocker evidence, exact commands and statuses, and explicit M1.4+ non-implementation. Do not add Closes #22 while the overall rejection gate is BLOCKED; do not merge.

~~~powershell
git push --set-upstream origin chris/m1-3-rejection-atomicity
gh pr create --draft --base master --head chris/m1-3-rejection-atomicity
~~~

## Plan self-review

- The design requirement for complete authoritative nonmutation is covered by Task 4 explicit helper and matrix.
- The unsupported ordinal is covered by a failing test before the production guard in Tasks 2–3.
- The binding mismatch is covered as BeforeState, not as player input, in Task 4.
- The existing closed player error surface is tested without adding endpoint ownership in Task 5.
- Environment checkpoint/limit counters and replay state are explicitly identified as missing executable owners and cannot become PASS by test fabrication.
- M1.4+ work, RNG draws, allocator consumption, checkpoint/restore/fork/replay execution, endpoint binding, and Magic semantics are explicitly excluded.
- No plan step contains a placeholder or asks an implementer to infer an unspecified validation rule.
