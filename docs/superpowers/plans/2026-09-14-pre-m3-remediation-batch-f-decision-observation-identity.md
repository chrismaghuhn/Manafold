# Pre-M3 Remediation Batch F: Decision, Observation, and Identity Implementation Plan

**Status:** implementation plan ready for independent review; production implementation not yet authorized

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Close the confirmed Batch F decision, observed-event, and actor-bound PlayerStep gaps while preserving the existing Rules-owned identity boundary and leaving FND-028 fail-closed pending an accepted ADR.

**Architecture:** Keep mtgml-state::validate_engine_state() and mtgml-rules::validate_transition_contract() as the authoritative cross-component boundaries. Add no-op checks to the Rules semantic cursor, one widened shared candidate-capacity helper to both Rust dense-ID paths, semantic V2 observation and PlayerStep checks in Rust and Python, and evidence-only regressions for FND-013, FND-027, and FND-028. Do not add a resolver call to project_player_request() or a second final identity comparison to the lifecycle projector.

**Tech Stack:** Rust workspace with Cargo and locked dependencies; Python 3.13 reference DTOs and pinned repository scripts; canonical JSON wire fixtures; GitHub Actions via one PR against master.

**Date:** 2026-09-14
**Base:** b24bba153f2aa74bd59e8d6a872a0612ff7f76aa
**Branch:** chris/pre-m3-remediation-batch-f-decision-observation-identity

---

## Scope and invariants

The implementation is limited to FND-009, FND-013, FND-014, FND-015,
FND-016A, FND-027 evidence, and the FND-028 policy candidate. FND-016B is
DEFER_TO_EVD_005. FND-026B remains BLOCKED_CONTRACT_AMBIGUITY, and
FND-026C remains DEFERRED_P2.

The following values remain unchanged:

~~~text
PUBLIC_API_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO
NEW_MAGIC_SEMANTICS = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
MERGE_PERFORMED = NO
~~~

The only new Rust API-level value is the internal/experimental
DecisionValidationError::CandidateCapacityExceeded variant. It is not a wire
value, schema value, player-facing error code, digest input, or replay value.

## File map

| File | Responsibility in this plan |
| --- | --- |
| crates/mtgml-rules/src/semantic_cursor.rs | Reject no-op LifeChanged and ObjectTapped events at the Rules semantic owner. |
| crates/mtgml-rules/src/tests.rs and crates/mtgml-rules/src/tests/batch_f.rs | Batch-F Rules RED tests, positive controls, and FND-027 transition-parity regression. |
| crates/mtgml-decision/src/lib.rs | Shared widened candidate-capacity helper, checked dense conversions, and exact-binding primitive tests. |
| crates/mtgml-state/src/tests.rs and crates/mtgml-state/src/tests/batch_f.rs | FND-013 authoritative-state exact-binding evidence and FND-028 zero-policy characterization. |
| crates/mtgml-observation/src/error.rs | Typed Rust diagnostic for an identity-less V2 ObjectMoved. |
| crates/mtgml-observation/src/observed_event.rs | V2 ObjectMoved semantic validation. |
| crates/mtgml-observation/src/player_step.rs | FND-016A local rejection/request/status closure. |
| crates/mtgml-observation/src/tests.rs and crates/mtgml-observation/src/tests/batch_f.rs | V2 observed-event and PlayerStep RED/GREEN tests. |
| crates/mtgml-environment/src/tests.rs and crates/mtgml-environment/src/tests/batch_f.rs | FND-027 call-order and FND-028 boundary characterization. |
| crates/mtgml-conformance/src/isolation/paired.rs | Keep the test-only rejected-step fixture valid under FND-016A. |
| python/src/mtgml/decision.py | Python logical candidate-capacity parity. |
| python/src/mtgml/observation.py | Python V2 observed-event and PlayerStep semantic parity. |
| python/tests/test_batch_f.py | Cross-language Batch-F semantic tests without large allocations. |
| wire/negative/observed-event-v2-object-moved-no-identity.json | Canonical V2 observed-event negative fixture. |
| wire/negative/player-step-v2-rejection-missing-next-decision.json | Canonical FND-016A negative fixture for a current-request rejection without a decision. |
| wire/negative/player-step-v2-unavailable-with-next-decision.json | Canonical FND-016A negative fixture for an unavailable rejection carrying a decision. |
| wire/negative/manifest.json | Register the three new semantic negative fixtures. |
| docs/DECISION_PROTOCOL.md | State the exact binding owner and dense candidate-count capacity. |
| docs/INFORMATION_MODEL.md | State the V2 ObjectMoved visible-identity rule. |
| docs/RULES_SEMANTICS.md | State the non-no-op rule for mutation event families without imposing a blanket event rule. |
| docs/ML_ENVIRONMENT.md | State the local FND-016A rejection matrix and the EVD-005 boundary. |
| docs/superpowers/specs/2026-09-14-player-id-zero-policy-adr-candidate.md | Unnumbered, non-authoritative FND-028 ADR candidate owned by architecture maintainers. |
| docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-f-dispositions-and-evidence.md | Final Batch-F disposition, integration matrix, and exact gate evidence. |
| docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-f-decision-observation-identity-design.md | Append exact implementation/evidence names after the final gates. |
| docs/normative-document-register.v1.json | Register the Batch-F plan, ADR candidate, and evidence document after each exists. |

No generated vocabulary, JSON Schema, historical fixture, replay type, or
production endpoint method is modified.

## Task 1: Add the Batch-F RED characterization tests

**Files:**

- Modify: crates/mtgml-rules/src/tests.rs
- Create: crates/mtgml-rules/src/tests/batch_f.rs
- Modify: crates/mtgml-observation/src/tests.rs
- Create: crates/mtgml-observation/src/tests/batch_f.rs
- Modify: crates/mtgml-environment/src/tests.rs
- Create: crates/mtgml-environment/src/tests/batch_f.rs
- Modify: crates/mtgml-state/src/tests.rs
- Create: crates/mtgml-state/src/tests/batch_f.rs

- [ ] **Step 1: Wire the test-only Batch-F files**

Append these includes after the existing Batch-E includes:

~~~rust
// crates/mtgml-rules/src/tests.rs
include!("tests/batch_f.rs");

// crates/mtgml-observation/src/tests.rs
include!("tests/batch_f.rs");

// crates/mtgml-environment/src/tests.rs
include!("tests/batch_f.rs");
~~~

Run:

~~~powershell
cargo test -p mtgml-rules --locked
cargo test -p mtgml-observation --locked
cargo test -p mtgml-environment --locked
~~~

Expected: the existing base suites pass with no Batch-F test names yet.

- [ ] **Step 2: Write the Rules no-op RED tests**

In crates/mtgml-rules/src/tests/batch_f.rs, add a helper that builds a
single-event accepted product from state_without_pending_decision():

~~~rust
fn single_event_product(
    before: &EngineState,
    after: EngineState,
    event: AuthoritativeRuleEvent,
) -> TransitionResult {
    let audit = vec![event.event.semantic_delta()];
    let delta = mtgml_state::StateDelta::between(before, &after, audit).unwrap();
    TransitionResult {
        accepted: true,
        next_state: after.clone(),
        delta,
        events: vec![event],
        next_decision: None,
        status: mtgml_model::EpisodeStatus::Running,
    }
}
~~~

Add these tests:

~~~rust
#[test]
fn fnd_009_noop_life_change_is_rejected_by_the_transition_contract() {
    let before = state_without_pending_decision();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    let event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::LifeChanged {
            player: PlayerId(1),
            from: 40,
            to: 40,
        },
    };
    let result = single_event_product(&before, after, event);
    assert_eq!(
        validate_transition_contract(&before, &result),
        Err(TransitionViolation::LifeChange)
    );
}

#[test]
fn fnd_009_noop_object_tap_is_rejected_by_the_transition_contract() {
    let before = state_without_pending_decision();
    let mut after = before.clone();
    after.revision = StateRevision(1);
    after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    let event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::ObjectTapped {
            object: mtgml_model::GameObjectId(1),
            from: false,
            to: false,
        },
    };
    let result = single_event_product(&before, after, event);
    assert_eq!(
        validate_transition_contract(&before, &result),
        Err(TransitionViolation::TapChange)
    );
}

#[test]
fn fnd_009_real_life_and_tap_mutations_remain_valid() {
    let before = state_without_pending_decision();

    let mut life_after = before.clone();
    life_after.revision = StateRevision(1);
    life_after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    life_after.core.players.get_mut(&PlayerId(1)).unwrap().life = 39;
    let life_event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::LifeChanged {
            player: PlayerId(1),
            from: 40,
            to: 39,
        },
    };
    assert!(validate_transition_contract(
        &before,
        &single_event_product(&before, life_after, life_event)
    )
    .is_ok());

    let mut tap_after = before.clone();
    tap_after.revision = StateRevision(1);
    tap_after.allocators.next_rule_event_id = mtgml_model::RuleEventId(2);
    tap_after.zones.objects.get_mut(&mtgml_model::GameObjectId(1)).unwrap().tapped = true;
    let tap_event = AuthoritativeRuleEvent {
        event_id: mtgml_model::RuleEventId(1),
        state_revision: StateRevision(1),
        event: AuthoritativeRuleEventKind::ObjectTapped {
            object: mtgml_model::GameObjectId(1),
            from: false,
            to: true,
        },
    };
    assert!(validate_transition_contract(
        &before,
        &single_event_product(&before, tap_after, tap_event)
    )
    .is_ok());
}
~~~

Add a same-incarnation ZoneTransition control that asserts the existing
Err(TransitionViolation::ZoneTransition). This records that FND-009 does not
broaden into a blanket rejection of occurrence-only events or a second
ZoneTransition rule.

- [ ] **Step 3: Write the bounded FND-014 source RED**

Add this test to the existing cfg(test) module in
crates/mtgml-decision/src/lib.rs before the production helper exists:

~~~rust
#[test]
fn fnd_014_dense_candidate_paths_have_checked_u32_boundaries() {
    let source = include_str!("lib.rs");
    assert!(!source.contains("expect(\"candidate ordering is bounded by u32\")"));
    assert!(!source.contains("index as u32"));
}
~~~

This is a bounded source-contract RED, not an artificial huge allocation. It
fails on the base for the exact expect() and index as u32 expressions.

- [ ] **Step 4: Write the V2 observed-event RED and controls**

In crates/mtgml-observation/src/tests/batch_f.rs, add an envelope factory and
assert that ObjectMoved(None, None) is rejected while old-only, new-only, and
both-present forms are accepted:

~~~rust
fn moved(old_object: Option<u64>, new_object: Option<u64>) -> ObservedEventEnvelopeV2 {
    ObservedEventEnvelopeV2 {
        schema_version: OBSERVED_EVENT_SCHEMA_V2.into(),
        sequence: VisibleSequence(1),
        state_revision: StateRevision(0),
        event: ObservedEventKindV2::ObjectMoved {
            old_object: old_object.map(OpaqueObjectId),
            new_object: new_object.map(OpaqueObjectId),
            from: mtgml_model::ZoneKind::Hand,
            to: mtgml_model::ZoneKind::Battlefield,
        },
    }
}

#[test]
fn fnd_015_object_moved_requires_at_least_one_visible_identity() {
    assert!(moved(None, None).validate().is_err());
    assert!(moved(Some(3), None).validate().is_ok());
    assert!(moved(None, Some(11)).validate().is_ok());
    assert!(moved(Some(3), Some(11)).validate().is_ok());
}
~~~

- [ ] **Step 5: Write the FND-016A local RED matrix**

Build the fixtures with these exact helpers in the new observation test file:

~~~rust
fn valid_information_state() -> PlayerInformationStateV2 {
    let mut state = PlayerInformationStateV2 {
        schema_version: INFORMATION_STATE_SCHEMA_V2.into(),
        perspective: PlayerId(1),
        state_revision: StateRevision(0),
        current_observation: observation(b"{}", b"{}"),
        next_visible_sequence: VisibleSequence(0),
        retained_knowledge: Vec::new(),
        digest: mtgml_model::InformationStateDigestV2::from_canonical_bytes(b"placeholder"),
    };
    let (_, digest) = mtgml_wire::compute_information_state_digest_v2(&state.digest_input()).unwrap();
    state.digest = digest;
    state
}

fn valid_current_request() -> PlayerDecisionRequestV2 {
    PlayerDecisionRequestV2 {
        schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(0),
        actor: PlayerId(1),
        visibility: DecisionVisibility::Public,
        decision: DecisionDomainV2::ChooseOne,
        candidates: vec![
            VisibleCandidateV2 {
                candidate_id: CandidateIdV1(0),
                intent: CandidateIntent::ChooseBoolean { value: false },
            },
            VisibleCandidateV2 {
                candidate_id: CandidateIdV1(1),
                intent: CandidateIntent::ChooseBoolean { value: true },
            },
        ],
    }
}

fn rejected_step(
    code: PlayerSubmissionCodeV1,
    next_decision: Option<PlayerDecisionRequestV2>,
    status: EpisodeStatus,
) -> PlayerStepV2 {
    PlayerStepV2 {
        schema_version: PLAYER_STEP_SCHEMA_V2.into(),
        information_state: valid_information_state(),
        observed_events: Vec::new(),
        next_decision,
        status,
        submission: PlayerStepSubmissionV1::Rejected { code },
    }
}
~~~

For each of StaleDecision, InvalidAnswer, InvalidCandidate, DuplicateAssignment,
InvalidCardinality, InvalidNumber, and InvalidOrder, assert that
rejected_step(code, None, EpisodeStatus::Running).validate() returns
ObservationValidationError::Submission and that the same code with
Some(valid_current_request()) returns Ok(()). Assert that
UnavailableDecision with a current request returns Submission while the same
code without one returns Ok(()). Assert that fixture-only EpisodeClosed with a
non-running status and no next decision succeeds. Every accepted local
rejection fixture must have an empty observed-event vector.

The first assertions and unavailable-with-decision case are expected RED on
the base. The test must also assert an empty observed-event batch in every
accepted local rejection shape.

- [ ] **Step 6: Add FND-013, FND-027, and FND-028 characterization evidence**

In crates/mtgml-state/src/tests/batch_f.rs, add same-variant/different-value
pending-candidate cases for SelectPlayer, SelectMode, ChooseBoolean,
DeclareNumber, SelectObject, and ActivateAbility. Every mismatch must return
Err(EngineStateViolation::PendingDecisionMismatch) while the matching control
returns Ok(()). The direct authoritative request validator and projection
remain structural positive controls.

In crates/mtgml-rules/src/tests/batch_f.rs, mutate a lifecycle-owned identity
field in an otherwise valid after-state and assert that
validate_transition_contract() returns
Err(TransitionViolation::OccurrencePairing). This is evidence for the existing
Rules owner and does not call the projector.

In crates/mtgml-environment/src/tests/batch_f.rs, characterize a distinct
[PlayerId(0), PlayerId(1)] state, environment binding of player zero, a V3
manifest deck with player zero, and the existing rejection of a V3 replay step
whose actor is zero. Record the contradiction without imposing a new policy.

- [ ] **Step 7: Run the RED/characterization tests and commit**

~~~powershell
cargo test -p mtgml-rules --locked fnd_009
cargo test -p mtgml-decision --locked fnd_014
cargo test -p mtgml-observation --locked fnd_015
cargo test -p mtgml-observation --locked fnd_016a
cargo test -p mtgml-state --locked fnd_013
cargo test -p mtgml-rules --locked fnd_027
cargo test -p mtgml-environment --locked fnd_028
~~~

Expected: the confirmed-defect tests fail for the base behavior; FND-013,
FND-027, and the bounded FND-028 characterizations pass with their documented
current behavior. Fix test setup errors before committing.

~~~powershell
git diff --check
git status --short
git add crates/mtgml-rules/src/tests.rs crates/mtgml-rules/src/tests/batch_f.rs crates/mtgml-observation/src/tests.rs crates/mtgml-observation/src/tests/batch_f.rs crates/mtgml-environment/src/tests.rs crates/mtgml-environment/src/tests/batch_f.rs crates/mtgml-state/src/tests.rs crates/mtgml-state/src/tests/batch_f.rs crates/mtgml-decision/src/lib.rs
git diff --cached --check
git commit -m "tests: characterize Batch-F decision observation identity boundaries"
~~~

Expected commit content: tests only. No production implementation, Python
implementation, fixture, schema, or documentation is included in this RED
commit.

## Task 2: Close FND-009 at the Rules semantic owner

**Files:**

- Modify: crates/mtgml-rules/src/semantic_cursor.rs
- Modify: docs/RULES_SEMANTICS.md
- Test: crates/mtgml-rules/src/tests/batch_f.rs

- [ ] **Step 1: Add the minimal no-op checks**

At the start of the LifeChanged arm, before mutating the cursor value, add:

~~~rust
if from == to {
    return Err(TransitionViolation::LifeChange);
}
~~~

At the start of the ObjectTapped arm, add:

~~~rust
if from == to {
    return Err(TransitionViolation::TapChange);
}
~~~

Leave the existing object/player lookup, before-value, and final-state proof
unchanged. Do not add checks to PublicOutcome, RandomValueSampled, or
envelope-producing PerspectiveOccurrence families.

- [ ] **Step 2: Update the normative event classification**

In docs/RULES_SEMANTICS.md, state that LifeChanged and ObjectTapped are
mutation events and require unequal endpoints; ZoneTransition is an
incarnation/identity change and already requires distinct old/new objects;
occurrence-only events retain their existing occurrence semantics. Do not say
that every event must mutate core state.

- [ ] **Step 3: Run GREEN and commit**

~~~powershell
cargo test -p mtgml-rules --locked fnd_009
cargo test -p mtgml-rules --locked transition_contract
git diff --check
git add crates/mtgml-rules/src/semantic_cursor.rs crates/mtgml-rules/src/tests/batch_f.rs docs/RULES_SEMANTICS.md
git commit -m "fix: reject no-op mutation events"
~~~

Expected: both no-op tests fail before this change and pass afterward; real
life/tap controls and ZoneTransition rejection remain green.

## Task 3: Close FND-014 with one widened capacity rule

**Files:**

- Modify: crates/mtgml-decision/src/lib.rs
- Modify: docs/DECISION_PROTOCOL.md
- Modify: python/src/mtgml/decision.py
- Create: python/tests/test_batch_f.py
- Test: crates/mtgml-decision/src/lib.rs

- [ ] **Step 1: Add the internal typed error and shared Rust helper**

Add CandidateCapacityExceeded to DecisionValidationError. Keep it an
internal/experimental Rust error; do not map it to a new player submission code.

Near CandidateOrderingV1, define one shared rule:

~~~rust
const CANDIDATE_ID_COUNT_CAPACITY: u64 = u64::from(u32::MAX) + 1;

fn validate_candidate_capacity(
    candidate_count: usize,
) -> Result<(), DecisionValidationError> {
    let count = u64::try_from(candidate_count)
        .map_err(|_| DecisionValidationError::CandidateCapacityExceeded)?;
    if count > CANDIDATE_ID_COUNT_CAPACITY {
        return Err(DecisionValidationError::CandidateCapacityExceeded);
    }
    Ok(())
}
~~~

This is widened before comparison. It does not use usize::MAX + 1 and does
not depend on a platform truncating the capacity.

- [ ] **Step 2: Apply the helper to both Rust candidate paths**

At the start of CandidateOrderingV1::assign_dense, call
validate_candidate_capacity(candidates.len())? before consuming or sorting the
input. Replace the expect() conversion with a checked conversion and collect
the result:

~~~rust
validate_candidate_capacity(candidates.len())?;
let assigned = keyed
    .into_iter()
    .enumerate()
    .map(|(index, (_, visible_intent, trusted_binding))| {
        let candidate_id = u32::try_from(index)
            .map_err(|_| DecisionValidationError::CandidateCapacityExceeded)?;
        Ok(AuthoritativeCandidateV2 {
            candidate_id: CandidateIdV1(candidate_id),
            visible_intent,
            trusted_binding,
        })
    })
    .collect::<Result<Vec<_>, DecisionValidationError>>()?;
Ok(assigned)
~~~

At the start of CandidateOrderingV1::validate_public, call the same helper.
Inside the loop use:

~~~rust
let expected = u32::try_from(index)
    .map_err(|_| DecisionValidationError::CandidateCapacityExceeded)?;
if candidate.candidate_id.0 != expected {
    return Err(DecisionValidationError::CandidateIdsNotDense);
}
~~~

Do not retain index as u32 or a second bound constant.

- [ ] **Step 3: Add bounded Rust GREEN boundary tests**

Add tests in the existing decision test module:

~~~rust
#[test]
fn candidate_capacity_uses_the_full_u32_id_domain_without_allocation() {
    let capacity = u64::from(u32::MAX) + 1;
    let Ok(last_count) = usize::try_from(capacity) else {
        return;
    };
    assert_eq!(validate_candidate_capacity(last_count), Ok(()));
    let first_unrepresentable = last_count.checked_add(1).unwrap();
    assert_eq!(
        validate_candidate_capacity(first_unrepresentable),
        Err(DecisionValidationError::CandidateCapacityExceeded)
    );
}

#[test]
fn dense_assignment_and_public_validation_remain_exact_for_small_inputs() {
    let assigned = CandidateOrderingV1::assign_dense(vec![
        (CandidateIntent::Confirm, EngineCandidateBinding::Confirm),
        (CandidateIntent::PassPriority, EngineCandidateBinding::PassPriority),
    ]).unwrap();
    assert_eq!(
        assigned.iter().map(|candidate| candidate.candidate_id).collect::<Vec<_>>(),
        vec![CandidateIdV1(0), CandidateIdV1(1)]
    );
    let visible = assigned.iter().map(|candidate| VisibleCandidateV2 {
        candidate_id: candidate.candidate_id,
        intent: candidate.visible_intent.clone(),
    }).collect::<Vec<_>>();
    assert!(CandidateOrderingV1::validate_public(&visible).is_ok());
}
~~~

The boundary test skips only on a host whose usize cannot represent 2^32; it
never allocates a large vector. The source RED from Task 1 is the pre-fix
evidence for the old panic/unchecked paths.

- [ ] **Step 4: Keep Python's logical limit aligned**

In python/src/mtgml/decision.py, define
_CANDIDATE_ID_COUNT_CAPACITY = 2**32 and a small
_validate_candidate_capacity(candidate_count: int) -> None helper. Call it at
the start of PlayerDecisionRequestV2.validate() before the candidate loop.
Python's arbitrary integers avoid the Rust conversion hazard, but the logical
contract must reject a count above the same representable domain.

Add python/tests/test_batch_f.py coverage for the helper at 2**32 and 2**32 + 1,
plus a small dense request. Do not build a tuple containing billions of
candidates.

- [ ] **Step 5: Document and verify FND-014**

Add to docs/DECISION_PROTOCOL.md that dense CandidateIdV1 values use the full
0..=u32::MAX domain, so the dense count capacity is 2^32, and that both
authoritative assignment and public validation use the same checked boundary.

Run and commit:

~~~powershell
cargo test -p mtgml-decision --locked
C:\Python313\python.exe -m pytest python/tests/test_batch_f.py -q
git diff --check
git add crates/mtgml-decision/src/lib.rs docs/DECISION_PROTOCOL.md python/src/mtgml/decision.py python/tests/test_batch_f.py
git commit -m "fix: make candidate capacity fail closed"
~~~

Expected: the pre-fix source RED fails, the typed boundary returns without
allocation, dense IDs remain exact, and no public/wire error code changes.

## Task 4: Close FND-015 across Rust, Python, and the semantic fixture corpus

**Files:**

- Modify: crates/mtgml-observation/src/error.rs
- Modify: crates/mtgml-observation/src/observed_event.rs
- Modify: crates/mtgml-observation/src/tests/batch_f.rs
- Modify: python/src/mtgml/observation.py
- Modify: python/tests/test_batch_f.py
- Modify: docs/INFORMATION_MODEL.md
- Create: wire/negative/observed-event-v2-object-moved-no-identity.json
- Modify: wire/negative/manifest.json

- [ ] **Step 1: Add the Rust semantic error and check**

Add a narrowly named ObservationValidationError::ObjectMovedIdentity. In
ObservedEventEnvelopeV2::validate(), match the V2 ObjectMoved variant before
the random/public outcome arms:

~~~rust
ObservedEventKindV2::ObjectMoved {
    old_object: None,
    new_object: None,
    ..
} => Err(ObservationValidationError::ObjectMovedIdentity),
~~~

Leave V1 ObservedEventKind::ObjectMoved unchanged. Keep the optional V2 wire
fields and schema exactly as they are.

- [ ] **Step 2: Add Python semantic parity**

After parsing V2 old_object and new_object, reject the pair when both are None
with WireError("semantic.observed_event", "object_moved must reveal at least one identity").
The old-only, new-only, and both-present forms remain constructible.

- [ ] **Step 3: Add the shared negative fixture**

Create the canonical JSON bytes with both fields explicitly null:

~~~json
{"event":{"from":"hand","kind":"object_moved","new_object":null,"old_object":null,"to":"battlefield"},"schema_version":"observed-event-envelope.v2","sequence":"1","state_revision":"0"}
~~~

Register it in wire/negative/manifest.json with contract
observed-event-envelope.v2, expected error semantic.observed_event, and the
existing rust-python-semantic-or-decode layer. Do not alter the V2 schema.

- [ ] **Step 4: Update the normative observation text and verify**

State in docs/INFORMATION_MODEL.md that an emitted V2 ObjectMoved envelope
contains at least one perspective-visible opaque identity; audience policies
remain the Rules owner's responsibility.

Run:

~~~powershell
cargo test -p mtgml-observation --locked
cargo test -p mtgml-wire --locked every_shared_negative_fixture_is_rejected_with_the_expected_code
C:\Python313\python.exe -m pytest python/tests/test_batch_f.py python/tests/test_wire_contracts.py -q
git diff --check
git add crates/mtgml-observation/src/error.rs crates/mtgml-observation/src/observed_event.rs crates/mtgml-observation/src/tests/batch_f.rs python/src/mtgml/observation.py python/tests/test_batch_f.py docs/INFORMATION_MODEL.md wire/negative/observed-event-v2-object-moved-no-identity.json wire/negative/manifest.json
git commit -m "fix: require visible identity in V2 object moves"
~~~

Expected: the FND-015 RED becomes GREEN, Python and Rust reject the same
negative fixture, all three positive identity shapes pass, and existing
eventful production projection remains valid.

## Task 5: Close FND-016A and preserve the FND-016B/EVD-005 boundary

**Files:**

- Modify: crates/mtgml-observation/src/player_step.rs
- Modify: crates/mtgml-observation/src/tests/batch_f.rs
- Modify: python/src/mtgml/observation.py
- Modify: python/tests/test_batch_f.py
- Modify: crates/mtgml-conformance/src/isolation/paired.rs
- Modify: docs/ML_ENVIRONMENT.md
- Create: wire/negative/player-step-v2-rejection-missing-next-decision.json
- Create: wire/negative/player-step-v2-unavailable-with-next-decision.json
- Modify: wire/negative/manifest.json

- [ ] **Step 1: Implement the Rust code-specific matrix**

Keep the existing event emptiness, perspective/revision, status, and
EpisodeClosed checks. Replace the rejection tail with this code-specific shape:

~~~rust
if let PlayerStepSubmissionV1::Rejected { code } = &self.submission {
    if !self.observed_events.is_empty() {
        return Err(ObservationValidationError::Submission);
    }
    match code {
        PlayerSubmissionCodeV1::EpisodeClosed => {
            if matches!(self.status, EpisodeStatus::Running)
                || self.next_decision.is_some()
            {
                return Err(ObservationValidationError::Submission);
            }
        }
        PlayerSubmissionCodeV1::UnavailableDecision => {
            if !matches!(self.status, EpisodeStatus::Running)
                || self.next_decision.is_some()
            {
                return Err(ObservationValidationError::Submission);
            }
        }
        PlayerSubmissionCodeV1::StaleDecision
        | PlayerSubmissionCodeV1::InvalidAnswer
        | PlayerSubmissionCodeV1::InvalidCandidate
        | PlayerSubmissionCodeV1::DuplicateAssignment
        | PlayerSubmissionCodeV1::InvalidCardinality
        | PlayerSubmissionCodeV1::InvalidNumber
        | PlayerSubmissionCodeV1::InvalidOrder => {
            if !matches!(self.status, EpisodeStatus::Running)
                || self.next_decision.is_none()
            {
                return Err(ObservationValidationError::Submission);
            }
        }
    }
}
~~~

The existing preceding decision validation still proves that any present
decision belongs to the information-state perspective and revision.

- [ ] **Step 2: Mirror the local matrix in Python**

Use the same three branches in PlayerStepV2.validate() with Python's
self.submission.code, self.status.kind, self.next_decision, and
self.observed_events. Keep PlayerStepSubmissionV1's wire vocabulary unchanged.

- [ ] **Step 3: Repair the test-only rejected-step fixture**

crates/mtgml-conformance/src/isolation/paired.rs::rejected_submit_result()
currently creates InvalidAnswer with no decision. Add the same valid current
two-candidate request used by the PlayerStep tests and set it as
next_decision. This keeps the fixture a valid current-request rejection without
adding a new production endpoint meaning.

- [ ] **Step 4: Add two focused semantic negative fixtures**

Derive both fixtures from the checked-in player-step-v2-rejected.json bytes:

1. Remove next_decision while retaining submission.code = invalid_answer.
2. Retain next_decision while changing the submission code to
   unavailable_decision.

Register both with contract player-step.v2, expected error
semantic.player_step, and the existing shared semantic/decode rejection layer.
Keep the existing episode_closed running-status negative fixture; it remains a
fixture-only boundary test.

- [ ] **Step 5: Document the split and verify**

Add the exact local matrix to docs/ML_ENVIRONMENT.md:

~~~text
EpisodeClosed       -> non-Running, next_decision=None, observed_events=[]
UnavailableDecision -> Running,     next_decision=None, observed_events=[]
Stale/Invalid*      -> Running,     next_decision=Some(current actor request), observed_events=[]
~~~

State explicitly that the local validator can require decision presence and
actor/revision coherence but cannot prove equality with the pre-call product.
Set FND-016B = DEFER_TO_EVD_005; do not add an independent fingerprint
implementation here and do not claim EVD-005 closed.

Run:

~~~powershell
cargo test -p mtgml-observation --locked fnd_016a
cargo test -p mtgml-conformance --locked paired
cargo test -p mtgml-wire --locked every_shared_negative_fixture_is_rejected_with_the_expected_code
C:\Python313\python.exe -m pytest python/tests/test_batch_f.py python/tests/test_player_api.py -q
git diff --check
git add crates/mtgml-observation/src/player_step.rs crates/mtgml-observation/src/tests/batch_f.rs python/src/mtgml/observation.py python/tests/test_batch_f.py crates/mtgml-conformance/src/isolation/paired.rs docs/ML_ENVIRONMENT.md wire/negative/player-step-v2-rejection-missing-next-decision.json wire/negative/player-step-v2-unavailable-with-next-decision.json wire/negative/manifest.json
git commit -m "fix: close actor-bound PlayerStep rejection fields"
~~~

Expected: every FND-016A code row has an executable local acceptance/rejection
assertion. Environment before/after parity remains explicitly deferred.

## Task 6: Record FND-013, FND-027, and FND-028 decisions without new authority

**Files:**

- Modify: docs/DECISION_PROTOCOL.md
- Modify: docs/contracts/ENGINE_STATE_CLOSURE.md
- Modify: crates/mtgml-state/src/tests/batch_f.rs
- Modify: crates/mtgml-rules/src/tests/batch_f.rs
- Modify: crates/mtgml-environment/src/tests/batch_f.rs
- Create: docs/superpowers/specs/2026-09-14-player-id-zero-policy-adr-candidate.md
- Modify: docs/normative-document-register.v1.json

- [ ] **Step 1: Clarify FND-013 ownership in normative docs**

Add the following explicit split to docs/DECISION_PROTOCOL.md near exact
binding validation and to docs/contracts/ENGINE_STATE_CLOSURE.md near validation
ownership:

~~~text
AuthoritativeDecisionRequestV2::validate() owns local structural request
validity. validate_pending_authoritative_request() owns the exact
visible-to-trusted candidate binding, including scalar payload equality and
perspective resolver equality. project_player_request() projects a request
after that authoritative state boundary has passed; it is not a second binding
authority.
~~~

Do not change project_player_request() or expose any trusted field.

- [ ] **Step 2: Complete the FND-013 exact-binding evidence**

Run the state tests from Task 1 and verify the exact named controls. The state
cases must exercise:

~~~text
SelectPlayer(1) vs trusted SelectPlayer(2)
SelectMode(1) vs trusted SelectMode(2)
ChooseBoolean(true) vs trusted ChooseBoolean(false)
DeclareNumber(1) vs trusted DeclareNumber(2)
opaque object resolving to GameObjectId(1) vs trusted GameObjectId(2)
opaque ability resolving to AbilityInstanceId(1) vs trusted AbilityInstanceId(2)
~~~

For each mismatch, assert PendingDecisionMismatch and unchanged state. For
each matching pair, assert validate_engine_state() succeeds. Record
FND-013 = RESOLVED_ON_BASE.

- [ ] **Step 3: Complete the FND-027 Rules-owner regression**

Use the existing SemanticValidationCursor::lifecycle_identities_match()
behavior. Add negative cases for an extra/missing object mapping, retired-ID
mismatch, next opaque-object allocator mismatch, ability mapping mismatch, and
perspective-player-set mismatch. The cases may fail at either the state
validator or the lifecycle cursor, but every case must fail closed from
validate_transition_contract() before the projector callback can run.

Keep the test-only assertion focused on the existing owner. Do not add a final
identity comparison to project_occurrence_envelopes() and do not change the
intentional next_player_decision_id exclusion.

- [ ] **Step 4: Write the unnumbered FND-028 ADR candidate**

Create docs/superpowers/specs/2026-09-14-player-id-zero-policy-adr-candidate.md
with:

~~~text
Status: proposed, non-authoritative, blocks implementation of a global zero policy
Owner: architecture maintainers
Reviewers: model/state, replay, observation, and environment maintainers
Final ADR number: not allocated
~~~

Include the current matrix for parser/model, state maps, reset, active/priority,
owner/controller, decision actor, SelectPlayer, knowledge, perspective identity,
PlayerInformationState, observed events, Replay V3 manifest decks, Replay V3
step actor, environment binding, and schemas. Compare:

~~~text
A. zero valid everywhere;
B. zero forbidden at authoritative/player identity boundaries;
C. zero is a defined reserved sentinel.
~~~

The candidate must cite the contradictory current executable signals, preserve
historical fixtures, and state that no option is implemented by Batch F. Do not
allocate docs/adr/0050... or another final ADR number.

Register the candidate and the Batch-F plan in
docs/normative-document-register.v1.json only after the paths exist.

- [ ] **Step 5: Verify documentation and commit the decision records**

~~~powershell
C:\Python313\python.exe scripts/check_documentation.py
git diff --check
git add docs/DECISION_PROTOCOL.md docs/contracts/ENGINE_STATE_CLOSURE.md docs/superpowers/specs/2026-09-14-player-id-zero-policy-adr-candidate.md docs/normative-document-register.v1.json
git commit -m "docs: record Batch-F binding and PlayerId policy boundaries"
~~~

Expected: documentation links/register pass, no PlayerId parser or boundary
behavior changes, and FND-028 remains BLOCKED_CONTRACT_AMBIGUITY.

## Task 7: Build the bounded F4 integration evidence document

**Files:**

- Create: docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-f-dispositions-and-evidence.md
- Modify: docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-f-decision-observation-identity-design.md
- Modify: docs/normative-document-register.v1.json

- [ ] **Step 1: Map every required integration row to actual evidence**

The evidence document must include the exact test/fixture names and actual
result for these rows:

~~~text
1  valid decision request projection
2  scalar payload-mismatched trusted binding
3  resolver-backed object binding mismatch
4  candidate capacity rejection
5  valid dense assignment
6  no-op LifeChanged
7  no-op ObjectTapped
8  valid ObjectMoved old-only
9  valid ObjectMoved new-only
10 invalid ObjectMoved neither-visible
11 every actor-bound PlayerStep rejection code
12 lifecycle final identity mismatch rejected by transition contract
13 PlayerId zero behavior at each characterized boundary
14 accepted controls retain byte-compatible outputs
15 typed rejection remains nonmutating
~~~

Rows 2, 3, 6, 7, 10, 12, and 13 must cite exact closed evidence. Row 13 is
blocked as a policy decision even though current acceptance/rejection probes are
recorded. Row 11 must say FND-016A local closure and
FND-016B = DEFER_TO_EVD_005. The document must not claim any EVD item is
globally closed.

- [ ] **Step 2: Record dispositions and compatibility**

Use this exact disposition block:

~~~text
FND_009 = CONFIRMED
FND_013 = RESOLVED_ON_BASE
FND_014 = CONFIRMED
FND_015 = CONFIRMED
FND_016 = SPLIT_REQUIRED
FND_016A = CONFIRMED
FND_016B = DEFER_TO_EVD_005
FND_027 = RESOLVED_ON_BASE
FND_028 = BLOCKED_CONTRACT_AMBIGUITY
~~~

Record ADR_CANDIDATES as the unnumbered PlayerId-zero candidate and
ADR_ACCEPTED = NONE. Record FND-026B and FND-026C exactly as inherited from
Batch E. Register the evidence document after it exists.

- [ ] **Step 3: Update the approved design's evidence appendix**

Append the exact RED commit SHA, fix commit SHAs, focused test names, and final
dispositions to the design document. Change only evidence/status metadata; do
not rewrite its approved architecture.

- [ ] **Step 4: Commit the evidence documents**

~~~powershell
C:\Python313\python.exe scripts/check_documentation.py
git diff --check
git add docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-f-dispositions-and-evidence.md docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-f-decision-observation-identity-design.md docs/normative-document-register.v1.json
git commit -m "docs: record Batch-F dispositions and evidence"
~~~

## Task 8: Run the complete local verification matrix

**Files:** none beyond verification output outside the reproducible source
archive.

- [ ] **Step 1: Inspect exact source scope**

~~~powershell
git status --short
git diff --check
git diff --stat
git diff --cached --stat
git ls-files --others --exclude-standard
git diff --name-only b24bba153f2aa74bd59e8d6a872a0612ff7f76aa...HEAD
~~~

Every changed path must be one of the planned files. Any unrelated path is
removed or separately explained before continuing.

- [ ] **Step 2: Run the required affected package suites**

~~~powershell
cargo test -p mtgml-model --locked
cargo test -p mtgml-decision --locked
cargo test -p mtgml-state --locked
cargo test -p mtgml-rules --locked
cargo test -p mtgml-observation --locked
cargo test -p mtgml-environment --locked
cargo test -p mtgml-wire --locked
cargo test -p mtgml-conformance --locked
~~~

Record each package's exit code and test count separately.

- [ ] **Step 3: Run native Rust gates**

~~~powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
~~~

- [ ] **Step 4: Run Python, schema, wire, documentation, and maintainer gates**

Use the pinned interpreter and record the actual result of each command:

~~~powershell
C:\Python313\python.exe scripts/verify_repository.py
C:\Python313\python.exe scripts/check_rust_source_structure.py
C:\Python313\python.exe scripts/check_documentation.py
C:\Python313\python.exe scripts/validate_schemas.py
C:\Python313\python.exe scripts/validate_maintainer_artifacts.py
C:\Python313\python.exe scripts/verify_python_toolchain.py
C:\Python313\python.exe scripts/run_python_tests.py
C:\Python313\python.exe -m pytest python/tests/test_batch_f.py python/tests/test_wire_contracts.py python/tests/test_player_api.py -q
~~~

Run the applicable M2 decision/information/endpoint profiles:

~~~powershell
C:\Python313\python.exe scripts/run_m2_d_gates.py
C:\Python313\python.exe scripts/run_m2_e_gates.py
C:\Python313\python.exe scripts/run_m2_g_gates.py
C:\Python313\python.exe scripts/run_m2_h_gates.py
~~~

- [ ] **Step 5: Run wrapper checks with honest status handling**

~~~powershell
just check-fast
just check
~~~

If either command cannot start because WSL /bin/bash is unavailable, record
the corresponding key as BLOCKED; do not infer it from direct Cargo/Python
success. Record direct constituent profiles separately.

- [ ] **Step 6: Verify compatibility and final local cleanliness**

~~~powershell
rg -n "PlayerStepV3|CandidateIdV2|decision-response.v3|observed-event-envelope.v3|SystemTime|thread_rng|rand::" crates python schemas wire docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-f* || exit 1
git diff --check
git status --porcelain=v1
~~~

The search must not find a new version, clock/RNG dependency, or unrelated
scope. Existing historical references are classified, not deleted.

## Task 9: Independent exact-head code review

Before pushing the final branch, dispatch a read-only code reviewer with the
full final diff from base SHA b24bba153f2aa74bd59e8d6a872a0612ff7f76aa to the
current HEAD. The reviewer must inspect every changed Rust/Python/fixture/doc
path and return findings classified as blocker/major/minor/nit.

The review prompt must require explicit checks for:

~~~text
FND-009 event-owner-only no-op checks
FND-013 no resolver in project_player_request()
FND-014 one widened capacity rule in both Rust paths
FND-015 V1 unchanged and Rust/Python semantic parity
FND-016A matrix without EVD-005 or FND-026B semantics
FND-027 no duplicated projector identity authority
FND-028 no executed zero-policy guess or final ADR number
no privileged player exposure
no public/wire/schema/digest/RNG/historical replay changes
~~~

Do not interrupt the reviewer. Close it only after it reaches a final status.
Fix every blocker/major finding, rerun the affected tests, and review the new
head. A minor/nit is either fixed or recorded explicitly in the evidence
document; no review result is treated as verification by itself.

## Task 10: Push one branch, open one PR, and wait for hosted CI

- [ ] **Step 1: Final exact-head checks**

~~~powershell
git fetch origin
git rev-parse origin/master
git rev-parse HEAD
git status --porcelain=v1
git diff --check
cargo test --workspace --all-features --locked
~~~

origin/master must remain the actual clean current remote master and the final
evidence document must be part of the tested HEAD.

- [ ] **Step 2: Push the single branch**

~~~powershell
gh auth status
git push --set-upstream origin chris/pre-m3-remediation-batch-f-decision-observation-identity
git rev-parse HEAD
git ls-remote origin refs/heads/chris/pre-m3-remediation-batch-f-decision-observation-identity
~~~

The remote branch SHA must equal local HEAD exactly.

- [ ] **Step 3: Open exactly one PR against master**

Use the approved title:

~~~powershell
gh pr create --base master --head chris/pre-m3-remediation-batch-f-decision-observation-identity --title "Pre-M3 remediation Batch F: decision, observation, and identity hardening" --body-file .batch-f-pr-body.md
~~~

Create .batch-f-pr-body.md only for the PR operation and remove it after the
PR body has been accepted if it is not part of the planned source. The body
must include scope, exact dispositions, FND-013/FND-027 ownership, the FND-014
capacity rule, the FND-016A/B split, the FND-028 ADR candidate, all verification
statuses, compatibility fields, M3_AUTHORIZED = NO, and the statement that the
PR must not be merged by this task.

- [ ] **Step 4: Wait for exact-head hosted CI**

~~~powershell
$batchFPrNumber = gh pr view --json number --jq .number
gh pr checks $batchFPrNumber --watch
gh pr view $batchFPrNumber --json baseRefName,headRefName,headRefOid,statusCheckRollup,url
git rev-parse HEAD
~~~

Record HOSTED_CI = PASS only when all required checks for the exact PR head are
green. If any check fails or remains pending, record the actual status and do
not claim Batch F closure. Do not merge.

## Plan self-review checklist

- [ ] FND-009 has RED no-op tests, two minimal Rules-owner checks, and real mutation controls.
- [ ] FND-013 records structural decision validation, exact state-boundary validation, and resolver-free projection.
- [ ] FND-014 uses one widened u64 capacity calculation and covers both assign_dense() and validate_public() without a huge allocation.
- [ ] FND-015 changes V2 semantic validation only, updates Python parity, and leaves V1/schema/wire shape unchanged.
- [ ] FND-016A has all seven current-request rejection codes plus UnavailableDecision and fixture-only EpisodeClosed; FND-016B remains EVD-005-owned.
- [ ] FND-027 is evidence-only on the base and does not create a second projector identity authority.
- [ ] FND-028 has an unnumbered architecture-owned ADR candidate and no executable zero policy.
- [ ] FND-026B remains blocked and no neutral PlayerStep is introduced.
- [ ] Every production behavior change has a visible pre-fix RED or a documented RESOLVED_ON_BASE evidence path.
- [ ] No generated vocabulary, schema, public endpoint, digest, RNG, or historical replay change is included.
- [ ] Final workspace, wrapper, direct-profile, Python, schema, maintainer, M2, and hosted statuses are reported separately and only from executed evidence.
