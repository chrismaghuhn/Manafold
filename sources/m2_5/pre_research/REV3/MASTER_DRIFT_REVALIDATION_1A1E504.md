# M2.5.A Master-Drift Revalidation Report — Post-#83

- **Slice:** M2.5.A additive post-merge MASTER_DRIFT revalidation
- **Status:** `MASTER_DRIFT_REVALIDATED_FOR_DESCENDANTS`
- **Revalidation record:** [`master_drift_revalidation_1a1e504.v1.json`](master_drift_revalidation_1a1e504.v1.json)
- **Historical closure:** [`master_drift_closure_REV3.json`](master_drift_closure_REV3.json)
- **Historical import provenance:** [`IMPORT_PROVENANCE.json`](IMPORT_PROVENANCE.json)

This report is an additive validation layer. It does not rewrite, reinterpret, or
replace the historical REV3 import-time evidence. The historical closure remains
the authority for the original import-time master; this report records the
separate review that advances the effective descendant-drift anchor after merged
PR #83.

## 1. Preserved historical identity

| Item | Value | Raw SHA-256 / authority |
|---|---|---|
| Original REV3 recorded repository SHA | `05bd341635ba2150d721659b15b62c5ad311637f` | Historical closure and import provenance agree |
| Original import-time verified master SHA | `9eb5da3d2cfa2c4612d22d208d658fd4132b7f6f` | Historical closure and `IMPORT_PROVENANCE.json` agree |
| `master_drift_closure_REV3.json` | historical closure | `0bc7d190233214a7ca75f7aad590eda71afd43128b41b58a3a2f1606e7e10708` |
| `IMPORT_PROVENANCE.json` | historical import record | `39b236bf8885d705c9580a1b5d223efe5e96d284341724bc6016252546d0b7d0` |
| `MASTER_DRIFT_REPORT.md` | historical closure report | `e419a1e79733d9a4f3f43c1169e1f950bab129598a8727262888e7da0b93d56d` |
| REV3 source package | maintainer-private archive | `99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90` |

The three historical repository files remain byte-identical. In particular,
`verified_current_master_sha_at_import` remains the original
`9eb5da3d2cfa2c4612d22d208d658fd4132b7f6f`; it is not a mutable pointer to the
current master.

## 2. Reviewed post-B1.Final range

The exact range reviewed is:

```text
df3d760de2c6b22403764725e0ef707161bbce13..1a1e504cf2e6232d5b8da47bdfb989980aa41884
```

The ancestry is the reviewed PR #83 merge:

```text
1a1e504cf2e6232d5b8da47bdfb989980aa41884
├── first parent:  df3d760de2c6b22403764725e0ef707161bbce13
└── second parent: 34e23c57203b775d43e06e7946766566e4002a99
                     └── parent: b9765aa45321cb36a2a6531aa613bcb2788b1d26
```

The complete Git commit set in the range, in ancestry order, is:

```text
b9765aa45321cb36a2a6531aa613bcb2788b1d26
34e23c57203b775d43e06e7946766566e4002a99
1a1e504cf2e6232d5b8da47bdfb989980aa41884
```

The recomputed changed-file set is exactly:

```text
pytest.ini
python/tests/test_m2_h_gate_runner.py
scripts/run_python_tests.py
```

No other path is included in the reviewed range.

## 3. Actual change review

The three files were inspected as bytes and as executable/test behavior:

| Path | Observed change | Semantic assessment |
|---|---|---|
| `pytest.ini` | Adds `pythonpath = python/src` so pytest resolves the repository source layout | Test bootstrap only |
| `python/tests/test_m2_h_gate_runner.py` | Adds regression probes for source-path bootstrap, editable-install masking, and the M2.H import/skip path | Test coverage only |
| `scripts/run_python_tests.py` | Adds `configure_source_import_path()` and calls it before unittest discovery; keeps the shared runner entrypoint and result handling | Test runner bootstrap only |

The reviewed bytes do not alter Magic rules, authoritative Rust game semantics,
state, RNG, decisions, observations, information state, replay/checkpoints, Card
IR, capability definitions, B1/B1.Final authority, B2 classification/family
semantics, the REV3 candidate universe, interaction-model inputs, ranking formula
inputs, deck-pair status, or M3 status. The tests and runner only make the
repository Python source layout available before test imports/discovery.

The master-drift normative control surfaces were independently compared between
the previous effective head and the new master and are unchanged:

```text
crates/mtgml-rules/src/lib.rs
python/src/mtgml/observation.py
schemas/player-step.v2.schema.json
wire/golden/manifest.json
docs/contracts/WIRE_CONTRACT.md
docs/adr/0041-capability-oriented-semantic-domains-and-explicit-semantic-ownership.md
cards/capabilities/registry.json
```

## 4. Revalidation result

```text
material_semantic_drift          = NO
impact_classification            = TEST_INFRASTRUCTURE_ONLY
rev3_evidence_reusable           = YES
research_data_revalidation       = NO
MASTER_DRIFT                     = PASS
effective descendant anchor      = 1a1e504cf2e6232d5b8da47bdfb989980aa41884
```

The three repair paths are accepted only as the exact, reviewed historical
`df3d760..1a1e504` delta. They are not added to the permanent descendant
allowlist. Any future edit to them after `1a1e504` therefore requires another
reviewed revalidation and fails the ordinary descendant check until that occurs;
near-miss names remain rejected.

The historical closure, import provenance, and historical report remain immutable
inputs. The additive revalidation JSON binds all three historical files, the
revalidation report, and the pinned REV3 package by raw SHA-256. It does not bind
its own derived report back into the historical closure, so the evidence graph
remains acyclic.

## 5. Downstream state

This revalidation changes only the effective `MASTER_DRIFT` result for the new
master and its descendants:

```text
MASTER_DRIFT                       = PASS
CLASSIFICATION_REFERENCE_CLOSURE   = PASS
OFFICIAL_RULE_CITATION_CLOSURE     = PASS
DECLARED_INTERACTION_MODEL_CLOSURE = BLOCKED
REV2_REUSE_RATIO_REPRODUCIBLE      = BLOCKED
RANKING_UNCERTAINTY_PROPAGATION    = BLOCKED
DECK_PAIR_LOCKED                   = NO
AUTHORITATIVE_RANKING_AVAILABLE    = NO
M3_STARTED                         = NO
```
