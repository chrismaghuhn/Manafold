# Maintainer Verification Profiles

**Status:** accepted

## Development
Use `just check-fast` continuously. This profile performs generated-contract,
repository, documentation, schema, golden-path, and explicit small Python
smoke checks. It does not scan the complete candidate/classification universe
or build the full review worklist.

`python scripts/run_python_tests.py --profile smoke` uses a closed allowlist.
New tests enter the full profile automatically and do not enter Smoke unless
the allowlist is deliberately changed.

## Integration
Use `just check` before review-ready status. Integration includes the complete
Python suite via `python scripts/run_python_tests.py --profile full`, then
Ruff, Mypy, Cargo, and maintainer-artifact checks. Missing native tools fail
this profile.

## Certification / release
Use `just check-all`, then `just release-candidate`. Release evidence is valid only with no `NOT_RUN` or `FAIL`.

## Bootstrap and diagnostics
`just doctor` is non-mutating. `just bootstrap` only creates/updates `.venv` and installed Python packages; it never edits contracts or `Cargo.lock`.
