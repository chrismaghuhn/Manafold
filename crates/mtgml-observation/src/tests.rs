//! Ownership: existing inline `mod tests` block moved verbatim from the
//! former monolithic `lib.rs`; module name and test identities unchanged.

use super::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_model::{
    EpisodeStatus, EventSequence, InformationStateDigest, ObservationDigest, PlayerId,
    StateRevision, VisibleSequence,
};

fn observation(payload: &[u8], digest_payload: &[u8]) -> ObservationEnvelope {
    ObservationEnvelope {
        schema_version: OBSERVATION_SCHEMA.into(),
        perspective: PlayerId(1),
        state_revision: StateRevision(0),
        payload_codec: "synthetic-m2-observation.v1".into(),
        payload_base64: STANDARD.encode(payload),
        digest: ObservationDigest::from_canonical_bytes(digest_payload),
    }
}

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
    let observation = observation(b"{}", b"{}");
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

#[test]
fn observation_digest_binding_accepts_matching_payload_and_rejects_mismatch() {
    assert_eq!(observation(b"{}", b"{}").validate(), Ok(()));
    assert_eq!(
        observation(b"{}", br#"{"x":1}"#).validate(),
        Err(ObservationValidationError::DigestMismatch)
    );
}

#[test]
fn observation_digest_binding_propagates_through_information_and_player_step() {
    let current_observation = observation(b"{}", br#"{"x":1}"#);
    let information_state = InformationStateEnvelope {
        schema_version: INFORMATION_STATE_SCHEMA.into(),
        perspective: PlayerId(1),
        state_revision: StateRevision(0),
        current_observation,
        public_history_length: 0,
        private_history_length: 0,
        digest: InformationStateDigest::from_canonical_bytes(b"information-state"),
    };
    assert_eq!(
        information_state.validate(),
        Err(ObservationValidationError::DigestMismatch)
    );

    let player_step = PlayerStep {
        schema_version: PLAYER_STEP_SCHEMA.into(),
        information_state,
        observed_events: vec![],
        next_decision: None,
        status: EpisodeStatus::Running,
    };
    assert_eq!(
        player_step.validate(),
        Err(ObservationValidationError::DigestMismatch)
    );
}

#[test]
fn observation_digest_known_value_binds_exact_payload_bytes() {
    let digest = ObservationDigest::from_canonical_bytes(b"{}");
    assert_eq!(
        digest.as_str(),
        "90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3"
    );
    assert_ne!(digest, ObservationDigest::from_canonical_bytes(b"e30="));
}
