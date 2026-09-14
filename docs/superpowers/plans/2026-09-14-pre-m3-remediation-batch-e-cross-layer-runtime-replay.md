# Pre-M3 Remediation Batch E Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Characterize and minimally close the Batch-E runtime, checkpoint,
replay, canonical-identity, and eventful-reprojection gaps while preserving
M2 semantics, historical replay meaning, and all existing public/wire/digest/RNG
contracts.

**Status:** active implementation plan

**Architecture:** Keep mtgml-state as the authoritative state owner,
mtgml-rules::validate_transition_contract as the one ordered semantic proof,
the environment as the complete precommit owner, and the existing production
lifecycle projector as the only observed-event projector. Add boundary-local
validation and replay-control application only where the current V3 contracts
already provide the required data. Leave non-actor PlayerStepV2 semantics and
live delivery explicitly unresolved/deferred.

**Tech Stack:** Rust workspace, Cargo locked tests, canonical V3 CBOR digests,
Serde DTOs, test-only M2.E fixture support, Python/schema/maintainer checks,
GitHub CLI, and GitHub Actions.

---

## File map

| File | Responsibility in this plan |
|---|---|
| crates/mtgml-state/src/tests.rs | Include the Batch-E state characterization tests. |
| crates/mtgml-state/src/tests/batch_e.rs | FND-007 staging and FND-008 state-family probes that belong to the state/rules boundary. |
| crates/mtgml-rules/src/tests.rs | Include the Batch-E rules characterization tests. |
| crates/mtgml-rules/src/tests/batch_e.rs | FND-008 reachable mutation-family matrix and FND-012B causality characterization. |
| crates/mtgml-environment/src/tests.rs | Include Batch-E environment tests and expose existing fixture helpers lexically. |
| crates/mtgml-environment/src/tests/batch_e.rs | FND-020, FND-022B, FND-025, FND-024, and environment-boundary RED/GREEN tests. |
| crates/mtgml-environment/src/replay.rs | Apply trusted external replay counters through the replay-owned checkpoint path. |
| crates/mtgml-environment/src/checkpoint.rs | Enforce V3 status order and exact EngineState player-universe closure. |
| crates/mtgml-environment/src/synthetic/replay.rs | Bind current synthetic producer schema/contract identities and canonicalize configuration decks before manifest identity. |
| crates/mtgml-environment/src/synthetic.rs | Add only the cfg(test) switch that routes the existing transaction pipeline through the eventful situation generator. |
| crates/mtgml-environment/src/synthetic/eventful.rs | Test-only eventful situation construction using FixtureTransition and production transition validation. |
| crates/mtgml-environment/src/replay_parity_tests.rs | FND-026D live/replay exact non-actor reprojection through production projection. |
| crates/mtgml-replay/src/validation.rs | Add precise V3 validation categories for keyed-array and player-universe failures. |
| crates/mtgml-replay/src/v3.rs | Enforce V3 manifest/deck/status boundaries and document structural validation. |
| crates/mtgml-persistence/src/checkpoint_digest.rs | Preserve the accepted defensive PlayerOutcome sort and known-answer bytes. |
| docs/REPLAY_AND_DETERMINISM.md | Clarify detached structural versus backend-verified replay evidence. |
| docs/ML_ENVIRONMENT.md | Clarify actor-bound PlayerStepV2 and deferred non-actor delivery. |
| docs/STATE_HASHING.md | Clarify that V3 boundary validation owns canonical input order while the digest helper retains defensive sorting. |
| docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-e-cross-layer-runtime-replay-design.md | Add final evidence-backed dispositions and exact gate results. |
| docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-e-dispositions-and-evidence.md | Record the required final Batch-E disposition matrix and PR evidence. |

No JSON schema, wire DTO, digest domain, RNG implementation, historical replay
fixture, public endpoint method, queue, mailbox, or M3 file is changed.

## Task 1: Reconfirm exact authority and clean baseline

**Files:** None.

- [ ] Verify the implementation branch and source identity before any test edit:

~~~powershell
git fetch origin
git rev-parse origin/master
git rev-parse HEAD
git status --porcelain=v1
git branch --show-current
~~~

Expected result: origin/master and the branch starting point both resolve to
9996cfd0fcd4ef67d98cb0422611cebabd20e46b; after the approved design and plan
commits, verify this with git merge-base HEAD origin/master. The worktree is
clean and the branch is chris/pre-m3-remediation-batch-e-cross-layer-closure.

- [ ] Re-read the approved Batch-E design and the active normative sources after
  this plan is loaded:

~~~text
docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-e-cross-layer-runtime-replay-design.md
docs/RULES_SEMANTICS.md
docs/EXECUTION_MODEL.md
docs/INFORMATION_MODEL.md
docs/REPLAY_AND_DETERMINISM.md
docs/contracts/ENGINE_STATE_CLOSURE.md
docs/ML_ENVIRONMENT.md
docs/STATE_HASHING.md
docs/DECISION_PROTOCOL.md
docs/maintenance/API_LIFECYCLE.md
docs/contracts/COMPATIBILITY_POLICY.md
docs/adr/0035-deterministic-hmac-sha256-counter-rng.md
docs/adr/0039-perspective-local-decision-identity-and-typed-staged-choices.md
docs/adr/0040-m2-information-lifecycle-and-v3-state-identity.md
docs/adr/0041-capability-oriented-semantic-domains-and-explicit-semantic-ownership.md
docs/adr/0049-knowledge-chronology-and-ordered-zone-canonicality.md
~~~

- [ ] Run the clean baseline again and preserve its exact output in the task
  evidence notes:

~~~powershell
cargo test --workspace --all-features --locked
~~~

Expected result: exit code 0; no failure is attributed to Batch E.

## Task 2: Add E1 characterization tests before production changes

**Files:**

- Create: crates/mtgml-state/src/tests/batch_e.rs
- Modify: crates/mtgml-state/src/tests.rs
- Create: crates/mtgml-rules/src/tests/batch_e.rs
- Modify: crates/mtgml-rules/src/tests.rs
- Modify: crates/mtgml-environment/src/tests.rs

### Step 1: Characterize the lifecycle staging boundary

- [ ] Add a state test that applies a valid-sequence UpdateLocation to a
  declared location that does not match the current live object location:

~~~rust
#[test]
fn fnd_007_lifecycle_seam_can_stage_before_physical_state_completion() {
    let mut state = synthetic_state();
    let before = state.clone();
    let audit = PerspectiveLifecycleAuditV1 {
        perspective: PlayerId(1),
        sequence: VisibleSequence(1),
        mutation: PerspectiveLifecycleMutationV1 {
            identity: IdentityMutationV1::None,
            knowledge: Some(KnowledgeMutationV1::UpdateLocation {
                opaque: OpaqueObjectId(1),
                fact: KnownLocationFactV2 {
                    location: ZoneLocation {
                        zone: ZoneKind::Hand,
                        player: Some(PlayerId(1)),
                        position: ZonePosition::Unordered,
                        visibility: VisibilityPartition::OwnerOnly,
                        partition: None,
                    },
                    provenance: KnowledgeAcquisitionReason::Observed {
                        channel: KnowledgeHistoryChannel::Public,
                        sequence: VisibleSequence(1),
                        cause: KnowledgeAcquisitionCause::PublicEvent,
                    },
                },
            }),
        },
    };

    assert_eq!(
        apply_perspective_lifecycle(&mut state, &audit),
        Ok(()),
        "the lower-level lifecycle seam is a staging primitive"
    );
    assert_ne!(state, before);
    assert_eq!(
        validate_engine_state(&state),
        Err(EngineStateViolation::KnowledgeMismatch)
    );
}
~~~

This is characterization, not a production RED claim. It proves why full state
validation must remain at the complete transition owner: the existing fixture
ordering performs the physical transition before the lifecycle occurrence.

- [ ] Add the paired environment/rules test using the existing
  tracked_incarnation_product() and FixtureTransition helpers. Assert that the
  complete accepted product validates, its physical and lifecycle events are
  ordered, and the resulting state passes validate_engine_state():

~~~rust
#[test]
fn fnd_007_atomic_owner_closes_the_staged_lifecycle_product() {
    let (before, result) = tracked_incarnation_product().unwrap();
    validate_engine_state(&result.next_state).unwrap();
    mtgml_rules::validate_transition_contract(&before, &result).unwrap();
}
~~~

Record FND-007 as RESOLVED_ON_BASE only if this paired evidence confirms that
no production caller commits the standalone staged state. Otherwise keep the
escaping path as a separate confirmed finding and stop only that slice.

### Step 2: Characterize reachable FND-008 mutation families

- [ ] Add a rules test table that exercises the existing transition-contract
  owner with these accepted-product families and records the expected outcome:

| Family | Probe | Expected current disposition |
|---|---|---|
| revision | Existing revision jump negative plus exact +1 accepted control. | Exact +1 only; jump rejected. |
| player life | Existing two LifeChanged events. | Event/cursor owned. |
| object/location/tapped | Existing snapshot and tap cursor checks. | Event/cursor owned. |
| decision/continuation | Existing create/clear and continuation tests. | Event/cursor or progression owned. |
| RNG | Existing sampled value/cursor proof. | RNG event/cursor owned. |
| identity allocators | Existing rewind/reuse tests. | Progression owned. |
| perspective knowledge/identity | Existing lifecycle occurrence cursor. | Lifecycle event owned. |
| has_lost, active/priority player, turn number | One mutation per field with exact delta and no event. | Rejected as unexplained. |
| stack/effects/triggers | Valid-state control plus attempted M2 mutation. | Unreachable/unsupported in current foundation. |
| format state | Valid Commander structural control plus attempted semantic mutation. | Unreachable/contract-bound in current foundation. |
| player-universe shape | Add/remove a core player while preserving other fields. | Rejected by state/transition closure. |

- [ ] Use the existing helper shape for every mutation-specific negative:

~~~rust
fn assert_fnd_008_rejected(before: &EngineState, after: EngineState) {
    let result = accepted_product_for_contract(before, after, Vec::new());
    assert_contract_rejects_without_mutation(before, &result);
}
~~~

- [ ] Do not add StateChanged, turn, priority, stack, trigger, format, or
  placeholder Magic events. If a family cannot be probed without inventing
  M3 semantics, record it as RESOLVED_ON_BASE /
  NOT_REACHABLE_CURRENT_FOUNDATION or BLOCKED_CONTRACT_AMBIGUITY in the
  evidence document.

### Step 3: Characterize FND-012B independently of RNG

- [ ] Add the arbitrary-code control and empty-code negative to the rules
  Batch-E tests:

~~~rust
#[test]
fn fnd_012b_announced_outcome_is_its_separate_presentation_occurrence() {
    let (before, result) = outcome_occurrence_product(
        PerspectiveObservationPolicyV1::AnnouncedOutcome {
            code: "arbitrary-public-code".into(),
        },
    );
    validate_transition_contract(&before, &result).unwrap();
}

#[test]
fn fnd_012b_empty_announced_outcome_fails_closed() {
    let (before, result) = outcome_occurrence_product(
        PerspectiveObservationPolicyV1::AnnouncedOutcome { code: String::new() },
    );
    assert_contract_rejects_without_mutation(&before, &result);
}
~~~

The accepted result is separate from the preceding random-result proof. No
production change is authorized for FND-012B if these controls confirm the
current contract; record it as REJECTED.

### Step 4: Run characterization evidence and commit

- [ ] Run the tests before production changes:

~~~powershell
cargo test -p mtgml-state --all-features --locked fnd_007
cargo test -p mtgml-rules --all-features --locked fnd_008
cargo test -p mtgml-rules --all-features --locked fnd_012b
~~~

The characterization tests may pass because they describe BASE behavior. Any
test intended to prove a confirmed fix must be added in Task 3 or later and
must visibly fail on BASE before the fix.

- [ ] Commit the characterization evidence:

~~~powershell
git add crates/mtgml-state/src/tests.rs crates/mtgml-state/src/tests/batch_e.rs crates/mtgml-rules/src/tests.rs crates/mtgml-rules/src/tests/batch_e.rs
git commit -m "tests: characterize Batch-E runtime closure"
~~~

## Task 3: Add E2 RED tests for producer identity, status universe, and keyed arrays

**Files:**

- Create: crates/mtgml-environment/src/tests/batch_e.rs
- Modify: crates/mtgml-environment/src/tests.rs
- Modify: crates/mtgml-replay/src/tests.rs

- [ ] Add the producer identity RED test. Start from config([PlayerId(1),
  PlayerId(2)]), replace only schemas.observation with
  observation-envelope.v999, and assert the desired result is a closed
  ReplayIdentityMismatch. On BASE this must fail because construction currently
  succeeds:

~~~rust
#[test]
fn fnd_020_current_producer_rejects_a_false_observation_schema_identity() {
    let mut invalid = config([PlayerId(1), PlayerId(2)]);
    invalid.replay.schemas.observation = "observation-envelope.v999".into();
    assert!(matches!(
        SyntheticM1EnvironmentBackend::new(
            [PlayerId(1), PlayerId(2)],
            seed(),
            invalid,
        ),
        Err(ControllerError::ReplayIdentityMismatch)
    ));
}
~~~

- [ ] Add the detached-reader compatibility control in mtgml-replay by
  changing only the nonempty observation identity in a structurally valid V3
  manifest and asserting manifest.validate().is_ok(). This control must stay
  green after the producer guard: detached validation is not a current
  implementation probe.

- [ ] Add the FND-022B checkpoint RED matrix over a two-player state:

~~~rust
#[test]
fn fnd_022b_checkpoint_requires_the_exact_authoritative_player_universe() {
    let (state, _) = two_perspective_outcome_product();
    let codec = CheckpointCodecIdentity {
        codec_id: "synthetic-m2-memory".into(),
        semantic_version: "3".into(),
    };
    for players in [
        Vec::new(),
        vec![PlayerOutcome { player: PlayerId(999), result: PlayerResult::Win }],
        vec![
            PlayerOutcome { player: PlayerId(1), result: PlayerResult::Win },
            PlayerOutcome { player: PlayerId(1), result: PlayerResult::Loss },
        ],
    ] {
        let status = EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players,
        };
        assert!(
            EnvironmentCheckpointV3::new(
                state.clone(),
                status,
                EnvironmentLimitCounters::default(),
                codec.clone(),
            )
            .is_err()
        );
    }
    assert!(EnvironmentCheckpointV3::new(
        state,
        EpisodeStatus::Terminal {
            reason: TerminalReason::Concession,
            players: vec![
                PlayerOutcome { player: PlayerId(1), result: PlayerResult::Win },
                PlayerOutcome { player: PlayerId(2), result: PlayerResult::Loss },
            ],
        },
        EnvironmentLimitCounters::default(),
        codec,
    ).is_ok());
}
~~~

- [ ] Add the V3 replay status-universe control against the manifest deck set.
  Build a valid two-deck ReplayManifestV3, set the initial status to an empty
  terminal outcome, recompute its checkpoint digest, and assert
  manifest.validate().is_err(). The test must also cover a foreign player and
  an exact two-player outcome.

- [ ] Add FND-025 RED cases:

  - an exact two-player checkpoint status whose players array is [2, 1];
  - a V3 manifest whose decks array is [PlayerId(2), PlayerId(1)];
  - a V3 replay step whose closed status outcome array is [2, 1];
  - duplicate deck and duplicate status keys as controls;
  - direct calls to calculate_checkpoint_digest_v3() for [1, 2] and [2, 1]
    that remain equal, proving the digest helper's defensive sort is still the
    accepted behavior.

Use is_err() for the new boundary RED assertions until the new error variants
exist. Do not change EpisodeStatus::validate() in this task.

- [ ] Run only the new boundary tests and record the BASE failures:

~~~powershell
cargo test -p mtgml-environment --all-features --locked fnd_020
cargo test -p mtgml-environment --all-features --locked fnd_022b
cargo test -p mtgml-environment --all-features --locked fnd_025
cargo test -p mtgml-replay --all-features --locked fnd_020
cargo test -p mtgml-replay --all-features --locked fnd_025
~~~

- [ ] Commit the visible RED characterization:

~~~powershell
git add crates/mtgml-environment/src/tests.rs crates/mtgml-environment/src/tests/batch_e.rs crates/mtgml-replay/src/tests.rs
git commit -m "tests: add Batch-E replay and checkpoint RED cases"
~~~

## Task 4: Implement FND-020, FND-022B, and FND-025 minimally

**Files:**

- Modify: crates/mtgml-environment/src/synthetic/replay.rs
- Modify: crates/mtgml-environment/src/checkpoint.rs
- Modify: crates/mtgml-replay/src/validation.rs
- Modify: crates/mtgml-replay/src/v3.rs
- Do not modify: crates/mtgml-model/src/lib.rs
- Do not modify: crates/mtgml-persistence/src/checkpoint_digest.rs except to
  add a test that locks the existing sort and KAT bytes.

### Step 1: Guard current producer identities

- [ ] Add one producer-owned helper in synthetic/replay.rs:

~~~rust
fn current_v3_schema_versions() -> ReplaySchemaVersionsV1 {
    ReplaySchemaVersionsV1 {
        observation: mtgml_observation::OBSERVATION_SCHEMA.into(),
        information_state: mtgml_observation::INFORMATION_STATE_SCHEMA_V2.into(),
        decision: mtgml_decision::PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
        decision_response: mtgml_decision::DECISION_RESPONSE_V2_SCHEMA.into(),
        observed_event: mtgml_observation::OBSERVED_EVENT_SCHEMA_V2.into(),
        player_step: mtgml_observation::PLAYER_STEP_SCHEMA_V2.into(),
        replay_step: mtgml_replay::REPLAY_STEP_SCHEMA_V3.into(),
    }
}

fn validate_current_producer_identity(
    config: &SyntheticM1EnvironmentConfig,
) -> Result<(), ControllerError> {
    if config.replay.randomness_contract_id != mtgml_random::MTGML_RNG_V1
        || config.replay.schemas != current_v3_schema_versions()
    {
        return Err(ControllerError::ReplayIdentityMismatch);
    }
    Ok(())
}
~~~

- [ ] Call this helper at the beginning of build_manifest(). Keep
  ReplayManifestV3::validate()'s detached observation identity behavior
  unchanged. Sort a cloned configuration deck list by DeckIdentityV1.player
  before inserting it into the manifest; never normalize an already persisted
  manifest.

- [ ] Run the FND-020 producer RED test to GREEN and the detached-reader
  control again. The producer must reject a false current identity; the
  detached manifest may still validate its nonempty identity.

- [ ] Commit:

~~~powershell
git add crates/mtgml-environment/src/synthetic/replay.rs
git commit -m "fix: bind current replay producer identities"
~~~

### Step 2: Add V3 boundary-local status validation

- [ ] Add NoncanonicalStatusOrder and StatusPlayerUniverse to
  CheckpointValidationError, then add a private checkpoint helper:

~~~rust
fn validate_v3_status_for_players(
    status: &EpisodeStatus,
    expected: &BTreeSet<PlayerId>,
) -> Result<(), CheckpointValidationError> {
    let outcomes = match status {
        EpisodeStatus::Running => return Ok(()),
        EpisodeStatus::Terminal { players, .. }
        | EpisodeStatus::Truncated { players, .. } => players,
    };
    if outcomes.windows(2).any(|window| window[0].player >= window[1].player) {
        return Err(CheckpointValidationError::NoncanonicalStatusOrder);
    }
    let actual = outcomes.iter().map(|outcome| outcome.player).collect();
    if actual != *expected {
        return Err(CheckpointValidationError::StatusPlayerUniverse);
    }
    Ok(())
}
~~~

- [ ] In EnvironmentCheckpointV3::validate(), validate the state first, call
  self.status.validate() for local duplicate/enum checks, construct
  state.core.players.keys().copied().collect::<BTreeSet<_>>(), and call the
  boundary helper before accepting the checkpoint. Leave the shared model
  validator unchanged.

- [ ] Update existing valid terminal/truncated checkpoint tests that used an
  empty outcome list to use the exact state player set. Keep intentionally
  corrupted checkpoints corrupted by mutating their status/digest after a
  valid checkpoint is built.

### Step 3: Add V3 replay keyed-array and player-universe validation

- [ ] Add precise replay errors:

~~~rust
#[error("V3 replay keyed array is not in canonical order")]
NoncanonicalKeyOrder,
#[error("V3 replay status does not cover the manifest player universe")]
StatusPlayerUniverse,
~~~

- [ ] In ReplayManifestV3::validate(), iterate decks in input order and reject
  player >= next.player as NoncanonicalKeyOrder; retain duplicate detection.
  Build the manifest player set and validate the initial closed status against it
  with a replay-private helper that checks ascending order and exact set.

- [ ] In AuthoritativeReplayV3::validate(), apply the same status boundary
  helper to every episode_status_after using the manifest deck-player set.
  Keep InitialEnvironmentIdentityV3::validate() independent of a player
  universe because it has no state/deck context.

- [ ] Preserve the existing player_outcomes_value() sort in
  mtgml-persistence/src/checkpoint_digest.rs. Add a test that direct digest
  helper calls with [1, 2] and [2, 1] remain byte-identical and that the
  canonical checkpoint boundary rejects [2, 1] before it becomes authoritative.

- [ ] Run the FND-022B/FND-025 focused tests to GREEN:

~~~powershell
cargo test -p mtgml-environment --all-features --locked fnd_022b
cargo test -p mtgml-environment --all-features --locked fnd_025
cargo test -p mtgml-replay --all-features --locked fnd_025
cargo test -p mtgml-persistence --all-features --locked checkpoint_digest
~~~

- [ ] Commit:

~~~powershell
git add crates/mtgml-environment/src/checkpoint.rs crates/mtgml-replay/src/validation.rs crates/mtgml-replay/src/v3.rs crates/mtgml-persistence/src/checkpoint_digest.rs crates/mtgml-environment/src/tests.rs crates/mtgml-environment/src/tests/batch_e.rs crates/mtgml-replay/src/tests.rs
git commit -m "fix: close V3 status and keyed-array boundaries"
~~~

## Task 5: Add FND-022E RED evidence and apply trusted external replay counters

**Files:**

- Modify: crates/mtgml-environment/src/tests/batch_e.rs
- Modify: crates/mtgml-environment/src/replay.rs

- [ ] Add the RED test from a real one-step live replay. Increase only
  resource_units_consumed and wall_clock_elapsed_millis, reseal the step's
  checkpoint digest and final identity, and assert that the desired replay
  result is successful with the recorded external values. On BASE this fails
  with AfterDigestMismatch because the executor currently carries the old
  values forward.

- [ ] Add controls for:

  - backward external progression, rejected by structural replay validation;
  - accepted deterministic counter progression, still exact +1/+events;
  - accepted trusted execution rejection, preserving every counter and
    checkpoint field;
  - source-controller checkpoint/replay equality after a replay with external
    progression;
  - no host-clock dependency, backed by a source search with no SystemTime,
    Instant::now, or equivalent read in the replay executor.

- [ ] Replace the single expected_counters() result with a helper that keeps
  deterministic fields exact and adopts only the two explicit replay-control
  fields:

~~~rust
fn deterministic_counters(
    before: &EnvironmentCheckpointV3,
    transition: &mtgml_rules::TransitionResult,
) -> Result<EnvironmentLimitCounters, ControllerError> {
    let event_count = u64::try_from(transition.events.len()).map_err(|_| {
        ControllerError::CounterOverflow {
            counter: "rule_events_emitted",
        }
    })?;
    Ok(EnvironmentLimitCounters {
        decisions_submitted: checked_counter_add(
            before.limit_counters.decisions_submitted,
            1,
            "decisions_submitted",
        )?,
        accepted_transitions: checked_counter_add(
            before.limit_counters.accepted_transitions,
            1,
            "accepted_transitions",
        )?,
        rule_events_emitted: checked_counter_add(
            before.limit_counters.rule_events_emitted,
            event_count,
            "rule_events_emitted",
        )?,
        resource_units_consumed: before.limit_counters.resource_units_consumed,
        wall_clock_elapsed_millis: before.limit_counters.wall_clock_elapsed_millis,
    })
}
~~~

~~~rust
fn expected_counters(
    before: &EnvironmentCheckpointV3,
    transition: &mtgml_rules::TransitionResult,
    recorded: &EnvironmentLimitCounters,
    step_index: u64,
) -> Result<EnvironmentLimitCounters, ControllerError> {
    if !transition.accepted {
        return Ok(before.limit_counters.clone());
    }
    let mut expected = deterministic_counters(before, transition)?;
    if recorded.resource_units_consumed < before.limit_counters.resource_units_consumed
        || recorded.wall_clock_elapsed_millis
            < before.limit_counters.wall_clock_elapsed_millis
    {
        return Err(ReplayExecutionError::CounterMismatch { step_index }.into());
    }
    expected.resource_units_consumed = recorded.resource_units_consumed;
    expected.wall_clock_elapsed_millis = recorded.wall_clock_elapsed_millis;
    Ok(expected)
}
~~~

- [ ] After the trusted response executes and its deterministic transition/status
  validates, create a candidate checkpoint with the actual executed state/status
  and the recorded counter set. If external values differ from the executed
  checkpoint, apply that candidate through backend.restore(candidate) on the
  replay-owned backend. Then call checkpoint(backend) again, recompute the
  checkpoint identity, and compare all recorded after fields. This reuses the
  existing trusted checkpoint mutation boundary and introduces no counter setter
  or hidden state.

- [ ] Reject a recorded status that differs from the transition's already-defined
  status before applying external counters. A deterministic accepted transition
  whose own status is Truncated remains a normal committed replay step; an
  external-counter-driven status change without a threshold contract fails
  closed as unsupported.

- [ ] Run the focused tests to GREEN:

~~~powershell
cargo test -p mtgml-environment --all-features --locked fnd_022e
cargo test -p mtgml-environment --all-features --locked replay
cargo test -p mtgml-replay --all-features --locked counter
~~~

- [ ] Commit:

~~~powershell
git add crates/mtgml-environment/src/replay.rs crates/mtgml-environment/src/errors.rs crates/mtgml-environment/src/tests/batch_e.rs
git commit -m "fix: apply trusted external replay counters"
~~~

## Task 6: Make FND-023 and FND-024 evidence explicit

**Files:**

- Modify: crates/mtgml-replay/src/v3.rs
- Modify: crates/mtgml-environment/src/tests/batch_e.rs
- Modify: crates/mtgml-environment/src/tests/checkpoint_replay.rs if the
  existing resealed replay test is the clearest location.
- Modify: docs/REPLAY_AND_DETERMINISM.md
- Modify: docs/ML_ENVIRONMENT.md

### Step 1: Document and test FND-023's two trust levels

- [ ] Add a doc comment to AuthoritativeReplayV3::validate() stating that it
  is detached structural validation only. Name the exact proofs it provides:
  schema/local shape, canonical order, manifest/deck consistency, internal
  identity-chain shape, and DTO-level counter/revision rules.

- [ ] Add a doc comment to execute_replay_from_checkpoint() and
  ReplayExecutionReport stating that backend/checkpoint verification proves
  pending actor/request binding, authoritative execution, after-state identity,
  counter application, and transition parity.

- [ ] Extend the existing tampered/resealed replay test with:

~~~rust
tampered.validate().unwrap();
let result = fresh.execute_replay_from_checkpoint(c0, tampered);
assert!(matches!(
    result,
    Err(ControllerError::ReplayExecution(
        ReplayExecutionError::AfterDigestMismatch { .. }
    ))
));
~~~

This proves a structurally valid replay can fail backend verification and that
the source controller remains unchanged.

- [ ] Add the same distinction to REPLAY_AND_DETERMINISM.md without changing
  V3 wire shape or historical support classification.

### Step 2: Characterize the complete FND-024 rejection matrix

- [ ] Add one environment test table covering:

| Layer | Test input | Expected replay | Expected mutation |
|---|---|---:|---|
| A | Malformed/noncanonical wire bytes through submit_response_bytes. | None. | No typed submit, no state/status/counter/replay mutation. |
| B | Stale response, invalid candidate, wrong answer variant, unavailable decision, closed endpoint. | None. | Typed rejected actor PlayerStepV2; state/status/counters/replay unchanged. |
| C | Invalid typed response through execute_trusted_response. | Only if explicitly hand-recorded as a diagnostic step. | accepted=false; complete checkpoint identity unchanged. |
| D1 | Accepted transition whose defined deterministic result is Truncated. | Yes. | Committed after status/counters are recorded. |
| D2 | Service/internal failure before commit. | No. | Candidate discarded; closed service failure. |
| D3 | External-counter-driven truncation with no reviewed threshold. | No successful step. | Fail closed; no invented status/outcomes. |

- [ ] Reuse existing real endpoints and replay recorder assertions. Do not add a
  player rejection to live accepted replay history, do not advance
  decisions_submitted for Layer B, and do not label any result as trajectory
  data. Keep the layer-C diagnostic replay control and its complete identity
  preservation.

- [ ] Update ML_ENVIRONMENT.md with the actor-bound submission meaning and the
  no-live-non-actor-delivery foundation boundary. Update the Batch-E design
  spec's evidence section with the exact test names and BASE/GREEN results.

- [ ] Commit:

~~~powershell
git add crates/mtgml-replay/src/v3.rs crates/mtgml-environment/src/tests/batch_e.rs crates/mtgml-environment/src/tests/checkpoint_replay.rs docs/REPLAY_AND_DETERMINISM.md docs/ML_ENVIRONMENT.md
git commit -m "docs: define Batch-E replay trust and rejection boundaries"
~~~

## Task 7: Add the test-only eventful situation and FND-026D exact reprojection

**Files:**

- Create: crates/mtgml-environment/src/synthetic/eventful.rs
- Modify: crates/mtgml-environment/src/synthetic.rs
- Modify: crates/mtgml-environment/src/replay_parity_tests.rs

### Step 1: Establish a semantic RED before enabling the fixture mode

- [ ] Add a test in replay_parity_tests.rs that uses the current ordinary
  backend and asserts that the eventful replay batch is nonempty. Run it and
  preserve the expected BASE failure: the current synthetic accepted path has
  no PerspectiveOccurrence, so the production projector returns empty batches.
  Do not call this a production defect; it is the missing evidence setup for
  FND-026D.

### Step 2: Build the test-only eventful situation generator

- [ ] Add cfg(test) mod eventful; to synthetic.rs and a
  cfg(test) eventful_fixture: bool field initialized to false by normal
  constructors. Preserve this flag through fork_boxed() and restore() so replay
  execution on the internal fork uses the same test-only situation.

- [ ] In synthetic/eventful.rs, add a factory that starts with
  construct_synthetic_engine_state(), retains its valid pending entry decision,
  adds GameObjectId(3) and GameObjectId(4) in public Exile with unique
  physical/card identities, advances next_object_id to 5, rebuilds the normal
  V3 checkpoint/manifest, and sets eventful_fixture = true.

- [ ] Add an eventful transition function that uses FixtureTransition only to
  create one situation:

  1. Reserve the first rule-event ID for DecisionCleared by running the fixture
     on a clone with the pending decision removed and the reserved cursor.
  2. Move object 3 to public Battlefield, creating a fresh incarnation.
  3. Add a P1 Appeared occurrence with Allocate + Acquire and explicit public
     provenance.
  4. Add a P2 AnnouncedOutcome occurrence with code p2-public and no lifecycle
     mutation.
  5. Prepend the DecisionCleared event for the original pending decision.
  6. Rebuild the StateDelta from the original before state to the fixture after
     state with all event audit operations.
  7. Return a normal accepted TransitionResult and run the production
     validate_transition_contract() before it reaches environment commit.

The eventful function is a test-only situation generator, not a second
projector or a new rules authority. It must use
apply_perspective_lifecycle, FixtureTransition, StateDelta::between, and the
production transition validator.

- [ ] In the existing environment transaction pipeline, select the eventful
  transition producer only under cfg(test) && self.eventful_fixture. The normal
  commit pipeline must remain unchanged: checkpoint, transition validation,
  rejection branch, candidate counters/checkpoint, replay append/export,
  production occurrence projection, actor PlayerStep validation, and commit.

### Step 3: Add the exact live/replay reprojection test

- [ ] Add eventful_replay_reprojects_both_perspectives_byte_exactly() in
  replay_parity_tests.rs:

~~~rust
let controller = TrustedEnvironmentController::new(eventful_backend());
let cp0 = controller.checkpoint().unwrap();
let live_transition = controller
    .execute_trusted_response(PlayerId(1), response(0, 0))
    .unwrap();
let live_after = controller.checkpoint().unwrap();
let replay = controller.export_replay().unwrap();

let live_events = crate::lifecycle_projection::project_occurrence_envelopes(
    &cp0.state,
    &live_after.state,
    &live_transition.events,
).unwrap();
assert!(!live_events[&PlayerId(1)].is_empty());
assert!(!live_events[&PlayerId(2)].is_empty());

let mut live_step = SyntheticM1EnvironmentBackend::player_step_from_state(
    &live_after.state,
    PlayerId(1),
    live_transition.status.clone(),
    PlayerStepSubmissionV1::Accepted,
).unwrap();
live_step.observed_events = live_events[&PlayerId(1)].clone();
live_step.validate().unwrap();

let report = controller
    .execute_replay_from_checkpoint(cp0.clone(), replay.clone())
    .unwrap();
let trace = &report.traces[0];
let replay_events = crate::lifecycle_projection::project_occurrence_envelopes(
    &trace.before.state,
    &trace.after.state,
    &trace.transition.events,
).unwrap();

for player in [PlayerId(1), PlayerId(2)] {
    assert_eq!(
        mtgml_wire::encode_canonical(&replay_events[&player]).unwrap(),
        mtgml_wire::encode_canonical(&live_events[&player]).unwrap(),
    );
}
let mut replay_step_for_actor = SyntheticM1EnvironmentBackend::player_step_from_state(
    &trace.after.state,
    PlayerId(1),
    trace.after.status.clone(),
    PlayerStepSubmissionV1::Accepted,
).unwrap();
replay_step_for_actor.observed_events = replay_events[&PlayerId(1)].clone();
replay_step_for_actor.validate().unwrap();
assert_eq!(
    mtgml_wire::encode_canonical(&replay_step_for_actor).unwrap(),
    mtgml_wire::encode_canonical(&live_step).unwrap(),
);
assert_ne!(
    mtgml_wire::encode_canonical(&live_events[&PlayerId(1)]).unwrap(),
    mtgml_wire::encode_canonical(&live_events[&PlayerId(2)]).unwrap(),
);
assert_eq!(controller.checkpoint().unwrap(), live_after);
assert_eq!(controller.export_replay().unwrap(), replay);
~~~

The final test must capture the real returned TransitionResult events rather
than use a second event list. Rebuild the actor step through
SyntheticM1EnvironmentBackend::player_step_from_state() and attach the
production projector output. Assert P1's ObjectMoved contains only opaque IDs,
P2's PublicOutcome contains only its public code, and serialized products
contain no GameObjectId, PhysicalCardId, DecisionId, RuleEventId, RNG seed/key/
cursor/raw words, checkpoint digest, or hidden identity/order.

- [ ] Run the production eventful reprojection test and the existing parity suite:

~~~powershell
cargo test -p mtgml-environment --all-features --locked eventful_replay_reprojects_both_perspectives_byte_exactly
cargo test -p mtgml-environment --all-features --locked replay_parity
cargo test -p mtgml-environment --all-features --locked information_projection
~~~

- [ ] Commit:

~~~powershell
git add crates/mtgml-environment/src/synthetic.rs crates/mtgml-environment/src/synthetic/eventful.rs crates/mtgml-environment/src/replay_parity_tests.rs
git commit -m "tests: prove eventful replay perspective reprojection"
~~~

## Task 8: Record FND-026B/FND-026C and bounded E4 disposition

**Files:**

- Modify: docs/ML_ENVIRONMENT.md if the actor-bound delivery clarification
  was not completed in Task 6.
- Modify: docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-e-cross-layer-runtime-replay-design.md
- Create: docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-e-dispositions-and-evidence.md

- [ ] Record FND-026B exactly as BLOCKED_CONTRACT_AMBIGUITY with this unanswered
  question: what neutral PlayerStepV2.submission value, or what separately
  versioned product, represents a non-actor perspective's complete eventful
  step? Explicitly record that existing envelope validation is closed, but
  complete non-actor PlayerStepV2 validation is not claimed.

- [ ] Record FND-026C exactly as DEFERRED_P2: current foundation endpoint calls
  are actor-bound; no live non-actor delivery is promised; events remain
  available through internal production projection for replay/trajectory
  derivation; no queue/mailbox/polling/callback/controller-global mutable state
  is introduced.

- [ ] Record FND-026D as evidence closed by the eventful exact-byte test, without
  closing EVD-010 globally.

- [ ] Add the bounded E4 matrix as a documentation/evidence table mapping each
  required scenario to an actual test. Mark deterministic truncation and trusted
  accepted=false according to reachability; mark external-counter status
  crossing unsupported; mark complete non-actor PlayerStep validation blocked by
  the exact FND-026B contract question. Do not claim E4 PASS if the E3 section
  gate remains blocked.

- [ ] Commit:

~~~powershell
git add docs/ML_ENVIRONMENT.md docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-e-cross-layer-runtime-replay-design.md docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-e-dispositions-and-evidence.md
git commit -m "docs: record Batch-E multi-perspective dispositions"
~~~

## Task 9: Run focused and workspace verification

**Files:** None, except verification output outside the reproducible source
archive as defined by the repository.

- [ ] Inspect the complete tracked/untracked scope and both staged/unstaged
  diffs:

~~~powershell
git status --short
git diff --check
git diff --stat
git diff --cached --stat
git ls-files --others --exclude-standard
~~~

- [ ] Run every affected package suite:

~~~powershell
cargo test -p mtgml-state --locked
cargo test -p mtgml-rules --locked
cargo test -p mtgml-replay --locked
cargo test -p mtgml-environment --locked
cargo test -p mtgml-observation --locked
cargo test -p mtgml-conformance --locked
~~~

- [ ] Run native Rust gates:

~~~powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
~~~

- [ ] Run the repository's Python, schema, wire parity, documentation,
  maintainer, golden-path, and integration profiles with the pinned interpreter
  from docs/maintenance/DEVELOPER_SETUP.md. Record exact counts and exit
  codes. If just check-fast or just check cannot start because the host cannot
  execute /bin/bash, record each wrapper as BLOCKED and report the successful
  direct constituent commands separately; never infer wrapper PASS.

- [ ] Verify no public/wire/schema/digest/RNG/historical replay change with:

~~~powershell
git diff --name-only 9996cfd0fcd4ef67d98cb0422611cebabd20e46b
rg -n "SystemTime|Instant::now|thread_rng|rand::|GameObjectId|RuleEventId|root_seed|raw_words" crates/mtgml-environment/src/replay.rs crates/mtgml-environment/src/synthetic/eventful.rs
~~~

The trusted identifiers search may find internal test assertions; classify those
as test-only and verify that no player DTO includes them.

## Task 10: Final exact-head evidence, push, and one PR

**Files:**

- Modify: docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-e-dispositions-and-evidence.md

- [ ] Write the final disposition matrix with every key required by the
  Batch-E task. Set TASK to PRE_M3_REMEDIATION_BATCH_E, BASE to
  9996cfd0fcd4ef67d98cb0422611cebabd20e46b, BRANCH to
  chris/pre-m3-remediation-batch-e-cross-layer-closure, and PR to the single
  URL created in this task. Copy each FND and E-section disposition from the
  executed evidence, set FND_026B to BLOCKED_CONTRACT_AMBIGUITY and FND_026C
  to DEFERRED_P2, and set ADR_CANDIDATES/ADR_ACCEPTED according to whether an
  accepted-architecture change was actually required. Set
  NEW_MAGIC_SEMANTICS, M3_STARTED, M3_AUTHORIZED, and MERGE_PERFORMED to NO.
  Set every verification key to the actual command result and count. Do not
  commit descriptive text in place of a measured value.

- [ ] Commit the evidence document only after all gates and counts are fresh:

~~~powershell
git add docs/superpowers/specs/2026-09-14-pre-m3-remediation-batch-e-dispositions-and-evidence.md
git commit -m "docs: record Batch-E dispositions and evidence"
~~~

- [ ] Re-run the final exact-head verification after the evidence commit:

~~~powershell
git status --porcelain=v1
git rev-parse HEAD
git diff --check
git diff --name-only 9996cfd0fcd4ef67d98cb0422611cebabd20e46b
cargo test --workspace --all-features --locked
~~~

- [ ] Use the GitHub CLI to confirm authentication, push the single branch, and
  verify the remote branch points to the exact local head:

~~~powershell
gh auth status
git push --set-upstream origin chris/pre-m3-remediation-batch-e-cross-layer-closure
git rev-parse HEAD
git ls-remote origin refs/heads/chris/pre-m3-remediation-batch-e-cross-layer-closure
~~~

- [ ] Open exactly one PR against master with title
  Pre-M3 remediation Batch E: cross-layer runtime and replay closure. The PR
  body must include scope, every exact disposition, producer identity policy,
  status-universe policy, external-counter owner/order, structural versus
  verified replay boundary, rejection matrix, FND-026B/C/D policy, all exact
  verification commands/counts, blocked/deferred work, compatibility fields,
  and M3_AUTHORIZED = NO.

- [ ] Wait for hosted CI on the exact final PR head and record its actual result.
  Store the PR number from the create command in a task-specific PowerShell
  variable and use it for the following checks:

~~~powershell
$batchEPrNumber = gh pr view --json number --jq .number
gh pr checks $batchEPrNumber --watch
git rev-parse HEAD
gh pr view $batchEPrNumber --json baseRefName,headRefName,headRefOid,statusCheckRollup,url
~~~

- [ ] Do not merge. End the final response with the exact Batch-E delivery keys
  required by the task: the actual PR URL, BASE `9996cfd0fcd4ef67d98cb0422611cebabd20e46b`,
  actual final HEAD, E1/E2/E3/E4 statuses, actual HOSTED_CI status,
  MERGE_PERFORMED = NO, FOUNDATION_READY_FOR_M3 = NO, and M3_AUTHORIZED = NO.

The next action is independent exact-head review.

## Plan self-review checklist

- [ ] FND-007, FND-008, and FND-012B are characterized before production edits,
  with no forced fix when the current ownership is correct.
- [ ] FND-025 does not modify global EpisodeStatus::validate() and preserves
  the checkpoint digest helper's defensive sort and KAT behavior.
- [ ] FND-024 distinguishes committed deterministic truncation, aborted internal
  failure, and unsupported external-counter-driven truncation.
- [ ] FND-022E uses the existing replay after-counter fields and the existing
  trusted checkpoint restore path; it adds no hidden setter or host-clock read.
- [ ] FND-023 clearly separates detached structural validation from backend
  verification.
- [ ] FND-026B remains blocked, FND-026C remains deferred, and FND-026D uses
  exactly one test-only situation generator plus the production projector.
- [ ] The plan changes no historical Replay V1/V2 meaning, wire shape, schema,
  digest domain, RNG algorithm, public endpoint API, or M3 semantics.
- [ ] Every verification status in the final matrix is backed by a fresh command
  on the exact final head.
