//! Perspective-safe observations, information states, and observed events.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_decision::{PlayerDecisionRequest, PlayerDecisionRequestV2};
use mtgml_model::{
    EpisodeStatus, EventSequence, InformationStateDigest, InformationStateDigestV2,
    ObservationDigest, OpaqueObjectId, PlayerId, StateRevision, VisibleSequence, ZoneKind,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const OBSERVATION_SCHEMA: &str = "observation-envelope.v1";
pub const INFORMATION_STATE_SCHEMA: &str = "information-state-envelope.v1";
pub const OBSERVED_EVENT_SCHEMA: &str = "observed-event-envelope.v1";
pub const PLAYER_STEP_SCHEMA: &str = "player-step.v1";
pub const INFORMATION_STATE_SCHEMA_V2: &str = "information-state-envelope.v2";
pub const OBSERVED_EVENT_SCHEMA_V2: &str = "observed-event-envelope.v2";
pub const PLAYER_STEP_SCHEMA_V2: &str = "player-step.v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationEnvelope {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
    pub payload_codec: String,
    pub payload_base64: String,
    pub digest: ObservationDigest,
}

impl ObservationEnvelope {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVATION_SCHEMA || self.payload_codec.is_empty() {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        let decoded = STANDARD
            .decode(&self.payload_base64)
            .map_err(|_| ObservationValidationError::Base64)?;
        if STANDARD.encode(decoded) != self.payload_base64 {
            return Err(ObservationValidationError::Base64);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InformationStateEnvelope {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
    pub current_observation: ObservationEnvelope,
    pub public_history_length: u64,
    pub private_history_length: u64,
    pub digest: InformationStateDigest,
}

impl InformationStateEnvelope {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != INFORMATION_STATE_SCHEMA {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.current_observation.validate()?;
        if self.current_observation.perspective != self.perspective
            || self.current_observation.state_revision != self.state_revision
        {
            return Err(ObservationValidationError::InformationStateMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedEventKind {
    ObjectMoved {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        old_object: Option<OpaqueObjectId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        new_object: Option<OpaqueObjectId>,
        from: ZoneKind,
        to: ZoneKind,
    },
    ObjectCeasedToExist {
        object: OpaqueObjectId,
    },
    LifeChanged {
        player: PlayerId,
        from: i64,
        to: i64,
    },
    ObjectTapped {
        object: OpaqueObjectId,
        tapped: bool,
    },
    DecisionAvailable {
        actor: PlayerId,
    },
    RandomOutcomeVisible {
        label: String,
        exclusive_upper_bound: u64,
        value: u64,
    },
    PublicOutcome {
        code: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedEventEnvelope {
    pub schema_version: String,
    pub sequence: EventSequence,
    pub state_revision: StateRevision,
    pub event: ObservedEventKind,
}

impl ObservedEventEnvelope {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVED_EVENT_SCHEMA {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        match &self.event {
            ObservedEventKind::RandomOutcomeVisible {
                label,
                exclusive_upper_bound,
                value,
            } => {
                if label.is_empty() {
                    return Err(ObservationValidationError::EmptyEventText);
                }
                if *exclusive_upper_bound == 0 || *value >= *exclusive_upper_bound {
                    return Err(ObservationValidationError::RandomOutcome);
                }
            }
            ObservedEventKind::PublicOutcome { code } if code.is_empty() => {
                return Err(ObservationValidationError::EmptyEventText);
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerStep {
    pub schema_version: String,
    pub information_state: InformationStateEnvelope,
    pub observed_events: Vec<ObservedEventEnvelope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_decision: Option<PlayerDecisionRequest>,
    pub status: EpisodeStatus,
}

impl PlayerStep {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != PLAYER_STEP_SCHEMA {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.information_state.validate()?;
        self.status
            .validate()
            .map_err(|_| ObservationValidationError::EpisodeStatus)?;
        let perspective = self.information_state.perspective;
        let revision = self.information_state.state_revision;
        for event in &self.observed_events {
            event.validate()?;
            if event.state_revision > revision {
                return Err(ObservationValidationError::FutureEvent);
            }
        }
        if let Some(decision) = &self.next_decision {
            decision
                .validate()
                .map_err(|_| ObservationValidationError::Decision)?;
            if decision.actor != perspective || decision.state_revision != revision {
                return Err(ObservationValidationError::Decision);
            }
        }
        if !matches!(&self.status, EpisodeStatus::Running) && self.next_decision.is_some() {
            return Err(ObservationValidationError::Decision);
        }
        Ok(())
    }

    pub fn observation(&self) -> &ObservationEnvelope {
        &self.information_state.current_observation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ObservationValidationError {
    #[error("unsupported schema version or empty codec")]
    SchemaOrCodec,
    #[error("payload is not canonical base64")]
    Base64,
    #[error("information state and current observation disagree")]
    InformationStateMismatch,
    #[error("observed event label/code must be non-empty")]
    EmptyEventText,
    #[error("random outcome is outside its declared range")]
    RandomOutcome,
    #[error("observed event belongs to a future revision")]
    FutureEvent,
    #[error("next decision is invalid for this endpoint")]
    Decision,
    #[error("episode status is invalid")]
    EpisodeStatus,
    #[error("information-state retained knowledge is not canonical")]
    RetainedKnowledge,
    #[error("information-state visible sequence is not monotonic")]
    VisibleSequence,
    #[error("information-state perspective or revision is inconsistent")]
    PerspectiveRevision,
}

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
    fn opaque_object_id(&self) -> OpaqueObjectId {
        match self {
            Self::Active {
                opaque_object_id, ..
            }
            | Self::Retired {
                opaque_object_id, ..
            } => *opaque_object_id,
        }
    }

    fn historical_locations(&self) -> &[PlayerKnownLocationFactV1] {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InformationStateDigestInputV2 {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
    pub current_observation: ObservationEnvelope,
    pub next_visible_sequence: VisibleSequence,
    pub retained_knowledge: Vec<PlayerKnownObjectV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerInformationStateV2 {
    pub schema_version: String,
    pub perspective: PlayerId,
    pub state_revision: StateRevision,
    pub current_observation: ObservationEnvelope,
    pub next_visible_sequence: VisibleSequence,
    pub retained_knowledge: Vec<PlayerKnownObjectV1>,
    pub digest: InformationStateDigestV2,
}

impl PlayerInformationStateV2 {
    pub fn digest_input(&self) -> InformationStateDigestInputV2 {
        InformationStateDigestInputV2 {
            schema_version: "information-state-digest-input.v2".into(),
            perspective: self.perspective,
            state_revision: self.state_revision,
            current_observation: self.current_observation.clone(),
            next_visible_sequence: self.next_visible_sequence,
            retained_knowledge: self.retained_knowledge.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != INFORMATION_STATE_SCHEMA_V2 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.current_observation.validate()?;
        if self.current_observation.perspective != self.perspective
            || self.current_observation.state_revision != self.state_revision
        {
            return Err(ObservationValidationError::PerspectiveRevision);
        }
        let mut previous = None;
        for record in &self.retained_knowledge {
            let current = record.opaque_object_id();
            if current.0 == 0 || previous.is_some_and(|previous| previous >= current) {
                return Err(ObservationValidationError::RetainedKnowledge);
            }
            previous = Some(current);
            if !record.provenance_is_valid(self.next_visible_sequence) {
                return Err(ObservationValidationError::VisibleSequence);
            }
            // Only actually observed facts carry visible sequences; the
            // strictly-increasing rule compares observed sequences.
            let observed: Vec<_> = record
                .historical_locations()
                .iter()
                .filter_map(|fact| provenance_sequence(&fact.provenance))
                .collect();
            if observed.windows(2).any(|window| window[0] >= window[1]) {
                return Err(ObservationValidationError::VisibleSequence);
            }
        }
        Ok(())
    }
}

impl PlayerKnownObjectV1 {
    /// Every provenance-bearing public fact must carry an accepted
    /// channel/cause combination and an observed sequence below this
    /// perspective's next unused visible sequence.
    fn provenance_is_valid(&self, next_visible_sequence: VisibleSequence) -> bool {
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

fn provenance_sequence(value: &PlayerKnowledgeProvenanceV1) -> Option<VisibleSequence> {
    match value {
        PlayerKnowledgeProvenanceV1::InitialConfiguration => None,
        PlayerKnowledgeProvenanceV1::Observed { sequence, .. } => Some(*sequence),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservedEventKindV2 {
    ObjectMoved {
        old_object: Option<OpaqueObjectId>,
        new_object: Option<OpaqueObjectId>,
        from: ZoneKind,
        to: ZoneKind,
    },
    ObjectCeasedToExist {
        object: OpaqueObjectId,
    },
    LifeChanged {
        player: PlayerId,
        from: i64,
        to: i64,
    },
    ObjectTapped {
        object: OpaqueObjectId,
        tapped: bool,
    },
    DecisionAvailable {
        actor: PlayerId,
    },
    RandomOutcomeVisible {
        label: String,
        exclusive_upper_bound: u64,
        value: u64,
    },
    PublicOutcome {
        code: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedEventEnvelopeV2 {
    pub schema_version: String,
    pub sequence: VisibleSequence,
    pub state_revision: StateRevision,
    pub event: ObservedEventKindV2,
}

impl ObservedEventEnvelopeV2 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != OBSERVED_EVENT_SCHEMA_V2 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        match &self.event {
            ObservedEventKindV2::RandomOutcomeVisible {
                label,
                exclusive_upper_bound,
                value,
            } if label.is_empty()
                || *exclusive_upper_bound == 0
                || *value >= *exclusive_upper_bound =>
            {
                Err(ObservationValidationError::RandomOutcome)
            }
            ObservedEventKindV2::PublicOutcome { code } if code.is_empty() => {
                Err(ObservationValidationError::EmptyEventText)
            }
            _ => Ok(()),
        }
    }
}

/// Versioned closed codes for typed player submission rejections
/// (ERROR_MODEL, layer B). Wire representation: snake_case strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerSubmissionCodeV1 {
    StaleDecision,
    UnavailableDecision,
    InvalidAnswer,
    InvalidCandidate,
    DuplicateAssignment,
    InvalidCardinality,
    InvalidNumber,
    InvalidOrder,
    EpisodeClosed,
}

/// Versioned closed service-failure code (ERROR_MODEL, layer C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerServiceErrorCodeV1 {
    ServiceUnavailable,
}

/// Versioned submission outcome carried by `PlayerStepV2`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlayerStepSubmissionV1 {
    Accepted,
    Rejected { code: PlayerSubmissionCodeV1 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerStepV2 {
    pub schema_version: String,
    pub information_state: PlayerInformationStateV2,
    pub observed_events: Vec<ObservedEventEnvelopeV2>,
    pub next_decision: Option<PlayerDecisionRequestV2>,
    pub status: EpisodeStatus,
    pub submission: PlayerStepSubmissionV1,
}

impl PlayerStepV2 {
    pub fn validate(&self) -> Result<(), ObservationValidationError> {
        if self.schema_version != PLAYER_STEP_SCHEMA_V2 {
            return Err(ObservationValidationError::SchemaOrCodec);
        }
        self.information_state.validate()?;
        self.status
            .validate()
            .map_err(|_| ObservationValidationError::EpisodeStatus)?;
        for event in &self.observed_events {
            event.validate()?;
            if event.state_revision > self.information_state.state_revision {
                return Err(ObservationValidationError::FutureEvent);
            }
        }
        if let Some(decision) = &self.next_decision {
            decision
                .validate()
                .map_err(|_| ObservationValidationError::Decision)?;
            if decision.actor != self.information_state.perspective
                || decision.state_revision != self.information_state.state_revision
            {
                return Err(ObservationValidationError::Decision);
            }
        }
        if !matches!(self.status, EpisodeStatus::Running) && self.next_decision.is_some() {
            return Err(ObservationValidationError::Decision);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_seven_observed_event_variants_deserialize() {
        let events = [
            concat!(
                r#"{"kind":"object_moved","old_object":"1","new_object":"2","#,
                r#""from":"battlefield","to":"graveyard"}"#,
            ),
            r#"{"kind":"object_ceased_to_exist","object":"1"}"#,
            r#"{"kind":"life_changed","player":"1","from":40,"to":39}"#,
            r#"{"kind":"object_tapped","object":"1","tapped":true}"#,
            r#"{"kind":"decision_available","actor":"1"}"#,
            concat!(
                r#"{"kind":"random_outcome_visible","label":"die","#,
                r#""exclusive_upper_bound":6,"value":2}"#,
            ),
            r#"{"kind":"public_outcome","code":"draw"}"#,
        ];
        for event in events {
            serde_json::from_str::<ObservedEventKind>(event).unwrap();
        }
    }

    #[test]
    fn observed_event_text_fields_are_closed_like_python_and_schema() {
        let empty_label = ObservedEventEnvelope {
            schema_version: OBSERVED_EVENT_SCHEMA.into(),
            sequence: EventSequence(0),
            state_revision: StateRevision(0),
            event: ObservedEventKind::RandomOutcomeVisible {
                label: String::new(),
                exclusive_upper_bound: 2,
                value: 0,
            },
        };
        assert_eq!(
            empty_label.validate(),
            Err(ObservationValidationError::EmptyEventText)
        );
        let empty_code = ObservedEventEnvelope {
            schema_version: OBSERVED_EVENT_SCHEMA.into(),
            sequence: EventSequence(0),
            state_revision: StateRevision(0),
            event: ObservedEventKind::PublicOutcome {
                code: String::new(),
            },
        };
        assert_eq!(
            empty_code.validate(),
            Err(ObservationValidationError::EmptyEventText)
        );
    }

    #[test]
    fn information_state_input_excludes_trusted_fields() {
        let observation = ObservationEnvelope {
            schema_version: OBSERVATION_SCHEMA.into(),
            perspective: PlayerId(1),
            state_revision: StateRevision(0),
            payload_codec: "synthetic-m2-observation.v1".into(),
            payload_base64: "e30=".into(),
            digest: ObservationDigest::from_canonical_bytes(b"{}"),
        };
        let input = InformationStateDigestInputV2 {
            schema_version: "information-state-digest-input.v2".into(),
            perspective: PlayerId(1),
            state_revision: StateRevision(0),
            current_observation: observation,
            next_visible_sequence: VisibleSequence(0),
            retained_knowledge: vec![],
        };
        let json = serde_json::to_string(&input).unwrap();
        for forbidden in [
            "EpisodeStatus",
            "environment_limit_counters",
            "checkpoint_digest",
            "root_seed",
            "GameObjectId",
            "physical_card",
        ] {
            assert!(
                !json.contains(forbidden),
                "unexpected trusted field {forbidden}"
            );
        }
        let object = serde_json::to_value(&input).unwrap();
        assert!(object.get("digest").is_none());
        assert_eq!(input.schema_version, "information-state-digest-input.v2");
    }
}

#[cfg(test)]
mod provenance_tests {
    use super::*;
    use crate::{PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1, OBSERVATION_SCHEMA};

    fn observation() -> ObservationEnvelope {
        ObservationEnvelope {
            schema_version: OBSERVATION_SCHEMA.into(),
            perspective: PlayerId(1),
            state_revision: StateRevision(0),
            payload_codec: "synthetic-m2-observation.v1".into(),
            payload_base64: "e30=".into(),
            digest: ObservationDigest::from_canonical_bytes(b"{}"),
        }
    }

    fn observed(
        channel: PlayerKnowledgeChannelV1,
        sequence: u64,
        cause: PlayerKnowledgeCauseV1,
    ) -> PlayerKnowledgeProvenanceV1 {
        PlayerKnowledgeProvenanceV1::Observed {
            channel,
            sequence: VisibleSequence(sequence),
            cause,
        }
    }

    fn state_with(
        next_visible_sequence: VisibleSequence,
        acquisition: PlayerKnowledgeProvenanceV1,
    ) -> PlayerInformationStateV2 {
        PlayerInformationStateV2 {
            schema_version: INFORMATION_STATE_SCHEMA_V2.into(),
            perspective: PlayerId(1),
            state_revision: StateRevision(0),
            current_observation: observation(),
            next_visible_sequence,
            retained_knowledge: vec![PlayerKnownObjectV1::Active {
                opaque_object_id: OpaqueObjectId(1),
                known_definition: None,
                current_known_location_fact: None,
                historical_locations: Vec::new(),
                acquisition,
            }],
            digest: InformationStateDigestV2::from_canonical_bytes(b"placeholder"),
        }
    }

    #[test]
    fn initial_configuration_is_not_bound_by_the_visible_cursor() {
        let initial = state_with(
            VisibleSequence(0),
            PlayerKnowledgeProvenanceV1::InitialConfiguration,
        );
        let record = &initial.retained_knowledge[0];
        assert!(record.provenance_is_valid(initial.next_visible_sequence));

        let observed_at_zero = state_with(
            VisibleSequence(0),
            observed(
                PlayerKnowledgeChannelV1::Public,
                0,
                PlayerKnowledgeCauseV1::PublicEvent,
            ),
        );
        let record = &observed_at_zero.retained_knowledge[0];
        assert!(!record.provenance_is_valid(observed_at_zero.next_visible_sequence));
    }
    #[test]
    fn future_provenance_sequence_is_rejected() {
        let state = state_with(
            VisibleSequence(1),
            observed(
                PlayerKnowledgeChannelV1::Public,
                999,
                PlayerKnowledgeCauseV1::PublicEvent,
            ),
        );
        assert!(matches!(
            state.validate(),
            Err(ObservationValidationError::VisibleSequence)
        ));
    }

    #[test]
    fn invalid_cause_channel_combination_is_rejected() {
        let state = state_with(
            VisibleSequence(5),
            observed(
                PlayerKnowledgeChannelV1::Public,
                1,
                PlayerKnowledgeCauseV1::PrivateLook,
            ),
        );
        assert!(matches!(
            state.validate(),
            Err(ObservationValidationError::VisibleSequence)
        ));
    }

    #[test]
    fn every_accepted_cause_is_validated_in_context() {
        let cases = [
            (
                PlayerKnowledgeChannelV1::Public,
                1,
                PlayerKnowledgeCauseV1::PublicEvent,
            ),
            (
                PlayerKnowledgeChannelV1::Public,
                2,
                PlayerKnowledgeCauseV1::ExplicitReveal,
            ),
            (
                PlayerKnowledgeChannelV1::Private,
                3,
                PlayerKnowledgeCauseV1::PrivateLook,
            ),
            (
                PlayerKnowledgeChannelV1::Private,
                4,
                PlayerKnowledgeCauseV1::OwnPrivateIdentity,
            ),
        ];
        for (channel, sequence, cause) in cases {
            let state = state_with(VisibleSequence(5), observed(channel, sequence, cause));
            // Digest is intentionally not recomputed here; semantic shape only.
            let result = {
                let mut previous = None;
                for record in &state.retained_knowledge {
                    if !record.provenance_is_valid(state.next_visible_sequence) {
                        previous = Some(Err(ObservationValidationError::VisibleSequence));
                        break;
                    }
                    previous = Some(Ok(()));
                }
                previous.unwrap()
            };
            assert!(result.is_ok(), "accepted combination must validate");
        }
    }
}
