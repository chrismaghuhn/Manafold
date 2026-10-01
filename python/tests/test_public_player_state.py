from __future__ import annotations

import copy
import json
import unittest
from collections.abc import Callable
from pathlib import Path

from mtgml.errors import WireError
from mtgml.magic_shared_execution_observation_v1 import MagicSharedExecutionObservationV1
from mtgml.observation import MagicBasicLandObservationV1, PlayerObservationV1

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


if __name__ == "__main__":
    unittest.main()
