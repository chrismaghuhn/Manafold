# Maintainer Playbook

**Status:** accepted process index  
**Last reviewed:** 2026-09-13

## Before changing anything

1. Identify the semantic surface and its stability class.
2. Read the relevant normative docs and ADRs.
3. Check [`OPEN_DECISIONS.md`](OPEN_DECISIONS.md) and the design lock matrix.
4. Pin authority/input artifacts.
5. State the support claim and exact non-goals.
6. Add red evidence before changing behavior.

## Architecture or contract change

- create an ADR;
- update every Rust/Python/schema/fixture representation together;
- classify compatibility/migration;
- update the normative register if documents/surfaces change;
- run direct and transitive capability/leak checks;
- preserve old fixtures/artifacts where compatibility requires it.

## Adding a mechanic

Follow [`rules/ADDING_RULES_AND_MECHANICS.md`](rules/ADDING_RULES_AND_MECHANICS.md). Use `scripts/scaffold_capability.py`, specify authority/state/events/decisions/information/order, add red conformance cases, implement reusable primitives, then advance lifecycle only with evidence.

## Adding a card

Follow [`cards/ADDING_CARDS.md`](cards/ADDING_CARDS.md). The current implementation uses `scripts/scaffold_card.py`; the long-term golden path is intended to converge on a one-command scaffold and capability-gap analysis workflow described in the card guide. Pin provenance, review generated IR, declare capabilities, test decisions/information/interactions, and certify only through a locked bundle.

## Changing a deck or bundle

Generate a scope-impact report. Recompute reachable definitions and recursive capability closure. Invalidate or supersede affected certification. Never replace a deck entry while retaining the previous bundle digest or claim.

## Hidden-information change

- enumerate each authorized perspective;
- specify knowledge gain/retention/invalidation;
- specify opaque identity lifecycle;
- add paired-state noninterference cases;
- review errors, events, candidate order, counts, and trajectory metadata for leaks.

## Replay/determinism change

- identify digest/schema/algorithm impact;
- add golden and negative fixtures;
- run repeated reset/step/checkpoint/fork/replay comparisons;
- prove no wall-clock/thread/map-order dependency;
- publish migration or new version rather than reinterpret old artifacts.

## Performance change

Preserve semantic parity first. Benchmark a pinned workload and report distributions, memory, and raw evidence. An optimization without reference-backend parity is not accepted.

## Release

Follow [`maintenance/RELEASE_PROCESS.md`](maintenance/RELEASE_PROCESS.md) and [`maintenance/FREEZE_LEVELS.md`](maintenance/FREEZE_LEVELS.md). Generate reports from executed commands; do not edit gate status manually.

## Deterministic failure capture and rerun

Package D provides an explicit, opt-in maintainer workflow for one bounded
command failure. `run_checks.py` does not invoke capture or rerun
automatically. Use the project Python directly so the command and its
provenance remain visible:

On Windows PowerShell:

```powershell
.venv\Scripts\python.exe -B scripts/capture_failure.py --case-id CASE -- .\.venv\Scripts\python.exe -B -c "raise SystemExit(1)"
.venv\Scripts\python.exe -B scripts/rerun_failure.py dist\failures\<packet-id>
```

On WSL/Linux:

```bash
.venv/bin/python -B scripts/capture_failure.py --case-id CASE -- .venv/bin/python -B -c 'raise SystemExit(1)'
.venv/bin/python -B scripts/rerun_failure.py dist/failures/<packet-id>
```

The default packet root is `dist/failures/`, owned by
`.mtgml-failure-output`. An existing unowned root is rejected. Capture
validates argv, the output root, and a clean source baseline before starting
the child. The child receives the exact argv list with `shell=False` and a
hard limit of 600 seconds. A missing executable, unsafe output root,
malformed signature marker, source mutation, or write failure is
`CAPTURE_BLOCKED` with no completed packet. A passing command is
`CAPTURE_PASS` with no packet. A nonzero exit preserves its original exit
status in a trusted packet; a timeout is recorded separately as
`COMMAND_TIMEOUT`.

Capture statuses and exit codes are:

| Status | Exit | Meaning |
| --- | ---: | --- |
| `CAPTURE_PASS` | 0 | Command exited zero and the source identity stayed unchanged; no packet is written. |
| `CAPTURE_COMMAND_EXIT` | original nonzero | Command exited nonzero and the trusted packet preserves that exit status. |
| `CAPTURE_TIMEOUT` | 124 | Command reached the 600-second limit and the packet records `COMMAND_TIMEOUT`. |
| `CAPTURE_BLOCKED` | 2 | Capture preconditions, marker parsing, source integrity, output safety, or packet writing failed; no completed packet exists. |

The rerunner validates the packet and log checksum, commit, Git tree, source
fingerprint, clean status, required capture-Python version, repository-
relative cwd, and argv before execution. The current `HEAD` must match the
recorded commit and its recorded Git tree. It never calls `fetch`, `checkout`,
`reset`, `clean`, or `stash`. Its statuses and exit codes are:

| Status | Exit | Meaning |
| --- | ---: | --- |
| `REPRODUCED` | 0 | Exact source preconditions and the declared failure predicate matched. |
| `NOT_REPRODUCED` | 1 | Execution was valid, but the exit, timeout, or structured signature differed. |
| `BLOCKED` | 2 | A precondition, marker, checksum, tool, or source-integrity check prevented valid comparison. |

`manafold.failure-packet.v1` is trusted local maintainer evidence, not a
public CI artifact, player diagnostic, ML field, replay authority, checkpoint
authority, or semantic fixture. It is a small command-level reproducer and
is not the complete ADR-0036 Portable Reproduction Bundle: it does not
capture a complete deterministic start point, checkpoint, response/replay
segment, content identity, authority identity, or RNG identity. Do not use
the standard path for commands known to emit raw seed material or other
private diagnostics.

Promotion remains human-reviewed:

```text
captured → reproduced → minimized/reviewed → expected semantics reviewed
→ RED regression → implementation → regression PASS
```

The tooling never copies actual output into expected output, edits golden
fixtures, promotes a failure automatically, or creates a support claim.
