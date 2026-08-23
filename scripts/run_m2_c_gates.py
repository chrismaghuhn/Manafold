#!/usr/bin/env python3
"""Execute the two M2.C executable gates on an exact clean source head.

Owned gates:

```text
CLOSED_DECISION_FAMILY_EXACTNESS
SERIALIZED_CONTINUATION_LIFECYCLE
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
OUTPUT = ROOT / "dist" / "m2-c-verification"
OUTPUT_MARKER = ".mtgml-m2-c-gates-output"
PINNED_TOOLCHAIN: dict[str, str | None] = {"channel": None}

GATE_FAMILY = "CLOSED_DECISION_FAMILY_EXACTNESS"
GATE_CONTINUATION = "SERIALIZED_CONTINUATION_LIFECYCLE"


@dataclass(frozen=True)
class EvidenceDefinition:
    kind: str
    name: str
    surface: str
    package: str | None = None


def rust(package: str, name: str, surface: str) -> EvidenceDefinition:
    return EvidenceDefinition("rust", name, surface, package)


GATE_TESTS: dict[str, tuple[EvidenceDefinition, ...]] = {
    GATE_FAMILY: (
        rust(
            "mtgml-decision",
            "tests::candidate_ordering_v1_exact_matrix",
            "numeric ordering, dense IDs, duplicate/order/cardinality rejection",
        ),
        rust(
            "mtgml-decision",
            "tests::candidate_generation_is_insertion_and_trusted_id_independent",
            "candidate order independent of insertion order and trusted bindings",
        ),
        rust(
            "mtgml-decision",
            "tests::duplicate_public_keys_fail_closed_even_with_distinct_trusted_bindings",
            "duplicate public ordering key fails closed without tie-breaking",
        ),
        rust(
            "mtgml-rules",
            "tests::synthetic_m2_choose_one_returns_authoritative_transition_product",
            "ChooseOne entry product and continuation creation",
        ),
        rust(
            "mtgml-rules",
            "tests::choose_number_stage_bounds_matrix",
            "ChooseNumber inclusive bounds, wrong variant, stale rejections",
        ),
        rust(
            "mtgml-rules",
            "tests::choose_many_stage_cardinality_matrix",
            "ChooseMany cardinalities, canonical set syntax, membership rejection",
        ),
        rust(
            "mtgml-rules",
            "tests::order_stage_semantics_matrix",
            "Order semantic sequence preservation and rejection matrix",
        ),
        rust(
            "mtgml-rules",
            "tests::unsatisfiable_authoritative_requests_are_internal_failures",
            "unsatisfiable authoritative requests fail closed before players",
        ),
        rust(
            "mtgml-rules",
            "tests::rejected_family_answers_preserve_the_complete_fingerprint",
            "family rejection complete nonmutation",
        ),
        rust(
            "mtgml-rules",
            "tests::optional_empty_chain_keeps_every_stage_explicit",
            "optional empty selection keeps every player stage explicit",
        ),
        rust(
            "mtgml-decision",
            "tests::closed_family_domain_boundaries_matrix",
            "inclusive min/max boundaries incl. maximum above candidate count",
        ),
        rust(
            "mtgml-decision",
            "tests::candidate_id_overflow_is_rejected",
            "CandidateId u32 overflow rejection",
        ),
        rust(
            "mtgml-rules",
            "tests::candidate_order_independent_of_global_allocator_history",
            "candidate order independent of global allocator history and RNG state",
        ),
        rust(
            "mtgml-environment",
            "tests::accepted_endpoint_submission_commits_v3_state_delta_and_replay",
            "endpoint chain commits delta/replay products atomically",
        ),
    ),
    GATE_CONTINUATION: (
        rust(
            "mtgml-state",
            "tests::assembly_payload_stage_invariants_are_enforced",
            "assembly stage payload invariants fail closed",
        ),
        rust(
            "mtgml-state",
            "tests::continuation_pending_program_coherence_matrix",
            "pending request must express exactly the referenced stage program",
        ),
        rust(
            "mtgml-state",
            "tests::pending_decision_must_reference_an_existing_continuation",
            "missing continuation reference rejected",
        ),
        rust(
            "mtgml-state",
            "tests::continuation_actor_must_own_the_referenced_request",
            "referenced continuation must belong to its request actor",
        ),
        rust(
            "mtgml-state",
            "tests::continuation_stage_must_match_its_payload",
            "stage index must match the frozen payload stage",
        ),
        rust(
            "mtgml-state",
            "tests::continuation_revision_must_not_be_future_dated",
            "future-dated continuation revisions rejected",
        ),
        rust(
            "mtgml-rules",
            "tests::continuation_chain_advances_with_fresh_explicit_identities",
            "one ContinuationId, fresh DecisionId/PlayerDecisionIdV1 per stage",
        ),
        rust(
            "mtgml-environment",
            "tests::continuation_checkpoint_restore_roundtrip_preserves_the_chain",
            "mid-chain checkpoint contains the continuation; restore advances equally",
        ),
        rust(
            "mtgml-environment",
            "tests::continuation_fork_equal_input_produces_equal_results",
            "fork parity for equal continuation input",
        ),
        rust(
            "mtgml-environment",
            "tests::continuation_replay_full_chain_parity",
            "replay reproduces create/advance/advance/complete exactly",
        ),
        rust(
            "mtgml-environment",
            "tests::stale_stage_response_is_rejected_without_any_mutation",
            "stale-stage response rejection preserves every identity value",
        ),
        rust(
            "mtgml-environment",
            "tests::order_permutations_bind_distinct_replay_identity",
            "semantic order permutations bind distinct replay responses",
        ),
        rust(
            "mtgml-replay",
            "tests::replay_v3_empty_accepted_rejected_identity_matrix",
            "ReplayV3 identity chaining remains intact under continuations",
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
        raise RuntimeError("M2.C verification output must remain below repository dist")
    if "verification" in relative.parts:
        raise RuntimeError("dist/verification is exclusively owned by release-candidate")
    if output.exists():
        marker = output / OUTPUT_MARKER
        if not marker.is_file():
            raise RuntimeError(f"refusing to replace unowned verification output: {output}")
        shutil.rmtree(output)
    logs = output / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    (output / OUTPUT_MARKER).write_text("owned by scripts/run_m2_c_gates.py\n", encoding="utf-8")
    return logs


def exact_rust_pass(output: str, returncode: int) -> bool:
    return bool(
        returncode == 0
        and re.search(r"running\s+1\s+test\b", output)
        and re.search(r"test result:\s+ok\.\s+1 passed;\s+0 failed\b", output)
    )


def execute_test(definition: EvidenceDefinition, logs: Path, index: int) -> dict[str, Any]:
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
        "# M2.C Gate Verification",
        "",
        "Generated outside the reproducible source archive by `scripts/run_m2_c_gates.py`.",
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
        evidence = [execute_test(definition, logs, index) for definition in definitions]
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
        "milestone": "M2.C",
        "reporter": "scripts/run_m2_c_gates.py",
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
    (output / "m2-c-gate-results.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (output / "M2_C_GATES.md").write_text(render_markdown(report), encoding="utf-8")
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
