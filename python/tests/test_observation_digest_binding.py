from __future__ import annotations

import base64
import hashlib
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.canonical import canonical_json_bytes
from mtgml.errors import WireError
from mtgml.observation import (
    MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1,
    OBSERVATION_SCHEMA_V2,
    ObservationEnvelope,
)
from mtgml.wire import decode_canonical


def digest_for_test(payload: bytes) -> str:
    return hashlib.sha256(b"mtgml.observation-digest.v1\x00" + payload).hexdigest()


def observation_wire(
    payload: bytes, digest_payload: bytes, payload_base64: str | None = None
) -> dict[str, object]:
    return {
        "digest": digest_for_test(digest_payload),
        "payload_base64": payload_base64 or base64.b64encode(payload).decode("ascii"),
        "payload_codec": MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1,
        "perspective": "1",
        "schema_version": OBSERVATION_SCHEMA_V2,
        "view_sequence": "0",
    }


class ObservationDigestBindingTests(unittest.TestCase):
    def test_from_wire_rejects_payload_a_with_digest_b(self) -> None:
        payload = canonical_json_bytes(observation_wire(b"{}", b'{"x":1}'))
        with self.assertRaises(WireError) as caught:
            decode_canonical("observation-envelope.v2", payload)
        self.assertEqual(caught.exception.code, "semantic.observation")

    def test_from_wire_rejects_invalid_base64(self) -> None:
        payload = canonical_json_bytes(observation_wire(b"{}", b"{}", payload_base64="***"))
        with self.assertRaises(WireError) as caught:
            decode_canonical("observation-envelope.v2", payload)
        self.assertEqual(caught.exception.code, "semantic.observation")

    def test_to_wire_rejects_manually_constructed_payload_a_with_digest_b(self) -> None:
        value = ObservationEnvelope(
            OBSERVATION_SCHEMA_V2,
            1,
            0,
            MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1,
            "e30=",
            digest_for_test(b'{"x":1}'),
        )
        with self.assertRaises(WireError) as caught:
            value.to_wire()
        self.assertEqual(caught.exception.code, "semantic.observation")

    def test_matching_digest_is_accepted_by_both_python_wire_paths(self) -> None:
        payload = canonical_json_bytes(observation_wire(b"{}", b"{}"))
        decoded = decode_canonical("observation-envelope.v2", payload)
        self.assertIsInstance(decoded, ObservationEnvelope)
        self.assertEqual(decoded.to_wire()["payload_base64"], "e30=")

    def test_known_value_uses_exact_payload_bytes_not_base64_text(self) -> None:
        self.assertEqual(
            digest_for_test(b"{}"),
            "90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3",
        )
        self.assertNotEqual(digest_for_test(b"{}"), digest_for_test(b"e30="))


if __name__ == "__main__":
    unittest.main()
