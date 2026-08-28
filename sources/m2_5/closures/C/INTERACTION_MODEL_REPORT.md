# M2.5.C Interaction Model Report

Status: source snapshot generated for C verification; this report is derived documentation, not a closure input.

## Identity and authority

- REV3 model: `interaction-model.v1`; candidate member: `derived/Pair_Interaction_Census_REV3.csv`.
- REV3 archive SHA-256: `99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90`.
- Verified implementation base/master: `186d4b69ee406b19e1707d3f067f2bec14af3a34`; the exact H_exec is recorded only in the verification summary.
- Authority graph: model -> review additions -> candidate universe -> semantic classes -> classifications -> closure -> report/evidence.
- Digest graph: model + review additions + candidate universe + semantic classes + classifications -> closure; report, negative matrix, and verification summary are outside the closure.
- The closure binds exactly five semantic C inputs and does not bind this report, the negative matrix, or the verification summary.
- Closure status: `PASS`; raw SHA-256: `d81cf101399edabc9ec7cc833978393391a56daffb8e2a6a422b44fc5a30d2a7`.
- B2 binding summary: `sources/m2_5/closures/B2/requirement_family_catalog.v1.json=a9dc94b86a2efdb6885081191e53380cf5b3723a58487600b6372bcb789abb92, sources/m2_5/closures/B2/card_semantic_classifications.v1.json=40cd5b9c37e26157a6df0449a75040f8a5879d825e3946dd500d666a502201d5, sources/m2_5/closures/B2/classification_closure.v1.json=ed6a0bf4b0eb83c85027fdcc61eaf32bfa7bb06d4de78c77d0946d87212e7d43`.
- B1.Final binding summary: `sources/m2_5/closures/B1/official_authority_citations.v3.json=aaf684335be10255843f4b5debd6fed71835043eef9e585c5fa024109248a25a, sources/m2_5/closures/B1/official_authority_citation_closure.v2.json=b6980cffcb71bf73acba6ef698a418ad83cac21bbd0121a4ca9becfe6d630dea`.

## Reconciliation

- REV3 candidates: `15679`; current total: `15679`.
- Unchanged: `15679`; stale/removed/merged: `0/0/0`.
- New B2-derived candidates: `0` (forbidden in C V1); targeted higher-order additions: `0`.

## Terminal review

- Required interaction: `18`.
- Not an interaction with proof: `15661`.
- Out of declared scope: `0`.
- Unresolved: `0`.
- Semantic classes: `18`; source instances: `15679`.
- Review additions: `0`; targeted higher-order authority is empty in C V1.

## Evidence boundary

Card-trigger classes use their exact joined OSI, terminal B2 assignments/boundaries, and CR-603 B1.Final citation. Family-pair rows remain concrete source instances and are not promoted from co-occurrence alone.
- High-risk review coverage: 18 exact card-trigger OSI joins; 0 targeted higher-order records; 0 B2-derived candidates; the 15,661 family-pair rows remain non-interaction co-occurrence dispositions.

## Gate state

```text
CLASSIFICATION_REFERENCE_CLOSURE = PASS
OFFICIAL_RULE_CITATION_CLOSURE = PASS
DECLARED_INTERACTION_MODEL_CLOSURE = PASS
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
