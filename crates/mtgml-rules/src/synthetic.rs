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

use mtgml_decision::{
    validate_candidate_binding, ActionCandidate, CandidateIntent, DecisionAnswerV2,
    DecisionDomainV2, DecisionResponseV2, EngineCandidateBinding, PerspectiveIdentityResolver,
};
use mtgml_model::{
    CandidateIdV1, ContinuationId, DecisionId, EpisodeStatus, GameObjectId, PlayerDecisionIdV1,
    PlayerId, RuleEventId, StateRevision,
};
use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
use mtgml_state::{
    AssemblyStageV2, ContinuationPayloadV2, ContinuationRecordV2, EngineState,
    EngineStateViolation, PendingDecisionRecordV2, StateDelta,
};

use crate::errors::KernelExecutionError;
use crate::events::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
use crate::product::build_accepted_product;
use crate::transition::{RulesKernel, TransitionResult};
use crate::validate_transition_contract;

// The inclusive ChooseCount interval has a single authority in the state
// crate beside the frozen payload it governs; never redefine it locally.
pub use mtgml_state::{SYNTHETIC_COUNT_MAX, SYNTHETIC_COUNT_MIN};

fn mtgml_rules_validate_runtime(state: &EngineState) -> Result<(), KernelExecutionError> {
    validate_synthetic_runtime_state(state)
}

fn global_stream() -> RandomStreamKeyV1 {
    RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1)
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

/// Context legality of the current synthetic program over a structurally
/// valid `EngineState`.
///
/// `validate_engine_state()` proves generic authoritative invariants; this
/// proof additionally guarantees that every decision this environment can
/// offer is actually executable by the kernel Ã¢â‚¬â€ including the standalone
/// root decision. An unsupported engine-offered path is an internal
/// soundness failure, never a player rejection.
pub fn validate_synthetic_runtime_state(state: &EngineState) -> Result<(), KernelExecutionError> {
    mtgml_state::validate_engine_state(state).map_err(KernelExecutionError::BeforeState)?;
    match state.execution.pending_decision.as_ref() {
        None => {
            // A completed environment holds neither continuation nor request.
            if !state.execution.continuations.is_empty() {
                Err(KernelExecutionError::UnsupportedStagePath)
            } else {
                Ok(())
            }
        }
        Some(pending) => {
            if pending.request.continuation_id.is_some() {
                // Stage requests are fully bound by state-level program
                // coherence, and every reachable stage is supported here.
                Ok(())
            } else {
                // Standalone decisions are only supported as the exact
                // synthetic entry program.
                entry_supported(state)
            }
        }
    }
}

/// Whether the committed state offers the one supported synthetic entry
/// decision with all preconditions the kernel needs to accept its answer.
fn entry_supported(state: &EngineState) -> Result<(), KernelExecutionError> {
    let Some(pending) = state.execution.pending_decision.as_ref() else {
        return Err(KernelExecutionError::UnsupportedStagePath);
    };
    let request = &pending.request;
    let actor = request.actor;
    if !matches!(request.decision, DecisionDomainV2::ChooseOne) || request.candidates.len() != 1 {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    let candidate = &request.candidates[0];
    let CandidateIntent::SelectObject {
        object: opaque_object,
    } = &candidate.visible_intent
    else {
        return Err(KernelExecutionError::UnsupportedStagePath);
    };
    if state
        .perspective_identities
        .resolve_object(actor, *opaque_object)
        != Some(GameObjectId(1))
        || candidate.trusted_binding
            != (EngineCandidateBinding::SelectObject {
                object: GameObjectId(1),
            })
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    let visible = ActionCandidate {
        candidate_id: candidate.candidate_id.to_string(),
        semantic_key: "candidate.0".into(),
        intent: candidate.visible_intent.clone(),
    };
    if validate_candidate_binding(
        &visible,
        &candidate.trusted_binding,
        actor,
        &state.perspective_identities,
    )
    .is_err()
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    let stream = global_stream();
    if !state.random.streams.contains_key(&stream)
        || state.core.players.get(&actor).map(|player| player.life) != Some(40)
    {
        return Err(KernelExecutionError::UnsupportedStagePath);
    }
    Ok(())
}

struct StageIdentity {
    revision: StateRevision,
    decision_id: DecisionId,
    player_decision_id: PlayerDecisionIdV1,
}

/// Advances only the authoritative revision; completion needs nothing else.
fn next_revision(state: &EngineState) -> Result<StateRevision, KernelExecutionError> {
    Ok(StateRevision(
        state
            .revision
            .0
            .checked_add(1)
            .ok_or(KernelExecutionError::RevisionOverflow)?,
    ))
}

/// Allocates every fresh identity up front: exhaustion must be detected
/// before a workspace is created or mutated. Only stages that actually
/// expose a new decision may call this.
fn fresh_stage_identity(
    state: &EngineState,
    actor: PlayerId,
) -> Result<StageIdentity, KernelExecutionError> {
    let revision = next_revision(state)?;
    let decision_id = state.allocators.next_decision_id;
    if state.allocators.next_decision_id.0 == u64::MAX {
        return Err(KernelExecutionError::Exhaustion("decision"));
    }
    let identity = state
        .perspective_identities
        .players
        .get(&actor)
        .ok_or(KernelExecutionError::Exhaustion("perspective"))?;
    let player_decision_id = identity.next_player_decision_id;
    if identity.next_player_decision_id.0 == u64::MAX {
        return Err(KernelExecutionError::Exhaustion("player_decision"));
    }
    Ok(StageIdentity {
        revision,
        decision_id,
        player_decision_id,
    })
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

    /// Stage 1: ChooseMembers fixes the unordered member set.
    fn apply_members_stage(
        state: &EngineState,
        candidate_ids: &[CandidateIdV1],
    ) -> Result<TransitionResult, KernelExecutionError> {
        let pending = state.execution.pending_decision.as_ref().expect("checked");
        let request = &pending.request;
        let actor = request.actor;
        let continuation_id = match request.continuation_id {
            Some(id) => id,
            None => return rejected(state),
        };
        let selected_count = match state
            .execution
            .continuations
            .get(&continuation_id)
            .map(|record| &record.payload)
        {
            Some(ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::ChooseMembers,
                selected_count: Some(count),
                selected_piece_keys,
                ordered_piece_keys,
            }) if selected_piece_keys.is_empty() && ordered_piece_keys.is_empty() => *count,
            _ => return rejected(state),
        };
        // The pending request bounds equal the authoritative partial count
        // (state-level program coherence), and the answer was validated
        // against them before dispatch - so the answer length must equal the
        // persisted authoritative count.
        if candidate_ids.len() != selected_count as usize {
            return rejected(state);
        }
        let mut selected_piece_keys = Vec::with_capacity(candidate_ids.len());
        for candidate_id in candidate_ids {
            selected_piece_keys.push(answered_piece(request, *candidate_id)?);
        }
        selected_piece_keys.sort_unstable();

        let identity = fresh_stage_identity(state, actor)?;
        let mut next = state.clone();
        next.revision = identity.revision;
        let events = vec![
            cleared_event(state, identity.revision, request.decision_id)?,
            created_event(state, identity.revision, identity.decision_id)?,
        ];
        let candidates = piece_candidates_from(&selected_piece_keys);

        build_accepted_product(state, next, events, move |workspace| {
            let record = workspace
                .execution
                .continuations
                .get_mut(&continuation_id)
                .expect("validated above");
            record.stage_index = AssemblyStageV2::OrderMembers.stage_index();
            record.payload = ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::OrderMembers,
                // Persist the authoritative partial value unchanged.
                selected_count: Some(selected_count),
                selected_piece_keys: selected_piece_keys.clone(),
                ordered_piece_keys: Vec::new(),
            };
            workspace.execution.pending_decision = Some(PendingDecisionRecordV2 {
                request: mtgml_decision::AuthoritativeDecisionRequestV2 {
                    decision_id: identity.decision_id,
                    player_decision_id: identity.player_decision_id,
                    state_revision: workspace.revision,
                    actor,
                    visibility: request.visibility,
                    decision: DecisionDomainV2::Order {
                        minimum: selected_count,
                        maximum: selected_count,
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

    /// Stage 2: Order supplies the semantic sequence and completes the
    /// continuation.
    fn apply_order_stage(
        state: &EngineState,
        candidate_ids: &[CandidateIdV1],
    ) -> Result<TransitionResult, KernelExecutionError> {
        let pending = state.execution.pending_decision.as_ref().expect("checked");
        let request = &pending.request;
        let continuation_id = match request.continuation_id {
            Some(id) => id,
            None => return rejected(state),
        };
        let selected_piece_keys = match state
            .execution
            .continuations
            .get(&continuation_id)
            .map(|record| &record.payload)
        {
            Some(ContinuationPayloadV2::SyntheticM2Assembly {
                stage: AssemblyStageV2::OrderMembers,
                selected_piece_keys,
                ordered_piece_keys: persisted,
                ..
            }) if persisted.is_empty() => selected_piece_keys.clone(),
            _ => return rejected(state),
        };
        let mut ordered_piece_keys = Vec::with_capacity(candidate_ids.len());
        for candidate_id in candidate_ids {
            ordered_piece_keys.push(answered_piece(request, *candidate_id)?);
        }
        let mut answered_set = ordered_piece_keys.clone();
        answered_set.sort_unstable();
        answered_set.dedup();
        if answered_set != selected_piece_keys || ordered_piece_keys.len() != answered_set.len() {
            return rejected(state);
        }

        // Completion consumes no fresh decision or visible identity: it must
        // remain possible even when both allocator cursors are exhausted.
        let revision = next_revision(state)?;
        let mut next = state.clone();
        next.revision = revision;
        let events = vec![cleared_event(state, revision, request.decision_id)?];

        build_accepted_product(state, next, events, move |workspace| {
            workspace.execution.continuations.remove(&continuation_id);
            workspace.execution.pending_decision = None;
            Ok(())
        })
    }
}

fn answered_piece(
    request: &mtgml_decision::AuthoritativeDecisionRequestV2,
    candidate_id: CandidateIdV1,
) -> Result<u32, KernelExecutionError> {
    let candidate = request.candidates.get(candidate_id.0 as usize);
    let piece = candidate.and_then(|candidate| {
        if candidate.candidate_id != candidate_id {
            return None;
        }
        match &candidate.visible_intent {
            CandidateIntent::SelectMode { mode_index } => Some(*mode_index),
            _ => None,
        }
    });
    // A dense authoritative request always contains every answered ID; a
    // miss means the engine-offered path is inconsistent (internal).
    piece.ok_or(KernelExecutionError::UnsupportedStagePath)
}

fn piece_candidates_from(pieces: &[u32]) -> Vec<mtgml_decision::AuthoritativeCandidateV2> {
    let pairs = pieces
        .iter()
        .map(|piece| {
            (
                CandidateIntent::SelectMode { mode_index: *piece },
                EngineCandidateBinding::SelectMode { mode_index: *piece },
            )
        })
        .collect();
    mtgml_decision::CandidateOrderingV1::assign_dense(pairs)
        .expect("selected pieces are distinct public ordering keys")
}

fn piece_candidates(count: u32) -> Vec<mtgml_decision::AuthoritativeCandidateV2> {
    let pairs = (0..count)
        .map(|piece| {
            (
                CandidateIntent::SelectMode { mode_index: piece },
                EngineCandidateBinding::SelectMode { mode_index: piece },
            )
        })
        .collect();
    mtgml_decision::CandidateOrderingV1::assign_dense(pairs)
        .expect("generated pieces are distinct public ordering keys")
}

fn bound_event(
    state: &EngineState,
    offset: u64,
    revision: StateRevision,
    kind: AuthoritativeRuleEventKind,
) -> Result<AuthoritativeRuleEvent, KernelExecutionError> {
    Ok(AuthoritativeRuleEvent {
        event_id: RuleEventId(
            state
                .allocators
                .next_rule_event_id
                .0
                .checked_add(offset)
                .ok_or(KernelExecutionError::RuleEventIdOverflow)?,
        ),
        state_revision: revision,
        event: kind,
    })
}

fn cleared_event(
    state: &EngineState,
    revision: StateRevision,
    decision: DecisionId,
) -> Result<AuthoritativeRuleEvent, KernelExecutionError> {
    bound_event(
        state,
        0,
        revision,
        AuthoritativeRuleEventKind::DecisionCleared { decision },
    )
}

fn created_event(
    state: &EngineState,
    revision: StateRevision,
    decision: DecisionId,
) -> Result<AuthoritativeRuleEvent, KernelExecutionError> {
    bound_event(
        state,
        1,
        revision,
        AuthoritativeRuleEventKind::DecisionCreated { decision },
    )
}

fn advance_player_allocator(
    workspace: &mut EngineState,
    actor: PlayerId,
    issued: PlayerDecisionIdV1,
) -> Result<(), KernelExecutionError> {
    let identity = workspace
        .perspective_identities
        .players
        .get_mut(&actor)
        .ok_or(KernelExecutionError::AfterState(
            EngineStateViolation::MissingTurnPlayer,
        ))?;
    identity.next_player_decision_id = PlayerDecisionIdV1(issued.0 + 1);
    Ok(())
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
