# Issue 130 Package B Reproducible Developer Setup Implementation Plan

**Status:** provisional implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the project-pinned Python environment the authority for normal Python verification, harden bootstrap and doctor failure behavior, and add a bounded exact-head Windows setup smoke to the protected aggregate gate.

**Architecture:** Keep `.python-version`, `rust-toolchain.toml`, `python/requirements-dev.lock`, and the existing scripts as the authoritative setup inputs. `bootstrap.py` owns only the project `.venv` and its pinned Python dependencies; `doctor.py` is non-mutating and proves the selected environment; `run_checks.py` invokes Python-owned tools through its selected interpreter. The Windows workflow proves the documented fresh-checkout path and is consumed by `manafold-pr-gate` without changing branch protection.

**Tech Stack:** Python 3.13.15, Python `venv`/`ensurepip`/`subprocess`, pytest/unittest, YAML GitHub Actions, existing Rust/rustup toolchain, repository documentation registry.

---

### Task 1: Preserve the verified baseline and isolate the implementation

**Files:**
- Read: `.python-version`, `rust-toolchain.toml`, `python/requirements-dev.lock`, `scripts/bootstrap.py`, `scripts/doctor.py`, `scripts/run_checks.py`, `scripts/aggregate_pr_gate.py`, `justfile`
- Read: `README.md`, `docs/NORMATIVE_HIERARCHY.md`, `docs/ROADMAP.md`, `docs/maintenance/MAINTAINER_PROFILES.md`, `docs/maintenance/TOOLCHAIN_POLICY.md`
- Test: `python/tests/test_developer_setup.py`, `python/tests/test_python_test_profiles.py`, `python/tests/test_m2_final_gate_runner.py`

- [x] **Step 1: Verify the live base ref and protection without mutation.**

Run `git fetch origin master`, verify `origin/master` is `e3073042c1366f4477b03764f0e5802e08298f8d`, inspect the live `master` branch protection, and create/use `chris/130-b-reproducible-developer-setup` from that exact SHA. Record that protection remains `manafold-pr-gate`, strict/up-to-date, administrator-enforced, zero approvals, force-push blocked, and deletion blocked.

- [x] **Step 2: Add the focused red tests before implementation.**

Run:

```powershell
<project-python> -B -m pytest -q python/tests/test_developer_setup.py python/tests/test_python_test_profiles.py python/tests/test_m2_final_gate_runner.py
```

Expected result before implementation: nonzero because the new bootstrap/doctor contracts, Windows prerequisite, and exact-head workflow do not yet exist.

### Task 2: Bind normal Python checks to the selected interpreter

**Files:**
- Modify: `scripts/run_checks.py`
- Test: `python/tests/test_python_test_profiles.py`

- [x] **Step 1: Add the exact reference-Python guard.**

Implement:

```python
def reference_python_matches() -> bool:
    required = (ROOT / ".python-version").read_text(encoding="utf-8").strip()
    actual = ".".join(map(str, sys.version_info[:3]))
    return actual == required and project_python_matches()
```

Implement `project_python_path()` for `.venv/Scripts/python.exe` on Windows and
`.venv/bin/python` on POSIX, compare normalized absolute executable paths
without resolving away POSIX virtual-environment symlinks, and at the start of
`main()`, before running any profile subprocess, return `2` and print the
version/path mismatch and a `RERUN` line when the guard is false.

- [x] **Step 2: Replace bare Python-tool commands with module invocation.**

Keep native commands unchanged and set the Python-owned integration commands to:

```python
[
    sys.executable,
    "-m",
    "ruff",
    "format",
    "--check",
    "python",
    "scripts",
]
[
    sys.executable,
    "-m",
    "ruff",
    "check",
    "python",
    "scripts",
]
[
    sys.executable,
    "-m",
    "mypy",
    "--config-file",
    "python/pyproject.toml",
]
```

- [x] **Step 3: Run the focused profile tests.**

Run `<project-python> -B -m pytest -q python/tests/test_python_test_profiles.py`. Expected result: PASS.

### Task 3: Make bootstrap bounded, idempotent, and fail closed

**Files:**
- Modify: `scripts/bootstrap.py`
- Test: `python/tests/test_developer_setup.py`

- [x] **Step 1: Keep the wrong-version precondition before `.venv` mutation.**

Read `.python-version`, compare it with `sys.version_info[:3]`, and emit a nonzero diagnostic before taking a protected-file snapshot or invoking `venv.EnvBuilder` when the versions differ.

- [x] **Step 2: Add bounded subprocess execution and Python identity probing.**

Define:

```python
MAX_BOOTSTRAP_SUBPROCESS_RUNTIME_SECONDS = 600
```

Run `ensurepip`, locked dependency installation, and editable installation through the selected `.venv` executable with `subprocess.run(..., timeout=600)`. On timeout print `TIMEOUT`, the command, `limit=600s`, and `RERUN`; on any nonzero result stop immediately and return nonzero.

- [x] **Step 3: Preserve ownership of frozen inputs.**

Snapshot bytes for `.python-version`, `rust-toolchain.toml`, `Cargo.lock`, and `python/requirements-dev.lock` before mutation. Compare them in `finally` and fail if any changed. Do not invoke `cargo`, `rustup`, or any Rust installer from bootstrap.

- [x] **Step 4: Verify rerun behavior with mocks.**

Run `<project-python> -B -m pytest -q python/tests/test_developer_setup.py -k bootstrap`. Expected result: PASS without real package installation; the tests cover exact Python, wrong Python, installation failure, timeout, idempotence, and protected files.

### Task 4: Make doctor a bounded, non-mutating environment proof

**Files:**
- Modify: `scripts/doctor.py`
- Test: `python/tests/test_developer_setup.py`

- [ ] **Step 1: Add bounded probes and project-interpreter identity.**

Define:

```python
DOCTOR_PROBE_TIMEOUT_SECONDS = 30

def project_python_path() -> Path:
    return ROOT / (".venv/Scripts/python.exe" if sys.platform == "win32" else ".venv/bin/python")
```

Every version/environment probe must use `subprocess.run(..., timeout=DOCTOR_PROBE_TIMEOUT_SECONDS)` and print `TIMEOUT`, command identity, `limit=30s`, and `RERUN` on timeout. The command probe must never install, update, activate, or write files.

- [ ] **Step 2: Prove Python and Python-owned tooling through `sys.executable`.**

Report repository root, OS/platform facts, shell variables, `sys.executable`, current and required Python versions, and whether `sys.executable` resolves to the project `.venv`. Check required modules with `find_spec`, and check `ruff`, `mypy`, and `pytest` using both pinned distribution metadata and `[sys.executable, "-m", tool, "--version"]`; never use `shutil.which` as the authority for those Python tools.

- [ ] **Step 3: Probe the pinned Rust environment without mutation.**

Read `rust-toolchain.toml`, report the required channel/components, verify `rustup`/`cargo`/`rustc`/`rustfmt`/`clippy-driver` availability and versions, check the active rustup toolchain matches `1.85.1`, and check `Cargo.lock`. On native Windows, `bash` and `just` are informational/optional for strict direct Python/Cargo use; on POSIX/WSL they remain required for the documented bash-oriented `just` path.

- [ ] **Step 4: Keep strict and diagnostic modes distinct.**

`doctor` reports detected problems and exits zero in diagnostic mode; `doctor --strict` exits nonzero for required Python, project `.venv`, module/tool, Rust, lockfile, or required native-tool mismatches. Run `<project-python> -B -m pytest -q python/tests/test_developer_setup.py -k doctor`. Expected result: PASS.

### Task 5: Add Windows setup smoke and aggregate prerequisite

**Files:**
- Create: `.github/workflows/windows-setup-smoke.yml`
- Modify: `.github/workflows/pr-fast.yml`
- Modify: `.github/workflows/pr-integration.yml`
- Modify: `.github/workflows/integration.yml`
- Modify: `.github/workflows/nightly.yml`
- Modify: `justfile`
- Modify: `scripts/aggregate_pr_gate.py`
- Test: `python/tests/test_m2_final_gate_runner.py`

- [ ] **Step 1: Make the aggregate require the stable Windows check.**

Insert `"windows-setup-smoke"` into `REQUIRED_CHECKS`. Retain existing fail-closed evaluation: success passes, failure/cancelled/skipped/neutral fails, and missing/pending waits before the bounded aggregate timeout fails.

- [ ] **Step 2: Define the exact-head Windows workflow.**

Use a stable job/check name and these key values:

```yaml
name: Windows Setup Smoke
jobs:
  windows-setup-smoke:
    name: windows-setup-smoke
    runs-on: windows-latest
```

Checkout with `ref: ${{ github.event.pull_request.head.sha }}`, assert `git rev-parse HEAD` equals that SHA, use `actions/setup-python` with `.python-version`, require no pre-existing `.venv`, run the bounded bootstrap, run `.venv\Scripts\python.exe scripts/doctor.py --strict`, and run `.venv\Scripts\python.exe scripts/run_checks.py fast`. Give each significant `run` step `timeout-minutes: 10`; do not run certification, fuzzing, soak, benchmark, or release work.

- [ ] **Step 3: Run aggregate/workflow tests.**

Run `<project-python> -B -m pytest -q python/tests/test_m2_final_gate_runner.py`. Expected result: PASS, including success and failure/cancelled/skipped/missing Windows cases and the exact-head/budgeted workflow structure.

- [ ] **Step 4: Route reference profiles through the project environment.**

After the setup action invokes host Python only for bootstrap, run the reference
profiles with `.venv/bin/python` on Ubuntu/WSL. Update the matching `just`
recipes to use the same POSIX project executable after bootstrap; retain the
host `python` invocations for diagnostic bootstrap entry points and retain the
intentional 3.11/3.12/3.13 compatibility matrix as a separate compatibility
test.

### Task 6: Document the durable setup contract

**Files:**
- Create: `docs/maintenance/DEVELOPER_SETUP.md`
- Modify: `README.md`
- Modify: `docs/README.md`
- Modify: `docs/maintenance/MAINTAINER_PROFILES.md`
- Modify: `docs/normative-document-register.v1.json`

- [ ] **Step 1: Write one setup document under maintainer ownership.**

Document `.python-version` `3.13.15`, `rust-toolchain.toml` `1.85.1`, the direct Windows commands using `py -3.13` plus exact patch validation, the POSIX/WSL commands using `python3.13`, the canonical `.venv\Scripts\python.exe` and `.venv/bin/python` paths, optional activation, strict doctor, fast/integration/certification checks, stale `.venv` recovery, and the distinction between native direct paths and bash-required `just` recipes. State that bootstrap/doctor do not mutate Rust or lock contracts, global Python tools are not verification authority, and even an exact-version global Python is rejected by `run_checks.py`.

- [ ] **Step 2: Link the setup document from current maintainer entry points.**

Add links from the root README’s Start here list, `docs/README.md` maintainer process links, and `MAINTAINER_PROFILES.md` without duplicating the implementation details or creating a status artifact.

- [ ] **Step 3: Register the new document.**

Add `docs/maintenance/DEVELOPER_SETUP.md` to `docs/normative-document-register.v1.json` as the accepted maintainer process document, preserving the existing deterministic ordering and registry schema.

### Task 7: Keep repository structural checks aligned

**Files:**
- Modify: `scripts/verify_repository.py`
- Test: existing repository/documentation checks

- [ ] **Step 1: Require the maintained setup/workflow entry points.**

Add the new workflow and setup document to the repository’s required-file checks, while keeping the existing `just` shell contract visible instead of claiming shell neutrality.

- [ ] **Step 2: Run focused structural checks.**

Run:

```powershell
<project-python> scripts/check_documentation.py
<project-python> scripts/verify_repository.py
<project-python> scripts/validate_schemas.py
```

Expected result: PASS. No semantic schema or contract file is changed by Package B.

### Task 8: Verify the package, inspect scope, and publish for independent review

**Files:**
- Verify: all modified/created files in this plan

- [ ] **Step 1: Run the project interpreter negative and positive paths.**

Run `python scripts/bootstrap.py` with the known wrong global Python and record the expected nonzero rejection without `.venv` mutation. Then run the exact project selector and verify `.venv` creation, strict doctor, and direct checks through the project executable. Do not report the intentional wrong-global negative as an overall gate failure.

- [ ] **Step 2: Run the requested local verification commands.**

Execute the exact project-Python commands for doctor, full Python tests, fast/integration/certification, Ruff, mypy, documentation, repository checks, and `git diff --check`; execute the requested native Rust format/check/clippy/test commands separately. Record only actual successful executions as `PASS`; unavailable or blocked commands remain `NOT_RUN`/`BLOCKED`.

- [ ] **Step 3: Inspect the final diff and ownership boundaries.**

Confirm no changes to `Cargo.lock`, semantic contracts, schemas, generated contract outputs, Magic rules/cards/decks, RNG, replay/checkpoint/digest semantics, or Package C/D tooling. Confirm normal subprocess limits remain at 600 seconds, no per-test timeout dependency was added, and no new long-running normal test exists.

- [ ] **Step 4: Commit and push the exact reviewed head.**

Use a focused commit such as `chore(maintainers): harden reproducible developer setup`, push `chris/130-b-reproducible-developer-setup`, open a PR titled `chore(maintainers): harden reproducible developer setup` referencing `Issue #130` and `Package B` without `Fixes #130`, and do not merge.

- [ ] **Step 5: Verify hosted exact-head evidence and stop for independent review.**

Wait for actual success on `PR Fast`, `PR Integration`, `windows-setup-smoke`, `manafold-pr-gate`, `Analyze (actions)`, `Analyze (python)`, `Analyze (rust)`, and `CodeQL` at the final PR SHA. Verify `manafold-pr-gate` remains the sole required branch-protection check. Report `INDEPENDENT_EXACT_SHA_REVIEW = PENDING` and `MERGE = NOT_PERFORMED`.

## Self-review

- [x] The plan preserves Python `3.13.15`, Rust `1.85.1`, existing lock ownership, branch protection, and the `600` second gate limit.
- [x] The plan covers deterministic tool selection, bounded bootstrap/doctor probes, native Windows and WSL/Linux paths, current `just` shell semantics, exact-head Windows evidence, aggregate fail-closed cases, and the requested negative tests.
- [x] The plan excludes M3, semantic engine work, Package C/D audit/pinning work, per-test timeout dependencies, and direct branch-protection mutation.
- [x] The plan leaves historical evidence immutable and adds no project-status source.
