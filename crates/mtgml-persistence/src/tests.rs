use super::{cbor, checkpoint_digest, envelope, PersistenceDecodeErrorV1};
use mtgml_model::{CheckpointCodecIdentity, EnvironmentLimitCounters, EpisodeStatus};

#[test]
fn canonical_cbor_v1_complete_profile_matrix() {
    // Every accepted primitive at its width boundaries.
    let accepted: Vec<(cbor::Value, Vec<u8>)> = [
        (cbor::Value::Null, vec![0xf6]),
        (cbor::Value::Bool(false), vec![0xf4]),
        (cbor::Value::Bool(true), vec![0xf5]),
        (cbor::Value::Unsigned(0), vec![0x00]),
        (cbor::Value::Unsigned(23), vec![0x17]),
        (cbor::Value::Unsigned(24), vec![0x18, 0x18]),
        (
            cbor::Value::Unsigned(u64::MAX),
            vec![0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
        (cbor::Value::Signed(-1), vec![0x20]),
        (
            cbor::Value::Signed(i64::MIN),
            vec![0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        ),
        (cbor::Value::Bytes(vec![]), vec![0x40]),
        (
            cbor::Value::Bytes(vec![0xab; 25]),
            std::iter::once(0x58)
                .chain(std::iter::once(25))
                .chain(std::iter::repeat_n(0xab, 25))
                .collect(),
        ),
        (cbor::Value::Text(String::new()), vec![0x60]),
        (
            cbor::Value::Text("\u{e9}\u{20ac}".to_owned()),
            vec![0x65, 0xc3, 0xa9, 0xe2, 0x82, 0xac],
        ),
        (cbor::Value::Array(vec![]), vec![0x80]),
    ]
    .into_iter()
    .collect();
    for (value, expected) in &accepted {
        let encoded = cbor::encode_canonical(value).unwrap();
        assert_eq!(&encoded, expected, "canonical bytes drifted for {value:?}");
        assert_eq!(cbor::decode_canonical(&encoded).unwrap(), *value);
    }

    // Every forbidden form with its exact ADR-0040 category.
    let forbidden: Vec<(Vec<u8>, PersistenceDecodeErrorV1)> = vec![
        (vec![0xa0], PersistenceDecodeErrorV1::DisallowedCborForm),
        (vec![0xc0], PersistenceDecodeErrorV1::DisallowedCborForm),
        (
            vec![0xfb, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        (
            vec![0xf9, 0x3c, 0x00],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        (
            vec![0x9f, 0x01, 0xff],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        (
            vec![0x7f, 0x62, 0x68, 0x69, 0xff],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        (vec![0xf7], PersistenceDecodeErrorV1::DisallowedCborForm),
        (
            vec![0xf8, 0x20],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        (vec![0x1c], PersistenceDecodeErrorV1::DisallowedCborForm),
        (vec![0x1d], PersistenceDecodeErrorV1::DisallowedCborForm),
        (vec![0x1e], PersistenceDecodeErrorV1::DisallowedCborForm),
        (vec![0x1f], PersistenceDecodeErrorV1::DisallowedCborForm),
        // Non-shortest integer encodings.
        (
            vec![0x18, 0x17],
            PersistenceDecodeErrorV1::NoncanonicalPrimitive,
        ),
        (
            vec![0x19, 0x00, 0xff],
            PersistenceDecodeErrorV1::NoncanonicalPrimitive,
        ),
        (
            vec![0x1a, 0x00, 0x00, 0xff, 0xff],
            PersistenceDecodeErrorV1::NoncanonicalPrimitive,
        ),
        (
            vec![0x1b, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff],
            PersistenceDecodeErrorV1::NoncanonicalPrimitive,
        ),
        (
            vec![0x38, 0x00],
            PersistenceDecodeErrorV1::NoncanonicalPrimitive,
        ),
        // Multi-defect precedence: disallowed form precedes noncanonical
        // primitive (rank 9 < rank 10).
        (
            vec![0xb8, 0x00],
            PersistenceDecodeErrorV1::DisallowedCborForm,
        ),
        // Malformed UTF-8.
        (vec![0x61, 0xff], PersistenceDecodeErrorV1::InvalidUtf8),
        // Trailing top-level data.
        (vec![0x01, 0x00], PersistenceDecodeErrorV1::TrailingData),
        // Truncated inputs report the framing category.
        (vec![], PersistenceDecodeErrorV1::EnvelopeLength),
        (vec![0x19, 0x01], PersistenceDecodeErrorV1::EnvelopeLength),
        (
            vec![0x45, 0x61, 0x62],
            PersistenceDecodeErrorV1::EnvelopeLength,
        ),
        // Signed range bound.
        (
            [0x3b]
                .into_iter()
                .chain((i64::MAX as u64 + 1).to_be_bytes())
                .collect(),
            PersistenceDecodeErrorV1::ValueOutOfRange,
        ),
        // Declared over-limit lengths are rejected before allocation.
        (
            [0x7a]
                .into_iter()
                .chain((1024u32 * 1024 + 1).to_be_bytes())
                .collect(),
            PersistenceDecodeErrorV1::StringTooLarge,
        ),
        (
            [0x5a]
                .into_iter()
                .chain(((64u32 * 1024 * 1024) + 1).to_be_bytes())
                .collect(),
            PersistenceDecodeErrorV1::PayloadTooLarge,
        ),
        (
            [0x9a]
                .into_iter()
                .chain(((1024u32 * 1024) + 1).to_be_bytes())
                .collect(),
            PersistenceDecodeErrorV1::ArrayTooLarge,
        ),
    ];
    for (bytes, expected) in forbidden {
        assert_eq!(
            cbor::decode_canonical(&bytes).unwrap_err(),
            expected,
            "input {:02x?}",
            bytes
        );
    }

    // Exactly 64 nested arrays decode; 65 exceed the declared depth budget.
    let boundary = |depth: usize| -> Vec<u8> {
        std::iter::repeat_n(0x81u8, depth)
            .chain(std::iter::once(0x00))
            .collect()
    };
    assert!(cbor::decode_canonical(&boundary(64)).is_ok());
    assert_eq!(
        cbor::decode_canonical(&boundary(65)).unwrap_err(),
        PersistenceDecodeErrorV1::DepthExceeded
    );

    // The item counter bounds the total decoded data items: five sibling
    // arrays at the maximum element count exceed MAX_ITEMS while every
    // individual array stays within its own declared limit.
    let mut item_bomb = vec![0x85];
    for _ in 0..5 {
        item_bomb.extend_from_slice(&[0x9a, 0x00, 0x10, 0x00, 0x00]);
        item_bomb.extend(std::iter::repeat_n(0xf6, 1024 * 1024));
    }
    assert!(item_bomb.len() <= cbor::MAX_PAYLOAD_BYTES);
    assert!(matches!(
        cbor::decode_canonical(&item_bomb),
        Err(PersistenceDecodeErrorV1::ItemLimitExceeded)
    ));
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

    let prefix_len = envelope::DIGEST_ENVELOPE_ID.len() + 1;
    let fields = parse_frames(&envelope);
    assert_eq!(fields.len(), 5);
    let assemble = |fields: &[&[u8]]| -> Vec<u8> {
        let mut output = envelope[..prefix_len].to_vec();
        for field in fields {
            output.extend(mtgml_frame(field));
        }
        output
    };

    // Identity defects (ADR-0040 rank 2).
    let identity_cases: Vec<(Vec<u8>, &str)> = vec![
        (b"not-an-envelope".to_vec(), "wrong prefix"),
        (envelope[..10].to_vec(), "truncated below prefix"),
        (
            assemble(&[
                &vec![b'A'; 256],
                &fields[1],
                &fields[2],
                &fields[3],
                &fields[4],
            ]),
            "identifier above the 255-byte bound",
        ),
        (
            assemble(&[b"sha-512", &fields[1], &fields[2], &fields[3], &fields[4]]),
            "unsupported algorithm",
        ),
        (
            assemble(&[
                &fields[0],
                &fields[1],
                b"mtgml.canonical-json.v1",
                &fields[3],
                &fields[4],
            ]),
            "unsupported payload codec",
        ),
        (
            assemble(&[&fields[0], &[0x80; 20], &fields[2], &fields[3], &fields[4]]),
            "non-ASCII identifier",
        ),
        // Cross-frame precedence: an early identity defect must beat any
        // later payload or framing defect (rank 2 < 3 < 4).
        (
            [
                assemble(&[b"sha-512", &fields[1], &fields[2], &fields[3]]),
                ((u64::from(cbor::MAX_PAYLOAD_BYTES as u32)) + 1)
                    .to_be_bytes()
                    .to_vec(),
            ]
            .concat(),
            "wrong algorithm + over-limit payload declaration",
        ),
        (
            [
                envelope[..prefix_len].to_vec(),
                mtgml_frame(b"sha-256"),
                mtgml_frame(&fields[1]),
                mtgml_frame(b"mtgml.canonical-json.v1"),
                mtgml_frame(&fields[3]),
                128_u64.to_be_bytes().to_vec(),
            ]
            .concat(),
            "wrong codec + truncated payload frame",
        ),
        (
            [
                envelope[..prefix_len].to_vec(),
                mtgml_frame(b"sha-256"),
                mtgml_frame(&[0x80, 0x80, 0x80, 0x80, b't', b'e', b's', b't']),
                vec![0x00, 0x00, 0x00],
            ]
            .concat(),
            "non-ASCII domain + truncated tail",
        ),
    ];
    for (input, why) in identity_cases {
        assert_eq!(
            envelope::decode_envelope(&input).unwrap_err(),
            PersistenceDecodeErrorV1::EnvelopeIdentity,
            "{why}"
        );
    }

    // Length/framing defects (rank 3) only surface on identity-valid input:
    // any truncated input that cannot carry the full prefix is an identity
    // defect first.
    let truncated_frame = [envelope[..prefix_len].to_vec(), vec![0; 4]].concat();
    assert_eq!(
        envelope::decode_envelope(&truncated_frame).unwrap_err(),
        PersistenceDecodeErrorV1::EnvelopeLength
    );
    let trailing = [envelope.as_slice(), &[0]].concat();
    assert_eq!(
        envelope::decode_envelope(&trailing).unwrap_err(),
        PersistenceDecodeErrorV1::EnvelopeLength
    );

    // Resource bounds before allocation.
    let mut over_limit_payload_decl = assemble(&[&fields[0], &fields[1], &fields[2], &fields[3]]);
    over_limit_payload_decl.extend((u64::from(cbor::MAX_PAYLOAD_BYTES as u32) + 1).to_be_bytes());
    assert_eq!(
        envelope::decode_envelope(&over_limit_payload_decl).unwrap_err(),
        PersistenceDecodeErrorV1::PayloadTooLarge
    );
}

fn parse_frames(envelope: &[u8]) -> Vec<Vec<u8>> {
    let mut offset = envelope::DIGEST_ENVELOPE_ID.len() + 1;
    let mut fields = Vec::new();
    while offset < envelope.len() {
        let length = usize::try_from(u64::from_be_bytes(
            envelope[offset..offset + 8].try_into().unwrap(),
        ))
        .unwrap();
        fields.push(envelope[offset + 8..offset + 8 + length].to_vec());
        offset += 8 + length;
    }
    fields
}

fn mtgml_frame(value: &[u8]) -> Vec<u8> {
    let mut output = (value.len() as u64).to_be_bytes().to_vec();
    output.extend_from_slice(value);
    output
}

/// The shared mechanical negative corpus is Rust-authoritative evidence:
/// every committed fixture must produce its manifest-declared category from
/// the Rust decoder. Python parity runs against the same corpus.
#[test]
fn persisted_negative_fixture_manifest_matches_rust_categories() {
    use std::collections::BTreeMap;

    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        #[serde(rename = "schema_version")]
        _schema_version: String,
        fixtures: Vec<Fixture>,
    }
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fixture {
        contract: String,
        expected_error_code: String,
        path: String,
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let raw = std::fs::read(root.join("persistence/negative/manifest.json")).unwrap();
    let manifest: Manifest = serde_json::from_slice(&raw).unwrap();
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    assert!(manifest.fixtures.len() >= 20, "corpus regressed");
    for fixture in &manifest.fixtures {
        let bytes = std::fs::read(root.join("persistence/negative").join(&fixture.path)).unwrap();
        let actual = match fixture.contract.as_str() {
            "canonical-cbor.v1" => cbor::decode_canonical(&bytes).map(|_| ()).unwrap_err(),
            "digest-envelope.v1" => envelope::decode_envelope(&bytes).map(|_| ()).unwrap_err(),
            other => panic!("unknown fixture contract {other}"),
        };
        assert_eq!(
            actual.as_str(),
            fixture.expected_error_code,
            "fixture {} drifted",
            fixture.path
        );
        assert!(seen
            .insert(fixture.path.clone(), fixture.contract.clone())
            .is_none());
    }
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
