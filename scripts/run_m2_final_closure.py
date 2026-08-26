#!/usr/bin/env python3
"""Execute the final M2 closure matrix on one clean exact source head.

This runner owns aggregation only.  It executes the existing M1 and M2.B-H
gate runners as subprocesses against the current ``HEAD``, validates their
authoritative JSON reports, runs the explicit M2 scope guard, and aggregates
exactly twenty gates:

```text
M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT          run_m2_b_contract_cut.py
CLOSED_DECISION_FAMILY_EXACTNESS                run_m2_c_gates.py
SERIALIZED_CONTINUATION_LIFECYCLE               run_m2_c_gates.py
VISIBLE_DECISION_CANONICAL_ORDER_AND_IDENTITY   run_m2_d_gates.py
PLAYER_PROJECTION_PERSPECTIVE_COHERENCE         run_m2_d_gates.py
PLAYER_SAFE_ERROR_MAPPING_AND_NONDISCLOSURE     run_m2_d_gates.py
KNOWLEDGE_RETENTION_INVALIDATION_AND_HISTORY    run_m2_e_gates.py
OPAQUE_ID_DISTINGUISHABILITY_LIFECYCLE          run_m2_e_gates.py
OBSERVED_EVENT_REDACTION_AND_SEQUENCE           run_m2_e_gates.py
SYNTHETIC_LEGAL_CHOICE_SOUNDNESS                run_m2_f_gates.py
SYNTHETIC_LEGAL_CHOICE_COMPLETENESS             run_m2_f_gates.py
M2_PAIRED_STATE_VISIBLE_BYTES_NONINTERFERENCE   run_m2_g_gates.py
M2_MULTI_ENDPOINT_INFORMATION_ISOLATION         run_m2_g_gates.py
M2_REJECTED_RESPONSE_COMPLETE_NONMUTATION       run_m2_g_gates.py
M2_CHECKPOINT_RESTORE_INFORMATION_IDENTITY      run_m2_g_gates.py
M2_FORK_INFORMATION_PARITY                      run_m2_g_gates.py
M2_REPLAY_INFORMATION_PARITY                    run_m2_g_gates.py
M2_RUST_PYTHON_PLAYER_WIRE_PARITY               run_m2_h_gates.py
M2_RULES_FREE_PYTHON_ADAPTER_PARITY             run_m2_h_gates.py
M1_GATE_REGRESSION_AND_M2_SCOPE_GUARD           run_m1_closure.py + local scope guard
```

No individual gate is redefined here and no historical PASS is inherited:
every gate status comes from a report produced by this invocation on this
head.  Authoritative mode requires a clean tracked source tree, an explicit
``--expect-commit`` pin equal to ``HEAD``, and an unchanged source identity
after verification.  Any ``FAIL``, ``BLOCKED``, ``NOT_RUN``, duplicate, or
missing gate registration blocks M2 completion.  ``--development`` executes
the same underlying children in their development modes diagnostically but
can never report completion.

Closure prerequisites: after the child gates and the scope guard, this
runner executes the repository certification profile
(``scripts/run_checks.py certification``), whose final command is the
deterministic source-archive reproducibility gate.  A non-PASS
certification result blocks ``COMPLETE`` exactly like a failed gate; the
normative "archive/reproducibility gate last" ordering is preserved because
the certification profile runs after every other verification step and is
followed only by the final read-only source snapshot and report writing
into gitignored ``dist/``.

Reports and logs are written only below ``dist/m2-final-verification/``
(never into the reproducible source archive).

Scope-guard notes: the vocabulary pattern scans deliberately cover only
``crates/``, ``tools/``, and ``python/src`` production sources.  The
verification tooling itself (this script, sibling gate runners, and their
tests) necessarily contains the literal forbidden vocabulary as detection
patterns, so scanning it would be self-defeating; maintainer-tooling changes
remain guarded by review and the pinned structural inventories.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tomllib
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "dist" / "m2-final-verification"
OUTPUT_MARKER = ".mtgml-m2-final-closure-output"

GATE_M1_REGRESSION = "M1_GATE_REGRESSION_AND_M2_SCOPE_GUARD"

EXPECTED_GATES: tuple[str, ...] = (
    "M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT",
    "CLOSED_DECISION_FAMILY_EXACTNESS",
    "SERIALIZED_CONTINUATION_LIFECYCLE",
    "VISIBLE_DECISION_CANONICAL_ORDER_AND_IDENTITY",
    "PLAYER_PROJECTION_PERSPECTIVE_COHERENCE",
    "PLAYER_SAFE_ERROR_MAPPING_AND_NONDISCLOSURE",
    "KNOWLEDGE_RETENTION_INVALIDATION_AND_HISTORY",
    "OPAQUE_ID_DISTINGUISHABILITY_LIFECYCLE",
    "OBSERVED_EVENT_REDACTION_AND_SEQUENCE",
    "SYNTHETIC_LEGAL_CHOICE_SOUNDNESS",
    "SYNTHETIC_LEGAL_CHOICE_COMPLETENESS",
    "M2_PAIRED_STATE_VISIBLE_BYTES_NONINTERFERENCE",
    "M2_MULTI_ENDPOINT_INFORMATION_ISOLATION",
    "M2_REJECTED_RESPONSE_COMPLETE_NONMUTATION",
    "M2_CHECKPOINT_RESTORE_INFORMATION_IDENTITY",
    "M2_FORK_INFORMATION_PARITY",
    "M2_REPLAY_INFORMATION_PARITY",
    "M2_RUST_PYTHON_PLAYER_WIRE_PARITY",
    "M2_RULES_FREE_PYTHON_ADAPTER_PARITY",
    GATE_M1_REGRESSION,
)

M1_GATE_NAMES: tuple[str, ...] = (
    "ENGINE_STATE_CONSTRUCTION_AND_INVARIANTS",
    "ACCEPTED_TRANSITION_EXACT_PRODUCT",
    "REJECTED_RESPONSE_COMPLETE_NONMUTATION",
    "STATE_DELTA_FULL_REAPPLICATION",
    "SEQUENTIAL_EVENT_DELTA_PARITY",
    "CHECKPOINT_RESTORE_COMPLETE_IDENTITY",
    "FORK_PARITY",
    "REPLAY_PARITY",
    "DETERMINISTIC_RNG_AND_ALLOCATORS",
    "MULTI_PLAYER_ENDPOINT_BINDING",
)


@dataclass(frozen=True)
class ChildRunner:
    slug: str
    script: str
    report_file: str
    gates: tuple[str, ...]
    supports_expect_commit: bool = True
    supports_development_flag: bool = True


CHILD_RUNNERS: tuple[ChildRunner, ...] = (
    ChildRunner(
        "m2-b",
        "scripts/run_m2_b_contract_cut.py",
        "m2-b-verification-results.json",
        ("M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT",),
    ),
    ChildRunner(
        "m2-c",
        "scripts/run_m2_c_gates.py",
        "m2-c-gate-results.json",
        (
            "CLOSED_DECISION_FAMILY_EXACTNESS",
            "SERIALIZED_CONTINUATION_LIFECYCLE",
        ),
    ),
    ChildRunner(
        "m2-d",
        "scripts/run_m2_d_gates.py",
        "m2-d-gate-results.json",
        (
            "VISIBLE_DECISION_CANONICAL_ORDER_AND_IDENTITY",
            "PLAYER_PROJECTION_PERSPECTIVE_COHERENCE",
            "PLAYER_SAFE_ERROR_MAPPING_AND_NONDISCLOSURE",
        ),
    ),
    ChildRunner(
        "m2-e",
        "scripts/run_m2_e_gates.py",
        "m2-e-gate-results.json",
        (
            "KNOWLEDGE_RETENTION_INVALIDATION_AND_HISTORY",
            "OPAQUE_ID_DISTINGUISHABILITY_LIFECYCLE",
            "OBSERVED_EVENT_REDACTION_AND_SEQUENCE",
        ),
    ),
    ChildRunner(
        "m2-f",
        "scripts/run_m2_f_gates.py",
        "m2-f-gate-results.json",
        (
            "SYNTHETIC_LEGAL_CHOICE_SOUNDNESS",
            "SYNTHETIC_LEGAL_CHOICE_COMPLETENESS",
        ),
    ),
    ChildRunner(
        "m2-g",
        "scripts/run_m2_g_gates.py",
        "m2-g-gate-results.json",
        (
            "M2_PAIRED_STATE_VISIBLE_BYTES_NONINTERFERENCE",
            "M2_MULTI_ENDPOINT_INFORMATION_ISOLATION",
            "M2_REJECTED_RESPONSE_COMPLETE_NONMUTATION",
            "M2_CHECKPOINT_RESTORE_INFORMATION_IDENTITY",
            "M2_FORK_INFORMATION_PARITY",
            "M2_REPLAY_INFORMATION_PARITY",
        ),
    ),
    ChildRunner(
        "m2-h",
        "scripts/run_m2_h_gates.py",
        "m2-h-gate-results.json",
        (
            "M2_RUST_PYTHON_PLAYER_WIRE_PARITY",
            "M2_RULES_FREE_PYTHON_ADAPTER_PARITY",
        ),
    ),
)

M1_CHILD = ChildRunner(
    "m1",
    "scripts/run_m1_closure.py",
    "m1-verification-results.json",
    M1_GATE_NAMES,
    supports_expect_commit=False,
    supports_development_flag=False,
)

VALID_STATUSES = frozenset({"PASS", "FAIL", "BLOCKED", "NOT_RUN"})


# ---------------------------------------------------------------------------
# Source identity and toolchain capture (same contract as the M1 reporter).
# ---------------------------------------------------------------------------


def run_command(command: Sequence[str]) -> subprocess.CompletedProcess[str]:
    environment = dict(os.environ)
    environment["CARGO_TERM_COLOR"] = "never"
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    return subprocess.run(
        list(command),
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )


def tracked_source_fingerprint() -> str:
    listed = run_command(("git", "ls-files", "-z"))
    if listed.returncode != 0:
        raise RuntimeError(listed.stdout.strip() or "git ls-files failed")
    hasher = hashlib.sha256()
    for encoded in listed.stdout.encode("utf-8").split(b"\0"):
        if not encoded:
            continue
        relative = encoded.decode("utf-8")
        payload = (ROOT / relative).read_bytes()
        relative_bytes = relative.encode("utf-8")
        hasher.update(len(relative_bytes).to_bytes(8, "big"))
        hasher.update(relative_bytes)
        hasher.update(len(payload).to_bytes(8, "big"))
        hasher.update(payload)
    return hasher.hexdigest()


def git_value(arguments: Sequence[str]) -> str:
    completed = run_command(("git", *arguments))
    if completed.returncode != 0:
        raise RuntimeError(completed.stdout.strip() or " ".join(("git", *arguments)))
    return completed.stdout.strip()


def capture_source_snapshot() -> dict[str, Any]:
    try:
        status_output = git_value(("status", "--porcelain=v1", "--untracked-files=all"))
        return {
            "status": "PASS" if not status_output else "BLOCKED",
            "commit": git_value(("rev-parse", "HEAD")),
            "tree": git_value(("rev-parse", "HEAD^{tree}")),
            "clean": not status_output,
            "git_status": status_output,
            "fingerprint": tracked_source_fingerprint(),
        }
    except (OSError, RuntimeError) as error:
        return {"status": "BLOCKED", "reason": str(error)}


def aggregate(statuses: Iterable[str]) -> str:
    """FAIL-dominant aggregation: any failure blocks completion.

    Unrecognized status vocabulary is treated as FAIL rather than silently
    accepted: only explicit PASS across every input can aggregate to PASS.
    """
    values = set(statuses)
    if "FAIL" in values or values - VALID_STATUSES:
        return "FAIL"
    if "BLOCKED" in values:
        return "BLOCKED"
    if "NOT_RUN" in values:
        return "NOT_RUN"
    return "PASS"


def reported_toolchain_version(name: str, output: str) -> str | None:
    first_line = output.splitlines()[0] if output.splitlines() else ""
    if name in {"rustc", "cargo"}:
        match = re.match(rf"^{re.escape(name)}\s+(\d+\.\d+\.\d+)(?=\s|\(|$)", first_line)
    elif name == "active_toolchain":
        match = re.match(r"^(\d+\.\d+\.\d+)(?=-|\s|$)", first_line)
    else:
        return None
    return match.group(1) if match else None


def capture_toolchain() -> dict[str, Any]:
    try:
        expected_python = (ROOT / ".python-version").read_text(encoding="utf-8").strip()
        with (ROOT / "rust-toolchain.toml").open("rb") as handle:
            expected_rust = str(tomllib.load(handle)["toolchain"]["channel"])
    except (KeyError, OSError, tomllib.TOMLDecodeError) as error:
        return {"status": "BLOCKED", "reason": f"toolchain policy unreadable: {error}"}

    python_version = platform.python_version()
    python_status = "PASS" if python_version == expected_python else "FAIL"
    rust_results: dict[str, dict[str, Any]] = {}
    for name, command in {
        "rustc": ("rustc", "--version"),
        "cargo": ("cargo", "--version"),
        "active_toolchain": ("rustup", "show", "active-toolchain"),
    }.items():
        if shutil.which(command[0]) is None:
            rust_results[name] = {"status": "NOT_RUN", "command": list(command)}
            continue
        try:
            completed = run_command(command)
        except OSError as error:
            rust_results[name] = {
                "status": "BLOCKED",
                "command": list(command),
                "reason": str(error),
            }
            continue
        output = completed.stdout.strip()
        rust_results[name] = {
            "status": "PASS" if completed.returncode == 0 else "FAIL",
            "command": list(command),
            "returncode": completed.returncode,
            "output": output,
        }
    rust_status = aggregate(result["status"] for result in rust_results.values())
    version_checks: dict[str, dict[str, Any]] = {}
    if rust_status == "PASS":
        for name, record in rust_results.items():
            reported = reported_toolchain_version(name, record.get("output", ""))
            version_checks[name] = {
                "expected": expected_rust,
                "reported": reported,
                "status": "PASS" if reported == expected_rust else "FAIL",
            }
        if any(check["status"] != "PASS" for check in version_checks.values()):
            rust_status = "FAIL"
    return {
        "status": aggregate((python_status, rust_status)),
        "python": {
            "status": python_status,
            "executable": sys.executable,
            "version": python_version,
            "expected_version": expected_python,
        },
        "rust": {
            "status": rust_status,
            "expected_channel": expected_rust,
            "commands": rust_results,
            "version_checks": version_checks,
        },
    }


# ---------------------------------------------------------------------------
# Child runner execution and strict report validation.
# ---------------------------------------------------------------------------


def child_command(
    child: ChildRunner, output_dir: Path, expected_commit: str | None, *, development: bool
) -> list[str]:
    command = [sys.executable, str(ROOT / child.script), "--output-dir", str(output_dir)]
    if expected_commit is not None and child.supports_expect_commit:
        command += ["--expect-commit", expected_commit]
    if development and child.supports_development_flag:
        command.append("--development")
    return command


def read_child_report(record: dict[str, Any], report_path: Path) -> dict[str, Any] | None:
    try:
        report = json.loads(report_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        record.update(
            {"status": "BLOCKED", "reason": f"authoritative child report unreadable: {error}"}
        )
        return None
    if not isinstance(report, dict):
        record.update({"status": "BLOCKED", "reason": "child report is not a JSON object"})
        return None
    return report


def execute_child(
    child: ChildRunner,
    output_root: Path,
    logs: Path,
    index: int,
    expected_commit: str | None,
    *,
    development: bool,
) -> dict[str, Any]:
    child_output = output_root / "runs" / child.slug
    command = child_command(child, child_output, expected_commit, development=development)
    log_path = logs / f"{index:03d}-{child.slug}.log"
    record: dict[str, Any] = {
        "runner": child.script,
        "owned_gates": list(child.gates),
        "command": command,
        "output_dir": str(child_output),
        "report_file": str(child_output / child.report_file),
        "expected_commit": expected_commit,
    }
    try:
        completed = run_command(command)
    except OSError as error:
        record.update({"status": "BLOCKED", "returncode": None, "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
        return record
    log_path.write_text(completed.stdout, encoding="utf-8")
    record["returncode"] = completed.returncode
    report = read_child_report(record, child_output / child.report_file)
    if report is None:
        return record
    record["child_mode"] = report.get("mode")
    record["source_commit"] = report.get("source_commit")
    identity = report.get("source_tree_identity")
    record["source_identity_status"] = (
        identity.get("status") if isinstance(identity, dict) else None
    )

    if child is M1_CHILD:
        statuses, problems = validate_m1_report(
            report, expected_commit, returncode=completed.returncode, strict=not development
        )
    else:
        statuses, problems = validate_slice_report(
            report,
            child,
            expected_commit,
            returncode=completed.returncode,
            strict=not development,
        )
    record["gates"] = statuses
    record["problems"] = problems
    record["status"] = "FAIL" if problems else aggregate(statuses.values())
    record["log"] = f"logs/{log_path.name}"
    return record


def validate_slice_report(
    report: dict[str, Any],
    child: ChildRunner,
    expected_commit: str | None,
    *,
    returncode: int,
    strict: bool,
) -> tuple[dict[str, str], list[str]]:
    """Validate one M2.B-H child report; returns gate statuses and problems.

    In non-strict (development) mode, environmental mismatches such as a
    dirty worktree or development-mode labeling are tolerated diagnostically;
    gate registration structure is always enforced.
    """
    problems: list[str] = []
    statuses: dict[str, str] = {}

    if strict and report.get("mode") != "authoritative":
        problems.append(f"child report mode is {report.get('mode')!r}, not 'authoritative'")
    identity = report.get("source_tree_identity")
    identity_status = identity.get("status") if isinstance(identity, dict) else None
    if strict and identity_status != "PASS":
        problems.append(f"child source_tree_identity status is {identity_status!r}")
    if strict and expected_commit is not None and report.get("source_commit") != expected_commit:
        problems.append(
            f"child source_commit {report.get('source_commit')!r} does not equal "
            f"the pinned head {expected_commit!r}"
        )

    gates = report.get("gates")
    if not isinstance(gates, list) or not gates:
        problems.append("child report has no gates list")
        return statuses, problems
    malformed = [gate for gate in gates if not isinstance(gate, dict)]
    if malformed:
        problems.append(f"child report contains {len(malformed)} malformed gate entries")
    names = tuple(gate.get("name") for gate in gates if isinstance(gate, dict))
    if names != child.gates:
        problems.append(
            f"child gate registration mismatch: expected {list(child.gates)}, found {list(names)}"
        )
    for gate in gates:
        if not isinstance(gate, dict):
            continue
        name = gate.get("name")
        # Slice runners differ: M2.B labels its single gate entry "status",
        # M2.C-H label theirs "gate_status".  Accept either spelling but
        # fail closed when both are present and disagree.
        recorded = {key: gate.get(key) for key in ("gate_status", "status") if key in gate}
        values = {value for value in recorded.values() if isinstance(value, str)}
        if len(values) > 1:
            problems.append(f"gate {name!r} has conflicting status fields {recorded!r}")
        status = next(iter(sorted(values))) if values else None
        if name in child.gates and isinstance(status, str):
            statuses[name] = status
        if status not in VALID_STATUSES:
            problems.append(f"gate {name!r} has invalid status {recorded!r}")
    for name in child.gates:
        statuses.setdefault(name, "NOT_RUN")

    reported_all_pass = all(status == "PASS" for status in statuses.values()) and not problems
    if strict and reported_all_pass and returncode != 0:
        problems.append(
            f"child report claims all gates PASS but exited {returncode}; "
            "ambiguous evidence fails closed"
        )
    if strict and not reported_all_pass and returncode == 0:
        problems.append(
            "child exited 0 while its evidence did not validate cleanly "
            "(non-PASS gate or validation problem); ambiguous evidence fails closed"
        )
    return statuses, problems


def validate_m1_report(
    report: dict[str, Any],
    expected_commit: str | None,
    *,
    returncode: int,
    strict: bool,
) -> tuple[dict[str, str], list[str]]:
    """Validate the M1 closure report; ten exact gates must be executable PASS."""
    problems: list[str] = []
    statuses: dict[str, str] = {}

    if strict and (
        report.get("overall") != "COMPLETE" or report.get("milestone_status") != "COMPLETE"
    ):
        problems.append(
            f"M1 report overall={report.get('overall')!r} "
            f"milestone_status={report.get('milestone_status')!r}"
        )
    identity = report.get("source_tree_identity")
    identity_status = identity.get("status") if isinstance(identity, dict) else None
    if strict and identity_status != "PASS":
        problems.append(f"M1 source_tree_identity status is {identity_status!r}")
    if strict and expected_commit is not None and report.get("source_commit") != expected_commit:
        problems.append(
            f"M1 source_commit {report.get('source_commit')!r} does not equal "
            f"the pinned head {expected_commit!r}"
        )

    gates = report.get("gates")
    if not isinstance(gates, list) or not gates:
        problems.append("M1 report has no gates list")
        return statuses, problems
    malformed = [gate for gate in gates if not isinstance(gate, dict)]
    if malformed:
        problems.append(f"M1 report contains {len(malformed)} malformed gate entries")
    names = tuple(gate.get("name") for gate in gates if isinstance(gate, dict))
    if names != M1_GATE_NAMES:
        problems.append(
            f"M1 gate registration mismatch: expected {list(M1_GATE_NAMES)}, found {list(names)}"
        )
    for gate in gates:
        if not isinstance(gate, dict):
            continue
        name = gate.get("name")
        status = gate.get("status")
        if name in M1_GATE_NAMES and isinstance(status, str):
            statuses[name] = status
        if status not in VALID_STATUSES:
            problems.append(f"M1 gate {name!r} has invalid status {status!r}")
            continue
        if status == "PASS" and not m1_evidence_is_executable_pass(gate):
            problems.append(f"M1 gate {name!r} PASS lacks executable per-test evidence")
    for name in M1_GATE_NAMES:
        statuses.setdefault(name, "NOT_RUN")

    reported_all_pass = all(status == "PASS" for status in statuses.values()) and not problems
    if strict and reported_all_pass and returncode != 0:
        problems.append(
            f"M1 report claims complete but exited {returncode}; ambiguous evidence fails closed"
        )
    return statuses, problems


def m1_evidence_is_executable_pass(gate: dict[str, Any]) -> bool:
    evidence = gate.get("evidence")
    if not isinstance(evidence, list) or not evidence:
        return False
    for item in evidence:
        if not isinstance(item, dict):
            return False
        command = item.get("command")
        if (
            item.get("status") != "PASS"
            or item.get("returncode") != 0
            or item.get("tests_observed") != 1
            or not isinstance(item.get("test"), str)
            or not item["test"]
            or not isinstance(command, list)
            or not command
            or not all(isinstance(part, str) and bool(part) for part in command)
        ):
            return False
    return True


# ---------------------------------------------------------------------------
# Explicit M2 scope guard (owned by M2.Final; read-only source checks).
# ---------------------------------------------------------------------------

SCOPE_SCAN_ROOTS: tuple[str, ...] = ("crates", "tools", "python/src")

SCOPE_SEARCH_PATTERNS: tuple[tuple[str, str], ...] = (
    (r"\bMCTS\b", "MCTS search"),
    (r"\bMonteCarlo\b", "Monte Carlo search"),
    (r"\bMonte\s+Carlo\b", "Monte Carlo search"),
    (r"\bCFR\b", "counterfactual regret minimization"),
    (r"\bcounterfactual\s+regret\b", "counterfactual regret minimization"),
    (r"\bdeterminiz(?:ation|e|ed|ing)\b", "determinization"),
    (r"\bminimax\b", "minimax search"),
    (r"\balpha[-_]?beta\s*(?:prun|search)", "alpha-beta search"),
    (r"\bplayout\b", "playout simulation"),
    (r"\brollout\b", "rollout simulation"),
)

SCOPE_VECTOR_PATTERNS: tuple[tuple[str, str], ...] = (
    (r"\btorch\b", "PyTorch training dependency"),
    (r"\bnumpy\b", "NumPy training dependency"),
    (r"\bVectorEnv\b", "vectorized environment architecture"),
    (r"\bvectorized\b", "vectorized environment architecture"),
    (r"\bDataParallel\b", "distributed data parallelism"),
    (r"\bDistributedDataParallel\b", "distributed data parallelism"),
    (r"(?m)^\s*import\s+ray\b", "Ray distributed runtime"),
    (r"(?m)^\s*from\s+ray\b", "Ray distributed runtime"),
)

SCOPE_HEURISTIC_PATTERNS: tuple[tuple[str, str], ...] = (
    (r"\bheuristic\b", "heuristic choice completion"),
    (r"\bauto_answer\b", "automatic answer completion"),
    (r"\bfallback_choice\b", "fallback choice completion"),
    (r"\bdefault_choice\b", "default choice completion"),
    (r"\barbitrary_choice\b", "arbitrary choice completion"),
    (r"\bpick_any\b", "arbitrary choice completion"),
)

SCOPE_MAGIC_PATTERNS: tuple[tuple[str, str], ...] = (
    (r"\bManaCost\b", "real Magic mana semantics"),
    (r"\bManaPool\b", "real Magic mana semantics"),
    (r"\bmana_pool\b", "real Magic mana semantics"),
    (r"(?i)\bplaneswalker\b", "real Magic card type"),
    (r"\bcombat_damage\b", "real Magic combat semantics"),
    (r"\btriggered_ability\b", "real Magic trigger semantics"),
    (r"\breplacement_effect\b", "real Magic replacement semantics"),
    (r"\bpriority_pass\b", "real Magic priority semantics"),
)

WORKSPACE_MEMBERS_ALLOWED: tuple[str, ...] = (
    "crates/mtgml-card-ir",
    "crates/mtgml-commander",
    "crates/mtgml-conformance",
    "crates/mtgml-decision",
    "crates/mtgml-environment",
    "crates/mtgml-model",
    "crates/mtgml-observation",
    "crates/mtgml-persistence",
    "crates/mtgml-random",
    "crates/mtgml-replay",
    "crates/mtgml-rules",
    "crates/mtgml-state",
    "crates/mtgml-wire",
    "tools/m2-semantic-adapter",
)

WORKSPACE_DEPENDENCIES_ALLOWED: frozenset[str] = frozenset(
    {"base64", "getrandom", "hmac", "serde", "serde_json", "sha2", "thiserror"}
)

SCHEMA_INVENTORY_ALLOWED: frozenset[str] = frozenset(
    {
        "authoritative-replay.v1.schema.json",
        "authoritative-replay.v2.schema.json",
        "authoritative-replay.v3.schema.json",
        "bundle-certification.v1.schema.json",
        "bundle-manifest.v1.schema.json",
        "capability-registry.v1.schema.json",
        "card-definition-manifest.v1.schema.json",
        "contract-vocabulary-catalog.v1.schema.json",
        "decision-response.v1.schema.json",
        "decision-response.v2.schema.json",
        "episode-status.v1.schema.json",
        "golden-path-index.v1.schema.json",
        "information-state-envelope.v1.schema.json",
        "information-state-envelope.v2.schema.json",
        "normative-document-register.v1.schema.json",
        "observation-envelope.v1.schema.json",
        "observed-event-envelope.v1.schema.json",
        "observed-event-envelope.v2.schema.json",
        "player-decision-request.v1.schema.json",
        "player-decision-request.v2.schema.json",
        "player-step.v1.schema.json",
        "player-step.v2.schema.json",
        "replay-manifest.v1.schema.json",
        "replay-manifest.v2.schema.json",
        "replay-manifest.v3.schema.json",
        "scope-impact-report.v1.schema.json",
    }
)

DECK_FILES_ALLOWED: frozenset[str] = frozenset({"example-deck-a.json", "example-deck-b.json"})


class ScopeCheckFailure(AssertionError):
    """A scope-guard check found unauthorized scope expansion."""


def scope_scan_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for relative in SCOPE_SCAN_ROOTS:
        base = root / relative
        if not base.is_dir():
            continue
        files.extend(sorted(base.rglob("*.rs")))
        files.extend(sorted(base.rglob("*.py")))
    return files


def scan_for_patterns(
    root: Path, patterns: Sequence[tuple[str, str]], label: str
) -> tuple[int, list[str]]:
    violations: list[str] = []
    compiled = [(re.compile(pattern), reason) for pattern, reason in patterns]
    scanned = 0
    for path in scope_scan_files(root):
        scanned += 1
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as error:
            raise RuntimeError(f"unreadable scope-scan input {path}: {error}") from error
        for regex, reason in compiled:
            for match in regex.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                violations.append(
                    f"{path.relative_to(root).as_posix()}:{line}: {reason} ({regex.pattern!r})"
                )
    if violations:
        raise ScopeCheckFailure(
            f"production sources contain {label}:\n" + "\n".join(violations[:50])
        )
    return scanned, violations


def load_root_manifest(root: Path) -> dict[str, Any]:
    with (root / "Cargo.toml").open("rb") as handle:
        return tomllib.load(handle)


def check_no_search_or_determinization(root: Path) -> str:
    scanned, _ = scan_for_patterns(root, SCOPE_SEARCH_PATTERNS, "search/determinization work")
    return f"no search/MCTS/CFR/determinization across {scanned} source files"


def check_no_vector_or_distributed_training(root: Path) -> str:
    scanned, _ = scan_for_patterns(root, SCOPE_VECTOR_PATTERNS, "vector/distributed training")
    return f"no vector/distributed-training architecture across {scanned} source files"


def check_no_hidden_heuristic_choices(root: Path) -> str:
    scanned, _ = scan_for_patterns(root, SCOPE_HEURISTIC_PATTERNS, "hidden choice completion")
    return f"no hidden/heuristic choice completion across {scanned} source files"


def check_no_real_magic_sources(root: Path) -> str:
    scanned, _ = scan_for_patterns(root, SCOPE_MAGIC_PATTERNS, "real Magic semantics")
    return f"no real Magic/card semantics across {scanned} source files"


def check_workspace_inventory_pinned(root: Path) -> str:
    manifest = load_root_manifest(root)
    members_raw = manifest.get("workspace", {}).get("members")
    if not isinstance(members_raw, list) or not members_raw:
        raise ScopeCheckFailure("root Cargo.toml has no [workspace].members list")
    members = sorted(str(member) for member in members_raw)
    unexpected = sorted(set(members) - set(WORKSPACE_MEMBERS_ALLOWED))
    missing = sorted(set(WORKSPACE_MEMBERS_ALLOWED) - set(members))
    if unexpected or missing:
        raise ScopeCheckFailure(
            "workspace membership drifted from the pinned M2 inventory "
            f"(unexpected={unexpected}, missing={missing})"
        )
    dependencies = manifest.get("workspace", {}).get("dependencies", {})
    keys = set(dependencies) if isinstance(dependencies, dict) else set()
    unexpected_deps = sorted(keys - WORKSPACE_DEPENDENCIES_ALLOWED)
    if unexpected_deps:
        raise ScopeCheckFailure(
            f"new workspace dependencies outside the pinned M2 inventory: {unexpected_deps}"
        )
    return (
        f"workspace members and dependencies match the pinned M2 inventory ({len(members)} members)"
    )


def check_python_runtime_dependencies_empty(root: Path) -> str:
    with (root / "python" / "pyproject.toml").open("rb") as handle:
        pyproject = tomllib.load(handle)
    project = pyproject.get("project", {})
    dependencies = project.get("dependencies", [])
    forbidden_sections = [
        section
        for section in ("dependency-groups", "optional-dependencies", "dev-dependencies")
        if pyproject.get(section) or project.get(section)
    ]
    if dependencies or forbidden_sections:
        raise ScopeCheckFailure(
            f"python package gained dependency surfaces "
            f"(dependencies={dependencies}, sections={forbidden_sections})"
        )
    return (
        "python package declares no runtime, optional, or grouped dependencies "
        "(rules-free contracts only)"
    )


def parse_workspace_members(root: Path) -> list[str]:
    manifest = load_root_manifest(root)
    members_raw = manifest.get("workspace", {}).get("members")
    if not isinstance(members_raw, list) or not members_raw:
        raise ScopeCheckFailure("root Cargo.toml has no [workspace].members list")
    return sorted(str(member) for member in members_raw)


def check_member_dependency_sources_pinned(root: Path) -> str:
    """Every workspace-member dependency must come from the pinned
    ``[workspace.dependencies]`` table or an in-repository path; direct
    registry requirements in member manifests are scope expansion."""
    violations: list[str] = []
    for member in parse_workspace_members(root):
        manifest_path = root / member / "Cargo.toml"
        if not manifest_path.is_file():
            violations.append(f"{member}: workspace member has no Cargo.toml")
            continue
        with manifest_path.open("rb") as handle:
            manifest = tomllib.load(handle)
        for table in ("dependencies", "dev-dependencies", "build-dependencies"):
            entries = manifest.get(table, {})
            if not isinstance(entries, dict):
                violations.append(f"{member}: [{table}] is not a table")
                continue
            for dep_name, spec in entries.items():
                if isinstance(spec, dict) and ("workspace" in spec or "path" in spec):
                    continue
                violations.append(
                    f"{member}: [{table}] {dep_name!r} is a direct registry "
                    f"dependency ({spec!r}); only workspace inheritance or "
                    "in-repo path dependencies are allowed"
                )
    if violations:
        raise ScopeCheckFailure(
            "workspace members declare unpinned dependency sources:\n" + "\n".join(violations[:50])
        )
    return "all workspace-member dependencies inherit the workspace table or use repo paths"


def check_open_decisions_rows(root: Path) -> str:
    text = (root / "docs" / "OPEN_DECISIONS.md").read_text(encoding="utf-8")
    for decision, subject in (
        ("OD-003", "exact deck manifests"),
        ("OD-009", "production Python/native transport"),
        ("OD-011", "semantic action-key/trajectory encoding"),
        ("OD-020", "search/determinization boundary"),
    ):
        row = re.search(rf"^\|\s*{decision}\s*\|\s*(\w+)\s*\|", text, flags=re.MULTILINE)
        if row is None:
            raise ScopeCheckFailure(f"{decision} ({subject}) row missing from OPEN_DECISIONS.md")
        if row.group(1) != "open":
            raise ScopeCheckFailure(
                f"{decision} ({subject}) is {row.group(1)!r}; M2.Final requires it to remain open"
            )
    return (
        "OD-003, OD-009, OD-011, and OD-020 remain open "
        "(no deck/transport/action-key/search decision)"
    )


def check_transport_and_trajectory_docs_unchanged(root: Path) -> str:
    environment = (root / "docs" / "ML_ENVIRONMENT.md").read_text(encoding="utf-8")
    if "OD-009 remains open" not in environment:
        raise ScopeCheckFailure("docs/ML_ENVIRONMENT.md no longer states OD-009 remains open")
    trajectories = (root / "docs" / "ML_TRAJECTORIES.md").read_text(encoding="utf-8")
    if "deferred until first dataset" not in trajectories:
        raise ScopeCheckFailure(
            "docs/ML_TRAJECTORIES.md no longer defers the concrete trajectory schema"
        )
    return "transport stays parity-only (OD-009 open); trajectory contract stays deferred (OD-011)"


def check_adapter_remains_unpublished_test_tool(root: Path) -> str:
    manifest_text = (root / "tools" / "m2-semantic-adapter" / "Cargo.toml").read_text(
        encoding="utf-8"
    )
    if not re.search(r"(?m)^\s*publish\s*=\s*false\s*$", manifest_text):
        raise ScopeCheckFailure(
            "tools/m2-semantic-adapter lost `publish = false`; the temporary "
            "parity adapter must stay a non-published test tool"
        )
    return "m2-semantic-adapter remains unpublished (temporary parity-only test tool)"


def check_schema_inventory_pinned(root: Path) -> str:
    schemas = sorted(path.name for path in (root / "schemas").rglob("*.schema.json"))
    unexpected = sorted(set(schemas) - SCHEMA_INVENTORY_ALLOWED)
    missing = sorted(SCHEMA_INVENTORY_ALLOWED - set(schemas))
    if unexpected or missing:
        raise ScopeCheckFailure(
            "schema inventory drifted from the pinned M2 inventory "
            f"(unexpected={unexpected}, missing={missing}); new trajectory/"
            "deck-lock/census schemas require explicit review"
        )
    forbidden = [
        name
        for name in schemas
        if "trajectory" in name or "deck-lock" in name or "deck_lock" in name or "census" in name
    ]
    if forbidden:
        raise ScopeCheckFailure(f"forbidden M2.5/M5 schema artifacts present: {forbidden}")
    return f"schema inventory matches the pinned M2 inventory ({len(schemas)} schemas)"


def check_card_and_deck_artifacts_unclaimed(root: Path) -> str:
    decks_dir = root / "cards" / "decks"
    decks = {path.name for path in decks_dir.iterdir() if path.is_file()}
    if decks != DECK_FILES_ALLOWED:
        raise ScopeCheckFailure(
            f"cards/decks inventory drifted: {sorted(decks)} != {sorted(DECK_FILES_ALLOWED)}; "
            "M2.5 exact deck lock work is out of scope"
        )
    deck_directories = [path.name for path in decks_dir.iterdir() if path.is_dir()]
    if deck_directories:
        raise ScopeCheckFailure(
            f"cards/decks contains subdirectories (possible M2.5 deck-lock work): "
            f"{sorted(deck_directories)}"
        )
    definition_entries = {path.name for path in (root / "cards" / "definitions").iterdir()}
    if definition_entries - {"example", ".gitkeep"}:
        raise ScopeCheckFailure(
            f"cards/definitions contains non-example card work: {sorted(definition_entries)}"
        )
    generated_entries = [
        path.name for path in (root / "cards" / "generated").iterdir() if path.name != ".gitkeep"
    ]
    if generated_entries:
        raise ScopeCheckFailure(f"cards/generated contains artifacts: {generated_entries}")
    return "card/deck trees contain only the pre-existing maintainer examples (no real-card claim)"


KERNEL_IMPL_PATTERN = re.compile(r"\bimpl\s+RulesKernel\s+for\s+([A-Za-z_][A-Za-z0-9_]*)")
EXPECTED_KERNEL_IMPLEMENTATIONS: frozenset[str] = frozenset({"SyntheticM1RulesKernel"})


def _is_test_convention_path(relative_posix: str) -> bool:
    parts = relative_posix.split("/")
    if any(part == "tests" for part in parts[:-1]):
        return True
    name = parts[-1]
    return name == "tests.rs" or name.endswith("_tests.rs")


def check_rules_backend_inventory(root: Path) -> str:
    """The M2 runtime must own exactly one production RulesKernel
    implementation, the synthetic reference kernel.  Any second
    implementation (for example an optimized or alternate rules backend) is
    unauthorized M3/scope expansion."""
    implementers: dict[str, list[str]] = {}
    scanned = 0
    for relative in ("crates", "tools"):
        base = root / relative
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            relative_posix = path.relative_to(root).as_posix()
            if _is_test_convention_path(relative_posix):
                continue
            scanned += 1
            try:
                text = path.read_text(encoding="utf-8")
            except OSError as error:
                raise RuntimeError(f"unreadable kernel-scan input {path}: {error}") from error
            for match in KERNEL_IMPL_PATTERN.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                implementers.setdefault(match.group(1), []).append(f"{relative_posix}:{line}")
    unexpected = sorted(set(implementers) - EXPECTED_KERNEL_IMPLEMENTATIONS)
    missing = sorted(EXPECTED_KERNEL_IMPLEMENTATIONS - set(implementers))
    if unexpected or missing:
        detail = (
            f"production RulesKernel implementations drifted from the pinned M2 "
            f"inventory (unexpected={unexpected}, missing={missing})"
        )
        found = {name: implementers[name] for name in unexpected if name in implementers}
        if found:
            detail += ": " + json.dumps(found, sort_keys=True)
        raise ScopeCheckFailure(detail)
    return (
        "exactly one production RulesKernel implementation "
        f"(SyntheticM1RulesKernel) across {scanned} production source files"
    )


SCOPE_CHECKS: tuple[tuple[str, Callable[[Path], str]], ...] = (
    ("scope::workspace_inventory_pinned", check_workspace_inventory_pinned),
    ("scope::python_runtime_dependencies_empty", check_python_runtime_dependencies_empty),
    ("scope::member_dependency_sources_pinned", check_member_dependency_sources_pinned),
    ("scope::rules_backend_inventory", check_rules_backend_inventory),
    ("scope::open_decisions_rows_still_open", check_open_decisions_rows),
    (
        "scope::transport_and_trajectory_docs_unchanged",
        check_transport_and_trajectory_docs_unchanged,
    ),
    ("scope::adapter_remains_unpublished_test_tool", check_adapter_remains_unpublished_test_tool),
    ("scope::schema_inventory_pinned", check_schema_inventory_pinned),
    ("scope::card_and_deck_artifacts_unclaimed", check_card_and_deck_artifacts_unclaimed),
    ("scope::no_real_magic_sources", check_no_real_magic_sources),
    ("scope::no_hidden_heuristic_choices", check_no_hidden_heuristic_choices),
    ("scope::no_search_or_determinization", check_no_search_or_determinization),
    ("scope::no_vector_or_distributed_training", check_no_vector_or_distributed_training),
)

EXPECTED_SCOPE_CHECKS: tuple[str, ...] = (
    "scope::workspace_inventory_pinned",
    "scope::python_runtime_dependencies_empty",
    "scope::member_dependency_sources_pinned",
    "scope::rules_backend_inventory",
    "scope::open_decisions_rows_still_open",
    "scope::transport_and_trajectory_docs_unchanged",
    "scope::adapter_remains_unpublished_test_tool",
    "scope::schema_inventory_pinned",
    "scope::card_and_deck_artifacts_unclaimed",
    "scope::no_real_magic_sources",
    "scope::no_hidden_heuristic_choices",
    "scope::no_search_or_determinization",
    "scope::no_vector_or_distributed_training",
)


def validate_scope_check_registry() -> str | None:
    """Fail closed if the executed scope-check registry ever drifts from the
    pinned inventory; a silently removed mandatory subcheck must not let the
    remaining checks aggregate to PASS."""
    actual = tuple(name for name, _ in SCOPE_CHECKS)
    if actual != EXPECTED_SCOPE_CHECKS:
        return (
            f"scope check registry drift: expected {list(EXPECTED_SCOPE_CHECKS)}, "
            f"found {list(actual)}"
        )
    return None


def execute_scope_guard(logs: Path) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for index, (name, function) in enumerate(SCOPE_CHECKS, start=1):
        entry: dict[str, Any] = {"check": name}
        log_path = logs / f"9{index:02d}-scope-{name.split('::')[1]}.log"
        try:
            detail = function(ROOT)
        except ScopeCheckFailure as error:
            entry.update({"status": "FAIL", "reason": str(error)})
            log_path.write_text(str(error) + "\n", encoding="utf-8")
        except (OSError, RuntimeError, KeyError, tomllib.TOMLDecodeError, ValueError) as error:
            entry.update({"status": "BLOCKED", "reason": str(error)})
            log_path.write_text(str(error) + "\n", encoding="utf-8")
        except Exception as error:  # fail closed on unexpected errors
            entry.update({"status": "FAIL", "reason": f"unexpected: {error!r}"})
            log_path.write_text(repr(error) + "\n", encoding="utf-8")
        else:
            entry.update({"status": "PASS", "detail": detail})
            log_path.write_text(detail + "\n", encoding="utf-8")
        entry["log"] = f"logs/{log_path.name}"
        results.append(entry)
    return results


# ---------------------------------------------------------------------------
# Certification profile prerequisite (archive/reproducibility gate last).
# ---------------------------------------------------------------------------


def execute_certification_profile(logs: Path) -> dict[str, Any]:
    """Run the repository certification profile as a closure prerequisite.

    ``scripts/run_checks.py certification`` executes the fast and integration
    profiles plus the pinned Python toolchain verification and the
    deterministic source-archive reproducibility gate, which is its final
    command.  A non-PASS result blocks milestone completion.
    """
    command = [sys.executable, str(ROOT / "scripts" / "run_checks.py"), "certification"]
    log_path = logs / "991-certification-profile.log"
    record: dict[str, Any] = {
        "prerequisite": "scripts/run_checks.py certification",
        "command": command,
        "log": f"logs/{log_path.name}",
    }
    try:
        completed = run_command(command)
    except OSError as error:
        record.update({"status": "BLOCKED", "returncode": None, "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
        return record
    output = completed.stdout
    log_path.write_text(output, encoding="utf-8")
    record["returncode"] = completed.returncode
    if completed.returncode == 0:
        record["status"] = "PASS"
        record["detail"] = (
            "certification profile passed; deterministic archive reproducibility gate executed last"
        )
    elif "MISSING TOOL" in output:
        record["status"] = "BLOCKED"
        record["reason"] = "certification profile could not run: missing required tool"
    else:
        record["status"] = "FAIL"
        record["reason"] = f"certification profile failed with exit code {completed.returncode}"
    return record


# ---------------------------------------------------------------------------
# Report assembly.
# ---------------------------------------------------------------------------


def finalize_source_identity(before: dict[str, Any], after: dict[str, Any]) -> dict[str, Any]:
    if before.get("status") != "PASS":
        return {**before, "after": after, "status": "BLOCKED"}
    if after.get("status") != "PASS":
        return {
            "status": "FAIL",
            "reason": "verification changed or dirtied the tracked source tree",
            "before": before,
            "after": after,
        }
    same = all(
        before.get(key) == after.get(key) for key in ("commit", "tree", "fingerprint", "clean")
    )
    return {
        "status": "PASS" if same else "FAIL",
        "commit": before.get("commit"),
        "tree": before.get("tree"),
        "fingerprint_before": before.get("fingerprint"),
        "fingerprint_after": after.get("fingerprint"),
        "clean_before": before.get("clean"),
        "clean_after": after.get("clean"),
        "reason": "unchanged" if same else "source identity changed",
    }


def compute_m1_matrix_status(m1_run: dict[str, Any] | None) -> str:
    if m1_run is None:
        return "NOT_RUN"
    components = list(m1_run.get("gates", {}).values())
    components.append(m1_run.get("status", "NOT_RUN"))
    return aggregate(components)


def merge_child_gate_statuses(child_runs: Sequence[dict[str, Any]]) -> dict[str, str]:
    """Merge parsed child gate statuses into the canonical gate map.

    A non-PASS child record poisons every gate that child owns: validation
    problems such as duplicate, missing, extra, or malformed gate entries
    block completion even when the canonical entries themselves look PASS.
    """
    merged: dict[str, str] = {}
    for record in child_runs:
        record_status = record.get("status", "NOT_RUN")
        parsed = record.get("gates", {})
        for name in record.get("owned_gates", []):
            merged[name] = aggregate((parsed.get(name, "NOT_RUN"), record_status))
    return merged


def build_report(
    *,
    mode: str,
    source_identity: dict[str, Any],
    toolchains: dict[str, Any],
    child_runs: list[dict[str, Any]],
    gates: list[dict[str, Any]],
    m1_regression: dict[str, Any],
    certification: dict[str, Any],
    expected_commit: str | None,
    generated_at: str | None = None,
) -> dict[str, Any]:
    gate_names = tuple(gate.get("name") for gate in gates)
    all_gates_pass = bool(gates) and all(gate.get("status") == "PASS" for gate in gates)
    complete = (
        mode == "authoritative"
        and source_identity.get("status") == "PASS"
        and toolchains.get("status") == "PASS"
        and all_gates_pass
        and gate_names == EXPECTED_GATES
        and m1_regression.get("status") == "PASS"
        and certification.get("status") == "PASS"
        and expected_commit is not None
        and source_identity.get("commit") == expected_commit
    )
    return {
        "generated_at": generated_at or datetime.now(UTC).isoformat(),
        "milestone": "M2",
        "reporter": "scripts/run_m2_final_closure.py",
        "mode": mode,
        "expected_commit": expected_commit,
        "source_commit": source_identity.get("commit"),
        "source_tree_identity": source_identity,
        "toolchains": toolchains,
        "child_runs": child_runs,
        "gates": gates,
        "m1_regression": m1_regression,
        "certification_prerequisite": certification,
        "overall": "COMPLETE" if complete else "INCOMPLETE",
        "milestone_status": "COMPLETE" if complete else "INCOMPLETE",
        "m2_5_status": "UNBLOCKED" if complete else "BLOCKED",
        "claims": {
            "playable_engine": False,
            "real_magic_rules": False,
            "real_card_support": False,
            "m3_started": False,
            "semantic_ownership_adr_accepted": False,
            "production_transport_decided": False,
            "stable_action_key_trajectory_contract_published": False,
            "search_determinization_present": False,
            "vector_or_distributed_training_present": False,
            "optimized_alternate_rules_backend_present": False,
            "hidden_heuristic_choice_completion_present": False,
            "m2_5_deck_lock_or_census_work_present": False,
        },
    }


def render_markdown(report: dict[str, Any]) -> str:
    regression = report["m1_regression"]
    lines = [
        "# M2.Final Closure Verification",
        "",
        "Generated outside the reproducible source archive by `scripts/run_m2_final_closure.py`.",
        "",
        f"- Mode: **{report['mode']}**",
        f"- Expected commit: `{report.get('expected_commit')}`",
        f"- Source commit: `{report.get('source_commit')}`",
        f"- Source identity: **{report['source_tree_identity'].get('status')}**",
        f"- Toolchains: **{report['toolchains'].get('status')}**",
        f"- M2.Final closure: **{report['overall']}**",
        "",
        "## M2 gate matrix",
        "",
        "| # | Gate | Status |",
        "|---:|---|---:|",
    ]
    for position, gate in enumerate(report["gates"], start=1):
        lines.append(f"| {position} | `{gate['name']}` | **{gate['status']}** |")
    lines.extend(
        [
            "",
            "## Gate 20 composition",
            "",
            f"- M1 regression (10 exact gates): **{regression.get('matrix_status')}**",
            f"- M2 scope guard: **{regression.get('scope_status')}**",
            (
                "- Certification profile (archive/reproducibility gate last): "
                f"**{report['certification_prerequisite'].get('status')}**"
                if "certification_prerequisite" in report
                else ""
            ),
            "",
            "| Scope check | Status |",
            "|---|---:|",
        ]
    )
    for check in regression.get("scope_checks", []):
        lines.append(f"| `{check['check']}` | **{check['status']}** |")
    lines.extend(
        [
            "",
            "## Status claims",
            "",
            (
                "- M1 = 10/10 PASS"
                if regression.get("matrix_status") == "PASS"
                else "- M1 = NOT CLOSED"
            ),
            f"- M2 = **{report['milestone_status']}**",
            f"- M2.5 = **{report['m2_5_status']}**",
            "- M3 STARTED = NO",
            "- REAL MAGIC SUPPORT = NO",
            "- REAL CARD SUPPORT = NO",
            "- SEMANTIC OWNERSHIP ADR ACCEPTED = NO",
            "",
            (
                "Only the authoritative exact-head report may claim `M2 = COMPLETE`; "
                "completion never claims playable Magic, real cards, or M2.5 work."
                if report["milestone_status"] == "COMPLETE"
                else "Development or incomplete runs never authorize milestone completion claims."
            ),
            "",
        ]
    )
    return "\n".join(lines)


def render_blockers(report: dict[str, Any]) -> str:
    lines = [
        "# M2.Final Blockers",
        "",
        "| Area | Status | Reason |",
        "|---|---:|---|",
    ]
    has_blocker = False

    def add(area: str, status: str, reason: str) -> None:
        nonlocal has_blocker
        has_blocker = True
        lines.append(f"| {area} | **{status}** | {reason} |")

    identity = report["source_tree_identity"]
    if identity.get("status") != "PASS":
        add(
            "source tree",
            str(identity.get("status")),
            str(identity.get("reason", "source identity check failed")),
        )
    if report["toolchains"].get("status") != "PASS":
        add(
            "toolchains",
            str(report["toolchains"].get("status")),
            "pinned toolchain identity did not pass",
        )
    for run in report["child_runs"]:
        if run.get("status") != "PASS":
            problems = "; ".join(run.get("problems", [])[:5]) or run.get(
                "reason", "child did not pass"
            )
            add(f"runner `{run['runner']}`", str(run["status"]), problems)
    for gate in report["gates"]:
        if gate["status"] != "PASS":
            add(f"gate `{gate['name']}`", gate["status"], "required gate not PASS")
    regression = report["m1_regression"]
    for check in regression.get("scope_checks", []):
        if check["status"] != "PASS":
            add(
                f"`{check['check']}`",
                check["status"],
                str(check.get("reason", "")),
            )
    certification = report.get("certification_prerequisite")
    if certification is not None and certification.get("status") != "PASS":
        add(
            "certification profile",
            str(certification.get("status")),
            str(certification.get("reason", "certification prerequisite not PASS")),
        )
    if not has_blocker:
        lines.append("| none | **PASS** | every required gate passed on the exact clean head |")
    lines.extend(["", f"M2 complete: **{report['milestone_status']}**", ""])
    return "\n".join(lines)


def prepare_output(output: Path) -> Path:
    if output.exists():
        marker = output / OUTPUT_MARKER
        if not marker.is_file():
            raise RuntimeError(f"refusing to replace unowned verification output: {output}")
        shutil.rmtree(output)
    logs = output / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    (output / OUTPUT_MARKER).write_text(
        "owned by scripts/run_m2_final_closure.py\n", encoding="utf-8"
    )
    return logs


def write_reports(output: Path, report: dict[str, Any]) -> None:
    (output / "m2-final-closure-results.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (output / "M2_FINAL_VERIFICATION.md").write_text(render_markdown(report), encoding="utf-8")
    (output / "M2_FINAL_BLOCKERS.md").write_text(render_blockers(report), encoding="utf-8")


def not_executed_report(
    mode: str,
    reason: str,
    toolchains: dict[str, Any],
    source_snapshot: dict[str, Any],
    expected_commit: str | None,
) -> dict[str, Any]:
    identity = {**source_snapshot, "status": "BLOCKED", "reason": reason}
    gates = [{"name": name, "status": "NOT_RUN", "reason": reason} for name in EXPECTED_GATES]
    return build_report(
        mode=mode,
        source_identity=identity,
        toolchains=toolchains,
        child_runs=[],
        gates=gates,
        m1_regression={
            "status": "NOT_RUN",
            "matrix_status": "NOT_RUN",
            "scope_status": "NOT_RUN",
            "scope_checks": [],
        },
        certification={"status": "NOT_RUN", "prerequisite": "scripts/run_checks.py certification"},
        expected_commit=expected_commit,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--development",
        action="store_true",
        help="run underlying children diagnostically; never reports completion",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=OUTPUT,
        help="external owned output directory (default: dist/m2-final-verification)",
    )
    parser.add_argument(
        "--expect-commit",
        metavar="SHA",
        default=None,
        help=(
            "required in authoritative mode: the exact commit identity every gate must execute on"
        ),
    )
    args = parser.parse_args()
    output = args.output_dir.resolve()
    if output == ROOT or ROOT not in output.parents or "dist" not in output.relative_to(ROOT).parts:
        parser.error("M2.Final verification output must remain below the repository dist directory")
    release_root = ROOT / "dist" / "verification"
    if output == (ROOT / "dist") or output == release_root or release_root in output.parents:
        parser.error(
            "M2.Final verification output must not collide with dist/ or the "
            "run_verification.py-owned dist/verification tree"
        )

    mode = "development" if args.development else "authoritative"
    toolchains = capture_toolchain()
    source_before = capture_source_snapshot()

    def blocked_exit(reason: str) -> int:
        report = not_executed_report(mode, reason, toolchains, source_before, args.expect_commit)
        try:
            prepare_output(output)
            write_reports(output, report)
        except (OSError, RuntimeError) as error:
            print(f"BLOCKED: could not write report: {error}")
            return 2
        print(
            json.dumps(
                {
                    "mode": mode,
                    "milestone_status": report["milestone_status"],
                    "m2_5_status": report["m2_5_status"],
                    "blocked_reason": reason,
                    "output_dir": str(output),
                },
                sort_keys=True,
            )
        )
        return 2

    if mode == "authoritative":
        if not source_before.get("clean"):
            return blocked_exit(
                "authoritative closure requires a clean tracked source tree before execution"
            )
        if not args.expect_commit:
            return blocked_exit(
                "authoritative closure requires an explicit --expect-commit source pin"
            )
        if source_before.get("commit") != args.expect_commit:
            return blocked_exit(
                f"HEAD {source_before.get('commit')!r} does not equal the pinned "
                f"--expect-commit {args.expect_commit!r}"
            )

    try:
        logs = prepare_output(output)
    except (OSError, RuntimeError) as error:
        print(f"BLOCKED: {error}")
        return 2

    registry_error = validate_scope_check_registry()
    if registry_error is not None:
        return blocked_exit(registry_error)

    child_runs: list[dict[str, Any]] = []
    m1_run: dict[str, Any] | None = None
    for index, child in enumerate((*CHILD_RUNNERS, M1_CHILD), start=1):
        record = execute_child(
            child,
            output,
            logs,
            index,
            args.expect_commit,
            development=args.development,
        )
        child_runs.append(record)
        if child is M1_CHILD:
            m1_run = record
    gate_statuses = merge_child_gate_statuses(child_runs)

    scope_checks = execute_scope_guard(logs)
    scope_status = aggregate(check["status"] for check in scope_checks)
    certification = execute_certification_profile(logs)
    m1_matrix_status = compute_m1_matrix_status(m1_run)
    m1_regression = {
        "status": aggregate((m1_matrix_status, scope_status)),
        "matrix_status": m1_matrix_status,
        "scope_status": scope_status,
        "scope_checks": scope_checks,
        "m1_runner": M1_CHILD.script,
        "m1_report": (m1_run or {}).get("report_file"),
    }

    gates: list[dict[str, Any]] = []
    for name in EXPECTED_GATES:
        if name == GATE_M1_REGRESSION:
            gates.append(
                {
                    "name": name,
                    "status": m1_regression["status"],
                    "owner": "scripts/run_m2_final_closure.py",
                    "composition": {
                        "m1_matrix_status": m1_matrix_status,
                        "m2_scope_guard_status": scope_status,
                        "scope_check_count": len(scope_checks),
                    },
                }
            )
        elif name in gate_statuses:
            gates.append({"name": name, "status": gate_statuses[name]})
        else:
            gates.append(
                {"name": name, "status": "NOT_RUN", "reason": "no child reported this gate"}
            )

    source_after = capture_source_snapshot()
    source_identity = finalize_source_identity(source_before, source_after)
    if mode == "authoritative" and source_identity.get("status") != "PASS":
        for gate in gates:
            if gate["status"] == "PASS":
                gate["status"] = "FAIL"
                gate["reason"] = "source identity changed during verification"
        if m1_regression["status"] == "PASS":
            m1_regression["status"] = "FAIL"

    report = build_report(
        mode=mode,
        source_identity=source_identity,
        toolchains=toolchains,
        child_runs=child_runs,
        gates=gates,
        m1_regression=m1_regression,
        certification=certification,
        expected_commit=args.expect_commit,
    )
    write_reports(output, report)
    print(
        json.dumps(
            {
                "mode": report["mode"],
                "milestone_status": report["milestone_status"],
                "m2_5_status": report["m2_5_status"],
                "output_dir": str(output),
                "source_commit": report["source_commit"],
                "source_identity": source_identity.get("status"),
                "toolchains": toolchains.get("status"),
                "gates": {gate["name"]: gate["status"] for gate in gates},
                "m1_matrix": m1_matrix_status,
                "m2_scope_guard": scope_status,
                "certification": certification.get("status"),
            },
            sort_keys=True,
        )
    )
    if args.development:
        child_ok = all(
            record.get("returncode") == 0
            or (
                record.get("runner") == M1_CHILD.script
                and record.get("gates")
                and all(status == "PASS" for status in record["gates"].values())
            )
            for record in child_runs
        )
        return 0 if child_ok and scope_status == "PASS" else 2
    return 0 if report["overall"] == "COMPLETE" else 2


if __name__ == "__main__":
    raise SystemExit(main())
