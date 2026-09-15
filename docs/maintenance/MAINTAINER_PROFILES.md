# Maintainer Verification Profiles

**Status:** accepted

## Current project entry point

Use the repository root [`README.md`](../../README.md) for current foundation,
active-work, and authorization status; use [`docs/ROADMAP.md`](../ROADMAP.md)
for milestone ordering. This file owns the local and PR verification profiles
below. Follow [`DEVELOPER_SETUP.md`](DEVELOPER_SETUP.md) for the reproducible
Python/Rust environment and platform-specific direct commands. External census
status never authorizes Manafold engine semantics.

## Development
Use `just check-fast` continuously. This profile performs generated-contract,
repository, documentation, schema, golden-path, and explicit small Python
smoke checks. It does not run the complete Python suite or the native-tool
integration checks.

`python scripts/run_python_tests.py --profile smoke` uses a closed allowlist.
New tests enter the full profile automatically and do not enter Smoke unless
the allowlist is deliberately changed.

## Integration
Use `just check` before review-ready status. Integration includes the complete
Python suite via `python scripts/run_python_tests.py --profile full`, then
Ruff, Mypy, Cargo, and maintainer-artifact checks. Missing native tools fail
this profile.

For pull requests, `PR Fast` provides short development feedback and `PR
Integration` runs the exact-head repository-owned integration profile.
`manafold-pr-gate` is the stable aggregate result: it passes only when the
mandatory Fast, Integration, and CodeQL checks all conclude `success`.

Normal gate subprocesses have a hard 600-second budget and fail closed with a
timeout diagnostic and rerun command. The current unittest, pytest, and Cargo
test surfaces do not provide existing per-case interruption seams; precise
per-test enforcement remains tooling-blocked, while the normal gate budget
prevents an individual hang from eventually producing PASS evidence.

## Certification / release
Use `just check-all`, then `just release-candidate`. Release evidence is valid only with no `NOT_RUN` or `FAIL`.

## Bootstrap and diagnostics
`just doctor` is non-mutating. `just bootstrap` only creates/updates `.venv` and installed Python packages; it never edits contracts or `Cargo.lock`.

## Dependency audit
Run the separate `just audit-dependencies` or direct audit command from
[`DEVELOPER_SETUP.md`](DEVELOPER_SETUP.md) when the separately installed audit
tools and advisory data are available. Its `PASS`/`FAIL`/`BLOCKED` result is
not part of `manafold-pr-gate`.
