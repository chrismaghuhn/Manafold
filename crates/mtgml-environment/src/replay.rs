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
    let checkpoint = backend.checkpoint()?;
    if replay.manifest.initial_state_revision != checkpoint.state.revision
        || replay.manifest.initial_state_digest != checkpoint.state_digest
        || replay.final_state_revision != checkpoint.state.revision
        || replay.final_state_digest != checkpoint.state_digest
        || !replay.steps.is_empty()
    {
        return Err(ReplayExecutionError::ManifestMismatch.into());
    }
    Ok(ReplayExecutionReport {
        traces: Vec::new(),
        final_checkpoint: checkpoint,
    })
}
