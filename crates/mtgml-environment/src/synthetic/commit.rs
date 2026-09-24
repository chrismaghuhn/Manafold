//! Synthetic backend adapters to the environment-owned transaction owners.
//!
//! Accepted response commit ordering lives in `response_transaction`; this
//! module only supplies Synthetic-owned state and replay construction. The
//! standalone no-response forced-progress path remains separately owned by
//! the existing reference transaction helper.

use std::collections::BTreeMap;

use mtgml_decision::DecisionResponseV2;
use mtgml_model::PlayerId;
use mtgml_observation::ObservedEventEnvelopeV2;
use mtgml_rules::TransitionResult;

use super::replay::build_manifest;
use super::SyntheticRulesEnvironmentBackend;
use crate::checkpoint::EnvironmentCheckpointV6;
use crate::errors::ControllerError;

impl SyntheticRulesEnvironmentBackend {
    pub(super) fn current_checkpoint(&self) -> Result<EnvironmentCheckpointV6, ControllerError> {
        crate::reference::current_checkpoint(
            &self.state,
            &self.status,
            &self.limit_counters,
            &self.codec,
            &self.execution_identity,
        )
    }

    /// Synthetic's response entry point delegates the complete accepted
    /// response commit to the single environment transaction authority.
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
        #[cfg(test)]
        let apply_override = self
            .eventful_fixture
            .then_some(super::eventful::apply as crate::response_transaction::TestApplyOverride);
        self.execute_response_inner(
            actor,
            response,
            before_commit,
            #[cfg(test)]
            None,
            #[cfg(test)]
            apply_override,
        )
    }

    fn execute_response_inner<F>(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
        before_commit: F,
        #[cfg(test)] failure_point: Option<
            crate::response_transaction::ResponseTransactionFailurePoint,
        >,
        #[cfg(test)] apply_override: Option<crate::response_transaction::TestApplyOverride>,
    ) -> Result<TransitionResult, ControllerError>
    where
        F: FnOnce(
            &EnvironmentCheckpointV6,
            &TransitionResult,
            &BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV2>>,
        ) -> Result<(), ControllerError>,
    {
        let transaction = crate::response_transaction::ResponseTransaction {
            state: &mut self.state,
            status: &mut self.status,
            limit_counters: &mut self.limit_counters,
            codec: &self.codec,
            execution_identity: &self.execution_identity,
            replay: &mut self.replay,
            kernel: &mut self.kernel,
        };
        #[cfg(test)]
        {
            crate::response_transaction::execute_response_transaction(
                transaction,
                actor,
                response,
                before_commit,
                failure_point,
                apply_override,
            )
        }
        #[cfg(not(test))]
        {
            crate::response_transaction::execute_response_transaction(
                transaction,
                actor,
                response,
                before_commit,
            )
        }
    }

    #[cfg(test)]
    pub(crate) fn execute_response_with_failure_point<F>(
        &mut self,
        actor: PlayerId,
        response: DecisionResponseV2,
        failure_point: Option<crate::response_transaction::ResponseTransactionFailurePoint>,
        before_commit: F,
    ) -> Result<TransitionResult, ControllerError>
    where
        F: FnOnce(
            &EnvironmentCheckpointV6,
            &TransitionResult,
            &BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV2>>,
        ) -> Result<(), ControllerError>,
    {
        let apply_override = self
            .eventful_fixture
            .then_some(super::eventful::apply as crate::response_transaction::TestApplyOverride);
        self.execute_response_inner(
            actor,
            response,
            before_commit,
            failure_point,
            apply_override,
        )
    }

    /// Standalone forced progress has no player response and remains a
    /// distinct transaction: it rebases an empty replay baseline and never
    /// fabricates a ReplayStepV6 input.
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
}
