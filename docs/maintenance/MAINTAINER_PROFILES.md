# Maintainer Verification Profiles

**Status:** accepted

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

## Certification / release
Use `just check-all`, then `just release-candidate`. Release evidence is valid only with no `NOT_RUN` or `FAIL`.

## Bootstrap and diagnostics
`just doctor` is non-mutating. `just bootstrap` only creates/updates `.venv` and installed Python packages; it never edits contracts or `Cargo.lock`.
