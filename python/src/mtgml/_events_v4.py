"""Rules-free decoder for detached G0 observed-event V4 products."""

from __future__ import annotations

from dataclasses import dataclass
from itertools import pairwise

from ._generated_contract_vocab import ZONE_KINDS
from .canonical import parse_u64_number, parse_uint, require_exact_keys, uint_wire
from .errors import WireError
from .magic_shared_execution_observation_v1 import (
    PublicTemporaryEffectV1,
    public_stack_item_from_wire,
)

OBSERVED_EVENT_SCHEMA_V4 = "observed-event-envelope.v4"
EVENT_KINDS_V3 = frozenset(
    {
        "object_moved",
        "object_ceased_to_exist",
        "life_changed",
        "object_tapped",
        "decision_available",
        "random_outcome_visible",
        "public_outcome",
        "mana_pool_changed",
        "counters_changed",
        "attachment_changed",
        "object_face_changed",
    }
)


COUNTER_KINDS_V3 = ("plus_one_plus_one", "minus_one_minus_one", "lore")


FACE_VALUES_V1 = ("front", "back")


def _u32(value: object, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**32 - 1:
        raise WireError("decode.invalid_json", f"{label} must be u32")
    return value


def _pool(value: object) -> dict[str, list[int]]:
    obj = require_exact_keys(value, {"unrestricted", "creature_spell_only"})
    result: dict[str, list[int]] = {}
    for key in ("unrestricted", "creature_spell_only"):
        bucket = obj[key]
        if not isinstance(bucket, list) or len(bucket) != 6:
            raise WireError("decode.invalid_json", "mana bucket must have six entries")
        result[key] = [_u32(item, "mana count") for item in bucket]
    return result


NEW_EVENT_KINDS_V1 = frozenset(
    {
        "stack_item_added",
        "stack_item_removed",
        "temporary_effect_created",
        "temporary_effect_expired",
    }
)
GAME_START_EVENT_KINDS = frozenset({"starting_player_chosen", "mulligan_declared"})
COMBAT_EVENT_KINDS = frozenset({"attackers_declared"})
EVENT_KINDS_V4 = EVENT_KINDS_V3 | NEW_EVENT_KINDS_V1 | GAME_START_EVENT_KINDS | COMBAT_EVENT_KINDS
_UINT_FIELDS = frozenset(
    {
        "object",
        "chooser",
        "starting_player",
        "attacking_player",
        "defending_player",
        "old_object",
        "new_object",
        "player",
        "actor",
        "source",
        "old_target",
        "new_target",
    }
)


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
        fields: dict[str, object]
        if kind == "object_moved":
            obj = require_exact_keys(
                value,
                {"kind", "old_object", "new_object", "from", "to", "entering_face", "tapped"},
            )
            if obj["from"] not in ZONE_KINDS or obj["to"] not in ZONE_KINDS:
                raise WireError("decode.invalid_json", "unknown zone kind")
            old = None if obj["old_object"] is None else parse_uint(obj["old_object"])
            new = None if obj["new_object"] is None else parse_uint(obj["new_object"])
            if old is None and new is None:
                raise WireError("semantic.observed_event", "object_moved needs a visible identity")
            face = obj["entering_face"]
            if face is not None and face not in FACE_VALUES_V1:
                raise WireError("decode.invalid_json", "unknown entering face")
            tapped = obj["tapped"]
            if tapped is not None and not isinstance(tapped, bool):
                raise WireError("decode.invalid_json", "tapped must be boolean or null")
            if (face is not None or tapped is not None) and (
                obj["to"] != "battlefield" or new is None
            ):
                raise WireError(
                    "semantic.observed_event", "entry facts require a visible battlefield entrant"
                )
            fields = {
                "old_object": old,
                "new_object": new,
                "from": obj["from"],
                "to": obj["to"],
                "entering_face": face,
                "tapped": tapped,
            }
        elif kind == "object_ceased_to_exist":
            obj = require_exact_keys(value, {"kind", "object"})
            fields = {"object": parse_uint(obj["object"])}
        elif kind == "life_changed":
            obj = require_exact_keys(value, {"kind", "player", "from", "to"})
            fields = {"player": parse_uint(obj["player"])}
            for key in ("from", "to"):
                raw = obj[key]
                if isinstance(raw, bool) or not isinstance(raw, int) or not -(2**63) <= raw < 2**63:
                    raise WireError("decode.invalid_json", "life value is outside i64")
                fields[key] = raw
        elif kind == "object_tapped":
            obj = require_exact_keys(value, {"kind", "object", "tapped"})
            if not isinstance(obj["tapped"], bool):
                raise WireError("decode.invalid_json", "tapped must be boolean")
            fields = {"object": parse_uint(obj["object"]), "tapped": obj["tapped"]}
        elif kind == "decision_available":
            obj = require_exact_keys(value, {"kind", "actor"})
            fields = {"actor": parse_uint(obj["actor"])}
        elif kind == "random_outcome_visible":
            obj = require_exact_keys(value, {"kind", "label", "exclusive_upper_bound", "value"})
            label = obj["label"]
            upper = parse_u64_number(obj["exclusive_upper_bound"])
            outcome = parse_u64_number(obj["value"])
            if not isinstance(label, str) or not label or upper == 0 or outcome >= upper:
                raise WireError("semantic.observed_event", "invalid random outcome")
            fields = {"label": label, "exclusive_upper_bound": upper, "value": outcome}
        elif kind == "public_outcome":
            obj = require_exact_keys(value, {"kind", "code"})
            if not isinstance(obj["code"], str) or not obj["code"]:
                raise WireError("semantic.observed_event", "empty public outcome")
            fields = {"code": obj["code"]}
        elif kind == "counters_changed":
            obj = require_exact_keys(value, {"kind", "object", "counter_kind", "from", "to"})
            if obj["counter_kind"] not in COUNTER_KINDS_V3:
                raise WireError("decode.invalid_json", "unknown counter kind")
            before, after = _u32(obj["from"], "counter before"), _u32(obj["to"], "counter after")
            if before == after:
                raise WireError("semantic.observed_event", "unchanged counter has no event")
            fields = {
                "object": parse_uint(obj["object"]),
                "counter_kind": obj["counter_kind"],
                "from": before,
                "to": after,
            }
        elif kind == "attachment_changed":
            obj = require_exact_keys(value, {"kind", "source", "old_target", "new_target"})
            old = None if obj["old_target"] is None else parse_uint(obj["old_target"])
            new = None if obj["new_target"] is None else parse_uint(obj["new_target"])
            if old == new:
                raise WireError("semantic.observed_event", "unchanged attachment has no event")
            fields = {"source": parse_uint(obj["source"]), "old_target": old, "new_target": new}
        elif kind == "object_face_changed":
            obj = require_exact_keys(value, {"kind", "object", "face"})
            if obj["face"] not in FACE_VALUES_V1:
                raise WireError("decode.invalid_json", "unknown face")
            fields = {"object": parse_uint(obj["object"]), "face": obj["face"]}
        elif kind == "mana_pool_changed":
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
                not isinstance(obj["cause"], str) or obj["cause"] not in {"resolved", "countered"}
            ):
                raise WireError("decode.invalid_json", "unknown stack removal cause")
            fields = {
                "stack_position_from_top": _u32(obj["stack_position_from_top"], "stack position"),
                "item": item_wire,
            }
            if kind == "stack_item_removed":
                fields["cause"] = obj["cause"]
        elif kind == "starting_player_chosen":
            obj = require_exact_keys(value, {"kind", "chooser", "starting_player"})
            fields = {
                "chooser": parse_uint(obj["chooser"]),
                "starting_player": parse_uint(obj["starting_player"]),
            }
        elif kind == "mulligan_declared":
            obj = require_exact_keys(value, {"kind", "player", "mulligan"})
            if not isinstance(obj["mulligan"], bool):
                raise WireError("decode.invalid_json", "mulligan must be boolean")
            fields = {"player": parse_uint(obj["player"]), "mulligan": obj["mulligan"]}
        elif kind == "attackers_declared":
            obj = require_exact_keys(
                value, {"kind", "attacking_player", "defending_player", "attackers"}
            )
            if not isinstance(obj["attackers"], list):
                raise WireError("decode.invalid_json", "attackers must be a list")
            attackers = tuple(parse_uint(item) for item in obj["attackers"])
            attacking, defending = (
                parse_uint(obj["attacking_player"]),
                parse_uint(obj["defending_player"]),
            )
            if attacking == defending or any(
                first >= second for first, second in pairwise(attackers)
            ):
                raise WireError(
                    "semantic.observed_event",
                    "attackers are ascending and the attacker is not the defender",
                )
            fields = {
                "attacking_player": attacking,
                "defending_player": defending,
                "attackers": attackers,
            }
        elif kind in {"temporary_effect_created", "temporary_effect_expired"}:
            obj = require_exact_keys(value, {"kind", "effect"})
            fields = {"effect": PublicTemporaryEffectV1.from_wire(obj["effect"]).to_wire()}
        else:
            raise AssertionError("event vocabulary is exhaustive")
        return cls(str(kind), tuple(sorted(fields.items())))

    def to_wire(self) -> dict[str, object]:
        result: dict[str, object] = {"kind": self.kind}
        for key, value in self.fields:
            if key == "attackers":
                result[key] = [uint_wire(item) for item in value]  # type: ignore[attr-defined]
            elif key in _UINT_FIELDS:
                result[key] = None if value is None else uint_wire(value)  # type: ignore[arg-type]
            else:
                result[key] = value
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
