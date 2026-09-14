//! Ownership: detached V2 retained-knowledge shapes and only their
//! existing local shape validation.

use std::collections::BTreeMap;

use mtgml_model::{CardDefinitionId, OpaqueObjectId, PhysicalCardId, PlayerId, VisibleSequence};
use serde::{Deserialize, Serialize};

use crate::knowledge::{KnowledgeAcquisitionReason, KnowledgeInvalidationReason};
use crate::m2_shape::M2ShapeViolation;
use crate::zones::{ZoneLocation, ZonePosition};

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

fn validate_location_against_acquisition(
    acquisition: &KnowledgeAcquisitionReason,
    provenance: &KnowledgeAcquisitionReason,
) -> Result<(), M2ShapeViolation> {
    let Some(acquisition_sequence) = acquisition.observed_sequence() else {
        return Ok(());
    };
    let Some(sequence) = provenance.observed_sequence() else {
        return Err(M2ShapeViolation::Knowledge);
    };
    if sequence.0 < acquisition_sequence.0
        || (sequence == acquisition_sequence && provenance != acquisition)
    {
        return Err(M2ShapeViolation::Knowledge);
    }
    Ok(())
}

fn validate_location_chronology(
    acquisition: &KnowledgeAcquisitionReason,
    historical_locations: &[KnownLocationFactV2],
    current_or_last_known: Option<&KnownLocationFactV2>,
    invalidation: Option<&KnowledgeInvalidationV2>,
) -> Result<(), M2ShapeViolation> {
    if historical_locations
        .iter()
        .chain(current_or_last_known.into_iter())
        .any(|fact| {
            matches!(
                fact.location.position,
                ZonePosition::Bottom { .. } | ZonePosition::Index { .. }
            )
        })
    {
        return Err(M2ShapeViolation::Knowledge);
    }
    let location_facts = historical_locations
        .iter()
        .chain(current_or_last_known.into_iter());
    let initial_location_count = location_facts
        .clone()
        .filter(|fact| {
            matches!(
                fact.provenance,
                KnowledgeAcquisitionReason::InitialConfiguration
            )
        })
        .count();
    if initial_location_count > 1 {
        return Err(M2ShapeViolation::Knowledge);
    }
    if acquisition.observed_sequence().is_some() && initial_location_count != 0 {
        return Err(M2ShapeViolation::Knowledge);
    }

    let mut saw_observed_history = false;
    let mut newest_observed_history = None;
    for fact in historical_locations {
        validate_location_against_acquisition(acquisition, &fact.provenance)?;
        match fact.provenance.observed_sequence() {
            None if saw_observed_history => return Err(M2ShapeViolation::Knowledge),
            None => {}
            Some(sequence) => {
                if newest_observed_history.is_some_and(|previous| sequence.0 <= previous) {
                    return Err(M2ShapeViolation::VisibleSequence);
                }
                saw_observed_history = true;
                newest_observed_history = Some(sequence.0);
            }
        }
    }

    if let Some(current_or_last_known) = current_or_last_known {
        validate_location_against_acquisition(acquisition, &current_or_last_known.provenance)?;
        match current_or_last_known.provenance.observed_sequence() {
            None => {
                if !historical_locations.is_empty() {
                    return Err(M2ShapeViolation::Knowledge);
                }
            }
            Some(sequence) => {
                if newest_observed_history.is_some_and(|previous| sequence.0 <= previous) {
                    return Err(M2ShapeViolation::Knowledge);
                }
                if let Some(acquisition_sequence) = acquisition.observed_sequence() {
                    if sequence.0 == acquisition_sequence.0
                        && current_or_last_known.provenance != *acquisition
                    {
                        return Err(M2ShapeViolation::Knowledge);
                    }
                }
            }
        }
    }

    if let Some(invalidation) = invalidation {
        let Some(invalidation_sequence) = invalidation.provenance.observed_sequence() else {
            return Err(M2ShapeViolation::Knowledge);
        };
        let mut newest_prior_observed = acquisition.observed_sequence().map(|sequence| sequence.0);
        for fact in historical_locations {
            if let Some(sequence) = fact.provenance.observed_sequence() {
                newest_prior_observed = Some(
                    newest_prior_observed.map_or(sequence.0, |previous| previous.max(sequence.0)),
                );
            }
        }
        if let Some(fact) = current_or_last_known {
            if let Some(sequence) = fact.provenance.observed_sequence() {
                newest_prior_observed = Some(
                    newest_prior_observed.map_or(sequence.0, |previous| previous.max(sequence.0)),
                );
            }
        }
        if newest_prior_observed.is_some_and(|previous| invalidation_sequence.0 <= previous) {
            return Err(M2ShapeViolation::Knowledge);
        }
    }
    Ok(())
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
    for (opaque, record) in &knowledge.active {
        if opaque != &record.opaque_object || opaque.0 == 0 {
            return Err(M2ShapeViolation::Knowledge);
        }
        provenance_is_valid(&record.acquisition)?;
        if let Some(fact) = record.known_location.as_ref() {
            provenance_is_valid(&fact.provenance)?;
        }
        for fact in &record.historical_locations {
            provenance_is_valid(&fact.provenance)?;
        }
        validate_location_chronology(
            &record.acquisition,
            &record.historical_locations,
            record.known_location.as_ref(),
            None,
        )?;
    }
    for (opaque, record) in &knowledge.retired {
        if opaque != &record.opaque_object || opaque.0 == 0 || knowledge.active.contains_key(opaque)
        {
            return Err(M2ShapeViolation::Knowledge);
        }
        provenance_is_valid(&record.acquisition)?;
        provenance_is_valid(&record.invalidation.provenance)?;
        if let Some(fact) = record.last_known_location.as_ref() {
            provenance_is_valid(&fact.provenance)?;
        }
        for fact in &record.historical_locations {
            provenance_is_valid(&fact.provenance)?;
        }
        validate_location_chronology(
            &record.acquisition,
            &record.historical_locations,
            record.last_known_location.as_ref(),
            Some(&record.invalidation),
        )?;
    }
    Ok(())
}
