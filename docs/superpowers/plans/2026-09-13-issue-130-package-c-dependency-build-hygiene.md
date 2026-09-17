# Issue 130 Package C Dependency and Build Hygiene Implementation Plan

**Status:** provisional implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pin every external GitHub Action immutably, add an explicit bounded Rust/Python dependency-audit path with fail-closed statuses, and define development/CI/public-release reproducibility levels without claiming public release reproducibility or attestation.

**Architecture:** The workflow files remain the authoritative Action-pin source; `scripts/verify_repository.py` rejects every external mutable Action reference. `scripts/run_dependency_audit.py` is a read-only orchestration boundary that invokes separately installed ecosystem-native tools, classifies clean/vulnerable/unavailable execution as `PASS`/`FAIL`/`BLOCKED`, and never participates in `manafold-pr-gate`. Existing `TOOLCHAIN_POLICY.md` and `DEPENDENCY_POLICY.md` own the reproducibility and update-review rules; no new status source or dependency bot is introduced.

**Tech Stack:** GitHub Actions, GitHub upstream tag refs, Python 3.13.15 project `.venv`, Rust 1.85.1/Cargo.lock, audit-only Rust 1.88.0 for RustSec `cargo-audit` 0.22.2, PyPA `pip-audit` 2.10.1, Python subprocess/unittest/pytest, existing documentation registry.

---

### Task 1: Verify the Package-B boundary and establish the exact base

**Files:**
- Read: PR #157, `origin/master`, `master` protection, `.python-version`, `rust-toolchain.toml`, `.github/dependabot.yml`
- Read: `docs/maintenance/TOOLCHAIN_POLICY.md`, `docs/maintenance/DEPENDENCY_POLICY.md`, `docs/maintenance/DEVELOPER_SETUP.md`, `docs/OPEN_DECISIONS.md`

- [x] **Step 1: Verify Package B is merged and current.**

Confirm PR #157 is `MERGED`, its merge tree contains the corrected `.venv` authority, `windows-setup-smoke`, and `manafold-pr-gate` prerequisite. Fetch `origin/master` and record the live merge SHA `46a83ba7e486ee8590d885bce8bc37795a2f0a80` as the Package-C base.

- [x] **Step 2: Verify protection without mutation.**

Confirm `master` is protected, `manafold-pr-gate` is the only required check, strict/up-to-date is enabled, administrator enforcement is enabled, required approvals remain zero, and force-push/deletion remain blocked. Do not change these settings.

### Task 2: Add red tests for Action pins, audit status, and release policy

**Files:**
- Create: `python/tests/test_package_c_hygiene.py`
- Test: `python/tests/test_package_c_hygiene.py`

- [ ] **Step 1: Add Action-pin validation tests.**

Test the pure verifier contract with these values:

```python
assert action_pin_error("actions/checkout@" + "a" * 40) is None
assert action_pin_error("actions/checkout@v7") is not None
assert action_pin_error("actions/checkout@v2") is not None
assert action_pin_error("actions/checkout@main") is not None
assert action_pin_error("actions/checkout@master") is not None
assert action_pin_error("actions/checkout@" + "a" * 39) is not None
assert action_pin_error("actions/checkout@" + "A" * 40) is not None
assert action_pin_error("./.github/actions/local") is None
```

Also assert every current workflow external Action reference is immutable and
`.github/dependabot.yml` still contains `package-ecosystem: github-actions`.

- [ ] **Step 2: Add audit orchestration tests without network or installation.**

Mock the bounded subprocess boundary and assert:

```text
Rust clean return code 0       -> PASS
Rust return code 1             -> FAIL
Rust executable missing        -> BLOCKED
Rust timeout                   -> BLOCKED
Python clean return code 0     -> PASS
Python return code 1           -> FAIL
Python module missing          -> BLOCKED
Python timeout                 -> BLOCKED
one PASS plus one BLOCKED      -> overall BLOCKED/nonzero
one FAIL plus one BLOCKED      -> overall FAIL/nonzero
```

The tests must inspect commands and timeout values, and must not invoke a live
advisory service or package installer.

- [ ] **Step 3: Add policy-structure tests.**

Read the existing Toolchain Policy and Open Decisions register and assert that
the three reproducibility levels are present, the development lock is not
described as hash/transitive locked, `OD-016` remains `partial`, `OD-021`
remains `open`, and the policy does not claim signed release attestation.

### Task 3: Pin all current external GitHub Actions

**Files:**
- Modify: `.github/workflows/integration.yml`
- Modify: `.github/workflows/nightly.yml`
- Modify: `.github/workflows/pr-fast.yml`
- Modify: `.github/workflows/pr-integration.yml`
- Modify: `.github/workflows/windows-setup-smoke.yml`

- [ ] **Step 1: Apply the verified upstream tag commits.**

Replace every current mutable reference with these exact commits resolved from
the official upstream tags:

```yaml
actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7
actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97 # v7
Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2
```

Preserve the existing action inputs, job names, exact-head behavior, and
Package-B `.venv` workflow behavior.

- [ ] **Step 2: Run the red Action tests and repository verifier.**

Run:

```powershell
<project-python> -B -m pytest -q python/tests/test_package_c_hygiene.py -k action
<project-python> scripts/verify_repository.py
```

Expected result: PASS with 16 current external Action uses and no mutable refs.

### Task 4: Implement the repository-owned Action-pin invariant

**Files:**
- Modify: `scripts/verify_repository.py`
- Test: `python/tests/test_package_c_hygiene.py`

- [ ] **Step 1: Add a pure fail-closed reference validator.**

Implement:

```python
ACTION_PIN_RE = re.compile(
    r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+@[0-9a-f]{40}$"
)

def action_pin_error(reference: str) -> str | None:
    if reference.startswith(("./", "../")):
        return None
    if ACTION_PIN_RE.fullmatch(reference):
        return None
    return "external GitHub Action must use a 40-character lowercase commit SHA"
```

Scan every `.github/workflows/*.yml` and `*.yaml` `uses:` value, allow only
local `./`/`../` actions as exceptions, and call the existing `fail()` path
with file and line context for any invalid external reference. Do not create a
separate registry or DSL.

- [ ] **Step 2: Require the audit workflow/script as repository entry points.**

Add `scripts/run_dependency_audit.py` and
`.github/workflows/dependency-audit.yml` to `verify_repository.py`’s required
files after those files exist. Keep the existing required-file checks and all
Package-B invariants unchanged.

- [ ] **Step 3: Run the verifier and negative matrix.**

Run `<project-python> scripts/verify_repository.py` and the complete Action-pin
tests. Expected result: PASS; mutable tags, branches, short SHAs,
uppercase/noncanonical SHAs, and non-local malformed values are rejected.

### Task 5: Implement bounded dependency-audit orchestration

**Files:**
- Create: `scripts/run_dependency_audit.py`
- Modify: `justfile`
- Modify: `scripts/README.md`
- Test: `python/tests/test_package_c_hygiene.py`

- [ ] **Step 1: Add one bounded subprocess boundary.**

Define:

```python
MAX_SINGLE_AUDIT_SUBPROCESS_RUNTIME_SECONDS = 600
AUDIT_PASS = "PASS"
AUDIT_FAIL = "FAIL"
AUDIT_BLOCKED = "BLOCKED"
```

Run `cargo-audit --version`, `cargo-audit audit --json`,
`<audit-python> -m pip_audit --version`, and
`<audit-python> -m pip_audit --requirement python/requirements-dev.lock
--format json --progress-spinner off` with `timeout=600`, captured output, and
no retry. `FileNotFoundError`, other spawn errors, and `TimeoutExpired` classify
as `BLOCKED` and print `TIMEOUT`/command/limit/`RERUN` diagnostics where
applicable.

- [ ] **Step 2: Keep audit semantics fail-closed.**

Use ecosystem-native exit semantics: return code `0` is clean `PASS`, return
code `1` is a vulnerability/policy `FAIL`, and any other executed nonzero
result is an operational/advisory-data `BLOCKED`. A missing version probe
blocks that ecosystem before its audit command. Aggregate with `FAIL` dominant,
then `BLOCKED`, then `PASS`; return process codes `0`, `1`, and `2`
respectively.

Print explicit fields for both ecosystems and `AUDIT_OVERALL`. Never pass
`--fix`, mutate `Cargo.lock`, write a suppression list, or install/update a
dependency from this script.

- [ ] **Step 3: Add explicit local/just entry points.**

Add:

```make
audit-dependencies:
    {{project_python}} scripts/run_dependency_audit.py
```

Document that the audit tools are installed separately and that missing tools
produce `BLOCKED`; ordinary bootstrap, fast, and integration do not install or
fetch advisory data.

- [ ] **Step 4: Run mocked audit tests.**

Run `<project-python> -B -m pytest -q python/tests/test_package_c_hygiene.py -k audit`. Expected result: PASS with no network or package-install side effects.

### Task 6: Add the separate scheduled/manual audit workflow

**Files:**
- Create: `.github/workflows/dependency-audit.yml`
- Test: `python/tests/test_package_c_hygiene.py`

- [ ] **Step 1: Define the non-gating workflow.**

Use `workflow_dispatch` and a weekly schedule, `ubuntu-latest`, and a job-level
`timeout-minutes: 10`. Pin checkout and setup-python using the immutable SHAs
from Task 3. Bootstrap the project `.venv`, install `pip-audit==2.10.1` in a
runner-temp audit environment, install audit-only Rust 1.88.0, install
`cargo-audit==0.22.2` into a runner-temp Cargo root with `--locked` using
`cargo +1.88.0`, and invoke
`.venv/bin/python scripts/run_dependency_audit.py` with those tool paths.

Do not add this workflow to `manafold-pr-gate` or branch protection. Do not
run fixes, dependency upgrades, full certification, fuzzing, soak, benchmark,
or release archive work.

- [ ] **Step 2: Bound every significant hosted step.**

Give bootstrap, audit-tool installation, and audit execution explicit
`timeout-minutes: 10`. The workflow must fail visibly for vulnerabilities or
tool/advisory unavailability; no `continue-on-error` may turn either result
green.

- [ ] **Step 3: Add workflow-structure tests.**

Assert the workflow uses only immutable Actions, is scheduled/manual rather
than a PR-gate prerequisite, has the stable audit command, uses the pinned
tool versions, and bounds the significant run steps.

### Task 7: Define reproducibility levels and preserve open decisions

**Files:**
- Modify: `docs/maintenance/TOOLCHAIN_POLICY.md`
- Modify: `docs/maintenance/DEPENDENCY_POLICY.md`
- Modify: `docs/maintenance/DEVELOPER_SETUP.md`
- Modify: `docs/maintenance/MAINTAINER_PROFILES.md`
- Test: `python/tests/test_package_c_hygiene.py`

- [ ] **Step 1: Add the three owned policy levels.**

In `TOOLCHAIN_POLICY.md`, define:

```text
LEVEL 1 — DEVELOPMENT REFERENCE
exact Python + project .venv + exact direct development pins + Cargo.lock + pinned Rust

LEVEL 2 — CI REFERENCE
exact source head + exact reference Python/Rust + immutable Action revisions + project tools + locked Cargo graph

LEVEL 3 — PUBLIC RELEASE REPRODUCIBILITY
an explicitly accepted fully resolved/hash-locked Python environment OR immutable reproducible build image,
plus artifact provenance and attestation decisions
```

State that Package C defines the boundary but selects neither public strategy
implicitly.

- [ ] **Step 2: Make OD-016/OD-021 boundaries explicit.**

State that `OD-016 = PARTIAL` because the public release strategy is still
unselected and that `OD-021 = OPEN` for certification artifact signing/
attestation. Keep `requirements-dev.lock` described as exact direct pins only;
do not call it transitive/hash locked.

- [ ] **Step 3: Document update/audit review ownership.**

Document Dependabot’s `github-actions`, `cargo`, and `pip` proposal role, human
review plus exact-head gates as acceptance authority, and the separate audit
workflow. Explain that security dependency changes still undergo normal
semantic/ADR review when they affect codecs, RNG, replay, checkpoints, hashing,
or wire contracts.

- [ ] **Step 4: Run policy/documentation tests.**

Run `<project-python> -B -m pytest -q python/tests/test_package_c_hygiene.py -k policy` and `<project-python> scripts/check_documentation.py`. Expected result: PASS with no new status source and no false release claim.

### Task 8: Full local/hosted verification and independent review checkpoint

**Files:**
- Verify: all files changed in Tasks 3–7

- [ ] **Step 1: Run the complete requested local gates.**

Using the Package-B project interpreter, execute doctor strict, full Python
tests, Fast/Integration/Certification, Ruff, mypy, Cargo fmt/check/clippy/test,
documentation, repository, and `git diff --check`. Run the actual dependency
audit path if both tools and advisory data are available; otherwise report its
actual `BLOCKED` result without upgrading it.

- [ ] **Step 2: Inspect scope and immutable ownership.**

Confirm `Cargo.lock`, semantic contracts, schemas, generated outputs, Magic
rules/cards/decks, RNG, replay/checkpoint/digest semantics, Package D tooling,
branch protection, and Dependabot configuration were not weakened or
unrelatedly changed. Confirm no normal gate or audit subprocess exceeds 600
seconds and no retry loop multiplies that budget.

- [ ] **Step 3: Commit and push the exact Package-C head.**

Use branch `chris/130-c-dependency-build-hygiene` and a focused commit such as
`chore(maintainers): harden dependency and build hygiene`. Push and open a PR
titled `chore(maintainers): harden dependency and build hygiene`, reference
`Issue #130` and `Package C`, do not use `Fixes #130`, and do not merge.

- [ ] **Step 4: Verify hosted exact-head evidence.**

Wait for actual success on `PR Fast`, `PR Integration`,
`windows-setup-smoke`, `manafold-pr-gate`, all three Analyze checks, and
`CodeQL` at the final PR head. Verify the separate audit workflow independently
as `PASS`, `FAIL`, `BLOCKED`, or `NOT_RUN`; it is not a PR-gate prerequisite.

- [ ] **Step 5: Stop for independent Exact-SHA review.**

Report the final base/head, audit status, policy levels, unchanged protection,
and `INDEPENDENT_EXACT_SHA_REVIEW = PENDING`; keep merge unperformed.
