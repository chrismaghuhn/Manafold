from __future__ import annotations

import json
import unittest
from pathlib import Path

from mtgml.persistence import (
    CHECKPOINT_DOMAIN_V8,
    CHECKPOINT_INPUT_SCHEMA_V8,
    FULL_STATE_DOMAIN_V7,
    FULL_STATE_INPUT_SCHEMA_V7,
    decode_canonical,
    encode_canonical,
    encode_envelope,
    hash_envelope,
)

ROOT = Path(__file__).resolve().parents[2]


class G0CDigestIdentityFixtureTests(unittest.TestCase):
    def _check_enveloped_fixture(
        self, name: str, schema_id: str, domain: str, header_index: int = 0
    ) -> list[object]:
        fixture = json.loads((ROOT / "persistence/golden" / name).read_text(encoding="utf-8"))
        payload = bytes.fromhex(fixture["canonical_payload_hex"])
        value = decode_canonical(payload)
        self.assertEqual(encode_canonical(value), payload)
        self.assertEqual(value[header_index], schema_id)
        self.assertEqual(value[header_index + 1], domain)
        self.assertEqual(
            hash_envelope(encode_envelope(domain, schema_id, payload)).hex(),
            fixture["expected_digest"],
        )
        return value

    def test_identity_fixture_inventory_stays_detached(self) -> None:
        index = json.loads(
            (ROOT / "persistence/golden/m4-g0c-digest-identity-fixtures.v1.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(
            index["status"],
            "detached canonical identity vectors; not runtime-writer evidence",
        )
        self.assertEqual(
            {item["path"] for item in index["fixtures"]},
            {"full-state-digest-v7-kat.v1.json", "checkpoint-digest-v8-kat.v1.json"},
        )
        self.assertTrue(all(not item["state_validity_claim"] for item in index["fixtures"]))

    def test_full_state_digest_v7_fixture_binds_successor_children(self) -> None:
        value = self._check_enveloped_fixture(
            "full-state-digest-v7-kat.v1.json",
            FULL_STATE_INPUT_SCHEMA_V7,
            FULL_STATE_DOMAIN_V7,
        )
        self.assertEqual(len(value), 14)
        self.assertEqual(value[4][0], "zones_v2")
        self.assertEqual(value[6][0], "execution_v4")

    def test_checkpoint_digest_v8_fixture_binds_v7_state_and_codec(self) -> None:
        value = self._check_enveloped_fixture(
            "checkpoint-digest-v8-kat.v1.json",
            CHECKPOINT_INPUT_SCHEMA_V8,
            CHECKPOINT_DOMAIN_V8,
        )
        self.assertEqual(len(value), 7)
        self.assertEqual(value[2][2], FULL_STATE_DOMAIN_V7)
        self.assertEqual(value[2][4], FULL_STATE_INPUT_SCHEMA_V7)
        self.assertEqual(value[5], ["in-memory-reference", "8"])


if __name__ == "__main__":
    unittest.main()
