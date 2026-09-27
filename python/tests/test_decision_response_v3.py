from __future__ import annotations

import unittest

from mtgml.decision import DecisionAnswerV2, DecisionResponseV3
from mtgml.errors import WireError
from mtgml.wire import decode_canonical, encode_canonical


class DecisionResponseV3Tests(unittest.TestCase):
    def test_round_trip_uses_view_sequence_without_global_revision(self) -> None:
        response = DecisionResponseV3(
            "decision-response.v3",
            2**64 - 1,
            0,
            DecisionAnswerV2("select_one", candidate_id=2),
        )
        wire = response.to_wire()
        self.assertNotIn("state_revision", wire)
        self.assertEqual(wire["view_sequence"], "0")
        self.assertEqual(
            decode_canonical("decision-response.v3", encode_canonical(response)), response
        )

    def test_rejects_global_revision_and_unknown_version(self) -> None:
        base = {
            "schema_version": "decision-response.v3",
            "player_decision_id": "1",
            "view_sequence": "2",
            "answer": {"kind": "select_one", "candidate_id": 0},
        }
        with self.assertRaises(WireError):
            DecisionResponseV3.from_wire(base | {"state_revision": "3"})
        with self.assertRaises(WireError):
            DecisionResponseV3.from_wire(base | {"schema_version": "decision-response.v2"})


if __name__ == "__main__":
    unittest.main()
