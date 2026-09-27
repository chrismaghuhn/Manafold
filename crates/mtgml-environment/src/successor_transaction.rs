//! Atomic successor response transaction for the integration runtime.
//!
//! This is the environment commit owner, not a second backend. It stages one
//! RulesKernel product, verifies the V6-bound replacement and V7 checkpoint,
//! validates every perspective's V3 step, appends to a cloned V7 recorder,
//! and only then publishes the candidate values to the caller.

use std::collections::BTreeMap;

use mtgml_decision::DecisionResponseV2;
use mtgml_model::{EnvironmentLimitCounters, EpisodeStatus, ExecutionIdentityV1, PlayerId};
use mtgml_observation::{PlayerStepSubmissionV1, PlayerStepV3};
use mtgml_replay::{ReplayRecorderV7, ReplayStepV7};
use mtgml_rules::{KernelExecutionError, ProgramKernelV1, RulesKernel, TransitionResult};
use mtgml_state::{EngineStatePartsV2, StateDeltaV2};

use crate::checkpoint_v7::EnvironmentCheckpointV7;
use crate::errors::{ControllerError, EnvironmentCommitError};

pub struct SuccessorResponseTransaction<'a> {
    pub state: &'a mut EngineStatePartsV2,
    pub status: &'a mut EpisodeStatus,
    pub limit_counters: &'a mut EnvironmentLimitCounters,
    pub execution_identity: &'a ExecutionIdentityV1,
    pub replay: &'a mut ReplayRecorderV7,
    pub kernel: &'a mut ProgramKernelV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuccessorTransactionOutput {
    pub transition: TransitionResult,
    pub checkpoint: EnvironmentCheckpointV7,
    pub player_steps: BTreeMap<PlayerId, PlayerStepV3>,
}

pub fn execute_successor_response_transaction(
    transaction: SuccessorResponseTransaction<'_>,
    actor: PlayerId,
    response: DecisionResponseV2,
) -> Result<SuccessorTransactionOutput, ControllerError> {
    let SuccessorResponseTransaction {
        state,
        status,
        limit_counters,
        execution_identity,
        replay,
        kernel,
    } = transaction;

    let before = EnvironmentCheckpointV7::new(
        state.clone(),
        status.clone(),
        limit_counters.clone(),
        execution_identity.clone(),
    )?;

    let transition = match RulesKernel::apply(kernel, state, actor, &response, status) {
        Ok(product) => product,
        Err(KernelExecutionError::BasicLandTransition(
            mtgml_rules::BasicLandTransitionError::InvalidSelection
            | mtgml_rules::BasicLandTransitionError::InvalidLand
            | mtgml_rules::BasicLandTransitionError::InvalidAbility,
        )) => rejected_product(state, status)?,
        Err(error) => return Err(error.into()),
    };

    let admission = kernel
        .executable_profile_admission()
        .ok_or(ControllerError::SemanticContractUnsupported)?;
    mtgml_rules::validate_successor_transition_contract(admission, state, &transition)?;

    if transition.accepted {
        let replacement = transition
            .delta
            .apply(state)
            .map_err(|_| EnvironmentCommitError::CandidateMismatch)?;
        if replacement != transition.next_state {
            return Err(EnvironmentCommitError::CandidateMismatch.into());
        }
    } else if transition.next_state != *state
        || !transition.events.is_empty()
        || transition.status != *status
    {
        return Err(EnvironmentCommitError::RejectedMutation.into());
    }

    let candidate_counters = if transition.accepted {
        EnvironmentLimitCounters {
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
                u64::try_from(transition.events.len()).map_err(|_| {
                    ControllerError::CounterOverflow {
                        counter: "rule_events_emitted",
                    }
                })?,
                "rule_events_emitted",
            )?,
            resource_units_consumed: before.limit_counters.resource_units_consumed,
            wall_clock_elapsed_millis: before.limit_counters.wall_clock_elapsed_millis,
        }
    } else {
        before.limit_counters.clone()
    };

    let candidate = EnvironmentCheckpointV7::new(
        transition.next_state.clone(),
        transition.status.clone(),
        candidate_counters,
        before.execution_identity.clone(),
    )?;
    let player_steps = crate::successor_projection::project_successor_player_steps(
        state,
        &transition,
        crate::successor_projection::SuccessorProjectionAuthority {
            execution_identity: admission.execution_identity(),
            semantic_manifest: admission.semantic_contract_manifest(),
            rules_manifest: admission.rules_contract_manifest(),
            catalog: admission.verified_catalog(),
        },
        actor,
        mtgml_observation::PlayerSubmissionCodeV1::InvalidCandidate,
    )?;
    validate_player_steps(
        state,
        &candidate,
        actor,
        transition.accepted,
        transition.next_decision.as_ref(),
        &player_steps,
    )?;

    // Rejected submissions are observable PlayerSteps but are not semantic
    // history. They must not alter the authoritative Replay V7 sequence.
    let candidate_replay = if transition.accepted {
        let step_index =
            u64::try_from(replay.step_count()).map_err(|_| ControllerError::CounterOverflow {
                counter: "replay_step_index",
            })?;
        let step = ReplayStepV7 {
            step_index,
            actor,
            checkpoint_digest_before: before.checkpoint_digest.clone(),
            state_revision_before: before.state.predecessor_v5.revision,
            response,
            accepted: true,
            state_revision_after: candidate.state.predecessor_v5.revision,
            full_state_digest_after: candidate.state_digest.clone(),
            episode_status_after: candidate.status.clone(),
            environment_limit_counters_after: candidate.limit_counters.clone(),
            checkpoint_digest_after: candidate.checkpoint_digest.clone(),
        };
        let mut candidate_replay = replay.clone();
        candidate_replay.append(step)?;
        candidate_replay.export()?;
        candidate_replay
    } else {
        replay.clone()
    };

    *state = candidate.state.clone();
    *status = candidate.status.clone();
    *limit_counters = candidate.limit_counters.clone();
    *replay = candidate_replay;

    Ok(SuccessorTransactionOutput {
        transition,
        checkpoint: candidate,
        player_steps,
    })
}

fn rejected_product(
    state: &EngineStatePartsV2,
    status: &EpisodeStatus,
) -> Result<TransitionResult, ControllerError> {
    Ok(TransitionResult {
        accepted: false,
        next_state: state.clone(),
        delta: StateDeltaV2::between(state, state, Vec::new())
            .map_err(|_| EnvironmentCommitError::CandidateMismatch)?,
        events: Vec::new(),
        next_decision: state.execution_v3.pending_decision.clone(),
        status: status.clone(),
    })
}

fn validate_player_steps(
    before: &EngineStatePartsV2,
    after: &EnvironmentCheckpointV7,
    actor: PlayerId,
    accepted: bool,
    next_decision: Option<&mtgml_decision::AuthoritativeDecisionRequestV3>,
    steps: &BTreeMap<PlayerId, PlayerStepV3>,
) -> Result<(), EnvironmentCommitError> {
    let players: std::collections::BTreeSet<_> = after
        .state
        .predecessor_v5
        .core
        .players
        .keys()
        .copied()
        .collect();
    if steps
        .keys()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        != players
    {
        return Err(EnvironmentCommitError::PlayerProjectionInvalid);
    }
    for (perspective, step) in steps {
        step.validate()
            .map_err(|_| EnvironmentCommitError::PlayerProjectionInvalid)?;
        if step.information_state.perspective != *perspective
            || step.information_state.state_revision != after.state.predecessor_v5.revision
            || step.status != after.status
        {
            return Err(EnvironmentCommitError::PlayerProjectionInvalid);
        }
        let expected_request = next_decision
            .filter(|request| request.actor == *perspective)
            .map(|request| request.project_player_request())
            .transpose()
            .map_err(|_| EnvironmentCommitError::PlayerProjectionInvalid)?;
        if step.next_decision != expected_request {
            return Err(EnvironmentCommitError::PlayerProjectionInvalid);
        }
        let expected_submission = if *perspective != actor || accepted {
            PlayerStepSubmissionV1::Accepted
        } else {
            // The projector chooses the precise public rejection code. This
            // transaction still requires the rejected-step empty-event rule.
            match &step.submission {
                PlayerStepSubmissionV1::Rejected { .. } => {}
                PlayerStepSubmissionV1::Accepted => {
                    return Err(EnvironmentCommitError::PlayerProjectionInvalid)
                }
            }
            continue;
        };
        if step.submission != expected_submission {
            return Err(EnvironmentCommitError::PlayerProjectionInvalid);
        }
    }
    if !accepted {
        let unchanged = before == &after.state;
        if !unchanged || steps.values().any(|step| !step.observed_events.is_empty()) {
            return Err(EnvironmentCommitError::RejectedMutation);
        }
    }
    Ok(())
}

fn checked_add(value: u64, amount: u64, counter: &'static str) -> Result<u64, ControllerError> {
    value
        .checked_add(amount)
        .ok_or(ControllerError::CounterOverflow { counter })
}
