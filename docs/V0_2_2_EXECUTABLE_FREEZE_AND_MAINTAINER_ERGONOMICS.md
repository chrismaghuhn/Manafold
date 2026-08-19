# V0.2.2 — Executable Freeze and Maintainer Ergonomics

**Status:** CONTRACT_FROZEN

V0.2.2 preserves the V0.2.1 semantic architecture and reduces the maintenance cost of keeping Rust, Python, schemas, fixtures and release evidence aligned.

## Contract catalog

`contracts/catalog/contract-vocabulary.v1.json` is authoritative only for mechanically duplicated closed vocabulary: terminal/truncation reasons, player results, zone names, observed-event kinds, selected schema IDs, and stable wire error-code names.

`scripts/generate_contracts.py` generates the corresponding Rust/Python vocabulary, the two catalog-owned schemas, and a reference document. `--check` is a required drift gate. Magic semantics, DTO layout, cross-field validation, state invariants and information-flow rules remain hand-written reviewed contracts.

## Maintainer profiles

- `just check-fast`: tight local iteration.
- `just check`: review-ready integration, including Rust/Ruff/Mypy.
- `just check-all`: certification/release smoke plus archive reproducibility.
- `just release-candidate`: authoritative external gate report.

## Golden path

`examples/golden-path/` is a tested synthetic vertical example across capability registry, card manifest, bundle, public wire fixtures, recursive closure and fail-closed certification. Its certification must remain `blocked` until a real executable kernel supplies semantic evidence.

## CI

PR Fast catches drift quickly; Integration runs full typed contracts on main/on-demand; Nightly Certification Smoke runs the expensive full profile.

## Freeze rule

V0.2.2 is `CONTRACT_FROZEN`. M1 is unblocked.
