# Maintainer Verification Profiles

**Status:** accepted

## Development
Use `just check-fast` continuously.

## Integration
Use `just check` before review-ready status. Missing native tools fail this profile.

## Certification / release
Use `just check-all`, then `just release-candidate`. Release evidence is valid only with no `NOT_RUN` or `FAIL`.

## Bootstrap and diagnostics
`just doctor` is non-mutating. `just bootstrap` only creates/updates `.venv` and installed Python packages; it never edits contracts or `Cargo.lock`.
