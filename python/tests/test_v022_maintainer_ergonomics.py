from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class V022MaintainerErgonomicsTests(unittest.TestCase):
    def run_script(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, *args],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_generated_contracts_are_not_drifting(self) -> None:
        result = self.run_script("scripts/generate_contracts.py", "--check")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_synthetic_golden_path_fails_closed_at_certification(self) -> None:
        result = self.run_script("scripts/validate_golden_path.py")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("fails closed", result.stdout)


if __name__ == "__main__":
    unittest.main()
