#!/usr/bin/env python3
"""Execute the single M2.B structural/versioning cut gate.

The authoritative mode requires a clean exact source head.  ``--development``
is useful while preparing a commit, but it can only report ``NOT_RUN`` for the
gate even when every underlying check succeeds.
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
STARTING_SHA = "a4e769eb940611d34df05fc79effd9430891d897"
OUTPUT = ROOT / "dist" / "m2-b-verification"
OUTPUT_MARKER = ".mtgml-m2-b-contract-cut-output"
GATE_NAME = "M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT"


@dataclass(frozen=True)
class EvidenceDefinition:
    kind: str
    name: str
    surface: str
    package: str | None = None


def rust(package: str, name: str, surface: str) -> EvidenceDefinition:
    return EvidenceDefinition("rust", name, surface, package)


def python(node_id: str, surface: str) -> EvidenceDefinition:
    return EvidenceDefinition("python", node_id, surface)


def source(name: str, surface: str) -> EvidenceDefinition:
    return EvidenceDefinition("source", name, surface)


# Keep this table explicit and reviewable.  It is the only owner of the Issue
# #49 executable gate; later M2.Final closure may reuse these tests but does
# not redefine this slice's evidence.
GATE_TESTS: dict[str, tuple[EvidenceDefinition, ...]] = {
    GATE_NAME: (
        rust(
            "mtgml-model",
            "tests::m2_b_candidate_id_is_u32_and_v3_digest_is_raw",
            "u32 CandidateId and raw V3 digest semantics",
        ),
        rust(
            "mtgml-persistence",
            "tests::canonical_cbor_v1_complete_profile_matrix",
            "complete canonical CBOR profile",
        ),
        rust(
            "mtgml-persistence",
            "tests::digest_envelope_v1_known_answer_matrix",
            "exact digest-envelope framing and hash",
        ),
        rust(
            "mtgml-persistence",
            "tests::checkpoint_digest_v3_known_answer",
            "single-owner CheckpointDigestV3 known answer",
        ),
        rust(
            "mtgml-decision",
            "tests::candidate_ordering_v1_exact_matrix",
            "numeric ordering, dense IDs, duplicate/order/cardinality rejection",
        ),
        rust(
            "mtgml-decision",
            "tests::candidate_id_overflow_is_rejected",
            "CandidateId u32 overflow rejection",
        ),
        rust(
            "mtgml-state",
            "tests::full_state_digest_v3_known_answer",
            "FullStateDigestV3 known answer",
        ),
        rust(
            "mtgml-state",
            "tests::m2_b_full_state_digest_v3_mutation_matrix",
            "authoritative V3 digest mutation matrix",
        ),
        rust(
            "mtgml-state",
            "tests::state_delta_uses_full_state_digest_v3",
            "StateDelta V3 identity and exact reapplication",
        ),
        rust(
            "mtgml-state",
            "tests::deterministic_structural_identity_repeats_exactly",
            "deterministic structural identity repeat",
        ),
        rust(
            "mtgml-rules",
            "tests::synthetic_m2_choose_one_returns_authoritative_transition_product",
            "authoritative TransitionResult product",
        ),
        rust(
            "mtgml-observation",
            "tests::information_state_input_excludes_trusted_fields",
            "information digest input exclusion boundary",
        ),
        rust(
            "mtgml-wire",
            "tests::information_state_digest_v2_known_answer",
            "single-owner canonical InformationStateDigestV2 bytes",
        ),
        rust(
            "mtgml-environment",
            "tests::checkpoint_v3_validation_and_restore_nonmutation_matrix",
            "CheckpointV3 validation and restore nonmutation",
        ),
        rust(
            "mtgml-environment",
            "tests::synthetic_endpoint_returns_v2_surface",
            "synthetic endpoint V2 public surface",
        ),
        rust(
            "mtgml-replay",
            "tests::replay_v3_empty_accepted_rejected_identity_matrix",
            "ReplayV3 empty/accepted/rejected identity chain",
        ),
        source(
            "source_check::v1_v2_fixtures_are_immutable",
            "complete baseline fixture inventory and byte immutability",
        ),
        source(
            "source_check::no_current_v2_producer",
            "current V3 producer boundary and staging retirement",
        ),
        python(
            "python/tests/test_persistence_codec.py::test_cross_language_mechanical_golden_vectors",
            "Python persisted codec golden parity",
        ),
        python(
            "python/tests/test_persistence_codec.py::test_cross_language_mechanical_negative_categories",
            "Python mechanical negative categories",
        ),
        python(
            "python/tests/test_schema_parity.py::test_m2_b_detached_schema_fixtures",
            "V2/V3 detached schema parity",
        ),
        python(
            "python/tests/test_player_api.py::test_v2_public_boundary_excludes_privileged_fields",
            "player-safe Python boundary",
        ),
        python("scripts/generate_contracts.py --check", "generated contract drift check"),
        python("scripts/check_rust_source_structure.py", "Rust source structure check"),
        python("scripts/check_documentation.py", "documentation contract check"),
        python("scripts/validate_schemas.py", "wire schema validation"),
        python("scripts/validate_golden_path.py", "golden path structural validation"),
    )
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
    return bool(command) and (command[0] == sys.executable or shutil.which(command[0]) is not None)


def command_text(command: Sequence[str]) -> str:
    return " ".join(command)


def git_value(arguments: Sequence[str]) -> str:
    completed = run_command(("git", *arguments))
    if completed.returncode != 0:
        raise RuntimeError(completed.stdout.strip() or command_text(("git", *arguments)))
    return completed.stdout.strip()


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


def source_snapshot() -> dict[str, Any]:
    try:
        status = git_value(("status", "--porcelain=v1", "--untracked-files=all"))
        return {
            "status": "PASS" if not status else "BLOCKED",
            "clean": not status,
            "git_status": status,
            "commit": git_value(("rev-parse", "HEAD")),
            "tree": git_value(("rev-parse", "HEAD^{tree}")),
            "fingerprint": tracked_source_fingerprint(),
        }
    except (OSError, RuntimeError) as error:
        return {"status": "BLOCKED", "reason": str(error), "clean": False}


def toolchain_snapshot() -> dict[str, Any]:
    try:
        expected_python = (ROOT / ".python-version").read_text(encoding="utf-8").strip()
        with (ROOT / "rust-toolchain.toml").open("rb") as handle:
            expected_rust = str(tomllib.load(handle)["toolchain"]["channel"])
    except (OSError, KeyError, tomllib.TOMLDecodeError) as error:
        return {"status": "BLOCKED", "reason": f"toolchain policy unreadable: {error}"}

    python_version = platform.python_version()
    python_result = run_command((sys.executable, "--version"))
    python_status = (
        "PASS" if python_result.returncode == 0 and python_version == expected_python else "FAIL"
    )
    rust: dict[str, Any] = {}
    pinned = f"+{expected_rust}"
    for name, command in (
        ("rustc", ("rustc", pinned, "--version")),
        ("cargo", ("cargo", pinned, "--version")),
        ("active_toolchain", ("rustup", "show", "active-toolchain")),
    ):
        if not command_available(command):
            rust[name] = {"status": "NOT_RUN", "command": list(command), "reason": "tool not found"}
            continue
        try:
            result = run_command(command)
        except OSError as error:
            rust[name] = {"status": "BLOCKED", "command": list(command), "reason": str(error)}
            continue
        output = result.stdout.strip()
        if name == "active_toolchain":
            match = re.match(
                r"^(\d+\.\d+\.\d+)(?:-|\s|$)", output.splitlines()[0] if output else ""
            )
        else:
            match = re.match(
                rf"^{name}\s+(\d+\.\d+\.\d+)", output.splitlines()[0] if output else ""
            )
        reported = match.group(1) if match else None
        rust[name] = {
            "status": "PASS" if result.returncode == 0 and reported == expected_rust else "FAIL",
            "command": list(command),
            "returncode": result.returncode,
            "reported": reported,
            "expected": expected_rust,
            "output": output,
        }
    statuses = [python_status, *(item["status"] for item in rust.values())]
    overall = (
        "PASS"
        if all(status == "PASS" for status in statuses)
        else (
            "BLOCKED" if "BLOCKED" in statuses else "NOT_RUN" if "NOT_RUN" in statuses else "FAIL"
        )
    )
    return {
        "status": overall,
        "python": {"status": python_status, "version": python_version, "expected": expected_python},
        "rust": rust,
    }


def prepare_output(output: Path) -> Path:
    relative = output.relative_to(ROOT)
    if "dist" not in relative.parts or output == ROOT:
        raise RuntimeError("M2.B verification output must remain below repository dist")
    if "verification" in relative.parts:
        raise RuntimeError("dist/verification is exclusively owned by release-candidate")
    if output.exists():
        marker = output / OUTPUT_MARKER
        if not marker.is_file():
            raise RuntimeError(f"refusing to replace unowned verification output: {output}")
        shutil.rmtree(output)
    logs = output / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    (output / OUTPUT_MARKER).write_text(
        "owned by scripts/run_m2_b_contract_cut.py\n", encoding="utf-8"
    )
    return logs


def exact_rust_pass(output: str, returncode: int) -> bool:
    return bool(
        returncode == 0
        and re.search(r"running\s+1\s+test\b", output)
        and re.search(r"test result:\s+ok\.\s+1 passed;\s+0 failed\b", output)
    )


def execute_command(definition: EvidenceDefinition, logs: Path, index: int) -> dict[str, Any]:
    if definition.kind == "rust":
        assert definition.package is not None
        command = (
            "cargo",
            "test",
            "--package",
            definition.package,
            "--locked",
            "--lib",
            "--",
            definition.name,
            "--exact",
        )
    elif definition.kind == "python" and definition.name.startswith("python/tests/"):
        command = (sys.executable, "-m", "pytest", "-q", definition.name)
    else:
        command = (sys.executable, *definition.name.split())
    log_name = (
        f"{index:03d}-{definition.kind}-{definition.name.replace('/', '-').replace(':', '-')}.log"
    )
    log_path = logs / log_name
    evidence: dict[str, Any] = {
        "kind": definition.kind,
        "name": definition.name,
        "surface": definition.surface,
        "command": list(command),
        "log": f"logs/{log_path.name}",
    }
    if not command_available(command):
        evidence.update({"status": "NOT_RUN", "reason": f"{command[0]} not found"})
        log_path.write_text(f"{command[0]} not found\n", encoding="utf-8")
        return evidence
    try:
        completed = run_command(command)
    except OSError as error:
        evidence.update({"status": "BLOCKED", "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
        return evidence
    output = completed.stdout
    log_path.write_text(output, encoding="utf-8")
    passed = (
        exact_rust_pass(output, completed.returncode)
        if definition.kind == "rust"
        else completed.returncode == 0
    )
    evidence.update(
        {
            "status": "PASS" if passed else "FAIL",
            "returncode": completed.returncode,
            "reason": "exact check passed" if passed else "check did not pass",
        }
    )
    return evidence


def baseline_json(relative: str) -> Any:
    raw = git_value(("show", f"{STARTING_SHA}:{relative}"))
    return json.loads(raw)


def check_historical_inventory() -> str:
    inventory_path = ROOT / "wire" / "historical" / "v1-v2-fixtures.json"
    inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
    if inventory.get("source_commit") != STARTING_SHA:
        raise AssertionError("historical inventory source SHA changed")
    entries = inventory.get("fixtures")
    if not isinstance(entries, list):
        raise AssertionError("historical inventory fixtures are not a list")
    actual: dict[tuple[str, str, str], dict[str, Any]] = {}
    for entry in entries:
        key = (str(entry["manifest"]), str(entry["contract"]), str(entry["path"]))
        if key in actual:
            raise AssertionError(f"duplicate historical inventory entry: {key}")
        actual[key] = entry
    baseline: dict[tuple[str, str, str], None] = {}
    for kind in ("golden", "negative"):
        manifest = baseline_json(f"wire/{kind}/manifest.json")
        for entry in manifest["fixtures"]:
            key = (kind, str(entry["contract"]), str(entry["path"]))
            if key in baseline:
                raise AssertionError(f"duplicate baseline fixture entry: {key}")
            baseline[key] = None
    if set(actual) != set(baseline):
        raise AssertionError(
            "historical inventory set differs: "
            f"missing={set(baseline) - set(actual)}, "
            f"extra={set(actual) - set(baseline)}"
        )
    for kind, contract, path in baseline:
        baseline_bytes = run_command(
            ("git", "show", f"{STARTING_SHA}:wire/{kind}/{path}")
        ).stdout.encode("utf-8")
        current_path = ROOT / "wire" / kind / path
        if not current_path.is_file() or current_path.read_bytes() != baseline_bytes:
            raise AssertionError(f"historical fixture bytes changed: {kind}/{path}")
        expected = hashlib.sha256(baseline_bytes).hexdigest()
        if actual[(kind, contract, path)].get("sha256") != expected:
            raise AssertionError(f"historical fixture hash mismatch: {kind}/{path}")
    return f"covered {len(baseline)} baseline fixtures from {STARTING_SHA}"


def source_files() -> Iterable[Path]:
    for root in (ROOT / "crates", ROOT / "python", ROOT / "schemas", ROOT / "wire"):
        if not root.exists():
            continue
        yield from (
            path for path in root.rglob("*") if path.is_file() and "__pycache__" not in path.parts
        )


def current_source_files() -> Iterable[Path]:
    for relative in (
        "crates/mtgml-state/src",
        "crates/mtgml-rules/src",
        "crates/mtgml-environment/src",
        "crates/mtgml-replay/src/v3.rs",
    ):
        root = ROOT / relative
        if root.is_file():
            yield root
        elif root.exists():
            yield from root.rglob("*.rs")


def contains_exact_identifier(text: str, identifier: str) -> bool:
    return re.search(rf"(?<![A-Za-z0-9_]){re.escape(identifier)}(?![A-Za-z0-9_])", text) is not None


def check_no_current_v2_producer() -> str:
    forbidden = (
        "EngineStateV2",
        "LegacyEngineStateV2",
        "EnvironmentCheckpointV2",
        "ReplayControlV1",
    )
    forbidden_hits: list[str] = []
    for path in current_source_files():
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        for token in forbidden:
            if contains_exact_identifier(text, token):
                forbidden_hits.append(f"{path.relative_to(ROOT)}:{token}")
    if forbidden_hits:
        raise AssertionError("forbidden current version names: " + ", ".join(forbidden_hits))
    current_layers = {
        "crates/mtgml-state/src": ("FullStateDigestV2", "AuthoritativeReplayV2"),
        "crates/mtgml-rules/src": ("FullStateDigestV2", "DecisionResponse"),
        "crates/mtgml-environment/src": (
            "FullStateDigestV2",
            "AuthoritativeReplayV2",
            "DecisionResponse",
        ),
    }
    for relative, tokens in current_layers.items():
        root = ROOT / relative
        for path in root.rglob("*.rs"):
            text = path.read_text(encoding="utf-8")
            for token in tokens:
                if contains_exact_identifier(text, token):
                    raise AssertionError(
                        f"current producer token {token} in {path.relative_to(ROOT)}"
                    )
    if (ROOT / "wire" / "staging").exists():
        raise AssertionError("temporary wire staging directory remains")
    return (
        "current state/rules/environment producers are V3-only; "
        "detached V1/V2 readers remain isolated"
    )


SOURCE_CHECKS: dict[str, Callable[[], str]] = {
    "source_check::v1_v2_fixtures_are_immutable": check_historical_inventory,
    "source_check::no_current_v2_producer": check_no_current_v2_producer,
}


def execute_source_check(definition: EvidenceDefinition, logs: Path, index: int) -> dict[str, Any]:
    log_path = (
        logs / f"{index:03d}-source-{definition.name.replace(':', '-').replace('/', '-')}.log"
    )
    evidence: dict[str, Any] = {
        "kind": "source",
        "name": definition.name,
        "surface": definition.surface,
        "command": ["runner", definition.name],
        "log": f"logs/{log_path.name}",
    }
    try:
        result = SOURCE_CHECKS[definition.name]()
    except (AssertionError, KeyError) as error:
        evidence.update({"status": "FAIL", "returncode": 1, "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        evidence.update({"status": "BLOCKED", "reason": str(error)})
        log_path.write_text(str(error) + "\n", encoding="utf-8")
    else:
        evidence.update({"status": "PASS", "returncode": 0, "reason": result})
        log_path.write_text(result + "\n", encoding="utf-8")
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
        "# M2.B Contract Cut Verification",
        "",
        "Generated outside the reproducible source archive by `scripts/run_m2_b_contract_cut.py`.",
        "",
        f"- Mode: **{report['mode']}**",
        f"- Source commit: `{report.get('source_commit')}`",
        f"- Gate: **{report['gate_status']}**",
        f"- Toolchains: **{report['toolchains'].get('status')}**",
        "",
        "| Evidence | Status | Surface |",
        "|---|---:|---|",
    ]
    for item in report["gates"][0]["evidence"]:
        lines.append(f"| `{item['name']}` | **{item['status']}** | {item['surface']} |")
    lines.extend(["", "Development mode never produces an authoritative PASS.", ""])
    return "\n".join(lines)


def render_blockers(report: dict[str, Any]) -> str:
    lines = [
        "# M2.B Contract Cut Blockers",
        "",
        "| Evidence | Status | Reason |",
        "|---|---:|---|",
    ]
    for item in report["gates"][0]["evidence"]:
        if item["status"] != "PASS":
            lines.append(f"| `{item['name']}` | **{item['status']}** | {item.get('reason', '')} |")
    if report["gate_status"] == "PASS":
        lines.append("| none | **PASS** | every named M2.B check passed on a clean exact head |")
    lines.extend(["", f"`{GATE_NAME}`: **{report['gate_status']}**", ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--development",
        action="store_true",
        help="run underlying checks but force gate status NOT_RUN",
    )
    parser.add_argument(
        "--output-dir", type=Path, default=OUTPUT, help="external sibling output directory"
    )
    args = parser.parse_args()
    output = args.output_dir.resolve()
    try:
        logs = prepare_output(output)
    except (OSError, RuntimeError, ValueError) as error:
        print(f"BLOCKED: {error}")
        return 2

    before = source_snapshot()
    toolchains = toolchain_snapshot()
    evidence: list[dict[str, Any]] = []
    for index, definition in enumerate(GATE_TESTS[GATE_NAME], start=1):
        evidence.append(
            execute_source_check(definition, logs, index)
            if definition.kind == "source"
            else execute_command(definition, logs, index)
        )
    after = source_snapshot()
    underlying = aggregate(item["status"] for item in evidence)
    source_identity_status = (
        "PASS"
        if before.get("clean")
        and after.get("clean")
        and before.get("commit") == after.get("commit")
        and before.get("tree") == after.get("tree")
        else ("BLOCKED" if not before.get("clean") else "FAIL")
    )
    if args.development:
        gate_status = "NOT_RUN"
    elif source_identity_status != "PASS":
        gate_status = source_identity_status
    else:
        gate_status = aggregate((underlying, toolchains.get("status", "BLOCKED")))
    report = {
        "generated_at": datetime.now(UTC).isoformat(),
        "mode": "development" if args.development else "authoritative",
        "milestone": "M2.B",
        "reporter": "scripts/run_m2_b_contract_cut.py",
        "source_commit": before.get("commit"),
        "source_tree_identity": {
            "status": source_identity_status,
            "before": before,
            "after": after,
        },
        "toolchains": toolchains,
        "gates": [{"name": GATE_NAME, "status": gate_status, "evidence": evidence}],
        "underlying_status": underlying,
        "gate_status": gate_status,
        "M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT": gate_status,
        "host": {
            "platform": platform.platform(),
            "node": platform.node(),
            "python": sys.executable,
        },
    }
    (output / "m2-b-verification-results.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (output / "M2_B_VERIFICATION.md").write_text(render_markdown(report), encoding="utf-8")
    (output / "M2_B_BLOCKERS.md").write_text(render_blockers(report), encoding="utf-8")
    print(
        json.dumps(
            {
                "mode": report["mode"],
                "gate": gate_status,
                "underlying": underlying,
                "output_dir": str(output),
                "source_commit": report["source_commit"],
                "source_identity": source_identity_status,
                "before_clean": before.get("clean"),
                "after_clean": after.get("clean"),
                "before_git_status": before.get("git_status", ""),
                "after_git_status": after.get("git_status", ""),
                "toolchains": {
                    "status": toolchains.get("status"),
                    "python": toolchains.get("python", {}),
                    "rust": {
                        name: value.get("status")
                        for name, value in toolchains.get("rust", {}).items()
                    },
                },
            },
            sort_keys=True,
        )
    )
    if args.development:
        return 0 if underlying == "PASS" and toolchains.get("status") == "PASS" else 2
    return 0 if gate_status == "PASS" else 2


if __name__ == "__main__":
    raise SystemExit(main())
