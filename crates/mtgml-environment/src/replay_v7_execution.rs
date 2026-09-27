//! Executable Replay V7 seam over the single admitted successor kernel and
//! atomic environment transaction.

use mtgml_card_ir::ExecutableProfileAdmissionV1;
use mtgml_replay::{AuthoritativeReplayV7, ReplayRecorderV7};
use mtgml_rules::ProgramKernelV1;

use crate::checkpoint_v7::EnvironmentCheckpointV7;
use crate::errors::{ControllerError, ReplayExecutionError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayV7ExecutionReport {
    pub initial_checkpoint: EnvironmentCheckpointV7,
    pub final_checkpoint: EnvironmentCheckpointV7,
    pub transitions: Vec<crate::successor_transaction::SuccessorTransactionOutput>,
}

/// Re-executes the external DecisionResponseV2 control sequence from the
/// supplied V7 checkpoint. Recorded observations/events are never consulted
/// as transition input. Every replay step is checked against the actual V6
/// state digest and V7 checkpoint produced by the admitted RulesKernel.
pub fn execute_authoritative_replay_v7(
    admission: ExecutableProfileAdmissionV1,
    initial_checkpoint: EnvironmentCheckpointV7,
    replay: AuthoritativeReplayV7,
) -> Result<ReplayV7ExecutionReport, ControllerError> {
    replay.validate()?;
    initial_checkpoint.validate()?;
    if replay.manifest.execution_identity != *admission.execution_identity()
        || replay.manifest.semantic_contract.semantic_contract_id
            != *admission.semantic_contract_id()
        || replay.manifest.semantic_contract.manifest != *admission.semantic_contract_manifest()
        || replay.manifest.semantic_contract.rules_manifest != *admission.rules_contract_manifest()
        || replay
            .manifest
            .semantic_contract
            .content_contract
            .as_ref()
            .is_none_or(|content| content.content_contract_id != *admission.content_contract_id())
    {
        return Err(ReplayExecutionError::ReplayV7Identity.into());
    }
    let initial_identity = &replay.manifest.initial_identity;
    if initial_checkpoint.state.predecessor_v5.revision != initial_identity.state_revision
        || initial_checkpoint.state_digest != initial_identity.full_state_digest
        || initial_checkpoint.status != initial_identity.episode_status
        || initial_checkpoint.limit_counters != initial_identity.environment_limit_counters
        || initial_checkpoint.codec != initial_identity.checkpoint_codec_identity
        || initial_checkpoint.checkpoint_digest != initial_identity.checkpoint_digest
        || initial_checkpoint.execution_identity != initial_identity.execution_identity
    {
        return Err(ReplayExecutionError::ReplayV7BeforeIdentity.into());
    }
    initial_checkpoint.restore_with_verified_contracts(
        admission.semantic_contract_manifest(),
        admission.rules_contract_manifest(),
        Some(admission.verified_catalog()),
    )?;

    let mut state = initial_checkpoint.state.clone();
    let mut status = initial_checkpoint.status.clone();
    let mut limit_counters = initial_checkpoint.limit_counters.clone();
    let mut recorder = ReplayRecorderV7::new(replay.manifest.clone())?;
    let mut kernel = ProgramKernelV1::for_executable_profile(admission.clone())
        .map_err(|_| ControllerError::SemanticContractUnsupported)?;
    let mut transitions = Vec::with_capacity(replay.steps.len());

    for (index, step) in replay.steps.iter().enumerate() {
        let before = EnvironmentCheckpointV7::new(
            state.clone(),
            status.clone(),
            limit_counters.clone(),
            admission.execution_identity().clone(),
        )?;
        if before.checkpoint_digest != step.checkpoint_digest_before
            || before.state.predecessor_v5.revision != step.state_revision_before
        {
            return Err(ReplayExecutionError::ReplayV7BeforeStep {
                step_index: index as u64,
            }
            .into());
        }
        let output = crate::successor_transaction::execute_successor_response_transaction(
            crate::successor_transaction::SuccessorResponseTransaction {
                state: &mut state,
                status: &mut status,
                limit_counters: &mut limit_counters,
                execution_identity: admission.execution_identity(),
                replay: &mut recorder,
                kernel: &mut kernel,
            },
            step.actor,
            step.response.clone(),
        )?;
        if output.transition.accepted != step.accepted
            || output.checkpoint.state.predecessor_v5.revision != step.state_revision_after
            || output.checkpoint.state_digest != step.full_state_digest_after
            || output.checkpoint.status != step.episode_status_after
            || output.checkpoint.limit_counters != step.environment_limit_counters_after
            || output.checkpoint.checkpoint_digest != step.checkpoint_digest_after
        {
            return Err(ReplayExecutionError::ReplayV7StepMismatch {
                step_index: index as u64,
            }
            .into());
        }
        transitions.push(output);
    }

    let generated = recorder.export()?;
    if generated != replay {
        return Err(ReplayExecutionError::ReplayV7FinalIdentity.into());
    }
    let final_checkpoint = EnvironmentCheckpointV7::new(
        state,
        status,
        limit_counters,
        admission.execution_identity().clone(),
    )?;
    Ok(ReplayV7ExecutionReport {
        initial_checkpoint,
        final_checkpoint,
        transitions,
    })
}
