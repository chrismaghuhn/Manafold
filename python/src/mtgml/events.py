from __future__ import annotations

from dataclasses import dataclass

from ._generated_contract_vocab import (
    OBSERVED_EVENT_KINDS,
    OBSERVED_EVENT_SCHEMA,
    ZONE_KINDS,
)
from .canonical import parse_uint, require_exact_keys, require_nonempty, uint_wire
from .errors import WireError

_EVENT_KINDS = OBSERVED_EVENT_KINDS
_ZONE_KINDS = ZONE_KINDS


@dataclass(frozen=True, slots=True)
class ObservedEvent:
    kind: str
    payload: tuple[tuple[str, object], ...]

    @classmethod
    def from_wire(cls, value: object) -> ObservedEvent:
        if not isinstance(value, dict) or value.get("kind") not in _EVENT_KINDS:
            raise WireError("decode.invalid_json", "unknown observed event kind")
        kind = str(value["kind"])
        required: dict[str, set[str]] = {
            "object_moved": {"kind", "from", "to"},
            "object_ceased_to_exist": {"kind", "object"},
            "life_changed": {"kind", "player", "from", "to"},
            "object_tapped": {"kind", "object", "tapped"},
            "decision_available": {"kind", "actor"},
            "random_outcome_visible": {
                "kind",
                "label",
                "exclusive_upper_bound",
                "value",
            },
            "public_outcome": {"kind", "code"},
        }
        optional = {"old_object", "new_object"} if kind == "object_moved" else set()
        obj = require_exact_keys(value, required[kind], optional)
        payload: dict[str, object] = {}
        if kind == "object_moved":
            if obj["from"] not in _ZONE_KINDS or obj["to"] not in _ZONE_KINDS:
                raise WireError("decode.invalid_json", "unknown zone kind")
            payload["from"] = str(obj["from"])
            payload["to"] = str(obj["to"])
            for key in ("old_object", "new_object"):
                if key in obj:
                    payload[key] = parse_uint(obj[key])
        elif kind == "object_ceased_to_exist":
            payload["object"] = parse_uint(obj["object"])
        elif kind == "life_changed":
            payload["player"] = parse_uint(obj["player"])
            for key in ("from", "to"):
                raw = obj[key]
                if isinstance(raw, bool) or not isinstance(raw, int) or not -(2**63) <= raw < 2**63:
                    raise WireError("decode.invalid_json", "life value is outside i64")
                payload[key] = raw
        elif kind == "object_tapped":
            payload["object"] = parse_uint(obj["object"])
            if not isinstance(obj["tapped"], bool):
                raise WireError("decode.invalid_json", "tapped must be boolean")
            payload["tapped"] = obj["tapped"]
        elif kind == "decision_available":
            payload["actor"] = parse_uint(obj["actor"])
        elif kind == "random_outcome_visible":
            label = require_nonempty(obj["label"], "label")
            upper, result = obj["exclusive_upper_bound"], obj["value"]
            if (
                isinstance(upper, bool)
                or isinstance(result, bool)
                or not isinstance(upper, int)
                or not isinstance(result, int)
            ):
                raise WireError("decode.invalid_json", "random outcome bounds must be integers")
            if upper <= 0 or upper > 2**64 - 1 or result < 0 or result >= upper:
                raise WireError("semantic.observed_event", "random outcome is outside its range")
            payload.update(label=label, exclusive_upper_bound=upper, value=result)
        else:
            payload["code"] = require_nonempty(obj["code"], "code")
        return cls(kind, tuple(sorted(payload.items())))

    def to_wire(self) -> dict[str, object]:
        result: dict[str, object] = {"kind": self.kind}
        for key, value in self.payload:
            result[key] = (
                uint_wire(value)
                if key in {"old_object", "new_object", "object", "player", "actor"}
                else value
            )
        ObservedEvent.from_wire(result)
        return result


@dataclass(frozen=True, slots=True)
class ObservedEventEnvelope:
    schema_version: str
    sequence: int
    state_revision: int
    event: ObservedEvent

    @classmethod
    def from_wire(cls, value: object) -> ObservedEventEnvelope:
        obj = require_exact_keys(value, {"schema_version", "sequence", "state_revision", "event"})
        if obj["schema_version"] != OBSERVED_EVENT_SCHEMA:
            raise WireError("decode.invalid_json", "unsupported observed-event schema")
        return cls(
            OBSERVED_EVENT_SCHEMA,
            parse_uint(obj["sequence"]),
            parse_uint(obj["state_revision"]),
            ObservedEvent.from_wire(obj["event"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "event": self.event.to_wire(),
            "schema_version": self.schema_version,
            "sequence": uint_wire(self.sequence),
            "state_revision": uint_wire(self.state_revision),
        }
