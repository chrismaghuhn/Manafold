from __future__ import annotations

from dataclasses import dataclass

from ._observation_m3 import SyntheticM3Priority, SyntheticM3TurnPosition
from .canonical import parse_uint, require_exact_keys, uint_wire
from .errors import WireError

MAGIC_M3_OBSERVATION_SCHEMA = "magic-m3-observation.v1"


@dataclass(frozen=True, slots=True)
class MagicM3CompletedOrder:
    owner: int
    ordered_objects: tuple[int, ...]

    @classmethod
    def from_wire(cls, value: object) -> MagicM3CompletedOrder:
        obj = require_exact_keys(value, {"owner", "ordered_objects"})
        raw_objects = obj["ordered_objects"]
        if not isinstance(raw_objects, list):
            raise WireError("decode.invalid_json", "ordered_objects must be an array")
        objects = tuple(parse_uint(item) for item in raw_objects)
        if len(objects) < 2 or len(set(objects)) != len(objects):
            raise WireError("semantic.magic_m3_observation", "invalid completed order")
        return cls(parse_uint(obj["owner"]), objects)

    def to_wire(self) -> dict[str, object]:
        if len(self.ordered_objects) < 2 or len(set(self.ordered_objects)) != len(
            self.ordered_objects
        ):
            raise WireError("semantic.magic_m3_observation", "invalid completed order")
        return {
            "owner": uint_wire(self.owner),
            "ordered_objects": [uint_wire(x) for x in self.ordered_objects],
        }


@dataclass(frozen=True, slots=True)
class MagicM3PendingSbaOrdering:
    completed_orders: tuple[MagicM3CompletedOrder, ...]
    next_order_owner: int

    @classmethod
    def from_wire(cls, value: object) -> MagicM3PendingSbaOrdering:
        obj = require_exact_keys(value, {"completed_orders", "next_order_owner"})
        raw_orders = obj["completed_orders"]
        if not isinstance(raw_orders, list):
            raise WireError("decode.invalid_json", "completed_orders must be an array")
        result = cls(
            tuple(MagicM3CompletedOrder.from_wire(x) for x in raw_orders),
            parse_uint(obj["next_order_owner"]),
        )
        result.to_wire()
        return result

    def to_wire(self) -> dict[str, object]:
        owners = [order.owner for order in self.completed_orders]
        if len(set(owners)) != len(owners) or self.next_order_owner in owners:
            raise WireError("semantic.magic_m3_observation", "invalid APNAP order progress")
        return {
            "completed_orders": [order.to_wire() for order in self.completed_orders],
            "next_order_owner": uint_wire(self.next_order_owner),
        }


@dataclass(frozen=True, slots=True)
class MagicM3Observation:
    schema_version: str
    active_player: int
    turn_number: str
    turn_position: SyntheticM3TurnPosition
    priority: SyntheticM3Priority
    pending_sba_ordering: MagicM3PendingSbaOrdering | None

    @classmethod
    def from_wire(cls, value: object) -> MagicM3Observation:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "active_player",
                "turn_number",
                "turn_position",
                "priority",
                "pending_sba_ordering",
            },
        )
        if obj["schema_version"] != MAGIC_M3_OBSERVATION_SCHEMA:
            raise WireError(
                "semantic.magic_m3_observation", "unsupported Magic M3 observation schema"
            )
        turn_number = obj["turn_number"]
        if not isinstance(turn_number, str):
            raise WireError("decode.invalid_json", "turn number must be a string")
        try:
            parse_uint(turn_number)
        except WireError as exc:
            raise WireError("semantic.magic_m3_observation", exc.message) from exc
        pending_raw = obj["pending_sba_ordering"]
        return cls(
            MAGIC_M3_OBSERVATION_SCHEMA,
            parse_uint(obj["active_player"]),
            turn_number,
            SyntheticM3TurnPosition.from_wire(obj["turn_position"]),
            SyntheticM3Priority.from_wire(obj["priority"]),
            None if pending_raw is None else MagicM3PendingSbaOrdering.from_wire(pending_raw),
        )

    def validate(self) -> None:
        if self.schema_version != MAGIC_M3_OBSERVATION_SCHEMA:
            raise WireError(
                "semantic.magic_m3_observation", "unsupported Magic M3 observation schema"
            )
        uint_wire(self.active_player)
        if not isinstance(self.turn_number, str):
            raise WireError("decode.invalid_json", "turn number must be a string")
        try:
            parse_uint(self.turn_number)
        except WireError as exc:
            raise WireError("semantic.magic_m3_observation", exc.message) from exc
        self.turn_position.to_wire()
        self.priority.to_wire()
        if self.pending_sba_ordering is not None:
            self.pending_sba_ordering.to_wire()

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "active_player": uint_wire(self.active_player),
            "pending_sba_ordering": None
            if self.pending_sba_ordering is None
            else self.pending_sba_ordering.to_wire(),
            "priority": self.priority.to_wire(),
            "schema_version": MAGIC_M3_OBSERVATION_SCHEMA,
            "turn_number": self.turn_number,
            "turn_position": self.turn_position.to_wire(),
        }
