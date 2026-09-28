//! RulesKernel-owned S1-A boundary for exact live-object lookups.
//!
//! This module does not derive characteristics. It binds one object lookup to
//! the opaque executable-profile admission and structurally validated V8 state
//! supplied by the current RulesKernel path.

use mtgml_card_ir::{
    BasicLandProfileV1, CardSemanticBindingV1, ExecutableProfileAdmissionV1, FaceDefinitionV1,
    FaceKey, VerifiedContentCatalogV1, BASIC_LAND_PROFILE_ID_V1,
};
use mtgml_model::{CardDefinitionId, ExecutionIdentityV1, GameObjectId, PlayerId, ZoneKind};
use mtgml_state::{EngineStatePartsV3, EngineStatePartsV3Error, GameObject, ZoneLocation};
use thiserror::Error;

/// Trusted object facts joined by S1-A. This remains internal to the rules
/// crate and is not a player-facing or serialized value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QueriedObjectV1 {
    pub(crate) object: GameObjectId,
    pub(crate) card_definition: CardDefinitionId,
    pub(crate) owner: PlayerId,
    pub(crate) controller: PlayerId,
    pub(crate) zone: ZoneKind,
    pub(crate) face_key: FaceKey,
}

/// Borrowed, object-scoped authority for one admitted RulesKernel query.
///
/// Construction accepts the opaque preflight token, not a caller-supplied
/// catalog, content ID, execution identity, or "verified" flag. The exact
/// query object is stored in the authority so it cannot be substituted after
/// construction.
#[derive(Debug)]
pub(crate) struct S1QueryAuthority<'a> {
    admission: &'a ExecutableProfileAdmissionV1,
    _state: &'a EngineStatePartsV3,
    queried: QueriedObjectV1,
    _object: &'a GameObject,
    _location: &'a ZoneLocation,
    _definition: &'a mtgml_card_ir::CardDefinitionEnvelopeV1,
    _face: &'a FaceDefinitionV1,
}

impl<'a> S1QueryAuthority<'a> {
    pub(crate) fn for_object(
        admission: &'a ExecutableProfileAdmissionV1,
        state: &'a EngineStatePartsV3,
        object_id: GameObjectId,
    ) -> Result<Self, S1QueryError> {
        validate_admission_binding(admission)?;

        if let Err(error) = state.validate_structure() {
            let object_is_live = state.predecessor_v5.zones.objects.contains_key(&object_id);
            if object_is_live
                && !state
                    .predecessor_v5
                    .zones
                    .locations
                    .contains_key(&object_id)
            {
                return Err(S1QueryError::MissingZoneLocation(object_id));
            }
            if object_is_live && !state.card_rules_state.faces.faces.contains_key(&object_id) {
                return Err(S1QueryError::FaceStateMissing(object_id));
            }
            return Err(S1QueryError::InconsistentState(error));
        }

        let object = state
            .predecessor_v5
            .zones
            .objects
            .get(&object_id)
            .ok_or_else(|| classify_absent_object(state, object_id))?;
        if object.id != object_id {
            return Err(S1QueryError::InconsistentState(
                EngineStatePartsV3Error::PredecessorState,
            ));
        }
        let location = state
            .predecessor_v5
            .zones
            .locations
            .get(&object_id)
            .ok_or(S1QueryError::MissingZoneLocation(object_id))?;
        if object.face_down {
            return Err(S1QueryError::FaceDownCharacteristicsUnsupported(object_id));
        }

        let catalog = admission.verified_catalog();
        let definition = catalog
            .get(admission.content_contract_id(), object.card_definition)
            .map_err(|error| match error {
                mtgml_card_ir::DefinitionLookupErrorV1::ContentContractMismatch => {
                    S1QueryError::ContentContractMismatch
                }
                mtgml_card_ir::DefinitionLookupErrorV1::MissingDefinition => {
                    S1QueryError::MissingCardDefinition(object.card_definition)
                }
            })?;

        let current_face = state
            .card_rules_state
            .faces
            .faces
            .get(&object_id)
            .copied()
            .ok_or(S1QueryError::FaceStateMissing(object_id))?;
        let face = definition
            .faces
            .iter()
            .find(|face| face.face_key.0 == current_face)
            .ok_or(S1QueryError::UnknownFace {
                definition: object.card_definition,
                face_key: FaceKey(current_face),
            })?;

        if !is_admitted_basic_land_profile(definition) {
            return Err(S1QueryError::ProfileNotAdmitted);
        }

        Ok(Self {
            admission,
            _state: state,
            queried: QueriedObjectV1 {
                object: object_id,
                card_definition: object.card_definition,
                owner: object.owner,
                controller: object.controller,
                zone: location.zone,
                face_key: FaceKey(current_face),
            },
            _object: object,
            _location: location,
            _definition: definition,
            _face: face,
        })
    }

    pub(crate) fn queried_object(&self) -> QueriedObjectV1 {
        self.queried
    }

    pub(crate) fn execution_identity(&self) -> &ExecutionIdentityV1 {
        self.admission.execution_identity()
    }
}

fn validate_admission_binding(
    admission: &ExecutableProfileAdmissionV1,
) -> Result<&VerifiedContentCatalogV1, S1QueryError> {
    let content_id = admission.content_contract_id();
    let catalog = admission.verified_catalog();
    if content_id != catalog.content_contract_id() {
        return Err(S1QueryError::ContentContractMismatch);
    }
    Ok(catalog)
}

fn is_admitted_basic_land_profile(definition: &mtgml_card_ir::CardDefinitionEnvelopeV1) -> bool {
    matches!(
        &definition.semantic_binding,
        CardSemanticBindingV1::ProfiledV1 {
            profile_id,
            body: BasicLandProfileV1 { .. },
        } if profile_id.as_str() == BASIC_LAND_PROFILE_ID_V1
    )
}

fn classify_absent_object(state: &EngineStatePartsV3, object_id: GameObjectId) -> S1QueryError {
    // Object IDs start at one and the sole allocator advances by exactly one
    // for each allocation. In a structurally valid admitted state, an absent
    // ID below this high-water mark therefore names a departed incarnation.
    let next_id = state.predecessor_v5.allocators.next_object_id;
    if object_id.0 != 0 && object_id.0 < next_id.0 {
        S1QueryError::StaleObjectIncarnation(object_id)
    } else {
        S1QueryError::UnknownObject(object_id)
    }
}

// Keep later-batch variants in this cohesive error contract without adding
// placeholder successful query paths in S1-A.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum S1QueryError {
    #[error("game object {0:?} is not known to this state")]
    UnknownObject(GameObjectId),
    #[error("game object incarnation {0:?} is no longer live")]
    StaleObjectIncarnation(GameObjectId),
    #[error("live game object {0:?} has no zone location")]
    MissingZoneLocation(GameObjectId),
    #[error("card definition {0:?} is not in the admitted content catalog")]
    MissingCardDefinition(CardDefinitionId),
    #[error("admitted content identity does not match its verified catalog")]
    ContentContractMismatch,
    #[error("the requested semantic profile is not admitted by this Rules context")]
    ProfileNotAdmitted,
    #[error("definition {definition:?} has no face {face_key:?}")]
    UnknownFace {
        definition: CardDefinitionId,
        face_key: FaceKey,
    },
    #[error("live game object {0:?} has no current face entry")]
    FaceStateMissing(GameObjectId),
    #[error("face-down characteristics for object {0:?} are unsupported")]
    FaceDownCharacteristicsUnsupported(GameObjectId),
    #[error("characteristic query {0:?} is not implemented by this authority")]
    UnsupportedCharacteristic(CharacteristicKind),
    #[error("characteristic contributor {0:?} is not admitted by this authority")]
    UnsupportedContributor(ContributorKind),
    #[error("attachment reference for object {0:?} is invalid")]
    InvalidAttachmentReference(GameObjectId),
    #[error("counter state for object {0:?} is invalid")]
    InvalidCounterState(GameObjectId),
    #[error("authoritative state is inconsistent: {0}")]
    InconsistentState(EngineStatePartsV3Error),
    #[error("characteristic arithmetic overflowed")]
    ArithmeticOverflow,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharacteristicKind {
    PowerToughness,
    TypeLine,
    Color,
    Keyword,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContributorKind {
    StaticSource,
    TemporaryEffect,
    ProfileAbility,
}
