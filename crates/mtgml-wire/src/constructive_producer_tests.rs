use mtgml_decision::{
    CandidateIntent, DecisionAnswerV2, DecisionDomainV2, DecisionResponseV2, DecisionVisibility,
    PlayerDecisionRequestV2, VisibleCandidateV2, DECISION_RESPONSE_V2_SCHEMA,
    PLAYER_DECISION_REQUEST_V2_SCHEMA,
};
use mtgml_model::{
    CandidateIdV1, CardDefinitionId, EpisodeStatus, InformationStateDigestV2, ObservationDigest,
    OpaqueObjectId, PlayerDecisionIdV1, PlayerId, PlayerOutcome, PlayerResult, StateRevision,
    TerminalReason, VisibleSequence, ZoneKind,
};
use mtgml_observation::{
    ObservationEnvelope, ObservedEventEnvelopeV2, ObservedEventKindV2, PlayerInformationStateV2,
    PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1, PlayerKnowledgeInvalidationReasonV1,
    PlayerKnowledgeInvalidationV1, PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1,
    PlayerKnownLocationV1, PlayerKnownObjectV1, PlayerStepSubmissionV1, PlayerStepV2,
    INFORMATION_STATE_SCHEMA_V2, OBSERVATION_SCHEMA, OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
};

use crate::encode_canonical;

const OBSERVATION_DIGEST: &str = "90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3";
const GOLDEN_INFORMATION_STATE_DIGEST_V2: &str =
    "3ca09ef084ebca02f6350db80423db3d8eeddb1d42fe7b2868acf2dbb752ebfd";

fn golden_fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        super::tests::repository_root()
            .join("wire/golden")
            .join(name),
    )
    .expect("golden fixture is readable")
}

fn observed_provenance(
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

fn constructed_information_state_v2() -> PlayerInformationStateV2 {
    PlayerInformationStateV2 {
        schema_version: INFORMATION_STATE_SCHEMA_V2.to_owned(),
        perspective: PlayerId(1),
        state_revision: StateRevision(0),
        current_observation: ObservationEnvelope {
            schema_version: OBSERVATION_SCHEMA.to_owned(),
            perspective: PlayerId(1),
            state_revision: StateRevision(0),
            payload_codec: "synthetic-m2-observation.v1".to_owned(),
            payload_base64: "e30=".to_owned(),
            digest: ObservationDigest::parse(OBSERVATION_DIGEST).expect("observation digest"),
        },
        next_visible_sequence: VisibleSequence(5),
        retained_knowledge: vec![
            PlayerKnownObjectV1::Active {
                opaque_object_id: OpaqueObjectId(3),
                known_definition: Some(CardDefinitionId(42)),
                current_known_location_fact: Some(PlayerKnownLocationFactV1 {
                    location: PlayerKnownLocationV1 {
                        zone: ZoneKind::Exile,
                        player: Some(PlayerId(2)),
                    },
                    provenance: observed_provenance(
                        PlayerKnowledgeChannelV1::Public,
                        4,
                        PlayerKnowledgeCauseV1::ExplicitReveal,
                    ),
                }),
                historical_locations: vec![PlayerKnownLocationFactV1 {
                    location: PlayerKnownLocationV1 {
                        zone: ZoneKind::Hand,
                        player: None,
                    },
                    provenance: observed_provenance(
                        PlayerKnowledgeChannelV1::Private,
                        3,
                        PlayerKnowledgeCauseV1::OwnPrivateIdentity,
                    ),
                }],
                acquisition: observed_provenance(
                    PlayerKnowledgeChannelV1::Private,
                    1,
                    PlayerKnowledgeCauseV1::PrivateLook,
                ),
            },
            PlayerKnownObjectV1::Retired {
                opaque_object_id: OpaqueObjectId(7),
                known_definition: None,
                last_known_location_fact: Some(PlayerKnownLocationFactV1 {
                    location: PlayerKnownLocationV1 {
                        zone: ZoneKind::Battlefield,
                        player: None,
                    },
                    provenance: PlayerKnowledgeProvenanceV1::InitialConfiguration,
                }),
                historical_locations: Vec::new(),
                acquisition: observed_provenance(
                    PlayerKnowledgeChannelV1::Public,
                    2,
                    PlayerKnowledgeCauseV1::PublicEvent,
                ),
                invalidation: PlayerKnowledgeInvalidationV1 {
                    provenance: observed_provenance(
                        PlayerKnowledgeChannelV1::Public,
                        4,
                        PlayerKnowledgeCauseV1::ExplicitReveal,
                    ),
                    reason: PlayerKnowledgeInvalidationReasonV1::Shuffle,
                },
            },
        ],
        digest: InformationStateDigestV2::parse(GOLDEN_INFORMATION_STATE_DIGEST_V2)
            .expect("golden information-state digest"),
    }
}

#[test]
fn information_state_envelope_v2_constructs_the_golden_bytes() {
    assert_eq!(
        encode_canonical(&constructed_information_state_v2()).unwrap(),
        golden_fixture("information-state-envelope.v2.json")
    );
}

fn constructed_choose_one_request_v2() -> PlayerDecisionRequestV2 {
    PlayerDecisionRequestV2 {
        schema_version: PLAYER_DECISION_REQUEST_V2_SCHEMA.to_owned(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(0),
        actor: PlayerId(1),
        visibility: DecisionVisibility::Public,
        decision: DecisionDomainV2::ChooseOne,
        candidates: vec![
            VisibleCandidateV2 {
                candidate_id: CandidateIdV1(0),
                intent: CandidateIntent::ChooseBoolean { value: false },
            },
            VisibleCandidateV2 {
                candidate_id: CandidateIdV1(1),
                intent: CandidateIntent::ChooseBoolean { value: true },
            },
        ],
    }
}

#[test]
fn player_decision_request_v2_constructs_the_golden_bytes() {
    assert_eq!(
        encode_canonical(&constructed_choose_one_request_v2()).unwrap(),
        golden_fixture("player-decision-request.v2.json")
    );
}

#[test]
fn observation_envelope_v1_constructs_the_golden_bytes() {
    let value = ObservationEnvelope {
        schema_version: OBSERVATION_SCHEMA.to_owned(),
        perspective: PlayerId(1),
        state_revision: StateRevision(0),
        payload_codec: "synthetic-json.v1".to_owned(),
        payload_base64: "e30=".to_owned(),
        digest: ObservationDigest::parse(OBSERVATION_DIGEST).expect("observation digest"),
    };
    assert_eq!(
        encode_canonical(&value).unwrap(),
        golden_fixture("observation-envelope.v1.json")
    );
}

#[test]
fn observed_event_envelope_v2_object_moved_constructs_the_golden_bytes() {
    let value = ObservedEventEnvelopeV2 {
        schema_version: OBSERVED_EVENT_SCHEMA_V2.to_owned(),
        sequence: VisibleSequence(1),
        state_revision: StateRevision(0),
        event: ObservedEventKindV2::ObjectMoved {
            old_object: Some(OpaqueObjectId(3)),
            new_object: Some(OpaqueObjectId(11)),
            from: ZoneKind::Hand,
            to: ZoneKind::Battlefield,
        },
    };
    assert_eq!(
        encode_canonical(&value).unwrap(),
        golden_fixture("observed-event-v2-object-moved.json")
    );
}

#[test]
fn decision_response_v2_select_one_constructs_the_golden_bytes() {
    let value = DecisionResponseV2 {
        schema_version: DECISION_RESPONSE_V2_SCHEMA.to_owned(),
        player_decision_id: PlayerDecisionIdV1(1),
        state_revision: StateRevision(0),
        answer: DecisionAnswerV2::SelectOne {
            candidate_id: CandidateIdV1(1),
        },
    };
    assert_eq!(
        encode_canonical(&value).unwrap(),
        golden_fixture("decision-response.v2-select-one.json")
    );
}

#[test]
fn player_step_v2_constructs_the_golden_bytes() {
    let value = PlayerStepV2 {
        schema_version: PLAYER_STEP_SCHEMA_V2.to_owned(),
        information_state: constructed_information_state_v2(),
        observed_events: Vec::new(),
        next_decision: Some(constructed_choose_one_request_v2()),
        status: EpisodeStatus::Running,
        submission: PlayerStepSubmissionV1::Accepted,
    };
    assert_eq!(
        encode_canonical(&value).unwrap(),
        golden_fixture("player-step.v2.json")
    );
}

#[test]
fn episode_status_terminal_concession_constructs_the_golden_bytes() {
    let value = EpisodeStatus::Terminal {
        reason: TerminalReason::Concession,
        players: vec![
            PlayerOutcome {
                player: PlayerId(1),
                result: PlayerResult::Win,
            },
            PlayerOutcome {
                player: PlayerId(2),
                result: PlayerResult::Loss,
            },
        ],
    };
    assert_eq!(
        encode_canonical(&value).unwrap(),
        golden_fixture("episode-status-terminal-concession.v1.json")
    );
}
