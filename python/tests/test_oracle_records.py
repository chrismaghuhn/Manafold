from __future__ import annotations

import gzip
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/extract_oracle_records.py"

LIONS = (
    b'{"object":"card","oracle_id":"aaaa","name":"Savannah Lions","mana_cost":"{W}",'
    b'"type_line":"Creature \\u2014 Cat","oracle_text":"","power":"2","toughness":"1"}\n'
)
OGRE = (
    b'{"object":"card","oracle_id":"bbbb","name":"Gray Ogre","mana_cost":"{2}{R}",'
    b'"type_line":"Creature \\u2014 Ogre","oracle_text":"","power":"2","toughness":"2"}\n'
)


def archive(directory: Path, lines: list[bytes]) -> tuple[Path, str]:
    path = directory / "oracle-cards.jsonl.gz"
    path.write_bytes(gzip.compress(b"".join(lines), mtime=0))
    return path, hashlib.sha256(path.read_bytes()).hexdigest()


def run(*arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *arguments], text=True, capture_output=True, check=False
    )


class OracleRecordExtractionTests(unittest.TestCase):
    def test_records_carry_the_digest_of_their_exact_line_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path, digest = archive(Path(directory), [LIONS, OGRE])
            result = run(str(path), "--archive-sha256", digest, "Gray Ogre", "Savannah Lions")
            self.assertEqual(result.returncode, 0, result.stderr)
            records = json.loads(result.stdout)
            names = [record["name"] for record in records]
            self.assertEqual(names, ["Gray Ogre", "Savannah Lions"])
            self.assertEqual(records[0]["oracle_id"], "bbbb")
            self.assertEqual(records[0]["record_sha256"], hashlib.sha256(OGRE).hexdigest())
            self.assertEqual(records[1]["record_sha256"], hashlib.sha256(LIONS).hexdigest())
            self.assertEqual(records[1]["type_line"], "Creature — Cat")
            self.assertEqual(records[1]["mana_cost"], "{W}")
            self.assertEqual((records[1]["power"], records[1]["toughness"]), ("2", "1"))
            self.assertEqual(records[1]["oracle_text"], "")

    def test_a_different_archive_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path, _ = archive(Path(directory), [LIONS])
            result = run(str(path), "--archive-sha256", "0" * 64, "Savannah Lions")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("archive SHA-256", result.stderr)

    def test_a_missing_or_ambiguous_name_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path, digest = archive(Path(directory), [LIONS, LIONS.replace(b"aaaa", b"cccc")])
            missing = run(str(path), "--archive-sha256", digest, "Gray Ogre")
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn("Gray Ogre", missing.stderr)
            ambiguous = run(str(path), "--archive-sha256", digest, "Savannah Lions")
            self.assertNotEqual(ambiguous.returncode, 0)
            self.assertIn("Savannah Lions", ambiguous.stderr)


if __name__ == "__main__":
    unittest.main()
