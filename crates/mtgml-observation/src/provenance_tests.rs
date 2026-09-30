//! Ownership: existing inline `mod provenance_tests` block moved verbatim
//! from the former monolithic `lib.rs`; module name and test identities
//! unchanged.

use super::*;
use crate::{
    PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1, PlayerKnowledgeInvalidationReasonV1,
    PlayerKnowledgeInvalidationV1, MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1,
    OBSERVATION_SCHEMA_V2,
};
use mtgml_model::{
    InformationStateDigestV3, ObservationDigest, OpaqueObjectId, PlayerId, VisibleSequence,
};

fn observation(view_sequence: VisibleSequence) -> ObservationEnvelopeV2 {
    ObservationEnvelopeV2 {
        schema_version: OBSERVATION_SCHEMA_V2.into(),
        perspective: PlayerId(1),
        view_sequence,
        payload_codec: MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1.into(),
        payload_base64: "e30=".into(),
        digest: ObservationDigest::from_canonical_bytes(b"{}"),
    }
}

fn state_with_knowledge(
    next_visible_sequence: VisibleSequence,
    retained_knowledge: Vec<PlayerKnownObjectV1>,
) -> PlayerInformationStateV3 {
    PlayerInformationStateV3 {
        schema_version: INFORMATION_STATE_SCHEMA_V3.into(),
        perspective: PlayerId(1),
        current_observation: observation(next_visible_sequence),
        next_visible_sequence,
        retained_knowledge,
        digest: InformationStateDigestV3::from_canonical_bytes(b"placeholder"),
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
) -> PlayerInformationStateV3 {
    state_with_knowledge(
        next_visible_sequence,
        vec![PlayerKnownObjectV1::Active {
            opaque_object_id: OpaqueObjectId(1),
            known_definition: None,
            current_known_location_fact: None,
            historical_locations: Vec::new(),
            acquisition,
        }],
    )
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

#[test]
fn invalidation_cannot_come_from_the_initial_configuration() {
    let retired = |provenance| {
        state_with_knowledge(
            VisibleSequence(5),
            vec![PlayerKnownObjectV1::Retired {
                opaque_object_id: OpaqueObjectId(1),
                known_definition: None,
                last_known_location_fact: None,
                historical_locations: Vec::new(),
                acquisition: observed(
                    PlayerKnowledgeChannelV1::Public,
                    2,
                    PlayerKnowledgeCauseV1::PublicEvent,
                ),
                invalidation: PlayerKnowledgeInvalidationV1 {
                    provenance,
                    reason: PlayerKnowledgeInvalidationReasonV1::Shuffle,
                },
            }],
        )
    };
    assert_eq!(
        retired(observed(
            PlayerKnowledgeChannelV1::Public,
            3,
            PlayerKnowledgeCauseV1::PublicEvent
        ))
        .validate(),
        Ok(())
    );
    assert_eq!(
        retired(PlayerKnowledgeProvenanceV1::InitialConfiguration).validate(),
        Err(ObservationValidationError::VisibleSequence)
    );
}
