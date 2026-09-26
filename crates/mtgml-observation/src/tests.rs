//! Ownership: existing inline `mod tests` block moved verbatim from the
//! former monolithic `lib.rs`; module name and test identities unchanged.

use super::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use mtgml_model::{
    EpisodeStatus, EventSequence, InformationStateDigest, ObservationDigest, OpaqueObjectId,
    PlayerId, StateRevision, VisibleSequence,
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
fn magic_combat_observation_v3_rejects_an_object_as_attacker_and_blocker() {
    let mut observation: MagicObservationV3 = serde_json::from_str(include_str!(
        "../../../wire/golden/magic-combat-observation.v3.json"
    ))
    .unwrap();
    observation.combat.as_mut().unwrap().blockers[0].blocker = Some(mtgml_model::OpaqueObjectId(7));
    assert_eq!(
        observation.validate(),
        Err(ObservationValidationError::ObservationPayload)
    );
}

#[test]
fn magic_combat_observation_v4_golden_preserves_life_marks_and_blocked_history() {
    let observation: MagicObservationV4 = serde_json::from_str(include_str!(
        "../../../wire/golden/magic-combat-observation.v4.json"
    ))
    .unwrap();
    observation.validate().unwrap();
    assert_eq!(observation.player_life[1].life, 17);
    assert_eq!(observation.marked_damage[0].amount, "2");
    assert_eq!(
        observation.combat.as_ref().unwrap().blockers[0].status,
        MagicBlockedStatusV4::Blocked
    );
}

#[test]
fn magic_combat_observation_v4_rejects_inconsistent_unblocked_relation() {
    let mut observation: MagicObservationV4 = serde_json::from_str(include_str!(
        "../../../wire/golden/magic-combat-observation.v4.json"
    ))
    .unwrap();
    observation.combat.as_mut().unwrap().blockers[0].status = MagicBlockedStatusV4::Unblocked;
    assert_eq!(
        observation.validate(),
        Err(ObservationValidationError::ObservationPayload)
    );
}

#[test]
fn magic_combat_observation_v4_rejects_multiple_blocked_attackers() {
    let mut observation: MagicObservationV4 = serde_json::from_str(include_str!(
        "../../../wire/golden/magic-combat-observation.v4.json"
    ))
    .unwrap();
    let combat = observation.combat.as_mut().unwrap();
    combat.attackers = vec![OpaqueObjectId(3), OpaqueObjectId(5)];
    combat.blockers = vec![
        MagicCombatBlockerAssignmentV4 {
            attacker: OpaqueObjectId(3),
            status: MagicBlockedStatusV4::Blocked,
            blocker: Some(OpaqueObjectId(4)),
        },
        MagicCombatBlockerAssignmentV4 {
            attacker: OpaqueObjectId(5),
            status: MagicBlockedStatusV4::Blocked,
            blocker: Some(OpaqueObjectId(6)),
        },
    ];
    assert_eq!(
        observation.validate(),
        Err(ObservationValidationError::ObservationPayload)
    );

    let combat = observation.combat.as_mut().unwrap();
    combat.blockers = vec![
        MagicCombatBlockerAssignmentV4 {
            attacker: OpaqueObjectId(3),
            status: MagicBlockedStatusV4::Blocked,
            blocker: None,
        },
        MagicCombatBlockerAssignmentV4 {
            attacker: OpaqueObjectId(5),
            status: MagicBlockedStatusV4::Blocked,
            blocker: None,
        },
    ];
    assert_eq!(
        observation.validate(),
        Err(ObservationValidationError::ObservationPayload)
    );
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
fn observed_event_v2_random_empty_label_uses_empty_text_error() {
    let event = ObservedEventEnvelopeV2 {
        schema_version: OBSERVED_EVENT_SCHEMA_V2.into(),
        sequence: VisibleSequence(0),
        state_revision: StateRevision(0),
        event: ObservedEventKindV2::RandomOutcomeVisible {
            label: String::new(),
            exclusive_upper_bound: 2,
            value: 0,
        },
    };

    assert_eq!(
        event.validate(),
        Err(ObservationValidationError::EmptyEventText)
    );
}

#[test]
fn observed_event_v3_entry_has_one_move_with_explicit_entry_values() {
    let event: ObservedEventEnvelopeV3 = serde_json::from_str(include_str!(
        "../../../schemas/examples/observed-event-v3-entry-back-tapped.json"
    ))
    .unwrap();
    event.validate().unwrap();
    assert!(matches!(
        event.event,
        ObservedEventKindV3::ObjectMoved {
            old_object: Some(OpaqueObjectId(3)),
            new_object: Some(OpaqueObjectId(9)),
            entering_face: Some(ObservedFaceV1::Back),
            tapped: Some(true),
            ..
        }
    ));
}

#[test]
fn observed_event_v3_rejects_move_without_any_visible_identity() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/observed-event-v3-object-moved.json"
    ))
    .unwrap();
    value["event"]["old_object"] = serde_json::Value::Null;
    value["event"]["new_object"] = serde_json::Value::Null;
    let event: ObservedEventEnvelopeV3 = serde_json::from_value(value).unwrap();
    assert_eq!(
        event.validate(),
        Err(ObservationValidationError::ObjectMovedIdentity)
    );
}

#[test]
fn player_step_v3_composes_request_and_rejected_steps_have_no_events() {
    let mut accepted: PlayerStepV3 = serde_json::from_str(include_str!(
        "../../../schemas/examples/player-step-v3-event-next-decision.json"
    ))
    .unwrap();
    // The schema example predates semantic PlayerStep validation. Keep its
    // valid information-state identity and align the request/event to it.
    accepted.next_decision.as_mut().unwrap().state_revision = mtgml_model::StateRevision(0);
    accepted.observed_events[0].sequence = VisibleSequence(4);
    accepted.observed_events[0].state_revision = mtgml_model::StateRevision(0);
    accepted.validate().unwrap();
    assert!(accepted.next_decision.is_some());

    let mut rejected: PlayerStepV3 = serde_json::from_str(include_str!(
        "../../../schemas/examples/player-step-v3-rejected-no-events.json"
    ))
    .unwrap();
    rejected.next_decision = accepted.next_decision.clone();
    rejected.validate().unwrap();
    assert!(rejected.observed_events.is_empty());
}

#[test]
fn basic_land_observation_v1_accepts_closed_public_state_facts_only() {
    let observation: MagicBasicLandObservationV1 = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1-ordered.json"
    ))
    .unwrap();
    observation.validate().unwrap();

    let with_candidates =
        include_str!("../../../schemas/examples/magic-basic-land-observation-v1.json").replace(
            "\"schema_version\":",
            "\"candidates\":[],\n  \"schema_version\":",
        );
    assert!(serde_json::from_str::<MagicBasicLandObservationV1>(&with_candidates).is_err());
}

#[test]
fn v3_successor_examples_deserialize_under_the_rust_contracts() {
    for fixture in [
        include_str!("../../../schemas/examples/observed-event-v3-object-moved.json"),
        include_str!("../../../schemas/examples/observed-event-v3-entry-back-tapped.json"),
        include_str!("../../../schemas/examples/observed-event-v3-mana-pool-changed.json"),
        include_str!("../../../schemas/examples/observed-event-v3-counters-changed.json"),
        include_str!("../../../schemas/examples/observed-event-v3-attachment-changed.json"),
        include_str!("../../../schemas/examples/observed-event-v3-face-changed.json"),
    ] {
        let event: ObservedEventEnvelopeV3 = serde_json::from_str(fixture).unwrap();
        event.validate().unwrap();
    }
    let step: PlayerStepV3 = serde_json::from_str(include_str!(
        "../../../schemas/examples/player-step-v3.json"
    ))
    .unwrap();
    step.validate().unwrap();
    let mut event_and_decision: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/player-step-v3-event-next-decision.json"
    ))
    .unwrap();
    event_and_decision["next_decision"]["state_revision"] = serde_json::json!("0");
    event_and_decision["observed_events"][0]["sequence"] = serde_json::json!("4");
    event_and_decision["observed_events"][0]["state_revision"] = serde_json::json!("0");
    let step: PlayerStepV3 = serde_json::from_value(event_and_decision).unwrap();
    step.validate().unwrap();
    let mut rejected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/player-step-v3-rejected-no-events.json"
    ))
    .unwrap();
    rejected["next_decision"] = serde_json::from_value::<serde_json::Value>(
        serde_json::to_value(&step.next_decision).unwrap(),
    )
    .unwrap();
    let rejected: PlayerStepV3 = serde_json::from_value(rejected).unwrap();
    rejected.validate().unwrap();
    for fixture in [
        include_str!("../../../schemas/examples/magic-basic-land-observation-v1.json"),
        include_str!("../../../schemas/examples/magic-basic-land-observation-v1-ordered.json"),
    ] {
        let observation: MagicBasicLandObservationV1 = serde_json::from_str(fixture).unwrap();
        observation.validate().unwrap();
    }
}

#[test]
fn basic_land_observation_v1_rejects_duplicated_candidate_authority() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../schemas/examples/magic-basic-land-observation-v1.json"
    ))
    .unwrap();
    value["candidates"] = serde_json::json!([]);
    assert!(serde_json::from_value::<MagicBasicLandObservationV1>(value).is_err());
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

include!("tests/batch_f.rs");
