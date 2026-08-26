//! The bounded synthetic M2.C decision protocol.
//!
//! One deterministic scenario exercises every closed decision family and the
//! one frozen typed continuation:
//!
//! ```text
//! entry   ChooseOne/SelectOne   rule-relevant product, creates C
//! stage 0 ChooseNumber          count -> ChooseMembers
//! stage 1 ChooseMany            member set -> OrderMembers
//! stage 2 Order                 semantic order -> completion (C removed)
//! ```
//!
//! Every accepted response is one atomic transition produced in an isolated
//! workspace clone; every player-caused rejection returns the committed state
//! unchanged.

mod helpers;
mod runtime;
mod stages;

pub use runtime::validate_synthetic_runtime_state;

use mtgml_decision::{DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2};
use mtgml_model::{ContinuationId, DecisionId, PlayerId};
use mtgml_state::{
    AssemblyStageV2, ContinuationPayloadV2, ContinuationRecordV2, EngineState,
    EngineStateViolation, PendingDecisionRecordV2,
};

use crate::errors::KernelExecutionError;
use crate::events::AuthoritativeRuleEventKind;
use crate::product::build_accepted_product;
use crate::transition::{RulesKernel, TransitionResult};

// The inclusive ChooseCount interval has a single authority in the state
// crate beside the frozen payload it governs; never redefine it locally.
pub use mtgml_state::{SYNTHETIC_COUNT_MAX, SYNTHETIC_COUNT_MIN};

use helpers::{
    advance_player_allocator, bound_event, cleared_event, created_event, fresh_stage_identity,
    global_stream, piece_candidates, rejected,
};
use runtime::entry_supported;

fn mtgml_rules_validate_runtime(state: &EngineState) -> Result<(), KernelExecutionError> {
    validate_synthetic_runtime_state(state)
}

#[derive(Debug, Default)]
pub struct SyntheticM1RulesKernel;

impl RulesKernel for SyntheticM1RulesKernel {
    fn apply(
        &mut self,
        state: &EngineState,
        trusted_actor: PlayerId,
        response: &DecisionResponseV2,
    ) -> Result<TransitionResult, KernelExecutionError> {
        mtgml_rules_validate_runtime(state)?;

        let Some(pending) = state.execution.pending_decision.as_ref() else {
            return rejected(state);
        };
        let request = &pending.request;
        let Ok(player_request) = request.project_player_request() else {
            return rejected(state);
        };
        // Stale visible identity/revision and wrong answer variants are
        // player-caused rejections decided before any execution.
        if trusted_actor != request.actor
            || response.validate_for(&player_request).is_err()
            || response.state_revision != state.revision
        {
            return rejected(state);
        }

        match (&request.decision, &response.answer) {
            (DecisionDomainV2::ChooseOne, DecisionAnswerV2::SelectOne { .. }) => {
                self.apply_entry(state, response)
            }
            (DecisionDomainV2::ChooseNumber { .. }, DecisionAnswerV2::ChooseNumber { value }) => {
                self.apply_count_stage(state, *value)
            }
            (
                DecisionDomainV2::ChooseMany { .. },
                DecisionAnswerV2::SelectMany { candidate_ids },
            ) => Self::apply_members_stage(state, candidate_ids),
            (DecisionDomainV2::Order { .. }, DecisionAnswerV2::Order { candidate_ids }) => {
                Self::apply_order_stage(state, candidate_ids)
            }
            // Any other domain/answer combination is invalid_answer.
            _ => rejected(state),
        }
    }
}

impl SyntheticM1RulesKernel {
    /// Entry action: the accepted ChooseOne keeps its rule-relevant product
    /// and creates the synthetic assembly continuation with stage 0.
    fn apply_entry(
        &mut self,
        state: &EngineState,
        response: &DecisionResponseV2,
    ) -> Result<TransitionResult, KernelExecutionError> {
        let pending = state.execution.pending_decision.as_ref().expect("checked");
        let request = &pending.request;
        let actor = request.actor;
        if entry_supported(state).is_err() {
            return rejected(state);
        }
        let DecisionAnswerV2::SelectOne { candidate_id } = &response.answer else {
            unreachable!("dispatch guarantees SelectOne");
        };
        if candidate_id.0 != 0 {
            return rejected(state);
        }

        let identity = fresh_stage_identity(state, actor)?;
        let continuation_id = state.allocators.next_continuation_id;
        if state.allocators.next_continuation_id.0 == u64::MAX {
            return Err(KernelExecutionError::Exhaustion("continuation"));
        }
        let stream = global_stream();

        // Deterministic rule-relevant product of the entry acceptance.
        let mut next = state.clone();
        next.revision = identity.revision;
        next.allocators.allocate_effect_id()?;
        let mut events = Vec::new();
        for (from, to) in [(40_i64, 39_i64), (39, 38)] {
            events.push(bound_event(
                state,
                events.len() as u64,
                identity.revision,
                AuthoritativeRuleEventKind::LifeChanged {
                    player: actor,
                    from,
                    to,
                },
            )?);
            next.core
                .players
                .get_mut(&actor)
                .ok_or(KernelExecutionError::AfterState(
                    EngineStateViolation::MissingTurnPlayer,
                ))?
                .life = to;
        }
        let cursor_before = state.random.lookup_stream(&stream)?.next_raw_u64;
        let (value, raw_words_consumed) = next.uniform_below_u64(&stream, 10)?;
        let cursor_after = next.random.lookup_stream(&stream)?.next_raw_u64;
        events.push(bound_event(
            state,
            events.len() as u64,
            identity.revision,
            AuthoritativeRuleEventKind::RandomValueSampled {
                stream,
                bound: 10,
                value,
                raw_words_consumed,
                cursor_before,
                cursor_after,
            },
        )?);
        // Sequential decision cursor transitions inside one atomic product.
        events.push(bound_event(
            state,
            events.len() as u64,
            identity.revision,
            AuthoritativeRuleEventKind::DecisionCleared {
                decision: request.decision_id,
            },
        )?);
        events.push(bound_event(
            state,
            events.len() as u64,
            identity.revision,
            AuthoritativeRuleEventKind::DecisionCreated {
                decision: identity.decision_id,
            },
        )?);

        build_accepted_product(state, next, events, |workspace| {
            workspace.execution.pending_decision = Some(PendingDecisionRecordV2 {
                request: mtgml_decision::AuthoritativeDecisionRequestV2 {
                    decision_id: identity.decision_id,
                    player_decision_id: identity.player_decision_id,
                    state_revision: workspace.revision,
                    actor,
                    visibility: request.visibility,
                    decision: DecisionDomainV2::ChooseNumber {
                        minimum: i64::from(SYNTHETIC_COUNT_MIN),
                        maximum: i64::from(SYNTHETIC_COUNT_MAX),
                    },
                    candidates: Vec::new(),
                    continuation_id: Some(continuation_id),
                },
            });
            workspace.allocators.next_decision_id = DecisionId(identity.decision_id.0 + 1);
            workspace.allocators.next_continuation_id = ContinuationId(continuation_id.0 + 1);
            advance_player_allocator(workspace, actor, identity.player_decision_id)?;
            workspace.execution.continuations.insert(
                continuation_id,
                ContinuationRecordV2 {
                    id: continuation_id,
                    actor,
                    created_at_revision: workspace.revision,
                    stage_index: AssemblyStageV2::ChooseCount.stage_index(),
                    payload: ContinuationPayloadV2::SyntheticM2Assembly {
                        stage: AssemblyStageV2::ChooseCount,
                        selected_count: None,
                        selected_piece_keys: Vec::new(),
                        ordered_piece_keys: Vec::new(),
                    },
                },
            );
            Ok(())
        })
    }

    /// Stage 0: ChooseNumber fixes the member-set cardinality.
    fn apply_count_stage(
        &self,
        state: &EngineState,
        value: i64,
    ) -> Result<TransitionResult, KernelExecutionError> {
        let pending = state.execution.pending_decision.as_ref().expect("checked");
        let request = &pending.request;
        let actor = request.actor;
        let continuation_id = match request.continuation_id {
            Some(id) => id,
            None => return rejected(state),
        };
        match state
            .execution
            .continuations
            .get(&continuation_id)
            .map(|record| &record.payload)
        {
            Some(ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseCount,
                selected_count: None,
                ..
            }) => {}
            _ => return rejected(state),
        }
        let Ok(count) = u32::try_from(value) else {
            return rejected(state);
        };
        if !(SYNTHETIC_COUNT_MIN..=SYNTHETIC_COUNT_MAX).contains(&count) {
            return rejected(state);
        }

        let identity = fresh_stage_identity(state, actor)?;
        let mut next = state.clone();
        next.revision = identity.revision;
        let events = vec![
            cleared_event(state, identity.revision, request.decision_id)?,
            created_event(state, identity.revision, identity.decision_id)?,
        ];

        build_accepted_product(state, next, events, |workspace| {
            let record = workspace
                .execution
                .continuations
                .get_mut(&continuation_id)
                .expect("validated above");
            record.stage_index = AssemblyStageV2::ChooseMembers.stage_index();
            record.payload = ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseMembers,
                selected_count: Some(count),
                selected_piece_keys: Vec::new(),
                ordered_piece_keys: Vec::new(),
            };
            let candidates = piece_candidates(count);
            workspace.execution.pending_decision = Some(PendingDecisionRecordV2 {
                request: mtgml_decision::AuthoritativeDecisionRequestV2 {
                    decision_id: identity.decision_id,
                    player_decision_id: identity.player_decision_id,
                    state_revision: workspace.revision,
                    actor,
                    visibility: request.visibility,
                    decision: DecisionDomainV2::ChooseMany {
                        minimum: count,
                        maximum: count,
                    },
                    candidates,
                    continuation_id: Some(continuation_id),
                },
            });
            workspace.allocators.next_decision_id = DecisionId(identity.decision_id.0 + 1);
            advance_player_allocator(workspace, actor, identity.player_decision_id)?;
            Ok(())
        })
    }
}
