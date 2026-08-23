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
use mtgml_state::{EngineState, IdentityMutationV1, PerspectiveIdentityRecordV2};

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
}

type ProjectionResult<T> = Result<T, LifecycleProjectionError>;

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
        match &lifecycle.mutation.identity {
            IdentityMutationV1::None => {}
            IdentityMutationV1::Allocate { opaque, object } => {
                record_after.opaque_to_object.insert(*opaque, *object);
                record_after.object_to_opaque.insert(*object, *opaque);
            }
            IdentityMutationV1::Remap {
                opaque,
                from_object,
                to_object,
            } => {
                record_after.opaque_to_object.insert(*opaque, *to_object);
                record_after.object_to_opaque.remove(from_object);
                record_after.object_to_opaque.insert(*to_object, *opaque);
            }
            IdentityMutationV1::Retire { opaque, object } => {
                record_after.opaque_to_object.remove(opaque);
                record_after.object_to_opaque.remove(object);
                record_after.retired_object_ids.insert(*opaque);
            }
        }

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
