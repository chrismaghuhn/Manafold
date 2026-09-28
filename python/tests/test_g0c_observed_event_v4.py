from __future__ import annotations

import json
import unittest
from pathlib import Path

from mtgml._events_v4 import ObservedEventV4
from mtgml.errors import WireError

ROOT = Path(__file__).resolve().parents[2]


class ObservedEventV4DecoderTests(unittest.TestCase):
    def test_unhashable_kind_and_causes_fail_with_wire_error(self) -> None:
        with self.assertRaises(WireError):
            ObservedEventV4.from_wire({"kind": [], "event": {}})

        removal = json.loads(
            (ROOT / "schemas/examples/observed-event-v4-stack-removed.json").read_text(
                encoding="utf-8"
            )
        )["event"]
        removal["cause"] = []
        with self.assertRaises(WireError):
            ObservedEventV4.from_wire(removal)

        mana = json.loads(
            (ROOT / "schemas/examples/observed-event-v4-mana-spent.json").read_text(
                encoding="utf-8"
            )
        )["event"]
        mana["cause"] = {"unknown": True}
        with self.assertRaises(WireError):
            ObservedEventV4.from_wire(mana)


if __name__ == "__main__":
    unittest.main()
