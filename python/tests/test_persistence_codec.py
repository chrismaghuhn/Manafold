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
    MAX_ARRAY_ELEMENTS,
    MAX_BYTE_STRING_BYTES,
    MAX_DEPTH,
    MAX_IDENTIFIER_BYTES,
    MAX_ITEMS,
    MAX_PAYLOAD_BYTES,
    MAX_TEXT_BYTES,
    PersistenceError,
    calculate_checkpoint_digest_v3,
    decode_canonical,
    decode_envelope,
    encode_canonical,
    encode_envelope,
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

    def test_fnd_017a_rejects_local_checkpoint_identity_inputs(self) -> None:
        valid_counters = {
            "decisions_submitted": 0,
            "accepted_transitions": 0,
            "rule_events_emitted": 0,
            "resource_units_consumed": 0,
            "wall_clock_elapsed_millis": 0,
        }
        for codec_id, semantic_version, expected_label in (
            ("", "3", "empty codec id"),
            ("in-memory-reference", "", "empty semantic version"),
        ):
            with self.subTest(expected_label=expected_label):
                with self.assertRaises(PersistenceError) as caught:
                    calculate_checkpoint_digest_v3(
                        "07" * 32,
                        EpisodeStatus.running(),
                        valid_counters,
                        codec_id,
                        semantic_version,
                    )
                self.assertEqual(caught.exception.code, "semantic_validation")

        invalid_counters = {
            **valid_counters,
            "accepted_transitions": 1,
        }
        with self.assertRaises(PersistenceError) as caught:
            calculate_checkpoint_digest_v3(
                "07" * 32,
                EpisodeStatus.running(),
                invalid_counters,
                "in-memory-reference",
                "3",
            )
        self.assertEqual(caught.exception.code, "semantic_validation")

    def test_persistence_resource_boundaries_match_rust_contract(self) -> None:
        from mtgml.persistence import _Decoder

        def bytes_value(length: int) -> bytes:
            return b"\x5a" + length.to_bytes(4, "big") + bytes(length)

        def text_value(length: int) -> bytes:
            return b"\x7a" + length.to_bytes(4, "big") + (b"a" * length)

        def array_of_nulls(length: int) -> bytes:
            return b"\x9a" + length.to_bytes(4, "big") + (b"\xf6" * length)

        def nested_arrays(depth: int) -> bytes:
            return (b"\x81" * depth) + b"\x00"

        def item_boundary_payload(last_child_len: int) -> bytes:
            lengths = (1_048_576, 1_048_576, 1_048_576, last_child_len)
            payload = bytearray(b"\x84")
            for length in lengths:
                payload += b"\x9a" + length.to_bytes(4, "big") + (b"\xf6" * length)
            return bytes(payload)

        for total in (MAX_PAYLOAD_BYTES - 1, MAX_PAYLOAD_BYTES):
            self.assertEqual(len(bytes_value(total - 5)), total)
            self.assertEqual(len(decode_canonical(bytes_value(total - 5))), total - 5)
        with self.assertRaises(PersistenceError) as payload_error:
            decode_canonical(bytes(MAX_PAYLOAD_BYTES + 1))
        self.assertEqual(payload_error.exception.code, "payload_too_large")

        self.assertEqual(len(decode_canonical(text_value(MAX_TEXT_BYTES - 1))), MAX_TEXT_BYTES - 1)
        self.assertEqual(len(decode_canonical(text_value(MAX_TEXT_BYTES))), MAX_TEXT_BYTES)
        with self.assertRaises(PersistenceError) as text_error:
            decode_canonical(text_value(MAX_TEXT_BYTES + 1))
        self.assertEqual(text_error.exception.code, "string_too_large")

        self.assertEqual(
            len(decode_canonical(array_of_nulls(MAX_ARRAY_ELEMENTS - 1))), MAX_ARRAY_ELEMENTS - 1
        )
        self.assertEqual(
            len(decode_canonical(array_of_nulls(MAX_ARRAY_ELEMENTS))), MAX_ARRAY_ELEMENTS
        )
        with self.assertRaises(PersistenceError) as array_error:
            decode_canonical(array_of_nulls(MAX_ARRAY_ELEMENTS + 1))
        self.assertEqual(array_error.exception.code, "array_too_large")

        self.assertIsNotNone(decode_canonical(nested_arrays(MAX_DEPTH - 1)))
        self.assertIsNotNone(decode_canonical(nested_arrays(MAX_DEPTH)))
        with self.assertRaises(PersistenceError) as depth_error:
            decode_canonical(nested_arrays(MAX_DEPTH + 1))
        self.assertEqual(depth_error.exception.code, "depth_exceeded")

        exact_last_child_len = MAX_ITEMS - 5 - (3 * 1_048_576)
        self.assertIsNotNone(decode_canonical(item_boundary_payload(exact_last_child_len - 1)))
        self.assertIsNotNone(decode_canonical(item_boundary_payload(exact_last_child_len)))
        with self.assertRaises(PersistenceError) as items_error:
            decode_canonical(item_boundary_payload(exact_last_child_len + 1))
        self.assertEqual(items_error.exception.code, "item_limit_exceeded")

        for length in (MAX_BYTE_STRING_BYTES - 1, MAX_BYTE_STRING_BYTES):
            self.assertEqual(len(_Decoder(bytes_value(length)).value(0)), length)
        with self.assertRaises(PersistenceError) as byte_error:
            _Decoder(bytes_value(MAX_BYTE_STRING_BYTES + 1)).value(0)
        self.assertEqual(byte_error.exception.code, "payload_too_large")

        payload = encode_canonical([0])
        for length in (MAX_IDENTIFIER_BYTES - 1, MAX_IDENTIFIER_BYTES):
            self.assertTrue(encode_envelope("a" * length, "schema", payload))
        with self.assertRaises(PersistenceError) as identifier_error:
            encode_envelope("a" * (MAX_IDENTIFIER_BYTES + 1), "schema", payload)
        self.assertEqual(identifier_error.exception.code, "envelope_identity")


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

    def test_fnd_019_array_limit_precedes_depth_limit(self) -> None:
        from mtgml.persistence import MAX_DEPTH

        payload = (b"\x81" * MAX_DEPTH) + b"\x9a\x00\x10\x00\x01"
        with self.assertRaises(PersistenceError) as caught:
            decode_canonical(payload)
        self.assertEqual(caught.exception.code, "array_too_large")
