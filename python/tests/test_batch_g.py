from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import generate_contracts  # noqa: E402


class GeneratedArtifactByteTests(unittest.TestCase):
    def test_write_generated_emits_exact_lf_utf8_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "generated.txt"
            generate_contracts.write_generated(target, "one\ntwo\n")
            self.assertEqual(target.read_bytes(), b"one\ntwo\n")

    def test_raw_byte_check_rejects_crlf_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "generated.txt"
            target.write_bytes(b"one\r\ntwo\r\n")
            self.assertFalse(generate_contracts.generated_bytes_match(target, "one\ntwo\n"))

    def test_raw_byte_check_accepts_exact_lf_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "generated.txt"
            target.write_bytes(b"one\ntwo\n")
            self.assertTrue(generate_contracts.generated_bytes_match(target, "one\ntwo\n"))


if __name__ == "__main__":
    unittest.main()
