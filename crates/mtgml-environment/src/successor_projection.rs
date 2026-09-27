//! Perspective projection for authoritative successor transition products.

use std::collections::{BTreeMap, BTreeSet};

use mtgml_model::{OpaqueObjectId, PlayerId, VisibleSequence};
use mtgml_observation::{
    ManaPoolAfterV1, ObservedEventEnvelopeV3, ObservedEventKindV3, ObservedFaceV1,
    PlayerStepSubmissionV1, PlayerStepV3, PlayerSubmissionCodeV1, OBSERVED_EVENT_SCHEMA_V3,
    PLAYER_STEP_SCHEMA_V3,
};
use mtgml_rules::{AuthoritativeRuleEventKindV2, SuccessorObservationPolicyV1, TransitionResult};
use mtgml_state::{EngineStatePartsV2, PerspectiveIdentityRecordV2};

use crate::errors::ControllerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SuccessorProjectionError {
    #[error("successor event references an unknown perspective")]
    UnknownPerspective,
    #[error("successor event sequence does not continue the perspective cursor")]
    CursorMismatch,
    #[error("authorized object has no perspective opaque identity")]
    MissingOpaqueIdentity,
    #[error("after-state visible sequence differs from projected events")]
    FinalCursorMismatch,
    #[error("battlefield-entry event face/tapped facts disagree with authoritative state")]
    BattlefieldEntryFactsMismatch,
}

pub struct SuccessorProjectionAuthority<'a> {
    pub execution_identity: &'a mtgml_model::ExecutionIdentityV1,
    pub semantic_manifest: &'a mtgml_model::SemanticContractManifestV1,
    pub rules_manifest: &'a mtgml_model::RulesContractManifestV1,
    pub catalog: &'a mtgml_card_ir::VerifiedContentCatalogV1,
}

fn resolve(
    authorized: bool,
    object: mtgml_model::GameObjectId,
    identity: &PerspectiveIdentityRecordV2,
) -> Result<Option<OpaqueObjectId>, SuccessorProjectionError> {
    if !authorized {
        return Ok(None);
    }
    identity
        .object_to_opaque
        .get(&object)
        .copied()
        .map(Some)
        .ok_or(SuccessorProjectionError::MissingOpaqueIdentity)
}

/// Projects the exact rules-owned audience records. This function does not
/// infer visibility from CardDefinition, allocation order, or object storage.
pub fn project_successor_events_v3(
    before: &EngineStatePartsV2,
    after: &EngineStatePartsV2,
    events: &[mtgml_rules::AuthoritativeRuleEventV2],
) -> Result<BTreeMap<PlayerId, Vec<ObservedEventEnvelopeV3>>, SuccessorProjectionError> {
    let before_engine = before.materialize();
    let after_engine = after.materialize();
    let mut projected: BTreeMap<_, Vec<_>> = before_engine
        .knowledge
        .players
        .keys()
        .copied()
        .map(|player| (player, Vec::new()))
        .collect();
    let mut cursors: BTreeMap<_, _> = before_engine
        .knowledge
        .players
        .iter()
        .map(|(player, state)| (*player, state.next_visible_sequence.0))
        .collect();
    let mut identities = before_engine.perspective_identities.players.clone();

    for event in events {
        let AuthoritativeRuleEventKindV2::PerspectiveOccurrence {
            lifecycle,
            observation,
        } = &event.event
        else {
            continue;
        };
        let cursor = cursors
            .get_mut(&lifecycle.perspective)
            .ok_or(SuccessorProjectionError::UnknownPerspective)?;
        if *cursor != lifecycle.sequence.0 {
            return Err(SuccessorProjectionError::CursorMismatch);
        }
        *cursor = cursor
            .checked_add(1)
            .ok_or(SuccessorProjectionError::CursorMismatch)?;
        let before_identity = identities
            .get(&lifecycle.perspective)
            .ok_or(SuccessorProjectionError::UnknownPerspective)?
            .clone();
        let mut after_identity = before_identity.clone();
        mtgml_state::advance_identity_record(&mut after_identity, &lifecycle.mutation.identity);
        let kind = match observation {
            SuccessorObservationPolicyV1::MovedInSight {
                from,
                to,
                old_object,
                new_object,
                reveals_old,
                entering_face,
                tapped,
            } => ObservedEventKindV3::ObjectMoved {
                old_object: resolve(*reveals_old, *old_object, &before_identity)?,
                new_object: resolve(true, *new_object, &after_identity)?,
                from: *from,
                to: *to,
                entering_face: if *to == mtgml_model::ZoneKind::Battlefield {
                    let object_state = after
                        .predecessor_v5
                        .zones
                        .objects
                        .get(new_object)
                        .ok_or(SuccessorProjectionError::BattlefieldEntryFactsMismatch)?;
                    let face = after
                        .card_rules_state
                        .faces
                        .faces
                        .get(new_object)
                        .copied()
                        .ok_or(SuccessorProjectionError::BattlefieldEntryFactsMismatch)?;
                    let (authoritative_face, observed_face) = match face {
                        0 => (mtgml_rules::BasicLandFaceV1::Front, ObservedFaceV1::Front),
                        1 => (mtgml_rules::BasicLandFaceV1::Back, ObservedFaceV1::Back),
                        _ => return Err(SuccessorProjectionError::BattlefieldEntryFactsMismatch),
                    };
                    if object_state.tapped != *tapped || authoritative_face != *entering_face {
                        return Err(SuccessorProjectionError::BattlefieldEntryFactsMismatch);
                    }
                    Some(observed_face)
                } else {
                    None
                },
                tapped: (*to == mtgml_model::ZoneKind::Battlefield).then_some(*tapped),
            },
            SuccessorObservationPolicyV1::ObjectTapped { object, tapped } => {
                ObservedEventKindV3::ObjectTapped {
                    object: resolve(true, *object, &before_identity)?
                        .ok_or(SuccessorProjectionError::MissingOpaqueIdentity)?,
                    tapped: *tapped,
                }
            }
            SuccessorObservationPolicyV1::ManaPoolChanged { player } => {
                let pool = after
                    .card_rules_state
                    .mana
                    .pools
                    .get(player)
                    .ok_or(SuccessorProjectionError::UnknownPerspective)?;
                ObservedEventKindV3::ManaPoolChanged {
                    player: *player,
                    pool_after: ManaPoolAfterV1 {
                        unrestricted: pool.unrestricted,
                        creature_spell_only: pool.creature_spell_only,
                    },
                    cause: if pool
                        .unrestricted
                        .iter()
                        .chain(pool.creature_spell_only.iter())
                        .all(|amount| *amount == 0)
                    {
                        mtgml_observation::ManaPoolChangeCauseV1::Emptied
                    } else {
                        mtgml_observation::ManaPoolChangeCauseV1::Produced
                    },
                }
            }
        };
        let envelope = ObservedEventEnvelopeV3 {
            schema_version: OBSERVED_EVENT_SCHEMA_V3.to_owned(),
            sequence: VisibleSequence(lifecycle.sequence.0),
            state_revision: event.state_revision,
            event: kind,
        };
        envelope
            .validate()
            .map_err(|_| SuccessorProjectionError::CursorMismatch)?;
        projected
            .get_mut(&lifecycle.perspective)
            .ok_or(SuccessorProjectionError::UnknownPerspective)?
            .push(envelope);
        identities.insert(lifecycle.perspective, after_identity);
    }

    for (player, next_sequence) in cursors {
        let actual = after_engine
            .knowledge
            .players
            .get(&player)
            .ok_or(SuccessorProjectionError::UnknownPerspective)?
            .next_visible_sequence
            .0;
        if actual != next_sequence {
            return Err(SuccessorProjectionError::FinalCursorMismatch);
        }
    }
    Ok(projected)
}

/// Composes the new public observation, perspective event stream, and supplied
/// authoritative V3 request into one validated PlayerStep for every player.
pub fn project_successor_player_steps(
    before: &EngineStatePartsV2,
    transition: &TransitionResult,
    authority: SuccessorProjectionAuthority<'_>,
    actor: PlayerId,
    rejected_code: PlayerSubmissionCodeV1,
) -> Result<BTreeMap<PlayerId, PlayerStepV3>, ControllerError> {
    let events = project_successor_events_v3(before, &transition.next_state, &transition.events)
        .map_err(|error| ControllerError::Backend(error.to_string()))?;
    let players: BTreeSet<_> = transition
        .next_state
        .predecessor_v5
        .core
        .players
        .keys()
        .copied()
        .collect();
    let mut steps = BTreeMap::new();
    for perspective in players {
        let information_state = crate::project_successor_information_state(
            &transition.next_state,
            perspective,
            authority.execution_identity,
            authority.semantic_manifest,
            authority.rules_manifest,
            authority.catalog,
        )
        .map_err(|error| ControllerError::Backend(error.to_string()))?;
        let next_decision = transition
            .next_decision
            .as_ref()
            .filter(|request| request.actor == perspective)
            .map(|request| request.project_player_request())
            .transpose()
            .map_err(|error| ControllerError::Backend(error.to_string()))?;
        let submission = if transition.accepted || perspective != actor {
            PlayerStepSubmissionV1::Accepted
        } else {
            PlayerStepSubmissionV1::Rejected {
                code: rejected_code,
            }
        };
        let step = PlayerStepV3 {
            schema_version: PLAYER_STEP_SCHEMA_V3.to_owned(),
            information_state,
            observed_events: events
                .get(&perspective)
                .cloned()
                .ok_or_else(|| ControllerError::Backend("missing perspective events".into()))?,
            next_decision,
            status: transition.status.clone(),
            submission,
        };
        step.validate()
            .map_err(|_| ControllerError::Backend("invalid successor PlayerStep".into()))?;
        steps.insert(perspective, step);
    }
    Ok(steps)
}
