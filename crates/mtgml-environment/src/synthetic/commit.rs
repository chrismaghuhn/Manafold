//! Ownership: environment transaction material. The operation sequence
//! inside execute_response is FROZEN (ADR-0040): before checkpoint -> kernel
//! apply -> transition-contract validation -> rejected nonmutation branch ->
//! candidate counters/checkpoint -> candidate ReplayStep append/export ->
//! per-perspective occurrence projection -> before_commit callback -> atomic
//! commit of state/status/counters/replay.

use std::collections::BTreeMap;

use mtgml_decision::DecisionResponseV2;
use mtgml_model::PlayerId;
use mtgml_observation::ObservedEventEnvelopeV2;
use mtgml_replay::ReplayStepV3;
use mtgml_rules::{validate_transition_contract, RulesKernel, TransitionResult};

use super::SyntheticM1EnvironmentBackend;
use crate::checkpoint::{EnvironmentCheckpointV3, EnvironmentLimitCounters};
use crate::errors::{ControllerError, EnvironmentCommitError};

impl SyntheticM1EnvironmentBackend {
    pub(super) fn current_checkpoint(&self) -> Result<EnvironmentCheckpointV3, ControllerError> {
        Ok(EnvironmentCheckpointV3::new(
            self.state.clone(),
            self.status.clone(),
            self.limit_counters.clone(),
            self.codec.clone(),
        )?)
    }

    fn checked_add_counter(
        value: u64,
        increment: u64,
        counter: &'static str,
    ) -> Result<u64, ControllerError> {
        value
            .checked_add(increment)
            .ok_or(ControllerError::CounterOverflow { counter })
    }

    fn candidate_counters(
        before: &EnvironmentLimitCounters,
        event_count: usize,
    ) -> Result<EnvironmentLimitCounters, ControllerError> {
        let event_count =
            u64::try_from(event_count).map_err(|_| ControllerError::CounterOverflow {
                counter: "rule_events_emitted",
            })?;
        Ok(EnvironmentLimitCounters {
            decisions_submitted: Self::checked_add_counter(
                before.decisions_submitted,
                1,
                "decisions_submitted",
            )?,
            accepted_transitions: Self::checked_add_counter(
                before.accepted_transitions,
                1,
                "accepted_transitions",
            )?,
            rule_events_emitted: Self::checked_add_counter(
                before.rule_events_emitted,
                event_count,
                "rule_events_emitted",
            )?,
            resource_units_consumed: before.resource_units_consumed,
            wall_clock_elapsed_millis: before.wall_clock_elapsed_millis,
        })
    }

    pub(crate) fn execute_response<F>(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
        before_commit: F,
    ) -> Result<TransitionResult, ControllerError>
    where
        F: FnOnce(
            &EnvironmentCheckpointV3,
            &TransitionResult,
            &BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV2>>,
        ) -> Result<(), ControllerError>,
    {
        let before = self.current_checkpoint()?;
        let transition = self.kernel.apply(&before.state, actor, &response)?;
        validate_transition_contract(&before.state, &transition)?;

        if !transition.accepted {
            let after = self.current_checkpoint()?;
            if after != before {
                return Err(EnvironmentCommitError::RejectedMutation.into());
            }
            return Ok(transition);
        }

        let candidate_counters =
            Self::candidate_counters(&before.limit_counters, transition.events.len())?;
        let candidate = EnvironmentCheckpointV3::new(
            transition.next_state.clone(),
            transition.status.clone(),
            candidate_counters,
            before.codec.clone(),
        )?;
        if candidate.state != transition.next_state || candidate.status != transition.status {
            return Err(EnvironmentCommitError::CandidateMismatch.into());
        }

        let step_index = u64::try_from(self.replay.step_count()).map_err(|_| {
            ControllerError::CounterOverflow {
                counter: "replay_step_index",
            }
        })?;
        let step = ReplayStepV3 {
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
        let mut candidate_replay = self.replay.clone();
        candidate_replay.append(step)?;
        candidate_replay.export()?;
        // ADR-0040: every required per-perspective projection is validated
        // against the candidate product BEFORE the atomic commit.
        let occurrence_envelopes = crate::lifecycle_projection::project_occurrence_envelopes(
            &before.state,
            &transition.next_state,
            &transition.events,
        )
        .map_err(|_| {
            ControllerError::EnvironmentCommit(EnvironmentCommitError::PlayerProjectionInvalid)
        })?;
        before_commit(&candidate, &transition, &occurrence_envelopes)?;

        self.state = candidate.state;
        self.status = candidate.status;
        self.limit_counters = candidate.limit_counters;
        self.replay = candidate_replay;
        Ok(transition)
    }
}
