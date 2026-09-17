from __future__ import annotations

from dataclasses import dataclass

from ._generated_contract_vocab import OBSERVED_EVENT_KINDS, ZONE_KINDS
from .canonical import parse_u64_number, parse_uint, require_exact_keys, uint_wire
from .errors import WireError

OBSERVED_EVENT_SCHEMA_V2 = "observed-event-envelope.v2"

_EVENT_V2_REQUIRED_KEYS: dict[str, set[str]] = {
    "object_moved": {"kind", "from", "to"},
    "object_ceased_to_exist": {"kind", "object"},
    "life_changed": {"kind", "player", "from", "to"},
    "object_tapped": {"kind", "object", "tapped"},
    "decision_available": {"kind", "actor"},
    "random_outcome_visible": {"kind", "label", "exclusive_upper_bound", "value"},
    "public_outcome": {"kind", "code"},
}
_EVENT_V2_UINT_KEYS = frozenset({"old_object", "new_object", "object", "player", "actor"})


@dataclass(frozen=True, slots=True)
class ObservedEventV2:
    kind: str
    payload: tuple[tuple[str, object], ...]

    @classmethod
    def from_wire(cls, value: object) -> ObservedEventV2:
        if not isinstance(value, dict) or value.get("kind") not in OBSERVED_EVENT_KINDS:
            raise WireError("decode.invalid_json", "unknown observed event V2 kind")
        kind = str(value["kind"])
        optional = {"old_object", "new_object"} if kind == "object_moved" else set()
        obj = require_exact_keys(value, _EVENT_V2_REQUIRED_KEYS[kind], optional)
        payload: dict[str, object] = {}
        if kind == "object_moved":
            if obj["from"] not in ZONE_KINDS or obj["to"] not in ZONE_KINDS:
                raise WireError("decode.invalid_json", "unknown zone kind")
            payload["from"] = str(obj["from"])
            payload["to"] = str(obj["to"])
            for key in ("old_object", "new_object"):
                payload[key] = None if obj.get(key) is None else parse_uint(obj[key])
            if payload["old_object"] is None and payload["new_object"] is None:
                raise WireError(
                    "semantic.observed_event",
                    "object_moved must reveal at least one identity",
                )
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
            label = obj["label"]
            if not isinstance(label, str):
                raise WireError("decode.invalid_json", "label must be a string")
            upper = parse_u64_number(obj["exclusive_upper_bound"])
            outcome = parse_u64_number(obj["value"])
            if not label or upper == 0 or outcome >= upper:
                raise WireError(
                    "semantic.observed_event",
                    "random outcome is outside its declared range",
                )
            payload.update(label=label, exclusive_upper_bound=upper, value=outcome)
        else:
            code = obj["code"]
            if not isinstance(code, str):
                raise WireError("decode.invalid_json", "code must be a string")
            if not code:
                raise WireError("semantic.observed_event", "observed event code is empty")
            payload["code"] = code
        return cls(kind, tuple(sorted(payload.items())))

    def to_wire(self) -> dict[str, object]:
        result: dict[str, object] = {"kind": self.kind}
        for key, value in self.payload:
            result[key] = (
                uint_wire(value)  # type: ignore[arg-type]  # guaranteed int by wire contract
                if key in _EVENT_V2_UINT_KEYS and value is not None
                else value
            )
        ObservedEventV2.from_wire(result)
        return result


@dataclass(frozen=True, slots=True)
class ObservedEventEnvelopeV2:
    schema_version: str
    sequence: int
    state_revision: int
    event: ObservedEventV2

    @classmethod
    def from_wire(cls, value: object) -> ObservedEventEnvelopeV2:
        obj = require_exact_keys(value, {"schema_version", "sequence", "state_revision", "event"})
        if obj["schema_version"] != OBSERVED_EVENT_SCHEMA_V2:
            raise WireError("decode.invalid_json", "unsupported observed event V2")
        return cls(
            OBSERVED_EVENT_SCHEMA_V2,
            parse_uint(obj["sequence"]),
            parse_uint(obj["state_revision"]),
            ObservedEventV2.from_wire(obj["event"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "event": self.event.to_wire(),
            "schema_version": OBSERVED_EVENT_SCHEMA_V2,
            "sequence": uint_wire(self.sequence),
            "state_revision": uint_wire(self.state_revision),
        }
