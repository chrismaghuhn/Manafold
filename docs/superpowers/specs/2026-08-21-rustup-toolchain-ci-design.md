# Rustup Toolchain CI Recovery Design

**Status:** accepted for implementation
**Stability:** provisional
**Starting `origin/master`:** `bbbf0ee06b64889c318a7f0fd4d9b608d7181ed7`

## Goal

Restore the GitHub Actions Rust toolchain setup after the post-merge
Integration run failed while provisioning Rust `1.85.1`. This is a CI
reliability fix only; it does not change Manafold semantics, milestone status,
or release evidence.

## Observed failure

Integration run `32440005191` failed in the exact pinned-toolchain step on
`bbbf0ee06b64889c318a7f0fd4d9b608d7181ed7`:

```text
rustup toolchain install 1.85.1 --profile minimal --component rustfmt,clippy --force
error: failed to install component: 'clippy-preview-x86_64-unknown-linux-gnu', detected conflict: 'bin/cargo-clippy'
```

The runner reported recovery from a partially installed toolchain and rolled
back the installation. The failure is in runner-local toolchain state, before
repository checks execute.

## Design

Each workflow that installs the pinned toolchain first removes any stale or
partial `1.85.1` installation, tolerating the normal not-installed result,
then installs the same pinned toolchain without `--force`:

```text
rustup toolchain uninstall 1.85.1 || true
rustup toolchain install 1.85.1 --profile minimal --component <existing-components>
```

The component set remains `rustfmt` for PR Fast and `rustfmt,clippy` for
Integration and Nightly. The Rust version, minimal profile, cache step, and
all repository checks remain unchanged. The three workflows are updated
consistently so the same stale-installation failure cannot recur in another
Rust job.

## Evidence boundary

The existing failed Integration run is the RED evidence. A fresh PR Fast run,
manually dispatched Integration run, and manually dispatched Nightly run on
the final fix head must all succeed before this remediation is reported as
green. Local Rust and repository checks provide regression coverage but cannot
replace the hosted runner evidence.

## Explicit non-goals

No Rust source, Python source, generated contract, fixture, project status,
milestone closure, dependency version, or GitHub branch protection setting is
changed. The new PR is not merged by this change unless separately authorized.
