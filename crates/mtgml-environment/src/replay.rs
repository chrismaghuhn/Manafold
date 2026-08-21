use mtgml_model::EnvironmentLimitCounters;
use mtgml_replay::{AuthoritativeReplayV3, InitialEnvironmentIdentityV3};
use mtgml_rules::validate_transition_contract;

use crate::checkpoint::EnvironmentCheckpointV3;
use crate::controller::EnvironmentBackend;
use crate::errors::{ControllerError, ReplayExecutionError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayExecutionTrace {
    pub step_index: u64,
    pub before: EnvironmentCheckpointV3,
    pub transition: mtgml_rules::TransitionResult,
    pub after: EnvironmentCheckpointV3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayExecutionReport {
    pub traces: Vec<ReplayExecutionTrace>,
    pub final_checkpoint: EnvironmentCheckpointV3,
}

fn checked_counter_add(
    value: u64,
    increment: u64,
    counter: &'static str,
) -> Result<u64, ControllerError> {
    value
        .checked_add(increment)
        .ok_or(ControllerError::CounterOverflow { counter })
}

fn expected_counters(
    before: &EnvironmentCheckpointV3,
    transition: &mtgml_rules::TransitionResult,
) -> Result<EnvironmentLimitCounters, ControllerError> {
    if !transition.accepted {
        return Ok(before.limit_counters.clone());
    }
    let event_count =
        u64::try_from(transition.events.len()).map_err(|_| ControllerError::CounterOverflow {
            counter: "rule_events_emitted",
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

fn checkpoint(
    backend: &dyn EnvironmentBackend,
) -> Result<EnvironmentCheckpointV3, ControllerError> {
    let checkpoint = backend.checkpoint()?;
    checkpoint
        .validate()
        .map_err(ControllerError::CheckpointValidation)?;
    Ok(checkpoint)
}

fn identity_from_checkpoint(checkpoint: &EnvironmentCheckpointV3) -> InitialEnvironmentIdentityV3 {
    InitialEnvironmentIdentityV3 {
        state_revision: checkpoint.state.revision,
        full_state_digest: checkpoint.state_digest.clone(),
        episode_status: checkpoint.status.clone(),
        environment_limit_counters: checkpoint.limit_counters.clone(),
        checkpoint_codec_identity: checkpoint.codec.clone(),
        checkpoint_digest: checkpoint.checkpoint_digest.clone(),
    }
}

pub(crate) fn execute_replay(
    backend: &mut dyn EnvironmentBackend,
    replay: AuthoritativeReplayV3,
) -> Result<ReplayExecutionReport, ControllerError> {
    replay.validate()?;
    let segment = backend.export_replay()?;
    if replay.manifest != segment.manifest {
        return Err(ReplayExecutionError::ManifestMismatch.into());
    }
    let mut expected = replay.manifest.initial_identity.clone();
    let mut traces = Vec::with_capacity(replay.steps.len());
    for step in replay.steps {
        let before = checkpoint(backend)?;
        let actual_before = identity_from_checkpoint(&before);
        if actual_before.state_revision != step.state_revision_before {
            return Err(ReplayExecutionError::BeforeRevisionMismatch {
                step_index: step.step_index,
            }
            .into());
        }
        if actual_before != expected
            || actual_before.checkpoint_digest != step.checkpoint_digest_before
        {
            return Err(ReplayExecutionError::BeforeDigestMismatch {
                step_index: step.step_index,
            }
            .into());
        }
        let pending_actor = before
            .state
            .execution
            .pending_decision
            .as_ref()
            .map(|pending| pending.request.actor)
            .ok_or(ReplayExecutionError::ActorUnavailable {
                step_index: step.step_index,
            })?;
        if pending_actor != step.actor {
            return Err(ReplayExecutionError::ActorUnavailable {
                step_index: step.step_index,
            }
            .into());
        }
        let transition = backend.execute_trusted_response(step.actor, step.response.clone())?;
        validate_transition_contract(&before.state, &transition)?;
        let after = checkpoint(backend)?;
        if transition.accepted != step.accepted {
            return Err(ReplayExecutionError::OutcomeMismatch {
                step_index: step.step_index,
            }
            .into());
        }
        if transition.next_state != after.state || transition.status != after.status {
            return Err(ReplayExecutionError::TransitionMismatch {
                step_index: step.step_index,
            }
            .into());
        }
        if after.limit_counters != expected_counters(&before, &transition)? {
            return Err(ReplayExecutionError::CounterMismatch {
                step_index: step.step_index,
            }
            .into());
        }
        let actual_after = identity_from_checkpoint(&after);
        if actual_after.state_revision != step.state_revision_after
            || actual_after.full_state_digest != step.full_state_digest_after
            || actual_after.episode_status != step.episode_status_after
            || actual_after.environment_limit_counters != step.environment_limit_counters_after
            || actual_after.checkpoint_digest != step.checkpoint_digest_after
        {
            return Err(ReplayExecutionError::AfterDigestMismatch {
                step_index: step.step_index,
            }
            .into());
        }
        expected = actual_after;
        traces.push(ReplayExecutionTrace {
            step_index: step.step_index,
            before,
            transition,
            after,
        });
    }
    let final_checkpoint = checkpoint(backend)?;
    if identity_from_checkpoint(&final_checkpoint) != replay.final_identity {
        return Err(ReplayExecutionError::FinalIdentityMismatch.into());
    }
    Ok(ReplayExecutionReport {
        traces,
        final_checkpoint,
    })
}
