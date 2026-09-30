from __future__ import annotations

from dataclasses import dataclass

from .canonical import parse_uint, require_exact_keys, uint_wire
from .errors import WireError


@dataclass(frozen=True, slots=True)
class MagicCompletedOrder:
    owner: int
    ordered_objects: tuple[int, ...]

    @classmethod
    def from_wire(cls, value: object) -> MagicCompletedOrder:
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
class MagicPendingSbaOrdering:
    completed_orders: tuple[MagicCompletedOrder, ...]
    next_order_owner: int

    @classmethod
    def from_wire(cls, value: object) -> MagicPendingSbaOrdering:
        obj = require_exact_keys(value, {"completed_orders", "next_order_owner"})
        raw_orders = obj["completed_orders"]
        if not isinstance(raw_orders, list):
            raise WireError("decode.invalid_json", "completed_orders must be an array")
        result = cls(
            tuple(MagicCompletedOrder.from_wire(x) for x in raw_orders),
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
