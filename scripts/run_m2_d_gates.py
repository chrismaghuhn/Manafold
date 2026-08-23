#!/usr/bin/env python3
"""Execute the three M2.D executable gates on an exact clean source head.

Owned gates:

```text
VISIBLE_DECISION_CANONICAL_ORDER_AND_IDENTITY
PLAYER_PROJECTION_PERSPECTIVE_COHERENCE
PLAYER_SAFE_ERROR_MAPPING_AND_NONDISCLOSURE
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
OUTPUT = ROOT / "dist" / "m2-d-verification"
OUTPUT_MARKER = ".mtgml-m2-d-gates-output"
PINNED_TOOLCHAIN: dict[str, str | None] = {"channel": None}

GATE_VISIBLE = "VISIBLE_DECISION_CANONICAL_ORDER_AND_IDENTITY"
GATE_COHERENCE = "PLAYER_PROJECTION_PERSPECTIVE_COHERENCE"
GATE_ERRORS = "PLAYER_SAFE_ERROR_MAPPING_AND_NONDISCLOSURE"


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
    GATE_VISIBLE: (
        rust(
            "mtgml-decision",
            "tests::candidate_ordering_v1_exact_matrix",
            "canonical candidate order and dense IDs (M2.C guarantee)",
        ),
        rust(
            "mtgml-environment",
            "tests::visible_decision_exposes_no_trusted_identities_or_internals",
            "visible decision carries no trusted identities or internals",
        ),
        rust(
            "mtgml-rules",
            "tests::candidate_order_independent_of_global_allocator_history",
            "visible surface independent of global allocator history",
        ),
        rust(
            "mtgml-environment",
            "tests::projection_bytes_survive_checkpoint_restore_and_equal_forks",
            "visible decision bytes survive restore/equal fork exactly",
        ),
        source(
            "source_check::single_count_authority",
            "single program-interval authority consumed by the kernel",
        ),
    ),
    GATE_COHERENCE: (
        rust(
            "mtgml-observation",
            "tests::information_state_input_excludes_trusted_fields",
            "information digest input excludes trusted fields",
        ),
        rust(
            "mtgml-environment",
            "tests::projection_perspective_and_revision_coherence_matrix",
            "perspective/revision coherence across all read surfaces",
        ),
        rust(
            "mtgml-environment",
            "tests::episode_status_does_not_change_the_information_digest",
            "episode status separate from information-state digest identity",
        ),
        rust(
            "mtgml-environment",
            "tests::projection_reads_are_pure_and_order_independent",
            "projection purity and deterministic read ordering",
        ),
        rust(
            "mtgml-environment",
            "tests::observation_equals_information_state_current_observation_bytes",
            "observation equals information-state current observation bytes",
        ),
    ),
    GATE_ERRORS: (
        rust(
            "mtgml-decision",
            "tests::submission_validation_precedence_matrix",
            "compound validation precedence is deterministic",
        ),
        rust(
            "mtgml-environment",
            "tests::malformed_wire_bytes_never_reach_the_semantic_endpoint",
            "layer A: wire failures never invoke the semantic endpoint",
        ),
        rust(
            "mtgml-environment",
            "tests::typed_rejection_codes_matrix",
            "layer B: every closed typed rejection code endpoint-driven",
        ),
        rust(
            "mtgml-environment",
            "tests::internal_failures_surface_only_service_unavailable",
            "layer C: internal failures surface only service_unavailable",
        ),
        rust(
            "mtgml-environment",
            "tests::request_existence_is_not_an_error_oracle",
            "request existence does not alter the public error class",
        ),
        rust(
            "mtgml-environment",
            "tests::player_api_errors_do_not_render_trusted_values",
            "public failure rendering stays closed without trusted detail",
        ),
        rust(
            "mtgml-wire",
            "tests::every_shared_negative_fixture_is_rejected_with_the_expected_code",
            "shared negative fixture corpus rejects with exact codes",
        ),
        python(
            "python/tests/test_player_api.py::PlayerApiTests::test_v2_public_boundary_excludes_privileged_fields",
            "Python player boundary excludes privileged fields",
        ),
        python(
            "python/tests/test_player_api.py::PlayerStepSubmissionContractTests::"
            "test_step_level_rejection_invariants",
            "Python step-level rejection invariants enforce the closed semantics",
        ),
        python(
            "python/tests/test_wire_contracts.py::SharedFixtureTests::"
            "test_every_negative_fixture_is_rejected_with_expected_code",
            "Python rejects every shared negative fixture with the expected code",
        ),
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
        raise RuntimeError("M2.D verification output must remain below repository dist")
    if "verification" in relative.parts:
        raise RuntimeError("dist/verification is exclusively owned by release-candidate")
    if output.exists():
        marker = output / OUTPUT_MARKER
        if not marker.is_file():
            raise RuntimeError(f"refusing to replace unowned verification output: {output}")
        shutil.rmtree(output)
    logs = output / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    (output / OUTPUT_MARKER).write_text("owned by scripts/run_m2_d_gates.py\n", encoding="utf-8")
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


def check_single_count_authority() -> str:
    """The synthetic count interval has exactly one definition, in the state
    authority beside the frozen payload; the rules kernel consumes it."""
    state_source = (ROOT / "crates" / "mtgml-state" / "src" / "m2_shape.rs").read_text(
        encoding="utf-8"
    )
    if "pub const SYNTHETIC_COUNT_MIN: u32 = 0;" not in state_source:
        raise AssertionError("state authority lost the program interval definition")
    rules_dir = ROOT / "crates" / "mtgml-rules" / "src"
    for path in rules_dir.glob("*.rs"):
        text = path.read_text(encoding="utf-8")
        if "const SYNTHETIC_COUNT_M" in text:
            raise AssertionError(f"rules crate redefined the program interval: {path.name}")
    rules_lib = (rules_dir / "synthetic.rs").read_text(encoding="utf-8")
    if "pub use mtgml_state::{SYNTHETIC_COUNT_MAX, SYNTHETIC_COUNT_MIN};" not in rules_lib:
        raise AssertionError("rules kernel no longer consumes the state-owned interval")
    # Verify the actual consumer sites, not just the import: the stage
    # request construction and the acceptance range check must reference
    # the state-owned constants.
    consumers = [
        "minimum: i64::from(SYNTHETIC_COUNT_MIN)",
        "maximum: i64::from(SYNTHETIC_COUNT_MAX)",
        "(SYNTHETIC_COUNT_MIN..=SYNTHETIC_COUNT_MAX).contains",
    ]
    for site in consumers:
        if site not in rules_lib:
            raise AssertionError(f"kernel consumer site missing: {site}")
    return "program interval defined once in mtgml-state and consumed by the kernel"


SOURCE_CHECKS = {
    "source_check::single_count_authority": check_single_count_authority,
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
        "# M2.D Gate Verification",
        "",
        "Generated outside the reproducible source archive by `scripts/run_m2_d_gates.py`.",
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
        "milestone": "M2.D",
        "reporter": "scripts/run_m2_d_gates.py",
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
    (output / "m2-d-gate-results.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (output / "M2_D_GATES.md").write_text(render_markdown(report), encoding="utf-8")
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
