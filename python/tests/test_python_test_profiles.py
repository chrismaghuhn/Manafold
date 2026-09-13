from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import run_checks
import run_python_tests


class PythonTestProfileTests(unittest.TestCase):
    """The fast gate stays small while full discovery remains available."""

    def _run_checks(self, *arguments: str) -> int:
        with mock.patch.object(sys, "argv", ["run_checks.py", *arguments]):
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
            return unittest.TestSuite()

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
        run.assert_called_once_with(commands[0], cwd=run_checks.ROOT)


if __name__ == "__main__":
    unittest.main()
