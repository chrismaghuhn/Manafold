# Manafold — V0.2.2 Foundation Snapshot

> Historical V0.2.2 foundation snapshot. Live current status is owned by the
> repository root [`README.md`](../README.md) and [`docs/ROADMAP.md`](../docs/ROADMAP.md);
> this file is a snapshot/pointer, not a second status authority.

- **Version:** `0.2.2`
- **Foundation:** V0.2.2 Executable Freeze & Maintainer Ergonomics
- **Freeze:** `CONTRACT_FROZEN`
- **M1 unblocked:** `true`
- **Playable engine:** `false`
- **Real Magic rules:** `false`
- **Real card support:** `false`

## V0.2.2 foundation gate snapshot

| Gate | Status |
|---|---:|
| `archive_reproducibility` | **PASS** |
| `cargo_check` | **PASS** |
| `cargo_clippy` | **PASS** |
| `cargo_fmt` | **PASS** |
| `cargo_lock` | **PASS** |
| `cargo_test` | **PASS** |
| `documentation_contracts` | **PASS** |
| `generated_contract_drift` | **PASS** |
| `maintainer_artifacts` | **PASS** |
| `mypy` | **PASS** |
| `python_tests` | **PASS** |
| `python_toolchain` | **PASS** |
| `repository_verifier` | **PASS** |
| `ruff` | **PASS** |
| `ruff_format` | **PASS** |
| `rust_source_structure` | **PASS** |
| `schema_validation` | **PASS** |
| `source_tree_unchanged` | **PASS** |
| `synthetic_golden_path` | **PASS** |

## What V0.2.2 added

- Single-source mechanical contract vocabulary and drift checking.
- Staged maintainer workflows (`doctor`, `bootstrap`, `check-fast`, `check`, `check-all`, `release-candidate`).
- Split PR/integration/nightly CI.
- Tested synthetic golden path.
- Deterministic source archive as the last verification gate.
- `source_tree_unchanged` gate ensuring verification does not mutate archived source.
