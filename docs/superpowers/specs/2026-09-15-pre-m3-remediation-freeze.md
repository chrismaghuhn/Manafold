# Pre-M3 Remediation Freeze

**Status:** evidence record

**Date:** 2026-09-15

**Task:** `PRE_M3_REMEDIATION_FREEZE`

**Issue:** https://github.com/chrismaghuhn/Manafold/issues/164

**Base:** `f11ac945e5e221161e57b1a7ae4b38a4d00f8aff`

**Exact foundation head:** `f11ac945e5e221161e57b1a7ae4b38a4d00f8aff`

**Remote verification:** `git ls-remote origin refs/heads/master` returned
`f11ac945e5e221161e57b1a7ae4b38a4d00f8aff`.

**Evidence-head rule:** the final freeze evidence commit is reported by the
review record and final handoff. It is intentionally not embedded in this
source-tree report, because that would make the report self-referential.

## Freeze result

```text
FINAL_FOUNDATION_CLOSURE = PASS
CANONICAL_FINDINGS_TOTAL = 53
CANONICAL_FINDINGS_DISPOSITIONED = 53
FND_TOTAL = 32
FND_DISPOSITIONED = 32
EVD_TOTAL = 15
EVD_DISPOSITIONED = 15
HRD_TOTAL = 6
HRD_DISPOSITIONED = 6
OPEN_P0_BLOCKERS = 0
OPEN_P1_BLOCKERS = 0
REQUIRED_P2_BLOCKERS = 0
DEFERRED_NONBLOCKING_ITEMS = FND-026C, HRD-006
BLOCKED_NONBLOCKING_ITEMS = FND-026B
PRE_M3_REMEDIATION_FREEZE = PASS
FOUNDATION_READY_FOR_M3 = YES
M3_STARTED = NO
M3_AUTHORIZED = NO
ISSUE_164 = OPEN
```

The authoritative 53-row disposition is
[`2026-09-15-final-foundation-closure.md`](2026-09-15-final-foundation-closure.md).
It is present on the merged master tree and retains the two approved
provenance corrections for FND-006 and FND-028.

The closure policy is preserved exactly:

```text
FND-026B = BLOCKED_CONTRACT_AMBIGUITY
FND-026B_FREEZE_READINESS = NONBLOCKING_PROVISIONAL_POLICY
FND-026B_MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = NO
FND-026C = DEFERRED_P2
HRD-006 = DEFERRED_P2
```

No queue, mailbox, poll API, callback, controller-global hidden state, or
neutral non-actor `PlayerStep` meaning was introduced.

## Closure identity and no-drift proof

The Final Foundation Closure was reviewed and merged by PR #175:

```text
FINAL_FOUNDATION_CLOSURE_REPORT_COMMIT = 10e7ce83a14f60b2af131f1e080b907332020e64
FINAL_FOUNDATION_CLOSURE_PR = 175
FINAL_FOUNDATION_CLOSURE_REVIEW = APPROVE
FINAL_FOUNDATION_CLOSURE_MERGE = f11ac945e5e221161e57b1a7ae4b38a4d00f8aff
MERGED_MASTER = f11ac945e5e221161e57b1a7ae4b38a4d00f8aff
PR_175_HOSTED_CI = PASS
```

The merged master tree is identical to the reviewed closure tree:

```text
REVIEWED_CLOSURE_TREE = 11a095475632477210ff9089f848c74613babe1c
FOUNDATION_HEAD_TREE = 11a095475632477210ff9089f848c74613babe1c
10e7ce83..f11ac945 = NO_TREE_DRIFT
```

The closure code/evidence head remains the code identity captured by the
closure report:

```text
CODE_VERIFICATION_HEAD = f271dcc5be82910e7edf5203b9fe1f2bf91f8358
EVIDENCE_INPUT_HEAD = f271dcc5be82910e7edf5203b9fe1f2bf91f8358
f271dcc5..f11ac945 = closure report and register only
```

The bounded change-aware review covered the recorded remediation chain
`AUDIT_BASE -> Batch A -> Batch B -> Batch C -> Batch D -> Batch E -> Batch F
-> Batch G -> Batch H -> ADR 0050 -> current master`. It used the exact-tree
comparison and the current executable gates below; it did not restart the
aborted broad Issue #105 audit. No new concrete correctness, privacy,
determinism, replay, identity, schema, wire, checkpoint, RNG, or conformance
defect was introduced by the merge from the reviewed closure head to current
master.

## Frozen contract identities

This task records the identities already authoritative at the exact foundation
head. It introduces no new protocol or version.

| Surface | Frozen identity or rule | Authority / owner |
|---|---|---|
| Public player/replay wire | Canonical compact UTF-8 JSON; sorted keys; canonical decimal IDs; lowercase SHA-256 hex; padded standard Base64; duplicate and unknown-field rejection | `docs/contracts/WIRE_CONTRACT.md`; `mtgml-wire` |
| Persisted semantic codec | `mtgml.canonical-cbor.v1` | ADR 0038; `mtgml-persistence` |
| Digest envelope | `mtgml.digest-envelope.v1`; algorithm `sha-256` | ADR 0038; `docs/STATE_HASHING.md` |
| Full-state digest | Domain `mtgml.full-state-digest.v3`; input schema `full-state-digest-input.v3`; payload codec `mtgml.canonical-cbor.v1` | ADR 0038; `mtgml-state` / `mtgml-persistence` |
| Checkpoint digest | Domain `mtgml.checkpoint-digest.v3`; input schema `environment-checkpoint-digest-input.v3`; payload codec `mtgml.canonical-cbor.v1` | ADR 0038; `mtgml-environment` / `mtgml-persistence` |
| Information-state digest | Domain `mtgml.information-state-digest.v2`; input schema `information-state-digest-input.v2`; canonical public JSON | ADR 0040; `mtgml-wire` / `mtgml-observation` |
| Observation | `observation-envelope.v1`; current payload identity `synthetic-m2-observation.v1` | ADR 0048; `mtgml-observation` |
| Information state | `information-state-envelope.v2` | ADR 0040; `mtgml-observation` / `mtgml-wire` |
| Decision request/response | `player-decision-request.v2`; `decision-response.v2` | ADR 0039/0040; `mtgml-decision` / `mtgml-wire` |
| Observed event and PlayerStep | `observed-event-envelope.v2`; `player-step.v2` | ADR 0040; `mtgml-observation` / `mtgml-wire` |
| Replay | `replay-manifest.v3`; `authoritative-replay.v3`; `replay-step.v3` | `docs/REPLAY_AND_DETERMINISM.md`; `mtgml-replay` |
| Episode status | `episode-status.v1` | `mtgml-environment` / `mtgml-wire` |
| RNG | `mtgml.rng.v1`; `hmac-sha256-counter.v1`; typed stream keys; big-endian `u64` lanes; unbiased rejection sampling; descending Fisher-Yates | ADR 0035; `mtgml-random` |
| Historical compatibility | V1/V2 values retain their original meaning; historical compatibility source `a4e769eb940611d34df05fc79effd9430891d897` remains immutable evidence | ADR 0048; `docs/REPLAY_AND_DETERMINISM.md` |
| Foundation policy decisions | ADR 0030, 0033, 0035, 0036, 0038, 0039, 0040, 0041, 0048, 0049, and 0050 are accepted at this head | `docs/adr/README.md` |

The contract identities above are corroborated by the exact M2.Final runner,
wire/schema fixtures, persistence known answers, checkpoint/full-state digest
tests, replay parity, RNG tests, and the source-structure/scope guards listed
in the gate record. A contract document is not used as a substitute for those
executions.

## Exact-head executable evidence

The code, semantic, profile, and repository PASS entries below were executed
with the foundation code at
`f11ac945e5e221161e57b1a7ae4b38a4d00f8aff` before this freeze report was
committed. Documentation, maintainer-artifact, register, contract-drift, and
whitespace checks were also rerun after the report was added and before the
evidence commit. The final post-commit archive and documentation checks are
part of the external exact-head review evidence and are not self-referenced
here.

| Gate | Result | Evidence |
|---|---|---|
| Rust format | `PASS` | `cargo fmt --all -- --check` |
| Rust workspace check | `PASS` | `cargo check --workspace --all-targets --all-features --locked` |
| Rust Clippy | `PASS` | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` |
| Rust workspace tests | `PASS` | `cargo test --workspace --all-features --locked`; 490 tests passed, no failures; doc tests passed |
| Python full profile | `PASS` | `C:\Python313\python.exe scripts/run_python_tests.py --profile full`; 398 passed, 3 known adapter-binary skips |
| Ruff format | `PASS` | `.venv\Scripts\python.exe -m ruff format --check python scripts`; 82 files formatted |
| Ruff lint | `PASS` | `.venv\Scripts\python.exe -m ruff check python scripts` |
| Mypy | `PASS` | `.venv\Scripts\python.exe -m mypy --config-file python\pyproject.toml`; 17 source files |
| Generated contracts | `PASS` | `.venv\Scripts\python.exe scripts\generate_contracts.py --check` |
| Repository verification | `PASS` | `.venv\Scripts\python.exe scripts\verify_repository.py`; 689 files, 38 golden, 46 negative fixtures |
| Rust source structure | `PASS` | `.venv\Scripts\python.exe scripts\check_rust_source_structure.py`; 138 files |
| Documentation | `PASS` | `.venv\Scripts\python.exe scripts\check_documentation.py`; 45 ADRs and local links |
| Schemas and fixtures | `PASS` | `.venv\Scripts\python.exe scripts\validate_schemas.py`; 38 wire fixtures and 9 maintainer artifacts |
| Maintainer artifacts | `PASS` | `.venv\Scripts\python.exe scripts\validate_maintainer_artifacts.py`; 7 artifacts |
| Golden path | `PASS` | `.venv\Scripts\python.exe scripts\validate_golden_path.py` |
| Python toolchain | `PASS` | `.venv\Scripts\python.exe scripts\verify_python_toolchain.py`; Python 3.13.15, Rust 1.85.1, 6 exact tool pins |
| Direct fast profile | `PASS` | `.venv\Scripts\python.exe scripts\run_checks.py fast` |
| Direct integration profile | `PASS` | `.venv\Scripts\python.exe scripts\run_checks.py integration` |
| Direct certification profile | `PASS` | `.venv\Scripts\python.exe scripts\run_checks.py certification` |
| M2.G | `PASS` | `.venv\Scripts\python.exe scripts\run_m2_g_gates.py --expect-commit f11ac945e5e221161e57b1a7ae4b38a4d00f8aff`; all 6 gates and source identity |
| M2.Final | `PASS` | `.venv\Scripts\python.exe scripts\run_m2_final_closure.py --expect-commit f11ac945e5e221161e57b1a7ae4b38a4d00f8aff`; all 20 gates, M1 regression, scope guard, certification, and source identity |
| Repository verification runner | `PASS` | `.venv\Scripts\python.exe scripts\run_verification.py`; 20/20 gates, freeze PASS, source tree unchanged |
| Foundation archive baseline | `PASS` | `.venv\Scripts\python.exe scripts\verify_archive_reproducibility.py`; 689 safe files, deterministic archive |
| Whitespace | `PASS` | `git diff --check` |

The full profile's three skips are the known adapter-binary-dependent M2.H
scenario modules. They remain explicitly unrun evidence for the deferred
adapter completeness item and do not become a hidden PASS. The core and
information-safety gates that are required for this foundation claim passed;
HRD-006 remains `DEFERRED_P2`.

## Wrapper status

The repository `just` entry points were invoked separately. Every wrapper was
`BLOCKED` before its recipe could execute because this Windows environment
cannot start the configured WSL `/bin/bash`:

```text
just doctor = BLOCKED
just check-fast = BLOCKED
just check = BLOCKED
just check-all = BLOCKED
just release-candidate = BLOCKED
just archive-check = BLOCKED
reason = WSL CreateProcessCommon:818: execvpe(/bin/bash) failed: No such file or directory
```

These wrapper results are not reported as PASS. The direct native equivalents
above are the executable evidence for the foundation claim, and the wrapper
limitation remains an environment/tooling limitation rather than a semantic
or source defect.

## Scope and authorization boundary

This freeze record contains no production, schema, fixture, protocol, card,
capability, or test changes. It does not add a new public version or resolve a
deferred contract. It records the exact post-remediation foundation head and
the evidence required to keep that head stable.

```text
NEW_MAGIC_SEMANTICS = NO
CARD_IR_CHANGE = NO
COMMANDER_IMPLEMENTATION = NO
FND-026B_IMPLEMENTATION = NO
FND-026C_IMPLEMENTATION = NO
HRD-006_IMPLEMENTATION = NO
BROAD_ISSUE_105_AUDIT_RESTART = NO
UNRELATED_REFACTOR = NO
```

This is a freeze pass, not an M3 entry decision. A separate reviewed decision
is still required before M3 work begins or M3 is authorized.

```text
PRE_M3_REMEDIATION_FREEZE = PASS
FOUNDATION_READY_FOR_M3 = YES
M3_STARTED = NO
M3_AUTHORIZED = NO
HOSTED_CI = PENDING
PRE_M3_FREEZE_REVIEW = PENDING
ISSUE_164 = OPEN
```
