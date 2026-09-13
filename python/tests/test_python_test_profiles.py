from __future__ import annotations

import subprocess
import sys
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import run_checks
import run_python_tests


class PythonTestProfileTests(unittest.TestCase):
    """The fast gate stays small while full discovery remains available."""

    @staticmethod
    def _passing_suite() -> unittest.TestSuite:
        return unittest.TestSuite([unittest.FunctionTestCase(lambda: None)])

    def _run_python_tests(self, *arguments: str) -> int:
        with mock.patch.object(sys, "argv", ["run_python_tests.py", *arguments]):
            return run_python_tests.main()

    def _run_checks(self, *arguments: str) -> int:
        with (
            mock.patch.object(sys, "argv", ["run_checks.py", *arguments]),
            mock.patch.object(run_checks, "reference_python_matches", return_value=True),
        ):
            try:
                return run_checks.main()
            except SystemExit as error:
                return int(error.code)

    def test_full_profile_keeps_unfiltered_discovery(self) -> None:
        with mock.patch.object(
            run_python_tests.unittest.defaultTestLoader,
            "discover",
            return_value=unittest.TestSuite(),
        ) as discover:
            run_python_tests.build_suite("full")

        discover.assert_called_once_with(str(ROOT / "python" / "tests"))

    def test_smoke_profile_uses_only_the_explicit_allowlist(self) -> None:
        loaded: list[str] = []

        def load(name: str) -> unittest.TestSuite:
            loaded.append(name)
            return self._passing_suite()

        with (
            mock.patch.object(
                run_python_tests.unittest.defaultTestLoader,
                "loadTestsFromName",
                side_effect=load,
            ),
            mock.patch.object(
                run_python_tests.unittest.defaultTestLoader,
                "discover",
            ) as discover,
        ):
            run_python_tests.build_suite("smoke")

        self.assertEqual(loaded, list(run_python_tests.SMOKE_TESTS))
        discover.assert_not_called()

    def test_full_profile_is_the_runner_default(self) -> None:
        self.assertEqual(run_python_tests.DEFAULT_PROFILE, "full")

    def test_fast_gate_uses_smoke_and_integration_adds_full(self) -> None:
        smoke = [sys.executable, "scripts/run_python_tests.py", "--profile", "smoke"]
        full = [sys.executable, "scripts/run_python_tests.py", "--profile", "full"]

        self.assertIn(smoke, run_checks.FAST)
        self.assertNotIn(
            [sys.executable, "scripts/run_python_tests.py"],
            run_checks.FAST,
        )
        self.assertIn(full, run_checks.INTEGRATION_EXTRA)

    def test_python_tools_are_bound_to_the_selected_interpreter(self) -> None:
        self.assertIn(
            [sys.executable, "-m", "ruff", "format", "--check", "python", "scripts"],
            run_checks.INTEGRATION_EXTRA,
        )
        self.assertIn(
            [sys.executable, "-m", "ruff", "check", "python", "scripts"],
            run_checks.INTEGRATION_EXTRA,
        )
        self.assertIn(
            [sys.executable, "-m", "mypy", "--config-file", "python/pyproject.toml"],
            run_checks.INTEGRATION_EXTRA,
        )
        self.assertNotIn(
            ["ruff", "format", "--check", "python", "scripts"],
            run_checks.INTEGRATION_EXTRA,
        )
        self.assertNotIn(
            ["ruff", "check", "python", "scripts"],
            run_checks.INTEGRATION_EXTRA,
        )
        self.assertNotIn(
            ["mypy", "--config-file", "python/pyproject.toml"],
            run_checks.INTEGRATION_EXTRA,
        )

    def test_wrong_reference_python_is_rejected_before_running_checks(self) -> None:
        output = StringIO()
        with (
            mock.patch.object(sys, "argv", ["run_checks.py", "fast"]),
            mock.patch.object(run_checks, "reference_python_matches", return_value=False),
            mock.patch.object(run_checks, "run") as run,
            redirect_stdout(output),
        ):
            result = run_checks.main()

        self.assertNotEqual(result, 0)
        run.assert_not_called()
        self.assertIn("Python", output.getvalue())

    def test_full_profile_rejects_zero_discovered_tests(self) -> None:
        with mock.patch.object(
            run_python_tests,
            "build_suite",
            return_value=unittest.TestSuite(),
        ):
            result = self._run_python_tests("--profile", "full")

        self.assertNotEqual(result, 0)

    def test_smoke_profile_rejects_an_empty_allowlisted_member(self) -> None:
        loaded: list[str] = []

        def load(name: str) -> unittest.TestSuite:
            loaded.append(name)
            if name == run_python_tests.SMOKE_TESTS[0]:
                return unittest.TestSuite()
            return self._passing_suite()

        with mock.patch.object(
            run_python_tests.unittest.defaultTestLoader,
            "loadTestsFromName",
            side_effect=load,
        ):
            result = self._run_python_tests("--profile", "smoke")

        self.assertNotEqual(result, 0)
        self.assertEqual(loaded, [run_python_tests.SMOKE_TESTS[0]])

    def test_nonempty_full_profile_preserves_success(self) -> None:
        suite = self._passing_suite()
        with mock.patch.object(run_python_tests, "build_suite", return_value=suite):
            result = self._run_python_tests("--profile", "full")

        self.assertEqual(result, 0)

    def test_nonempty_smoke_profile_preserves_success(self) -> None:
        suite = self._passing_suite()
        with mock.patch.object(run_python_tests, "build_suite", return_value=suite):
            result = self._run_python_tests("--profile", "smoke")

        self.assertEqual(result, 0)

    def test_incomplete_execution_result_is_rejected(self) -> None:
        suite = self._passing_suite()
        result = mock.Mock()
        result.wasSuccessful.return_value = True
        result.testsRun = 0
        runner = mock.Mock()
        runner.run.return_value = result

        with (
            mock.patch.object(run_python_tests, "build_suite", return_value=suite),
            mock.patch.object(
                run_python_tests.unittest,
                "TextTestRunner",
                return_value=runner,
            ),
        ):
            exit_code = self._run_python_tests("--profile", "full")

        self.assertNotEqual(exit_code, 0)
        runner.run.assert_called_once_with(suite)

    def test_allow_missing_tools_is_allowed_for_fast(self) -> None:
        with mock.patch.object(run_checks, "command_available", return_value=False):
            result = self._run_checks("fast", "--allow-missing-tools")

        self.assertEqual(result, 0)

    def test_allow_missing_tools_is_rejected_for_integration(self) -> None:
        with mock.patch.object(run_checks, "command_available", return_value=False):
            result = self._run_checks("integration", "--allow-missing-tools")

        self.assertNotEqual(result, 0)

    def test_allow_missing_tools_is_rejected_for_certification(self) -> None:
        with mock.patch.object(run_checks, "command_available", return_value=False):
            result = self._run_checks("certification", "--allow-missing-tools")

        self.assertNotEqual(result, 0)

    def test_strict_profiles_reject_missing_tools(self) -> None:
        with mock.patch.object(run_checks, "command_available", return_value=False):
            integration_result = self._run_checks("integration")
            certification_result = self._run_checks("certification")

        self.assertNotEqual(integration_result, 0)
        self.assertNotEqual(certification_result, 0)

    def test_failed_subprocess_remains_nonzero(self) -> None:
        commands = [["tool-a"], ["tool-b"]]
        results = [
            subprocess.CompletedProcess(commands[0], 7),
            subprocess.CompletedProcess(commands[1], 0),
        ]

        with (
            mock.patch.object(run_checks, "command_available", return_value=True),
            mock.patch.object(run_checks.subprocess, "run", side_effect=results) as run,
        ):
            result = run_checks.run(commands, allow_missing=False)

        self.assertEqual(result, 7)
        run.assert_called_once_with(
            commands[0],
            cwd=run_checks.ROOT,
            timeout=run_checks.MAX_SINGLE_GATE_SUBPROCESS_RUNTIME_SECONDS,
        )

    def test_timed_out_subprocess_fails_closed_with_rerun_diagnostic(self) -> None:
        command = ["tool-a", "--flag"]
        output = StringIO()

        with (
            mock.patch.object(run_checks, "command_available", return_value=True),
            mock.patch.object(
                run_checks.subprocess,
                "run",
                side_effect=subprocess.TimeoutExpired(command, 600),
            ),
            mock.patch("time.perf_counter", side_effect=[1.0, 601.0]),
            redirect_stdout(output),
        ):
            try:
                result = run_checks.run([command], allow_missing=False)
            except subprocess.TimeoutExpired as error:
                self.fail(f"runner leaked subprocess timeout: {error}")

        self.assertEqual(result, 124)
        self.assertEqual(
            output.getvalue(),
            "RUN tool-a --flag\n"
            "TIMEOUT 600.000s tool-a --flag (limit=600s)\n"
            "RERUN: tool-a --flag\n",
        )

    def test_successful_subprocess_reports_duration_and_command(self) -> None:
        command = ["tool-a", "--flag"]
        output = StringIO()

        with (
            mock.patch.object(run_checks, "command_available", return_value=True),
            mock.patch.object(
                run_checks.subprocess,
                "run",
                return_value=subprocess.CompletedProcess(command, 0),
            ),
            mock.patch("time.perf_counter", side_effect=[2.0, 2.125]),
            redirect_stdout(output),
        ):
            result = run_checks.run([command], allow_missing=False)

        self.assertEqual(result, 0)
        self.assertEqual(
            output.getvalue(),
            "RUN tool-a --flag\nPASS 0.125s tool-a --flag\n",
        )

    def test_failed_subprocess_reports_duration_and_rerun_command(self) -> None:
        command = ["tool-a", "--flag"]
        output = StringIO()

        with (
            mock.patch.object(run_checks, "command_available", return_value=True),
            mock.patch.object(
                run_checks.subprocess,
                "run",
                return_value=subprocess.CompletedProcess(command, 7),
            ),
            mock.patch("time.perf_counter", side_effect=[3.0, 3.25]),
            redirect_stdout(output),
        ):
            result = run_checks.run([command], allow_missing=False)

        self.assertEqual(result, 7)
        self.assertEqual(
            output.getvalue(),
            "RUN tool-a --flag\nFAIL 0.250s tool-a --flag\nRERUN: tool-a --flag\n",
        )

    def test_missing_tool_reports_exact_rerun_command(self) -> None:
        command = ["missing-tool", "--flag"]
        output = StringIO()

        with (
            mock.patch.object(run_checks, "command_available", return_value=False),
            redirect_stdout(output),
        ):
            result = run_checks.run([command], allow_missing=False)

        self.assertEqual(result, 2)
        self.assertEqual(
            output.getvalue(),
            "MISSING TOOL: missing-tool\nRERUN: missing-tool --flag\n",
        )

    def test_fast_missing_tool_skip_reports_no_pass(self) -> None:
        command = ["missing-tool", "--flag"]
        output = StringIO()

        with (
            mock.patch.object(run_checks, "command_available", return_value=False),
            redirect_stdout(output),
        ):
            result = run_checks.run([command], allow_missing=True)

        self.assertEqual(result, 0)
        self.assertIn("MISSING TOOL: missing-tool", output.getvalue())
        self.assertIn("RERUN: missing-tool --flag", output.getvalue())
        self.assertNotIn("PASS", output.getvalue())


if __name__ == "__main__":
    unittest.main()
