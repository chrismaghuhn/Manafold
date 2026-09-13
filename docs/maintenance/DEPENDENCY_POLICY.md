# Dependency Policy

**Status:** accepted dependency policy  
**Stability:** normative


Prefer standard library and small focused dependencies in semantic hot paths.
Record purpose, trust boundary, and replacement cost. Lock dependencies; do not
auto-merge updates. Parser, codec, FFI, network, and source-fetch dependencies
are untrusted-input boundaries. Avoid dependencies that embed rules/card data or
global mutable state. Benchmark representation/hot-path changes.

Foundation CI may use reviewed vendor major tags; certified releases pin GitHub
Actions to reviewed full commit SHAs and use Dependabot for proposed updates.


The exact distinction between direct development pins, transitive locks, the reference interpreter/compiler, and public release build images is defined in [`TOOLCHAIN_POLICY.md`](TOOLCHAIN_POLICY.md). A filename containing `lock` is not by itself evidence of a fully resolved or hash-locked environment.

## Update and audit ownership

Dependabot remains the proposal mechanism for the `github-actions`, `cargo`,
and `pip` ecosystems. An Action update follows:

```text
Dependabot proposes a new Action digest/SHA
→ maintainer reviews the upstream version and change
→ exact-head repository gates execute
→ master protection and manafold-pr-gate accept the merge
```

Dependency updates follow the same review boundary:

```text
Dependabot or an explicit maintainer PR
→ security and compatibility impact review
→ normal exact-head gates
→ semantic/ADR review when contracts may be affected
→ merge
```

Changes that affect RNG, canonical codecs, replay, checkpoints, hashing, or
wire contracts still follow the existing compatibility and ADR process. A
security update does not bypass semantic compatibility.

## Dependency audit path

`<project-python> scripts/run_dependency_audit.py` (or the Bash-oriented
`just audit-dependencies` recipe after project bootstrap) is the explicit
read-only audit path. It runs RustSec `cargo-audit` 0.22.2 against `Cargo.lock`
and PyPA `pip-audit` 2.10.1 against `python/requirements-dev.lock`. The audit
tools are installed separately from ordinary bootstrap. `cargo-audit` is built
with audit-only Rust 1.88.0; Manafold itself remains on reference Rust 1.85.1.
Normal Fast/Integration gates do not download advisory databases.

Each ecosystem reports `PASS`, `FAIL`, or `BLOCKED`. A missing audit tool,
advisory service, or bounded subprocess produces `BLOCKED`, never `PASS`.
The audit workflow is scheduled/manual and is intentionally not a prerequisite
of `manafold-pr-gate`, because advisory-service availability is separate from
merge correctness.
