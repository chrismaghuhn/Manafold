from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class DocumentationContractTests(unittest.TestCase):
    def test_documentation_checker_passes(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(ROOT / "scripts/check_documentation.py")],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stdout)

    def test_maintainer_artifact_validator_passes(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(ROOT / "scripts/validate_maintainer_artifacts.py")],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stdout)


if __name__ == "__main__":
    unittest.main()
