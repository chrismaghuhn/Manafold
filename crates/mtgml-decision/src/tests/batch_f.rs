#[test]
fn fnd_014_dense_candidate_paths_have_checked_u32_boundaries() {
    let source = include_str!("../lib.rs");
    assert!(!source.contains("expect(\"candidate ordering is bounded by u32\")"));
    assert!(!source.contains("index as u32"));
}
