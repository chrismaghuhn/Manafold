use super::*;
use mtgml_model::{
    InformationStateDigestV2, ObservationDigest, PlayerId, StateRevision, VisibleSequence,
};
use mtgml_observation::{InformationStateDigestInputV2, ObservationEnvelope, OBSERVATION_SCHEMA};

pub(crate) fn repository_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn all_golden_wire_fixtures_roundtrip_canonically() {
    verify_golden_fixture_directory(&repository_root().join("wire/golden")).unwrap();
}

#[test]
fn every_shared_negative_fixture_is_rejected_with_the_expected_code() {
    verify_negative_fixture_directory(&repository_root().join("wire/negative")).unwrap();
}

#[test]
fn information_state_digest_v2_known_answer() {
    let input = InformationStateDigestInputV2 {
        schema_version: "information-state-digest-input.v2".into(),
        perspective: PlayerId(1),
        state_revision: StateRevision(0),
        current_observation: ObservationEnvelope {
            schema_version: OBSERVATION_SCHEMA.into(),
            perspective: PlayerId(1),
            state_revision: StateRevision(0),
            payload_codec: "synthetic-m2-observation.v1".into(),
            payload_base64: "e30=".into(),
            digest: ObservationDigest::from_canonical_bytes(b"{}"),
        },
        next_visible_sequence: VisibleSequence(0),
        retained_knowledge: vec![],
    };
    let (bytes, digest) = compute_information_state_digest_v2(&input).unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        r#"{"current_observation":{"digest":"90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3","payload_base64":"e30=","payload_codec":"synthetic-m2-observation.v1","perspective":"1","schema_version":"observation-envelope.v1","state_revision":"0"},"next_visible_sequence":"0","perspective":"1","retained_knowledge":[],"schema_version":"information-state-digest-input.v2","state_revision":"0"}"#
    );
    assert_eq!(
        digest,
        InformationStateDigestV2::parse(
            "a329332227a8e6f4ca95e4e798e5fad3996f344ec924070b71080f44291e2f33",
        )
        .unwrap()
    );
}
