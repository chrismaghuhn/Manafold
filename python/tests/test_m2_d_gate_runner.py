from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from run_m2_d_gates import exact_python_pass

NODE = (
    "python/tests/test_player_api.py::PlayerStepSubmissionContractTests::"
    "test_step_level_rejection_invariants"
)
TAIL = NODE.rsplit("::", 1)[-1]
PASSED_LINE = f"python/tests/test_player_api.py::{TAIL} PASSED                        [100%]"


def output(
    summary: str | None,
    *,
    passed_line: bool = True,
    node: str | None = None,
) -> str:
    parts: list[str] = []
    if passed_line:
        shown = PASSED_LINE if node is None else f"python/tests/x.py::{node} SKIPPED         [100%]"
        parts.append(shown)
    if summary is not None:
        parts.append("")
        parts.append(f"============================== {summary} ==============================")
    return "\n".join(parts) + "\n"


class ExactPythonPassMatrixTests(unittest.TestCase):
    """exact_python_pass must stay fail-closed: exit 0 alone never proves
    that the addressed exact test executed and passed (run_m2_d_gates)."""

    def test_single_pass_is_accepted(self) -> None:
        self.assertTrue(exact_python_pass(output("1 passed in 0.06s"), 0, NODE))

    def test_warnings_alongside_a_single_pass_are_accepted(self) -> None:
        self.assertTrue(exact_python_pass(output("1 passed, 3 warnings in 0.10s"), 0, NODE))

    def test_exit_code_zero_with_skip_is_rejected(self) -> None:
        self.assertFalse(exact_python_pass(output("1 skipped in 0.01s"), 0, NODE))

    def test_exit_code_zero_with_xfail_is_rejected(self) -> None:
        self.assertFalse(exact_python_pass(output("1 xfailed in 0.01s"), 0, NODE))

    def test_passed_plus_deselected_substitute_is_rejected(self) -> None:
        self.assertFalse(exact_python_pass(output("1 deselected, 1 passed in 0.01s"), 0, NODE))

    def test_two_passes_are_rejected(self) -> None:
        self.assertFalse(exact_python_pass(output("2 passed in 0.01s"), 0, NODE))

    def test_missing_statistics_summary_is_rejected(self) -> None:
        self.assertFalse(exact_python_pass(output(None), 0, NODE))

    def test_quiet_progress_only_output_is_rejected(self) -> None:
        quiet = (
            ".                                                                        [100%]\n"
            "1 passed in 0.06s\n"
        )
        self.assertFalse(exact_python_pass(quiet, 0, NODE))

    def test_summary_without_the_addressed_node_is_rejected(self) -> None:
        other = (
            "python/tests/test_other.py::OtherTests::test_something_else PASSED   [100%]\n"
            "\n============================== 1 passed in 0.01s ==============================\n"
        )
        self.assertFalse(exact_python_pass(other, 0, NODE))

    def test_nonzero_exit_code_is_rejected_even_with_a_clean_summary(self) -> None:
        self.assertFalse(exact_python_pass(output("1 passed in 0.06s"), 1, NODE))


if __name__ == "__main__":
    unittest.main()
