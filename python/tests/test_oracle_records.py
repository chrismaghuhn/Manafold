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
    b'{"object":"card","oracle_id":"aaaa","name":"Savannah Lions","layout":"normal",'
    b'"mana_cost":"{W}","type_line":"Creature \\u2014 Cat","oracle_text":"","keywords":[],'
    b'"power":"2","toughness":"1"}\n'
)
OGRE = (
    b'{"object":"card","oracle_id":"bbbb","name":"Gray Ogre","layout":"normal",'
    b'"mana_cost":"{2}{R}","type_line":"Creature \\u2014 Ogre","oracle_text":"","keywords":[],'
    b'"power":"2","toughness":"2"}\n'
)
MOUNTAIN = (
    b'{"object":"card","oracle_id":"cccc","name":"Mountain","layout":"normal","mana_cost":"",'
    b'"type_line":"Basic Land \\u2014 Mountain","oracle_text":"({T}: Add {R}.)","keywords":[]}\n'
)


def variant(base: bytes, **changes: object) -> bytes:
    """A record like `base` with some fields replaced, as one JSONL line."""
    record = json.loads(base)
    record.update(changes)
    return json.dumps(record, separators=(",", ":")).encode("ascii") + b"\n"


def without(base: bytes, field: str) -> bytes:
    record = json.loads(base)
    del record[field]
    return json.dumps(record, separators=(",", ":")).encode("ascii") + b"\n"


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
            self.assertEqual(records[1]["layout"], "normal")
            self.assertEqual(records[1]["keywords"], [])

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

    def test_a_record_is_found_by_its_oracle_id(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path, digest = archive(Path(directory), [LIONS, OGRE, MOUNTAIN])
            result = run(
                str(path),
                "--archive-sha256",
                digest,
                "Mountain",
                "--oracle-id",
                "bbbb",
                "--oracle-id",
                "aaaa",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            records = json.loads(result.stdout)
            self.assertEqual(
                [record["name"] for record in records],
                ["Mountain", "Gray Ogre", "Savannah Lions"],
                "names first, then oracle ids, each in the order given",
            )
            self.assertEqual(records[1]["record_sha256"], hashlib.sha256(OGRE).hexdigest())

    def test_an_unknown_or_repeated_oracle_id_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            twin = LIONS.replace(b"Savannah Lions", b"Savannah Cats")
            path, digest = archive(Path(directory), [LIONS, twin])
            unknown = run(str(path), "--archive-sha256", digest, "--oracle-id", "zzzz")
            self.assertNotEqual(unknown.returncode, 0)
            self.assertIn("zzzz", unknown.stderr)
            ambiguous = run(str(path), "--archive-sha256", digest, "--oracle-id", "aaaa")
            self.assertNotEqual(ambiguous.returncode, 0)
            self.assertIn("aaaa", ambiguous.stderr)

    def test_a_request_without_names_or_oracle_ids_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path, digest = archive(Path(directory), [LIONS])
            result = run(str(path), "--archive-sha256", digest)
            self.assertNotEqual(result.returncode, 0)

    def test_vanilla_creatures_pass_the_vanilla_check(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path, digest = archive(Path(directory), [LIONS, OGRE])
            result = run(
                str(path),
                "--archive-sha256",
                digest,
                "--vanilla-creature",
                "Savannah Lions",
                "--oracle-id",
                "bbbb",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(len(json.loads(result.stdout)), 2)

    def test_a_record_that_is_not_a_vanilla_creature_is_refused(self) -> None:
        cases: dict[str, tuple[bytes, str]] = {
            "a land": (MOUNTAIN, "oracle text"),
            "another layout": (variant(LIONS, layout="token"), "layout"),
            "card faces": (
                variant(LIONS, card_faces=[{"name": "Savannah Lions"}]),
                "card_faces",
            ),
            "oracle text": (variant(LIONS, oracle_text="Flying"), "oracle text"),
            "a keyword": (variant(LIONS, keywords=["Flying"]), "keywords"),
            "a missing keywords list": (without(LIONS, "keywords"), "keywords"),
            "a star power": (variant(LIONS, power="*"), "power"),
            "a variable toughness": (variant(LIONS, toughness="1+*"), "toughness"),
            "no toughness": (variant(LIONS, toughness=None), "toughness"),
            "zero toughness": (variant(LIONS, toughness="0"), "toughness"),
            "negative toughness": (variant(LIONS, toughness="-1"), "toughness"),
            "negative power": (variant(LIONS, power="-1"), "power"),
            "a supertype": (variant(LIONS, type_line="Legendary Creature — Cat"), "type line"),
            "another card type": (
                variant(LIONS, type_line="Artifact Creature — Cat"),
                "type line",
            ),
            "no subtype": (variant(LIONS, type_line="Creature"), "type line"),
        }
        for name, (line, reason) in cases.items():
            with self.subTest(name), tempfile.TemporaryDirectory() as directory:
                path, digest = archive(Path(directory), [line])
                card = json.loads(line)["name"]
                refused = run(str(path), "--archive-sha256", digest, "--vanilla-creature", card)
                self.assertNotEqual(refused.returncode, 0, refused.stdout)
                self.assertIn(card, refused.stderr)
                self.assertIn(reason, refused.stderr)
                self.assertEqual(refused.stdout, "", "nothing is printed for a refused record")

    def test_the_vanilla_check_is_only_applied_when_asked_for(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            flying = variant(LIONS, oracle_text="Flying")
            path, digest = archive(Path(directory), [MOUNTAIN, flying])
            result = run(str(path), "--archive-sha256", digest, "Mountain", "Savannah Lions")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(len(json.loads(result.stdout)), 2)


if __name__ == "__main__":
    unittest.main()
