//! Ownership: environment transaction material. The operation sequence
//! inside execute_response is FROZEN (ADR-0040): before checkpoint -> kernel
//! apply -> shared forced-progress closure -> transition-contract validation
//! -> rejected nonmutation branch -> candidate counters/checkpoint ->
//! candidate ReplayStep append/export -> per-perspective occurrence
//! projection -> before_commit callback -> atomic commit of
//! state/status/counters/replay.
//!
//! The forced-progress closure drains mandatory no-choice work into the same
//! atomic product (same commit, same replay step) before validation. In the
//! current synthetic protocol no post-response no-choice work exists, so the
//! closure is a proven no-op on every reachable path today; its shape is
//! required for turn-structure forced progress and for sharing the one
//! rules-owned primitive across response, initialization, and T0 proof.

use std::collections::BTreeMap;

use mtgml_decision::DecisionResponseV2;
use mtgml_model::PlayerId;
use mtgml_observation::ObservedEventEnvelopeV2;
use mtgml_replay::ReplayStepV6;
use mtgml_rules::{validate_transition_contract, TransitionResult};
use mtgml_state::StateDelta;

use super::replay::build_manifest;
use super::SyntheticM1EnvironmentBackend;
use crate::checkpoint::{EnvironmentCheckpointV6, EnvironmentLimitCounters};
use crate::errors::{ControllerError, EnvironmentCommitError};

impl SyntheticM1EnvironmentBackend {
    pub(super) fn current_checkpoint(&self) -> Result<EnvironmentCheckpointV6, ControllerError> {
        crate::reference::current_checkpoint(
            &self.state,
            &self.status,
            &self.limit_counters,
            &self.codec,
            &self.execution_identity,
        )
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

    /// Rules-owned forced-progress commit. Mirrors the accepted-response
    /// commit discipline without any player submission: the kernel primitive
    /// runs on the committed state, its product passes transition-contract
    /// validation, and the atomic commit advances state, status, and exactly
    /// the counters the progress consumed. No response exists, so
    /// `decisions_submitted` never increments (which by the counter
    /// invariant also pins `accepted_transitions`), and no replay step is
    /// appended — responseless progress is execution semantics, not a
    /// synthetic player action, and ReplayStepV6 carries no response field
    /// to fabricate — but the recorder baseline is rebased onto the
    /// post-progress checkpoint so the next real response appends against a
    /// continuous identity instead of orphaning into RevisionDiscontinuity.
    /// Rebase is lossless by construction: the kernel admits standalone
    /// progress only on pristine revision-0 setups, which always pair with
    /// an empty recorder; anything else fails closed here. Failed progress
    /// commits nothing.
    pub(crate) fn execute_forced_progress(&mut self) -> Result<TransitionResult, ControllerError> {
        let config = self.config.clone();
        let mut transaction = crate::reference::ReferenceEnvironmentTransaction {
            state: &mut self.state,
            status: &mut self.status,
            limit_counters: &mut self.limit_counters,
            codec: &self.codec,
            execution_identity: &self.execution_identity,
            replay: &mut self.replay,
            kernel: &mut self.kernel,
        };
        crate::reference::execute_forced_progress_transaction(&mut transaction, move |checkpoint| {
            build_manifest(&config, checkpoint)
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
            &EnvironmentCheckpointV6,
            &TransitionResult,
            &BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV2>>,
        ) -> Result<(), ControllerError>,
    {
        let before = self.current_checkpoint()?;
        #[cfg(test)]
        let mut transition = if self.eventful_fixture {
            super::eventful::apply(&before.state, actor, &response)?
        } else {
            self.kernel.apply(&before.state, actor, &response)?
        };
        #[cfg(not(test))]
        let mut transition = self.kernel.apply(&before.state, actor, &response)?;
        // Shared forced-progress closure: while the product stops at no
        // Decision, drain one rules-owned advance into the same atomic
        // product. Event identities continue sequentially because the
        // advance derives them from the post-apply allocator heads; the
        // delta is recomputed over the whole before-to-final span. At most
        // one working advance can occur per transaction: every working
        // advance ends at a Decision, a terminal outcome, or an error.
        if transition.accepted
            && transition.next_decision.is_none()
            && transition.status == mtgml_model::EpisodeStatus::Running
        {
            let advanced = self
                .kernel
                .advance_forced_progress(&transition.next_state)?;
            if advanced.next_state != transition.next_state {
                let mut events = transition.events;
                events.extend(advanced.events);
                let audit = events
                    .iter()
                    .map(|event| event.event.semantic_delta())
                    .collect();
                let delta = StateDelta::between(&before.state, &advanced.next_state, audit)
                    .map_err(|_| {
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
            let after = self.current_checkpoint()?;
            if after != before {
                return Err(EnvironmentCommitError::RejectedMutation.into());
            }
            return Ok(transition);
        }

        let candidate_counters =
            Self::candidate_counters(&before.limit_counters, transition.events.len())?;
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

        let step_index = u64::try_from(self.replay.step_count()).map_err(|_| {
            ControllerError::CounterOverflow {
                counter: "replay_step_index",
            }
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
