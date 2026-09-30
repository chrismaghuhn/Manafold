use mtgml_model::{
    CardDefinitionId, EpisodeStatus, InformationStateDigestV3, ObservationDigest, OpaqueObjectId,
    PlayerId, PlayerOutcome, PlayerResult, TerminalReason, VisibleSequence, ZoneKind,
};
use mtgml_observation::{
    ObservationEnvelopeV2, PlayerInformationStateV3, PlayerKnowledgeCauseV1,
    PlayerKnowledgeChannelV1, PlayerKnowledgeInvalidationReasonV1, PlayerKnowledgeInvalidationV1,
    PlayerKnowledgeProvenanceV1, PlayerKnownLocationFactV1, PlayerKnownLocationV1,
    PlayerKnownObjectV1, INFORMATION_STATE_SCHEMA_V3, MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1,
    OBSERVATION_SCHEMA_V2,
};

use crate::{compute_information_state_digest_v3, decode_canonical, encode_canonical};

const OBSERVATION_DIGEST: &str = "90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3";

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

fn observation_envelope_v2() -> ObservationEnvelopeV2 {
    ObservationEnvelopeV2 {
        schema_version: OBSERVATION_SCHEMA_V2.to_owned(),
        perspective: PlayerId(1),
        view_sequence: VisibleSequence(5),
        payload_codec: MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1.to_owned(),
        payload_base64: "e30=".to_owned(),
        digest: ObservationDigest::parse(OBSERVATION_DIGEST).expect("observation digest"),
    }
}

/// Retained knowledge with all four observed causes, active and retired: the
/// richest information state, which no wire golden carries.
fn rich_information_state_v3() -> PlayerInformationStateV3 {
    let mut state = PlayerInformationStateV3 {
        schema_version: INFORMATION_STATE_SCHEMA_V3.to_owned(),
        perspective: PlayerId(1),
        current_observation: observation_envelope_v2(),
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
        digest: InformationStateDigestV3::from_canonical_bytes(b"placeholder"),
    };
    let (_, digest) = compute_information_state_digest_v3(&state.digest_input()).unwrap();
    state.digest = digest;
    state
}

#[test]
fn information_state_v3_with_rich_retained_knowledge_round_trips() {
    let state = rich_information_state_v3();
    state.validate().unwrap();
    let bytes = encode_canonical(&state).unwrap();
    // The canonical decoder recomputes and checks the digest.
    let decoded: PlayerInformationStateV3 = decode_canonical(&bytes).unwrap();
    assert_eq!(decoded, state);
    assert_eq!(encode_canonical(&decoded).unwrap(), bytes);
}

#[test]
fn observation_envelope_v2_constructs_the_golden_bytes() {
    assert_eq!(
        encode_canonical(&observation_envelope_v2()).unwrap(),
        golden_fixture("observation-envelope.v2.json")
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
