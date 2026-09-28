from __future__ import annotations

import json
import unittest
from pathlib import Path

from mtgml.decision_v4 import CandidateIntentV4, CostRouteDescriptorV1, PrintedManaSymbolsV1
from mtgml.errors import WireError

ROOT = Path(__file__).resolve().parents[2]


class CostRouteDescriptorV1Tests(unittest.TestCase):
    def test_route_descriptors_round_trip_closed_typed_symbol_counts(self) -> None:
        request = json.loads(
            (ROOT / "schemas/examples/player-decision-request-v4-cost-route.json").read_text(
                encoding="utf-8"
            )
        )
        intents = [
            CandidateIntentV4.from_wire(candidate["intent"])
            for candidate in request["candidates"]
        ]
        self.assertEqual(
            [intent.to_wire() for intent in intents],
            [candidate["intent"] for candidate in request["candidates"]],
        )

    def test_rejects_route_class_ordinal_mismatch_and_out_of_range_ordinal(self) -> None:
        with self.assertRaises(WireError):
            CostRouteDescriptorV1.from_wire(
                {
                    "route_class": "normal",
                    "printed_mana_symbols": {
                        "colored_wubrg_counts": [0, 0, 0, 0, 0],
                        "colorless_count": 0,
                        "generic_count": 1,
                    },
                    "profile_local_option_ordinal": 4,
                }
            )

        malformed = CostRouteDescriptorV1(
            "alternative",
            PrintedManaSymbolsV1((0, 0, 0, 0, 0), 0, 1),
            -1,
        )
        with self.assertRaises(WireError):
            malformed.to_wire()


if __name__ == "__main__":
    unittest.main()
