from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import run_m2_final_closure as final
from run_m2_final_closure import (
    CHILD_RUNNERS,
    EXPECTED_GATES,
    GATE_M1_REGRESSION,
    M1_CHILD,
    M1_GATE_NAMES,
    aggregate,
    build_report,
    compute_m1_matrix_status,
    finalize_source_identity,
    merge_child_gate_statuses,
    prepare_output,
    read_child_report,
    validate_m1_report,
    validate_slice_report,
)

COMMIT = "a" * 40


def passing_gates() -> list[dict[str, object]]:
    return [{"name": name, "status": "PASS"} for name in EXPECTED_GATES]


def passing_m1_regression() -> dict[str, object]:
    return {
        "status": "PASS",
        "matrix_status": "PASS",
        "scope_status": "PASS",
        "scope_checks": [],
    }


def complete_report(**overrides: object) -> dict[str, object]:
    payload: dict[str, object] = {
        "mode": "authoritative",
        "source_identity": {"status": "PASS", "commit": COMMIT},
        "toolchains": {"status": "PASS"},
        "child_runs": [],
        "gates": passing_gates(),
        "m1_regression": passing_m1_regression(),
        "expected_commit": COMMIT,
    }
    payload.update(overrides)
    return build_report(**payload)  # type: ignore[arg-type]


def slice_report(
    child: object,
    *,
    status: str = "PASS",
    mode: str = "authoritative",
    commit: str = COMMIT,
    identity_status: str = "PASS",
) -> dict[str, object]:
    return {
        "mode": mode,
        "source_commit": commit,
        "source_tree_identity": {"status": identity_status},
        "gates": [{"name": name, "gate_status": status} for name in child.gates],  # type: ignore[attr-defined]
    }


def m1_report(*, status: str = "PASS", overall: str = "COMPLETE") -> dict[str, object]:
    return {
        "overall": overall,
        "milestone_status": overall,
        "mode": "authoritative",
        "source_commit": COMMIT,
        "source_tree_identity": {"status": "PASS"},
        "gates": [
            {
                "name": name,
                "status": status,
                "evidence": [
                    {
                        "status": status,
                        "test": f"tests::{name.lower()}",
                        "command": ["cargo", "test", "--exact", name],
                        "returncode": 0 if status == "PASS" else 1,
                        "tests_observed": 1 if status == "PASS" else 0,
                    }
                ],
            }
            for name in M1_GATE_NAMES
        ],
    }


class GateRegistryTests(unittest.TestCase):
    def test_expected_gate_list_is_exactly_the_normative_twenty(self) -> None:
        self.assertEqual(len(EXPECTED_GATES), 20)
        self.assertEqual(len(set(EXPECTED_GATES)), 20)
        self.assertEqual(EXPECTED_GATES[-1], GATE_M1_REGRESSION)

    def test_child_runners_partition_the_first_nineteen_gates_exactly_once(self) -> None:
        owned = [gate for child in CHILD_RUNNERS for gate in child.gates]
        self.assertEqual(len(owned), len(set(owned)))
        self.assertEqual(set(owned), set(EXPECTED_GATES) - {GATE_M1_REGRESSION})
        self.assertEqual(len(owned), 19)

    def test_m1_child_owns_exactly_the_ten_m1_gates(self) -> None:
        self.assertEqual(M1_CHILD.gates, M1_GATE_NAMES)
        self.assertFalse(M1_CHILD.supports_expect_commit)
        self.assertFalse(M1_CHILD.supports_development_flag)


class AggregationTests(unittest.TestCase):
    def test_aggregation_is_fail_dominant(self) -> None:
        self.assertEqual(aggregate(["PASS", "PASS"]), "PASS")
        self.assertEqual(aggregate(["PASS", "NOT_RUN"]), "NOT_RUN")
        self.assertEqual(aggregate(["BLOCKED", "NOT_RUN"]), "BLOCKED")
        self.assertEqual(aggregate(["FAIL", "BLOCKED", "NOT_RUN"]), "FAIL")

    def test_unknown_status_vocabulary_is_fail_not_pass(self) -> None:
        for unknown in ("GREEN", "pass", "", "COMPLETE"):
            with self.subTest(unknown=unknown):
                self.assertEqual(aggregate([unknown]), "FAIL")
                self.assertEqual(aggregate(["PASS", unknown]), "FAIL")


class BuildReportTests(unittest.TestCase):
    def test_all_pass_authoritative_report_completes_m2(self) -> None:
        report = complete_report()
        self.assertEqual(report["milestone_status"], "COMPLETE")
        self.assertEqual(report["m2_5_status"], "UNBLOCKED")
        claims = report["claims"]
        self.assertFalse(claims["real_magic_rules"])
        self.assertFalse(claims["real_card_support"])
        self.assertFalse(claims["m3_started"])
        self.assertFalse(claims["semantic_ownership_adr_accepted"])

    def test_development_mode_never_completes(self) -> None:
        report = complete_report(mode="development")
        self.assertEqual(report["milestone_status"], "INCOMPLETE")
        self.assertEqual(report["m2_5_status"], "BLOCKED")

    def test_missing_expect_commit_never_completes(self) -> None:
        report = complete_report(expected_commit=None)
        self.assertEqual(report["milestone_status"], "INCOMPLETE")

    def test_head_commit_mismatch_never_completes(self) -> None:
        report = complete_report(source_identity={"status": "PASS", "commit": "b" * 40})
        self.assertEqual(report["milestone_status"], "INCOMPLETE")

    def test_any_non_pass_gate_blocks_completion(self) -> None:
        for status in ("FAIL", "BLOCKED", "NOT_RUN"):
            gates = passing_gates()
            gates[7]["status"] = status
            with self.subTest(status=status):
                report = complete_report(gates=gates)
                self.assertEqual(report["milestone_status"], "INCOMPLETE")
                self.assertEqual(report["m2_5_status"], "BLOCKED")

    def test_duplicate_gate_registration_blocks_completion(self) -> None:
        gates = passing_gates()
        gates[1] = dict(gates[0])
        report = complete_report(gates=gates)
        self.assertEqual(report["milestone_status"], "INCOMPLETE")

    def test_missing_gate_registration_blocks_completion(self) -> None:
        gates = [gate for gate in passing_gates() if gate["name"] != EXPECTED_GATES[3]]
        report = complete_report(gates=gates)
        self.assertEqual(report["milestone_status"], "INCOMPLETE")

    def test_renamed_gate_blocks_completion(self) -> None:
        gates = passing_gates()
        gates[0]["name"] = "NOT_AN_M2_GATE"
        report = complete_report(gates=gates)
        self.assertEqual(report["milestone_status"], "INCOMPLETE")

    def test_non_pass_source_toolchain_or_regression_blocks(self) -> None:
        variants = [
            {"source_identity": {"status": "BLOCKED", "commit": COMMIT}},
            {"toolchains": {"status": "FAIL"}},
            {"m1_regression": {**passing_m1_regression(), "status": "NOT_RUN"}},
        ]
        for overrides in variants:
            with self.subTest(overrides=overrides):
                report = complete_report(**overrides)
                self.assertEqual(report["milestone_status"], "INCOMPLETE")


class ValidateSliceReportTests(unittest.TestCase):
    def test_valid_authoritative_child_report_has_no_problems(self) -> None:
        for child in CHILD_RUNNERS:
            with self.subTest(runner=child.slug):
                statuses, problems = validate_slice_report(
                    slice_report(child), child, COMMIT, returncode=0, strict=True
                )
                self.assertEqual(problems, [])
                self.assertEqual(set(statuses.values()), {"PASS"})

    def test_stale_child_commit_fails_strict_but_is_tolerated_in_development(self) -> None:
        child = CHILD_RUNNERS[0]
        report = slice_report(child, commit="c" * 40)
        _, strict_problems = validate_slice_report(report, child, COMMIT, returncode=0, strict=True)
        _, dev_problems = validate_slice_report(report, child, COMMIT, returncode=0, strict=False)
        self.assertTrue(any("pinned head" in problem for problem in strict_problems))
        self.assertEqual(dev_problems, [])

    def test_development_mode_child_is_rejected_in_strict_validation(self) -> None:
        child = CHILD_RUNNERS[1]
        report = slice_report(child, mode="development")
        _, problems = validate_slice_report(report, child, COMMIT, returncode=0, strict=True)
        self.assertTrue(any("not 'authoritative'" in problem for problem in problems))

    def test_dirty_child_identity_fails_strict_validation(self) -> None:
        child = CHILD_RUNNERS[2]
        report = slice_report(child, identity_status="BLOCKED")
        _, problems = validate_slice_report(report, child, COMMIT, returncode=0, strict=True)
        self.assertTrue(any("source_tree_identity" in problem for problem in problems))

    def test_duplicate_child_gate_registration_is_always_flagged(self) -> None:
        child = CHILD_RUNNERS[3]
        report = slice_report(child)
        report["gates"] = [*report["gates"], dict(report["gates"][0])]  # type: ignore[index]
        _, strict_problems = validate_slice_report(report, child, COMMIT, returncode=0, strict=True)
        _, dev_problems = validate_slice_report(report, child, COMMIT, returncode=0, strict=False)
        self.assertTrue(any("registration mismatch" in problem for problem in strict_problems))
        self.assertTrue(any("registration mismatch" in problem for problem in dev_problems))

    def test_m2_b_status_key_spelling_is_accepted(self) -> None:
        child = CHILD_RUNNERS[0]
        report = {
            "mode": "authoritative",
            "source_commit": COMMIT,
            "source_tree_identity": {"status": "PASS"},
            "gates": [{"name": child.gates[0], "status": "PASS"}],
        }
        statuses, problems = validate_slice_report(report, child, COMMIT, returncode=0, strict=True)
        self.assertEqual(problems, [])
        self.assertEqual(statuses[child.gates[0]], "PASS")

    def test_conflicting_gate_status_fields_fail_closed(self) -> None:
        child = CHILD_RUNNERS[0]
        report = {
            "mode": "authoritative",
            "source_commit": COMMIT,
            "source_tree_identity": {"status": "PASS"},
            "gates": [{"name": child.gates[0], "gate_status": "PASS", "status": "FAIL"}],
        }
        _, problems = validate_slice_report(report, child, COMMIT, returncode=0, strict=True)
        self.assertTrue(any("conflicting status fields" in problem for problem in problems))

    def test_invalid_gate_status_is_flagged(self) -> None:
        child = CHILD_RUNNERS[4]
        report = slice_report(child, status="GREEN")
        _, problems = validate_slice_report(report, child, COMMIT, returncode=0, strict=True)
        self.assertTrue(any("invalid status" in problem for problem in problems))

    def test_exit_code_contradictions_fail_closed_in_strict_mode(self) -> None:
        child = CHILD_RUNNERS[5]
        passing = slice_report(child)
        _, pass_problems = validate_slice_report(passing, child, COMMIT, returncode=3, strict=True)
        self.assertTrue(any("exited 3" in problem for problem in pass_problems))
        failing = slice_report(child, status="FAIL")
        _, fail_problems = validate_slice_report(failing, child, COMMIT, returncode=0, strict=True)
        self.assertTrue(any("did not validate cleanly" in problem for problem in fail_problems))
        _, dev_problems = validate_slice_report(failing, child, COMMIT, returncode=0, strict=False)
        self.assertFalse(any("validate cleanly" in problem for problem in dev_problems))

    def test_missing_gates_list_is_flagged(self) -> None:
        child = CHILD_RUNNERS[6]
        _, problems = validate_slice_report(
            {"mode": "authoritative"}, child, COMMIT, returncode=0, strict=True
        )
        self.assertTrue(any("no gates list" in problem for problem in problems))


class ValidateM1ReportTests(unittest.TestCase):
    def test_valid_complete_m1_report_has_no_problems(self) -> None:
        statuses, problems = validate_m1_report(m1_report(), COMMIT, returncode=0, strict=True)
        self.assertEqual(problems, [])
        self.assertEqual(sorted(statuses), sorted(M1_GATE_NAMES))

    def test_pass_without_executable_evidence_is_flagged(self) -> None:
        report = m1_report()
        report["gates"][-1]["evidence"][0]["tests_observed"] = 0  # type: ignore[index]
        _, problems = validate_m1_report(report, COMMIT, returncode=0, strict=True)
        self.assertTrue(any("executable per-test evidence" in problem for problem in problems))

    def test_wrong_m1_registration_is_flagged(self) -> None:
        report = m1_report()
        report["gates"] = list(report["gates"])[:-1]  # type: ignore[attr-defined]
        statuses, problems = validate_m1_report(report, COMMIT, returncode=0, strict=True)
        self.assertTrue(any("registration mismatch" in problem for problem in problems))
        self.assertEqual(statuses[M1_GATE_NAMES[-1]], "NOT_RUN")

    def test_incomplete_overall_fails_strict_only(self) -> None:
        report = m1_report(overall="INCOMPLETE")
        _, strict_problems = validate_m1_report(report, COMMIT, returncode=2, strict=True)
        _, dev_problems = validate_m1_report(report, COMMIT, returncode=2, strict=False)
        self.assertTrue(strict_problems)
        self.assertEqual(dev_problems, [])

    def test_stale_m1_head_is_flagged_strictly(self) -> None:
        report = m1_report()
        report["source_commit"] = "d" * 40
        _, problems = validate_m1_report(report, COMMIT, returncode=0, strict=True)
        self.assertTrue(any("pinned head" in problem for problem in problems))


class ComputeM1MatrixStatusTests(unittest.TestCase):
    def test_missing_run_is_not_run(self) -> None:
        self.assertEqual(compute_m1_matrix_status(None), "NOT_RUN")

    def test_failed_child_record_poisons_matrix_even_with_passing_gates(self) -> None:
        run = {"status": "FAIL", "gates": {name: "PASS" for name in M1_GATE_NAMES}}
        self.assertEqual(compute_m1_matrix_status(run), "FAIL")


class ScopeScanTests(unittest.TestCase):
    def test_clean_tree_scan_reports_file_count_without_violations(self) -> None:
        scanned, violations = final.scan_for_patterns(
            ROOT, final.SCOPE_SEARCH_PATTERNS, "search work"
        )
        self.assertGreater(scanned, 0)
        self.assertEqual(violations, [])

    def test_injected_search_vocabulary_is_detected(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            source = base / "crates" / "mtgml-rules" / "src"
            source.mkdir(parents=True)
            (source / "lib.rs").write_text("// harmless\n", encoding="utf-8")
            (source / "search.rs").write_text(
                'pub const STRATEGY: &str = "MCTS";\n', encoding="utf-8"
            )
            with self.assertRaises(final.ScopeCheckFailure) as caught:
                final.scan_for_patterns(base, final.SCOPE_SEARCH_PATTERNS, "search work")
            message = str(caught.exception)
            self.assertIn("search.rs", message)
            self.assertIn("MCTS search", message)

    def test_injected_heuristic_choice_is_detected(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            package = base / "python" / "src" / "mtgml"
            package.mkdir(parents=True)
            (package / "client.py").write_text("DEFAULT_CHOICE = auto_answer\n", encoding="utf-8")
            with self.assertRaises(final.ScopeCheckFailure):
                final.scan_for_patterns(
                    base, final.SCOPE_HEURISTIC_PATTERNS, "hidden choice completion"
                )

    def test_live_scope_checks_pass_on_this_tree(self) -> None:
        for name, function in final.SCOPE_CHECKS:
            with self.subTest(check=name):
                detail = function(ROOT)
                self.assertIsInstance(detail, str)
                self.assertTrue(detail)


class ChildCommandTests(unittest.TestCase):
    def test_expect_commit_and_development_flags_follow_child_capabilities(self) -> None:
        output = Path("out")
        slice_command = final.child_command(CHILD_RUNNERS[0], output, COMMIT, development=True)
        self.assertIn("--expect-commit", slice_command)
        self.assertIn("--development", slice_command)
        m1_command = final.child_command(M1_CHILD, output, COMMIT, development=False)
        self.assertNotIn("--expect-commit", m1_command)
        self.assertNotIn("--development", m1_command)


class MergeChildGateStatusesTests(unittest.TestCase):
    def test_healthy_records_merge_parsed_statuses(self) -> None:
        runs = [
            {
                "status": "PASS",
                "owned_gates": list(child.gates),
                "gates": {name: "PASS" for name in child.gates},
            }
            for child in CHILD_RUNNERS
        ]
        merged = merge_child_gate_statuses(runs)
        self.assertEqual(set(merged), set(EXPECTED_GATES) - {GATE_M1_REGRESSION})
        self.assertEqual(set(merged.values()), {"PASS"})

    def test_failed_record_poisons_all_gates_it_owns_even_when_parsed_pass(self) -> None:
        child = CHILD_RUNNERS[2]
        anomalous = {
            "status": "FAIL",
            "owned_gates": list(child.gates),
            "gates": {name: "PASS" for name in child.gates},
            "problems": ["child gate registration mismatch"],
        }
        healthy = {
            "status": "PASS",
            "owned_gates": list(CHILD_RUNNERS[0].gates),
            "gates": {name: "PASS" for name in CHILD_RUNNERS[0].gates},
        }
        merged = merge_child_gate_statuses([healthy, anomalous])
        self.assertEqual(merged[child.gates[0]], "FAIL")
        self.assertEqual(merged[child.gates[-1]], "FAIL")
        self.assertEqual(merged[CHILD_RUNNERS[0].gates[0]], "PASS")

    def test_missing_parsed_gate_defaults_to_not_run(self) -> None:
        run = {"status": "PASS", "owned_gates": ["X"], "gates": {}}
        self.assertEqual(merge_child_gate_statuses([run])["X"], "NOT_RUN")


class FinalizeSourceIdentityTests(unittest.TestCase):
    def clean_snapshot(self) -> dict[str, object]:
        return {
            "status": "PASS",
            "commit": COMMIT,
            "tree": "t" * 40,
            "clean": True,
            "fingerprint": "f" * 64,
        }

    def test_clean_and_identical_snapshots_pass(self) -> None:
        result = finalize_source_identity(self.clean_snapshot(), self.clean_snapshot())
        self.assertEqual(result["status"], "PASS")

    def test_dirty_before_blocks(self) -> None:
        before = {**self.clean_snapshot(), "status": "BLOCKED", "clean": False}
        result = finalize_source_identity(before, before)
        self.assertEqual(result["status"], "BLOCKED")

    def test_failed_after_fails(self) -> None:
        after = {**self.clean_snapshot(), "status": "FAIL"}
        result = finalize_source_identity(self.clean_snapshot(), after)
        self.assertEqual(result["status"], "FAIL")

    def test_fingerprint_drift_fails(self) -> None:
        after = {**self.clean_snapshot(), "fingerprint": "0" * 64}
        result = finalize_source_identity(self.clean_snapshot(), after)
        self.assertEqual(result["status"], "FAIL")


class PrepareOutputTests(unittest.TestCase):
    def test_refuses_to_replace_unowned_directory(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            (base / "payload.txt").write_text("precious\n", encoding="utf-8")
            with self.assertRaises(RuntimeError):
                prepare_output(base)
            self.assertTrue((base / "payload.txt").is_file())

    def test_owned_directory_is_recreated_with_marker(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "owned"
            prepare_output(out)
            marker = out / final.OUTPUT_MARKER
            self.assertTrue(marker.is_file())
            stale = out / "stale.json"
            stale.write_text("{}", encoding="utf-8")
            prepare_output(out)
            self.assertFalse(stale.exists())
            self.assertTrue(marker.is_file())


class ReadChildReportTests(unittest.TestCase):
    def test_unreadable_report_blocks(self) -> None:
        record: dict[str, object] = {}
        result = read_child_report(record, Path("Z:/definitely/missing/report.json"))
        self.assertIsNone(result)
        self.assertEqual(record["status"], "BLOCKED")

    def test_non_object_report_blocks(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "report.json"
            path.write_text("[1, 2, 3]\n", encoding="utf-8")
            record: dict[str, object] = {}
            result = read_child_report(record, path)
            self.assertIsNone(result)
            self.assertEqual(record["status"], "BLOCKED")

    def test_invalid_json_report_blocks(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "report.json"
            path.write_text("{truncated", encoding="utf-8")
            record: dict[str, object] = {}
            result = read_child_report(record, path)
            self.assertIsNone(result)
            self.assertEqual(record["status"], "BLOCKED")


class ScopeInventoryNegativeTests(unittest.TestCase):
    def member_manifest_tree(self, base: Path) -> None:
        (base / "Cargo.toml").write_text(
            '[workspace]\nmembers = ["crates/a"]\n[workspace.dependencies]\nserde = "=1"\n',
            encoding="utf-8",
        )
        crate = base / "crates" / "a"
        crate.mkdir(parents=True)

    def test_direct_registry_dependency_in_member_manifest_fails(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            self.member_manifest_tree(base)
            (base / "crates" / "a" / "Cargo.toml").write_text(
                '[package]\nname = "a"\n[dependencies]\nserde.workspace = true\ntokio = "1"\n',
                encoding="utf-8",
            )
            with self.assertRaises(final.ScopeCheckFailure):
                final.check_member_dependency_sources_pinned(base)

    def test_path_and_workspace_dependencies_in_members_pass(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            self.member_manifest_tree(base)
            (base / "crates" / "a" / "Cargo.toml").write_text(
                '[package]\nname = "a"\n[dependencies]\nserde.workspace = true\n'
                'b = { path = "../b" }\n',
                encoding="utf-8",
            )
            detail = final.check_member_dependency_sources_pinned(base)
            self.assertIn("workspace", detail)

    def test_python_dependency_group_fails(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            package = base / "python"
            package.mkdir()
            (package / "pyproject.toml").write_text(
                '[project]\nname = "x"\ndependencies = []\n'
                '[dependency-groups]\ntorch = ["torch"]\n',
                encoding="utf-8",
            )
            with self.assertRaises(final.ScopeCheckFailure):
                final.check_python_runtime_dependencies_empty(base)

    def test_nested_schema_addition_is_detected_by_recursive_scan(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            nested = base / "schemas" / "future"
            nested.mkdir(parents=True)
            (nested / "trajectory.v1.schema.json").write_text("{}", encoding="utf-8")
            with self.assertRaises(final.ScopeCheckFailure):
                final.check_schema_inventory_pinned(base)

    def test_deck_subdirectory_is_rejected(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            decks = base / "cards" / "decks"
            decks.mkdir(parents=True)
            (decks / "example-deck-a.json").write_text("{}", encoding="utf-8")
            (decks / "example-deck-b.json").write_text("{}", encoding="utf-8")
            (decks / "commander-lock").mkdir()
            definitions = base / "cards" / "definitions"
            definitions.mkdir()
            (base / "cards" / "generated").mkdir()
            with self.assertRaises(final.ScopeCheckFailure):
                final.check_card_and_deck_artifacts_unclaimed(base)


if __name__ == "__main__":
    unittest.main()
