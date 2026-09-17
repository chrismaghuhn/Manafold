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
    INFORMATION_STATE_SCHEMA,
    OBSERVATION_SCHEMA,
    ObservationEnvelope,
)
from mtgml.wire import decode_canonical


def digest_for_test(payload: bytes) -> str:
    return hashlib.sha256(b"mtgml.observation-digest.v1\x00" + payload).hexdigest()


def observation_wire(payload: bytes, digest_payload: bytes) -> dict[str, object]:
    return {
        "digest": digest_for_test(digest_payload),
        "payload_base64": base64.b64encode(payload).decode("ascii"),
        "payload_codec": "synthetic-m2-observation.v1",
        "perspective": "1",
        "schema_version": OBSERVATION_SCHEMA,
        "state_revision": "0",
    }


class ObservationDigestBindingTests(unittest.TestCase):
    def test_from_wire_rejects_payload_a_with_digest_b(self) -> None:
        payload = canonical_json_bytes(observation_wire(b"{}", b'{"x":1}'))
        with self.assertRaises(WireError) as caught:
            decode_canonical("observation-envelope.v1", payload)
        self.assertEqual(caught.exception.code, "semantic.observation")

    def test_to_wire_rejects_manually_constructed_payload_a_with_digest_b(self) -> None:
        value = ObservationEnvelope(
            OBSERVATION_SCHEMA,
            1,
            0,
            "synthetic-m2-observation.v1",
            "e30=",
            digest_for_test(b'{"x":1}'),
        )
        with self.assertRaises(WireError) as caught:
            value.to_wire()
        self.assertEqual(caught.exception.code, "semantic.observation")

    def test_matching_digest_is_accepted_by_both_python_wire_paths(self) -> None:
        payload = canonical_json_bytes(observation_wire(b"{}", b"{}"))
        decoded = decode_canonical("observation-envelope.v1", payload)
        self.assertIsInstance(decoded, ObservationEnvelope)
        self.assertEqual(decoded.to_wire()["payload_base64"], "e30=")

    def test_known_value_uses_exact_payload_bytes_not_base64_text(self) -> None:
        self.assertEqual(
            digest_for_test(b"{}"),
            "90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3",
        )
        self.assertNotEqual(digest_for_test(b"{}"), digest_for_test(b"e30="))

    def test_nested_information_and_player_step_paths_reuse_observation_decode(self) -> None:
        observation = observation_wire(b"{}", b'{"x":1}')
        information_state = {
            "current_observation": observation,
            "digest": "0" * 64,
            "perspective": "1",
            "private_history_length": 0,
            "public_history_length": 0,
            "schema_version": INFORMATION_STATE_SCHEMA,
            "state_revision": "0",
        }
        cases = (
            ("information-state-envelope.v1", information_state),
            (
                "player-step.v1",
                {
                    "information_state": information_state,
                    "observed_events": [],
                    "schema_version": "player-step.v1",
                    "status": {"kind": "running"},
                },
            ),
        )
        for contract, value in cases:
            with self.subTest(contract=contract):
                with self.assertRaises(WireError) as caught:
                    decode_canonical(contract, canonical_json_bytes(value))
                self.assertEqual(caught.exception.code, "semantic.observation")


if __name__ == "__main__":
    unittest.main()
