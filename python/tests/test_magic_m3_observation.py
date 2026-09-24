from __future__ import annotations

import json
import unittest
from pathlib import Path

from mtgml.canonical import canonical_json_bytes
from mtgml.errors import WireError
from mtgml.observation import (
    MAGIC_OBSERVATION_SCHEMA_V1,
    MAGIC_OBSERVATION_SCHEMA_V2,
    MAGIC_OBSERVATION_SCHEMA_V3,
)
from mtgml.wire import decode_canonical, encode_canonical

ROOT = Path(__file__).resolve().parents[1]


class MagicObservationTests(unittest.TestCase):
    def test_combat_payload_roundtrips_perspective_local_participation(self) -> None:
        payload = (
            (ROOT / ".." / "wire" / "golden" / "magic-combat-observation.v2.json")
            .read_bytes()
            .strip()
        )
        value = decode_canonical(MAGIC_OBSERVATION_SCHEMA_V2, payload)
        self.assertEqual(encode_canonical(value), payload)
        self.assertEqual(value.combat.attackers, (7,))
        self.assertEqual(value.combat.defending_player, 2)

    def test_v3_combat_payload_roundtrips_bounded_blocker_assignment(self) -> None:
        payload = (
            (ROOT / ".." / "wire" / "golden" / "magic-combat-observation.v3.json")
            .read_bytes()
            .strip()
        )
        value = decode_canonical(MAGIC_OBSERVATION_SCHEMA_V3, payload)
        self.assertEqual(encode_canonical(value), payload)
        self.assertEqual(value.combat.attackers, (7,))
        self.assertEqual(value.combat.blockers[0].blocker, 8)

    def test_v2_meaning_still_rejects_v3_payload_bytes(self) -> None:
        payload = (
            (ROOT / ".." / "wire" / "golden" / "magic-combat-observation.v3.json")
            .read_bytes()
            .strip()
        )
        with self.assertRaises(WireError):
            decode_canonical(MAGIC_OBSERVATION_SCHEMA_V2, payload)

    def test_v3_rejects_an_object_that_is_both_attacker_and_blocker(self) -> None:
        value = json.loads(
            (ROOT / ".." / "wire" / "golden" / "magic-combat-observation.v3.json").read_bytes()
        )
        value["combat"]["blockers"][0]["blocker"] = "7"
        with self.assertRaises(WireError):
            decode_canonical(MAGIC_OBSERVATION_SCHEMA_V3, canonical_json_bytes(value))

    def test_golden_magic_payloads_roundtrip_and_keep_perspective_local_ids(self) -> None:
        names = (
            "magic-m3-observation-null.json",
            "magic-m3-observation-stage-0.json",
            "magic-m3-observation-stage-1-p1.json",
            "magic-m3-observation-stage-1-p2.json",
        )
        values = []
        for name in names:
            payload = (ROOT / ".." / "wire" / "golden" / name).read_bytes().strip()
            value = decode_canonical(MAGIC_OBSERVATION_SCHEMA_V1, payload)
            self.assertEqual(encode_canonical(value), payload)
            values.append(value)
        self.assertEqual(values[0].pending_sba_ordering, None)
        self.assertEqual(values[1].pending_sba_ordering.next_order_owner, 1)
        p1 = values[2].pending_sba_ordering
        p2 = values[3].pending_sba_ordering
        self.assertEqual(p1.completed_orders[0].owner, p2.completed_orders[0].owner)
        self.assertEqual(p1.completed_orders[0].ordered_objects, (901, 902))
        self.assertEqual(p2.completed_orders[0].ordered_objects, (801, 802))
        self.assertEqual(p1.next_order_owner, p2.next_order_owner)

    def test_magic_negative_fixtures_are_rejected(self) -> None:
        manifest = json.loads((ROOT / ".." / "wire" / "negative" / "manifest.json").read_text())
        cases = [
            case for case in manifest["fixtures"] if case["contract"] == MAGIC_OBSERVATION_SCHEMA_V1
        ]
        self.assertGreaterEqual(len(cases), 8)
        for case in cases:
            payload = (ROOT / ".." / "wire" / "negative" / case["path"]).read_bytes().strip()
            with self.subTest(fixture=case["path"]), self.assertRaises(WireError):
                decode_canonical(MAGIC_OBSERVATION_SCHEMA_V1, payload)
