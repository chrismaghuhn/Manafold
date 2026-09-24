use crate::semantic_execution_generated::{
    magic_execution_profile, magic_s3_a_ordered_sba_0_1_0_semantic_contract_id,
    magic_s3_b_basic_priority_0_1_0_semantic_contract_id,
    magic_s3_c_draw_interaction_0_1_0_semantic_contract_id,
    magic_turn_structure_0_1_0_semantic_contract_id,
};
use mtgml_model::SemanticContractIdV1;

fn assert_profile(
    id: SemanticContractIdV1,
    expected_id: &str,
    expected_capabilities: (bool, bool, bool, bool),
) {
    assert_eq!(id.as_str(), expected_id);
    let profile = magic_execution_profile(id).expect("exact catalog identity is admitted");
    assert_eq!(
        (
            profile.allows_turn_structure_0_1_0(),
            profile.allows_state_based_actions_combat_0_1_0(),
            profile.allows_basic_priority_0_1_0(),
            profile.allows_draw_card_0_1_0(),
        ),
        expected_capabilities,
    );
}

#[test]
fn recognized_contracts_keep_their_exact_semantic_capability_closures() {
    assert_profile(
        magic_turn_structure_0_1_0_semantic_contract_id(),
        "7e8f54f15bd27d16643422f6904a23ea2004cab1098b56f8cd842a2397ff42fe",
        (true, false, false, false),
    );
    assert_profile(
        magic_s3_a_ordered_sba_0_1_0_semantic_contract_id(),
        "51efc0307d9ef8fc4fca46f8ea6e4ea5d5293cb8301c2a7917a590982079020e",
        (true, true, false, false),
    );
    assert_profile(
        magic_s3_b_basic_priority_0_1_0_semantic_contract_id(),
        "c480cbae69bf0496aff83bb973a859721bfa0f969351b33b3f0b09ee3f7c5498",
        (true, true, true, false),
    );
    assert_profile(
        magic_s3_c_draw_interaction_0_1_0_semantic_contract_id(),
        "2818c779c0a1f3b534d5551d9842a26e64ea93fa5906e8d43b499c4a3e042cb5",
        (true, true, true, true),
    );
}

#[test]
fn unknown_contract_does_not_construct_an_execution_profile() {
    assert!(magic_execution_profile(SemanticContractIdV1::from_digest_bytes([0xA5; 32])).is_none());
}
