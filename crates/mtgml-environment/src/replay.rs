use mtgml_replay::AuthoritativeReplayV2;

use crate::checkpoint::EnvironmentCheckpointV2;
use crate::controller::EnvironmentBackend;
use crate::errors::{ControllerError, ReplayExecutionError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayExecutionTrace {
    pub step_index: u64,
    pub before: EnvironmentCheckpointV2,
    pub transition: mtgml_rules::TransitionResult,
    pub after: EnvironmentCheckpointV2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayExecutionReport {
    pub traces: Vec<ReplayExecutionTrace>,
    pub final_checkpoint: EnvironmentCheckpointV2,
}

pub(crate) fn execute_replay(
    backend: &mut dyn EnvironmentBackend,
    replay: AuthoritativeReplayV2,
) -> Result<ReplayExecutionReport, ControllerError> {
    replay.validate()?;
    let segment = backend.export_replay()?;
    if replay.manifest != segment.manifest {
        return Err(ReplayExecutionError::ManifestMismatch.into());
    }
    let mut expected_digest = replay.manifest.initial_state_digest.clone();
    let mut traces = Vec::with_capacity(replay.steps.len());
    for step in replay.steps {
        let before = backend.checkpoint()?;
        if before.state.revision != step.state_revision_before {
            return Err(ReplayExecutionError::BeforeRevisionMismatch {
                step_index: step.step_index,
            }
            .into());
        }
        if before.state_digest != expected_digest {
            return Err(ReplayExecutionError::BeforeDigestMismatch {
                step_index: step.step_index,
            }
            .into());
        }
        let actor = before
            .state
            .execution
            .pending_decision
            .as_ref()
            .map(|pending| pending.request.actor)
            .ok_or(ReplayExecutionError::ActorUnavailable {
                step_index: step.step_index,
            })?;
        let transition = backend.execute_trusted_response(actor, step.response.clone())?;
        let after = backend.checkpoint()?;
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
        if after.state.revision != step.state_revision_after {
            return Err(ReplayExecutionError::AfterRevisionMismatch {
                step_index: step.step_index,
            }
            .into());
        }
        if after.state_digest != step.state_digest_after {
            return Err(ReplayExecutionError::AfterDigestMismatch {
                step_index: step.step_index,
            }
            .into());
        }
        expected_digest = step.state_digest_after;
        traces.push(crate::replay::ReplayExecutionTrace {
            step_index: step.step_index,
            before,
            transition,
            after,
        });
    }
    let checkpoint = backend.checkpoint()?;
    if replay.final_state_revision != checkpoint.state.revision
        || replay.final_state_digest != checkpoint.state_digest
    {
        return Err(ReplayExecutionError::FinalIdentityMismatch.into());
    }
    Ok(ReplayExecutionReport {
        traces,
        final_checkpoint: checkpoint,
    })
}
