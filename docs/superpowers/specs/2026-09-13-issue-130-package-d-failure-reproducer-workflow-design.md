# Issue #130 Package D — Deterministic Failure / Reproducer Workflow

**Status:** design v2 after review; implementation remains unauthorized by this document

**Issue:** [#130 — Maintainer hardening](https://github.com/chrismaghuhn/Manafold/issues/130)

**Starting source:** `15f339cd0ca875a9f18a44c743796120ed87ade7`

**Branch:** `chris/130-d-failure-reproducer-workflow`

## 1. Goal

Package D makes deterministic foundation failures easier to diagnose and
reproduce:

```text
failure
→ first useful typed difference
→ trusted generated packet
→ exact rerun command
→ exact-baseline rerun
→ REPRODUCED / NOT_REPRODUCED / BLOCKED
→ reviewed regression evidence
```

The package adds maintainer ergonomics. It does not add Magic semantics,
semantic authority, player diagnostics, a public artifact, or an M3 TestKit.

The Package-D packet is deliberately not the complete ADR-0036 portable
reproduction bundle. It is a smaller command-level maintainer reproducer. It
does not provide a complete deterministic start point, a response/replay
segment, a complete checkpoint, or automatic content, authority, or RNG
identity capture. A packet without those artifacts must not be presented as a
portable engine reproduction or as replay evidence.

## 2. Preconditions and boundaries

Package C is complete at the starting source through merged PR #158. The
pytest 8.4.1 advisory remains a separately owned remediation. Package D does
not change pytest, either lockfile, Rust toolchains, audit policy, or
dependency versions.

The current checkout is not used for implementation. The implementation
branch starts from the verified `origin/master` commit above in an isolated
worktree. No change touches the user's dirty `master` checkout.

Package D does not:

- implement M3, Magic rules, cards, decks, or a second rules engine;
- add `mtgml-diagnostics` or another Rust crate;
- add a public schema, REST endpoint, MCP resource, or player API;
- add a public or ML-facing failure dataset;
- capture root seeds, complete checkpoints, hidden zones, or arbitrary process
  environments;
- add a generic reflection, Serde, tracing, fuzzing, or minimization framework;
- rewrite expectations, golden fixtures, support claims, issues, or capability
  lifecycle state.

The accepted contracts in `docs/DEBUG_ARCHITECTURE_CONTRACT.md`,
`docs/OBSERVABILITY_AND_DEBUGGING.md`, ADR 0036, and the repository's
information-safety and deterministic-replay documents remain authoritative.

## 3. Ownership and data flow

The package uses the existing trusted conformance owner and a rules-free
maintainer script boundary:

```text
mtgml-conformance
  typed first-difference diagnosis
          ↓ human-readable failure output
scripts/failure_packet.py
  provenance, packet validation, checksums, source identity
          ↓
scripts/capture_failure.py / scripts/rerun_failure.py
  bounded command orchestration
```

The Rust side compares typed values. The Python side runs commands and
packages evidence. Python never interprets legality, state, RNG, events, or
expected semantics. The normal `scripts/run_checks.py` path remains
unchanged; capture is opt-in.

No player endpoint, model-facing DTO, trajectory, or public wire surface can
reach the trusted diagnostic types or packet files.

## 4. Structured first difference

`crates/mtgml-conformance/src/diagnostics.rs` owns the repository-private
diagnostic types. The module remains part of `mtgml-conformance`; it is not a
new crate and is not a public wire contract.

The concrete shape is:

```rust
ConformanceDifference {
    surface: ConformanceSurface,
    semantic_path: String,
    mismatch_kind: ConformanceMismatchKind,
    expected_summary: String,
    actual_summary: String,
    sequence: Option<SequenceDifference>,
}

SequenceDifference {
    first_differing_index: usize,
    expected_length: usize,
    actual_length: usize,
}
```

`ConformanceMismatchKind` includes:

```text
VALUE_CHANGED
EXPECTED_ENTRY_MISSING
UNEXPECTED_EXTRA_ENTRY
PLAYER_MISSING
UNEXPECTED_PLAYER
PLAYER_STEP_DIFFERED
REJECTED_MUTATION
```

`ConformanceFailure` retains its existing coarse variants and gains an
additive detailed variant carrying a coarse classification and one
`ConformanceDifference`. Existing classification access remains available;
the detailed variant improves the default assertion's rendered failure
without changing contract-error meaning.

The exact comparison order is:

```text
current decision
submitted response
transition contract
acceptance
authoritative events
semantic delta audit
resulting state digest
next decision
episode status
player projections
rejected-mutation invariant
```

Contract validation is executed as its own stage. A
`validate_transition_contract()` error remains `Contract(...)`; it is never
converted into expected-vs-actual evidence.

Ordered events and delta audit operations compare from index zero. The first
unequal pair is `VALUE_CHANGED`; a shorter actual sequence is
`EXPECTED_ENTRY_MISSING`; a longer actual sequence is
`UNEXPECTED_EXTRA_ENTRY`. Paths use the owning surface, for example
`transition.events[3]` and `transition.delta.audit[1]`.

Player projections compare the deterministic `BTreeMap<PlayerId, ...>` union.
The first missing expected player, unexpected actual player, or differing
player step is reported with a path such as `player_steps[player:1]`. The
diagnostic does not recursively serialize or expose another player's hidden
state.

Scalar surfaces report a typed surface, a semantic path, and human summaries.
Summaries are ephemeral diagnostic text. They are not failure identity, are
not hashed, and do not determine rerun status. They may use a small typed
human renderer, but no arbitrary Rust serialization is a diagnostic contract.

Failure identity uses structured fields only:

```text
origin kind + case ID + failure classification
+ optional surface + semantic path + mismatch kind
```

The assertion is observationally inert: it does not repair state, choose a
decision, consume RNG, allocate an identity, reorder candidates/events, or
alter any authoritative output.

## 5. Internal failure-packet contract

The packet is an internal, trusted, experimental maintainer artifact named
`manafold.failure-packet.v1`. It is not added to `schemas/`, the public wire
contract, replay, checkpoint, or ML data model.

The packet contains `manifest.json` and a captured `command.log`. Its required
manifest fields are:

```text
format = manafold.failure-packet.v1
packet_id
sensitivity
origin.kind
origin.case_id
source.commit
source.tree
source.fingerprint
source.clean
command.argv
command.cwd
execution.outcome
execution.exit_status
execution.timeout_seconds
failure.classification
failure_signature
artifacts.log.path
artifacts.log.sha256
tools
reproduction.display_command (human-facing only)
```

The three source identities have distinct meanings:

```text
source.commit
  git revision at capture time

source.tree
  Git tree object named by that commit

source.fingerprint
  SHA-256 over Manafold's relevant source files, using the existing archive
  `source_files()` set and its generated/output/cache exclusions
```

The packet records `command.argv` as a list and `command.cwd` as a
repository-relative path. These are the only machine-authoritative
reexecution inputs. The `reproduction` field may contain a display string
such as `<project-python> scripts/rerun_failure.py <packet>`; no runner ever
executes placeholders from that field.

`failure_signature` contains the structured diagnostic identity. For generic
commands it contains the origin, case, and command-failure classification.
The Package-D command path records this generic signature; the Rust
first-difference detail remains in its typed failure and captured log. A
trusted harness may also emit one explicit internal marker using the reviewed
signature fields:

```text
MANAFOLD_FAILURE_SIGNATURE v1 surface=events path=transition.events[3] mismatch_kind=value_changed
```

Capture and rerun parse only this exact marker shape. They never derive a
signature by parsing arbitrary exception or `Debug` text. The marker carries
no expected/actual values. If a packet records structured marker fields, a
rerun with the same nonzero process result but different marker fields is
`NOT_REPRODUCED`; a missing or malformed marker is `BLOCKED` because the
structured predicate cannot be compared.

The generic execution predicate is separate from diagnostic identity:

```text
COMMAND_EXIT
  same failure signature AND same nonzero exit_status

COMMAND_TIMEOUT
  same failure signature AND the rerun reaches the declared timeout
```

Therefore exit 1 differs from exit 2, and a timeout differs from an ordinary
nonzero exit. A packet with a structured predicate but no comparable
structured rerun observation produces `BLOCKED`, not a false
`REPRODUCED` result.

Unavailable tool identities are omitted. The packet never contains a complete
environment snapshot, secret material, raw root seed, RNG stream material,
private hand/library, or opaque-ID mapping.

## 6. Packet storage and write safety

The default output root is `dist/failures/`, outside the reproducible source
set. The root is owned by a marker named `.mtgml-failure-output`. An existing
unmarked root is rejected; an existing packet ID is never overwritten.

Packet creation is staged in a temporary sibling directory. The log,
checksum, and complete manifest are written and validated before the staged
directory is atomically renamed to its final packet ID. A failed write never
reports a complete packet.

An output path must resolve either under the accepted generated-output area or
to an explicitly configured external temporary/output root. Repository-root,
repository-source, traversal, and escaping paths are rejected. Packet member
paths are relative and cannot contain `..`, absolute roots, or alternate
streams.

## 7. Capture workflow

The maintainer invokes:

```text
<project-python> scripts/capture_failure.py \
  --case-id SOME_CASE -- <command> <args...>
```

Capture validates the argv, output root, and clean source baseline before
starting the command. It runs the exact argv with `shell=False` and applies
the hard maximum `MAX_SINGLE_REPRO_SUBPROCESS_RUNTIME_SECONDS = 600`.

The results are:

```text
command exits 0 and source remains unchanged
  CAPTURE_PASS; no packet; exit 0

command exits nonzero and source remains unchanged
  CAPTURE_COMMAND_EXIT; trusted packet; original nonzero exit preserved

command reaches the 600-second limit and source remains unchanged
  CAPTURE_TIMEOUT; trusted packet with COMMAND_TIMEOUT; nonzero timeout exit

executable is missing, a precondition fails, output is unsafe, packet writing
fails, or the command mutates source/Git state
  CAPTURE_BLOCKED; no completed packet; exit 2
```

A missing executable is a blocker because no declared execution occurred. The
tool may print an ephemeral `BLOCKED` and `RERUN` diagnostic to stderr, but it
does not create `manafold.failure-packet.v1` for that case.

Capture does not automatically integrate with `run_checks.py`, does not retry,
does not run a command through a remote service, and does not expose a player
or network entry point.

## 8. Deterministic rerun workflow

The maintainer invokes:

```text
<project-python> scripts/rerun_failure.py <failure-packet>
```

Before execution, the rerunner validates:

```text
packet format and required field types
manifest and log locations
log checksum
current HEAD == source.commit
current HEAD tree == source.tree
current source fingerprint == source.fingerprint
current clean-status requirement
referenced files and repository-relative cwd
nonempty argv with no NUL or shell interpretation
```

If any prerequisite fails, the rerunner prints the exact required identity or
rerun command and returns `BLOCKED`. It never calls `checkout`, `reset`,
`clean`, `stash`, `fetch`, or another Git mutator.

The rerunner snapshots source identity before and after execution. Any source
or Git-state mutation is `BLOCKED`, even if the child exits with the recorded
failure. A missing rerun executable is also `BLOCKED`, not
`NOT_REPRODUCED`, because the predicate was not executed.

The result vocabulary is:

```text
REPRODUCED
  exact source preconditions are valid and the declared failure predicate
  is observed; exit 0 from the rerunner

NOT_REPRODUCED
  execution is valid, but the predicate differs; exit 1

BLOCKED
  exact reproduction cannot validly execute or compare; exit 2
```

For example, captured exit 1 followed by rerun exit 2 is
`NOT_REPRODUCED`; captured timeout followed by an ordinary rerun exit 1 is
also `NOT_REPRODUCED`. A rerun timeout for a `COMMAND_TIMEOUT` packet is
`REPRODUCED` only when it reaches the declared timeout condition.

`REPRODUCED` never means that the tested behavior is correct. It means only
that the declared failure occurred again.

## 9. Sensitivity and safe summaries

The trusted packet and its log are separate from a safe summary. The safe
summary renderer constructs output only from an explicit allowlist:

```text
format
packet ID
origin kind and case ID
failure classification
safe signature fields
source commit, when the sink permits it
```

It does not read or render argv, cwd, logs, tool paths, expected/actual
summaries, hidden state, private data, seeds, or trusted IDs. Tests use sentinel
values for those categories and verify that the safe renderer never emits
them. No post-hoc string replacement is the security model.

The capture default is trusted local storage, not public output. The `trusted`
packet classification authorizes restricted maintainer handling; it does not
assert that arbitrary command output is safe or secret-free. The standard
path has no secret-capability option and must not be used for a command known
or expected to emit `secret_seed_material`. Such a command is outside Package
D and remains `BLOCKED` until a separately reviewed secret diagnostic path
exists. No CI workflow uploads full packets in Package D. No player API,
experiment telemetry, or ML dataset gets diagnostic detail or trusted error
strings.

## 10. Regression promotion and minimization

Package D documents this human-reviewed lifecycle:

```text
captured
→ reproduced
→ minimized/reviewed
→ expected semantics independently reviewed
→ RED regression authored
→ fix implemented
→ regression PASS
```

The tooling never copies actual output into expected output, edits golden
fixtures, creates support claims, marks an issue fixed, or certifies a case.
Minimization is manual or uses an existing native shrinker. Package D only
preserves the first difference and may trim output after it; it does not add a
replay reducer, card reducer, state reducer, fuzzing framework, or search
minimizer.

## 11. Planned files

The implementation plan may modify only the following focused areas:

```text
crates/mtgml-conformance/src/diagnostics.rs
crates/mtgml-conformance/src/lib.rs
crates/mtgml-conformance/src/lifecycle.rs
scripts/failure_packet.py
scripts/capture_failure.py
scripts/rerun_failure.py
python/tests/test_failure_packet.py
docs/MAINTAINER_PLAYBOOK.md
docs/maintenance/DEVELOPER_SETUP.md
scripts/README.md
```

The design specification itself is registered as a provisional process
document. No schema, lockfile, dependency, workflow, branch-protection rule,
card, deck, generated contract, or public API changes as part of the design
commit.

## 12. Required evidence

Rust tests use synthetic foundation values and prove every required first
difference, deterministic precedence, sequence classification, player-map
classification, rejected-mutation classification, and contract separation.

Python tests use temporary local repositories and local commands. They cover
capture pass/fail/timeout/blocking behavior, exact commit/tree/fingerprint,
argv preservation, checksums, output safety, source nonmutation, exact-head
reruns, `REPRODUCED`, `NOT_REPRODUCED`, `BLOCKED`, a same-command nonzero rerun
with a different structured signature yielding `NOT_REPRODUCED`, and the
sentinel-based safe-summary boundary. They use no live network.

The implementation must also perform one deliberate synthetic failure
demonstration:

```text
failure → first difference → packet → exact HEAD → emitted rerun
→ same-HEAD REPRODUCED → unchanged source tree
```

Generated demonstration packets are removed after evidence capture. No
intentional production failure remains in the branch.

Local and hosted verification remain separate evidence. The implementation
report must list every requested local gate and exact-head hosted check as
`PASS`, `FAIL`, `NOT_RUN`, or `BLOCKED`, without upgrading unexecuted evidence.
Package C's pytest advisory remains `OUT_OF_SCOPE / TRACKED_SEPARATELY`.

## 13. Acceptance boundary

Package D is complete only when the requested behavior, focused tests,
documentation, source-tree nonmutation, synthetic demonstration, local gates,
and final exact-head hosted evidence have actually run at the required level.
The branch may be pushed and a PR opened after verification, but it must not
merge or close Issue #130. The final report must state
`ISSUE_130_CLOSE_READY = YES | NO` and
`INDEPENDENT_EXACT_SHA_REVIEW = PENDING`.
