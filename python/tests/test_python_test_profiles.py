from __future__ import annotations

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
        self.assertNotIn("test_authority_review_worklist", loaded)
        self.assertNotIn("test_canary_review_packet", loaded)
        self.assertNotIn("test_canary_packet_qualification", loaded)
        self.assertNotIn("test_authority_source_resolver", loaded)

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


if __name__ == "__main__":
    unittest.main()
