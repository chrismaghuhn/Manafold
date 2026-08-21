from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import run_m1_closure as closure
from run_m1_closure import GATE_NAMES, build_report, exact_test_passed


def gate_results(status: str, *, with_evidence: bool = True) -> list[dict[str, object]]:
    return [
        {
            "name": name,
            "status": status,
            "surface": "synthetic M1 proof",
            "evidence": (
                [
                    {
                        "status": status,
                        "command": ["cargo", "test", "--exact", name],
                        "test": name,
                        "returncode": 0,
                        "tests_observed": 1,
                    }
                ]
                if with_evidence
                else []
            ),
        }
        for name in GATE_NAMES
    ]


class M1ClosureReporterTests(unittest.TestCase):
    def test_pass_gates_without_executable_evidence_do_not_complete(self) -> None:
        report = build_report(
            {"status": "PASS", "commit": "d" * 40},
            {"status": "PASS"},
            gate_results("PASS", with_evidence=False),
        )

        self.assertEqual(report["milestone_status"], "INCOMPLETE")
        self.assertEqual(report["m2_status"], "BLOCKED")

    def test_wrong_gate_names_do_not_complete(self) -> None:
        gates = gate_results("PASS")
        gates[0]["name"] = "NOT_AN_M1_GATE"
        report = build_report(
            {"status": "PASS", "commit": "e" * 40},
            {"status": "PASS"},
            gates,
        )

        self.assertEqual(report["milestone_status"], "INCOMPLETE")
        self.assertEqual(report["m2_status"], "BLOCKED")

    def test_rust_version_matching_is_component_exact(self) -> None:
        outputs = {
            "rustc": "rustc 1.85.10 (fake 2026-01-01)",
            "cargo": "cargo 1.85.1 (fake 2026-01-01)",
            "active_toolchain": "1.85.1-x86_64-pc-windows-msvc (overridden)",
        }

        def fake_run(command: tuple[str, ...]) -> subprocess.CompletedProcess[str]:
            output = outputs.get(command[0], "Python 3.13.15")
            if command[0] == "rustup":
                output = outputs["active_toolchain"]
            return subprocess.CompletedProcess(command, 0, output, "")

        with (
            patch.object(closure, "command_available", return_value=True),
            patch.object(closure, "run_command", side_effect=fake_run),
        ):
            result = closure.capture_toolchain()

        self.assertEqual(result["status"], "FAIL")

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
