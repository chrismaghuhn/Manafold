//! Ownership: detached V2 retained-knowledge shapes and only their
//! existing local shape validation.

use std::collections::BTreeMap;

use mtgml_model::{CardDefinitionId, OpaqueObjectId, PhysicalCardId, PlayerId, VisibleSequence};
use serde::{Deserialize, Serialize};

use crate::knowledge::{KnowledgeAcquisitionReason, KnowledgeInvalidationReason};
use crate::m2_shape::M2ShapeViolation;
use crate::zones::ZoneLocation;

/// One retained known-location fact. The fact owns its complete typed
/// provenance; no downstream layer may infer or synthesize it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownLocationFactV2 {
    pub location: ZoneLocation,
    pub provenance: KnowledgeAcquisitionReason,
}

/// Typed invalidation of a retired knowledge record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeInvalidationV2 {
    pub provenance: KnowledgeAcquisitionReason,
    pub reason: KnowledgeInvalidationReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeRecordV2 {
    pub opaque_object: OpaqueObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_card: Option<PhysicalCardId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_definition: Option<CardDefinitionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_location: Option<KnownLocationFactV2>,
    #[serde(default)]
    pub historical_locations: Vec<KnownLocationFactV2>,
    pub acquisition: KnowledgeAcquisitionReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetiredKnowledgeRecordV2 {
    pub opaque_object: OpaqueObjectId,
    #[serde(default)]
    pub physical_card: Option<PhysicalCardId>,
    #[serde(default)]
    pub card_definition: Option<CardDefinitionId>,
    #[serde(default)]
    pub last_known_location: Option<KnownLocationFactV2>,
    #[serde(default)]
    pub historical_locations: Vec<KnownLocationFactV2>,
    pub acquisition: KnowledgeAcquisitionReason,
    pub invalidation: KnowledgeInvalidationV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PlayerKnowledgeStateV2 {
    pub active: BTreeMap<OpaqueObjectId, KnowledgeRecordV2>,
    pub retired: BTreeMap<OpaqueObjectId, RetiredKnowledgeRecordV2>,
    pub next_visible_sequence: VisibleSequence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeStateV2 {
    pub players: BTreeMap<PlayerId, PlayerKnowledgeStateV2>,
}

pub(super) fn validate_knowledge(
    knowledge: &PlayerKnowledgeStateV2,
) -> Result<(), M2ShapeViolation> {
    let provenance_is_valid =
        |provenance: &KnowledgeAcquisitionReason| -> Result<(), M2ShapeViolation> {
            if !provenance.has_accepted_channel_cause()
                || !provenance.is_within_visible_sequence(knowledge.next_visible_sequence)
            {
                return Err(M2ShapeViolation::VisibleSequence);
            }
            Ok(())
        };
    let history_is_valid = |records: &[KnownLocationFactV2]| -> Result<(), M2ShapeViolation> {
        for fact in records {
            provenance_is_valid(&fact.provenance)?;
        }
        // Only observed facts carry visible sequences; the strictly
        // increasing rule compares those observed sequences.
        let observed: Vec<_> = records
            .iter()
            .filter_map(|fact| {
                fact.provenance
                    .observed_sequence()
                    .map(|sequence| sequence.0)
            })
            .collect();
        for window in observed.windows(2) {
            if window[0] >= window[1] {
                return Err(M2ShapeViolation::VisibleSequence);
            }
        }
        Ok(())
    };
    for (opaque, record) in &knowledge.active {
        if opaque != &record.opaque_object || opaque.0 == 0 {
            return Err(M2ShapeViolation::Knowledge);
        }
        provenance_is_valid(&record.acquisition)?;
        if let Some(fact) = record.known_location.as_ref() {
            provenance_is_valid(&fact.provenance)?;
        }
        history_is_valid(&record.historical_locations)?;
    }
    for (opaque, record) in &knowledge.retired {
        if opaque != &record.opaque_object || opaque.0 == 0 || knowledge.active.contains_key(opaque)
        {
            return Err(M2ShapeViolation::Knowledge);
        }
        // INFORMATION_MODEL.md: retirement records an explicit invalidation
        // reason *and visible sequence* â€” an unsequenced initial
        // configuration cannot invalidate anything.
        if record.invalidation.provenance.observed_sequence().is_none() {
            return Err(M2ShapeViolation::Knowledge);
        }
        provenance_is_valid(&record.acquisition)?;
        provenance_is_valid(&record.invalidation.provenance)?;
        if let Some(fact) = record.last_known_location.as_ref() {
            provenance_is_valid(&fact.provenance)?;
        }
        history_is_valid(&record.historical_locations)?;
    }
    Ok(())
}
