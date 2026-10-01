use super::*;

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
