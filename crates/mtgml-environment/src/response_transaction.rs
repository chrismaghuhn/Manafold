//! Single environment-owned response commit authority.
//!
//! This module owns the semantic-neutral transaction around one real
//! `DecisionResponseV2`. Rules meaning stays with `ProgramKernelV1`; this
//! module constructs and validates the complete V6 candidate and commits it
//! only after replay, occurrence projection, player projection, and the
//! caller's before-commit hook have succeeded.

use std::collections::BTreeMap;

use mtgml_decision::DecisionResponseV2;
use mtgml_model::{EpisodeStatus, ExecutionIdentityV1, PlayerId};
use mtgml_observation::ObservedEventEnvelopeV2;
use mtgml_replay::{ReplayRecorderV6, ReplayStepV6};
use mtgml_rules::{validate_transition_contract, ProgramKernelV1, TransitionResult};
use mtgml_state::{EngineState, StateDelta};

use crate::checkpoint::{
    CheckpointCodecIdentity, EnvironmentCheckpointV6, EnvironmentLimitCounters,
};
use crate::errors::{ControllerError, EnvironmentCommitError};

pub(crate) struct ResponseTransaction<'a> {
    pub(crate) state: &'a mut EngineState,
    pub(crate) status: &'a mut EpisodeStatus,
    pub(crate) limit_counters: &'a mut EnvironmentLimitCounters,
    pub(crate) codec: &'a CheckpointCodecIdentity,
    pub(crate) execution_identity: &'a ExecutionIdentityV1,
    pub(crate) replay: &'a mut ReplayRecorderV6,
    pub(crate) kernel: &'a mut ProgramKernelV1,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResponseTransactionFailurePoint {
    ForcedProgress,
    CandidateCheckpoint,
    ReplayAppend,
    ReplayExport,
    OccurrenceProjection,
    PlayerProjectionValidation,
}

#[cfg(test)]
pub(crate) type TestApplyOverride =
    fn(
        &EngineState,
        PlayerId,
        &DecisionResponseV2,
    ) -> Result<TransitionResult, mtgml_rules::KernelExecutionError>;

pub(crate) fn execute_response_transaction<F>(
    transaction: ResponseTransaction<'_>,
    actor: PlayerId,
    response: DecisionResponseV2,
    before_commit: F,
    #[cfg(test)] failure_point: Option<ResponseTransactionFailurePoint>,
    #[cfg(test)] apply_override: Option<TestApplyOverride>,
) -> Result<TransitionResult, ControllerError>
where
    F: FnOnce(
        &EnvironmentCheckpointV6,
        &TransitionResult,
        &BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV2>>,
    ) -> Result<(), ControllerError>,
{
    let ResponseTransaction {
        state,
        status,
        limit_counters,
        codec,
        execution_identity,
        replay,
        kernel,
    } = transaction;
    let before = EnvironmentCheckpointV6::new(
        state.clone(),
        status.clone(),
        limit_counters.clone(),
        codec.clone(),
        execution_identity.clone(),
    )?;

    #[cfg(test)]
    let mut transition = match apply_override {
        Some(apply) => apply(&before.state, actor, &response)?,
        None => kernel.apply(&before.state, actor, &response)?,
    };
    #[cfg(not(test))]
    let mut transition = kernel.apply(&before.state, actor, &response)?;

    // The kernel owns deterministic closure. The environment invokes at most
    // one forced advance, and only for an accepted response that leaves a
    // running game without a Decision.
    if transition.accepted
        && transition.next_decision.is_none()
        && transition.status == EpisodeStatus::Running
    {
        #[cfg(test)]
        fail_at(
            failure_point,
            ResponseTransactionFailurePoint::ForcedProgress,
        )?;

        let advanced = kernel.advance_forced_progress(&transition.next_state)?;
        if advanced.next_state != transition.next_state {
            let mut events = transition.events;
            events.extend(advanced.events);
            let audit = events
                .iter()
                .map(|event| event.event.semantic_delta())
                .collect();
            let delta =
                StateDelta::between(&before.state, &advanced.next_state, audit).map_err(|_| {
                    ControllerError::Backend("forced-progress merged delta failed".into())
                })?;
            transition = TransitionResult {
                accepted: true,
                next_state: advanced.next_state,
                delta,
                events,
                next_decision: advanced.next_decision,
                status: advanced.status,
            };
        }
    }
    validate_transition_contract(&before.state, &transition)?;

    if !transition.accepted {
        let after = EnvironmentCheckpointV6::new(
            state.clone(),
            status.clone(),
            limit_counters.clone(),
            codec.clone(),
            execution_identity.clone(),
        )?;
        if after != before {
            return Err(EnvironmentCommitError::RejectedMutation.into());
        }
        return Ok(transition);
    }

    let event_count =
        u64::try_from(transition.events.len()).map_err(|_| ControllerError::CounterOverflow {
            counter: "rule_events_emitted",
        })?;
    let candidate_counters = EnvironmentLimitCounters {
        decisions_submitted: checked_add(
            before.limit_counters.decisions_submitted,
            1,
            "decisions_submitted",
        )?,
        accepted_transitions: checked_add(
            before.limit_counters.accepted_transitions,
            1,
            "accepted_transitions",
        )?,
        rule_events_emitted: checked_add(
            before.limit_counters.rule_events_emitted,
            event_count,
            "rule_events_emitted",
        )?,
        resource_units_consumed: before.limit_counters.resource_units_consumed,
        wall_clock_elapsed_millis: before.limit_counters.wall_clock_elapsed_millis,
    };

    #[cfg(test)]
    fail_at(
        failure_point,
        ResponseTransactionFailurePoint::CandidateCheckpoint,
    )?;
    let candidate = EnvironmentCheckpointV6::new(
        transition.next_state.clone(),
        transition.status.clone(),
        candidate_counters,
        before.codec.clone(),
        before.execution_identity.clone(),
    )?;
    if candidate.state != transition.next_state || candidate.status != transition.status {
        return Err(EnvironmentCommitError::CandidateMismatch.into());
    }

    let step_index =
        u64::try_from(replay.step_count()).map_err(|_| ControllerError::CounterOverflow {
            counter: "replay_step_index",
        })?;
    let step = ReplayStepV6 {
        step_index,
        actor,
        checkpoint_digest_before: before.checkpoint_digest.clone(),
        state_revision_before: before.state.revision,
        response,
        accepted: true,
        state_revision_after: candidate.state.revision,
        full_state_digest_after: candidate.state_digest.clone(),
        episode_status_after: candidate.status.clone(),
        environment_limit_counters_after: candidate.limit_counters.clone(),
        checkpoint_digest_after: candidate.checkpoint_digest.clone(),
    };
    #[cfg(test)]
    fail_at(failure_point, ResponseTransactionFailurePoint::ReplayAppend)?;
    let mut candidate_replay = replay.clone();
    candidate_replay.append(step)?;
    #[cfg(test)]
    fail_at(failure_point, ResponseTransactionFailurePoint::ReplayExport)?;
    candidate_replay.export()?;

    #[cfg(test)]
    fail_at(
        failure_point,
        ResponseTransactionFailurePoint::OccurrenceProjection,
    )?;
    let occurrence_envelopes = crate::lifecycle_projection::project_occurrence_envelopes(
        &before.state,
        &transition.next_state,
        &transition.events,
    )
    .map_err(|_| {
        ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
    })?;

    #[cfg(test)]
    fail_at(
        failure_point,
        ResponseTransactionFailurePoint::PlayerProjectionValidation,
    )?;
    let projection_profile =
        crate::player_projection::profile_for_execution_identity(execution_identity)?;
    crate::player_projection::validate_candidate_projections_with_profile(
        &candidate.state,
        projection_profile,
    )
    .map_err(|_| {
        ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
    })?;

    before_commit(&candidate, &transition, &occurrence_envelopes)?;

    // No fallible work follows: the controller lock makes these replacements
    // one externally observable commit.
    *state = candidate.state;
    *status = candidate.status;
    *limit_counters = candidate.limit_counters;
    *replay = candidate_replay;
    Ok(transition)
}

fn checked_add(value: u64, increment: u64, counter: &'static str) -> Result<u64, ControllerError> {
    value
        .checked_add(increment)
        .ok_or(ControllerError::CounterOverflow { counter })
}

#[cfg(test)]
fn fail_at(
    selected: Option<ResponseTransactionFailurePoint>,
    expected: ResponseTransactionFailurePoint,
) -> Result<(), ControllerError> {
    if selected == Some(expected) {
        return Err(ControllerError::Backend(format!(
            "injected response transaction failure at {expected:?}"
        )));
    }
    Ok(())
}
