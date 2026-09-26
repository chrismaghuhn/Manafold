//! M2.E occurrence projection: trusted authoritative events in, per-
//! perspective redacted observed-event envelopes out.
//!
//! Ownership (ADR 0016/0040): rules already decided authorization inside the
//! occurrence records; this layer performs the opaque substitution from the
//! BEFORE/AFTER perspective identity snapshots and validates the complete
//! per-perspective visible-sequence product before any commit. The function
//! is strictly read-only.

use std::collections::BTreeMap;

use mtgml_model::{OpaqueObjectId, PlayerId, VisibleSequence};
use mtgml_rules::AuthoritativeRuleEventKind;
use mtgml_state::{EngineState, PerspectiveIdentityRecordV2};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LifecycleProjectionError {
    #[error("occurrence references an unknown perspective")]
    UnknownPerspective,
    #[error("occurrence sequence does not continue the perspective cursor")]
    CursorMismatch,
    #[error("final cursor does not equal the after-state cursor")]
    FinalCursorMismatch,
    #[error("authorized field references an object outside the snapshot mapping")]
    AuthorizedObjectUnresolvable,
    #[error("projected observed event envelope is invalid")]
    InvalidObservedEvent,
}

type ProjectionResult<T> = Result<T, LifecycleProjectionError>;

/// Entry facts already authorized for one perspective and visible occurrence.
/// This type carries projection input only; it does not derive Magic rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorizedBattlefieldEntryFactsV3 {
    pub entering_face: Option<mtgml_observation::ObservedFaceV1>,
    pub tapped: Option<bool>,
}

fn resolve(
    authorized: bool,
    object: mtgml_model::GameObjectId,
    record: &PerspectiveIdentityRecordV2,
) -> ProjectionResult<Option<OpaqueObjectId>> {
    if !authorized {
        // Authorization is rules-owned; absence of the flag means the field
        // stays absent even when a live mapping exists.
        return Ok(None);
    }
    match record.object_to_opaque.get(&object) {
        Some(opaque) => Ok(Some(*opaque)),
        // An authorized but unresolvable object is an internal invariant
        // failure: it must abort the candidate transition, never silently
        // render as absent.
        None => Err(LifecycleProjectionError::AuthorizedObjectUnresolvable),
    }
}

/// Projects every `PerspectiveOccurrence` into at most one observed envelope
/// for its perspective and proves that the per-perspective cursors advance
/// exactly once per occurrence and end at the after-state cursors.
pub fn project_occurrence_envelopes(
    before: &EngineState,
    after: &EngineState,
    events: &[mtgml_rules::AuthoritativeRuleEvent],
) -> ProjectionResult<BTreeMap<PlayerId, Vec<mtgml_observation::ObservedEventEnvelopeV2>>> {
    let mut envelopes: BTreeMap<PlayerId, Vec<mtgml_observation::ObservedEventEnvelopeV2>> = before
        .knowledge
        .players
        .keys()
        .copied()
        .map(|player| (player, Vec::new()))
        .collect();
    let mut cursors: BTreeMap<PlayerId, u64> = before
        .knowledge
        .players
        .iter()
        .map(|(player, knowledge)| (*player, knowledge.next_visible_sequence.0))
        .collect();

    // Sequential per-perspective identity snapshots: occurrence N resolves
    // its authorized references against the mapping state produced by
    // occurrences 1..=N (old fields pre-mutation, new fields post-mutation).
    let mut running: BTreeMap<PlayerId, PerspectiveIdentityRecordV2> =
        before.perspective_identities.players.clone();

    for event in events {
        let AuthoritativeRuleEventKind::PerspectiveOccurrence {
            lifecycle,
            observation,
        } = &event.event
        else {
            continue;
        };
        let perspective = lifecycle.perspective;
        let next_expected = cursors
            .get_mut(&perspective)
            .ok_or(LifecycleProjectionError::UnknownPerspective)?;
        if *next_expected != lifecycle.sequence.0 {
            return Err(LifecycleProjectionError::CursorMismatch);
        }
        *next_expected += 1;

        let record_before = running
            .get(&perspective)
            .ok_or(LifecycleProjectionError::UnknownPerspective)?
            .clone();
        let mut record_after = record_before.clone();
        mtgml_state::advance_identity_record(&mut record_after, &lifecycle.mutation.identity);
        use mtgml_rules::PerspectiveObservationPolicyV1 as Policy;
        let kind = match observation {
            Policy::MovedInSight {
                from_zone,
                to_zone,
                old_object,
                new_object,
                reveals_old,
                reveals_new,
            } => Some((
                mtgml_observation::ObservedEventKindV2::ObjectMoved {
                    old_object: resolve(*reveals_old, *old_object, &record_before)?,
                    new_object: resolve(*reveals_new, *new_object, &record_after)?,
                    from: *from_zone,
                    to: *to_zone,
                },
                lifecycle.sequence,
                event.state_revision,
            )),
            Policy::Appeared {
                from_zone,
                to_zone,
                new_object,
            } => Some((
                mtgml_observation::ObservedEventKindV2::ObjectMoved {
                    old_object: None,
                    new_object: resolve(true, *new_object, &record_after)?,
                    from: *from_zone,
                    to: *to_zone,
                },
                lifecycle.sequence,
                event.state_revision,
            )),
            Policy::NoEnvelope => None,
            Policy::ObjectTapped { object, tapped } => Some((
                mtgml_observation::ObservedEventKindV2::ObjectTapped {
                    object: resolve(true, *object, &record_before)?
                        .ok_or(LifecycleProjectionError::AuthorizedObjectUnresolvable)?,
                    tapped: *tapped,
                },
                lifecycle.sequence,
                event.state_revision,
            )),
            Policy::SawRandomOutcome {
                label,
                exclusive_upper_bound,
                value,
            } => Some((
                mtgml_observation::ObservedEventKindV2::RandomOutcomeVisible {
                    label: label.clone(),
                    exclusive_upper_bound: *exclusive_upper_bound,
                    value: *value,
                },
                lifecycle.sequence,
                event.state_revision,
            )),
            Policy::AnnouncedOutcome { code } => Some((
                mtgml_observation::ObservedEventKindV2::PublicOutcome { code: code.clone() },
                lifecycle.sequence,
                event.state_revision,
            )),
        };
        running.insert(perspective, record_after);
        if let Some((event_kind, sequence, state_revision)) = kind {
            let envelope = mtgml_observation::ObservedEventEnvelopeV2 {
                schema_version: mtgml_observation::OBSERVED_EVENT_SCHEMA_V2.into(),
                sequence: VisibleSequence(sequence.0),
                state_revision,
                event: event_kind,
            };
            envelope
                .validate()
                .map_err(|_| LifecycleProjectionError::InvalidObservedEvent)?;
            envelopes.entry(perspective).or_default().push(envelope);
        }
    }

    for (player, cursor) in &cursors {
        let knowledge = after
            .knowledge
            .players
            .get(player)
            .ok_or(LifecycleProjectionError::UnknownPerspective)?;
        if knowledge.next_visible_sequence.0 != *cursor {
            return Err(LifecycleProjectionError::FinalCursorMismatch);
        }
    }
    Ok(envelopes)
}

/// Projects the same rules-owned, perspective-authorized occurrences as V2,
/// encoding them with the detached V3 event shape. New M4 state event
/// producers are intentionally added by their owning RulesKernel work; this
/// adapter cannot manufacture such events.
pub fn project_occurrence_envelopes_v3(
    before: &EngineState,
    after: &EngineState,
    events: &[mtgml_rules::AuthoritativeRuleEvent],
) -> ProjectionResult<BTreeMap<PlayerId, Vec<mtgml_observation::ObservedEventEnvelopeV3>>> {
    project_occurrence_envelopes(before, after, events)?
        .into_iter()
        .map(|(player, envelopes)| {
            let projected = envelopes
                .into_iter()
                .map(mtgml_observation::ObservedEventEnvelopeV3::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| LifecycleProjectionError::InvalidObservedEvent)?;
            Ok((player, projected))
        })
        .collect()
}

/// Adds already-authorized face/tapped facts to visible battlefield-entry
/// occurrences after the V2 audience and opaque-identity projection. Facts
/// must identify an existing visible V3 `object_moved` entry by perspective
/// and sequence; this function neither creates events nor infers entry rules.
pub fn project_occurrence_envelopes_v3_with_entry_facts(
    before: &EngineState,
    after: &EngineState,
    events: &[mtgml_rules::AuthoritativeRuleEvent],
    entry_facts: &BTreeMap<(PlayerId, VisibleSequence), AuthorizedBattlefieldEntryFactsV3>,
) -> ProjectionResult<BTreeMap<PlayerId, Vec<mtgml_observation::ObservedEventEnvelopeV3>>> {
    use mtgml_observation::ObservedEventKindV3;

    let mut projected = project_occurrence_envelopes_v3(before, after, events)?;
    for ((perspective, sequence), facts) in entry_facts {
        if facts.entering_face.is_none() && facts.tapped.is_none() {
            return Err(LifecycleProjectionError::InvalidObservedEvent);
        }
        let Some(envelope) = projected.get_mut(perspective).and_then(|envelopes| {
            envelopes
                .iter_mut()
                .find(|event| event.sequence == *sequence)
        }) else {
            return Err(LifecycleProjectionError::InvalidObservedEvent);
        };
        let ObservedEventKindV3::ObjectMoved {
            new_object: Some(_),
            to: mtgml_model::ZoneKind::Battlefield,
            entering_face,
            tapped,
            ..
        } = &mut envelope.event
        else {
            return Err(LifecycleProjectionError::InvalidObservedEvent);
        };
        if entering_face.is_some() || tapped.is_some() {
            return Err(LifecycleProjectionError::InvalidObservedEvent);
        }
        *entering_face = facts.entering_face;
        *tapped = facts.tapped;
        envelope
            .validate()
            .map_err(|_| LifecycleProjectionError::InvalidObservedEvent)?;
    }
    Ok(projected)
}
