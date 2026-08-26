# M2.5.A Master-Drift Closure Report — REV3 Baseline Promotion

- **Slice:** M2.5.A (baseline promotion/provenance and MASTER_DRIFT closure only)
- **Status:** `M2_5_A_READY_FOR_EXTERNAL_REVIEW` (pending external review of this PR)
- **Machine-readable closure:** [`master_drift_closure_REV3.json`](master_drift_closure_REV3.json)
- **Import provenance:** [`IMPORT_PROVENANCE.json`](IMPORT_PROVENANCE.json)

## 1. Verified inputs

| Item | Value | Evidence |
|---|---|---|
| Input package | `Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip` | SHA-256 computed at import |
| Package SHA-256 | `99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90` | matched expected value exactly |
| Manifest | 72 entries; every path/byte-length/SHA-256 re-verified during import | `verification/import_structural_validation.json`, gate `PACKAGE_MANIFEST` = PASS |
| Expected artifacts (`expected_artifacts_REV3.json`) | all present in extracted package | independent cross-check during import |

Fresh execution evidence produced for this import (packaged claims were not trusted without re-execution):

| Check | Tool | Result |
|---|---|---|
| Structural validation | `tools/validate_manafold_m25_package_REV3.py` | PASS, 0 structural failures |
| Ranking validation | `tools/validate_manafold_m25_ranking_REV3.py` | PASS, 0 failures |
| Adversarial negative tests | `tools/run_manafold_m25_negative_tests_REV3.py` | PASS, 12/12 real mutation cases |
| Determinism comparison | two fresh offline generator runs + `compare_manafold_m25_runs_REV3.py` | PASS, 54/54 byte-identical files |

## 2. Repository identity

```text
REV3 baseline repo SHA (recorded at REV3 run start, refs/heads/master):
  05bd341635ba2150d721659b15b62c5ad311637f   (resolved 2026-08-23T01:50:03Z)

Current verified master SHA:
  9eb5da3d2cfa2c4612d22d208d658fd4132b7f6f

Relationship:
  05bd3416 is an ancestor of 9eb5da3d.
  Commits between: 10 (M2.C through M2.Final engine closure plus ADR 0041 acceptance).
```

Note: REV3's own `MASTER_DRIFT` gate was BLOCKED because master moved *during* the
REV3 run (`05bd3416` → `79da0dfe`). That gate proves run freshness only. This slice
owns the semantic question: does current master invalidate the imported research
assumptions?

## 3. Changed relevant contracts (range `05bd3416..9eb5da3d`)

| Surface | Change in range | Relevant to REV3 assumptions? |
|---|---|---|
| `docs/ROADMAP.md` (M2.5 scope) | unchanged | n/a — scope identical to what REV3 assumed |
| ADR 0041 capability-oriented semantic ownership | newly accepted (`9eb5da3d`) | examined in detail below |
| `docs/cards/*` (CAPABILITY_MODEL, CERTIFICATION, ADDING_CARDS) | unchanged | no drift |
| `cards/capabilities/registry.json`, `scripts/capability_census.py`, card/census schemas | unchanged | no drift |
| `docs/rules/AUTHORITY_POLICY.md` and other authority/source policies | unchanged | no drift |
| ADR 0037 derived card census / coverage index (identity model REV3 follows) | unchanged | no drift |
| `docs/INFORMATION_MODEL.md` (+9 lines, M2.E occurrence binding prose) | additive clarification | not consumed by REV3 |
| `docs/contracts/WIRE_CONTRACT.md` (+17 lines, M2.D/M2.E compatibility notes) | additive completion of already-accepted V2 shapes | not consumed by REV3 |
| `schemas/player-step.v2.schema.json` (adds required `submission`) | completes accepted V2 shape (M2.D) | not consumed by REV3 |
| Rust crates / Python engine implementation (M2.C–M2.Final) | large additions | offline research package consumes none of them |
| CI workflows, README, normative register | additive/bookkeeping | no impact |

### ADR 0041 assessment

ADR 0041 is accepted and authoritative. It requires that new M2.5 artifacts preserve:

```text
card/source identity != capability identity != semantic ownership != implementation != certification
```

The REV3 baseline already preserves every separation and introduces none of the
things ADR 0041 prohibits:

- `DeckRowIdentity`, source-provided card-level `OracleSemanticIdentity`, and
  separate face/printing identities are research/census identities only; they are
  not capability identifiers, runtime dispatch keys, or support claims
  (consistent with ADR 0037's census model).
- REV3 persists no authoritative scores, declares no replacement ranking formula,
  and makes no implementation or certification claims
  (`completion_status_REV3.json`: fail-closed; all research gates BLOCKED).
- ADR 0041 explicitly defers the semantic-domain inventory, process decomposition,
  and any machine-readable semantic-ownership representation to M2.5/M3 evidence;
  this import creates neither, so there is nothing to contradict.
- The remaining five REV3 gates stay BLOCKED exactly as packaged; importing the
  baseline does not promote any of them.

## 4. Impact classification

| REV3 assumption area | Impact | Revalidation required? |
|---|---|---|
| Deck-pair selection research (441 rows, 6 decks, quantity 600) | UNAFFECTED | no |
| Oracle/source identity model (`OracleSemanticIdentity` etc.) | UNAFFECTED | no |
| Classification authorities (402 shared; 392 non-terminal) | UNAFFECTED (non-terminal before and after) | no |
| Interaction census (15,679 candidates, all AMBIGUOUS_REQUIRES_REVIEW) | UNAFFECTED (blocked before and after) | no |
| Ranking status (no authoritative formula/scores; Fraction declared) | UNAFFECTED (fail-closed before and after) | no |
| M2 decision/information/wire interfaces | CHANGED in master, NOT consumed by REV3 | no |
| Semantic-ownership architecture | STRENGTHENED by ADR 0041 relative to REV3's identity separations | no |

## 5. Verdict

```text
material_semantic_drift        = NO
revalidation_required          = NO
rev3_evidence_reusable         = YES
MASTER_DRIFT                   = PASS
```

`MASTER_DRIFT = PASS` is granted on this evidence basis: current master does not
invalidate the imported REV3 research assumptions, and all derived artifacts were
re-verified deterministically against the current environment during import.

This PR itself adds only non-normative provenance artifacts under `sources/` plus
one standalone verification script; it modifies no normative contract, so it cannot
itself reintroduce drift against these assumptions.

Fail-closed property: `python scripts/check_m2_5_master_drift.py` grants PASS only
while HEAD equals `9eb5da3d2cfa2c4612d22d208d658fd4132b7f6f` and the closure record
is unmodified; its negative self-test proves stale/mismatched repository identity,
tampered records, missing evidence, and wrong schemas all fail closed.

## 6. State after M2.5.A

```text
MASTER_DRIFT                          = PASS
OFFICIAL_RULE_CITATION_CLOSURE        = BLOCKED
CLASSIFICATION_REFERENCE_CLOSURE      = BLOCKED
DECLARED_INTERACTION_MODEL_CLOSURE    = BLOCKED
REV2_REUSE_RATIO_REPRODUCIBLE         = BLOCKED
RANKING_UNCERTAINTY_PROPAGATION       = BLOCKED

DECK_PAIR_LOCKED                      = NO
AUTHORITATIVE_RANKING_AVAILABLE       = NO
M3_STARTED                            = NO
```
