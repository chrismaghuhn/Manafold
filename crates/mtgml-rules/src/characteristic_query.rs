//! RulesKernel-owned S1-A boundary for exact live-object lookups.
//!
//! This module does not derive characteristics. It binds one object lookup to
//! the opaque executable-profile admission and structurally validated V8 state
//! supplied by the current RulesKernel path.

use mtgml_card_ir::{
    BaseCharacteristicsV1, CardProfileBodyV1, CardSemanticBindingV1, ExecutableProfileAdmissionV1,
    FaceDefinitionV1, FaceKey, ManaColorV1, PrintedManaSymbolV1, VerifiedContentCatalogV1,
    BASIC_LAND_PROFILE_ID_V1, VANILLA_CREATURE_PROFILE_ID_V1,
};
use mtgml_model::{CardDefinitionId, GameObjectId, PlayerId, ZoneKind};
use mtgml_state::{EngineState, EngineStateError, GameObject, ZoneLocation};
use std::collections::BTreeSet;
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

/// Bounded base-stage result. It deliberately contains no counter-adjusted or
/// source/effect-derived values, which belong to later S1 batches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct S1BaseCharacteristicsV1 {
    pub(crate) queried: QueriedObjectV1,
    pub(crate) supertypes: Vec<String>,
    pub(crate) card_types: Vec<String>,
    pub(crate) subtypes: Vec<String>,
    pub(crate) colors: BTreeSet<ManaColorV1>,
    pub(crate) base_power_toughness: Option<(i64, i64)>,
}

/// Borrowed, object-scoped authority for one admitted RulesKernel query.
///
/// Construction accepts the opaque preflight token, not a caller-supplied
/// catalog, content ID, execution identity, or "verified" flag. The exact
/// query object is stored in the authority so it cannot be substituted after
/// construction.
#[derive(Debug)]
pub(crate) struct S1QueryAuthority<'a> {
    _state: &'a EngineState,
    queried: QueriedObjectV1,
    _object: &'a GameObject,
    _location: &'a ZoneLocation,
    _definition: &'a mtgml_card_ir::CardDefinitionEnvelopeV1,
    _face: &'a FaceDefinitionV1,
}

impl<'a> S1QueryAuthority<'a> {
    // Production callers query batches (`for_objects`); the single-object
    // form serves the unit witnesses and rules that ask about one object.
    #[allow(dead_code)]
    pub(crate) fn for_object(
        admission: &'a ExecutableProfileAdmissionV1,
        state: &'a EngineState,
        object_id: GameObjectId,
    ) -> Result<Self, S1QueryError> {
        validate_admission_binding(admission)?;
        validate_query_state(state, object_id)?;
        Self::in_validated_state(admission, state, object_id)
    }

    /// Queries several objects of one state, validating the admission and
    /// the state once. The result equals calling `for_object` on each object
    /// in order and stopping at the first error.
    pub(crate) fn for_objects(
        admission: &'a ExecutableProfileAdmissionV1,
        state: &'a EngineState,
        objects: &[GameObjectId],
    ) -> Result<Vec<Self>, S1QueryError> {
        let Some(first) = objects.first() else {
            return Ok(Vec::new());
        };
        validate_admission_binding(admission)?;
        validate_query_state(state, *first)?;
        objects
            .iter()
            .map(|object| Self::in_validated_state(admission, state, *object))
            .collect()
    }

    fn in_validated_state(
        admission: &'a ExecutableProfileAdmissionV1,
        state: &'a EngineState,
        object_id: GameObjectId,
    ) -> Result<Self, S1QueryError> {
        let object = state
            .zones
            .objects
            .get(&object_id)
            .ok_or_else(|| classify_absent_object(state, object_id))?;
        if object.id != object_id {
            return Err(S1QueryError::InconsistentState(
                EngineStateError::StateInvariant,
            ));
        }
        let location = state
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
            .card_rules
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

        if !is_admitted_profile(definition) {
            return Err(S1QueryError::ProfileNotAdmitted);
        }

        Ok(Self {
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

    /// The queried object's base characteristics, derived from its face
    /// (combat reads a creature's types and power from them).
    pub(crate) fn derive_base_characteristics(&self) -> S1BaseCharacteristicsV1 {
        derive_base_characteristics(self.queried, &self._face.base_characteristics)
    }
}

// Pure and profile-independent: callers provide one face already obtained
// from their verified content authority. Executable-profile admission stays
// outside this mapper, so later admitted profiles reuse the same semantics.
fn derive_base_characteristics(
    queried: QueriedObjectV1,
    face: &BaseCharacteristicsV1,
) -> S1BaseCharacteristicsV1 {
    let mut colors = face
        .color_indicator
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if let Some(mana_cost) = &face.mana_cost {
        for symbol in mana_cost {
            match symbol {
                PrintedManaSymbolV1::White => {
                    colors.insert(ManaColorV1::White);
                }
                PrintedManaSymbolV1::Blue => {
                    colors.insert(ManaColorV1::Blue);
                }
                PrintedManaSymbolV1::Black => {
                    colors.insert(ManaColorV1::Black);
                }
                PrintedManaSymbolV1::Red => {
                    colors.insert(ManaColorV1::Red);
                }
                PrintedManaSymbolV1::Green => {
                    colors.insert(ManaColorV1::Green);
                }
                PrintedManaSymbolV1::Hybrid(first, second) => {
                    colors.insert(*first);
                    colors.insert(*second);
                }
                PrintedManaSymbolV1::Generic(_) | PrintedManaSymbolV1::Colorless => {}
            }
        }
    }

    S1BaseCharacteristicsV1 {
        queried,
        supertypes: face.type_line.supertypes.clone(),
        card_types: face.type_line.card_types.clone(),
        subtypes: face.type_line.subtypes.clone(),
        colors,
        base_power_toughness: face
            .power_toughness
            .map(|(power, toughness)| (i64::from(power), i64::from(toughness))),
    }
}

/// Validates the state a query reads. An invalid state is reported through
/// `object_id` where the object's own records explain it.
fn validate_query_state(state: &EngineState, object_id: GameObjectId) -> Result<(), S1QueryError> {
    let Err(error) = state.validate_structure() else {
        return Ok(());
    };
    let object_is_live = state.zones.objects.contains_key(&object_id);
    if object_is_live && !state.zones.locations.contains_key(&object_id) {
        return Err(S1QueryError::MissingZoneLocation(object_id));
    }
    if object_is_live && !state.card_rules.faces.faces.contains_key(&object_id) {
        return Err(S1QueryError::FaceStateMissing(object_id));
    }
    Err(S1QueryError::InconsistentState(error))
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

fn is_admitted_profile(definition: &mtgml_card_ir::CardDefinitionEnvelopeV1) -> bool {
    matches!(
        &definition.semantic_binding,
        CardSemanticBindingV1::ProfiledV1 {
            profile_id,
            body: CardProfileBodyV1::BasicLand(_),
        } if profile_id.as_str() == BASIC_LAND_PROFILE_ID_V1
    ) || matches!(
        &definition.semantic_binding,
        CardSemanticBindingV1::ProfiledV1 {
            profile_id,
            body: CardProfileBodyV1::VanillaCreature,
        } if profile_id.as_str() == VANILLA_CREATURE_PROFILE_ID_V1
    )
}

fn classify_absent_object(state: &EngineState, object_id: GameObjectId) -> S1QueryError {
    // Object IDs start at one and the sole allocator advances by exactly one
    // for each allocation. In a structurally valid admitted state, an absent
    // ID below this high-water mark therefore names a departed incarnation.
    let next_id = state.allocators.next_object_id;
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
    InconsistentState(EngineStateError),
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

#[cfg(test)]
mod s1_b_detached_tests {
    use super::{
        derive_base_characteristics, QueriedObjectV1, S1BaseCharacteristicsV1, S1QueryError,
    };
    use mtgml_card_ir::{
        encode_content_manifest_v1, encode_provenance_catalog_v1, BaseCharacteristicsV1,
        CardDefinitionEnvelopeV1, CardSemanticBindingV1, ContentContractManifestV1,
        DefinitionProvenanceRecordV1, FaceDefinitionV1, FaceKey, ManaColorV1, PrintedManaSymbolV1,
        ProvenanceCatalogV1, SourceProvenanceV1, TypeLineV1, VerifiedContentCatalogV1,
    };
    use mtgml_model::{CardDefinitionId, GameObjectId};
    use mtgml_persistence::content_contract_digest::calculate_content_contract_id_v1;
    use mtgml_state::EngineState;
    use std::collections::BTreeSet;

    struct DetachedFixture {
        id: CardDefinitionId,
        faces: Vec<FaceDefinitionV1>,
    }

    fn face(
        key: u32,
        name: &str,
        mana_cost: Option<Vec<PrintedManaSymbolV1>>,
        color_indicator: Vec<ManaColorV1>,
        type_line: TypeLineV1,
        power_toughness: Option<(i32, i32)>,
    ) -> FaceDefinitionV1 {
        FaceDefinitionV1 {
            face_key: FaceKey(key),
            base_characteristics: BaseCharacteristicsV1 {
                name: name.to_owned(),
                mana_cost,
                color_indicator,
                type_line,
                power_toughness,
                loyalty: None,
                defense: None,
            },
        }
    }

    fn type_line(supertypes: &[&str], card_types: &[&str], subtypes: &[&str]) -> TypeLineV1 {
        TypeLineV1 {
            supertypes: supertypes.iter().map(|value| (*value).to_owned()).collect(),
            card_types: card_types.iter().map(|value| (*value).to_owned()).collect(),
            subtypes: subtypes.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn verified_catalog(fixtures: &[DetachedFixture]) -> VerifiedContentCatalogV1 {
        let manifest = ContentContractManifestV1 {
            schema_version: "content-contract-manifest.v1".to_owned(),
            definitions: fixtures
                .iter()
                .map(|fixture| CardDefinitionEnvelopeV1 {
                    envelope_version: "card-definition-envelope.v1".to_owned(),
                    card_definition_id: fixture.id,
                    faces: fixture.faces.clone(),
                    ability_identities: vec![],
                    semantic_binding: CardSemanticBindingV1::UnprofiledV1,
                    definition_references: vec![],
                    explicit_additional_requirements: vec![],
                })
                .collect(),
        };
        let content_bytes = encode_content_manifest_v1(&manifest).unwrap();
        let content_id = calculate_content_contract_id_v1(&content_bytes).unwrap();
        let provenance = ProvenanceCatalogV1 {
            schema_version: "definition-provenance-catalog.v1".to_owned(),
            records: fixtures
                .iter()
                .map(|fixture| DefinitionProvenanceRecordV1 {
                    content_contract_id: content_id.clone(),
                    card_definition_id: fixture.id,
                    source_provenance: SourceProvenanceV1 {
                        source_snapshot_id: "s1-b-detached-fixture.v1".to_owned(),
                        source_record_id: format!("fixture/{}", fixture.id.0),
                        source_record_codec_id: "s1-b-characteristics-fixture.v1".to_owned(),
                        source_record_digest: [0x5a; 32],
                    },
                })
                .collect(),
        };
        let provenance_bytes = encode_provenance_catalog_v1(&provenance).unwrap();
        VerifiedContentCatalogV1::build_from_bytes(&content_bytes, &content_id, &provenance_bytes)
            .unwrap()
    }

    // This fixture-only join is compiled only in the Rules unit-test module.
    // It exercises the pure mapper on canonical, provenance-verified but
    // deliberately UnprofiledV1 content; it cannot construct production
    // S1QueryAuthority or pass executable-profile admission.
    fn detached_result(
        catalog: &VerifiedContentCatalogV1,
        state: &EngineState,
        object_id: GameObjectId,
    ) -> Result<S1BaseCharacteristicsV1, S1QueryError> {
        state
            .validate_structure()
            .map_err(S1QueryError::InconsistentState)?;
        let object = state
            .zones
            .objects
            .get(&object_id)
            .ok_or(S1QueryError::UnknownObject(object_id))?;
        let location = state
            .zones
            .locations
            .get(&object_id)
            .ok_or(S1QueryError::MissingZoneLocation(object_id))?;
        if object.face_down {
            return Err(S1QueryError::FaceDownCharacteristicsUnsupported(object_id));
        }
        let definition = catalog
            .get(catalog.content_contract_id(), object.card_definition)
            .map_err(|_| S1QueryError::MissingCardDefinition(object.card_definition))?;
        let face_key = state
            .card_rules
            .faces
            .faces
            .get(&object_id)
            .copied()
            .ok_or(S1QueryError::FaceStateMissing(object_id))?;
        let selected_face = definition
            .faces
            .iter()
            .find(|face| face.face_key.0 == face_key)
            .ok_or(S1QueryError::UnknownFace {
                definition: object.card_definition,
                face_key: FaceKey(face_key),
            })?;
        let queried = QueriedObjectV1 {
            object: object_id,
            card_definition: object.card_definition,
            owner: object.owner,
            controller: object.controller,
            zone: location.zone,
            face_key: FaceKey(face_key),
        };
        Ok(derive_base_characteristics(
            queried,
            &selected_face.base_characteristics,
        ))
    }

    fn detached_state(definition: CardDefinitionId, face_key: u32) -> (EngineState, GameObjectId) {
        let state_v2 = crate::basic_land::s1_b_state_with_two_lands_fixture();
        let mut state = state_v2;
        let object_id = *state.zones.objects.keys().next().unwrap();
        state
            .zones
            .objects
            .get_mut(&object_id)
            .unwrap()
            .card_definition = definition;
        state.card_rules.faces.faces.insert(object_id, face_key);
        for (player, identity) in &state.perspective_identities.players {
            if let Some(opaque) = identity.object_to_opaque.get(&object_id) {
                if let Some(record) = state
                    .knowledge
                    .players
                    .get_mut(player)
                    .and_then(|knowledge| knowledge.active.get_mut(opaque))
                {
                    record.card_definition = Some(definition);
                }
            }
        }
        state.validate_structure().unwrap();
        (state, object_id)
    }

    fn fixtures() -> Vec<DetachedFixture> {
        use ManaColorV1::{Blue, White};
        use PrintedManaSymbolV1::{Colorless, Generic, Hybrid};
        // Ojer's front-face values below are hand-authored from the pinned
        // Oracle/CR witness cited by S1 Spec §9. The fixture catalog's
        // provenance intentionally identifies these bytes as test data, not
        // as a production Oracle record or executable profile.
        vec![
            DetachedFixture {
                id: CardDefinitionId(9001),
                faces: vec![face(
                    0,
                    "Ojer Axonil, Deepest Might",
                    Some(vec![
                        Generic(2),
                        PrintedManaSymbolV1::Red,
                        PrintedManaSymbolV1::Red,
                    ]),
                    vec![],
                    type_line(&["Legendary"], &["Creature"], &["God"]),
                    Some((4, 4)),
                )],
            },
            DetachedFixture {
                id: CardDefinitionId(9002),
                faces: vec![face(
                    0,
                    "Hybrid Fixture",
                    Some(vec![Hybrid(White, Blue)]),
                    vec![],
                    type_line(&[], &["Creature"], &["Fixture"]),
                    Some((0, 1)),
                )],
            },
            DetachedFixture {
                id: CardDefinitionId(9003),
                faces: vec![face(
                    0,
                    "Two Color Fixture",
                    Some(vec![PrintedManaSymbolV1::White, PrintedManaSymbolV1::Blue]),
                    vec![],
                    type_line(&[], &["Sorcery"], &[]),
                    None,
                )],
            },
            DetachedFixture {
                id: CardDefinitionId(9004),
                faces: vec![face(
                    0,
                    "Indicator Fixture",
                    None,
                    vec![Blue],
                    type_line(&[], &["Creature"], &[]),
                    Some((1, 1)),
                )],
            },
            DetachedFixture {
                id: CardDefinitionId(9005),
                faces: vec![face(
                    0,
                    "Generic Fixture",
                    Some(vec![Generic(2)]),
                    vec![],
                    type_line(&[], &["Artifact"], &[]),
                    None,
                )],
            },
            DetachedFixture {
                id: CardDefinitionId(9006),
                faces: vec![face(
                    0,
                    "Colorless Symbol Fixture",
                    Some(vec![Colorless]),
                    vec![],
                    type_line(&[], &["Artifact"], &[]),
                    None,
                )],
            },
            DetachedFixture {
                id: CardDefinitionId(9007),
                faces: vec![face(
                    0,
                    "Indicator And Cost Fixture",
                    Some(vec![PrintedManaSymbolV1::Green]),
                    vec![Blue],
                    type_line(&[], &["Creature"], &[]),
                    Some((i32::MIN, i32::MAX)),
                )],
            },
            DetachedFixture {
                id: CardDefinitionId(9008),
                faces: vec![
                    face(
                        0,
                        "Front Fixture",
                        Some(vec![PrintedManaSymbolV1::Blue]),
                        vec![],
                        type_line(&["Legendary"], &["Instant"], &["Front"]),
                        None,
                    ),
                    face(
                        1,
                        "Back Fixture",
                        Some(vec![PrintedManaSymbolV1::Green]),
                        vec![],
                        type_line(&[], &["Creature"], &["Back"]),
                        Some((0, 1)),
                    ),
                ],
            },
        ]
    }

    fn colors(values: &[ManaColorV1]) -> BTreeSet<ManaColorV1> {
        values.iter().copied().collect()
    }

    #[test]
    fn detached_unprofiled_faces_derive_independent_base_facts() {
        let fixtures = fixtures();
        let catalog = verified_catalog(&fixtures);
        let cases = [
            (
                CardDefinitionId(9001),
                0,
                colors(&[ManaColorV1::Red]),
                vec!["Legendary"],
                vec!["Creature"],
                vec!["God"],
                Some((4, 4)),
            ),
            (
                CardDefinitionId(9002),
                0,
                colors(&[ManaColorV1::White, ManaColorV1::Blue]),
                vec![],
                vec!["Creature"],
                vec!["Fixture"],
                Some((0, 1)),
            ),
            (
                CardDefinitionId(9003),
                0,
                colors(&[ManaColorV1::White, ManaColorV1::Blue]),
                vec![],
                vec!["Sorcery"],
                vec![],
                None,
            ),
            (
                CardDefinitionId(9004),
                0,
                colors(&[ManaColorV1::Blue]),
                vec![],
                vec!["Creature"],
                vec![],
                Some((1, 1)),
            ),
            (
                CardDefinitionId(9005),
                0,
                colors(&[]),
                vec![],
                vec!["Artifact"],
                vec![],
                None,
            ),
            (
                CardDefinitionId(9006),
                0,
                colors(&[]),
                vec![],
                vec!["Artifact"],
                vec![],
                None,
            ),
            (
                CardDefinitionId(9007),
                0,
                colors(&[ManaColorV1::Blue, ManaColorV1::Green]),
                vec![],
                vec!["Creature"],
                vec![],
                Some((i64::from(i32::MIN), i64::from(i32::MAX))),
            ),
        ];
        for (definition, face_key, expected_colors, supertypes, card_types, subtypes, pt) in cases {
            let (state, object) = detached_state(definition, face_key);
            let before = state.clone();
            let actual = detached_result(&catalog, &state, object).unwrap();
            assert_eq!(actual, detached_result(&catalog, &state, object).unwrap());
            assert_eq!(actual.queried.face_key.0, face_key);
            assert_eq!(actual.colors, expected_colors, "definition {definition:?}");
            assert_eq!(
                actual.supertypes,
                supertypes
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                actual.card_types,
                card_types
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                actual.subtypes,
                subtypes.into_iter().map(str::to_owned).collect::<Vec<_>>()
            );
            assert_eq!(actual.base_power_toughness, pt);
            assert_eq!(state, before);
        }
    }

    #[test]
    fn detached_face_query_uses_exact_state_face_and_rejects_unknown_face() {
        let fixtures = fixtures();
        let catalog = verified_catalog(&fixtures);
        let (front_state, object) = detached_state(CardDefinitionId(9008), 0);
        let front = detached_result(&catalog, &front_state, object).unwrap();
        assert_eq!(front.queried.face_key, FaceKey(0));
        assert_eq!(front.colors, colors(&[ManaColorV1::Blue]));
        assert_eq!(front.card_types, vec!["Instant"]);
        assert_eq!(front.subtypes, vec!["Front"]);
        assert_eq!(front.base_power_toughness, None);

        let (back_state, object) = detached_state(CardDefinitionId(9008), 1);
        let back = detached_result(&catalog, &back_state, object).unwrap();
        assert_eq!(back.queried.face_key, FaceKey(1));
        assert_eq!(back.colors, colors(&[ManaColorV1::Green]));
        assert_eq!(back.card_types, vec!["Creature"]);
        assert_eq!(back.subtypes, vec!["Back"]);
        assert_eq!(back.base_power_toughness, Some((0, 1)));

        let (unknown_state, object) = detached_state(CardDefinitionId(9008), u32::MAX);
        let before = unknown_state.clone();
        assert!(matches!(
            detached_result(&catalog, &unknown_state, object),
            Err(S1QueryError::UnknownFace { face_key, .. }) if face_key.0 == u32::MAX
        ));
        assert_eq!(unknown_state, before);
    }
}
