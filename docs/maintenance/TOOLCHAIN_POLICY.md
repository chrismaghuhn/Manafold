# Toolchain Policy

**Status:** accepted V0.2.2 reference-toolchain policy
**Stability:** normative

## Reference development and freeze toolchain

V0.2.2 uses one exact reference toolchain for generated freeze evidence:

```text
Python: 3.13.15
Rust:   1.85.1 with rustfmt and Clippy
```

`.python-version`, `rust-toolchain.toml`, CI, the environment doctor, and the generated verification report must agree. Running on another version may be useful for compatibility smoke tests but cannot satisfy the reference freeze gate.

## Python runtime compatibility versus reference version

The rules-free Python client currently declares `>=3.11,<3.14`. That is a compatibility intention, not the freeze interpreter. Mypy targets Python 3.11 deliberately so public DTO/client code remains valid at the lowest declared runtime; the full repository verification executes under Python 3.13.15.

Compatibility is proven by a version matrix. Freeze/release reproducibility is proven by the exact reference interpreter. Neither substitutes for the other.

## Reproducibility levels

The project uses three explicit reproducibility levels:

### Level 1 — Development reference

The development reference requires the exact Python interpreter `3.13.15`,
the project `.venv`, exact direct development-tool pins, `Cargo.lock`, and the
pinned Rust `1.85.1` toolchain with `rustfmt` and Clippy. This level supports
normal local development and repository verification.

### Level 2 — CI reference

The CI reference requires the exact checked-out source head, the exact
reference Python and Rust toolchains, immutable GitHub Action revisions,
project-owned Python tools, and the locked Cargo dependency graph. This level
supports trusted repository gate evidence; it does not by itself establish
public release reproducibility.

### Level 3 — Public release reproducibility

A public release requires one explicitly accepted strategy: either a fully
resolved/hash-locked Python dependency environment or an immutable reproducible
build image with pinned dependency provenance. It also requires the artifact
provenance and attestation decisions. Package C defines this boundary but does
not select either public strategy implicitly.

## Dependency pins and locks

`python/requirements-dev.lock` contains exact direct development-tool pins. It prevents silent movement of Ruff, Mypy, pytest, and schema tooling, but V0.2.2 does not claim that this file alone is a hash-locked transitive Python environment.

`Cargo.lock` is mandatory before contract freeze and all Rust commands use `--locked`. A public certified release additionally requires a reproducible build image or fully resolved/hash-locked Python environment, dependency provenance, and the attestation decision tracked by OD-016/OD-021.

Accordingly, `OD-016` remains `PARTIAL`: the public release strategy and its
transitive Python/build provenance are not yet accepted. `OD-021` remains
`OPEN`: certification artifact signing and attestation are not resolved here.

The separate dependency audit path uses RustSec `cargo-audit` and PyPA
`pip-audit` as read-only advisory checks. Audit availability is reported
separately from merge correctness and is not a reproducibility claim.

## Change rule

Changing a reference interpreter/compiler, supported Python range, direct tool pin, lockfile meaning, or CI image requires:

1. compatibility and reproducibility impact;
2. clean-machine evidence;
3. regenerated reports;
4. migration notes for public artifacts;
5. an ADR when persisted wire/replay/digest behavior can change.
