"""Rules-free decoder for detached G0 observed-event V4 products."""

from __future__ import annotations

from dataclasses import dataclass

from ._events_v3 import EVENT_KINDS_V3, ObservedEventV3, _pool, _u32
from .canonical import parse_uint, require_exact_keys, uint_wire
from .errors import WireError
from .magic_shared_execution_observation_v1 import (
    PublicTemporaryEffectV1,
    public_stack_item_from_wire,
)

OBSERVED_EVENT_SCHEMA_V4 = "observed-event-envelope.v4"
NEW_EVENT_KINDS_V1 = frozenset(
    {
        "stack_item_added",
        "stack_item_removed",
        "temporary_effect_created",
        "temporary_effect_expired",
    }
)
EVENT_KINDS_V4 = EVENT_KINDS_V3 | NEW_EVENT_KINDS_V1


@dataclass(frozen=True, slots=True)
class ObservedEventV4:
    kind: str
    fields: tuple[tuple[str, object], ...]

    @classmethod
    def from_wire(cls, value: object) -> ObservedEventV4:
        if (
            not isinstance(value, dict)
            or not isinstance(value.get("kind"), str)
            or value["kind"] not in EVENT_KINDS_V4
        ):
            raise WireError("decode.invalid_json", "unknown observed event V4 kind")
        kind = value["kind"]
        if kind in EVENT_KINDS_V3 and kind != "mana_pool_changed":
            old = ObservedEventV3.from_wire(value)
            return cls(old.kind, old.fields)
        if kind == "mana_pool_changed":
            obj = require_exact_keys(value, {"kind", "player", "pool_after", "cause"})
            if not isinstance(obj["cause"], str) or obj["cause"] not in {
                "produced",
                "emptied",
                "spent",
            }:
                raise WireError("decode.invalid_json", "unknown mana change cause")
            pool = _pool(obj["pool_after"])
            if obj["cause"] == "emptied" and any(pool[b][i] for b in pool for i in range(6)):
                raise WireError("semantic.observed_event", "emptied mana pool must be zero")
            fields = {
                "player": parse_uint(obj["player"]),
                "pool_after": pool,
                "cause": obj["cause"],
            }
        elif kind in {"stack_item_added", "stack_item_removed"}:
            required = {"kind", "stack_position_from_top", "item"}
            if kind == "stack_item_removed":
                required.add("cause")
            obj = require_exact_keys(value, required)
            item = public_stack_item_from_wire(obj["item"])
            item_wire = item.to_wire()
            if kind == "stack_item_removed" and (
                not isinstance(obj["cause"], str)
                or obj["cause"] not in {"resolved", "countered"}
            ):
                raise WireError("decode.invalid_json", "unknown stack removal cause")
            fields = {
                "stack_position_from_top": _u32(
                    obj["stack_position_from_top"], "stack position"
                ),
                "item": item_wire,
            }
            if kind == "stack_item_removed":
                fields["cause"] = obj["cause"]
        elif kind in {"temporary_effect_created", "temporary_effect_expired"}:
            obj = require_exact_keys(value, {"kind", "effect"})
            fields = {"effect": PublicTemporaryEffectV1.from_wire(obj["effect"]).to_wire()}
        else:
            raise AssertionError("event vocabulary is exhaustive")
        return cls(str(kind), tuple(sorted(fields.items())))

    def to_wire(self) -> dict[str, object]:
        result: dict[str, object] = {"kind": self.kind}
        result.update(self.fields)
        if "player" in result:
            result["player"] = uint_wire(result["player"])  # type: ignore[arg-type]
        if self.kind in EVENT_KINDS_V3 and self.kind != "mana_pool_changed":
            return ObservedEventV3(self.kind, self.fields).to_wire()
        ObservedEventV4.from_wire(result)
        return result


@dataclass(frozen=True, slots=True)
class ObservedEventEnvelopeV4:
    schema_version: str
    sequence: int
    event: ObservedEventV4

    @classmethod
    def from_wire(cls, value: object) -> ObservedEventEnvelopeV4:
        obj = require_exact_keys(value, {"schema_version", "sequence", "event"})
        if obj["schema_version"] != OBSERVED_EVENT_SCHEMA_V4:
            raise WireError("decode.invalid_json", "unsupported observed event V4")
        return cls(
            OBSERVED_EVENT_SCHEMA_V4,
            parse_uint(obj["sequence"]),
            ObservedEventV4.from_wire(obj["event"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "event": self.event.to_wire(),
            "schema_version": OBSERVED_EVENT_SCHEMA_V4,
            "sequence": uint_wire(self.sequence),
        }
