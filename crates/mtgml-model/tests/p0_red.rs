use mtgml_model::{CheckpointDigestV4, FullStateDigestV4};

#[test]
fn p0_v4_digest_wrappers_have_distinct_domains_and_input_schemas() {
    let full = FullStateDigestV4::from_digest_bytes([0x11; 32]);
    let checkpoint = CheckpointDigestV4::from_digest_bytes([0x22; 32]);

    assert_eq!(FullStateDigestV4::DOMAIN, "mtgml.full-state-digest.v4");
    assert_eq!(
        full.as_digest_reference().input_schema_id,
        "full-state-digest-input.v4"
    );
    assert_eq!(CheckpointDigestV4::DOMAIN, "mtgml.checkpoint-digest.v4");
    assert_eq!(checkpoint.as_str().len(), 64);
}
