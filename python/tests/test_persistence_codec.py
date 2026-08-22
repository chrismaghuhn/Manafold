from __future__ import annotations

import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.episode import EpisodeStatus
from mtgml.persistence import (
    PersistenceError,
    calculate_checkpoint_digest_v3,
    decode_canonical,
    decode_envelope,
)


class PersistenceCodecTests(unittest.TestCase):
    def test_cross_language_mechanical_golden_vectors(self) -> None:
        golden = ROOT / "persistence" / "golden"
        manifest = json.loads((golden / "manifest.json").read_text(encoding="utf-8"))
        for entry in manifest["fixtures"]:
            with self.subTest(path=entry["path"]):
                payload = (golden / entry["path"]).read_bytes()
                if entry["contract"] == "canonical-cbor.v1":
                    self.assertEqual(decode_canonical(payload), ["input.v1", 7])
                else:
                    reference, decoded = decode_envelope(payload)
                    self.assertEqual(decoded, bytes.fromhex("8268696e7075742e763107"))
                    self.assertEqual(hashlib.sha256(payload).hexdigest(), entry["sha256"])
                    self.assertEqual(reference["semantic_domain"], "mtgml.test-domain.v1")

    def test_cross_language_mechanical_negative_categories(self) -> None:
        negative = ROOT / "persistence" / "negative"
        manifest = json.loads((negative / "manifest.json").read_text(encoding="utf-8"))
        for entry in manifest["fixtures"]:
            with self.subTest(path=entry["path"]):
                with self.assertRaises(PersistenceError) as caught:
                    payload = (negative / entry["path"]).read_bytes()
                    if entry["contract"] == "canonical-cbor.v1":
                        decode_canonical(payload)
                    else:
                        decode_envelope(payload)
                self.assertEqual(caught.exception.code, entry["expected_error_code"])

    def test_checkpoint_digest_known_answer_matches_rust(self) -> None:
        counters = {
            "decisions_submitted": 0,
            "accepted_transitions": 0,
            "rule_events_emitted": 0,
            "resource_units_consumed": 0,
            "wall_clock_elapsed_millis": 0,
        }
        self.assertEqual(
            calculate_checkpoint_digest_v3(
                "07" * 32,
                EpisodeStatus.running(),
                counters,
                "mtgml.canonical-cbor.v1",
                "v3",
            ),
            "b0cf94e1f49fb58feb6ebc07d88b2a7e226be78c1ca92ee7b9772d4f51290f6c",
        )


def test_cross_language_mechanical_golden_vectors() -> None:
    test = PersistenceCodecTests("test_cross_language_mechanical_golden_vectors")
    test.test_cross_language_mechanical_golden_vectors()


def test_cross_language_mechanical_negative_categories() -> None:
    test = PersistenceCodecTests("test_cross_language_mechanical_negative_categories")
    test.test_cross_language_mechanical_negative_categories()


if __name__ == "__main__":
    unittest.main()


class PayloadFramingPrecedenceTests(unittest.TestCase):
    """The payload frame must end the envelope exactly: trailing bytes are a
    framing defect (rank 3) reported before the resource bound (rank 4)."""

    def test_trailing_byte_beats_payload_too_large(self) -> None:
        from mtgml.persistence import MAX_PAYLOAD_BYTES, decode_envelope

        declared = MAX_PAYLOAD_BYTES + 1

        def build(total_payload_bytes: int) -> bytes:
            envelope = bytearray()
            envelope += b"mtgml.digest-envelope.v1\x00"
            for field in (
                b"sha-256",
                b"mtgml.test-domain.v1",
                b"mtgml.canonical-cbor.v1",
                b"test-input.v1",
            ):
                envelope += len(field).to_bytes(8, "big") + field
            envelope += declared.to_bytes(8, "big")
            envelope += bytes(total_payload_bytes)
            return bytes(envelope)

        with self.assertRaises(PersistenceError) as exact:
            decode_envelope(build(declared))
        self.assertEqual(exact.exception.code, "payload_too_large")

        with self.assertRaises(PersistenceError) as trailing:
            decode_envelope(build(declared) + b"\x00")
        self.assertEqual(trailing.exception.code, "envelope_length")
