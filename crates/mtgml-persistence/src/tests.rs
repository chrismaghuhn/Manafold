use super::{cbor, checkpoint_digest, envelope, PersistenceDecodeErrorV1};
use mtgml_model::{CheckpointCodecIdentity, EnvironmentLimitCounters, EpisodeStatus};

#[test]
fn canonical_cbor_v1_complete_profile_matrix() {
    let values = [
        cbor::Value::Null,
        cbor::Value::Bool(false),
        cbor::Value::Bool(true),
        cbor::Value::Unsigned(0),
        cbor::Value::Unsigned(23),
        cbor::Value::Unsigned(24),
        cbor::Value::Unsigned(u64::MAX),
        cbor::Value::Signed(-1),
        cbor::Value::Signed(i64::MIN),
        cbor::Value::Bytes(vec![0, 1, 2]),
        cbor::Value::Text("hé".to_owned()),
        cbor::Value::Array(vec![cbor::Value::Unsigned(1), cbor::Value::Null]),
    ];
    for value in values {
        let encoded = cbor::encode_canonical(&value).unwrap();
        assert_eq!(cbor::decode_canonical(&encoded).unwrap(), value);
    }

    let forbidden = [
        vec![0x18, 0x17],                         // non-shortest unsigned
        vec![0x38, 0x00],                         // non-shortest negative
        vec![0x9f, 0x01, 0xff],                   // indefinite array
        vec![0xa0],                               // map
        vec![0xc0],                               // tag
        vec![0xfb, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0], // float
        vec![0xf7],                               // undefined
        vec![0x01, 0x00],                         // trailing value
    ];
    for bytes in forbidden {
        assert!(
            cbor::decode_canonical(&bytes).is_err(),
            "accepted {bytes:02x?}"
        );
    }
}

#[test]
fn digest_envelope_v1_known_answer_matrix() {
    let payload = cbor::encode_canonical(&cbor::Value::Array(vec![
        cbor::Value::Text("input.v1".to_owned()),
        cbor::Value::Unsigned(7),
    ]))
    .unwrap();
    let envelope =
        envelope::encode_envelope("mtgml.test-domain.v1", "test-input.v1", &payload).unwrap();
    let (reference, decoded_payload) = envelope::decode_envelope(&envelope).unwrap();
    assert_eq!(decoded_payload, payload);
    assert_eq!(reference.semantic_domain, "mtgml.test-domain.v1");
    assert_eq!(reference.input_schema_id, "test-input.v1");
    assert_eq!(reference.digest_bytes, envelope::hash_envelope(&envelope));
    assert_eq!(
        hex(&reference.digest_bytes),
        "b1188a072cbe39da6a521f51a3d5790fe1f0e4c46c25b5e90f62bf5ee4a7f6ad"
    );
    assert_eq!(reference.envelope_version, envelope::DIGEST_ENVELOPE_ID);
}

#[test]
fn checkpoint_digest_v3_known_answer() {
    let full_state = mtgml_model::DigestReferenceV1 {
        envelope_version: envelope::DIGEST_ENVELOPE_ID.to_owned(),
        algorithm_id: envelope::SHA256_ID.to_owned(),
        semantic_domain: "mtgml.full-state-digest.v3".to_owned(),
        payload_codec_id: envelope::CANONICAL_CBOR_ID.to_owned(),
        input_schema_id: "full-state-digest-input.v3".to_owned(),
        digest_bytes: [7; 32],
    };
    let counters = EnvironmentLimitCounters::default();
    let codec = CheckpointCodecIdentity {
        codec_id: envelope::CANONICAL_CBOR_ID.to_owned(),
        semantic_version: "v3".to_owned(),
    };
    let reference = checkpoint_digest::calculate_checkpoint_digest_v3(
        &full_state,
        &EpisodeStatus::Running,
        &counters,
        &codec,
    )
    .unwrap();
    assert_eq!(
        mtgml_model::CheckpointDigestV3::DOMAIN,
        "mtgml.checkpoint-digest.v3"
    );
    assert_eq!(reference.raw_bytes().len(), 32);
    assert_eq!(
        hex(&reference.raw_bytes()),
        "b0cf94e1f49fb58feb6ebc07d88b2a7e226be78c1ca92ee7b9772d4f51290f6c"
    );
}

#[test]
fn error_categories_are_closed_and_stable() {
    assert_eq!(
        PersistenceDecodeErrorV1::TrailingData.as_str(),
        "trailing_data"
    );
    assert_eq!(
        PersistenceDecodeErrorV1::UnsupportedHistoricalVersion.as_str(),
        "unsupported_historical_version"
    );
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut encoded, byte| {
            write!(encoded, "{byte:02x}").expect("writing hexadecimal bytes to String cannot fail");
            encoded
        },
    )
}
