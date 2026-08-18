from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class MaintainerScaffoldTests(unittest.TestCase):
    def test_card_scaffold_creates_manifest_and_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            command = [
                sys.executable,
                str(ROOT / "scripts/scaffold_card.py"),
                "test/card/example",
                "Example",
                "--root",
                str(work),
            ]
            first = subprocess.run(command, text=True, capture_output=True, check=False)
            self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
            manifest = work / "cards/definitions/test/card/example/manifest.json"
            self.assertTrue(manifest.is_file())
            value = json.loads(manifest.read_text(encoding="utf-8"))
            self.assertEqual(value["definition_id"], "test/card/example")
            second = subprocess.run(command, text=True, capture_output=True, check=False)
            self.assertNotEqual(second.returncode, 0)

    def test_capability_scaffold_registers_proposal(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            registry = work / "cards/capabilities/registry.json"
            registry.parent.mkdir(parents=True)
            registry.write_text(
                json.dumps(
                    {
                        "schema_version": "capability-registry.v1",
                        "registry_id": "test/capabilities",
                        "entries": [],
                    }
                ),
                encoding="utf-8",
            )
            command = [
                sys.executable,
                str(ROOT / "scripts/scaffold_capability.py"),
                "mechanic/example",
                "Example",
                "--root",
                str(work),
            ]
            completed = subprocess.run(command, text=True, capture_output=True, check=False)
            self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
            value = json.loads(registry.read_text(encoding="utf-8"))
            self.assertEqual(value["entries"][0]["key"], "mechanic/example")
            self.assertTrue((work / "docs/rules/capabilities/mechanic/example.md").is_file())


if __name__ == "__main__":
    unittest.main()
