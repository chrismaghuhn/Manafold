//! Validity-gated leak mutants for the paired-state matrix.
//!
//! Universal invariant (M2.G plan §E.4): a mutant receives REAL production
//! outputs plus the trusted context and returns a mutated output that REMAINS
//! a valid canonical product of the public contract — one coherent typed
//! change, dependent PUBLIC digests recomputed via
//! `compute_information_state_digest_v2`, `encode_canonical` succeeding and
//! canonical decode/validation passing — BEFORE any byte inequality counts as
//! leak detection. A mutant whose planned channel turns out structurally
//! impossible under that gate documents the precise validator reason below
//! and uses a designated fallback family (`next_visible_sequence` low-bit
//! stamping or the free-form observation payload string region); such
//! substitutions are reported in the slice summary and never loosen
//! validity.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_decision::PlayerDecisionRequestV2;
use mtgml_model::{
    CardDefinitionId, GameObjectId, InformationStateDigestV2, ObservationDigest, OpaqueObjectId,
    PlayerDecisionIdV1, PlayerId, VisibleSequence, ZoneKind,
};
use mtgml_observation::{
    ObservationEnvelope, PlayerInformationStateV2, PlayerKnowledgeCauseV1,
    PlayerKnowledgeChannelV1, PlayerKnowledgeProvenanceV1, PlayerKnownObjectV1,
    PlayerStepSubmissionV1, PlayerStepV2, PlayerSubmissionCodeV1,
};
use mtgml_random::{RandomStreamKeyV1, RandomStreamKindV1};
use mtgml_state::{
    EngineState, IdentityAllocatorState, KnowledgeAcquisitionCause, KnowledgeAcquisitionReason,
    KnowledgeHistoryChannel, PerspectiveIdentityRecordV2, VisibilityPartition, ZoneKey,
    ZonePosition,
};
use mtgml_wire::{compute_information_state_digest_v2, encode_canonical};

use super::HarnessError;

const P2: PlayerId = PlayerId(2);

/// One perspective's real typed boundary outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealOutputs {
    pub information_state: PlayerInformationStateV2,
    pub visible_decision: Option<PlayerDecisionRequestV2>,
}

/// Encoded comparable surfaces of one output set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceBytes {
    pub information_state_bytes: Vec<u8>,
    pub visible_decision_bytes: Option<Vec<u8>>,
    pub information_digest: InformationStateDigestV2,
    pub next_visible_sequence: VisibleSequence,
}

impl RealOutputs {
    /// Canonical encoded surfaces; encoding itself validates the wire
    /// contract including the persisted information-state digest.
    pub fn surfaces(&self) -> Result<SurfaceBytes, HarnessError> {
        let information_state_bytes =
            encode_canonical(&self.information_state).map_err(|_| HarnessError::WireEncoding)?;
        let visible_decision_bytes = self
            .visible_decision
            .as_ref()
            .map(|decision| encode_canonical(decision).map_err(|_| HarnessError::WireEncoding))
            .transpose()?;
        Ok(SurfaceBytes {
            information_state_bytes,
            visible_decision_bytes,
            information_digest: self.information_state.digest.clone(),
            next_visible_sequence: self.information_state.next_visible_sequence,
        })
    }
}

/// Captures one perspective's real typed outputs through endpoint handles.
pub fn capture_real_outputs(
    endpoints: &[mtgml_environment::PlayerEndpointHandle; 2],
    perspective: PlayerId,
) -> Result<RealOutputs, HarnessError> {
    use mtgml_environment::PlayerEndpoint;
    let handle = endpoints
        .iter()
        .find(|endpoint| endpoint.perspective() == perspective)
        .ok_or(HarnessError::BindFailed)?;
    let information_state = handle
        .information_state()
        .map_err(|_| HarnessError::EndpointService)?;
    let visible_decision = handle
        .visible_decision()
        .map_err(|_| HarnessError::EndpointService)?;
    Ok(RealOutputs {
        information_state,
        visible_decision,
    })
}

/// A leak mutant over the snapshot/information-state surface.
pub type LeakMutant = fn(RealOutputs, &EngineState) -> Result<RealOutputs, HarnessError>;

/// A leak mutant over one submitted transition product.
pub type StepLeakMutant =
    fn(PlayerStepV2, &EngineState, PlayerId) -> Result<PlayerStepV2, HarnessError>;

fn reseal(information_state: &mut PlayerInformationStateV2) -> Result<(), HarnessError> {
    let (_, digest) = compute_information_state_digest_v2(&information_state.digest_input())
        .map_err(|_| HarnessError::WireEncoding)?;
    information_state.digest = digest;
    Ok(())
}

fn fold_ids(values: impl Iterator<Item = u64>) -> u64 {
    values.fold(0xcbf2_9ce4_8422_2325, |accumulator, value| {
        (accumulator ^ value).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn perspective_identity(
    context: &EngineState,
    perspective: PlayerId,
) -> Result<&PerspectiveIdentityRecordV2, HarnessError> {
    context
        .perspective_identities
        .players
        .get(&perspective)
        .ok_or(HarnessError::TransformFixtureAbsent)
}

fn allocators(context: &EngineState) -> &IdentityAllocatorState {
    &context.allocators
}

fn concealed_members(context: &EngineState) -> Vec<GameObjectId> {
    let key = ZoneKey {
        zone: ZoneKind::Library,
        player: Some(P2),
        visibility: VisibilityPartition::FaceDown,
        partition: None,
    };
    context
        .zones
        .ordered_zones
        .get(&key)
        .cloned()
        .unwrap_or_default()
}

fn concealed_offset(context: &EngineState, object: GameObjectId) -> u32 {
    context
        .zones
        .locations
        .get(&object)
        .map(|location| match location.position {
            ZonePosition::Top { offset } => offset,
            _ => 0,
        })
        .unwrap_or(0)
}

fn concealed_definition(context: &EngineState, object: GameObjectId) -> u64 {
    context
        .zones
        .objects
        .get(&object)
        .map(|object| object.card_definition.0)
        .unwrap_or(0)
}

fn concealed_physical(context: &EngineState, object: GameObjectId) -> u64 {
    context
        .zones
        .objects
        .get(&object)
        .and_then(|object| object.physical_card)
        .map(|physical| physical.0)
        .unwrap_or(0)
}

fn observation_payload(observation: &ObservationEnvelope) -> Result<Vec<u8>, HarnessError> {
    STANDARD
        .decode(&observation.payload_base64)
        .map_err(|_| HarnessError::WireEncoding)
}

/// Replaces the observation payload content through the free-form payload
/// string region: canonical base64 re-encoding plus observation-digest
/// recomputation; callers reseal the information-state digest afterwards.
fn set_observation_payload(
    information_state: &mut PlayerInformationStateV2,
    content: &[u8],
) -> Result<(), HarnessError> {
    information_state.current_observation.payload_base64 = STANDARD.encode(content);
    information_state.current_observation.digest = ObservationDigest::from_canonical_bytes(content);
    Ok(())
}

fn public_provenance(reason: KnowledgeAcquisitionReason) -> PlayerKnowledgeProvenanceV1 {
    match reason {
        KnowledgeAcquisitionReason::InitialConfiguration => {
            PlayerKnowledgeProvenanceV1::InitialConfiguration
        }
        KnowledgeAcquisitionReason::Observed {
            channel,
            sequence,
            cause,
        } => PlayerKnowledgeProvenanceV1::Observed {
            channel: match channel {
                KnowledgeHistoryChannel::Public => PlayerKnowledgeChannelV1::Public,
                KnowledgeHistoryChannel::Private => PlayerKnowledgeChannelV1::Private,
            },
            sequence,
            cause: match cause {
                KnowledgeAcquisitionCause::PublicEvent => PlayerKnowledgeCauseV1::PublicEvent,
                KnowledgeAcquisitionCause::PrivateLook => PlayerKnowledgeCauseV1::PrivateLook,
                KnowledgeAcquisitionCause::ExplicitReveal => PlayerKnowledgeCauseV1::ExplicitReveal,
                KnowledgeAcquisitionCause::OwnPrivateIdentity => {
                    PlayerKnowledgeCauseV1::OwnPrivateIdentity
                }
            },
        },
    }
}

/// `(card definition, carries history)` of the non-witness perspective's
/// last active record; absent when the pair has no foreign knowledge.
fn foreign_summary(
    context: &EngineState,
    perspective: PlayerId,
) -> Result<(u64, bool), HarnessError> {
    let mut summary = None;
    for (player, knowledge) in &context.knowledge.players {
        if *player == perspective {
            continue;
        }
        if let Some((_, record)) = knowledge.active.iter().next_back() {
            summary = Some((
                record
                    .card_definition
                    .map(|definition| definition.0)
                    .unwrap_or(0),
                !record.historical_locations.is_empty(),
            ));
        }
    }
    summary.ok_or(HarnessError::TransformFixtureAbsent)
}

fn foreign_active_count(context: &EngineState, perspective: PlayerId) -> usize {
    context
        .knowledge
        .players
        .iter()
        .filter(|(player, _)| **player != perspective)
        .map(|(_, knowledge)| knowledge.active.len())
        .sum()
}

// === M1 =====================================================================

/// M1 — planned channel: re-sort `retained_knowledge` by trusted
/// `GameObjectId` order.
///
/// STRUCTURALLY IMPOSSIBLE under the validity gate: production emits
/// `retained_knowledge` in strictly ascending `OpaqueObjectId` order and
/// `PlayerInformationStateV2::validate` rejects any other order, while a
/// trusted-object-order sort either reproduces exactly that unique ascending
/// arrangement (no change, dead mutant) or violates the ordering rule
/// (invalid product). The detection test pins the rejection directly.
///
/// SUBSTITUTION (designated fallback family): stamp the folded hash of the
/// trusted opaque→object mapping into the visible-sequence cursor (`+1`, so
/// both sides always move). Pair 07a: the renamed mapping target separates
/// the sides. Pair 01 carries no mapping difference at all, so every
/// mapping-derived stamp is equally dead there; its coverage is the pinned
/// impossibility argument itself.
pub fn m1_resort_retained_knowledge_by_trusted_order(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    let perspective = outputs.information_state.perspective;
    let hash = fold_ids(
        perspective_identity(context, perspective)?
            .opaque_to_object
            .values()
            .map(|target| target.0),
    );
    let current = outputs.information_state.next_visible_sequence.0;
    outputs.information_state.next_visible_sequence = VisibleSequence(
        current
            .checked_add(1u64 + hash % 2)
            .ok_or(HarnessError::WireEncoding)?,
    );
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

// === M2 =====================================================================

/// M2 — planned channel: replace `CandidateIdV1` values with dense
/// hash(binding)-derived u32s.
///
/// STRUCTURALLY IMPOSSIBLE under the validity gate:
/// `CandidateOrderingV1::validate_public` pins visible candidate ids dense
/// from zero AND the array canonically ordered by public intent key, so the
/// id assignment is fully determined; any hash-derived reassignment fails
/// request validation (`CandidateIdsNotDense`). The detection test pins the
/// rejection directly.
///
/// SUBSTITUTION (designated fallback family): stamp a hash over the trusted
/// live-incarnation identity/definition pairs into the visible-sequence
/// cursor (`+2`, distinct from M1's shape). Pair 07a: the renamed incarnation
/// changes the fold; pair 01 carries no trusted difference to carry.
pub fn m2_candidate_ids_from_bindings(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    let hash = fold_ids(
        concealed_members(context)
            .into_iter()
            .flat_map(|object| [object.0, concealed_definition(context, object)]),
    );
    let current = outputs.information_state.next_visible_sequence.0;
    outputs.information_state.next_visible_sequence = VisibleSequence(
        current
            .checked_add(2u64 + hash % 2)
            .ok_or(HarnessError::WireEncoding)?,
    );
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

// === M3 =====================================================================

/// M3 — overwrite public identity surfaces with global-allocator-derived
/// values: every retained-knowledge `OpaqueObjectId` shifts monotonically by
/// `(global trigger cursor - 1)` (ascending order preserved) and the visible
/// decision's `PlayerDecisionIdV1` becomes the global trigger cursor value.
/// Pair 08: the unequally advanced trigger cursor separates the sides.
pub fn m3_identity_ids_from_global_allocators(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    let trigger_cursor = allocators(context).next_trigger_id.0;
    let shift = trigger_cursor
        .checked_sub(1)
        .ok_or(HarnessError::WireEncoding)?;
    for record in &mut outputs.information_state.retained_knowledge {
        match record {
            PlayerKnownObjectV1::Active {
                opaque_object_id, ..
            }
            | PlayerKnownObjectV1::Retired {
                opaque_object_id, ..
            } => {
                *opaque_object_id = OpaqueObjectId(
                    opaque_object_id
                        .0
                        .checked_add(shift)
                        .ok_or(HarnessError::WireEncoding)?,
                );
            }
        }
    }
    if let Some(decision) = outputs.visible_decision.as_mut() {
        decision.player_decision_id = PlayerDecisionIdV1(trigger_cursor.max(1));
    }
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

// === M4 =====================================================================

/// M4 — swap the semantic submission code to another VALID closed variant
/// actor-dependently: on the wrong-actor surface (the submitting actor does
/// not own the authoritative pending request) an `unavailable_decision`
/// outcome flips to `invalid_answer` exactly when the trusted context shows
/// the swapped face-down physical assignment (fixture predicate: the home
/// hidden incarnation no longer tracks its baseline physical card). Both
/// variants satisfy the identical rejection constraints, so validity holds
/// by construction.
///
/// Pairing note: the task's provisional axis-03 pairing cannot satisfy the
/// clean-output premise — the wrong-actor product embeds the submitting
/// player's OWN information state, which legitimately differs on axis 03
/// (that player holds the injected private look). Axis 04 keeps every
/// projected byte equal across sides while the trusted physical swap feeds
/// the flip, so it is used instead.
pub fn m4_submission_code_swap(
    mut step: PlayerStepV2,
    context: &EngineState,
    submitting_actor: PlayerId,
) -> Result<PlayerStepV2, HarnessError> {
    let wrong_actor = context
        .execution
        .pending_decision
        .as_ref()
        .is_some_and(|pending| pending.request.actor != submitting_actor);
    let members = concealed_members(context);
    let home = members
        .first()
        .copied()
        .ok_or(HarnessError::TransformFixtureAbsent)?;
    // Fixture predicate: the harness binds incarnation N to trusted physical
    // card N; the axis-04 swap breaks that baseline for the home object.
    let secret_present = concealed_physical(context, home) != home.0;
    if wrong_actor && secret_present {
        if let PlayerStepSubmissionV1::Rejected { code } = &mut step.submission {
            if *code == PlayerSubmissionCodeV1::UnavailableDecision {
                *code = PlayerSubmissionCodeV1::InvalidAnswer;
            }
        }
    }
    step.validate().map_err(|_| HarnessError::WireEncoding)?;
    Ok(step)
}

// === M5 =====================================================================

/// M5 — planned carrier: stamp the global event count into observed-event
/// sequence fields.
///
/// CARRIER ABSENT on the M2.C runtime path: the occurrence-only projector
/// hides the entry-stage random draw and the remaining authoritative events
/// are not perspective occurrences, so every real transition product carries
/// an EMPTY observed-event batch (the detection test pins this). There is no
/// sequence field to stamp without fabricating events.
///
/// SUBSTITUTION (designated fallback family): stamp the global rule-event
/// count monotonically INTO the information state's visible-sequence summary
/// cursor (`+1 + count`). Pair 08 supplies sides whose global counters differ.
pub fn m5_stamp_global_event_count(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    let event_count = allocators(context).next_rule_event_id.0;
    let current = outputs.information_state.next_visible_sequence.0;
    let bump = event_count
        .checked_add(1)
        .ok_or(HarnessError::WireEncoding)?;
    outputs.information_state.next_visible_sequence = VisibleSequence(
        current
            .checked_add(bump)
            .ok_or(HarnessError::WireEncoding)?,
    );
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

// === M6 =====================================================================

/// M6 — inject the hidden definitions/physicals summary into the allowed
/// free-form payload content region of the observation envelope (canonical
/// base64 string; envelope validation permits it — verified by the detection
/// test), then recompute the dependent digests. Pairs 01/04.
pub fn m6_payload_definition_injection(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    let members = concealed_members(context);
    let definitions: Vec<String> = members
        .iter()
        .map(|object| concealed_definition(context, *object).to_string())
        .collect();
    let physicals: Vec<String> = members
        .iter()
        .map(|object| concealed_physical(context, *object).to_string())
        .collect();
    let mut content = observation_payload(&outputs.information_state.current_observation)?;
    content.extend_from_slice(
        format!(
            "|fd=defs:{};pcs:{}",
            definitions.join(","),
            physicals.join(",")
        )
        .as_bytes(),
    );
    set_observation_payload(&mut outputs.information_state, &content)?;
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

// === M7 =====================================================================

/// M7 — planned channel: append a position hint into the location fact's
/// public zone/player fields.
///
/// STRUCTURALLY IMPOSSIBLE literally: those fields are a CLOSED enum
/// (`ZoneKind`) and a canonical integer (`Option<PlayerId>`), not free-form
/// strings; there is nothing to append to without leaving the type.
///
/// SUBSTITUTION: append the trusted position hint into the observation
/// payload string region (the permitted free-form content), encoding each
/// concealed incarnation as `object@offset`. Pair 02: the permutation
/// changes the hint text.
pub fn m7_position_hint_injection(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    let hint: Vec<String> = concealed_members(context)
        .iter()
        .map(|object| format!("{}@{}", object.0, concealed_offset(context, *object)))
        .collect();
    let mut content = observation_payload(&outputs.information_state.current_observation)?;
    content.extend_from_slice(format!("|pos={}", hint.join(",")).as_bytes());
    set_observation_payload(&mut outputs.information_state, &content)?;
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

// === M8 =====================================================================

/// M8 — insert a foreign knowledge record into `retained_knowledge` of the
/// real `PlayerInformationStateV2`: appended at the highest opaque id plus
/// one (strictly ascending order preserved), carrying the foreign side's
/// last known definition and an observed public acquisition exactly when the
/// foreign side holds history entries. The dependent public digest is
/// recomputed. Pairs 03/09.
pub fn m8_insert_foreign_knowledge_record(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    let perspective = outputs.information_state.perspective;
    let (definition, has_history) = foreign_summary(context, perspective)?;
    let max_opaque = outputs
        .information_state
        .retained_knowledge
        .iter()
        .map(|record| match record {
            PlayerKnownObjectV1::Active {
                opaque_object_id, ..
            }
            | PlayerKnownObjectV1::Retired {
                opaque_object_id, ..
            } => opaque_object_id.0,
        })
        .max()
        .unwrap_or(0);
    let inserted = OpaqueObjectId(
        max_opaque
            .checked_add(1)
            .ok_or(HarnessError::WireEncoding)?,
    );
    let acquisition = if has_history {
        KnowledgeAcquisitionReason::Observed {
            channel: KnowledgeHistoryChannel::Public,
            sequence: VisibleSequence(0),
            cause: KnowledgeAcquisitionCause::PublicEvent,
        }
    } else {
        KnowledgeAcquisitionReason::InitialConfiguration
    };
    outputs
        .information_state
        .retained_knowledge
        .push(PlayerKnownObjectV1::Active {
            opaque_object_id: inserted,
            known_definition: Some(CardDefinitionId(definition)),
            current_known_location_fact: None,
            historical_locations: Vec::new(),
            acquisition: public_provenance(acquisition),
        });
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

// === M9 =====================================================================

/// M9 — embed seed/cursor hex inside a VALID variable-length payload STRING
/// (observation payload content region), then recompute the dependent
/// digests — same validity-first rule as M6. Pairs 05/06.
pub fn m9_payload_secret_hex_injection(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    let seed_hex = context.random.root_seed.to_lower_hex();
    let cursor = context
        .random
        .lookup_stream(&RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1))
        .ok()
        .map(|cursor| cursor.next_raw_u64)
        .unwrap_or(0);
    let mut content = observation_payload(&outputs.information_state.current_observation)?;
    content
        .extend_from_slice(format!("|rng=seed:{};cursor:{}", &seed_hex[..16], cursor).as_bytes());
    set_observation_payload(&mut outputs.information_state, &content)?;
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

// === M10 ====================================================================

/// M10 — inflate an authorized summary count beyond its true value.
///
/// `PlayerInformationStateV2` carries no `public_history_length` (that was
/// the V1 envelope); the nearest authorized summary count is the
/// visible-sequence cursor, which may legally rise. The inflation amount is
/// the trusted concealed-definition moment `Σ (index+1)·definition`,
/// secret-dependent on pairs 01 (definitions) and 02 (orderings) alike.
pub fn m10_summary_count_inflation(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    let moment: u64 = concealed_members(context)
        .iter()
        .enumerate()
        .map(|(index, object)| {
            (index as u64 + 1).wrapping_mul(concealed_definition(context, *object))
        })
        .fold(0u64, |accumulator, term| accumulator.wrapping_add(term));
    let current = outputs.information_state.next_visible_sequence.0;
    let bump = moment.checked_add(1).ok_or(HarnessError::WireEncoding)?;
    outputs.information_state.next_visible_sequence = VisibleSequence(
        current
            .checked_add(bump)
            .ok_or(HarnessError::WireEncoding)?,
    );
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

// === M11 ====================================================================

/// M11 — toggle presence of a genuinely OPTIONAL field secret-dependently:
/// the known-definition marker of the first retained record disappears
/// exactly when the trusted context carries the foreign private look.
/// Optionality is schema-legal both ways. Pair 03.
pub fn m11_optional_presence_toggle(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    if foreign_active_count(context, outputs.information_state.perspective) > 2 {
        if let Some(PlayerKnownObjectV1::Active {
            known_definition, ..
        }) = outputs.information_state.retained_knowledge.first_mut()
        {
            *known_definition = None;
        }
    }
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

// === M12 ====================================================================

/// M12 — vary the LENGTH of a valid string payload value secret-dependently:
/// in-string padding only (inside the payload content, before canonical
/// base64 encoding; no trailing junk outside the string). The pad length is
/// one plus the number of inversions in the trusted concealed ordering.
/// Pair 02.
pub fn m12_payload_length_variation(
    mut outputs: RealOutputs,
    context: &EngineState,
) -> Result<RealOutputs, HarnessError> {
    let members = concealed_members(context);
    let inversions: usize = members
        .iter()
        .enumerate()
        .map(|(index, object)| {
            members[index + 1..]
                .iter()
                .filter(|other| other.0 < object.0)
                .count()
        })
        .sum();
    let padding = 1usize
        .checked_add(inversions % 8)
        .ok_or(HarnessError::WireEncoding)?;
    let mut content = observation_payload(&outputs.information_state.current_observation)?;
    content.reserve(padding);
    content.extend(std::iter::repeat(b'x').take(padding));
    set_observation_payload(&mut outputs.information_state, &content)?;
    reseal(&mut outputs.information_state)?;
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolation::fingerprint::capture_transition_product;
    use crate::isolation::paired::{
        spawn_environment, synthetic_environment_config, AxisKind, PairedCase,
    };
    use crate::isolation::paired_matrix::build_axis_case;
    use mtgml_decision::{
        DecisionAnswerV2, DecisionResponseV2, DecisionValidationError, DECISION_RESPONSE_V2_SCHEMA,
    };
    use mtgml_environment::{PlayerEndpoint, PlayerEndpointHandle};
    use mtgml_model::CandidateIdV1;
    use mtgml_wire::decode_canonical;

    const P1: PlayerId = PlayerId(1);

    fn spawned_pair(
        case: &PairedCase,
    ) -> Result<
        [(
            mtgml_environment::TrustedEnvironmentController,
            [PlayerEndpointHandle; 2],
        ); 2],
        HarnessError,
    > {
        let config = synthetic_environment_config([P1, P2]);
        Ok([
            spawn_environment(case.state_a.clone(), &config)?,
            spawn_environment(case.state_b.clone(), &config)?,
        ])
    }

    /// Full three-part detection proof: clean outputs byte-equal across the
    /// pair, the deterministically applied mutant separates the sides, and
    /// both mutated outputs still pass canonical decode/validation.
    fn detect(axis: AxisKind, mutant: LeakMutant) -> Result<(), HarnessError> {
        let case = build_axis_case(axis)?;
        let pair = spawned_pair(&case)?;
        let clean_a = capture_real_outputs(&pair[0].1, case.perspective)?;
        let clean_b = capture_real_outputs(&pair[1].1, case.perspective)?;

        // 1. real projection outputs are byte-equal across A/B.
        assert_eq!(clean_a.surfaces()?, clean_b.surfaces()?);

        // 2. after applying the mutant deterministically to BOTH sides they
        //    differ (the leak would be visible).
        let mutated_a = mutant(clean_a, &case.state_a)?;
        let mutated_b = mutant(clean_b, &case.state_b)?;
        let surface_a = mutated_a.surfaces()?;
        let surface_b = mutated_b.surfaces()?;
        assert_ne!(
            surface_a, surface_b,
            "axis {axis:?}: mutant did not separate the sides"
        );

        // 3. both mutated outputs remain valid canonical products.
        assert_output_valid(&mutated_a)?;
        assert_output_valid(&mutated_b)?;
        Ok(())
    }

    fn assert_output_valid(outputs: &RealOutputs) -> Result<(), HarnessError> {
        let surfaces = outputs.surfaces()?;
        decode_canonical::<PlayerInformationStateV2>(&surfaces.information_state_bytes)
            .map_err(|_| HarnessError::WireEncoding)?;
        if let Some(decision_bytes) = &surfaces.visible_decision_bytes {
            decode_canonical::<PlayerDecisionRequestV2>(decision_bytes)
                .map_err(|_| HarnessError::WireEncoding)?;
        }
        Ok(())
    }

    #[test]
    fn detects_m1_resort_retained_knowledge() -> Result<(), HarnessError> {
        // Pin the literal channel's structural impossibility: a reordered
        // retained-knowledge array is rejected by validation.
        {
            let case = build_axis_case(AxisKind::ObjectRenaming)?;
            let pair = spawned_pair(&case)?;
            let outputs = capture_real_outputs(&pair[0].1, case.perspective)?;
            let mut reordered = outputs.information_state.clone();
            reordered.retained_knowledge.reverse();
            assert!(matches!(
                reordered.validate(),
                Err(mtgml_observation::ObservationValidationError::RetainedKnowledge)
            ));
        }
        detect(
            AxisKind::ObjectRenaming,
            m1_resort_retained_knowledge_by_trusted_order,
        )
    }

    #[test]
    fn detects_m2_candidate_ids() -> Result<(), HarnessError> {
        // Pin the literal channel's structural impossibility: a
        // binding-hash-derived candidate id breaks the dense-from-zero rule.
        {
            let case = build_axis_case(AxisKind::ObjectRenaming)?;
            let pair = spawned_pair(&case)?;
            let outputs = capture_real_outputs(&pair[0].1, PlayerId(1))?;
            let mut request = outputs.visible_decision.expect("entry decision present");
            request.candidates[0].candidate_id = CandidateIdV1(77);
            assert_eq!(
                request.validate(),
                Err(DecisionValidationError::CandidateIdsNotDense)
            );
        }
        detect(AxisKind::ObjectRenaming, m2_candidate_ids_from_bindings)
    }

    #[test]
    fn detects_m3_allocator_ids() -> Result<(), HarnessError> {
        detect(
            AxisKind::GlobalAllocatorHistory,
            m3_identity_ids_from_global_allocators,
        )
    }

    #[test]
    fn detects_m4_submission_code() -> Result<(), HarnessError> {
        // Axis 04 keeps every projected byte equal across sides (the trusted
        // physical swap is invisible) while feeding the actor-dependent flip;
        // on the provisional axis-03 pairing the wrong-actor product embeds
        // the private-look holder's own legitimately different information
        // state, so the clean-output premise fails there.
        let case = build_axis_case(AxisKind::FaceDownIdentity)?;
        let pair = spawned_pair(&case)?;
        let response = DecisionResponseV2 {
            schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
            player_decision_id: mtgml_model::PlayerDecisionIdV1(1),
            state_revision: mtgml_model::StateRevision(0),
            answer: DecisionAnswerV2::SelectOne {
                candidate_id: CandidateIdV1(0),
            },
        };
        // Wrong-actor surface: P2 submits against P1's pending entry request.
        let endpoint_a = pair[0]
            .1
            .iter()
            .find(|endpoint| endpoint.perspective() == P2)
            .unwrap();
        let endpoint_b = pair[1]
            .1
            .iter()
            .find(|endpoint| endpoint.perspective() == P2)
            .unwrap();
        let step_a = endpoint_a.submit(response.clone()).unwrap();
        let step_b = endpoint_b.submit(response).unwrap();

        // 1. clean products are byte-equal across A/B.
        assert_eq!(
            encode_canonical(&step_a).unwrap(),
            encode_canonical(&step_b).unwrap()
        );
        // 2. the actor-dependent swap separates the sides.
        let mutated_a = m4_submission_code_swap(step_a, &case.state_a, P2)?;
        let mutated_b = m4_submission_code_swap(step_b, &case.state_b, P2)?;
        assert_ne!(
            encode_canonical(&mutated_a).unwrap(),
            encode_canonical(&mutated_b).unwrap()
        );
        // The flip landed exactly where intended.
        assert_eq!(
            mutated_a.submission,
            PlayerStepSubmissionV1::Rejected {
                code: PlayerSubmissionCodeV1::UnavailableDecision
            }
        );
        assert_eq!(
            mutated_b.submission,
            PlayerStepSubmissionV1::Rejected {
                code: PlayerSubmissionCodeV1::InvalidAnswer
            }
        );
        // 3. both remain fully valid canonical products.
        decode_canonical::<PlayerStepV2>(&encode_canonical(&mutated_a).unwrap()).unwrap();
        decode_canonical::<PlayerStepV2>(&encode_canonical(&mutated_b).unwrap()).unwrap();
        Ok(())
    }

    #[test]
    fn detects_m5_event_sequence_stamp() -> Result<(), HarnessError> {
        // Pin the absent carrier: real accepted transition products carry an
        // empty observed-event batch on this runtime path.
        {
            let case = build_axis_case(AxisKind::GlobalAllocatorHistory)?;
            let pair = spawned_pair(&case)?;
            for endpoints in [&pair[0].1, &pair[1].1] {
                let handle = endpoints
                    .iter()
                    .find(|endpoint| endpoint.perspective() == PlayerId(1))
                    .unwrap();
                let request = handle.visible_decision().unwrap().unwrap();
                let step = handle
                    .submit(DecisionResponseV2 {
                        schema_version: DECISION_RESPONSE_V2_SCHEMA.into(),
                        player_decision_id: request.player_decision_id,
                        state_revision: request.state_revision,
                        answer: DecisionAnswerV2::SelectOne {
                            candidate_id: request.candidates[0].candidate_id,
                        },
                    })
                    .unwrap();
                let product = capture_transition_product(Ok(step)).unwrap();
                assert!(product.observed_event_bytes.is_empty());
            }
        }
        detect(
            AxisKind::GlobalAllocatorHistory,
            m5_stamp_global_event_count,
        )
    }

    #[test]
    fn detects_m6_payload_definition_channel() -> Result<(), HarnessError> {
        // Envelope validation permits the payload-string region: the
        // validity gate below decodes the mutated products successfully.
        detect(
            AxisKind::OpponentHiddenDefinition,
            m6_payload_definition_injection,
        )?;
        detect(AxisKind::FaceDownIdentity, m6_payload_definition_injection)
    }

    #[test]
    fn detects_m7_position_hint_channel() -> Result<(), HarnessError> {
        detect(
            AxisKind::HiddenConcealedOrdering,
            m7_position_hint_injection,
        )
    }

    #[test]
    fn detects_m8_foreign_record_insertion() -> Result<(), HarnessError> {
        detect(
            AxisKind::ForeignPrivateLook,
            m8_insert_foreign_knowledge_record,
        )?;
        detect(
            AxisKind::ForeignKnowledgeHistory,
            m8_insert_foreign_knowledge_record,
        )
    }

    #[test]
    fn detects_m9_payload_secret_hex_channel() -> Result<(), HarnessError> {
        detect(AxisKind::RootSeedPreAuth, m9_payload_secret_hex_injection)?;
        detect(AxisKind::HiddenRngCursor, m9_payload_secret_hex_injection)
    }

    #[test]
    fn detects_m10_summary_count_inflation() -> Result<(), HarnessError> {
        detect(
            AxisKind::OpponentHiddenDefinition,
            m10_summary_count_inflation,
        )?;
        detect(
            AxisKind::HiddenConcealedOrdering,
            m10_summary_count_inflation,
        )
    }

    #[test]
    fn detects_m11_optional_presence_toggle() -> Result<(), HarnessError> {
        detect(AxisKind::ForeignPrivateLook, m11_optional_presence_toggle)
    }

    #[test]
    fn detects_m12_payload_length_variation() -> Result<(), HarnessError> {
        detect(
            AxisKind::HiddenConcealedOrdering,
            m12_payload_length_variation,
        )
    }

    #[test]
    fn dead_mutant_guard_m8_clean_inputs_do_not_diverge() -> Result<(), HarnessError> {
        let case = build_axis_case(AxisKind::ForeignPrivateLook)?;
        let pair = spawned_pair(&case)?;
        let outputs = capture_real_outputs(&pair[0].1, case.perspective)?;
        let first = m8_insert_foreign_knowledge_record(outputs.clone(), &case.state_a)?;
        let second = m8_insert_foreign_knowledge_record(outputs, &case.state_a)?;
        assert_eq!(first.surfaces()?, second.surfaces()?);
        Ok(())
    }
}
