//! Immutable, rules-neutral card content foundation.
//!
//! This crate validates and identifies content. It does not execute a card,
//! mutate game state, or provide an executable Card-IR vocabulary.

use mtgml_model::{CapabilityRequirementV1, CardDefinitionId};
use mtgml_persistence::cbor::{self, Value};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CARD_DEFINITION_ENVELOPE_V1: &str = "card-definition-envelope.v1";
pub const CONTENT_CONTRACT_MANIFEST_V1: &str = "content-contract-manifest.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FaceKey(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AbilityKey(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CardSemanticProfileId(String);

impl CardSemanticProfileId {
    pub fn parse(value: impl Into<String>) -> Result<Self, ContentValidationErrorV1> {
        let value = value.into();
        let Some((name, version)) = value.rsplit_once('@') else {
            return Err(ContentValidationErrorV1::InvalidSemanticBinding);
        };
        if name.is_empty()
            || name.split('/').any(|segment| {
                segment.is_empty()
                    || !segment
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                    || !(segment.as_bytes()[0].is_ascii_lowercase()
                        || segment.as_bytes()[0].is_ascii_digit())
            })
        {
            return Err(ContentValidationErrorV1::InvalidSemanticBinding);
        }
        let parts = version.split('.').collect::<Vec<_>>();
        if parts.len() != 3
            || parts.iter().any(|part| {
                part.is_empty()
                    || !part.bytes().all(|b| b.is_ascii_digit())
                    || (part.len() > 1 && part.starts_with('0'))
            })
        {
            return Err(ContentValidationErrorV1::InvalidSemanticBinding);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentContractManifestV1 {
    pub schema_version: String,
    pub definitions: Vec<CardDefinitionEnvelopeV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardDefinitionEnvelopeV1 {
    pub envelope_version: String,
    pub card_definition_id: CardDefinitionId,
    pub faces: Vec<FaceDefinitionV1>,
    pub ability_identities: Vec<AbilityIdentityV1>,
    pub semantic_binding: CardSemanticBindingV1,
    pub definition_references: Vec<DefinitionReferenceV1>,
    pub explicit_additional_requirements: Vec<CapabilityRequirementV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaceDefinitionV1 {
    pub face_key: FaceKey,
    pub base_characteristics: BaseCharacteristicsV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbilityIdentityV1 {
    pub ability_key: AbilityKey,
    pub face_key: FaceKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeLineV1 {
    pub supertypes: Vec<String>,
    pub card_types: Vec<String>,
    pub subtypes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseCharacteristicsV1 {
    pub name: String,
    pub mana_cost: Option<Vec<PrintedManaSymbolV1>>,
    pub color_indicator: Vec<ManaColorV1>,
    pub type_line: TypeLineV1,
    pub power_toughness: Option<(i32, i32)>,
    pub loyalty: Option<i32>,
    pub defense: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaColorV1 {
    White,
    Blue,
    Black,
    Red,
    Green,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "variant", content = "value", rename_all = "snake_case")]
pub enum PrintedManaSymbolV1 {
    Generic(u32),
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
    Hybrid(ManaColorV1, ManaColorV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum CardSemanticBindingV1 {
    UnprofiledV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionReferenceV1 {
    pub relation: String,
    pub target: CardDefinitionId,
    pub target_face_key: Option<FaceKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProvenanceV1 {
    pub source_snapshot_id: String,
    pub source_record_id: String,
    pub source_record_codec_id: String,
    pub source_record_digest: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionProvenanceRecordV1 {
    pub content_contract_id: mtgml_model::ContentContractIdV1,
    pub card_definition_id: CardDefinitionId,
    pub source_provenance: SourceProvenanceV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceCatalogV1 {
    pub schema_version: String,
    pub records: Vec<DefinitionProvenanceRecordV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ContentValidationErrorV1 {
    #[error("manifest schema or envelope version is unknown")]
    UnknownEnvelopeVersion,
    #[error("definitions are not in unique ascending identity order")]
    DuplicateDefinitionId,
    #[error("face identity is invalid or noncanonical")]
    InvalidLocalIdentity,
    #[error("ability identity or face binding is invalid")]
    InvalidLocalReference,
    #[error("characteristic value is invalid")]
    InvalidCharacteristic,
    #[error("semantic binding is not admitted by this contract")]
    ProfiledBindingNotAdmitted,
    #[error("semantic binding identity is malformed")]
    InvalidSemanticBinding,
    #[error("definition reference is invalid or noncanonical")]
    InvalidDefinitionReference,
    #[error("capability requirement is invalid or noncanonical")]
    InvalidCapabilityRequirement,
    #[error("canonical encoding is malformed or does not match the typed value")]
    MalformedEnvelope,
}

pub fn validate_content_manifest_v1(
    manifest: &ContentContractManifestV1,
) -> Result<(), ContentValidationErrorV1> {
    if manifest.schema_version != CONTENT_CONTRACT_MANIFEST_V1 {
        return Err(ContentValidationErrorV1::UnknownEnvelopeVersion);
    }
    let mut previous_id = None;
    for definition in &manifest.definitions {
        if definition.envelope_version != CARD_DEFINITION_ENVELOPE_V1 {
            return Err(ContentValidationErrorV1::UnknownEnvelopeVersion);
        }
        match previous_id {
            Some(previous) if previous == definition.card_definition_id.0 => {
                return Err(ContentValidationErrorV1::DuplicateDefinitionId)
            }
            Some(previous) if previous > definition.card_definition_id.0 => {
                return Err(ContentValidationErrorV1::DuplicateDefinitionId)
            }
            _ => {}
        }
        previous_id = Some(definition.card_definition_id.0);
        validate_definition(definition)?;
    }
    Ok(())
}

fn validate_definition(
    definition: &CardDefinitionEnvelopeV1,
) -> Result<(), ContentValidationErrorV1> {
    if definition.faces.is_empty() {
        return Err(ContentValidationErrorV1::InvalidLocalIdentity);
    }
    for (index, face) in definition.faces.iter().enumerate() {
        if usize::try_from(face.face_key.0).ok() != Some(index) {
            return Err(ContentValidationErrorV1::InvalidLocalIdentity);
        }
        validate_characteristics(&face.base_characteristics)?;
    }

    let mut ability_keys = std::collections::BTreeSet::new();
    let mut previous_ability = None;
    for identity in &definition.ability_identities {
        if identity.face_key.0 as usize >= definition.faces.len()
            || !ability_keys.insert(identity.ability_key)
        {
            return Err(ContentValidationErrorV1::InvalidLocalReference);
        }
        let key = (identity.face_key.0, identity.ability_key.0);
        if previous_ability.is_some_and(|previous| previous >= key) {
            return Err(ContentValidationErrorV1::InvalidLocalIdentity);
        }
        previous_ability = Some(key);
    }

    if !matches!(
        definition.semantic_binding,
        CardSemanticBindingV1::UnprofiledV1
    ) {
        return Err(ContentValidationErrorV1::ProfiledBindingNotAdmitted);
    }
    validate_references(&definition.definition_references)?;
    validate_requirements(&definition.explicit_additional_requirements)?;
    Ok(())
}

fn validate_characteristics(value: &BaseCharacteristicsV1) -> Result<(), ContentValidationErrorV1> {
    if !valid_text(&value.name) {
        return Err(ContentValidationErrorV1::InvalidCharacteristic);
    }
    for symbols in value.mana_cost.iter().flatten() {
        match symbols {
            PrintedManaSymbolV1::Generic(n) if *n == 0 => {
                return Err(ContentValidationErrorV1::InvalidCharacteristic)
            }
            PrintedManaSymbolV1::Hybrid(a, b) if a == b => {
                return Err(ContentValidationErrorV1::InvalidCharacteristic)
            }
            _ => {}
        }
    }
    let colors = value
        .color_indicator
        .iter()
        .copied()
        .map(color_value)
        .collect::<Vec<_>>();
    if colors.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ContentValidationErrorV1::InvalidCharacteristic);
    }
    for terms in [
        &value.type_line.supertypes,
        &value.type_line.card_types,
        &value.type_line.subtypes,
    ] {
        let mut seen = std::collections::BTreeSet::new();
        if terms
            .iter()
            .any(|term| !valid_text(term) || !seen.insert(term))
        {
            return Err(ContentValidationErrorV1::InvalidCharacteristic);
        }
    }
    Ok(())
}

fn validate_references(
    references: &[DefinitionReferenceV1],
) -> Result<(), ContentValidationErrorV1> {
    let mut previous = None;
    for reference in references {
        if reference.relation != "required_definition" {
            return Err(ContentValidationErrorV1::InvalidDefinitionReference);
        }
        let key = (
            reference.target.0,
            reference.target_face_key.map(|face| face.0),
            reference.relation.as_str(),
        );
        if previous.is_some_and(|value| value >= key) {
            return Err(ContentValidationErrorV1::InvalidDefinitionReference);
        }
        previous = Some(key);
    }
    Ok(())
}

fn validate_requirements(
    requirements: &[CapabilityRequirementV1],
) -> Result<(), ContentValidationErrorV1> {
    let mut previous: Option<(&str, &str)> = None;
    let mut seen_key: Option<&str> = None;
    for requirement in requirements {
        if !valid_capability_key(&requirement.key)
            || !valid_capability_version(&requirement.version)
        {
            return Err(ContentValidationErrorV1::InvalidCapabilityRequirement);
        }
        let key = (requirement.key.as_str(), requirement.version.as_str());
        if previous.is_some_and(|value| value >= key) || seen_key == Some(requirement.key.as_str())
        {
            return Err(ContentValidationErrorV1::InvalidCapabilityRequirement);
        }
        previous = Some(key);
        seen_key = Some(requirement.key.as_str());
    }
    Ok(())
}

fn valid_capability_key(value: &str) -> bool {
    let segments = value.split('/').collect::<Vec<_>>();
    let word = |segment: &str| {
        let mut bytes = segment.bytes();
        bytes
            .next()
            .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
            && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    match segments.as_slice() {
        [head, rest @ ..]
            if ["rules", "mechanic", "decision", "visibility", "tooling"].contains(head) =>
        {
            !rest.is_empty() && rest.iter().all(|segment| word(segment))
        }
        ["format", namespace, rest @ ..] => {
            !namespace.is_empty()
                && namespace
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                && !rest.is_empty()
                && rest.iter().all(|segment| word(segment))
        }
        _ => false,
    }
}

fn valid_capability_version(value: &str) -> bool {
    let pieces = value.split('.').collect::<Vec<_>>();
    pieces.len() == 3
        && pieces
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

fn color_value(color: ManaColorV1) -> Vec<u8> {
    let text = match color {
        ManaColorV1::White => "white",
        ManaColorV1::Blue => "blue",
        ManaColorV1::Black => "black",
        ManaColorV1::Red => "red",
        ManaColorV1::Green => "green",
    };
    cbor::encode_canonical(&Value::Text(text.to_owned())).expect("color is bounded ASCII")
}

pub fn encode_content_manifest_v1(
    manifest: &ContentContractManifestV1,
) -> Result<Vec<u8>, ContentValidationErrorV1> {
    validate_content_manifest_v1(manifest)?;
    let definitions = manifest.definitions.iter().map(definition_value).collect();
    let value = Value::Array(vec![
        Value::Text(CONTENT_CONTRACT_MANIFEST_V1.to_owned()),
        Value::Text("mtgml.content-contract.v1".to_owned()),
        Value::Array(definitions),
    ]);
    cbor::encode_canonical(&value).map_err(|_| ContentValidationErrorV1::MalformedEnvelope)
}

/// Decode one canonical CBOR content manifest and require that the closed
/// typed representation re-encodes to the exact original payload bytes.
pub fn decode_content_manifest_v1(
    bytes: &[u8],
) -> Result<ContentContractManifestV1, ContentValidationErrorV1> {
    let value =
        cbor::decode_canonical(bytes).map_err(|_| ContentValidationErrorV1::MalformedEnvelope)?;
    let manifest = manifest_from_value(value)?;
    validate_content_manifest_v1(&manifest)?;
    let encoded = encode_content_manifest_v1(&manifest)?;
    if encoded != bytes {
        return Err(ContentValidationErrorV1::MalformedEnvelope);
    }
    Ok(manifest)
}

fn manifest_from_value(
    value: Value,
) -> Result<ContentContractManifestV1, ContentValidationErrorV1> {
    let mut fields = array(value, 3)?;
    if take_text(fields.remove(0))? != CONTENT_CONTRACT_MANIFEST_V1
        || take_text(fields.remove(0))? != "mtgml.content-contract.v1"
    {
        return Err(ContentValidationErrorV1::UnknownEnvelopeVersion);
    }
    let definitions = take_array(fields.remove(0))?
        .into_iter()
        .map(definition_from_value)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ContentContractManifestV1 {
        schema_version: CONTENT_CONTRACT_MANIFEST_V1.to_owned(),
        definitions,
    })
}

fn definition_from_value(
    value: Value,
) -> Result<CardDefinitionEnvelopeV1, ContentValidationErrorV1> {
    let mut fields = array(value, 7)?;
    if take_text(fields.remove(0))? != CARD_DEFINITION_ENVELOPE_V1 {
        return Err(ContentValidationErrorV1::UnknownEnvelopeVersion);
    }
    let card_definition_id = CardDefinitionId(take_unsigned(fields.remove(0))?);
    let faces = take_array(fields.remove(0))?
        .into_iter()
        .map(face_from_value)
        .collect::<Result<Vec<_>, _>>()?;
    let ability_identities = take_array(fields.remove(0))?
        .into_iter()
        .map(ability_from_value)
        .collect::<Result<Vec<_>, _>>()?;
    let semantic_binding = binding_from_value(fields.remove(0))?;
    let definition_references = take_array(fields.remove(0))?
        .into_iter()
        .map(reference_from_value)
        .collect::<Result<Vec<_>, _>>()?;
    let explicit_additional_requirements = take_array(fields.remove(0))?
        .into_iter()
        .map(requirement_from_value)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CardDefinitionEnvelopeV1 {
        envelope_version: CARD_DEFINITION_ENVELOPE_V1.to_owned(),
        card_definition_id,
        faces,
        ability_identities,
        semantic_binding,
        definition_references,
        explicit_additional_requirements,
    })
}

fn face_from_value(value: Value) -> Result<FaceDefinitionV1, ContentValidationErrorV1> {
    let mut fields = array(value, 2)?;
    Ok(FaceDefinitionV1 {
        face_key: FaceKey(take_u32(fields.remove(0))?),
        base_characteristics: characteristics_from_value(fields.remove(0))?,
    })
}

fn ability_from_value(value: Value) -> Result<AbilityIdentityV1, ContentValidationErrorV1> {
    let mut fields = array(value, 2)?;
    Ok(AbilityIdentityV1 {
        ability_key: AbilityKey(take_u32(fields.remove(0))?),
        face_key: FaceKey(take_u32(fields.remove(0))?),
    })
}

fn characteristics_from_value(
    value: Value,
) -> Result<BaseCharacteristicsV1, ContentValidationErrorV1> {
    let mut fields = array(value, 7)?;
    let name = take_text(fields.remove(0))?;
    let mana_cost = match fields.remove(0) {
        Value::Null => None,
        Value::Array(values) => Some(
            values
                .into_iter()
                .map(mana_symbol_from_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        _ => return Err(ContentValidationErrorV1::MalformedEnvelope),
    };
    let color_indicator = take_array(fields.remove(0))?
        .into_iter()
        .map(|value| parse_color(&take_text(value)?))
        .collect::<Result<Vec<_>, _>>()?;
    let mut type_line = array(fields.remove(0), 3)?;
    let supertypes = take_text_array(type_line.remove(0))?;
    let card_types = take_text_array(type_line.remove(0))?;
    let subtypes = take_text_array(type_line.remove(0))?;
    let power_toughness = match fields.remove(0) {
        Value::Null => None,
        Value::Array(values) => {
            let mut pair = values;
            if pair.len() != 2 {
                return Err(ContentValidationErrorV1::MalformedEnvelope);
            }
            let toughness = take_i32(pair.pop().unwrap())?;
            let power = take_i32(pair.pop().unwrap())?;
            Some((power, toughness))
        }
        _ => return Err(ContentValidationErrorV1::MalformedEnvelope),
    };
    let loyalty = take_optional_i32(fields.remove(0))?;
    let defense = take_optional_i32(fields.remove(0))?;
    Ok(BaseCharacteristicsV1 {
        name,
        mana_cost,
        color_indicator,
        type_line: TypeLineV1 {
            supertypes,
            card_types,
            subtypes,
        },
        power_toughness,
        loyalty,
        defense,
    })
}

fn mana_symbol_from_value(value: Value) -> Result<PrintedManaSymbolV1, ContentValidationErrorV1> {
    let mut fields = array(value, 2)?;
    let variant = take_text(fields.remove(0))?;
    let payload = fields.remove(0);
    let unit = || matches!(payload, Value::Null);
    match variant.as_str() {
        "generic" => match payload {
            Value::Unsigned(value) if value > 0 && value <= u64::from(u32::MAX) => {
                Ok(PrintedManaSymbolV1::Generic(value as u32))
            }
            _ => Err(ContentValidationErrorV1::InvalidCharacteristic),
        },
        "white" if unit() => Ok(PrintedManaSymbolV1::White),
        "blue" if unit() => Ok(PrintedManaSymbolV1::Blue),
        "black" if unit() => Ok(PrintedManaSymbolV1::Black),
        "red" if unit() => Ok(PrintedManaSymbolV1::Red),
        "green" if unit() => Ok(PrintedManaSymbolV1::Green),
        "colorless" if unit() => Ok(PrintedManaSymbolV1::Colorless),
        "hybrid" => {
            let mut pair = array(payload, 2)?;
            let b = parse_color(&take_text(pair.pop().unwrap())?)?;
            let a = parse_color(&take_text(pair.pop().unwrap())?)?;
            if a == b {
                return Err(ContentValidationErrorV1::InvalidCharacteristic);
            }
            Ok(PrintedManaSymbolV1::Hybrid(a, b))
        }
        _ => Err(ContentValidationErrorV1::InvalidCharacteristic),
    }
}

fn binding_from_value(value: Value) -> Result<CardSemanticBindingV1, ContentValidationErrorV1> {
    let mut fields = array(value, 2)?;
    match take_text(fields.remove(0))?.as_str() {
        "unprofiled" if matches!(fields.remove(0), Value::Null) => {
            Ok(CardSemanticBindingV1::UnprofiledV1)
        }
        "profiled" => Err(ContentValidationErrorV1::ProfiledBindingNotAdmitted),
        _ => Err(ContentValidationErrorV1::InvalidSemanticBinding),
    }
}

fn reference_from_value(value: Value) -> Result<DefinitionReferenceV1, ContentValidationErrorV1> {
    let mut fields = array(value, 3)?;
    let relation = take_text(fields.remove(0))?;
    let target = CardDefinitionId(take_unsigned(fields.remove(0))?);
    let target_face_key = match fields.remove(0) {
        Value::Null => None,
        value => Some(FaceKey(take_u32(value)?)),
    };
    Ok(DefinitionReferenceV1 {
        relation,
        target,
        target_face_key,
    })
}

fn requirement_from_value(
    value: Value,
) -> Result<CapabilityRequirementV1, ContentValidationErrorV1> {
    let mut fields = array(value, 2)?;
    Ok(CapabilityRequirementV1 {
        key: take_text(fields.remove(0))?,
        version: take_text(fields.remove(0))?,
    })
}

fn take_optional_i32(value: Value) -> Result<Option<i32>, ContentValidationErrorV1> {
    match value {
        Value::Null => Ok(None),
        value => take_i32(value).map(Some),
    }
}

fn take_i32(value: Value) -> Result<i32, ContentValidationErrorV1> {
    let number = match value {
        Value::Signed(number) => number,
        Value::Unsigned(number) => {
            i64::try_from(number).map_err(|_| ContentValidationErrorV1::InvalidCharacteristic)?
        }
        _ => return Err(ContentValidationErrorV1::MalformedEnvelope),
    };
    i32::try_from(number).map_err(|_| ContentValidationErrorV1::InvalidCharacteristic)
}

fn take_u32(value: Value) -> Result<u32, ContentValidationErrorV1> {
    u32::try_from(take_unsigned(value)?)
        .map_err(|_| ContentValidationErrorV1::InvalidCharacteristic)
}

fn take_unsigned(value: Value) -> Result<u64, ContentValidationErrorV1> {
    match value {
        Value::Unsigned(number) => Ok(number),
        _ => Err(ContentValidationErrorV1::MalformedEnvelope),
    }
}

fn take_text(value: Value) -> Result<String, ContentValidationErrorV1> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(ContentValidationErrorV1::MalformedEnvelope),
    }
}

fn take_text_array(value: Value) -> Result<Vec<String>, ContentValidationErrorV1> {
    take_array(value)?.into_iter().map(take_text).collect()
}

fn take_array(value: Value) -> Result<Vec<Value>, ContentValidationErrorV1> {
    match value {
        Value::Array(values) => Ok(values),
        _ => Err(ContentValidationErrorV1::MalformedEnvelope),
    }
}

fn array(value: Value, expected: usize) -> Result<Vec<Value>, ContentValidationErrorV1> {
    let values = take_array(value)?;
    if values.len() != expected {
        return Err(ContentValidationErrorV1::MalformedEnvelope);
    }
    Ok(values)
}

fn parse_color(value: &str) -> Result<ManaColorV1, ContentValidationErrorV1> {
    match value {
        "white" => Ok(ManaColorV1::White),
        "blue" => Ok(ManaColorV1::Blue),
        "black" => Ok(ManaColorV1::Black),
        "red" => Ok(ManaColorV1::Red),
        "green" => Ok(ManaColorV1::Green),
        _ => Err(ContentValidationErrorV1::InvalidCharacteristic),
    }
}

fn definition_value(definition: &CardDefinitionEnvelopeV1) -> Value {
    Value::Array(vec![
        Value::Text(CARD_DEFINITION_ENVELOPE_V1.to_owned()),
        Value::Unsigned(definition.card_definition_id.0),
        Value::Array(definition.faces.iter().map(face_value).collect()),
        Value::Array(
            definition
                .ability_identities
                .iter()
                .map(ability_value)
                .collect(),
        ),
        Value::Array(vec![Value::Text("unprofiled".to_owned()), Value::Null]),
        Value::Array(
            definition
                .definition_references
                .iter()
                .map(reference_value)
                .collect(),
        ),
        Value::Array(
            definition
                .explicit_additional_requirements
                .iter()
                .map(requirement_value)
                .collect(),
        ),
    ])
}

fn face_value(face: &FaceDefinitionV1) -> Value {
    Value::Array(vec![
        Value::Unsigned(u64::from(face.face_key.0)),
        characteristics_value(&face.base_characteristics),
    ])
}

fn ability_value(value: &AbilityIdentityV1) -> Value {
    Value::Array(vec![
        Value::Unsigned(u64::from(value.ability_key.0)),
        Value::Unsigned(u64::from(value.face_key.0)),
    ])
}

fn characteristics_value(value: &BaseCharacteristicsV1) -> Value {
    Value::Array(vec![
        Value::Text(value.name.clone()),
        value.mana_cost.as_ref().map_or(Value::Null, |symbols| {
            Value::Array(symbols.iter().map(mana_symbol_value).collect())
        }),
        Value::Array(
            value
                .color_indicator
                .iter()
                .copied()
                .map(color_name)
                .map(text)
                .collect(),
        ),
        Value::Array(vec![
            Value::Array(
                value
                    .type_line
                    .supertypes
                    .iter()
                    .cloned()
                    .map(text)
                    .collect(),
            ),
            Value::Array(
                value
                    .type_line
                    .card_types
                    .iter()
                    .cloned()
                    .map(text)
                    .collect(),
            ),
            Value::Array(value.type_line.subtypes.iter().cloned().map(text).collect()),
        ]),
        value.power_toughness.map_or(Value::Null, |(p, t)| {
            Value::Array(vec![
                Value::Signed(i64::from(p)),
                Value::Signed(i64::from(t)),
            ])
        }),
        value
            .loyalty
            .map_or(Value::Null, |n| Value::Signed(i64::from(n))),
        value
            .defense
            .map_or(Value::Null, |n| Value::Signed(i64::from(n))),
    ])
}

fn mana_symbol_value(symbol: &PrintedManaSymbolV1) -> Value {
    let (name, payload) = match symbol {
        PrintedManaSymbolV1::Generic(n) => ("generic", Value::Unsigned(u64::from(*n))),
        PrintedManaSymbolV1::White => ("white", Value::Null),
        PrintedManaSymbolV1::Blue => ("blue", Value::Null),
        PrintedManaSymbolV1::Black => ("black", Value::Null),
        PrintedManaSymbolV1::Red => ("red", Value::Null),
        PrintedManaSymbolV1::Green => ("green", Value::Null),
        PrintedManaSymbolV1::Colorless => ("colorless", Value::Null),
        PrintedManaSymbolV1::Hybrid(a, b) => (
            "hybrid",
            Value::Array(vec![text(color_name(*a)), text(color_name(*b))]),
        ),
    };
    Value::Array(vec![text(name), payload])
}

fn reference_value(reference: &DefinitionReferenceV1) -> Value {
    Value::Array(vec![
        text(reference.relation.clone()),
        Value::Unsigned(reference.target.0),
        reference
            .target_face_key
            .map_or(Value::Null, |face| Value::Unsigned(u64::from(face.0))),
    ])
}

fn requirement_value(requirement: &CapabilityRequirementV1) -> Value {
    Value::Array(vec![
        text(requirement.key.clone()),
        text(requirement.version.clone()),
    ])
}

fn color_name(color: ManaColorV1) -> &'static str {
    match color {
        ManaColorV1::White => "white",
        ManaColorV1::Blue => "blue",
        ManaColorV1::Black => "black",
        ManaColorV1::Red => "red",
        ManaColorV1::Green => "green",
    }
}

fn text(value: impl Into<String>) -> Value {
    Value::Text(value.into())
}
