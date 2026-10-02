from __future__ import annotations

import copy
import json
import unittest
from collections.abc import Callable
from pathlib import Path

from mtgml.errors import WireError
from mtgml.magic_shared_execution_observation_v1 import MagicSharedExecutionObservationV1
from mtgml.observation import (
    AssignedDamageObservationV1,
    BlockObservationV1,
    DeclaredBlockObservationV1,
    MagicBasicLandObservationV1,
    PermanentObservationV1,
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


class PermanentObservationTests(unittest.TestCase):
    def test_observation_shows_every_permanent_with_its_controller_and_the_attackers(
        self,
    ) -> None:
        value = _example("magic-basic-land-observation-v1.json")
        observation = MagicBasicLandObservationV1.from_wire(value)
        # Two creatures and a non-creature, each with its controller.
        self.assertEqual(
            observation.permanents,
            (
                PermanentObservationV1(
                    object=6,
                    controller=1,
                    controlled_since_turn=2,
                    power=3,
                    toughness=3,
                    marked_damage=2,
                ),
                PermanentObservationV1(
                    object=7,
                    controller=0,
                    controlled_since_turn=1,
                    power=2,
                    toughness=2,
                    marked_damage=1,
                ),
                PermanentObservationV1(
                    object=8,
                    controller=0,
                    controlled_since_turn=3,
                    power=None,
                    toughness=None,
                    marked_damage=0,
                ),
            ),
        )
        self.assertEqual(observation.attacking, (7,))
        self.assertEqual(observation.to_wire()["permanents"], value["permanents"])
        self.assertEqual(observation.to_wire()["attacking"], value["attacking"])
        rows = value["permanents"]
        assert isinstance(rows, list)
        self.assertIsNone(rows[2]["power"])
        self.assertIsNone(rows[2]["toughness"])
        self.assertIsInstance(rows[0]["controlled_since_turn"], str)

    def test_shared_observation_carries_the_same_fields(self) -> None:
        # The shared example has temporary effects, so no row is a creature:
        # the projection shows no creature while an effect could change it.
        value = _example("magic-shared-execution-observation-v1.json")
        observation = MagicSharedExecutionObservationV1.from_wire(value)
        base = observation.base_observation
        self.assertEqual([row.object for row in base.permanents], [3, 4, 7, 8])
        self.assertEqual({row.controller for row in base.permanents}, {0, 1})
        self.assertTrue(all(row.power is None for row in base.permanents))
        self.assertEqual(base.attacking, ())
        self.assertEqual(observation.to_wire()["permanents"], value["permanents"])
        self.assertEqual(observation.to_wire()["attacking"], value["attacking"])

    def test_malformed_permanents_and_attackers_are_rejected(self) -> None:
        value = _example("magic-basic-land-observation-v1.json")

        def permanents(edit: Callable[[list[dict[str, object]]], None]) -> dict[str, object]:
            broken = copy.deepcopy(value)
            rows = broken["permanents"]
            assert isinstance(rows, list)
            edit(rows)
            return broken

        def field(key: str, replacement: object) -> dict[str, object]:
            broken = copy.deepcopy(value)
            broken[key] = replacement
            return broken

        def entry(key: str, replacement: object, row: int = 0) -> dict[str, object]:
            return permanents(lambda rows: rows[row].__setitem__(key, replacement))

        missing = copy.deepcopy(value)
        del missing["attacking"]
        no_permanents = copy.deepcopy(value)
        del no_permanents["permanents"]
        old_name = copy.deepcopy(value)
        old_name["creatures"] = []
        no_controller = permanents(lambda rows: rows[0].pop("controller"))
        no_power = permanents(lambda rows: rows[0].pop("power"))
        # Attackers that are creature rows, in order, are accepted, and so are
        # the ends of the arrival turn's range.
        MagicBasicLandObservationV1.from_wire(field("attacking", ["6", "7"]))
        nobody_attacks = field("attacking", [])
        nobody_attacks["blocked"] = []
        nobody_attacks["blocking"] = []
        MagicBasicLandObservationV1.from_wire(nobody_attacks)
        for turn in ("0", str(2**64 - 1)):
            MagicBasicLandObservationV1.from_wire(entry("controlled_since_turn", turn))
        pairing = "power and toughness are not both present or both null"
        canonical = "expected canonical unsigned decimal string"
        # Each case is wrong in exactly one way, and says so.
        for broken, reason in (
            (permanents(lambda rows: rows.reverse()), "permanent rows are not ordered"),
            (
                permanents(lambda rows: rows.__setitem__(0, dict(rows[1]))),
                "permanent rows are not ordered",
            ),
            (entry("toughness", None, 1), pairing),
            (entry("power", 1, 2), pairing),
            (entry("toughness", 1, 2), pairing),
            (field("attacking", ["7", "6"]), "attacking creatures are not ordered"),
            (field("attacking", ["7", "7"]), "attacking creatures are not ordered"),
            (field("attacking", ["8"]), "an attacker is not a creature"),
            (field("attacking", ["7", "8"]), "an attacker is not a creature"),
            (field("attacking", ["9"]), "an attacker is not a creature"),
            (entry("power", "3"), "power is outside its i64 range"),
            (entry("power", True), "power is outside its i64 range"),
            (entry("power", 2**63), "power is outside its i64 range"),
            (entry("toughness", "3"), "toughness is outside its i64 range"),
            (entry("controlled_since_turn", 1), canonical),
            (entry("controlled_since_turn", -1), canonical),
            (entry("controlled_since_turn", True), canonical),
            (entry("controlled_since_turn", None), canonical),
            (entry("controlled_since_turn", "01"), canonical),
            (entry("controlled_since_turn", "+1"), canonical),
            (entry("controlled_since_turn", "-1"), canonical),
            (entry("controlled_since_turn", "1a"), canonical),
            (entry("controlled_since_turn", " 1"), canonical),
            (entry("controlled_since_turn", ""), canonical),
            (entry("controlled_since_turn", str(2**64)), "unsigned integer is out of range"),
            (entry("controller", 1), canonical),
            (entry("extra", 0), "closed contract"),
            (no_controller, "closed contract"),
            (no_power, "closed contract"),
            (no_permanents, "closed contract"),
            (old_name, "closed contract"),
            (missing, "closed contract"),
        ):
            with self.assertRaisesRegex(WireError, reason):
                MagicBasicLandObservationV1.from_wire(broken)


class BlockObservationTests(unittest.TestCase):
    def test_observation_shows_the_blocks_and_marked_damage(self) -> None:
        # CR 509.1g, 509.1h, 120.3e.
        value = _example("magic-basic-land-observation-v1.json")
        observation = MagicBasicLandObservationV1.from_wire(value)
        self.assertEqual(observation.attacking, (7,))
        self.assertEqual(observation.blocked, (7,))
        self.assertEqual(observation.blocking, (BlockObservationV1(blocker=6, attacker=7),))
        # The partial answers are not told yet: the keys are there, and null.
        self.assertIsNone(observation.pending_blocks)
        self.assertIsNone(observation.pending_damage_assignment)
        self.assertIsNone(value["pending_blocks"])
        self.assertIsNone(value["pending_damage_assignment"])
        self.assertEqual([row.marked_damage for row in observation.permanents], [2, 1, 0])
        # Marked damage is a decimal string on the wire, as the arrival turn is.
        rows = value["permanents"]
        assert isinstance(rows, list)
        self.assertEqual([row["marked_damage"] for row in rows], ["2", "1", "0"])
        for key in ("blocked", "blocking", "pending_blocks", "pending_damage_assignment"):
            self.assertEqual(observation.to_wire()[key], value[key])
        self.assertEqual(observation.to_wire()["permanents"], rows)

    def test_a_blocker_whose_attacker_left_combat_blocks_nothing(self) -> None:
        value = _example("magic-basic-land-observation-v1-ordered.json")
        observation = MagicBasicLandObservationV1.from_wire(value)
        self.assertEqual(observation.blocked, ())
        self.assertEqual(observation.blocking, (BlockObservationV1(blocker=5, attacker=None),))
        blocking = value["blocking"]
        assert isinstance(blocking, list)
        self.assertIsNone(blocking[0]["attacker"])
        self.assertEqual(observation.to_wire()["blocking"], blocking)

    def test_shared_observation_carries_the_same_fields(self) -> None:
        value = _example("magic-shared-execution-observation-v1.json")
        observation = MagicSharedExecutionObservationV1.from_wire(value)
        base = observation.base_observation
        self.assertEqual((base.blocked, base.blocking), ((), ()))
        self.assertTrue(all(row.marked_damage == 0 for row in base.permanents))
        for key in ("blocked", "blocking", "pending_blocks", "pending_damage_assignment"):
            self.assertEqual(observation.to_wire()[key], value[key])

    def test_the_pending_answers_have_a_shape(self) -> None:
        value = _example("magic-basic-land-observation-v1.json")
        value["pending_blocks"] = [
            {"blocker": "6", "attacker": "7"},
            {"blocker": "7", "attacker": None},
        ]
        value["pending_damage_assignment"] = [{"attacker": "7", "blocker": "6", "amount": "2"}]
        observation = MagicBasicLandObservationV1.from_wire(value)
        self.assertEqual(
            observation.pending_blocks,
            (
                DeclaredBlockObservationV1(blocker=6, attacker=7),
                DeclaredBlockObservationV1(blocker=7, attacker=None),
            ),
        )
        self.assertEqual(
            observation.pending_damage_assignment,
            (AssignedDamageObservationV1(attacker=7, blocker=6, amount=2),),
        )
        self.assertEqual(observation.to_wire()["pending_blocks"], value["pending_blocks"])
        self.assertEqual(
            observation.to_wire()["pending_damage_assignment"], value["pending_damage_assignment"]
        )
        for key, replacement in (
            ("pending_damage_assignment", [{"attacker": "7", "blocker": "6", "amount": 2}]),
            ("pending_damage_assignment", [{"attacker": "7", "blocker": "6", "amount": "02"}]),
            ("pending_damage_assignment", [{"attacker": "7", "blocker": "6"}]),
            ("pending_blocks", [{"blocker": "6"}]),
            ("pending_blocks", [{"blocker": "6", "attacker": "7", "extra": 0}]),
            ("pending_blocks", {}),
        ):
            broken = copy.deepcopy(value)
            broken[key] = replacement
            with self.assertRaises(WireError):
                MagicBasicLandObservationV1.from_wire(broken)

    def test_malformed_blocks_and_marked_damage_are_rejected(self) -> None:
        value = _example("magic-basic-land-observation-v1.json")

        def field(key: str, replacement: object) -> dict[str, object]:
            broken = copy.deepcopy(value)
            broken[key] = replacement
            return broken

        def attacking_and_blocked(attacking: list[str], blocked: list[str]) -> dict[str, object]:
            broken = field("attacking", attacking)
            broken["blocked"] = blocked
            return broken

        def blocks(first: tuple[str, str], second: tuple[str, str]) -> dict[str, object]:
            return field(
                "blocking",
                [
                    {"blocker": first[0], "attacker": first[1]},
                    {"blocker": second[0], "attacker": second[1]},
                ],
            )

        def entry(key: str, replacement: object, row: int = 0) -> dict[str, object]:
            broken = copy.deepcopy(value)
            rows = broken["permanents"]
            assert isinstance(rows, list)
            rows[row][key] = replacement
            return broken

        def block_entry(key: str, replacement: object) -> dict[str, object]:
            broken = copy.deepcopy(value)
            rows = broken["blocking"]
            assert isinstance(rows, list)
            rows[0][key] = replacement
            return broken

        def without(key: str) -> dict[str, object]:
            broken = copy.deepcopy(value)
            del broken[key]
            return broken

        no_attacker = copy.deepcopy(value)
        rows = no_attacker["blocking"]
        assert isinstance(rows, list)
        del rows[0]["attacker"]
        no_marked_damage = copy.deepcopy(value)
        rows = no_marked_damage["permanents"]
        assert isinstance(rows, list)
        del rows[0]["marked_damage"]
        # Accepted: no block at all, a second blocker, and the largest u64 mark.
        MagicBasicLandObservationV1.from_wire(field("blocking", []))
        MagicBasicLandObservationV1.from_wire(blocks(("6", "7"), ("7", "7")))
        MagicBasicLandObservationV1.from_wire(entry("marked_damage", str(2**64 - 1)))
        canonical = "expected canonical unsigned decimal string"
        ordered = "blocked attackers are not ordered"
        attacking = "a blocked attacker is not attacking"
        rows_order = "block rows are not ordered"
        invalid = "a block does not name an attacking, blocked attacker and a creature"
        marked = "damage is marked on a permanent that is not a creature"
        for broken, reason in (
            (field("blocked", ["7", "7"]), ordered),
            (attacking_and_blocked(["6", "7"], ["7", "6"]), ordered),
            (field("blocked", ["6"]), attacking),
            (field("blocked", ["9"]), attacking),
            (blocks(("7", "7"), ("6", "7")), rows_order),
            (blocks(("6", "7"), ("6", "7")), rows_order),
            (block_entry("attacker", "6"), invalid),
            (block_entry("attacker", "9"), invalid),
            (field("blocked", []), invalid),
            (block_entry("blocker", "8"), invalid),
            (block_entry("blocker", "9"), invalid),
            (entry("marked_damage", "1", 2), marked),
            (entry("marked_damage", 2), canonical),
            (entry("marked_damage", -1), canonical),
            (entry("marked_damage", True), canonical),
            (entry("marked_damage", None), canonical),
            (entry("marked_damage", "02"), canonical),
            (entry("marked_damage", "+2"), canonical),
            (entry("marked_damage", "-2"), canonical),
            (entry("marked_damage", "2a"), canonical),
            (entry("marked_damage", ""), canonical),
            (entry("marked_damage", str(2**64)), "unsigned integer is out of range"),
            (block_entry("extra", 0), "closed contract"),
            (no_attacker, "closed contract"),
            (no_marked_damage, "closed contract"),
            (without("blocked"), "closed contract"),
            (without("blocking"), "closed contract"),
            (without("pending_blocks"), "closed contract"),
            (without("pending_damage_assignment"), "closed contract"),
        ):
            with self.assertRaisesRegex(WireError, reason):
                MagicBasicLandObservationV1.from_wire(broken)


if __name__ == "__main__":
    unittest.main()
