#!/usr/bin/env python3
"""Execute the six M2.G executable gates on an exact clean source head.

Owned gates:

```text
M2_PAIRED_STATE_VISIBLE_BYTES_NONINTERFERENCE
M2_MULTI_ENDPOINT_INFORMATION_ISOLATION
M2_REJECTED_RESPONSE_COMPLETE_NONMUTATION
M2_CHECKPOINT_RESTORE_INFORMATION_IDENTITY
M2_FORK_INFORMATION_PARITY
M2_REPLAY_INFORMATION_PARITY
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
OUTPUT = ROOT / "dist" / "m2-g-verification"
OUTPUT_MARKER = ".mtgml-m2-g-gates-output"
PINNED_TOOLCHAIN: dict[str, str | None] = {"channel": None}

GATE_PAIRED = "M2_PAIRED_STATE_VISIBLE_BYTES_NONINTERFERENCE"
GATE_ENDPOINT = "M2_MULTI_ENDPOINT_INFORMATION_ISOLATION"
GATE_REJECTED = "M2_REJECTED_RESPONSE_COMPLETE_NONMUTATION"
GATE_CHECKPOINT = "M2_CHECKPOINT_RESTORE_INFORMATION_IDENTITY"
GATE_FORK = "M2_FORK_INFORMATION_PARITY"
GATE_REPLAY = "M2_REPLAY_INFORMATION_PARITY"

AXIS_PREFIX = "isolation::paired_matrix::tests::axis_"
MUTANT_PREFIX = "isolation::mutants::tests::detects_"


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


EXPECTED_EVIDENCE: dict[str, tuple[str, ...]] = {
    GATE_PAIRED: (
        f"{AXIS_PREFIX}01_opponent_hidden_definition_byte_equality",
        f"{AXIS_PREFIX}02_hidden_concealed_ordering_byte_equality",
        f"{AXIS_PREFIX}03_foreign_private_look_byte_equality",
        f"{AXIS_PREFIX}04_face_down_identity_byte_equality",
        f"{AXIS_PREFIX}05_root_seed_pre_auth_byte_equality",
        f"{AXIS_PREFIX}06_hidden_rng_cursor_byte_equality",
        f"{AXIS_PREFIX}07a_object_renaming_byte_equality",
        f"{AXIS_PREFIX}07b_ability_renaming_byte_equality",
        f"{AXIS_PREFIX}08_global_allocator_history_byte_equality",
        f"{AXIS_PREFIX}09_foreign_knowledge_history_byte_equality",
        f"{AXIS_PREFIX}03_foreign_private_look_transition_parity",
        f"{AXIS_PREFIX}04_face_down_identity_transition_parity",
        f"{AXIS_PREFIX}05_root_seed_pre_auth_transition_parity",
        f"{AXIS_PREFIX}06_hidden_rng_cursor_transition_parity",
        f"{AXIS_PREFIX}07b_ability_renaming_transition_parity",
        f"{AXIS_PREFIX}09_foreign_knowledge_history_transition_parity",
        "isolation::paired_matrix::tests::paired_rejection_parity_hidden_axes",
        f"{MUTANT_PREFIX}m1_resort_retained_knowledge",
        f"{MUTANT_PREFIX}m2_candidate_ids",
        f"{MUTANT_PREFIX}m3_allocator_ids",
        f"{MUTANT_PREFIX}m4_submission_code",
        f"{MUTANT_PREFIX}m5_event_sequence_stamp",
        f"{MUTANT_PREFIX}m6_payload_definition_channel",
        f"{MUTANT_PREFIX}m7_position_hint_channel",
        f"{MUTANT_PREFIX}m8_foreign_record_insertion",
        f"{MUTANT_PREFIX}m9_payload_secret_hex_channel",
        f"{MUTANT_PREFIX}m10_summary_count_inflation",
        f"{MUTANT_PREFIX}m11_optional_presence_toggle",
        f"{MUTANT_PREFIX}m12_payload_length_variation",
        "isolation::mutants::tests::dead_mutant_guard_m8_clean_inputs_do_not_diverge",
        "isolation::witnesses::tests::identity_equivalence_is_scoped_to_witness_perspective",
        "isolation::witnesses::tests::decision_relation_hides_foreign_actor_requests",
        "isolation::witnesses::tests::every_axis_predicate_fails_vacuous_identical_pairs",
        "source_check::no_production_isolation_hooks",
    ),
    GATE_ENDPOINT: (
        "isolation::endpoint_pair::tests::case_coexist_binding_permanent",
        "isolation::endpoint_pair::tests::case_public_private_projection_agreement",
        "isolation::endpoint_pair::tests::case_mixed_audience_projection_split",
        "isolation::endpoint_pair::tests::case_wrong_perspective_closed_surface",
        "isolation::endpoint_pair::tests::purity_read_order_matrix",
        "isolation::endpoint_pair::tests::restore_with_live_handles_and_rebinding",
        "isolation::endpoint_pair::tests::fork_controller_isolation",
        "isolation::endpoint_pair::tests::accepted_determinism_twins",
    ),
    GATE_REJECTED: (
        "isolation::rejection::tests::semantic_matrix_fingerprint_stable",
        "isolation::rejection::tests::above_maximum_answers_classify_before_the_cardinality_arm",
        "isolation::rejection::tests::coverage_tags_match_executed_matrix",
        "isolation::wire_boundary::tests::malformed_classes_zero_submit_and_zero_mutation",
        "isolation::wire_boundary::tests::canonical_stale_bytes_reach_semantic_submit_exactly_once",
        "isolation::wire_boundary::tests::coverage_tags_match_executed_classes",
    ),
    GATE_CHECKPOINT: (
        "isolation::checkpoint_parity::tests::restore_decision_rich",
        "isolation::checkpoint_parity::tests::restore_information_rich",
        "isolation::checkpoint_parity::tests::corrupt_checkpoint_restores_fail_closed",
    ),
    GATE_FORK: (
        "isolation::fork_parity::tests::fork_decision_rich",
        "isolation::fork_parity::tests::fork_information_rich",
        "isolation::fork_parity::tests::divergence_only_from_inputs",
        "isolation::fork_parity::tests::cross_mutation_isolation_matrix",
    ),
    GATE_REPLAY: (
        "synthetic::replay_parity_tests::historical_reprojection_byte_exact",
        "synthetic::replay_parity_tests::diagnostic_rejected_step_executes_with_intact_identity_chain",
        "synthetic::replay_parity_tests::recorded_inactive_counter_progression_fails_closed_without_live_mutation",
        "isolation::replay_parity::tests::final_identity_and_snapshot_parity",
        "tests::diagnostic_step_preserves_complete_identity",
        "tests::inactive_counter_forward_progression_passes_structural_monotonicity",
        "source_check::no_host_clock_sampling",
    ),
}

REQUIRED_COVERAGE: dict[str, frozenset[str]] = {
    "axis_tags": frozenset({"01", "02", "03", "04", "05", "06", "07a", "07b", "08", "09"}),
    # Every hidden axis slug must own accepted-submission transition
    # evidence. Axes 01/02/07a/08 execute that comparison inside their
    # `*_byte_equality` nodes; every other axis owns a dedicated
    # `*_transition_parity` node.
    "paired_transition_nodes": frozenset(
        {
            f"{AXIS_PREFIX}01_opponent_hidden_definition_byte_equality",
            f"{AXIS_PREFIX}02_hidden_concealed_ordering_byte_equality",
            f"{AXIS_PREFIX}03_foreign_private_look_transition_parity",
            f"{AXIS_PREFIX}04_face_down_identity_transition_parity",
            f"{AXIS_PREFIX}05_root_seed_pre_auth_transition_parity",
            f"{AXIS_PREFIX}06_hidden_rng_cursor_transition_parity",
            f"{AXIS_PREFIX}07a_object_renaming_byte_equality",
            f"{AXIS_PREFIX}07b_ability_renaming_transition_parity",
            f"{AXIS_PREFIX}08_global_allocator_history_byte_equality",
            f"{AXIS_PREFIX}09_foreign_knowledge_history_transition_parity",
        }
    ),
    # Shared paired rejection matrix: kills secret-dependent error
    # classification (InvalidCandidate-vs-InvalidAnswer leak shape) for
    # every hidden axis.
    "paired_rejection_nodes": frozenset(
        {"isolation::paired_matrix::tests::paired_rejection_parity_hidden_axes"}
    ),
    "mutant_tags": frozenset({f"m{index}" for index in range(1, 13)}),
    "rejection_semantic_rows": frozenset(
        {
            "isolation::rejection::tests::semantic_matrix_fingerprint_stable",
            "isolation::rejection::tests::above_maximum_answers_classify_before_the_cardinality_arm",
        }
    ),
    "wire_rejection_classes": frozenset(
        {
            "isolation::wire_boundary::tests::malformed_classes_zero_submit_and_zero_mutation",
        }
    ),
    "checkpoint_restore_classes": frozenset(
        {
            "isolation::checkpoint_parity::tests::restore_decision_rich",
            "isolation::checkpoint_parity::tests::restore_information_rich",
        }
    ),
    # Mixed-audience endpoint evidence: the isolation gate must prove one
    # shared occurrence whose authorized projections differ per perspective.
    "endpoint_mixed_projection_nodes": frozenset(
        {
            "isolation::endpoint_pair::tests::case_public_private_projection_agreement",
            "isolation::endpoint_pair::tests::case_mixed_audience_projection_split",
        }
    ),
    "fork_parity_classes": frozenset(
        {
            "isolation::fork_parity::tests::fork_decision_rich",
            "isolation::fork_parity::tests::fork_information_rich",
        }
    ),
    "replay_nodes": frozenset(
        {
            "synthetic::replay_parity_tests::historical_reprojection_byte_exact",
            "synthetic::replay_parity_tests::diagnostic_rejected_step_executes_with_intact_identity_chain",
            "synthetic::replay_parity_tests::recorded_inactive_counter_progression_fails_closed_without_live_mutation",
            "isolation::replay_parity::tests::final_identity_and_snapshot_parity",
            "tests::diagnostic_step_preserves_complete_identity",
            "tests::inactive_counter_forward_progression_passes_structural_monotonicity",
        }
    ),
}


def _validate_registry_manifests() -> None:
    """Fail closed at startup unless every executed evidence node matches the
    expected exact-set manifest: missing, duplicate, and extra registrations
    each abort before any evidence executes."""
    declared_gates = set(GATE_TESTS)
    expected_gates = set(EXPECTED_EVIDENCE)
    if declared_gates != expected_gates:
        raise ValueError(
            "gate manifest drift: "
            f"unmanifested={sorted(declared_gates - expected_gates)} "
            f"undeclared={sorted(expected_gates - declared_gates)}"
        )
    seen_across_gates: set[str] = set()
    for gate_name, definitions in GATE_TESTS.items():
        names = tuple(definition.name for definition in definitions)
        if len(names) != len(set(names)):
            duplicates = sorted({name for name in names if names.count(name) > 1})
            raise ValueError(f"{gate_name}: duplicate evidence node registration {duplicates}")
        expected = EXPECTED_EVIDENCE[gate_name]
        if len(expected) != len(set(expected)):
            raise ValueError(f"{gate_name}: expected manifest itself contains duplicates")
        if list(names) != list(expected):
            held = set(names)
            wanted = set(expected)
            raise ValueError(
                f"{gate_name}: evidence manifest drift "
                f"missing={sorted(wanted - held)} extra={sorted(held - wanted)} "
                f"(order must match EXPECTED_EVIDENCE)"
            )
        shared = seen_across_gates.intersection(names)
        if shared:
            raise ValueError(f"duplicate evidence node registered across gates: {sorted(shared)}")
        seen_across_gates.update(names)


def _validate_required_coverage() -> None:
    """Assert coverage tags against registered node names.  Semantic row and
    wire-class identities that are not readable from Python are pinned by
    explicit expected-name sets instead."""
    registered = {
        definition.name for definitions in GATE_TESTS.values() for definition in definitions
    }
    missing_axes = sorted(
        tag
        for tag in REQUIRED_COVERAGE["axis_tags"]
        if not any(name.startswith(f"{AXIS_PREFIX}{tag}_") for name in registered)
    )
    missing_mutants = sorted(
        tag
        for tag in REQUIRED_COVERAGE["mutant_tags"]
        if not any(name.startswith(f"{MUTANT_PREFIX}{tag}_") for name in registered)
    )
    pinned = (
        REQUIRED_COVERAGE["rejection_semantic_rows"]
        | REQUIRED_COVERAGE["wire_rejection_classes"]
        | REQUIRED_COVERAGE["checkpoint_restore_classes"]
        | REQUIRED_COVERAGE["fork_parity_classes"]
        | REQUIRED_COVERAGE["endpoint_mixed_projection_nodes"]
        | REQUIRED_COVERAGE["replay_nodes"]
        | REQUIRED_COVERAGE["paired_transition_nodes"]
        | REQUIRED_COVERAGE["paired_rejection_nodes"]
    )
    unpinned = sorted(pinned - registered)
    if missing_axes or missing_mutants or unpinned:
        raise ValueError(
            "required coverage unmet: "
            f"axes={missing_axes} mutants={missing_mutants} pinned={unpinned}"
        )


GATE_TESTS: dict[str, tuple[EvidenceDefinition, ...]] = {
    GATE_PAIRED: (
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}01_opponent_hidden_definition_byte_equality",
            "axis 01 opponent hidden definition byte equality",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}02_hidden_concealed_ordering_byte_equality",
            "axis 02 hidden concealed ordering byte equality",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}03_foreign_private_look_byte_equality",
            "axis 03 foreign private look byte equality",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}04_face_down_identity_byte_equality",
            "axis 04 face-down identity byte equality",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}05_root_seed_pre_auth_byte_equality",
            "axis 05 root seed pre-authentication byte equality",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}06_hidden_rng_cursor_byte_equality",
            "axis 06 hidden RNG cursor byte equality",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}07a_object_renaming_byte_equality",
            "axis 07a object renaming byte equality",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}07b_ability_renaming_byte_equality",
            "axis 07b ability renaming byte equality",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}08_global_allocator_history_byte_equality",
            "axis 08 global allocator history byte equality",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}09_foreign_knowledge_history_byte_equality",
            "axis 09 foreign knowledge history byte equality",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}03_foreign_private_look_transition_parity",
            "axis 03 foreign private look accepted-transition parity",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}04_face_down_identity_transition_parity",
            "axis 04 face-down identity accepted-transition parity",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}05_root_seed_pre_auth_transition_parity",
            "axis 05 root seed pre-authentication accepted-transition parity",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}06_hidden_rng_cursor_transition_parity",
            "axis 06 hidden RNG cursor accepted-transition parity",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}07b_ability_renaming_transition_parity",
            "axis 07b ability renaming accepted-transition parity",
        ),
        rust(
            "mtgml-conformance",
            f"{AXIS_PREFIX}09_foreign_knowledge_history_transition_parity",
            "axis 09 foreign knowledge history accepted-transition parity",
        ),
        rust(
            "mtgml-conformance",
            "isolation::paired_matrix::tests::paired_rejection_parity_hidden_axes",
            "paired rejection parity matrix across every hidden axis",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m1_resort_retained_knowledge",
            "mutant m1 retained-knowledge reordering detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m2_candidate_ids",
            "mutant m2 candidate id channel detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m3_allocator_ids",
            "mutant m3 allocator id channel detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m4_submission_code",
            "mutant m4 submission code channel detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m5_event_sequence_stamp",
            "mutant m5 event sequence stamp detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m6_payload_definition_channel",
            "mutant m6 payload definition channel detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m7_position_hint_channel",
            "mutant m7 position hint channel detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m8_foreign_record_insertion",
            "mutant m8 foreign record insertion detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m9_payload_secret_hex_channel",
            "mutant m9 payload secret hex channel detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m10_summary_count_inflation",
            "mutant m10 summary count inflation detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m11_optional_presence_toggle",
            "mutant m11 optional presence toggle detected",
        ),
        rust(
            "mtgml-conformance",
            f"{MUTANT_PREFIX}m12_payload_length_variation",
            "mutant m12 payload length variation detected",
        ),
        rust(
            "mtgml-conformance",
            "isolation::mutants::tests::dead_mutant_guard_m8_clean_inputs_do_not_diverge",
            "dead mutant guard: clean inputs do not diverge",
        ),
        rust(
            "mtgml-conformance",
            "isolation::witnesses::tests::identity_equivalence_is_scoped_to_witness_perspective",
            "identity equivalence scoped to witness perspective",
        ),
        rust(
            "mtgml-conformance",
            "isolation::witnesses::tests::decision_relation_hides_foreign_actor_requests",
            "decision relation hides foreign actor requests",
        ),
        rust(
            "mtgml-conformance",
            "isolation::witnesses::tests::every_axis_predicate_fails_vacuous_identical_pairs",
            "every axis predicate fails vacuous identical pairs",
        ),
        source(
            "source_check::no_production_isolation_hooks",
            "SUPPLEMENTAL: production crates carry no isolation test hooks",
        ),
    ),
    GATE_ENDPOINT: (
        rust(
            "mtgml-conformance",
            "isolation::endpoint_pair::tests::case_coexist_binding_permanent",
            "coexisting endpoints keep permanent bindings",
        ),
        rust(
            "mtgml-conformance",
            "isolation::endpoint_pair::tests::case_public_private_projection_agreement",
            "public and private projections agree",
        ),
        rust(
            "mtgml-conformance",
            "isolation::endpoint_pair::tests::case_mixed_audience_projection_split",
            "mixed-audience projection split across paired endpoints",
        ),
        rust(
            "mtgml-conformance",
            "isolation::endpoint_pair::tests::case_wrong_perspective_closed_surface",
            "wrong perspective sees a closed surface",
        ),
        rust(
            "mtgml-conformance",
            "isolation::endpoint_pair::tests::purity_read_order_matrix",
            "reads are pure across every read order",
        ),
        rust(
            "mtgml-conformance",
            "isolation::endpoint_pair::tests::restore_with_live_handles_and_rebinding",
            "restore survives live handles and rebinding",
        ),
        rust(
            "mtgml-conformance",
            "isolation::endpoint_pair::tests::fork_controller_isolation",
            "fork controllers stay isolated",
        ),
        rust(
            "mtgml-conformance",
            "isolation::endpoint_pair::tests::accepted_determinism_twins",
            "accepted submissions produce determinism twins",
        ),
    ),
    GATE_REJECTED: (
        rust(
            "mtgml-conformance",
            "isolation::rejection::tests::semantic_matrix_fingerprint_stable",
            "semantic rejection matrix fingerprint stable",
        ),
        rust(
            "mtgml-conformance",
            "isolation::rejection::tests::above_maximum_answers_classify_before_the_cardinality_arm",
            "above-maximum answers classify before the cardinality arm",
        ),
        rust(
            "mtgml-conformance",
            "isolation::rejection::tests::coverage_tags_match_executed_matrix",
            "coverage tags match the executed rejection matrix",
        ),
        rust(
            "mtgml-conformance",
            "isolation::wire_boundary::tests::malformed_classes_zero_submit_and_zero_mutation",
            "malformed wire classes submit zero and mutate zero",
        ),
        rust(
            "mtgml-conformance",
            "isolation::wire_boundary::tests::canonical_stale_bytes_reach_semantic_submit_exactly_once",
            "canonical stale bytes reach semantic submit exactly once",
        ),
        rust(
            "mtgml-conformance",
            "isolation::wire_boundary::tests::coverage_tags_match_executed_classes",
            "coverage tags match the executed wire classes",
        ),
    ),
    GATE_CHECKPOINT: (
        rust(
            "mtgml-conformance",
            "isolation::checkpoint_parity::tests::restore_decision_rich",
            "restore parity in the decision-rich scenario",
        ),
        rust(
            "mtgml-conformance",
            "isolation::checkpoint_parity::tests::restore_information_rich",
            "restore parity in the information-rich scenario",
        ),
        rust(
            "mtgml-conformance",
            "isolation::checkpoint_parity::tests::corrupt_checkpoint_restores_fail_closed",
            "corrupted checkpoint restores fail closed",
        ),
    ),
    GATE_FORK: (
        rust(
            "mtgml-conformance",
            "isolation::fork_parity::tests::fork_decision_rich",
            "fork parity in the decision-rich scenario",
        ),
        rust(
            "mtgml-conformance",
            "isolation::fork_parity::tests::fork_information_rich",
            "fork parity in the information-rich scenario",
        ),
        rust(
            "mtgml-conformance",
            "isolation::fork_parity::tests::divergence_only_from_inputs",
            "fork divergence only from inputs",
        ),
        rust(
            "mtgml-conformance",
            "isolation::fork_parity::tests::cross_mutation_isolation_matrix",
            "cross-mutation isolation matrix",
        ),
    ),
    GATE_REPLAY: (
        rust(
            "mtgml-environment",
            "synthetic::replay_parity_tests::historical_reprojection_byte_exact",
            "historical reprojection byte-exact against the recording",
        ),
        rust(
            "mtgml-environment",
            "synthetic::replay_parity_tests::diagnostic_rejected_step_executes_with_intact_identity_chain",
            "diagnostic rejected step keeps the identity chain intact",
        ),
        rust(
            "mtgml-environment",
            "synthetic::replay_parity_tests::recorded_inactive_counter_progression_fails_closed_without_live_mutation",
            "recorded inactive counter progression fails closed without live mutation",
        ),
        rust(
            "mtgml-conformance",
            "isolation::replay_parity::tests::final_identity_and_snapshot_parity",
            "Node B final identity and snapshot parity",
        ),
        rust(
            "mtgml-replay",
            "tests::diagnostic_step_preserves_complete_identity",
            "replay diagnostic step preserves complete identity",
        ),
        rust(
            "mtgml-replay",
            "tests::inactive_counter_forward_progression_passes_structural_monotonicity",
            "replay inactive counter progression passes structural monotonicity",
        ),
        source(
            "source_check::no_host_clock_sampling",
            "SUPPLEMENTAL: production crates never sample the host clock; "
            "counter exactness is separate behavioral evidence",
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
        raise RuntimeError("M2.G verification output must remain below repository dist")
    if "verification" in relative.parts:
        raise RuntimeError("dist/verification is exclusively owned by release-candidate")
    if output.exists():
        marker = output / OUTPUT_MARKER
        if not marker.is_file():
            raise RuntimeError(f"refusing to replace unowned verification output: {output}")
        shutil.rmtree(output)
    logs = output / "logs"
    logs.mkdir(parents=True, exist_ok=True)
    (output / OUTPUT_MARKER).write_text("owned by scripts/run_m2_g_gates.py\n", encoding="utf-8")
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


PRODUCTION_SOURCE_EXCLUDED_CRATES = frozenset({"mtgml-conformance"})
ISOLATION_HOOK_PATTERNS = (r"\bisolation::", r"\bLeakMutant\b", r"\bdetects_m")
CLOCK_SAMPLING_PATTERNS = (r"\bInstant::now\b", r"\bSystemTime\b", r"\bstd::time\b")


def production_source_files() -> list[Path]:
    files: list[Path] = []
    crates_root = ROOT / "crates"
    for crate in sorted(crates_root.iterdir()):
        if crate.name in PRODUCTION_SOURCE_EXCLUDED_CRATES:
            continue
        src = crate / "src"
        if src.is_dir():
            files.extend(sorted(src.rglob("*.rs")))
    return files


def scan_production_sources(patterns: Sequence[str], label: str) -> tuple[int, list[str]]:
    violations: list[str] = []
    scanned = 0
    for path in production_source_files():
        scanned += 1
        text = path.read_text(encoding="utf-8")
        for pattern in patterns:
            for match in re.finditer(pattern, text):
                line = text.count("\n", 0, match.start()) + 1
                violations.append(
                    f"{path.relative_to(ROOT).as_posix()}:{line}: {label} pattern {pattern!r}"
                )
    return scanned, violations


def check_no_production_isolation_hooks() -> str:
    """Conformance-owned isolation vocabulary must never leak into shipped
    crates: no ``isolation::`` paths, leak-mutant types, or mutant-detect
    entry points may appear outside mtgml-conformance."""
    scanned, violations = scan_production_sources(ISOLATION_HOOK_PATTERNS, "isolation hook")
    if violations:
        raise AssertionError(
            "production sources reference isolation test machinery:\n" + "\n".join(violations)
        )
    return f"no isolation hooks across {scanned} production source files"


def check_no_host_clock_sampling() -> str:
    """Deterministic simulation forbids wall-clock sampling in shipped
    crates.  Supplemental only: counter exactness under replay is proven by
    dedicated behavioral evidence nodes in this runner."""
    scanned, violations = scan_production_sources(CLOCK_SAMPLING_PATTERNS, "host clock sampling")
    if violations:
        raise AssertionError("production sources sample the host clock:\n" + "\n".join(violations))
    return f"no host clock sampling across {scanned} production source files"


SOURCE_CHECKS = {
    "source_check::no_production_isolation_hooks": check_no_production_isolation_hooks,
    "source_check::no_host_clock_sampling": check_no_host_clock_sampling,
}


_validate_registry_manifests()
_validate_required_coverage()


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
        "# M2.G Gate Verification",
        "",
        "Generated outside the reproducible source archive by `scripts/run_m2_g_gates.py`.",
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
        "milestone": "M2.G",
        "reporter": "scripts/run_m2_g_gates.py",
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
    (output / "m2-g-gate-results.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (output / "M2_G_GATES.md").write_text(render_markdown(report), encoding="utf-8")
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
