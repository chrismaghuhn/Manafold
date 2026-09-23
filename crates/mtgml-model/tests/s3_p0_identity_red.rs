//! S3.P0 Task 1 RED contract for the new state/checkpoint digest identities.
//!
//! Expected RED at the planning head: the intended V5/V6 model identities do
//! not exist yet. This file becomes a permanent contract test after Task 2.

use mtgml_model::{CheckpointDigestV6, FullStateDigestV5};

#[test]
fn v5_and_v6_digest_domains_are_distinct_versioned_model_values() {
    assert_eq!(FullStateDigestV5::DOMAIN, "mtgml.full-state-digest.v5");
    assert_eq!(CheckpointDigestV6::DOMAIN, "mtgml.checkpoint-digest.v6");

    let full = FullStateDigestV5::from_digest_bytes([0x51; 32]);
    let reference = full.as_digest_reference();
    assert_eq!(reference.semantic_domain, "mtgml.full-state-digest.v5");
    assert_eq!(reference.input_schema_id, "full-state-digest-input.v5");
}
