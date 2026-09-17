//! T0 conformance facade contract (M3.T0-01, RED remediation 02).
//!
//! T0 is a *thin private conformance facade over the real Rust kernel*.  It
//! feeds explicit setups and explicit player responses into the authoritative
//! engine and compares exact, independently authored expected products.  It
//! is NOT a rules engine, NOT a Magic capability, NOT a simulator
//! replacement, NOT a card executor, NOT a public protocol, and NOT a wire
//! format.  The authoritative engine remains the sole source of state
//! transitions, legality, decisions, events, RNG, replay, checkpointing, and
//! information semantics.
//!
//! The case-level surface required by this contract is the private
//! implementation in this file; the tests below exercise it exclusively
//! through the real kernel and the real player boundary.
//!
//! Required surface (narrowest form for the selected synthetic cases; the
//! per-step semantic payload is the EXISTING `crate::ConformanceStep`
//! authority, composed, never duplicated):
//!
//! ```text
//! ConformanceCase            named, documented sequence of explicit steps
//! ConformanceStepRef         one existing ConformanceStep (its `response`
//!                            is the explicit response; its exact trusted and
//!                            player-facing expectations are verified by the
//!                            existing `assert_exact_transition`) plus the
//!                            authored environment limit-counter deltas and a
//!                            stable step label
//! ConformanceRejectionStepRef explicit Layer-B player-boundary rejection
//!                            (the rejection happens BEFORE kernel execution;
//!                            no fabricated TransitionResult) carrying the
//!                            submitting actor, the explicit response, and
//!                            the PlayerBoundaryRejectionExpectation
//! PlayerBoundaryRejectionExpectation narrowest composition for a rejected
//!                            player-boundary step (trusted kernel untouched)
//! LimitCounterDeltas         authored per-step environment-counter deltas
//! ConformanceCaseDiagnostic  deterministic first-divergence report built
//!                            from the real comparison product of the first
//!                            failing step (existing ConformanceDifference),
//!                            never assembled by hand
//! run_case(case, controller, endpoints)
//!                            execution entry point: for every step, drives
//!                            the explicit response through BOTH the trusted
//!                            controller path and the player endpoint on one
//!                            deterministic starting checkpoint, proves their
//!                            parity, and compares every actual product
//!                            against the authored ConformanceStep via the
//!                            existing `assert_exact_transition`; returns the
//!                            diagnostic for the FIRST mismatching step
//!                            (Ok(()) when all pass)
//! ```
//!
//! Feasibility mechanism (mtgml-conformance-only GREEN, no production API
//! change): each step executes against ONE starting checkpoint `cp`:
//!
//! ```text
//! fork = controller.fork()                       same exact checkpoint
//! trusted = fork.execute_trusted_response(actor, step.response)
//!                                                trusted TransitionResult on a
//!                                                deterministic fork
//! player_step = endpoints[actor].submit(step.response)
//!                                                real PlayerEndpointHandle
//!                                                boundary -> PlayerStepV2
//! parity: controller.checkpoint().state == trusted.next_state (and exact
//!                                                FullStateDigestV4 equality)
//!                                                before proceeding
//! assert_exact_transition(&cp.state, current_decision, &step.response,
//!                         &trusted, actual_steps{actor: player_step},
//!                         &step.step)             existing stronger authority
//! counters: controller.checkpoint().limit_counters - cp.limit_counters
//!                         == step.expected_limit_counter_deltas
//! ```
//!
//! The rejection witnesses apply the same composition: a stale response is
//! submitted through the real endpoint (Layer-B `StaleDecision` rejection,
//! kernel untouched for the presented answer path).
//!
//! Contract invariants (enforced by review and by the tests below):
//!
//! ```text
//! EXPECTED_VALUES_ARE_LITERAL  expectations are authored data transcribed
//!                              from accepted evidence; RNG golden vectors are
//!                              independently verified against mtgml.rng.v1;
//!                              state digests are execution-verified (the
//!                              expectation is literal, but its provenance is
//!                              the authoritative kernel transition, not an
//!                              independent state constructor)
//! EXECUTION_IS_REQUIRED        a case without run_case against the real
//!                              kernel is not conformance evidence; data
//!                              shapes alone satisfy nothing
//! RESPONSES_ARE_EXPLICIT       every step carries its literal response; no
//!                              default/first/empty/pass selection, no
//!                              malformed-answer repair, no hidden automation
//! NO_SECOND_RULES_ENGINE       the facade never calculates legality, costs,
//!                              payments, replacement/prevention effects,
//!                              trigger order, state-based actions, layers,
//!                              combat legality, or turn progression; it
//!                              submits what the case supplies and compares
//!                              what the kernel returns
//! REJECTION_NONMUTATION        rejected steps must be proven nonmutating by
//!                              capturing the existing complete-fingerprint
//!                              authority before AND after the real
//!                              submission of the invalid response through
//!                              the real trusted path, never a second
//!                              fingerprint implementation and never a
//!                              tautological no-op comparison
//! TRUSTED_ONLY                 the facade and its diagnostics live in this
//!                              conformance crate only; player observation,
//!                              information-state, step, and endpoint
//!                              surfaces gain nothing
//! ```

use std::collections::BTreeMap;

use mtgml_decision::{AuthoritativeDecisionRequestV2, DecisionResponseV2, PlayerDecisionRequestV2};
use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle, TrustedEnvironmentController};
use mtgml_model::PlayerId;
use mtgml_observation::PlayerStepV2;
use mtgml_rules::TransitionResult;

use crate::{assert_exact_transition, ConformanceFailure, ConformanceStep};

/// Authored per-step environment limit-counter deltas for ONE case step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LimitCounterDeltas {
    pub decisions_submitted: u64,
    pub accepted_transitions: u64,
    pub rule_events_emitted: u64,
}

/// One named, documented sequence of explicit steps.
#[derive(Debug, Clone)]
pub struct ConformanceCase {
    pub name: &'static str,
    pub description: &'static str,
    pub steps: Vec<ConformanceCaseStep>,
}

/// A single case step: either an accepted kernel transition verified through
/// both the trusted fork and the real player endpoint, or an explicit Layer-B
/// rejection that happens before any kernel execution.
#[derive(Debug, Clone)]
pub enum ConformanceCaseStep {
    Transition(Box<ConformanceStepRef>),
    LayerBRejection(Box<ConformanceRejectionStepRef>),
}

impl ConformanceCaseStep {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Transition(step) => step.label,
            Self::LayerBRejection(step) => step.label,
        }
    }
}

/// One existing `ConformanceStep` authority (its `response` is the explicit
/// authored response; its trusted and player-facing expectations are verified
/// by the existing `assert_exact_transition`) plus authored environment
/// limit-counter deltas and a stable step label.
#[derive(Debug, Clone)]
pub struct ConformanceStepRef {
    pub label: &'static str,
    pub step: ConformanceStep,
    pub expected_limit_counter_deltas: LimitCounterDeltas,
}

/// An explicit Layer-B player-boundary rejection. The rejection happens
/// BEFORE kernel execution; no fabricated TransitionResult is involved.
#[derive(Debug, Clone)]
pub struct ConformanceRejectionStepRef {
    pub label: &'static str,
    pub actor: PlayerId,
    pub response: DecisionResponseV2,
    pub expectation: PlayerBoundaryRejectionExpectation,
}

/// Narrowest composition for a rejected player-boundary step. The trusted
/// kernel is untouched, so the returned product is the complete mirrored
/// `PlayerStepV2` (submission outcome, episode status, information state
/// including its digest, observed events, and next visible decision) plus the
/// separately re-read preserved visible decision. Binding the WHOLE step is
/// intentional: checking only the closed rejection code would miss a
/// `next_decision` / information-state / observed-event regression on the
/// rejected path, exactly the invariant the accepted path already binds via
/// `expected_player_steps`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerBoundaryRejectionExpectation {
    pub player_step: PlayerStepV2,
    pub preserved_visible_decision: Option<PlayerDecisionRequestV2>,
}

/// Deterministic first-divergence report built from the real comparison
/// product of the first failing step, never assembled by hand. The case name
/// and description document which authored case produced the divergence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceCaseDiagnostic {
    pub case: String,
    pub case_description: String,
    pub step_label: String,
    pub first_failing_step_index: Option<usize>,
    pub failure: ConformanceFailure,
}

/// Execution entry point. For every transition step the explicit response is
/// driven through BOTH the trusted controller fork and the real player
/// endpoint on one deterministic starting checkpoint; their parity is proven
/// before the full authored-product comparison via `assert_exact_transition`;
/// the environment limit-counter deltas are independently compared. Rejection
/// steps are submitted through the real endpoint and proven nonmutating by a
/// complete four-group fingerprint bracket. Returns the diagnostic for the
/// FIRST mismatching step (`Ok(())` when all pass).
pub fn run_case(
    case: &ConformanceCase,
    controller: &TrustedEnvironmentController,
    endpoints: &[PlayerEndpointHandle; 2],
) -> Result<(), Box<ConformanceCaseDiagnostic>> {
    for (index, step) in case.steps.iter().enumerate() {
        if let Err(failure) = run_step(step, controller, endpoints) {
            return Err(Box::new(ConformanceCaseDiagnostic {
                case: case.name.to_string(),
                case_description: case.description.to_string(),
                step_label: step.label().to_string(),
                first_failing_step_index: Some(index),
                failure,
            }));
        }
    }
    Ok(())
}

fn run_step(
    step: &ConformanceCaseStep,
    controller: &TrustedEnvironmentController,
    endpoints: &[PlayerEndpointHandle; 2],
) -> Result<(), ConformanceFailure> {
    match step {
        ConformanceCaseStep::Transition(tr) => run_transition(tr, controller, endpoints),
        ConformanceCaseStep::LayerBRejection(rr) => run_rejection(rr, controller, endpoints),
    }
}

fn infrastructure<E>(error: E) -> ConformanceFailure
where
    E: std::fmt::Display,
{
    ConformanceFailure::Contract(format!("facade infrastructure: {error}"))
}

fn endpoint_for(
    endpoints: &[PlayerEndpointHandle; 2],
    actor: PlayerId,
) -> Result<&PlayerEndpointHandle, ConformanceFailure> {
    endpoints
        .iter()
        .find(|endpoint| endpoint.perspective() == actor)
        .ok_or_else(|| {
            ConformanceFailure::Contract("no bound player endpoint matches the step actor".into())
        })
}

fn run_transition(
    tr: &ConformanceStepRef,
    controller: &TrustedEnvironmentController,
    endpoints: &[PlayerEndpointHandle; 2],
) -> Result<(), ConformanceFailure> {
    let actor = tr
        .step
        .expected_current_decision
        .as_ref()
        .map(|request| request.actor)
        .ok_or_else(|| {
            ConformanceFailure::Contract(
                "transition step lacks the authored current decision that carries the actor".into(),
            )
        })?;
    let endpoint = endpoint_for(endpoints, actor)?;

    let before = controller.checkpoint().map_err(infrastructure)?;
    let current_decision: Option<&AuthoritativeDecisionRequestV2> = before
        .state
        .execution
        .pending_decision
        .as_ref()
        .map(|record| &record.request);

    // Trusted path on a deterministic fork of the SAME starting checkpoint.
    // Explicitly bind: both legs start from identical authoritative state.
    let fork = controller.fork().map_err(infrastructure)?;
    let fork_before = fork.checkpoint().map_err(infrastructure)?;
    if fork_before != before {
        return Err(ConformanceFailure::Contract(
            "trusted fork must start from the identical checkpoint as the shared environment"
                .into(),
        ));
    }
    let trusted: TransitionResult = fork
        .execute_trusted_response(actor, tr.step.response.clone())
        .map_err(infrastructure)?;

    // Real player boundary on the shared environment.
    let player_step = endpoint
        .submit(tr.step.response.clone())
        .map_err(infrastructure)?;

    // Parity: the shared endpoint-mutated instant must equal the trusted
    // fork's product exactly. The complete `EnvironmentCheckpointV4` binds
    // authoritative state, full-state digest, episode status, limit counters,
    // codec identity, and checkpoint digest; the authoritative replay
    // products bind the recorded transition and the terminal identity. The
    // trusted fork is a fresh segment anchored at the identical starting
    // checkpoint, so its recorder holds exactly this one step.
    let after = controller.checkpoint().map_err(infrastructure)?;
    let trusted_digest = trusted
        .next_state
        .digest()
        .map_err(|_| ConformanceFailure::Contract("trusted next-state digest failed".into()))?;
    if after.state != trusted.next_state || after.state_digest != trusted_digest {
        return Err(ConformanceFailure::Contract(
            "trusted fork and real player endpoint diverged (state parity)".into(),
        ));
    }
    let fork_after = fork.checkpoint().map_err(infrastructure)?;
    if fork_after != after {
        return Err(ConformanceFailure::Contract(
            "trusted fork and real player endpoint diverged (checkpoint parity)".into(),
        ));
    }
    let fork_replay = fork.export_replay().map_err(infrastructure)?;
    let main_replay = controller.export_replay().map_err(infrastructure)?;
    if fork_replay.steps.len() != 1 {
        return Err(ConformanceFailure::Contract(
            "trusted fork replay must hold exactly the one executed step".into(),
        ));
    }
    let fork_step = &fork_replay.steps[0];
    let Some(main_step) = main_replay.steps.last() else {
        return Err(ConformanceFailure::Contract(
            "shared environment replay is missing the executed step".into(),
        ));
    };
    if fork_step.step_index != 0 || main_step.state_revision_before != before.state.revision {
        return Err(ConformanceFailure::Contract(
            "replay step identity is inconsistent with the step under test".into(),
        ));
    }
    // The fork records the identical transition in its own segment, so only
    // the segment-local step index may differ. Aligning it lets the whole
    // `ReplayStepV4` be compared instead of hand-picked fields.
    let mut aligned_fork_step = fork_step.clone();
    aligned_fork_step.step_index = main_step.step_index;
    if &aligned_fork_step != main_step || fork_replay.final_identity != main_replay.final_identity {
        return Err(ConformanceFailure::Contract(
            "trusted fork and real player endpoint diverged (replay parity)".into(),
        ));
    }

    let actual_player_steps = BTreeMap::from([(actor, player_step)]);
    assert_exact_transition(
        &before.state,
        current_decision,
        &tr.step.response,
        &trusted,
        &actual_player_steps,
        &tr.step,
    )?;

    let actual_deltas = LimitCounterDeltas {
        decisions_submitted: counter_delta(
            after.limit_counters.decisions_submitted,
            before.limit_counters.decisions_submitted,
        )?,
        accepted_transitions: counter_delta(
            after.limit_counters.accepted_transitions,
            before.limit_counters.accepted_transitions,
        )?,
        rule_events_emitted: counter_delta(
            after.limit_counters.rule_events_emitted,
            before.limit_counters.rule_events_emitted,
        )?,
    };
    if actual_deltas != tr.expected_limit_counter_deltas {
        return Err(ConformanceFailure::Contract(format!(
            "environment limit-counter deltas differed (expected {} decisions, {} accepted, {} events; actual {} / {} / {})",
            tr.expected_limit_counter_deltas.decisions_submitted,
            tr.expected_limit_counter_deltas.accepted_transitions,
            tr.expected_limit_counter_deltas.rule_events_emitted,
            actual_deltas.decisions_submitted,
            actual_deltas.accepted_transitions,
            actual_deltas.rule_events_emitted,
        )));
    }
    Ok(())
}

/// Fail-closed counter progression: a conformance step must never silently
/// fold a counter regression into a legitimate-looking zero delta.
fn counter_delta(after: u64, before: u64) -> Result<u64, ConformanceFailure> {
    after.checked_sub(before).ok_or_else(|| {
        ConformanceFailure::Contract(format!(
            "environment limit counter regressed (before {before}, after {after})"
        ))
    })
}

fn run_rejection(
    rr: &ConformanceRejectionStepRef,
    controller: &TrustedEnvironmentController,
    endpoints: &[PlayerEndpointHandle; 2],
) -> Result<(), ConformanceFailure> {
    let before =
        crate::isolation::capture_complete(controller, endpoints).map_err(infrastructure)?;
    let endpoint = endpoint_for(endpoints, rr.actor)?;

    let step = endpoint
        .submit(rr.response.clone())
        .map_err(infrastructure)?;
    if step != rr.expectation.player_step {
        return Err(ConformanceFailure::Contract(
            "rejected PlayerStepV2 differed from the authored full product".into(),
        ));
    }
    let preserved = endpoint.visible_decision().map_err(infrastructure)?;
    if preserved != rr.expectation.preserved_visible_decision {
        return Err(ConformanceFailure::Contract(
            "rejected submission did not preserve the visible decision".into(),
        ));
    }

    let after =
        crate::isolation::capture_complete(controller, endpoints).map_err(infrastructure)?;
    if before != after {
        return Err(ConformanceFailure::Contract(
            "rejected submission mutated the environment (fingerprint drift)".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod t0_01_red_contract {
    use std::collections::BTreeMap;

    use mtgml_decision::{
        AuthoritativeCandidateV2, AuthoritativeDecisionRequestV2, CandidateIntent,
        DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2, DecisionVisibility,
        EngineCandidateBinding, PlayerDecisionRequestV2, VisibleCandidateV2,
        DECISION_RESPONSE_V2_SCHEMA, PLAYER_DECISION_REQUEST_V2_SCHEMA,
    };
    use mtgml_model::{
        CandidateIdV1, CardDefinitionId, ContinuationId, DecisionId, EpisodeStatus,
        FullStateDigestV4, GameObjectId, InformationStateDigestV2, ObservationDigest,
        OpaqueObjectId, PlayerDecisionIdV1, PlayerId, RuleEventId, StateRevision, VisibleSequence,
        ZoneKind,
    };
    use mtgml_observation::{
        ObservationEnvelope, PlayerInformationStateV2, PlayerKnowledgeProvenanceV1,
        PlayerKnownLocationFactV1, PlayerKnownLocationV1, PlayerKnownObjectV1,
        PlayerStepSubmissionV1, PlayerStepV2, PlayerSubmissionCodeV1, INFORMATION_STATE_SCHEMA_V2,
        OBSERVATION_SCHEMA, PLAYER_STEP_SCHEMA_V2, SYNTHETIC_M3_OBSERVATION_SCHEMA,
    };
    use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
    use mtgml_rules::{AuthoritativeRuleEvent, AuthoritativeRuleEventKind};
    use mtgml_state::SemanticDeltaOperation;

    use crate::facade::{
        run_case, ConformanceCase, ConformanceCaseStep, ConformanceRejectionStepRef,
        ConformanceStepRef, LimitCounterDeltas, PlayerBoundaryRejectionExpectation,
    };

    use crate::isolation::{base_pair_state, capture_complete, spawn_environment};
    use crate::{ConformanceFailureClass, ConformanceStep, ExpectedResponseResult};

    const P1: PlayerId = PlayerId(1);
    const P2: PlayerId = PlayerId(2);

    const SEED_HEX_A: &str = "3333333333333333333333333333333333333333333333333333333333333333";

    fn synthetic_stream() -> RandomStreamKeyV1 {
        RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1)
    }

    fn explicit_entry_response() -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        }
    }

    fn explicit_count_response() -> DecisionResponseV2 {
        DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(2),
            state_revision: StateRevision(1),
            answer: DecisionAnswerV2::ChooseNumber { value: 2 },
        }
    }

    fn expected_entry_events() -> Vec<AuthoritativeRuleEvent> {
        let stream = synthetic_stream();
        vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(1),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::LifeChanged {
                    player: P1,
                    from: 40,
                    to: 39,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(2),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::LifeChanged {
                    player: P1,
                    from: 39,
                    to: 38,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(3),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::RandomValueSampled {
                    stream,
                    bound: 10,
                    value: 2,
                    raw_words_consumed: 1,
                    cursor_before: 0,
                    cursor_after: 1,
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(4),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::DecisionCleared {
                    decision: DecisionId(1),
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(5),
                state_revision: StateRevision(1),
                event: AuthoritativeRuleEventKind::DecisionCreated {
                    decision: DecisionId(2),
                },
            },
        ]
    }

    fn expected_entry_delta() -> Vec<SemanticDeltaOperation> {
        let stream = synthetic_stream();
        vec![
            SemanticDeltaOperation::LifeChanged {
                player: P1,
                from: 40,
                to: 39,
            },
            SemanticDeltaOperation::LifeChanged {
                player: P1,
                from: 39,
                to: 38,
            },
            SemanticDeltaOperation::RandomValueSampled {
                stream,
                bound: 10,
                value: 2,
                raw_words_consumed: 1,
                cursor_before: 0,
                cursor_after: 1,
            },
            SemanticDeltaOperation::DecisionCleared {
                decision: DecisionId(1),
            },
            SemanticDeltaOperation::DecisionCreated {
                decision: DecisionId(2),
            },
        ]
    }

    fn base_authoritative_request() -> AuthoritativeDecisionRequestV2 {
        AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(1),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: P1,
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![AuthoritativeCandidateV2 {
                candidate_id: CandidateIdV1(0),
                visible_intent: CandidateIntent::SelectObject {
                    object: OpaqueObjectId(1),
                },
                trusted_binding: EngineCandidateBinding::SelectObject {
                    object: GameObjectId(1),
                },
            }],
            continuation_id: None,
        }
    }

    fn base_visible_decision() -> PlayerDecisionRequestV2 {
        PlayerDecisionRequestV2 {
            schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(1),
            state_revision: StateRevision(0),
            actor: P1,
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseOne,
            candidates: vec![VisibleCandidateV2 {
                candidate_id: CandidateIdV1(0),
                intent: CandidateIntent::SelectObject {
                    object: OpaqueObjectId(1),
                },
            }],
        }
    }

    fn next_authoritative_choose_number() -> AuthoritativeDecisionRequestV2 {
        AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(2),
            player_decision_id: PlayerDecisionIdV1(2),
            state_revision: StateRevision(1),
            actor: P1,
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
            continuation_id: Some(ContinuationId(1)),
        }
    }

    fn next_visible_choose_number() -> PlayerDecisionRequestV2 {
        PlayerDecisionRequestV2 {
            schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(2),
            state_revision: StateRevision(1),
            actor: P1,
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseNumber {
                minimum: 0,
                maximum: 3,
            },
            candidates: Vec::new(),
        }
    }

    fn base_p1_observation(state_revision: StateRevision) -> ObservationEnvelope {
        let payload_base64 =
            "eyJhY3RpdmVfcGxheWVyIjoiMSIsInByaW9yaXR5Ijp7ImtpbmQiOiJub25lIn0sInNjaGVtYV92ZXJzaW9uIjoic3ludGhldGljLW0zLW9ic2VydmF0aW9uLnYxIiwidHVybl9udW1iZXIiOiIxIiwidHVybl9wb3NpdGlvbiI6eyJraW5kIjoiYmVnaW5uaW5nIiwic3RlcCI6InVudGFwIn19";
        ObservationEnvelope {
            schema_version: OBSERVATION_SCHEMA.into(),
            perspective: P1,
            state_revision,
            payload_codec: SYNTHETIC_M3_OBSERVATION_SCHEMA.into(),
            payload_base64: payload_base64.into(),
            digest: ObservationDigest::parse(
                "9129fde7674374383922b1af48edfccce7dbfdb87a24af79ee77af5ff928e4e5",
            )
            .expect("observation digest"),
        }
    }

    fn base_p1_retained_knowledge() -> Vec<PlayerKnownObjectV1> {
        vec![PlayerKnownObjectV1::Active {
            opaque_object_id: OpaqueObjectId(1),
            known_definition: Some(CardDefinitionId(1)),
            current_known_location_fact: Some(PlayerKnownLocationFactV1 {
                location: PlayerKnownLocationV1 {
                    zone: ZoneKind::Battlefield,
                    player: None,
                },
                provenance: PlayerKnowledgeProvenanceV1::InitialConfiguration,
            }),
            historical_locations: Vec::new(),
            acquisition: PlayerKnowledgeProvenanceV1::InitialConfiguration,
        }]
    }

    fn base_p1_information_state(
        state_revision: StateRevision,
        info_digest_hex: &str,
    ) -> PlayerInformationStateV2 {
        PlayerInformationStateV2 {
            schema_version: INFORMATION_STATE_SCHEMA_V2.into(),
            perspective: P1,
            state_revision,
            current_observation: base_p1_observation(state_revision),
            next_visible_sequence: VisibleSequence(1),
            retained_knowledge: base_p1_retained_knowledge(),
            digest: InformationStateDigestV2::parse(info_digest_hex).expect("info digest"),
        }
    }

    fn entry_accepted_player_step_p1() -> PlayerStepV2 {
        PlayerStepV2 {
            schema_version: PLAYER_STEP_SCHEMA_V2.into(),
            information_state: base_p1_information_state(
                StateRevision(1),
                "e9598c10728b7582aa45cba5533d70e3806cde859c98b4b876f86d0f768aa223",
            ),
            observed_events: Vec::new(),
            next_decision: Some(next_visible_choose_number()),
            status: EpisodeStatus::Running,
            submission: PlayerStepSubmissionV1::Accepted,
        }
    }

    fn entry_rejected_player_step_p1() -> PlayerStepV2 {
        PlayerStepV2 {
            schema_version: PLAYER_STEP_SCHEMA_V2.into(),
            information_state: base_p1_information_state(
                StateRevision(0),
                "1709ad67928e88a80e8a4ea3d0ec2acfdabcdba954830249040d7689bd447eb0",
            ),
            observed_events: Vec::new(),
            next_decision: Some(base_visible_decision()),
            status: EpisodeStatus::Running,
            submission: PlayerStepSubmissionV1::Rejected {
                code: PlayerSubmissionCodeV1::StaleDecision,
            },
        }
    }

    fn expected_player_steps_for_entry() -> BTreeMap<PlayerId, PlayerStepV2> {
        BTreeMap::from([(P1, entry_accepted_player_step_p1())])
    }

    fn expected_count_events() -> Vec<AuthoritativeRuleEvent> {
        vec![
            AuthoritativeRuleEvent {
                event_id: RuleEventId(6),
                state_revision: StateRevision(2),
                event: AuthoritativeRuleEventKind::DecisionCleared {
                    decision: DecisionId(2),
                },
            },
            AuthoritativeRuleEvent {
                event_id: RuleEventId(7),
                state_revision: StateRevision(2),
                event: AuthoritativeRuleEventKind::DecisionCreated {
                    decision: DecisionId(3),
                },
            },
        ]
    }

    fn expected_count_delta() -> Vec<SemanticDeltaOperation> {
        vec![
            SemanticDeltaOperation::DecisionCleared {
                decision: DecisionId(2),
            },
            SemanticDeltaOperation::DecisionCreated {
                decision: DecisionId(3),
            },
        ]
    }

    fn count_next_authoritative_request() -> AuthoritativeDecisionRequestV2 {
        AuthoritativeDecisionRequestV2 {
            decision_id: DecisionId(3),
            player_decision_id: PlayerDecisionIdV1(3),
            state_revision: StateRevision(2),
            actor: P1,
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseMany {
                minimum: 2,
                maximum: 2,
            },
            candidates: vec![
                AuthoritativeCandidateV2 {
                    candidate_id: CandidateIdV1(0),
                    visible_intent: CandidateIntent::SelectMode { mode_index: 0 },
                    trusted_binding: EngineCandidateBinding::SelectMode { mode_index: 0 },
                },
                AuthoritativeCandidateV2 {
                    candidate_id: CandidateIdV1(1),
                    visible_intent: CandidateIntent::SelectMode { mode_index: 1 },
                    trusted_binding: EngineCandidateBinding::SelectMode { mode_index: 1 },
                },
            ],
            continuation_id: Some(ContinuationId(1)),
        }
    }

    fn count_next_visible_request() -> PlayerDecisionRequestV2 {
        PlayerDecisionRequestV2 {
            schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.into(),
            player_decision_id: PlayerDecisionIdV1(3),
            state_revision: StateRevision(2),
            actor: P1,
            visibility: DecisionVisibility::Public,
            decision: DecisionDomainV2::ChooseMany {
                minimum: 2,
                maximum: 2,
            },
            candidates: vec![
                VisibleCandidateV2 {
                    candidate_id: CandidateIdV1(0),
                    intent: CandidateIntent::SelectMode { mode_index: 0 },
                },
                VisibleCandidateV2 {
                    candidate_id: CandidateIdV1(1),
                    intent: CandidateIntent::SelectMode { mode_index: 1 },
                },
            ],
        }
    }

    fn count_accepted_player_step_p1() -> PlayerStepV2 {
        PlayerStepV2 {
            schema_version: PLAYER_STEP_SCHEMA_V2.into(),
            information_state: base_p1_information_state(
                StateRevision(2),
                "7cb3660ad7fc0fa99c440735da8e8524b35dfd8166a40d5f990d4d2cf9f1f280",
            ),
            observed_events: Vec::new(),
            next_decision: Some(count_next_visible_request()),
            status: EpisodeStatus::Running,
            submission: PlayerStepSubmissionV1::Accepted,
        }
    }

    fn entry_expected_state_digest() -> FullStateDigestV4 {
        FullStateDigestV4::parse("4bf8babd2e5661bd64e84e4830dae09c633d0db348e4df7e75915a15c6c1d3e7")
            .expect("entry digest")
    }

    fn count_expected_state_digest() -> FullStateDigestV4 {
        FullStateDigestV4::parse("03e13400f71135656196ea83b00bad1821df341ea8b9a03634e45fc0ae83ed0a")
            .expect("count digest")
    }

    fn entry_conformance_step() -> ConformanceStep {
        ConformanceStep {
            expected_current_decision: Some(base_authoritative_request()),
            response: explicit_entry_response(),
            expected_response_result: ExpectedResponseResult::Accepted,
            expected_authoritative_events: expected_entry_events(),
            expected_semantic_delta: expected_entry_delta(),
            expected_state_digest: entry_expected_state_digest(),
            expected_next_decision: Some(next_authoritative_choose_number()),
            expected_player_steps: expected_player_steps_for_entry(),
            expected_status: EpisodeStatus::Running,
        }
    }

    fn count_conformance_step() -> ConformanceStep {
        ConformanceStep {
            expected_current_decision: Some(next_authoritative_choose_number()),
            response: explicit_count_response(),
            expected_response_result: ExpectedResponseResult::Accepted,
            expected_authoritative_events: expected_count_events(),
            expected_semantic_delta: expected_count_delta(),
            expected_state_digest: count_expected_state_digest(),
            expected_next_decision: Some(count_next_authoritative_request()),
            expected_player_steps: BTreeMap::from([(P1, count_accepted_player_step_p1())]),
            expected_status: EpisodeStatus::Running,
        }
    }

    fn entry_case() -> ConformanceCase {
        ConformanceCase {
            name: "synthetic-entry-choose-one-accepted",
            description: "one explicit ChooseOne response commits the accepted V4 transition",
            steps: vec![ConformanceCaseStep::Transition(Box::new(
                ConformanceStepRef {
                    label: "step-1-entry-choose-one",
                    step: entry_conformance_step(),
                    expected_limit_counter_deltas: LimitCounterDeltas {
                        decisions_submitted: 1,
                        accepted_transitions: 1,
                        rule_events_emitted: 5,
                    },
                },
            ))],
        }
    }

    fn witness_a_case() -> ConformanceCase {
        entry_case()
    }

    #[test]
    fn witness_a_executes_accepted_entry_transition_against_authored_products() {
        let case = witness_a_case();

        let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("spawned trusted environment");

        run_case(&case, &controller, &endpoints)
            .expect("authored entry expectations must match the authoritative kernel");
    }

    #[test]
    fn witness_b_rejected_submission_is_bracketed_by_fingerprints() {
        let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("spawned trusted environment");

        let case = ConformanceCase {
            name: "synthetic-entry-stale-revision-rejected",
            description: "a stale-revision response is rejected without any mutation",
            steps: vec![ConformanceCaseStep::LayerBRejection(Box::new(
                ConformanceRejectionStepRef {
                    label: "step-1-stale-revision-rejected",
                    actor: P1,
                    response: DecisionResponseV2 {
                        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
                        player_decision_id: PlayerDecisionIdV1(1),
                        state_revision: StateRevision(5),
                        answer: DecisionAnswerV2::SelectOne {
                            candidate_id: CandidateIdV1(0),
                        },
                    },
                    expectation: PlayerBoundaryRejectionExpectation {
                        player_step: entry_rejected_player_step_p1(),
                        preserved_visible_decision: Some(base_visible_decision()),
                    },
                },
            ))],
        };

        let before = capture_complete(&controller, &endpoints).expect("fingerprint capture");
        run_case(&case, &controller, &endpoints)
            .expect("the authored rejection expectations must hold");
        let after = capture_complete(&controller, &endpoints).expect("fingerprint capture");
        assert_eq!(before, after);
    }

    #[test]
    fn witness_c_multi_step_mismatch_reports_first_failing_step() {
        let case = ConformanceCase {
            name: "synthetic-assembly-two-explicit-steps",
            description: "ChooseOne then ChooseNumber; step-2 expectation deliberately wrong",
            steps: vec![
                ConformanceCaseStep::Transition(Box::new(ConformanceStepRef {
                    label: "step-1-entry-choose-one",
                    step: entry_conformance_step(),
                    expected_limit_counter_deltas: LimitCounterDeltas {
                        decisions_submitted: 1,
                        accepted_transitions: 1,
                        rule_events_emitted: 5,
                    },
                })),
                ConformanceCaseStep::Transition(Box::new(ConformanceStepRef {
                    label: "step-2-choose-count",
                    step: {
                        let mut step = count_conformance_step();
                        step.expected_authoritative_events[1].event =
                            AuthoritativeRuleEventKind::DecisionCreated {
                                decision: DecisionId(99),
                            };
                        step.expected_semantic_delta[1] = SemanticDeltaOperation::DecisionCreated {
                            decision: DecisionId(99),
                        };
                        step
                    },
                    expected_limit_counter_deltas: LimitCounterDeltas {
                        decisions_submitted: 1,
                        accepted_transitions: 1,
                        rule_events_emitted: 2,
                    },
                })),
            ],
        };

        let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("spawned trusted environment");

        let diagnostic = run_case(&case, &controller, &endpoints)
            .expect_err("the deliberate step-2 mismatch must fail the case");
        assert_eq!(diagnostic.case, case.name);
        assert_eq!(diagnostic.step_label, "step-2-choose-count");
        assert_eq!(diagnostic.first_failing_step_index, Some(1));
        let failure = diagnostic.failure;
        let classification = failure.classification().expect("detailed failure");
        assert_eq!(classification, ConformanceFailureClass::Events);
        let difference = failure.difference().expect("detailed difference");
        assert_eq!(difference.semantic_path, "transition.events[1]");
    }

    #[test]
    fn diagnostics_are_deterministic_across_reruns() {
        let mismatched_case = {
            let mut step = entry_conformance_step();
            step.expected_authoritative_events[1].event = AuthoritativeRuleEventKind::LifeChanged {
                player: P1,
                from: 39,
                to: 37,
            };
            ConformanceCase {
                name: "deterministic-diagnostic-case",
                description: "corrupted second life event must fail deterministically",
                steps: vec![ConformanceCaseStep::Transition(Box::new(
                    ConformanceStepRef {
                        label: "step-1-entry-choose-one",
                        step,
                        expected_limit_counter_deltas: LimitCounterDeltas {
                            decisions_submitted: 1,
                            accepted_transitions: 1,
                            rule_events_emitted: 5,
                        },
                    },
                ))],
            }
        };

        let run = || {
            let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
            let config = crate::isolation::synthetic_environment_config([P1, P2]);
            let (controller, endpoints) =
                spawn_environment(state, &config).expect("spawned trusted environment");
            run_case(&mismatched_case, &controller, &endpoints)
                .expect_err("the corrupted expectation must fail the case")
        };

        let first = run();
        let second = run();
        assert_eq!(first, second);
        assert_eq!(first.case, mismatched_case.name);
        assert_eq!(first.step_label, "step-1-entry-choose-one");
        assert_eq!(first.first_failing_step_index, Some(0));
        let failure = first.failure;
        let classification = failure.classification().expect("detailed failure");
        assert_eq!(classification, ConformanceFailureClass::Events);
        let difference = failure.difference().expect("detailed difference");
        assert_eq!(difference.semantic_path, "transition.events[1]");
    }

    #[test]
    fn rng_golden_matches_independently_verified_mtgml_rng_v1() {
        use mtgml_random::sampling::uniform_below_u64;
        use mtgml_random::{RandomStreamCursorV1, RootSeed256};

        let root_seed = RootSeed256::from_lower_hex(SEED_HEX_A).expect("valid hex root seed");
        let stream_key = synthetic_stream();
        let cursor = RandomStreamCursorV1::default();

        let bound: u64 = 10;
        let threshold = ((1u128 << 64) % (bound as u128)) as u64;
        assert_eq!(
            threshold, 6,
            "mtgml.rng.v1: threshold for bound=10 is exactly 6"
        );

        let (rng_value, raw_words_consumed, cursor_after) =
            uniform_below_u64(&root_seed, &stream_key, &cursor, bound)
                .expect("RNG derivation per mtgml.rng.v1");

        assert_eq!(rng_value, 2, "RNG golden value for bound 10 must be 2");
        assert_eq!(raw_words_consumed, 1, "exactly one raw word consumed");
        assert_eq!(cursor_after.next_raw_u64, 1, "cursor advanced to 1");
    }

    #[test]
    fn state_digest_golden_matches_execution_verified_transition() {
        let state = base_pair_state(SEED_HEX_A).expect("base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("environment spawned");

        let case = witness_a_case();
        run_case(&case, &controller, &endpoints).expect("witness case must execute successfully");

        let after = controller.checkpoint().expect("checkpoint");
        let observed_digest = after.state_digest;
        let authored_digest = entry_expected_state_digest();

        assert_eq!(
            observed_digest, authored_digest,
            "state digest after execution must equal authored golden expectation"
        );
    }

    #[test]
    fn responses_are_explicit_and_never_default_selected() {
        let case = entry_case();
        let ConformanceCaseStep::Transition(ref tr) = case.steps[0] else {
            panic!("first step must be an accepted transition");
        };
        assert_eq!(tr.step.response.player_decision_id, PlayerDecisionIdV1(1));
        assert_eq!(
            tr.step.response.answer,
            DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0)
            }
        );
        assert_eq!(tr.step.response.state_revision, StateRevision(0));
    }

    #[test]
    fn facade_support_stays_behind_the_trusted_boundary() {
        let state = base_pair_state(SEED_HEX_A).expect("accepted synthetic base state");
        let config = crate::isolation::synthetic_environment_config([P1, P2]);
        let (controller, endpoints) =
            spawn_environment(state, &config).expect("spawned trusted environment");
        let complete = capture_complete(&controller, &endpoints).expect("trusted fingerprint");
        assert_eq!(complete, complete);
    }
}
