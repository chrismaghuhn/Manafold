# Rustup Toolchain CI Recovery Plan

**Status:** provisional

> **For agentic workers:** Execute the steps in order and keep the final-head
> hosted evidence tied to the exact commit under review.

**Goal:** Make the pinned Rust `1.85.1` setup recoverable on GitHub-hosted
runners without changing project behavior.

## File map

- Modify `.github/workflows/pr-fast.yml`: recover `1.85.1`, then install it
  with `rustfmt` and no `--force`.
- Modify `.github/workflows/integration.yml`: recover `1.85.1`, then install it
  with `rustfmt,clippy` and no `--force`.
- Modify `.github/workflows/nightly.yml`: apply the same Integration setup.
- Create the design and this plan under `docs/superpowers/`.
- Modify `docs/normative-document-register.v1.json` to register the two
  provisional process artifacts.

## Task 1: Record the diagnosis

- [x] Capture the exact failed run `32440005191`, head SHA, failing command,
  and `cargo-clippy` conflict in the design.
- [x] Define the recovery command and preserve existing versions/components.

## Task 2: Patch the workflows

- [x] Replace each forced install with:

  ```text
  rustup toolchain uninstall 1.85.1 || true
  rustup toolchain install 1.85.1 --profile minimal --component <existing-components>
  ```

- [x] Confirm no unrelated workflow changes are present.

## Task 3: Verify locally

- [x] Run `git diff --check`.
- [x] Run `python scripts/run_checks.py fast`.
- [x] Run `python scripts/check_documentation.py`.
- [x] Run `python scripts/verify_repository.py`.
- [x] Run `cargo +1.85.1 fmt --all -- --check` and
  `cargo test --workspace --all-features --locked`.
- [x] Run `cargo +1.85.1 check --workspace --all-targets --all-features --locked`.

## Task 4: Verify on GitHub

- [ ] Commit and push one focused branch; open one Draft PR against `master`.
- [ ] Verify PR Fast and CodeQL on the exact final PR head.
- [ ] Dispatch Integration and Nightly on that same head and verify success.
- [ ] Report any unavailable or failing gate as `NOT_RUN` or `FAIL`; do not
  infer hosted success from local checks.

## Completion boundary

This plan closes only the Rustup CI provisioning failure. It does not update
M1.F, M1 closure, or any project milestone status.
