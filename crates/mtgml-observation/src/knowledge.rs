//! Ownership: player-safe retained-knowledge DTO vocabulary (V1) and only
//! their existing private validation/access helpers.

use mtgml_model::{OpaqueObjectId, PlayerId, VisibleSequence, ZoneKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerKnownLocationV1 {
    pub zone: ZoneKind,
    pub player: Option<PlayerId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlayerKnowledgeProvenanceV1 {
    InitialConfiguration,
    Observed {
        channel: PlayerKnowledgeChannelV1,
        sequence: VisibleSequence,
        cause: PlayerKnowledgeCauseV1,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerKnowledgeChannelV1 {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerKnowledgeCauseV1 {
    PublicEvent,
    PrivateLook,
    ExplicitReveal,
    OwnPrivateIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerKnownLocationFactV1 {
    pub location: PlayerKnownLocationV1,
    pub provenance: PlayerKnowledgeProvenanceV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerKnowledgeInvalidationV1 {
    pub provenance: PlayerKnowledgeProvenanceV1,
    pub reason: PlayerKnowledgeInvalidationReasonV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerKnowledgeInvalidationReasonV1 {
    HiddenTransition,
    Randomization,
    Shuffle,
    ExplicitForget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlayerKnownObjectV1 {
    Active {
        opaque_object_id: OpaqueObjectId,
        known_definition: Option<mtgml_model::CardDefinitionId>,
        current_known_location_fact: Option<PlayerKnownLocationFactV1>,
        historical_locations: Vec<PlayerKnownLocationFactV1>,
        acquisition: PlayerKnowledgeProvenanceV1,
    },
    Retired {
        opaque_object_id: OpaqueObjectId,
        known_definition: Option<mtgml_model::CardDefinitionId>,
        last_known_location_fact: Option<PlayerKnownLocationFactV1>,
        historical_locations: Vec<PlayerKnownLocationFactV1>,
        acquisition: PlayerKnowledgeProvenanceV1,
        invalidation: PlayerKnowledgeInvalidationV1,
    },
}

impl PlayerKnownObjectV1 {
    pub(super) fn opaque_object_id(&self) -> OpaqueObjectId {
        match self {
            Self::Active {
                opaque_object_id, ..
            }
            | Self::Retired {
                opaque_object_id, ..
            } => *opaque_object_id,
        }
    }

    pub(super) fn historical_locations(&self) -> &[PlayerKnownLocationFactV1] {
        match self {
            Self::Active {
                historical_locations,
                ..
            }
            | Self::Retired {
                historical_locations,
                ..
            } => historical_locations,
        }
    }
}

impl PlayerKnownObjectV1 {
    /// Every provenance-bearing public fact must carry an accepted
    /// channel/cause combination and an observed sequence below this
    /// perspective's next unused visible sequence.
    pub(super) fn provenance_is_valid(&self, next_visible_sequence: VisibleSequence) -> bool {
        let valid = |provenance: &PlayerKnowledgeProvenanceV1| -> bool {
            match provenance {
                // InitialConfiguration owns no visible sequence and is not
                // bound by the perspective cursor.
                PlayerKnowledgeProvenanceV1::InitialConfiguration => true,
                PlayerKnowledgeProvenanceV1::Observed {
                    channel,
                    sequence,
                    cause,
                } => {
                    sequence.0 < next_visible_sequence.0
                        && matches!(
                            (channel, cause),
                            (
                                PlayerKnowledgeChannelV1::Public,
                                PlayerKnowledgeCauseV1::PublicEvent
                            ) | (
                                PlayerKnowledgeChannelV1::Public,
                                PlayerKnowledgeCauseV1::ExplicitReveal
                            ) | (
                                PlayerKnowledgeChannelV1::Private,
                                PlayerKnowledgeCauseV1::PrivateLook
                            ) | (
                                PlayerKnowledgeChannelV1::Private,
                                PlayerKnowledgeCauseV1::OwnPrivateIdentity
                            )
                        )
                }
            }
        };
        match self {
            Self::Active {
                current_known_location_fact,
                historical_locations,
                acquisition,
                ..
            } => {
                valid(acquisition)
                    && current_known_location_fact
                        .as_ref()
                        .is_none_or(|fact| valid(&fact.provenance))
                    && historical_locations
                        .iter()
                        .all(|fact| valid(&fact.provenance))
            }
            Self::Retired {
                last_known_location_fact,
                historical_locations,
                acquisition,
                invalidation,
                ..
            } => {
                // Invalidation must be an observed fact: retirement records
                // an explicit reason *and visible sequence*.
                let invalidation_is_observed = matches!(
                    &invalidation.provenance,
                    PlayerKnowledgeProvenanceV1::Observed { .. }
                );
                invalidation_is_observed
                    && valid(&invalidation.provenance)
                    && valid(acquisition)
                    && last_known_location_fact
                        .as_ref()
                        .is_none_or(|fact| valid(&fact.provenance))
                    && historical_locations
                        .iter()
                        .all(|fact| valid(&fact.provenance))
            }
        }
    }
}

pub(super) fn provenance_sequence(value: &PlayerKnowledgeProvenanceV1) -> Option<VisibleSequence> {
    match value {
        PlayerKnowledgeProvenanceV1::InitialConfiguration => None,
        PlayerKnowledgeProvenanceV1::Observed { sequence, .. } => Some(*sequence),
    }
}
