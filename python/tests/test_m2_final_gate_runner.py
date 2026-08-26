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
        self.assertTrue(any("non-PASS gates" in problem for problem in fail_problems))
        _, dev_problems = validate_slice_report(failing, child, COMMIT, returncode=0, strict=False)
        self.assertFalse(any("non-PASS gates" in problem for problem in dev_problems))

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


if __name__ == "__main__":
    unittest.main()
