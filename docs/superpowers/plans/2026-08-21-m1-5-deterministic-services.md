# M1.5 Deterministic Services Implementation Plan

**Status:** accepted

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the accepted M1 synthetic transition with one exact draw from
the typed `SyntheticM1/Global` stream and one checked `EffectInstanceId`
allocation, with causal event validation, exact delta/digest parity, complete
rejection nonmutation, and checkpoint-capture evidence.

**Architecture:** Keep `EngineState` authoritative and the existing
`SyntheticM1RulesKernel` as the only synthetic executor. Add one typed
effect-ID allocator operation in `mtgml-state`, one trusted
`RandomValueSampled` event and matching semantic delta operation, and extend
the existing sequential semantic cursor to validate the RNG event by rerunning
the authoritative `mtgml-random` sampler. Keep `mtgml-rules` independent of
`mtgml-environment`; use the existing `EnvironmentCheckpointV2` only from
environment-owned tests.

**Tech Stack:** Rust 1.85.1, `mtgml-rng.v1`, `mtgml-state`,
`mtgml-rules`, `mtgml-environment`, serde, locked Cargo tests, Python
repository checks, and `just` verification profiles.

---

**Starting master:** `da563135cebfc17efff6f2b6692950a0360f23ed`

**Branch:** `chris/m1-5-deterministic-services`

## Files and responsibilities

- Modify `crates/mtgml-state/src/identity.rs`: add the narrow checked
  `EffectInstanceId` allocator operation and its typed overflow error.
- Modify `crates/mtgml-state/src/lib.rs`: re-export the allocator error.
- Modify `crates/mtgml-state/src/tests.rs`: prove normal allocation and
  `u64::MAX` zero-mutation behavior.
- Modify `crates/mtgml-state/src/delta.rs`: add
  `SemanticDeltaOperation::RandomValueSampled`.
- Modify `crates/mtgml-rules/src/events.rs`: add the matching authoritative
  `RandomValueSampled` event and exact semantic-delta conversion.
- Modify `crates/mtgml-rules/src/semantic_cursor.rs`: retain the before-state
  root seed and validate exact RNG event causality with
  `uniform_below_u64`.
- Modify `crates/mtgml-rules/src/errors.rs`: wrap typed RNG and allocator
  service failures as trusted execution errors.
- Modify `crates/mtgml-rules/src/synthetic.rs`: execute the local M1.5
  service order and emit the four-event accepted product.
- Modify `crates/mtgml-rules/src/tests.rs`: add all accepted, negative,
  repeatability, isolation, rejection, and exhaustion evidence.
- Modify `crates/mtgml-environment/src/tests.rs`: prove that
  `EnvironmentCheckpointV2` preserves advanced RNG and allocator state.
- Modify `docs/normative-document-register.v1.json`: register the M1.5
  process artifacts.
- Create `docs/superpowers/specs/2026-08-21-m1-5-deterministic-services-design.md`:
  accepted design and boundary.
- Create `docs/superpowers/plans/2026-08-21-m1-5-deterministic-services.md`:
  this executable plan.

Do not modify RNG contract semantics, generated vocabulary, public player
schemas, observations, replay runtime, environment transaction ownership, or
M1 status artifacts.

### Task 1: Commit the reconciled M1.5 process artifacts

**Files:**

- `docs/superpowers/specs/2026-08-21-m1-5-deterministic-services-design.md`
- `docs/superpowers/plans/2026-08-21-m1-5-deterministic-services.md`
- `docs/normative-document-register.v1.json`

- [ ] **Step 1: Verify branch and source identity**

Run:

~~~
git branch --show-current
git rev-parse HEAD
git rev-parse origin/master
~~~

Expected:

~~~
chris/m1-5-deterministic-services
da563135cebfc17efff6f2b6692950a0360f23ed
da563135cebfc17efff6f2b6692950a0360f23ed
~~~

- [ ] **Step 2: Self-review the process artifacts**

Run:

~~~
python -m json.tool docs/normative-document-register.v1.json
git diff --check
~~~

Expected: JSON parsing and `git diff --check` exit with code 0; manual review
confirms that the process artifacts contain no placeholder instructions.

- [ ] **Step 3: Run documentation validation**

Run:

~~~
python scripts/check_documentation.py
~~~

Expected: the registered M1.5 design and plan exist, all registered document
headers are valid, and the command exits 0.

- [ ] **Step 4: Commit only process artifacts**

~~~
git add -- docs/superpowers/specs/2026-08-21-m1-5-deterministic-services-design.md docs/superpowers/plans/2026-08-21-m1-5-deterministic-services.md docs/normative-document-register.v1.json
git commit -m "docs: add M1.5 deterministic services plan"
~~~

### Task 2: Add the typed effect-ID allocator with tests first

**Files:**

- Test: `crates/mtgml-state/src/tests.rs`
- Modify: `crates/mtgml-state/src/identity.rs`
- Modify: `crates/mtgml-state/src/lib.rs`

- [ ] **Step 1: Write the failing allocator tests**

Add to the state test module:

~~~
#[test]
fn effect_allocator_returns_current_id_and_checked_advances() {
    let mut allocators = IdentityAllocatorState::default();
    let allocated = allocators.allocate_effect_id().unwrap();

    assert_eq!(allocated, EffectInstanceId(1));
    assert_eq!(allocators.next_effect_id, EffectInstanceId(2));
}

#[test]
fn effect_allocator_exhaustion_does_not_mutate_allocator() {
    let mut allocators = IdentityAllocatorState::default();
    allocators.next_effect_id = EffectInstanceId(u64::MAX);
    let before = allocators.clone();

    assert_eq!(
        allocators.allocate_effect_id(),
        Err(IdentityAllocationError::EffectInstanceIdExhausted)
    );
    assert_eq!(allocators, before);
}
~~~

- [ ] **Step 2: Run the focused tests and confirm the expected red failure**

~~~
cargo test -p mtgml-state --locked effect_allocator
~~~

Expected: compilation fails because `allocate_effect_id` and
`IdentityAllocationError` do not exist yet.

- [ ] **Step 3: Implement the narrow typed allocator**

In `crates/mtgml-state/src/identity.rs`, add:

~~~
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IdentityAllocationError {
    #[error("effect instance identity is exhausted")]
    EffectInstanceIdExhausted,
}

impl IdentityAllocatorState {
    pub fn allocate_effect_id(
        &mut self,
    ) -> Result<mtgml_model::EffectInstanceId, IdentityAllocationError> {
        let allocated = self.next_effect_id;
        if allocated.0 == u64::MAX {
            return Err(IdentityAllocationError::EffectInstanceIdExhausted);
        }
        self.next_effect_id = mtgml_model::EffectInstanceId(
            allocated
                .0
                .checked_add(1)
                .ok_or(IdentityAllocationError::EffectInstanceIdExhausted)?,
        );
        Ok(allocated)
    }
}
~~~

In `crates/mtgml-state/src/lib.rs`, re-export
`IdentityAllocationError` with `IdentityAllocatorState`.

- [ ] **Step 4: Run the focused tests and confirm green**

~~~
cargo test -p mtgml-state --locked effect_allocator
~~~

Expected: both allocator tests pass.

- [ ] **Step 5: Commit the typed allocator**

~~~
git add -- crates/mtgml-state/src/identity.rs crates/mtgml-state/src/lib.rs crates/mtgml-state/src/tests.rs
git commit -m "feat: add checked effect identity allocator"
~~~

### Task 3: Add the RNG event/delta contract and cursor validation test-first

**Files:**

- Test: `crates/mtgml-rules/src/tests.rs`
- Modify: `crates/mtgml-state/src/delta.rs`
- Modify: `crates/mtgml-rules/src/events.rs`
- Modify: `crates/mtgml-rules/src/semantic_cursor.rs`
- Modify: `crates/mtgml-rules/src/validation.rs` only if a new precise
  transition violation is required; reuse `TransitionViolation::Randomness`
  when it remains sufficiently diagnostic.

- [ ] **Step 1: Add a failing causal RNG-event contract test**

Use the existing `state()` and `result()` helpers in the rules test module.
Build the canonical before/after pair by adding the existing
`SyntheticM1/Global` stream to `state()`, setting its after cursor to 1,
and making the only event:

~~~
AuthoritativeRuleEvent {
    event_id: RuleEventId(1),
    state_revision: StateRevision(1),
    event: AuthoritativeRuleEventKind::RandomValueSampled {
        stream: RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
        bound: 10,
        value: 1,
        raw_words_consumed: 1,
        cursor_before: 0,
        cursor_after: 1,
    },
}
~~~

Assert that `validate_transition_contract` accepts the exact event once the
contract is implemented. Add the five negative cases with the same helper:

~~~
wrong cursor_before = 1                   -> Err(Randomness)
wrong value = 2                           -> Err(Randomness)
raw_words_consumed = 2, cursor_after = 2 -> Err(Randomness)
after cursor = 1 with no RNG event        -> Err(Randomness)
correct event with altered delta audit    -> Err(EventDeltaMismatch)
~~~

- [ ] **Step 2: Run the focused contract tests and confirm the expected red failure**

~~~
cargo test -p mtgml-rules --locked random
cargo test -p mtgml-rules --locked event_delta
~~~

Expected: the new event test does not compile until the event and delta
variants exist; existing M1.4 tests remain green.

- [ ] **Step 3: Add matching typed event and delta variants**

In `crates/mtgml-state/src/delta.rs`, import
`RandomStreamKeyV1` and add:

~~~
RandomValueSampled {
    stream: RandomStreamKeyV1,
    bound: u64,
    value: u64,
    raw_words_consumed: u64,
    cursor_before: u64,
    cursor_after: u64,
},
~~~

In `crates/mtgml-rules/src/events.rs`, import
`RandomStreamKeyV1`, add the same variant to
`AuthoritativeRuleEventKind`, and map every field one-for-one in
`semantic_delta()`.

- [ ] **Step 4: Extend the semantic cursor with exact primitive validation**

Add `root_seed: RootSeed256` to `SemanticValidationCursor` and initialize
it from `state.random.root_seed`. Add this match arm:

~~~
AuthoritativeRuleEventKind::RandomValueSampled {
    stream,
    bound,
    value,
    raw_words_consumed,
    cursor_before,
    cursor_after,
} => {
    let current = self
        .random_counters
        .get(stream)
        .copied()
        .ok_or(TransitionViolation::Randomness)?;
    if current != *cursor_before {
        return Err(TransitionViolation::Randomness);
    }
    let current_cursor = RandomStreamCursorV1 {
        next_raw_u64: current,
    };
    let (expected_value, expected_consumed, expected_cursor) =
        mtgml_random::sampling::uniform_below_u64(
            &self.root_seed,
            stream,
            &current_cursor,
            *bound,
        )
        .map_err(|_| TransitionViolation::Randomness)?;
    if expected_value != *value
        || expected_consumed != *raw_words_consumed
        || expected_cursor.next_raw_u64 != *cursor_after
    {
        return Err(TransitionViolation::Randomness);
    }
    self.random_counters
        .insert(*stream, expected_cursor.next_raw_u64);
}
~~~

Use `TransitionViolation::Randomness` for absent streams, invalid bounds,
sampler exhaustion, and all field mismatches. Import
`RandomStreamCursorV1`, `RandomStreamKeyV1`, and `RootSeed256` as needed.
Do not hand-roll HMAC or bounded-sampling logic.

- [ ] **Step 5: Run the causal and existing composition tests**

~~~
cargo test -p mtgml-rules --locked random
cargo test -p mtgml-rules --locked life_event
cargo test -p mtgml-rules --locked event_delta
~~~

Expected: every focused test passes, including the five negative causal
checks and all pre-existing M1.4 cursor tests.

- [ ] **Step 6: Commit the event/delta contract**

~~~
git add -- crates/mtgml-state/src/delta.rs crates/mtgml-rules/src/events.rs crates/mtgml-rules/src/semantic_cursor.rs crates/mtgml-rules/src/tests.rs
git commit -m "feat: validate deterministic random sample events"
~~~

### Task 4: Add trusted service errors and the accepted M1.5 product

**Files:**

- Test: `crates/mtgml-rules/src/tests.rs`
- Modify: `crates/mtgml-rules/src/errors.rs`
- Modify: `crates/mtgml-rules/src/synthetic.rs`

- [ ] **Step 1: Write the failing exact accepted-product test**

Update the existing M1.4 exact-product test to expect:

~~~
revision                  0 -> 1
acting life               40 -> 38
pending decision          DecisionId(1) -> None
SyntheticM1/Global cursor 0 -> 1
next_effect_id            EffectInstanceId(1) -> EffectInstanceId(2)
next_rule_event_id        RuleEventId(1) -> RuleEventId(5)
random value              1
bound                     10
raw words                 1
~~~

The exact events must be:

~~~
LifeChanged { player: actor, from: 40, to: 39 }
LifeChanged { player: actor, from: 39, to: 38 }
RandomValueSampled {
    stream: RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
    bound: 10,
    value: 1,
    raw_words_consumed: 1,
    cursor_before: 0,
    cursor_after: 1,
}
DecisionCleared { decision: DecisionId(1) }
~~~

Build `expected_after` by cloning the before-state and changing only the
revision, acting-player life, pending decision, random cursor, effect
allocator, and rule-event allocator. Build `expected_delta` from the exact
event-derived audit. Assert complete `TransitionResult` equality against the
expected product, delta reapplication/digest parity, exact root seed
preservation, and unchanged unrelated streams/allocators/state components.

- [ ] **Step 2: Run the accepted test and confirm it fails for missing service work**

~~~
cargo test -p mtgml-rules --locked synthetic_m1_acceptance_returns_exact_transition_product
~~~

Expected: FAIL because the current kernel emits only the three M1.4 events,
does not advance the effect allocator, and does not advance the RNG cursor.

- [ ] **Step 3: Add typed trusted service errors**

In `crates/mtgml-rules/src/errors.rs`, import
`mtgml_random::RandomValidationError` and
`mtgml_state::IdentityAllocationError`, then add:

~~~
#[error("deterministic RNG service failed: {0}")]
Random(#[from] RandomValidationError),
#[error("identity allocator failed: {0}")]
IdentityAllocation(#[from] IdentityAllocationError),
~~~

Keep ordinary response failures on `Ok(accepted = false)`; only trusted
service failures use these variants.

- [ ] **Step 4: Implement the accepted local workspace order**

After all existing response and binding checks in
`crates/mtgml-rules/src/synthetic.rs`, use the existing state and random
services in this order:

~~~
let stream = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
if state.random.lookup_stream(&stream).is_err() {
    return rejected(state);
}
if state
    .core
    .players
    .get(&actor)
    .map(|player| player.life)
    != Some(40)
{
    return rejected(state);
}

let mut next_state = state.clone();
next_state.allocators.allocate_effect_id()?;
let next_revision = state.revision.0.checked_add(1)
    .ok_or(KernelExecutionError::RevisionOverflow)?;
let first_event_id = state.allocators.next_rule_event_id;
let second_event_id = RuleEventId(first_event_id.0.checked_add(1)
    .ok_or(KernelExecutionError::RuleEventIdOverflow)?);
let third_event_id = RuleEventId(second_event_id.0.checked_add(1)
    .ok_or(KernelExecutionError::RuleEventIdOverflow)?);
let fourth_event_id = RuleEventId(third_event_id.0.checked_add(1)
    .ok_or(KernelExecutionError::RuleEventIdOverflow)?);
let next_event_id = RuleEventId(fourth_event_id.0.checked_add(1)
    .ok_or(KernelExecutionError::RuleEventIdOverflow)?);
next_state.revision = StateRevision(next_revision);

next_state.core.players.get_mut(&actor)
    .ok_or(KernelExecutionError::AfterState(
        EngineStateViolation::MissingTurnPlayer,
    ))?
    .life = 39;
let mut events = vec![AuthoritativeRuleEvent {
    event_id: first_event_id,
    state_revision: next_state.revision,
    event: AuthoritativeRuleEventKind::LifeChanged {
        player: actor, from: 40, to: 39,
    },
}];
next_state.core.players.get_mut(&actor)
    .ok_or(KernelExecutionError::AfterState(
        EngineStateViolation::MissingTurnPlayer,
    ))?
    .life = 38;
events.push(AuthoritativeRuleEvent {
    event_id: second_event_id,
    state_revision: next_state.revision,
    event: AuthoritativeRuleEventKind::LifeChanged {
        player: actor, from: 39, to: 38,
    },
});

let cursor_before = next_state.random.lookup_stream(&stream)?.next_raw_u64;
let (value, raw_words_consumed) = next_state.uniform_below_u64(&stream, 10)?;
let cursor_after = next_state.random.lookup_stream(&stream)?.next_raw_u64;
events.push(AuthoritativeRuleEvent {
    event_id: third_event_id,
    state_revision: next_state.revision,
    event: AuthoritativeRuleEventKind::RandomValueSampled {
        stream, bound: 10, value, raw_words_consumed,
        cursor_before, cursor_after,
    },
});
next_state.execution.pending_decision = None;
events.push(AuthoritativeRuleEvent {
    event_id: fourth_event_id,
    state_revision: next_state.revision,
    event: AuthoritativeRuleEventKind::DecisionCleared {
        decision: request.decision_id,
    },
});
next_state.allocators.next_rule_event_id = next_event_id;
~~~

Retain the existing exact audit construction, `StateDelta::between`,
after-state validation, transition-contract validation, and accepted result
construction. The effect ID is intentionally not emitted or stored in an
`EffectRecord`; its only authoritative evidence is the complete allocator
replacement.

- [ ] **Step 5: Run the accepted product and complete rules tests**

~~~
cargo test -p mtgml-rules --locked synthetic_m1_acceptance
cargo test -p mtgml-rules --locked
~~~

Expected: the exact M1.5 accepted product, non-default actor regression, M1.3
rejection matrix, and all composition tests pass.

- [ ] **Step 6: Commit the accepted M1.5 transition**

~~~
git add -- crates/mtgml-rules/src/errors.rs crates/mtgml-rules/src/synthetic.rs crates/mtgml-rules/src/tests.rs
git commit -m "feat: consume deterministic RNG and effect identity in M1.5"
~~~

### Task 5: Add repeatability, isolation, rejection, and failure evidence

**Files:**

- Test: `crates/mtgml-rules/src/tests.rs`

- [ ] **Step 1: Add exact repeatability evidence**

Construct two independent states with the same explicit inputs and two
independent kernels:

~~~
let before_a = synthetic_state();
let before_b = synthetic_state();
let response_a = synthetic_response(&before_a);
let response_b = synthetic_response(&before_b);
let mut kernel_a = SyntheticM1RulesKernel;
let mut kernel_b = SyntheticM1RulesKernel;

let result_a = kernel_a.apply(&before_a, PlayerId(1), &response_a).unwrap();
let result_b = kernel_b.apply(&before_b, PlayerId(1), &response_b).unwrap();

assert_eq!(result_a, result_b);
assert_eq!(result_a.next_state.random.root_seed, before_a.random.root_seed);
assert_eq!(
    result_a.next_state.random.lookup_stream(
        &RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1)
    ).unwrap().next_raw_u64,
    1
);
assert_eq!(result_a.next_state.allocators.next_effect_id, EffectInstanceId(2));
~~~

The complete `TransitionResult` equality is the proof; do not assert that a
different seed must produce a different bounded value.

- [ ] **Step 2: Add stream and allocator isolation assertions**

Prepare a valid state with an additional player-scoped synthetic stream at
cursor 17. After an accepted transition assert that only
`SyntheticM1/Global` changes from 0 to 1, the player-scoped stream remains
17, `next_effect_id` changes 1 to 2, and every other allocator field equals
the before-state.

- [ ] **Step 3: Add missing-stream rejected-product evidence**

Clone the valid synthetic fixture, remove the required global stream, validate
the resulting generic state, submit the otherwise-valid response, and call
`assert_exact_rejected_product`. Assert that the result is
`Ok(accepted = false)`, not a `KernelExecutionError`.

- [ ] **Step 4: Extend the existing rejection matrix assertions**

Keep the existing complete rejection helper and matrix unchanged in structure.
For every normal rejected response assert the full state equality already
covered by the helper, including exact random stream maps, root seed, effect
allocator, all allocator cursors, empty events/audit, unchanged digests, and
unchanged revision.

- [ ] **Step 5: Add RNG and allocator exhaustion tests**

For RNG exhaustion, set the required stream cursor to `u64::MAX`, keep the
state valid, submit the valid response, and assert:

~~~
assert_eq!(
    kernel.apply(&before, PlayerId(1), &response),
    Err(KernelExecutionError::Random(
        RandomValidationError::StreamExhausted
    ))
);
assert_eq!(before, original_before);
~~~

For allocator exhaustion, set `next_effect_id` to
`EffectInstanceId(u64::MAX)`, keep the required stream at cursor 0, and
assert `Err(KernelExecutionError::IdentityAllocation(
IdentityAllocationError::EffectInstanceIdExhausted))`, with the original
state still equal to the input and its RNG cursor still 0. Neither test may
expect a rejected `TransitionResult`.

- [ ] **Step 6: Run focused failure evidence**

~~~
cargo test -p mtgml-rules --locked rejection
cargo test -p mtgml-rules --locked exhaustion
cargo test -p mtgml-rules --locked deterministic
~~~

Expected: all exact repeatability, isolation, rejection, missing-stream, RNG
exhaustion, and allocator exhaustion tests pass.

- [ ] **Step 7: Commit the evidence**

~~~
git add -- crates/mtgml-rules/src/tests.rs
git commit -m "test: prove M1.5 deterministic service boundaries"
~~~

### Task 6: Add checkpoint-capture evidence without M1.6 runtime work

**Files:**

- Test: `crates/mtgml-environment/src/tests.rs`

- [ ] **Step 1: Write the checkpoint continuation test**

Add the missing typed imports and create a valid state with:

~~~
state.allocators.next_effect_id = EffectInstanceId(2);
state.random = RandomStateV1::from_entries(
    RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
    vec![CanonicalRandomStreamEntryV1 {
        key: RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1),
        next_raw_u64: 1,
    }],
).unwrap();
~~~

Embed it in `EnvironmentCheckpointV2::new`, validate the checkpoint, encode
and decode it with the existing serde DTO representation, and assert:

~~~
assert_eq!(decoded, checkpoint);
assert_eq!(decoded.state.allocators.next_effect_id, EffectInstanceId(2));
assert_eq!(
    decoded.state.random.lookup_stream(&stream).unwrap().next_raw_u64,
    1
);
assert_eq!(decoded.state_digest, decoded.state.digest().unwrap());
decoded.validate().unwrap();
~~~

- [ ] **Step 2: Run environment checkpoint evidence**

~~~
cargo test -p mtgml-environment --locked checkpoint_captures
cargo test -p mtgml-environment --locked
~~~

Expected: the new DTO/capture test and all existing checkpoint tests pass.
Do not add an environment-to-rules dependency and do not implement restore,
fork, replay, or transaction ownership.

- [ ] **Step 3: Commit checkpoint evidence**

~~~
git add -- crates/mtgml-environment/src/tests.rs
git commit -m "test: prove checkpoint captures M1.5 continuation state"
~~~

### Task 7: Format, run local verification, and audit scope

**Files:** all changed files in the branch.

- [ ] **Step 1: Format and run focused Rust suites**

~~~
cargo fmt --all
cargo fmt --all -- --check
cargo test -p mtgml-random --locked
cargo test -p mtgml-rules --locked
cargo test -p mtgml-state --locked
cargo test -p mtgml-environment --locked
~~~

Expected: every command exits 0; the random suite remains byte-identical to
the accepted `mtgml.rng.v1` KATs.

- [ ] **Step 2: Run workspace and repository checks**

~~~
cargo test --workspace --all-features --locked
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
python scripts/run_checks.py fast
python scripts/check_documentation.py
python scripts/verify_repository.py
just check-fast
just check
~~~

Record each command from its actual result. If a required environment such as
WSL2/Hyper-V is unavailable, record `BLOCKED` and do not infer hosted success.

- [ ] **Step 3: Audit changed files and prohibited scope**

~~~
git diff --check origin/master...HEAD
git diff --stat origin/master...HEAD
git diff --name-only origin/master...HEAD
git status --short --branch
~~~

The changed-file set must be limited to the M1.5 process artifacts,
`mtgml-state` allocator/delta files and tests, `mtgml-rules` event/cursor/
error/kernel/test files, and the environment checkpoint test. No M1.6+
runtime code, public player schema, RNG contract version, generated
vocabulary, or status promotion may appear.

- [ ] **Step 4: Inspect hosted status only on the exact final head**

After pushing the final head, inspect the Draft PR checks with:

~~~
gh pr checks --watch=false
~~~

Report PR Fast, Integration, Nightly Certification Smoke, and CodeQL only when
the exact final head executed them successfully. Otherwise report the actual
state as `FAIL`, `BLOCKED`, or `NOT_RUN`.

### Task 8: Deliver one Draft PR without merging

**Files:** no additional source files.

- [ ] **Step 1: Record the final head**

~~~
git rev-parse HEAD
git push --set-upstream origin chris/m1-5-deterministic-services
~~~

- [ ] **Step 2: Open one Draft PR against master**

~~~
gh pr create --draft --base master --head chris/m1-5-deterministic-services --title "M1.5: deterministic services" --body-file m1-5-pr-body.md
~~~

The PR body must contain the starting and final SHAs, changed files, exact
stream/bound/value/raw-word/cursor evidence, exact effect allocator evidence,
event order and one outer revision, repeated exact-result evidence, all
negative and exhaustion evidence, checkpoint-capture evidence,
information-safety assessment, every executed command status, and explicit
confirmation that M1.6+ was not implemented. Include `Closes #24` only when
`DETERMINISTIC_RNG_AND_ALLOCATORS = PASS` is supported by executed evidence.
Do not merge.

- [ ] **Step 3: Final status statement**

Preserve these statuses unless later executable evidence changes them:

~~~
ENGINE_STATE_CONSTRUCTION_AND_INVARIANTS = PASS
ACCEPTED_TRANSITION_EXACT_PRODUCT         = PASS
STATE_DELTA_FULL_REAPPLICATION            = PASS
SEQUENTIAL_EVENT_DELTA_PARITY             = PASS
DETERMINISTIC_RNG_AND_ALLOCATORS          = PASS only after M1.5 evidence
REJECTED_RESPONSE_COMPLETE_NONMUTATION    = BLOCKED
CHECKPOINT_RESTORE_COMPLETE_IDENTITY      = NOT_RUN
FORK_PARITY                                = NOT_RUN
REPLAY_PARITY                              = NOT_RUN
MULTI_PLAYER_ENDPOINT_BINDING             = NOT_RUN
~~~

## Plan self-review

- The design's typed event, exact sampler validation, typed allocator,
  accepted event order, full replacement delta, checkpoint capture, and
  information-safety boundaries each have an implementation task.
- The canonical `"11"` fixture independently freezes value `1`, one
  consumed word, and cursor `0 -> 1`; no statistical or different-seed
  claim is used.
- Missing stream, normal rejection, RNG exhaustion, allocator exhaustion,
  event-field mismatches, audit disagreement, and final cursor mismatch are
  covered separately.
- The non-default actor regression and unchanged M1.3 rejection helper remain
  in the rules suite.
- No task adds an environment dependency to rules or implements checkpoint
  restore, fork, replay, endpoint submission, or M2 information-safety
  behavior.
- No placeholder instruction remains; every code-changing task names exact
  files, tests, commands, and expected outcomes.
