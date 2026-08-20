# M1.2 Accepted Transaction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Implement one exact accepted synthetic ChooseOne response transition with explicit trusted actor validation, one DecisionCleared event, complete StateDelta reapplication, and fail-closed rejection preparation.

**Architecture:** Correct the existing RulesKernel boundary to receive trusted_actor: PlayerId and return Result<TransitionResult, KernelExecutionError>. Add a purpose-specific SyntheticM1RulesKernel that validates the existing M1.1 request before cloning a local workspace, clears only the pending decision, validates the complete product, and returns rejected products for normal player-input failures. Keep the environment endpoint and later M1.3+ machinery unchanged.

**Tech Stack:** Rust 1.85.1, mtgml-rules, mtgml-state, mtgml-decision, typed FullStateDigestV2, StateDelta, AuthoritativeRuleEvent, Cargo locked workspace checks, repository just profiles, and GitHub Actions PR/integration/nightly workflows.

---

## Files and responsibilities

- Create: crates/mtgml-rules/src/errors.rs — trusted/internal kernel execution errors.
- Create: crates/mtgml-rules/src/synthetic.rs — the single accepted M1.2 kernel path and exact rejected-product helper.
- Modify: crates/mtgml-rules/src/transition.rs — explicit trusted actor and fallible kernel boundary; keep TransitionResult.accepted.
- Modify: crates/mtgml-rules/src/lib.rs — register and re-export the new types.
- Modify: crates/mtgml-rules/src/tests.rs — exact accepted product and minimal rejection/internal-failure evidence.
- Already committed: docs/superpowers/specs/2026-08-20-m1-2-accepted-transaction-design.md.
- Create and commit before implementation: docs/superpowers/plans/2026-08-20-m1-2-accepted-transaction.md.

No generated vocabulary, schema/digest version, environment endpoint, Python rules code, or M1.3+ implementation is changed.

### Task 1: Freeze the public kernel boundary

**Files:** crates/mtgml-rules/src/transition.rs, crates/mtgml-rules/src/errors.rs, crates/mtgml-rules/src/lib.rs, crates/mtgml-rules/src/tests.rs

- [ ] Step 1: Add a compile-use test before implementation.

Add this test to crates/mtgml-rules/src/tests.rs:

    #[test]
    fn synthetic_kernel_boundary_carries_trusted_actor() {
        let state = synthetic_state();
        let response = synthetic_response(&state);
        let mut kernel = SyntheticM1RulesKernel::default();
        let result = kernel.apply(&state, PlayerId(1), &response);
        assert!(result.is_ok());
    }

- [ ] Step 2: Run the failing boundary test.

    cargo test -p mtgml-rules --locked synthetic_kernel_boundary_carries_trusted_actor

Expected: compile failure because the synthetic kernel and actor-aware fallible boundary do not exist.

- [ ] Step 3: Create crates/mtgml-rules/src/errors.rs.

Use exactly these trusted/internal categories:

    use mtgml_state::{EngineStateViolation, StateDigestError};
    use thiserror::Error;

    use crate::TransitionViolation;

    #[derive(Debug, Error)]
    pub enum KernelExecutionError {
        #[error("before state is invalid: {0}")]
        BeforeState(EngineStateViolation),
        #[error("revision would overflow")]
        RevisionOverflow,
        #[error("rule event identity would overflow")]
        RuleEventIdOverflow,
        #[error("state delta construction failed: {0}")]
        Delta(StateDigestError),
        #[error("after state is invalid: {0}")]
        AfterState(EngineStateViolation),
        #[error("transition contract failed: {0}")]
        TransitionContract(TransitionViolation),
    }

Normal response validation failures must never use this error; they return an exact rejected TransitionResult.

- [ ] Step 4: Correct the RulesKernel signature.

In crates/mtgml-rules/src/transition.rs, import PlayerId and KernelExecutionError and use:

    pub trait RulesKernel: Send {
        fn apply(
            &mut self,
            state: &EngineState,
            trusted_actor: PlayerId,
            response: &mtgml_decision::DecisionResponse,
        ) -> Result<TransitionResult, KernelExecutionError>;
    }

Keep TransitionResult.accepted and all other TransitionResult fields unchanged. In lib.rs add mod errors and re-export KernelExecutionError.

- [ ] Step 5: Re-run the boundary test.

    cargo test -p mtgml-rules --locked synthetic_kernel_boundary_carries_trusted_actor

Expected: the public error/signature compiles; the remaining failure is the missing SyntheticM1RulesKernel implementation.

- [ ] Step 6: Commit only the boundary files.

    git add -- crates/mtgml-rules/src/errors.rs crates/mtgml-rules/src/transition.rs crates/mtgml-rules/src/lib.rs crates/mtgml-rules/src/tests.rs
    git commit -m "feat: carry trusted actor through rules kernel boundary"

### Task 2: Add exact M1.2 scenario evidence before implementation

**Files:** crates/mtgml-rules/src/tests.rs

- [ ] Step 1: Add M1.1 fixture helpers.

Use the existing constructor and no manually authored digest:

    fn synthetic_state() -> EngineState {
        construct_synthetic_engine_state(SyntheticResetInputs {
            players: [PlayerId(1), PlayerId(2)],
            root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        }).unwrap()
    }

    fn synthetic_response(state: &EngineState) -> DecisionResponse {
        let request = &state.execution.pending_decision.as_ref().unwrap().request;
        DecisionResponse {
            schema_version: DECISION_RESPONSE_SCHEMA.to_owned(),
            decision_id: request.decision_id,
            state_revision: request.state_revision,
            assignments: vec![CandidateAssignment {
                candidate_id: "select_public_object".to_owned(),
                ordinal: None,
            }],
        }
    }

- [ ] Step 2: Add the exact accepted-product test.

The test must assert the complete pending request and authoritative binding, then derive expected_after by cloning before and changing only revision, pending_decision, and next_rule_event_id:

    #[test]
    fn synthetic_m1_acceptance_returns_exact_transition_product() {
        let before = synthetic_state();
        let response = synthetic_response(&before);
        let before_digest = before.digest().unwrap();
        let pending = before.execution.pending_decision.as_ref().unwrap();

        assert_eq!(pending.request.actor, PlayerId(1));
        assert_eq!(pending.request.decision, DecisionKind::ChooseOne);
        assert_eq!(pending.request.candidates.len(), 1);
        assert_eq!(pending.request.candidates[0].candidate_id, "select_public_object");
        assert_eq!(
            pending.candidate_bindings["select_public_object"],
            EngineCandidateBinding::SelectObject { object: GameObjectId(1) }
        );

        let mut expected_after = before.clone();
        expected_after.revision = StateRevision(1);
        expected_after.execution.pending_decision = None;
        expected_after.allocators.next_rule_event_id = RuleEventId(2);

        let expected_event = AuthoritativeRuleEvent {
            event_id: RuleEventId(1),
            state_revision: StateRevision(1),
            event: AuthoritativeRuleEventKind::DecisionCleared {
                decision: DecisionId(1),
            },
        };
        let expected_audit = vec![SemanticDeltaOperation::DecisionCleared {
            decision: DecisionId(1),
        }];
        let expected_delta =
            StateDelta::between(&before, &expected_after, expected_audit.clone()).unwrap();

        let mut kernel = SyntheticM1RulesKernel::default();
        let result = kernel.apply(&before, PlayerId(1), &response).unwrap();

        assert!(result.accepted);
        assert_eq!(result.next_state, expected_after);
        assert_eq!(result.events, vec![expected_event]);
        assert_eq!(result.delta, expected_delta);
        assert_eq!(result.delta.audit, expected_audit);
        assert_eq!(result.next_decision, None);
        assert_eq!(result.status, EpisodeStatus::Running);
        assert_eq!(result.delta.before_revision, StateRevision(0));
        assert_eq!(result.delta.after_revision, StateRevision(1));
        assert_eq!(result.delta.before_digest, before_digest);
        assert_eq!(result.delta.after_digest, result.next_state.digest().unwrap());
        assert_ne!(before_digest, result.next_state.digest().unwrap());

        let reapplied = result.delta.apply(&before).unwrap();
        assert_eq!(reapplied, result.next_state);
        assert_eq!(reapplied.digest().unwrap(), result.next_state.digest().unwrap());
        validate_transition_contract(&before, &result).unwrap();
    }

Required imports are the existing types CandidateAssignment, DecisionKind, DecisionResponse, DECISION_RESPONSE_SCHEMA, EngineCandidateBinding, AuthoritativeRuleEvent, AuthoritativeRuleEventKind, EpisodeStatus, GameObjectId, RuleEventId, StateRevision, DecisionId, SemanticDeltaOperation, StateDelta, SyntheticResetInputs, construct_synthetic_engine_state, and RootSeed256.

- [ ] Step 3: Run the test before implementation.

    cargo test -p mtgml-rules --locked synthetic_m1_acceptance_returns_exact_transition_product

Expected: compile failure because SyntheticM1RulesKernel is still absent.

### Task 3: Implement the one-event synthetic kernel

**Files:** crates/mtgml-rules/src/synthetic.rs, crates/mtgml-rules/src/lib.rs

- [ ] Step 1: Add the purpose-specific kernel type.

Create:

    #[derive(Debug, Default)]
    pub struct SyntheticM1RulesKernel;

Register mod synthetic and re-export SyntheticM1RulesKernel in lib.rs.

- [ ] Step 2: Implement pre-workspace validation.

The RulesKernel::apply implementation must first validate the complete before state. Map failure to KernelExecutionError::BeforeState. Then, before cloning an accepted workspace, return rejected(state) when any of these normal submission checks fails: no pending decision, trusted actor differs from request.actor, response.validate() fails, decision ID differs, response revision differs from state.revision, decision is not ChooseOne, assignment count is not exactly one, candidate ID is not in the request, or validate_candidate_binding fails.

Use this exact validation shape:

    validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
    let Some(pending) = state.execution.pending_decision.as_ref() else {
        return rejected(state);
    };
    let request = &pending.request;
    if trusted_actor != request.actor
        || response.validate().is_err()
        || response.decision_id != request.decision_id
        || response.state_revision != state.revision
        || !matches!(request.decision, DecisionKind::ChooseOne)
        || response.assignments.len() != 1
    {
        return rejected(state);
    }

    let assignment = &response.assignments[0];
    let Some(candidate) = request.candidates.iter().find(
        |candidate| candidate.candidate_id == assignment.candidate_id
    ) else {
        return rejected(state);
    };
    let binding = pending.candidate_bindings.get(&assignment.candidate_id).ok_or(
        KernelExecutionError::BeforeState(EngineStateViolation::PendingDecisionMismatch)
    )?;
    if validate_candidate_binding(
        candidate,
        binding,
        trusted_actor,
        &state.perspective_identities,
    ).is_err() {
        return rejected(state);
    }

The missing binding after a successful before-state validation is an internal invariant failure, not a player rejection.

- [ ] Step 3: Implement the exact rejected product helper.

rejected(state) returns Result<TransitionResult, KernelExecutionError> with accepted false, next_state equal to state.clone(), no events, empty audit, current pending request as next_decision, and EpisodeStatus::Running. Build the zero delta with StateDelta::between(state, state, vec![]). Run validate_transition_contract(state, &result). Map delta or contract failures to KernelExecutionError. Do not add a rejection reason field or a player-facing error mapping.

- [ ] Step 4: Implement the accepted workspace.

After all normal validation succeeds, use checked arithmetic and mutate only the local clone:

    let mut next_state = state.clone();
    let next_revision = state.revision.0.checked_add(1)
        .ok_or(KernelExecutionError::RevisionOverflow)?;
    let event_id = state.allocators.next_rule_event_id;
    let next_event_id = event_id.0.checked_add(1)
        .ok_or(KernelExecutionError::RuleEventIdOverflow)?;

    next_state.revision = StateRevision(next_revision);
    next_state.execution.pending_decision = None;
    next_state.allocators.next_rule_event_id = RuleEventId(next_event_id);

    let events = vec![AuthoritativeRuleEvent {
        event_id,
        state_revision: next_state.revision,
        event: AuthoritativeRuleEventKind::DecisionCleared {
            decision: request.decision_id,
        },
    }];
    let audit = events.iter()
        .map(|event| event.event.semantic_delta())
        .collect::<Vec<_>>();
    let delta = StateDelta::between(state, &next_state, audit)
        .map_err(KernelExecutionError::Delta)?;

    let result = TransitionResult {
        accepted: true,
        next_state,
        delta,
        events,
        next_decision: None,
        status: EpisodeStatus::Running,
    };

Do not touch RNG, zones, core life/turn state, knowledge, perspective identities, or any allocator other than next_rule_event_id.

- [ ] Step 5: Validate the result before returning it.

Run validate_engine_state on result.next_state and map to AfterState. Then run validate_transition_contract(state, &result) and map to TransitionContract. Return Ok(result) only after both succeed.

- [ ] Step 6: Run focused tests and commit.

    cargo test -p mtgml-rules --locked synthetic_

Expected: all synthetic M1.2 tests PASS.

    git add -- crates/mtgml-rules/src/synthetic.rs crates/mtgml-rules/src/lib.rs crates/mtgml-rules/src/tests.rs
    git commit -m "feat: implement M1.2 accepted synthetic transaction"

### Task 4: Prove the rejection/internal-error distinction

**Files:** crates/mtgml-rules/src/tests.rs

- [ ] Step 1: Add wrong-actor rejected-product evidence.

Call kernel.apply(&before, PlayerId(2), &response) and assert: accepted false; next_state equals before; events and delta.audit are empty; before/after revisions and digests equal before; next_decision is the cloned current pending request; status is Running; delta.apply(before) equals before; validate_transition_contract succeeds.

- [ ] Step 2: Add an internal-before-state error test.

Clone synthetic_state, set invalid.allocators.next_rule_event_id = RuleEventId(0), submit the valid response, and assert:

    matches!(
        kernel.apply(&invalid, PlayerId(1), &response),
        Err(KernelExecutionError::BeforeState(
            EngineStateViolation::AllocatorBehind
        ))
    )

This proves malformed trusted state is an internal failure, not a player rejection.

- [ ] Step 3: Run and commit the focused rules suite.

    cargo test -p mtgml-rules --locked

Expected: all existing compositional event tests and all new M1.2 tests PASS.

    git add -- crates/mtgml-rules/src/tests.rs
    git commit -m "test: prove M1.2 rejection and kernel failure split"

### Task 5: Verify, push, and open the Draft PR

**Files:** no additional files are expected.

- [ ] Step 1: Run fast repository checks.

    just check-fast

Read the generated result and record each gate explicitly. Do not edit unrelated Python or generated vocabulary to clear unrelated failures.

- [ ] Step 2: Run every required locked command separately.

    cargo test -p mtgml-rules --locked
    cargo test -p mtgml-state --locked
    cargo test --workspace --all-features --locked
    cargo fmt --all -- --check
    cargo check --workspace --all-targets --all-features --locked
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

Every unexecuted or unavailable command is NOT_RUN, never PASS.

- [ ] Step 3: Run the integration profile.

    just check

Inspect generated verification output and preserve all FAIL, BLOCKED, and NOT_RUN statuses. Keep M1.3+ gates NOT_RUN.

- [ ] Step 4: Audit the exact diff.

    git diff --check origin/master...HEAD
    git diff --stat origin/master...HEAD
    git status --short --branch

Expected: only approved M1.2 code, tests, design, and plan documentation.

- [ ] Step 5: Push the dedicated branch.

    git push --set-upstream origin chris/m1-2-accepted-transaction
    git rev-parse HEAD

- [ ] Step 6: Create exactly one Draft PR.

Use gh pr create --draft --base master --head chris/m1-2-accepted-transaction. The body must include starting SHA 796dbc00a9ead81ede9c0e76f08446db8da85882, final SHA, changed files, exact scenario, trusted actor API change, exact local and hosted verification statuses, M1.2 gate statuses, all M1.3+ gates as NOT_RUN, and Closes #21 only if both M1.2 exit gates are actually PASS. Do not merge.

## Self-review checklist

- [ ] Normal response failures return Ok(accepted: false), never Err.
- [ ] Err(KernelExecutionError) is limited to trusted/internal failures.
- [ ] Accepted workspace creation occurs only after actor, response, decision, revision, cardinality, candidate, and binding validation.
- [ ] Exactly one DecisionCleared event and one matching audit entry are emitted.
- [ ] Only revision, pending decision, and next_rule_event_id change.
- [ ] Delta reapplication reconstructs the exact after-state and digest.
- [ ] No M1.3 rejection matrix, M1.4 multi-event behavior, M1.5 RNG/allocator consumption, PlayerEndpoint work, or real Magic behavior was added.
- [ ] No gate is reported PASS without executed evidence.

