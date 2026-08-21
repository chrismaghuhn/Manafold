#!/usr/bin/env python3
"""Run the ten M1 acceptance gates without mutating the source tree."""

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
OUTPUT = ROOT / "dist" / "verification" / "m1"
OUTPUT_MARKER = ".mtgml-m1-closure-output"


@dataclass(frozen=True)
class TestDefinition:
    package: str
    test_name: str
    surface: str


def test_definitions(
    package: str, names: Sequence[str], surface: str
) -> tuple[TestDefinition, ...]:
    return tuple(TestDefinition(package, name, surface) for name in names)


GATE_TESTS: dict[str, tuple[TestDefinition, ...]] = {
    "ENGINE_STATE_CONSTRUCTION_AND_INVARIANTS": (
        *test_definitions(
            "mtgml-state",
            (
                "tests::synthetic_constructor_builds_a_nontrivial_valid_state",
                "tests::synthetic_reset_is_exactly_deterministic_for_identical_inputs",
                "tests::valid_empty_shell_passes_cross_component_validation",
                "tests::pending_decision_must_match_state_revision",
                "tests::unordered_object_must_not_appear_in_ordered_zones",
                "tests::ordered_object_must_appear_exactly_once",
                "tests::ordered_object_must_not_be_missing",
                "tests::duplicate_live_physical_card_incarnation_rejected",
                "tests::pending_candidate_binding_must_match_authoritative_binding",
                "tests::opaque_allocator_must_reference_declared_player",
                "tests::commander_designation_must_reference_owned_live_physical_card",
                "tests::valid_commander_structural_references_are_accepted",
                "tests::commander_ledger_must_reference_a_designated_physical_card",
                "tests::commander_damage_ledger_must_reference_a_declared_player",
            ),
            "complete synthetic EngineState construction and cross-component invariants",
        ),
    ),
    "ACCEPTED_TRANSITION_EXACT_PRODUCT": (
        *test_definitions(
            "mtgml-rules",
            ("tests::synthetic_m1_acceptance_returns_exact_transition_product",),
            "RulesKernel state, ordered events, StateDelta, next decision, and status",
        ),
        *test_definitions(
            "mtgml-environment",
            ("tests::accepted_environment_transaction_commits_the_exact_m1_product",),
            "trusted environment transaction, counters, checkpoint, and replay product",
        ),
    ),
    "REJECTED_RESPONSE_COMPLETE_NONMUTATION": (
        *test_definitions(
            "mtgml-rules",
            ("tests::synthetic_rejection_matrix_preserves_complete_nonmutation",),
            "RulesKernel rejection matrix and complete owned-state nonmutation",
        ),
        *test_definitions(
            "mtgml-environment",
            (
                "tests::rejected_environment_submission_preserves_complete_outer_nonmutation",
                "tests::semantic_replay_executes_rejected_diagnostic_without_live_recording",
            ),
            "environment status, counters, checkpoint, and replay nonmutation",
        ),
        *test_definitions(
            "mtgml-replay",
            (
                "tests::replay_recorder_rejects_invalid_append_without_mutation",
                "tests::replay_recorder_keeps_rejected_diagnostic_identity",
                "tests::rejected_replay_step_must_preserve_the_full_state_digest",
            ),
            "rejected replay identity and recorder nonmutation",
        ),
    ),
    "STATE_DELTA_FULL_REAPPLICATION": (
        *test_definitions(
            "mtgml-state",
            ("tests::exact_delta_reapplies_every_component",),
            "full StateDelta application across every authoritative component",
        ),
        *test_definitions(
            "mtgml-rules",
            ("tests::synthetic_m1_acceptance_returns_exact_transition_product",),
            "accepted transition delta reapplication and digest parity",
        ),
    ),
    "SEQUENTIAL_EVENT_DELTA_PARITY": (
        *test_definitions(
            "mtgml-rules",
            (
                "tests::two_life_changes_in_one_atomic_transition_are_compositional",
                "tests::decision_clear_then_create_composes_to_the_next_decision",
                "tests::repeated_tap_changes_in_one_atomic_transition_are_compositional",
                "tests::consecutive_zone_incarnations_in_one_transition_are_compositional",
                "tests::second_life_event_must_use_cursor_life_after_first_event",
                "tests::reversed_dependent_life_events_fail",
                "tests::incomplete_life_trace_fails_final_projection",
                "tests::event_and_delta_audit_disagreement_fails",
                "tests::random_sample_event_is_required_for_final_cursor_progression",
                "tests::random_sample_event_and_delta_audit_must_agree",
            ),
            "ordered semantic cursor validation and final StateDelta projection parity",
        ),
    ),
    "CHECKPOINT_RESTORE_COMPLETE_IDENTITY": (
        *test_definitions(
            "mtgml-environment",
            (
                "tests::checkpoint_closes_state_status_and_limit_counters",
                "tests::checkpoint_digest_covers_status_and_limit_counters",
                "tests::checkpoint_captures_m1_5_continuation_state",
                "tests::synthetic_backend_checkpoint_captures_the_complete_initial_product",
                "tests::synthetic_backend_restore_rejects_state_tampering_without_mutation",
                "tests::synthetic_backend_restore_rejects_counter_tampering_without_mutation",
                "tests::synthetic_backend_restore_rejects_unsupported_codec_without_mutation",
                "tests::checkpoint_restore_repeats_exact_transition_and_replay_segment",
                "tests::accepted_state_restore_preserves_identity_and_rebases_empty_replay",
            ),
            "complete EnvironmentCheckpointV2 creation, validation, and restore identity",
        ),
    ),
    "FORK_PARITY": (
        *test_definitions(
            "mtgml-environment",
            (
                "tests::forks_from_a_checkpoint_begin_with_exact_identity_and_empty_segments",
                "tests::forks_with_the_same_input_have_exact_continuation_parity",
                "tests::forks_diverge_only_on_explicit_accepted_or_rejected_input",
            ),
            "checkpoint fork identity, same-input parity, and explicit-input divergence",
        ),
    ),
    "REPLAY_PARITY": (
        *test_definitions(
            "mtgml-environment",
            (
                "tests::semantic_replay_reproduces_the_live_accepted_transition_exactly",
                "tests::semantic_replay_executes_rejected_diagnostic_without_live_recording",
                "tests::semantic_replay_rejects_counter_divergence_at_first_step",
                "tests::semantic_replay_rejects_wrong_initial_digest_before_execution",
                "tests::semantic_replay_rejects_wrong_root_seed_before_execution",
                "tests::semantic_replay_rejects_tampered_accepted_after_digest_at_first_step",
                "tests::semantic_replay_rejects_tampered_accepted_flag_at_first_step",
                "tests::semantic_replay_rejects_stale_response_at_first_step",
            ),
            "semantic replay execution, exact live parity, and fail-closed divergence",
        ),
        *test_definitions(
            "mtgml-replay",
            (
                "tests::replay_recorder_starts_empty_segment_at_manifest_identity",
                "tests::replay_recorder_appends_exact_accepted_step",
                "tests::replay_recorder_rejects_invalid_append_without_mutation",
                "tests::replay_recorder_keeps_rejected_diagnostic_identity",
            ),
            "V2 replay recorder identity, contiguous steps, and rejection identity",
        ),
    ),
    "DETERMINISTIC_RNG_AND_ALLOCATORS": (
        *test_definitions(
            "mtgml-random",
            (
                "hmac_counter::tests::raw_words_0_to_7_kat",
                "hmac_counter::tests::stream_isolation",
                "sampling::tests::bound_ten_normative_kat",
                "sampling::tests::shuffle_normative_kat",
            ),
            "project-owned deterministic RNG derivation, sampling, and stream isolation",
        ),
        *test_definitions(
            "mtgml-state",
            (
                "tests::effect_allocator_returns_current_id_and_checked_advances",
                "tests::effect_allocator_exhaustion_does_not_mutate_allocator",
            ),
            "checked deterministic identity allocation and exhaustion atomicity",
        ),
        *test_definitions(
            "mtgml-rules",
            (
                "tests::deterministic_services_repeat_exact_transition_result",
                "tests::deterministic_services_isolate_named_stream_and_allocator_cursors",
                "tests::rng_exhaustion_is_a_typed_internal_failure_without_input_mutation",
                "tests::effect_allocator_exhaustion_is_a_typed_internal_failure_before_rng",
                "tests::random_sample_event_is_validated_against_authoritative_sampler",
            ),
            "accepted-transition RNG/allocator continuation and fail-closed service errors",
        ),
    ),
    "MULTI_PLAYER_ENDPOINT_BINDING": (
        *test_definitions(
            "mtgml-environment",
            (
                "tests::two_player_endpoints_can_remain_alive_simultaneously",
                "tests::synthetic_backend_player_surface_projects_two_bound_players",
                "tests::wrong_perspective_submission_is_nonmutating_and_shared_p1_submission_advances_both",
                "tests::wrong_perspective_submission_is_nonmutating_when_p2_owns_decision",
                "tests::non_default_player_ids_remain_bound_through_visibility_rejection_and_step",
                "tests::endpoint_submission_matches_trusted_authoritative_checkpoint_and_replay",
                "tests::unknown_player_binding_remains_rejected_without_exposing_backend_details",
                "tests::player_api_errors_do_not_render_trusted_or_hidden_values",
                "tests::successful_player_values_and_errors_do_not_render_trusted_provenance",
            ),
            (
                "two perspective-bound endpoints, actor ownership, nonmutation, "
                "and safe player surface"
            ),
        ),
    ),
}

GATE_NAMES = tuple(GATE_TESTS)


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


def command_text(command: Sequence[str]) -> str:
    return " ".join(command)


def exact_test_passed(output: str, returncode: int) -> bool:
    return bool(
        returncode == 0
        and re.search(r"running\s+1\s+test\b", output)
        and re.search(r"test result:\s+ok\.\s+1 passed;\s+0 failed\b", output)
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
        raise RuntimeError(completed.stdout.strip() or command_text(("git", *arguments)))
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


def aggregate_status(statuses: Iterable[str]) -> str:
    status_set = set(statuses)
    if status_set == {"PASS"}:
        return "PASS"
    if "BLOCKED" in status_set:
        return "BLOCKED"
    if "NOT_RUN" in status_set:
        return "NOT_RUN"
    return "FAIL"


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
    python_result: subprocess.CompletedProcess[str] | None = None
    python_reason: str | None = None
    try:
        python_result = run_command((sys.executable, "--version"))
    except OSError as error:
        python_status = "BLOCKED"
        python_reason = str(error)
    else:
        python_status = (
            "PASS"
            if python_result is not None
            and python_result.returncode == 0
            and python_version == expected_python
            else "FAIL"
        )
    rust_commands = {
        "rustc": ("rustc", "--version"),
        "cargo": ("cargo", "--version"),
        "active_toolchain": ("rustup", "show", "active-toolchain"),
    }
    rust_results: dict[str, dict[str, Any]] = {}
    for name, command in rust_commands.items():
        if not command_available(command):
            rust_results[name] = {
                "status": "NOT_RUN",
                "command": list(command),
                "reason": f"{command[0]} not found",
            }
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

    rust_status = aggregate_status(result["status"] for result in rust_results.values())
    version_checks: dict[str, dict[str, Any]] = {}
    if rust_status == "PASS":
        for name, record in rust_results.items():
            reported = reported_toolchain_version(name, record["output"])
            version_checks[name] = {
                "expected": expected_rust,
                "reported": reported,
                "status": "PASS" if reported == expected_rust else "FAIL",
            }
        if any(check["status"] != "PASS" for check in version_checks.values()):
            rust_status = "FAIL"
    combined_status = aggregate_status((python_status, rust_status))
    python_report: dict[str, Any] = {
        "status": python_status,
        "executable": sys.executable,
        "version": python_version,
        "expected_version": expected_python,
        "version_command": [sys.executable, "--version"],
    }
    if python_result is not None:
        python_report["version_output"] = python_result.stdout.strip()
    if python_reason is not None:
        python_report["reason"] = python_reason
    return {
        "status": combined_status,
        "python": python_report,
        "rust": {
            "status": rust_status,
            "expected_channel": expected_rust,
            "commands": rust_results,
            "version_checks": version_checks,
        },
    }


def prepare_output(output: Path) -> Path:
    if output.exists():
        marker = output / OUTPUT_MARKER
        if not marker.is_file():
            raise RuntimeError(f"refusing to replace unowned verification output: {output}")
        shutil.rmtree(output)
    logs = output / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    (output / OUTPUT_MARKER).write_text("owned by scripts/run_m1_closure.py\n", encoding="utf-8")
    return logs


def execute_test(definition: TestDefinition, logs: Path, index: int) -> dict[str, Any]:
    command = (
        "cargo",
        "test",
        "--package",
        definition.package,
        "--all-features",
        "--locked",
        "--lib",
        "--",
        definition.test_name,
        "--exact",
    )
    log_name = f"{index:03d}-{definition.package}-{definition.test_name.replace('::', '-')}.log"
    log_path = logs / log_name
    evidence: dict[str, Any] = {
        "package": definition.package,
        "test": definition.test_name,
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
    observed = 1 if re.search(r"running\s+1\s+test\b", output) else 0
    passed = exact_test_passed(output, completed.returncode)
    evidence.update(
        {
            "status": "PASS" if passed else "FAIL",
            "returncode": completed.returncode,
            "tests_observed": observed,
            "reason": "exact test passed" if passed else "exact test did not pass",
        }
    )
    return evidence


def executable_pass_evidence(item: Any) -> bool:
    command = item.get("command") if isinstance(item, dict) else None
    return (
        isinstance(item, dict)
        and item.get("status") == "PASS"
        and isinstance(item.get("test"), str)
        and bool(item["test"])
        and isinstance(command, list)
        and bool(command)
        and all(isinstance(part, str) and bool(part) for part in command)
        and item.get("returncode") == 0
        and item.get("tests_observed") == 1
    )


def complete_gate_set(gates: Any) -> bool:
    if not isinstance(gates, list):
        return False
    if tuple(gate.get("name") for gate in gates if isinstance(gate, dict)) != GATE_NAMES:
        return False
    return all(
        isinstance(gate, dict)
        and gate.get("status") == "PASS"
        and isinstance(gate.get("evidence"), list)
        and bool(gate["evidence"])
        and all(executable_pass_evidence(item) for item in gate["evidence"])
        for gate in gates
    )


def build_report(
    source_identity: dict[str, Any],
    toolchains: dict[str, Any],
    gates: list[dict[str, Any]],
    *,
    generated_at: str | None = None,
) -> dict[str, Any]:
    complete = (
        source_identity.get("status") == "PASS"
        and toolchains.get("status") == "PASS"
        and complete_gate_set(gates)
    )
    return {
        "generated_at": generated_at or datetime.now(UTC).isoformat(),
        "milestone": "M1",
        "reporter": "scripts/run_m1_closure.py",
        "source_commit": source_identity.get("commit"),
        "source_tree_identity": source_identity,
        "toolchains": toolchains,
        "gates": gates,
        "overall": "COMPLETE" if complete else "INCOMPLETE",
        "milestone_status": "COMPLETE" if complete else "INCOMPLETE",
        "m2_status": "UNBLOCKED" if complete else "BLOCKED",
        "claims": {
            "playable_engine": False,
            "real_magic_rules": False,
            "real_card_support": False,
        },
    }


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# M1 Verification",
        "",
        "Generated outside the reproducible source archive by `scripts/run_m1_closure.py`.",
        "",
        f"- Source commit: `{report.get('source_commit')}`",
        f"- M1.F final closure: **{report['overall']}**",
        f"- M1 status: **{report['milestone_status']}**",
        f"- M2 status: **{report['m2_status']}**",
        "",
        "## Toolchains",
        "",
        f"- Toolchain status: **{report['toolchains']['status']}**",
        f"- Python: `{report['toolchains'].get('python', {}).get('version')}`",
        f"- Rust channel: `{report['toolchains'].get('rust', {}).get('expected_channel')}`",
        "",
        "## M1 gates",
        "",
        "| Gate | Status | Exact evidence |",
        "|---|---:|---|",
    ]
    for gate in report["gates"]:
        evidence_text = "; ".join(
            f"`{item['package']}::{item['test']}` ({item['status']})" for item in gate["evidence"]
        )
        lines.append(f"| `{gate['name']}` | **{gate['status']}** | {evidence_text} |")
    lines.extend(
        [
            "",
            "## Claims",
            "",
            (
                "The M1 shell remains non-playable and contains no real Magic "
                "rules or real card support."
            ),
            "",
        ]
    )
    return "\n".join(lines)


def render_blockers(report: dict[str, Any]) -> str:
    lines = [
        "# M1 Blockers",
        "",
        "Generated from the same result set as `M1_VERIFICATION.md`.",
        "",
        "| Area | Status | Reason |",
        "|---|---:|---|",
    ]
    if report["source_tree_identity"].get("status") != "PASS":
        lines.append(
            f"| source tree | **{report['source_tree_identity'].get('status')}** | "
            f"{report['source_tree_identity'].get('reason', 'source identity check failed')} |"
        )
    if report["toolchains"].get("status") != "PASS":
        lines.append(
            f"| toolchains | **{report['toolchains'].get('status')}** | "
            "pinned toolchain identity did not pass |"
        )
    for gate in report["gates"]:
        for evidence in gate["evidence"]:
            if evidence["status"] != "PASS":
                lines.append(
                    f"| `{gate['name']}` / `{evidence['test']}` | **{evidence['status']}** | "
                    f"{evidence.get('reason', 'command did not pass')} |"
                )
    has_blocker = (
        report["source_tree_identity"].get("status") != "PASS"
        or report["toolchains"].get("status") != "PASS"
        or any(
            evidence["status"] != "PASS"
            for gate in report["gates"]
            for evidence in gate["evidence"]
        )
    )
    if not has_blocker:
        lines.append("| none | **PASS** | all source, toolchain, and gate checks passed |")
    lines.extend(["", f"M1 complete: **{report['milestone_status']}**", ""])
    return "\n".join(lines)


def write_reports(output: Path, report: dict[str, Any]) -> None:
    (output / "m1-verification-results.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (output / "M1_VERIFICATION.md").write_text(render_markdown(report), encoding="utf-8")
    (output / "M1_BLOCKERS.md").write_text(render_blockers(report), encoding="utf-8")


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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=OUTPUT,
        help="external owned output directory (default: dist/verification/m1)",
    )
    args = parser.parse_args()
    output = args.output_dir.resolve()
    if output == ROOT or ROOT not in output.parents or "dist" not in output.relative_to(ROOT).parts:
        parser.error("M1 verification output must remain below the repository dist directory")

    source_before = capture_source_snapshot()
    toolchains = capture_toolchain()
    try:
        logs = prepare_output(output)
    except (OSError, RuntimeError) as error:
        print(f"BLOCKED: {error}")
        return 2

    gates: list[dict[str, Any]] = []
    index = 1
    for gate_name, definitions in GATE_TESTS.items():
        evidence = []
        for definition in definitions:
            evidence.append(execute_test(definition, logs, index))
            index += 1
        gate_status = (
            "PASS" if evidence and all(item["status"] == "PASS" for item in evidence) else "FAIL"
        )
        if any(item["status"] == "NOT_RUN" for item in evidence):
            gate_status = "NOT_RUN"
        elif any(item["status"] == "BLOCKED" for item in evidence):
            gate_status = "BLOCKED"
        gates.append(
            {
                "name": gate_name,
                "status": gate_status,
                "surface": definitions[0].surface,
                "evidence": evidence,
            }
        )

    source_after = capture_source_snapshot()
    source_identity = finalize_source_identity(source_before, source_after)
    report = build_report(source_identity, toolchains, gates)
    write_reports(output, report)
    print(
        json.dumps(
            {
                "milestone_status": report["milestone_status"],
                "m2_status": report["m2_status"],
                "output_dir": str(output),
                "source_commit": report["source_commit"],
                "gates": {gate["name"]: gate["status"] for gate in gates},
            },
            sort_keys=True,
        )
    )
    return 0 if report["overall"] == "COMPLETE" else 2


if __name__ == "__main__":
    raise SystemExit(main())
