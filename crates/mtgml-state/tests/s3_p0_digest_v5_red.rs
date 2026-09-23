//! S3.P0 Task 1 RED contract for the typed V5 state digest.
//!
//! The continuation fixture's semantic payload includes the immutable
//! round-start revision, complete selected SBA action set and causes, APNAP
//! owners, next owner index, and every completed owner's exact permutation.
//! Its canonical payload tag is `magic_sba_graveyard_order_v1`.

use mtgml_model::FullStateDigestV5;
use mtgml_model::PlayerId;
use mtgml_random::RootSeed256;
use mtgml_state::{
    construct_synthetic_engine_state, ContinuationPayloadV2, SyntheticResetInputs,
    SyntheticV4Setup, FULL_STATE_DIGEST_INPUT_SCHEMA_V5,
};

fn synthetic_state() -> mtgml_state::EngineState {
    construct_synthetic_engine_state(SyntheticResetInputs {
        players: [PlayerId(1), PlayerId(2)],
        root_seed: RootSeed256::from_lower_hex(&"11".repeat(32)).unwrap(),
        setup: SyntheticV4Setup::m2_compatibility(),
    })
    .unwrap()
}

#[test]
fn engine_state_digest_uses_the_typed_v5_identity_and_schema() {
    let digest: FullStateDigestV5 = synthetic_state().digest().unwrap();
    assert_eq!(
        digest.as_digest_reference().semantic_domain,
        FullStateDigestV5::DOMAIN
    );
    assert_eq!(
        FULL_STATE_DIGEST_INPUT_SCHEMA_V5,
        "full-state-digest-input.v5"
    );
}

#[test]
fn v5_continuation_fixture_admits_the_magic_sba_order_variant() {
    let fixture = serde_json::json!({
        "kind": "magic_sba_graveyard_order_v1",
        "round_start_revision": "7",
        "selected_sba_actions": [
            {"object": "10", "causes": ["lethal_damage"]},
            {"object": "11", "causes": ["zero_toughness"]}
        ],
        "apnap_owners": ["1", "2"],
        "next_owner_index": 1,
        "completed_owner_orders": [
            {"owner": "1", "top_to_bottom": ["10", "11"]}
        ]
    });
    let payload: ContinuationPayloadV2 = serde_json::from_value(fixture).unwrap();

    assert_eq!(payload.stage_index(), 1);
}
