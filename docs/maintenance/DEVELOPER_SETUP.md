# Developer Setup

**Status:** accepted maintainer process

This document owns the reproducible local setup path. Current project and
milestone status remains owned by the repository root [`README.md`](../../README.md)
and the ordering in [`ROADMAP.md`](../ROADMAP.md).

## Required tools

Install these tools on the host before bootstrapping:

- Git;
- the Python 3.13 interpreter selected by the Python Launcher on Windows, or
  a `python3.13` interpreter on WSL/Linux;
- Rust through `rustup`.

The repository pins Python to `3.13.15` in [`.python-version`](../../.python-version)
and Rust to `1.85.1` with `rustfmt` and Clippy in
[`rust-toolchain.toml`](../../rust-toolchain.toml). Do not install another
environment manager for this path. `doctor` does not install or update either
toolchain.

## Native Windows

PowerShell and `cmd.exe` users can use the direct executable path; Bash is not
required for this golden path. From the repository root, first verify the
launcher-selected interpreter:

```powershell
py -3.13 --version
```

It must print `Python 3.13.15`. Then create or update the project environment:

```powershell
py -3.13 scripts/bootstrap.py
.venv\Scripts\python.exe scripts/doctor.py --strict
.venv\Scripts\python.exe scripts/run_checks.py fast
.venv\Scripts\python.exe scripts/run_checks.py integration
.venv\Scripts\python.exe scripts/run_checks.py certification
```

`bootstrap.py` performs the exact patch-version check itself. If `py -3.13`
selects a different patch version, bootstrap fails before changing `.venv`.
Install the required Python `3.13.15` interpreter and rerun the same command;
do not widen `.python-version` and do not rely on whichever `python.exe` is
first on `PATH`.

The canonical Windows executable after bootstrap is:

```text
.venv\Scripts\python.exe
```

Activation is optional. Direct invocation above is the correctness authority.

## WSL / Linux

From a Bash shell at the repository root, verify the interpreter and bootstrap:

```bash
python3.13 --version
python3.13 scripts/bootstrap.py
.venv/bin/python scripts/doctor.py --strict
.venv/bin/python scripts/run_checks.py fast
.venv/bin/python scripts/run_checks.py integration
.venv/bin/python scripts/run_checks.py certification
```

The version command must report `3.13.15`; the bootstrap exact check remains
authoritative if the host command selects another patch. The canonical POSIX
executable is:

```text
.venv/bin/python
```

## Python tool selection

Ruff, Mypy, pytest, and schema tooling belong to the project environment. The
normal checks invoke Python-owned tools through the selected interpreter (for
example, `python -m ruff`), so a global `ruff`, `mypy`, or `pytest` executable
cannot satisfy the project verification path. `run_checks.py` also rejects a
global Python even when its version is exactly `3.13.15`; its interpreter must
be the repository `.venv` executable path. Use the canonical `.venv` executable
directly; shell activation is only a convenience.

## Shell and `just`

The current `justfile` explicitly declares:

```text
set shell := ["bash", "-euo", "pipefail", "-c"]
```

Therefore the Bash-oriented `just doctor`, `just bootstrap`, `just check-fast`,
`just check`, and `just check-all` recipes are supported on WSL/Linux. After
bootstrap, the verification recipes use `.venv/bin/python`, so `just` cannot
silently substitute an exact-version global Python. Native
Windows PowerShell and `cmd.exe` users do not need Bash for the direct Python
and Cargo commands documented above. The repository does not claim a separate
Git Bash compatibility path; use WSL/Linux for the documented Bash-oriented
`just` path.

## Doctor and bootstrap behavior

`doctor` is diagnostic and non-mutating. It reports the repository root,
platform and shell facts, the running Python executable/version, the required
Python and Rust identities, project `.venv` identity, required Python modules,
project-owned Python tools, native Rust tools, rustup components, and
`Cargo.lock`. Its version probes are bounded to 30 seconds. `doctor --strict`
returns nonzero for a required mismatch; plain `doctor` reports problems for
diagnosis.

`bootstrap.py` is the only setup command in this path that mutates the
project-local `.venv`. Its dependency subprocesses have a 600-second maximum,
fail on installation errors or timeout, and print the failed command and a
rerun diagnostic. It does not install or update Rust and does not modify
`Cargo.lock`, `.python-version`, `rust-toolchain.toml`, or
`python/requirements-dev.lock`.

If the project environment is stale or broken, confirm that `.venv` is the
repository-local environment, remove or rename only that directory, and rerun
the exact selector command. For example, after checking the current directory
in PowerShell:

```powershell
Remove-Item -LiteralPath .venv -Recurse -Force
py -3.13 scripts/bootstrap.py
```

On WSL/Linux, the equivalent recovery is:

```bash
rm -rf -- .venv
python3.13 scripts/bootstrap.py
```

Do not remove a parent directory or a shared environment as part of recovery.

## Pull-request evidence

The hosted `windows-setup-smoke` job checks out the exact pull-request head,
asserts that SHA, verifies a fresh checkout and pinned Python/Rust setup, runs
bootstrap, then runs project `.venv` doctor and the fast smoke. The stable
`manafold-pr-gate` aggregate requires that Windows check in addition to the
existing `PR Fast`, `PR Integration`, and analyzer/CodeQL evidence. Branch
protection continues to require only `manafold-pr-gate`.
