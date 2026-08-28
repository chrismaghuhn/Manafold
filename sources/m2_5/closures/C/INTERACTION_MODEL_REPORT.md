# M2.5.C Interaction Model Report

Status: source snapshot generated for C verification; this report is derived documentation, not a closure input.

## Identity and authority

- REV3 model: `interaction-model.v1`; candidate member: `derived/Pair_Interaction_Census_REV3.csv`.
- REV3 archive SHA-256: `99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90`.
- Verified implementation base/master: `186d4b69ee406b19e1707d3f067f2bec14af3a34`; the exact H_exec is recorded only in the verification summary.
- Authority graph: model -> review additions -> candidate universe -> semantic classes -> classifications -> closure -> report/evidence.
- Digest graph: model + review additions + candidate universe + semantic classes + classification root (which binds all shards) -> closure; report, negative matrix, and verification summary are outside the closure.
- The closure binds exactly five semantic C inputs and does not bind this report, the negative matrix, or the verification summary.
- Closure status: `BLOCKED`; raw SHA-256: `96171db27ab1abd5ceace39bdca0b7e72f13ccab6dd60ff8f9ba6b6394e93532`.
- Fixed classification layout: `16` shards with `15 x 1000 + 679` records; root `interaction_classifications.v3.json` binds every shard.
- B2 binding summary: `sources/m2_5/closures/B2/requirement_family_catalog.v1.json=a9dc94b86a2efdb6885081191e53380cf5b3723a58487600b6372bcb789abb92, sources/m2_5/closures/B2/card_semantic_classifications.v1.json=40cd5b9c37e26157a6df0449a75040f8a5879d825e3946dd500d666a502201d5, sources/m2_5/closures/B2/classification_closure.v1.json=ed6a0bf4b0eb83c85027fdcc61eaf32bfa7bb06d4de78c77d0946d87212e7d43`.
- B1.Final binding summary: `sources/m2_5/closures/B1/official_authority_citations.v3.json=aaf684335be10255843f4b5debd6fed71835043eef9e585c5fa024109248a25a, sources/m2_5/closures/B1/official_authority_citation_closure.v2.json=b6980cffcb71bf73acba6ef698a418ad83cac21bbd0121a4ca9becfe6d630dea`.

## Reconciliation

- REV3 candidates: `15679`; current total: `15679`.
- Unchanged: `15679`; stale/removed/merged: `0/0/0`.
- New B2-derived candidates: `0` (forbidden in C V3); targeted higher-order additions: `0`.

## Review state

- Resolved required interaction: `18`.
- Resolved not an interaction with proof: `0`.
- Resolved out of declared scope: `0`.
- Unresolved: `15661`.
- Semantic classes: `18`; source instances: `15679`.
- Review additions: `0`; targeted higher-order authority is empty in C V3.
- V2 to V3 migration parity: `PASS`; source `43bf3ccc6ff639c900914947ee0883b4731b8409`; target `interaction_classifications.v3.json`.
- Git-history publishability preflight: `NOT_RUN`; checked commit `H_exec not created`; hosting limit `104857600` bytes.

## Required review-domain coverage

- triggers_and_lki: applicable=18, not_applicable=0, unresolved=15661.
- replacement_layers_and_dependency: applicable=0, not_applicable=18, unresolved=15661.
- copy_and_token_creation: applicable=0, not_applicable=18, unresolved=15661.
- target_legality_protection_and_identity: applicable=0, not_applicable=18, unresolved=15661.
- control_and_ownership: applicable=0, not_applicable=18, unresolved=15661.
- commander_and_format: applicable=0, not_applicable=18, unresolved=15661.
- hidden_information_and_visibility: applicable=0, not_applicable=18, unresolved=15661.
- ordering_and_temporal_dependencies: applicable=0, not_applicable=18, unresolved=15661.
- source_versus_affected_identity: applicable=0, not_applicable=18, unresolved=15661.
- controller_owner_and_decision_actor: applicable=0, not_applicable=18, unresolved=15661.
- higher_order_interactions: applicable=0, not_applicable=18, unresolved=15661.

## Evidence boundary

Card-trigger classes use their exact joined OSI, terminal B2 assignments/boundaries, and CR-603 B1.Final citation. Family-pair rows remain concrete source instances and are not promoted from co-occurrence alone.
- High-risk review coverage is derived from the authoritative eleven candidate-level assessments; family-pair rows remain unresolved until a separately approved Pair/Relation Review Authority exists.

## Gate state

```text
CLASSIFICATION_REFERENCE_CLOSURE = PASS
OFFICIAL_RULE_CITATION_CLOSURE = PASS
DECLARED_INTERACTION_MODEL_CLOSURE = BLOCKED
REV2_REUSE_RATIO_REPRODUCIBLE = BLOCKED
RANKING_UNCERTAINTY_PROPAGATION = BLOCKED
DECK_PAIR_LOCKED = false
AUTHORITATIVE_RANKING_AVAILABLE = false
M3_STARTED = false
```

## Verification matrix

- Negative cases: `42` (C-001 through C-042).

## Phase B command status at H_exec creation

These commands were not yet run against the exact H_exec snapshot; their actual Phase B results are recorded in the final summary.
- `py -3.13 scripts/check_m2_5_master_drift.py`: `NOT_RUN`.
- `py -3.13 scripts/check_m2_5_master_drift.py --negative-self-test`: `NOT_RUN`.
- `py -3.13 scripts/check_m2_5_master_drift.py --verify-archive`: `NOT_RUN`.
- `py -3.13 scripts/check_m2_5_b1_authority_citations.py`: `NOT_RUN`.
- `py -3.13 scripts/check_m2_5_b1_authority_citations.py --negative-self-test`: `NOT_RUN`.
- `py -3.13 scripts/check_m2_5_b2_classifications.py`: `NOT_RUN`.
- `py -3.13 scripts/check_m2_5_b2_classifications.py --negative-self-test`: `NOT_RUN`.
- `py -3.13 scripts/check_m2_5_b1_final_authority_citations.py`: `NOT_RUN`.
- `py -3.13 scripts/check_m2_5_b1_final_authority_citations.py --negative-self-test`: `NOT_RUN`.
- `py -3.13 scripts/check_m2_5_c_interactions.py`: `NOT_RUN`.
- `py -3.13 scripts/check_m2_5_c_interactions.py --negative-self-test`: `NOT_RUN`.
- `py -3.13 scripts/verify_repository.py`: `NOT_RUN`.
- `py -3.13 scripts/run_checks.py integration`: `NOT_RUN`.
- `cargo +1.85.1 fmt --all -- --check`: `NOT_RUN`.
- `cargo +1.85.1 check --workspace --all-targets --all-features --locked`: `NOT_RUN`.
- Applicable Ruff, Mypy, Clippy, Rust tests, schema, conformance, information-safety, replay, and maintainer gates: `NOT_RUN`.
- Final command statuses are evidence in the verification summary; the summary remains outside the closure.
