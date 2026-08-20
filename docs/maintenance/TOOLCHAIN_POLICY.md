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

## Dependency pins and locks

`python/requirements-dev.lock` contains exact direct development-tool pins. It prevents silent movement of Ruff, Mypy, pytest, and schema tooling, but V0.2.2 does not claim that this file alone is a hash-locked transitive Python environment.

`Cargo.lock` is mandatory before contract freeze and all Rust commands use `--locked`. A public certified release additionally requires a reproducible build image or fully resolved/hash-locked Python environment, dependency provenance, and the attestation decision tracked by OD-016/OD-021.

## Change rule

Changing a reference interpreter/compiler, supported Python range, direct tool pin, lockfile meaning, or CI image requires:

1. compatibility and reproducibility impact;
2. clean-machine evidence;
3. regenerated reports;
4. migration notes for public artifacts;
5. an ADR when persisted wire/replay/digest behavior can change.
