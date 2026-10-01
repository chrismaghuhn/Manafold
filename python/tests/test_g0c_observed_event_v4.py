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


class GameStartObservedEventTests(unittest.TestCase):
    def test_game_start_events_round_trip(self) -> None:
        for value in (
            {"kind": "starting_player_chosen", "chooser": "2", "starting_player": "1"},
            {"kind": "mulligan_declared", "player": "1", "mulligan": True},
            {"kind": "mulligan_declared", "player": "2", "mulligan": False},
        ):
            event = ObservedEventV4.from_wire(value)
            self.assertEqual(event.to_wire(), value)

    def test_malformed_game_start_events_are_rejected(self) -> None:
        for value in (
            {"kind": "starting_player_chosen", "chooser": "2"},
            {"kind": "starting_player_chosen", "chooser": 2, "starting_player": "1"},
            {"kind": "mulligan_declared", "player": "1", "mulligan": 1},
            {"kind": "mulligan_declared", "player": "1", "mulligan": True, "extra": 0},
        ):
            with self.assertRaises(WireError):
                ObservedEventV4.from_wire(value)


def _attackers_declared(**changes: object) -> dict[str, object]:
    value: dict[str, object] = {
        "kind": "attackers_declared",
        "attacking_player": "1",
        "defending_player": "2",
        "attackers": ["4", "9"],
    }
    value.update(changes)
    return value


class AttackersDeclaredObservedEventTests(unittest.TestCase):
    def test_attackers_declared_round_trips(self) -> None:
        for value in (
            _attackers_declared(),
            _attackers_declared(attackers=[]),
            _attackers_declared(attacking_player="2", defending_player="1", attackers=["7"]),
        ):
            event = ObservedEventV4.from_wire(value)
            self.assertEqual(event.kind, "attackers_declared")
            self.assertEqual(event.to_wire(), value)

    def test_malformed_attackers_declared_events_are_rejected(self) -> None:
        missing = _attackers_declared()
        del missing["defending_player"]
        for value, code in (
            (missing, "decode.invalid_json"),
            (_attackers_declared(extra=0), "decode.invalid_json"),
            (_attackers_declared(attackers="4"), "decode.invalid_json"),
            (_attackers_declared(attackers=[4]), "decode.invalid_json"),
            (_attackers_declared(attacking_player=1), "decode.invalid_json"),
            # Ascending and distinct, and the attacker is not the defender.
            (_attackers_declared(attackers=["9", "4"]), "semantic.observed_event"),
            (_attackers_declared(attackers=["4", "4"]), "semantic.observed_event"),
            (_attackers_declared(defending_player="1"), "semantic.observed_event"),
        ):
            with self.subTest(value=value), self.assertRaises(WireError) as raised:
                ObservedEventV4.from_wire(value)
            self.assertEqual(raised.exception.code, code)

    def test_the_wire_fixtures_decode(self) -> None:
        golden = json.loads(
            (ROOT / "wire/golden/observed-event-v4-attackers-declared.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(ObservedEventV4.from_wire(golden["event"]).to_wire(), golden["event"])
        negative = json.loads(
            (ROOT / "wire/negative/observed-event-v4-attackers-unordered.json").read_text(
                encoding="utf-8"
            )
        )
        with self.assertRaises(WireError) as raised:
            ObservedEventV4.from_wire(negative["event"])
        self.assertEqual(raised.exception.code, "semantic.observed_event")


if __name__ == "__main__":
    unittest.main()
