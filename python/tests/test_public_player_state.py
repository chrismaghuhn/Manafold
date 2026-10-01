from __future__ import annotations

import copy
import json
import unittest
from collections.abc import Callable
from pathlib import Path

from mtgml.errors import WireError
from mtgml.magic_shared_execution_observation_v1 import MagicSharedExecutionObservationV1
from mtgml.observation import (
    CreatureObservationV1,
    MagicBasicLandObservationV1,
    PlayerObservationV1,
)

ROOT = Path(__file__).resolve().parents[2]


def _example(name: str) -> dict[str, object]:
    value = json.loads((ROOT / "schemas/examples" / name).read_text(encoding="utf-8"))
    assert isinstance(value, dict)
    return value


class PublicPlayerStateTests(unittest.TestCase):
    def test_observation_shows_life_card_counts_and_tapped_permanents(self) -> None:
        value = _example("magic-basic-land-observation-v1.json")
        observation = MagicBasicLandObservationV1.from_wire(value)
        self.assertEqual(
            observation.players,
            (
                PlayerObservationV1(player=0, life=20, hand_count=6, library_count=33),
                PlayerObservationV1(player=1, life=-2, hand_count=7, library_count=33),
            ),
        )
        self.assertEqual(observation.tapped, (7,))
        self.assertEqual(observation.to_wire()["players"], value["players"])
        self.assertEqual(observation.to_wire()["tapped"], value["tapped"])

    def test_shared_observation_carries_the_same_fields(self) -> None:
        value = _example("magic-shared-execution-observation-v1.json")
        observation = MagicSharedExecutionObservationV1.from_wire(value)
        self.assertEqual(observation.base_observation.tapped, (7,))
        self.assertEqual(observation.to_wire()["players"], value["players"])

    def test_malformed_player_state_is_rejected(self) -> None:
        value = _example("magic-basic-land-observation-v1.json")

        def players(edit: Callable[[list[dict[str, object]]], None]) -> dict[str, object]:
            broken = copy.deepcopy(value)
            rows = broken["players"]
            assert isinstance(rows, list)
            edit(rows)
            return broken

        def field(key: str, replacement: object) -> dict[str, object]:
            broken = copy.deepcopy(value)
            broken[key] = replacement
            return broken

        def entry(key: str, replacement: object) -> dict[str, object]:
            return players(lambda rows: rows[0].__setitem__(key, replacement))

        missing = copy.deepcopy(value)
        del missing["tapped"]
        for broken in (
            players(lambda rows: rows.reverse()),
            players(lambda rows: rows.__setitem__(1, dict(rows[0]))),
            field("active_player", "2"),
            field("tapped", ["7", "7"]),
            field("tapped", ["8", "7"]),
            entry("life", "20"),
            entry("life", True),
            entry("life", 2**63),
            entry("hand_count", -1),
            entry("library_count", 2**32),
            entry("extra", 0),
            missing,
        ):
            with self.assertRaises(WireError):
                MagicBasicLandObservationV1.from_wire(broken)


class CreatureObservationTests(unittest.TestCase):
    def test_observation_shows_creatures_and_attackers(self) -> None:
        value = _example("magic-basic-land-observation-v1.json")
        observation = MagicBasicLandObservationV1.from_wire(value)
        self.assertEqual(
            observation.creatures,
            (
                CreatureObservationV1(
                    object=3, controller=1, power=3, toughness=3, controlled_since_turn=1
                ),
                CreatureObservationV1(
                    object=7, controller=0, power=2, toughness=1, controlled_since_turn=1
                ),
            ),
        )
        self.assertEqual(observation.attacking, (7,))
        self.assertEqual(observation.to_wire()["creatures"], value["creatures"])
        self.assertEqual(observation.to_wire()["attacking"], value["attacking"])

    def test_shared_observation_carries_the_same_fields(self) -> None:
        value = _example("magic-shared-execution-observation-v1.json")
        observation = MagicSharedExecutionObservationV1.from_wire(value)
        self.assertEqual(observation.base_observation.attacking, (7,))
        self.assertEqual(len(observation.base_observation.creatures), 2)
        self.assertEqual(observation.to_wire()["creatures"], value["creatures"])
        self.assertEqual(observation.to_wire()["attacking"], value["attacking"])

    def test_malformed_creatures_and_attackers_are_rejected(self) -> None:
        value = _example("magic-basic-land-observation-v1.json")

        def creatures(edit: Callable[[list[dict[str, object]]], None]) -> dict[str, object]:
            broken = copy.deepcopy(value)
            rows = broken["creatures"]
            assert isinstance(rows, list)
            edit(rows)
            return broken

        def field(key: str, replacement: object) -> dict[str, object]:
            broken = copy.deepcopy(value)
            broken[key] = replacement
            return broken

        def entry(key: str, replacement: object) -> dict[str, object]:
            return creatures(lambda rows: rows[0].__setitem__(key, replacement))

        missing = copy.deepcopy(value)
        del missing["attacking"]
        no_creatures = copy.deepcopy(value)
        del no_creatures["creatures"]
        no_controller = creatures(lambda rows: rows[0].pop("controller"))
        # Attackers that are creatures, in order, are accepted, and so are the
        # ends of the arrival turn's range.
        MagicBasicLandObservationV1.from_wire(field("attacking", ["3", "7"]))
        for turn in ("0", str(2**64 - 1)):
            MagicBasicLandObservationV1.from_wire(entry("controlled_since_turn", turn))
        # Each case is wrong in exactly one way, and says so.
        for broken, reason in (
            (creatures(lambda rows: rows.reverse()), "creature rows are not ordered"),
            (
                creatures(lambda rows: rows.__setitem__(0, dict(rows[1]))),
                "creature rows are not ordered",
            ),
            (field("attacking", ["7", "3"]), "attacking creatures are not ordered"),
            (field("attacking", ["7", "7"]), "attacking creatures are not ordered"),
            (field("attacking", ["8"]), "an attacker is not a creature"),
            (field("attacking", ["3", "8"]), "an attacker is not a creature"),
            (entry("power", "3"), "power is outside its i64 range"),
            (entry("power", True), "power is outside its i64 range"),
            (entry("power", 2**63), "power is outside its i64 range"),
            (entry("toughness", "3"), "toughness is outside its i64 range"),
            (entry("controlled_since_turn", 1), "expected canonical unsigned decimal string"),
            (entry("controlled_since_turn", -1), "expected canonical unsigned decimal string"),
            (entry("controlled_since_turn", True), "expected canonical unsigned decimal string"),
            (entry("controlled_since_turn", None), "expected canonical unsigned decimal string"),
            (entry("controlled_since_turn", "01"), "expected canonical unsigned decimal string"),
            (entry("controlled_since_turn", "+1"), "expected canonical unsigned decimal string"),
            (entry("controlled_since_turn", "-1"), "expected canonical unsigned decimal string"),
            (entry("controlled_since_turn", "1a"), "expected canonical unsigned decimal string"),
            (entry("controlled_since_turn", " 1"), "expected canonical unsigned decimal string"),
            (entry("controlled_since_turn", ""), "expected canonical unsigned decimal string"),
            (entry("controlled_since_turn", str(2**64)), "unsigned integer is out of range"),
            (entry("controller", 1), "expected canonical unsigned decimal string"),
            (entry("extra", 0), "closed contract"),
            (no_controller, "closed contract"),
            (no_creatures, "closed contract"),
            (missing, "closed contract"),
        ):
            with self.assertRaisesRegex(WireError, reason):
                MagicBasicLandObservationV1.from_wire(broken)


if __name__ == "__main__":
    unittest.main()
