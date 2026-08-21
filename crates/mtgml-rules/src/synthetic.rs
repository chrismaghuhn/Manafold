use mtgml_decision::{
    validate_candidate_binding, ActionCandidate, CandidateIntent, DecisionAnswerV2,
    DecisionDomainV2, DecisionResponseV2, EngineCandidateBinding, PerspectiveIdentityResolver,
};
use mtgml_model::{EpisodeStatus, GameObjectId, PlayerId, RuleEventId, StateRevision};
use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
use mtgml_state::{validate_engine_state, EngineState, EngineStateViolation, StateDelta};

use crate::errors::KernelExecutionError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
use crate::transition::{RulesKernel, TransitionResult};
use crate::validate_transition_contract;

#[derive(Debug, Default)]
pub struct SyntheticM1RulesKernel;

impl RulesKernel for SyntheticM1RulesKernel {
    fn apply(
        &mut self,
        state: &EngineState,
        trusted_actor: PlayerId,
        response: &DecisionResponseV2,
    ) -> Result<TransitionResult, KernelExecutionError> {
        validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;

        let Some(pending) = state.execution.pending_decision.as_ref() else {
            return rejected(state);
        };
        let request = &pending.request;
        let actor = request.actor;
        let Ok(player_request) = request.project_player_request() else {
            return rejected(state);
        };
        if trusted_actor != request.actor
            || response.validate_for(&player_request).is_err()
            || response.state_revision != state.revision
            || !matches!(request.decision, DecisionDomainV2::ChooseOne)
            || request.continuation_id.is_some()
            || request.candidates.len() != 1
        {
            return rejected(state);
        }

        let DecisionAnswerV2::SelectOne { candidate_id } = &response.answer else {
            return rejected(state);
        };
        if candidate_id.0 != 0 {
            return rejected(state);
        }
        let candidate = &request.candidates[0];
        let CandidateIntent::SelectObject {
            object: opaque_object,
        } = &candidate.visible_intent
        else {
            return rejected(state);
        };
        if state
            .perspective_identities
            .resolve_object(trusted_actor, *opaque_object)
            != Some(GameObjectId(1))
        {
            return rejected(state);
        }
        let binding = &candidate.trusted_binding;
        if binding
            != &(EngineCandidateBinding::SelectObject {
                object: GameObjectId(1),
            })
        {
            return rejected(state);
        }
        let visible = ActionCandidate {
            candidate_id: candidate.candidate_id.to_string(),
            semantic_key: "candidate.0".into(),
            intent: candidate.visible_intent.clone(),
        };
        if validate_candidate_binding(
            &visible,
            binding,
            trusted_actor,
            &state.perspective_identities,
        )
        .is_err()
        {
            return rejected(state);
        }

        let stream = RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1);
        if !state.random.streams.contains_key(&stream) {
            return rejected(state);
        }
        if state.core.players.get(&actor).map(|player| player.life) != Some(40) {
            return rejected(state);
        }

        let mut next_state = state.clone();
        next_state.allocators.allocate_effect_id()?;
        let next_revision = state
            .revision
            .0
            .checked_add(1)
            .ok_or(KernelExecutionError::RevisionOverflow)?;
        let first_event_id = state.allocators.next_rule_event_id;
        let second_event_id = RuleEventId(
            first_event_id
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
        );
        let third_event_id = RuleEventId(
            second_event_id
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
        );
        let fourth_event_id = RuleEventId(
            third_event_id
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
        );
        let next_event_id = RuleEventId(
            fourth_event_id
                .0
                .checked_add(1)
                .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
        );

        next_state.revision = StateRevision(next_revision);
        next_state
            .core
            .players
            .get_mut(&actor)
            .ok_or(KernelExecutionError::AfterState(
                EngineStateViolation::MissingTurnPlayer,
            ))?
            .life = 39;
        let mut events = vec![AuthoritativeRuleEvent {
            event_id: first_event_id,
            state_revision: next_state.revision,
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: actor,
                from: 40,
                to: 39,
            },
        }];
        next_state
            .core
            .players
            .get_mut(&actor)
            .ok_or(KernelExecutionError::AfterState(
                EngineStateViolation::MissingTurnPlayer,
            ))?
            .life = 38;
        events.push(AuthoritativeRuleEvent {
            event_id: second_event_id,
            state_revision: next_state.revision,
            event: AuthoritativeRuleEventKind::LifeChanged {
                player: actor,
                from: 39,
                to: 38,
            },
        });
        let cursor_before = next_state.random.lookup_stream(&stream)?.next_raw_u64;
        let (value, raw_words_consumed) = next_state.uniform_below_u64(&stream, 10)?;
        let cursor_after = next_state.random.lookup_stream(&stream)?.next_raw_u64;
        events.push(AuthoritativeRuleEvent {
            event_id: third_event_id,
            state_revision: next_state.revision,
            event: AuthoritativeRuleEventKind::RandomValueSampled {
                stream,
                bound: 10,
                value,
                raw_words_consumed,
                cursor_before,
                cursor_after,
            },
        });
        next_state.execution.pending_decision = None;
        events.push(AuthoritativeRuleEvent {
            event_id: fourth_event_id,
            state_revision: next_state.revision,
            event: AuthoritativeRuleEventKind::DecisionCleared {
                decision: request.decision_id,
            },
        });
        next_state.allocators.next_rule_event_id = next_event_id;
        let audit = events
            .iter()
            .map(|event| event.event.semantic_delta())
            .collect();
        let delta =
            StateDelta::between(state, &next_state, audit).map_err(KernelExecutionError::Delta)?;
        let result = TransitionResult {
            accepted: true,
            next_state,
            delta,
            events,
            next_decision: None,
            status: EpisodeStatus::Running,
        };

        validate_engine_state(&result.next_state).map_err(KernelExecutionError::AfterState)?;
        validate_transition_contract(state, &result)
            .map_err(KernelExecutionError::TransitionContract)?;
        Ok(result)
    }
}

fn rejected(state: &EngineState) -> Result<TransitionResult, KernelExecutionError> {
    let result = TransitionResult {
        accepted: false,
        next_state: state.clone(),
        delta: StateDelta::between(state, state, vec![]).map_err(KernelExecutionError::Delta)?,
        events: vec![],
        next_decision: state
            .execution
            .pending_decision
            .as_ref()
            .map(|record| record.request.clone()),
        status: EpisodeStatus::Running,
    };
    validate_transition_contract(state, &result)
        .map_err(KernelExecutionError::TransitionContract)?;
    Ok(result)
}
