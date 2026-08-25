//! Ownership: existing inline `mod provenance_tests` block moved verbatim
//! from the former monolithic `lib.rs`; module name and test identities
//! unchanged.

use super::*;
use crate::{PlayerKnowledgeCauseV1, PlayerKnowledgeChannelV1, OBSERVATION_SCHEMA};
use mtgml_model::{
    InformationStateDigestV2, ObservationDigest, OpaqueObjectId, PlayerId, StateRevision,
    VisibleSequence,
};

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
