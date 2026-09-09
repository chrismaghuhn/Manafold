from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import cast

from mtgml.authority import (
    AuthorityContractError,
    ParticipantRoleBridgeEntryV1,
    ParticipantRoleBridgeV1,
)
from mtgml.persistence import encode_canonical

ROOT = Path(__file__).resolve().parents[2]


def load_matrix(name: str) -> dict[str, object]:
    return json.loads((ROOT / "conformance" / "fixtures" / "authority" / name).read_text())


def bridge_from_entries(entries: list[dict[str, object]]) -> ParticipantRoleBridgeV1:
    return ParticipantRoleBridgeV1(
        tuple(ParticipantRoleBridgeEntryV1(**entry) for entry in entries)
    )


class ParticipantRoleBridgeV1Tests(unittest.TestCase):
    def test_golden_structural_values_have_exact_cbor_and_wire_forms(self) -> None:
        matrix = load_matrix("participant_role_bridge_golden_matrix.v1.json")
        for case in cast(list[dict[str, object]], matrix["valid"]):
            bridge = bridge_from_entries(cast(list[dict[str, object]], case["entries"]))
            self.assertEqual(bridge.to_cbor(), case["cbor"])
            self.assertEqual(bridge.to_wire(), case["wire"])
            self.assertEqual(
                ParticipantRoleBridgeV1.from_cbor(case["cbor"]).to_wire(),
                case["wire"],
            )
            self.assertEqual(
                encode_canonical(bridge.to_cbor()),
                encode_canonical(ParticipantRoleBridgeV1.from_cbor(case["cbor"]).to_cbor()),
            )

    def test_structural_negative_matrix_fails_closed(self) -> None:
        matrix = load_matrix("participant_role_bridge_negative_matrix.v1.json")
        for case in cast(list[dict[str, object]], matrix["negative"]):
            with self.subTest(case=case["name"]), self.assertRaises(AuthorityContractError):
                ParticipantRoleBridgeV1.from_cbor(case["cbor"])

    def test_position_order_is_structural_not_a_role_inference(self) -> None:
        bridge = bridge_from_entries(
            [
                {
                    "position": 0,
                    "participant_kind": "requirement_family",
                    "semantic_ref": "cap.mass_destruction",
                    "historical_source_role": "ordered_participant",
                    "reviewed_role": "affected",
                }
            ]
        )
        self.assertEqual(bridge.entries[0].position, 0)
        self.assertEqual(bridge.entries[0].reviewed_role, "affected")


if __name__ == "__main__":
    unittest.main()
