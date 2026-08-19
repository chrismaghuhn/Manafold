# Manafold — Current Project State

> Updated after successful `just release-candidate` on the freeze commit.

- **Version:** `0.2.2`
- **Foundation:** V0.2.2 Executable Freeze & Maintainer Ergonomics
- **Freeze:** `CONTRACT_FROZEN`
- **M1 unblocked:** `true`
- **Playable engine:** `false`
- **Real Magic rules:** `false`
- **Real card support:** `false`

## Gate status

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

## Current blockers

None. V0.2.2 is `CONTRACT_FROZEN`. M1 is unblocked.

## What V0.2.2 added

- Single-source mechanical contract vocabulary and drift checking.
- Staged maintainer workflows (`doctor`, `bootstrap`, `check-fast`, `check`, `check-all`, `release-candidate`).
- Split PR/integration/nightly CI.
- Tested synthetic golden path.
- Deterministic source archive as the last verification gate.
- `source_tree_unchanged` gate ensuring verification does not mutate archived source.
