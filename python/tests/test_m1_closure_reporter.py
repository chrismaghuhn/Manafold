from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from run_m1_closure import GATE_NAMES, build_report, exact_test_passed


def gate_results(status: str) -> list[dict[str, object]]:
    return [
        {
            "name": name,
            "status": status,
            "surface": "synthetic M1 proof",
            "evidence": [],
        }
        for name in GATE_NAMES
    ]


class M1ClosureReporterTests(unittest.TestCase):
    def test_all_ten_passes_complete_m1_and_unblocks_m2(self) -> None:
        report = build_report(
            {"status": "PASS", "commit": "a" * 40},
            {"status": "PASS"},
            gate_results("PASS"),
            generated_at="2026-08-21T00:00:00+00:00",
        )

        self.assertEqual(report["milestone_status"], "COMPLETE")
        self.assertEqual(report["m2_status"], "UNBLOCKED")
        self.assertEqual(
            report["claims"],
            {
                "playable_engine": False,
                "real_magic_rules": False,
                "real_card_support": False,
            },
        )

    def test_non_pass_gate_fails_closed(self) -> None:
        for status in ("NOT_RUN", "FAIL", "BLOCKED"):
            with self.subTest(status=status):
                report = build_report(
                    {"status": "PASS", "commit": "b" * 40},
                    {"status": "PASS"},
                    gate_results(status),
                )
                self.assertEqual(report["milestone_status"], "INCOMPLETE")
                self.assertEqual(report["m2_status"], "BLOCKED")

    def test_toolchain_failure_blocks_even_when_all_gates_pass(self) -> None:
        report = build_report(
            {"status": "PASS", "commit": "c" * 40},
            {"status": "FAIL"},
            gate_results("PASS"),
        )

        self.assertEqual(report["milestone_status"], "INCOMPLETE")
        self.assertEqual(report["m2_status"], "BLOCKED")

    def test_exact_test_output_requires_one_executed_passing_test(self) -> None:
        passing = (
            "running 1 test\n"
            "test tests::proof ... ok\n"
            "test result: ok. 1 passed; 0 failed; 0 ignored"
        )
        no_match = "running 0 tests\ntest result: ok. 0 passed; 0 failed; 1 filtered out"

        self.assertTrue(exact_test_passed(passing, 0))
        self.assertFalse(exact_test_passed(no_match, 0))
        self.assertFalse(exact_test_passed(passing, 1))


if __name__ == "__main__":
    unittest.main()
