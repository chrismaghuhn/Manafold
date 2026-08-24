#!/usr/bin/env python3
"""Execute the two M2.F executable gates on an exact clean source head.

Owned gates:

```text
KNOWLEDGE_RETENTION_INVALIDATION_AND_HISTORY
OPAQUE_ID_DISTINGUISHABILITY_LIFECYCLE
OBSERVED_EVENT_REDACTION_AND_SEQUENCE
```

The authoritative mode requires a clean source tree whose commit equals the
expected target SHA when one is supplied.  ``--development`` runs the same
underlying evidence but can never report an authoritative gate result.
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
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "dist" / "m2-f-verification"
OUTPUT_MARKER = ".mtgml-m2-f-gates-output"
PINNED_TOOLCHAIN: dict[str, str | None] = {"channel": None}

GATE_SOUNDNESS = "SYNTHETIC_LEGAL_CHOICE_SOUNDNESS"
GATE_COMPLETENESS = "SYNTHETIC_LEGAL_CHOICE_COMPLETENESS"


@dataclass(frozen=True)
class EvidenceDefinition:
    kind: str
    name: str
    surface: str
    package: str | None = None


def rust(package: str, name: str, surface: str) -> EvidenceDefinition:
    return EvidenceDefinition("rust", name, surface, package)


def source(name: str, surface: str) -> EvidenceDefinition:
    return EvidenceDefinition("source", name, surface)


def python(name: str, surface: str) -> EvidenceDefinition:
    return EvidenceDefinition("python", name, surface)


GATE_TESTS: dict[str, tuple[EvidenceDefinition, ...]] = {
    GATE_SOUNDNESS: (
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::soundness::live_matrix_passes",
            "P subset of R",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::soundness::detects_illegal_extra",
            "illegal extra detection",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::soundness::detects_advertised_rejected",
            "advertised rejected detection",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::soundness::detects_numeric_bound_mutants",
            "numeric bound mutant detection",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::soundness::detects_cardinality_mutants",
            "cardinality mutant detection",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::soundness::detects_illegal_later_stage_choice",
            "illegal later-stage choice detection",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::soundness::request_soundness_expected_request_matches",
            "request soundness at every stage",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::unsatisfiable::authoritative_request_fails_closed",
            "unsatisfiable fail-closed",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::soundness::detects_wrong_visible_candidate_semantics",
            "wrong visible candidate semantics detected via mapper",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::budget::violations_fail_closed",
            "exploration budget violations fail closed",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::invariance::set_vs_sequence_mutant_matrix",
            "ChooseMany set vs Order sequence semantics enforced in IR",
        ),
        source("source_check::oracle_boundary_guard", "oracle boundary guard"),
    ),
    GATE_COMPLETENESS: (
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::completeness::live_matrix_exactly_once",
            "exactly one canonical path per choice",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::completeness::detects_missing_choice",
            "missing choice detection",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::completeness::duplicate_paths_are_rejected",
            "duplicate path rejection",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::completeness::detects_later_stage_omission",
            "later-stage omission detection",
        ),
        rust(
            "mtgml-conformance",
            "legal_space::gate_evidence::invariance::insertion_order_does_not_change_reference_space",
            "insertion order does not change canonical reference space",
        ),
        source("source_check::oracle_boundary_guard", "oracle boundary guard"),
    ),
}



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


def command_available(command: Sequence[str]) -> bool:
    return bool(command) and shutil.which(command[0]) is not None


def git_value(arguments: Sequence[str]) -> str:
    completed = run_command(("git", *arguments))
    if completed.returncode != 0:
        raise RuntimeError(completed.stdout.strip() or "git command failed")
    return completed.stdout.strip()


def tracked_source_fingerprint() -> str:
    listed = run_command(("git", "ls-files", "-z"))
    if listed.returncode != 0:
        raise RuntimeError("git ls-files failed")
    hasher = hashlib.sha256()
    for encoded in listed.stdout.encode("utf-8").split(b"\0"):
        if not encoded:
            continue
        relative = encoded.decode("utf-8")
        payload = (ROOT / relative).read_bytes()
        hasher.update(len(relative.encode("utf-8")).to_bytes(8, "big"))
        hasher.update(relative.encode("utf-8"))
        hasher.update(len(payload).to_bytes(8, "big"))
        hasher.update(payload)
    return hasher.hexdigest()


def source_snapshot() -> dict[str, Any]:
    try:
        status = git_value(("status", "--porcelain=v1", "--untracked-files=all"))
        return {
            "clean": not status,
            "git_status": status,
            "commit": git_value(("rev-parse", "HEAD")),
            "tree": git_value(("rev-parse", "HEAD^{tree}")),
            "fingerprint": tracked_source_fingerprint(),
        }
    except (OSError, RuntimeError) as error:
        return {"clean": False, "reason": str(error)}


def toolchain_snapshot() -> dict[str, Any]:
    try:
        expected_python = (ROOT / ".python-version").read_text(encoding="utf-8").strip()
        with (ROOT / "rust-toolchain.toml").open("rb") as handle:
            expected_rust = str(tomllib.load(handle)["toolchain"]["channel"])
    except (OSError, KeyError, tomllib.TOMLDecodeError) as error:
        PINNED_TOOLCHAIN["channel"] = None
        return {"status": "BLOCKED", "reason": f"toolchain policy unreadable: {error}"}
    PINNED_TOOLCHAIN["channel"] = expected_rust

    python_version = platform.python_version()
    python_ok = python_version == expected_python
    rust_results: dict[str, Any] = {}
    pinned = f"+{expected_rust}"
    for name, command in (
        ("rustc", ("rustc", pinned, "--version")),
        ("cargo", ("cargo", pinned, "--version")),
    ):
        if not command_available(command):
            rust_results[name] = {"status": "NOT_RUN"}
            continue
        output = run_command(command).stdout.strip()
        match = re.match(rf"^{name}\s+(\d+\.\d+\.\d+)", output.splitlines()[0] if output else "")
        reported = match.group(1) if match else None
        rust_results[name] = {
            "reported": reported,
            "status": "PASS" if reported == expected_rust else "FAIL",
        }
    statuses = [
        "PASS" if python_ok else "FAIL",
        *(item["status"] for item in rust_results.values()),
    ]
    overall = (
        "PASS"
        if all(status == "PASS" for status in statuses)
        else "BLOCKED"
        if "BLOCKED" in statuses
        else "FAIL"
    )
    return {
        "status": overall,
        "python": {"version": python_version, "expected": expected_python},
        "rust": {"expected": expected_rust, **rust_results},
    }


def prepare_output(output: Path) -> Path:
    relative = output.relative_to(ROOT)
    if "dist" not in relative.parts or output == ROOT:
        raise RuntimeError("M2.F verification output must remain below repository dist")
    if "verification" in relative.parts:
        raise RuntimeError("dist/verification is exclusively owned by release-candidate")
    if output.exists():
        marker = output / OUTPUT_MARKER
        if not marker.is_file():
            raise RuntimeError(f"refusing to replace unowned verification output: {output}")
        shutil.rmtree(output)
    logs = output / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    (output / OUTPUT_MARKER).write_text("owned by scripts/run_m2_f_gates.py\n", encoding="utf-8")
    return logs


def exact_rust_pass(output: str, returncode: int) -> bool:
    return bool(
        returncode == 0
        and re.search(r"running\s+1\s+test\b", output)
        and re.search(r"test result:\s+ok\.\s+1 passed;\s+0 failed\b", output)
    )


_PYTHON_OUTCOME = re.compile(
    r"(?<![\w-])(\d+)\s+(passed|failed|error|skipped|xfailed|xpassed|deselected|warnings?)\b"
)


def exact_python_pass(output: str, returncode: int, test_name: str) -> bool:
    """Fail-closed PASS detection: exit 0 alone is never sufficient.  The
    addressed test node must report PASSED, exactly one executed passing
    test must appear in the statistics summary, and no substitute outcome
    (skip/xfail/deselect/error/failure) may be counted."""
    if returncode != 0:
        return False
    if not re.search(re.escape(test_name.rsplit("::", 1)[-1]) + r"\s+PASSED\b", output):
        return False
    summaries = [line for line in output.splitlines() if _PYTHON_OUTCOME.search(line)]
    if len(summaries) != 1:
        return False
    passed = 0
    substitutes = 0
    for count, kind in _PYTHON_OUTCOME.findall(summaries[0]):
        if kind == "passed":
            passed += int(count)
        elif kind not in ("warning", "warnings"):
            substitutes += int(count)
    return passed == 1 and substitutes == 0


def execute_source_check(definition: EvidenceDefinition, logs: Path, index: int) -> dict[str, Any]:
    log_path = logs / f"{index:03d}-source-{definition.name.replace(':', '-')}.log"
    evidence = {
        "package": None,
        "test": definition.name,
        "surface": definition.surface,
        "command": ["runner", definition.name],
        "log": f"logs/{log_path.name}",
    }
    try:
        result = SOURCE_CHECKS[definition.name]()
    except AssertionError as error:
        evidence.update({"status": "FAIL", "returncode": 1, "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
    except (OSError, KeyError) as error:
        evidence.update({"status": "BLOCKED", "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
    else:
        evidence.update({"status": "PASS", "returncode": 0, "reason": result})
        log_path.write_text(result + "\n", encoding="utf-8")
    return evidence


def execute_test(definition: EvidenceDefinition, logs: Path, index: int) -> dict[str, Any]:
    if definition.kind == "python":
        return execute_python_test(definition, logs, index)
    assert definition.package is not None
    pinned = PINNED_TOOLCHAIN["channel"]
    cargo = ("cargo", f"+{pinned}") if pinned else ("cargo",)
    command = (
        *cargo,
        "test",
        "--package",
        definition.package,
        "--locked",
        "--lib",
        "--",
        definition.name,
        "--exact",
    )
    log_name = f"{index:03d}-{definition.package}-{definition.name.replace('::', '-')}.log"
    log_path = logs / log_name
    evidence: dict[str, Any] = {
        "package": definition.package,
        "test": definition.name,
        "surface": definition.surface,
        "command": list(command),
        "log": f"logs/{log_path.name}",
    }
    if not command_available(command):
        evidence.update({"status": "NOT_RUN", "reason": "cargo not found"})
        log_path.write_text("cargo not found\n", encoding="utf-8")
        return evidence
    try:
        completed = run_command(command)
    except OSError as error:
        evidence.update({"status": "BLOCKED", "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
        return evidence
    output = completed.stdout
    log_path.write_text(output, encoding="utf-8")
    passed = exact_rust_pass(output, completed.returncode)
    observed = 1 if re.search(r"running\s+1\s+test\b", output) else 0
    evidence.update(
        {
            "status": "PASS" if passed else "FAIL",
            "returncode": completed.returncode,
            "tests_observed": observed,
            "reason": "exact test passed" if passed else "exact test did not pass",
        }
    )
    return evidence


def execute_python_test(definition: EvidenceDefinition, logs: Path, index: int) -> dict[str, Any]:
    # Double verbosity: pytest.ini pins global `addopts = -q ...`, so a
    # single -v only reaches net-default output where pytest 9 hides both
    # the per-node status and the decorated statistics line;
    # exact_python_pass needs both as executable evidence.
    command = (sys.executable, "-m", "pytest", "-v", "-v", definition.name)
    log_name = f"{index:03d}-python-{definition.name.replace('/', '-').replace(':', '-')}.log"
    log_path = logs / log_name
    evidence: dict[str, Any] = {
        "package": None,
        "test": definition.name,
        "surface": definition.surface,
        "command": list(command),
        "log": f"logs/{log_path.name}",
    }
    try:
        completed = run_command(command)
    except OSError as error:
        evidence.update({"status": "BLOCKED", "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
        return evidence
    output = completed.stdout
    log_path.write_text(output, encoding="utf-8")
    passed = exact_python_pass(output, completed.returncode, definition.name)
    evidence.update(
        {
            "status": "PASS" if passed else "FAIL",
            "returncode": completed.returncode,
            "tests_observed": 1 if passed else 0,
            "reason": (
                "exact python test executed and passed"
                if passed
                else "exact python test did not execute and pass"
            ),
        }
    )
    return evidence


def check_oracle_boundary_guard() -> str:
    """Oracle independence: no production crate transitively pulls conformance,
    and the explorer does not import reference-oracle symbols."""
    import json as _json
    import subprocess as _sp
    meta = _sp.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if meta.returncode != 0:
        raise AssertionError(f"cargo metadata failed: {meta.stderr[:200]}")
    for pkg in _json.loads(meta.stdout)["packages"]:
        if pkg["name"] in ("mtgml-rules", "mtgml-environment"):
            for dep in pkg.get("dependencies", []):
                if dep["name"] == "mtgml-conformance":
                    raise AssertionError(f"{pkg['name']} pulls mtgml-conformance")
    src = (ROOT / "crates" / "mtgml-conformance" / "src" / "legal_space" / "explorer.rs").read_text(encoding="utf-8")
    for pattern in ["super::oracle", "crate::legal_space::oracle", "SCENARIO_COUNT_"]:
        if pattern in src:
            raise AssertionError(f"explorer.rs imports oracle symbol: {pattern}")
    return "oracle boundary clean: cargo graph + source scan pass"


SOURCE_CHECKS = {
    "source_check::oracle_boundary_guard": check_oracle_boundary_guard,
}


def aggregate(statuses: Iterable[str]) -> str:
    values = set(statuses)
    if values == {"PASS"}:
        return "PASS"
    if "BLOCKED" in values:
        return "BLOCKED"
    if "NOT_RUN" in values:
        return "NOT_RUN"
    return "FAIL"


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# M2.F Gate Verification",
        "",
        "Generated outside the reproducible source archive by `scripts/run_m2_f_gates.py`.",
        "",
        f"- Mode: **{report['mode']}**",
        f"- Source commit: `{report.get('source_commit')}`",
    ]
    for gate in report["gates"]:
        lines.append(f"- `{gate['name']}`: **{gate['gate_status']}**")
    lines.extend(["", "| Evidence | Status | Surface |", "|---|---:|---|"])
    for gate in report["gates"]:
        for item in gate["evidence"]:
            lines.append(f"| `{item['test']}` | **{item['status']}** | {item['surface']} |")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--development", action="store_true")
    parser.add_argument("--output-dir", type=Path, default=OUTPUT)
    parser.add_argument("--expect-commit", metavar="SHA", default=None)
    args = parser.parse_args()
    output = args.output_dir.resolve()
    try:
        logs = prepare_output(output)
    except (OSError, RuntimeError, ValueError) as error:
        print(f"BLOCKED: {error}")
        return 2

    before = source_snapshot()
    toolchains = toolchain_snapshot()

    gates: list[dict[str, Any]] = []
    index = 1
    for gate_name, definitions in GATE_TESTS.items():
        evidence = [
            (
                execute_source_check(definition, logs, index)
                if definition.kind == "source"
                else execute_test(definition, logs, index)
            )
            for definition in definitions
        ]
        for _ in definitions:
            index += 1
        underlying = aggregate(item["status"] for item in evidence)
        gates.append({"name": gate_name, "underlying": underlying, "evidence": evidence})

    after = source_snapshot()
    if before.get("clean") and after.get("clean"):
        unchanged = (
            before.get("commit") == after.get("commit")
            and before.get("tree") == after.get("tree")
            and before.get("fingerprint") == after.get("fingerprint")
        )
        source_identity_status = "PASS" if unchanged else "FAIL"
    else:
        source_identity_status = "BLOCKED" if not before.get("clean") else "FAIL"

    expected_commit_note = None
    if (
        args.expect_commit
        and not args.development
        and source_identity_status == "PASS"
        and before.get("commit") != args.expect_commit
    ):
        source_identity_status = "FAIL"
        expected_commit_note = (
            f"source head {before.get('commit')} does not equal the "
            f"expected target SHA {args.expect_commit}"
        )

    for gate in gates:
        if args.development:
            gate["gate_status"] = "NOT_RUN"
        elif source_identity_status != "PASS":
            gate["gate_status"] = source_identity_status
        else:
            gate["gate_status"] = aggregate(
                (gate["underlying"], toolchains.get("status", "BLOCKED"))
            )

    overall_gate = aggregate(gate["gate_status"] for gate in gates)
    report = {
        "generated_at": datetime.now(UTC).isoformat(),
        "mode": "development" if args.development else "authoritative",
        "milestone": "M2.F",
        "reporter": "scripts/run_m2_f_gates.py",
        "source_commit": before.get("commit"),
        "expected_commit": args.expect_commit,
        "expected_commit_note": expected_commit_note,
        "source_tree_identity": {
            "status": source_identity_status,
            "before": before,
            "after": after,
        },
        "toolchains": toolchains,
        "gates": [
            {key: value for key, value in gate.items() if key != "underlying"} for gate in gates
        ],
        "host": {
            "platform": platform.platform(),
            "node": platform.node(),
            "python": sys.executable,
        },
    }
    (output / "m2-f-gate-results.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (output / "M2_F_GATES.md").write_text(render_markdown(report), encoding="utf-8")
    print(
        json.dumps(
            {
                "mode": report["mode"],
                "source_commit": report["source_commit"],
                "source_identity": source_identity_status,
                "gates": {gate["name"]: gate["gate_status"] for gate in gates},
                "overall": overall_gate,
                "output_dir": str(output),
            },
            sort_keys=True,
        )
    )
    if args.development:
        underlying_all = aggregate(gate["underlying"] for gate in gates)
        return 0 if underlying_all == "PASS" and toolchains.get("status") == "PASS" else 2
    return 0 if overall_gate == "PASS" else 2


if __name__ == "__main__":
    raise SystemExit(main())
