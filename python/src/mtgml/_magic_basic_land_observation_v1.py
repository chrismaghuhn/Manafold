from __future__ import annotations

from dataclasses import dataclass
from itertools import pairwise

from ._magic_observation import MagicPendingSbaOrdering
from ._synthetic_observation import SyntheticPriority, SyntheticTurnPosition
from .canonical import parse_uint, require_exact_keys, uint_wire
from .errors import WireError

JsonValue = object

MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1 = "magic-basic-land-observation.v1"
COUNTER_KINDS = ("plus_one_plus_one", "minus_one_minus_one", "lore")
FACE_VALUES = ("front", "back")


def _u32(value: object, label: str, *, minimum: int = 0) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not minimum <= value <= 2**32 - 1:
        raise WireError("decode.invalid_json", f"{label} is outside its u32 range")
    return value


@dataclass(frozen=True, slots=True)
class ManaPoolObservationV1:
    player: int
    unrestricted: tuple[int, int, int, int, int, int]
    creature_spell_only: tuple[int, int, int, int, int, int]

    @classmethod
    def from_wire(cls, value: object) -> ManaPoolObservationV1:
        obj = require_exact_keys(value, {"player", "unrestricted", "creature_spell_only"})
        buckets: list[tuple[int, int, int, int, int, int]] = []
        for name in ("unrestricted", "creature_spell_only"):
            raw = obj[name]
            if not isinstance(raw, list) or len(raw) != 6:
                raise WireError("decode.invalid_json", "mana vector must contain six values")
            buckets.append(tuple(_u32(item, "mana count") for item in raw))  # type: ignore[arg-type]
        return cls(parse_uint(obj["player"]), buckets[0], buckets[1])

    def to_wire(self) -> dict[str, object]:
        if len(self.unrestricted) != 6 or len(self.creature_spell_only) != 6:
            raise WireError("encode.serialization", "mana vectors must contain six values")
        return {
            "player": uint_wire(self.player),
            "unrestricted": [_u32(item, "mana count") for item in self.unrestricted],
            "creature_spell_only": [_u32(item, "mana count") for item in self.creature_spell_only],
        }


@dataclass(frozen=True, slots=True)
class CounterObservationV1:
    object: int
    counter_kind: str
    count: int

    @classmethod
    def from_wire(cls, value: JsonValue) -> CounterObservationV1:
        obj = require_exact_keys(value, {"object", "counter_kind", "count"})
        if obj["counter_kind"] not in COUNTER_KINDS:
            raise WireError("decode.invalid_json", "unknown public counter kind")
        count = _u32(obj["count"], "counter count", minimum=1)
        return cls(parse_uint(obj["object"]), str(obj["counter_kind"]), count)

    def to_wire(self) -> dict[str, JsonValue]:
        if self.counter_kind not in COUNTER_KINDS:
            raise WireError("encode.serialization", "unknown public counter kind")
        return {
            "object": uint_wire(self.object),
            "counter_kind": self.counter_kind,
            "count": _u32(self.count, "counter count", minimum=1),
        }


@dataclass(frozen=True, slots=True)
class AttachmentObservationV1:
    source: int
    target: int

    @classmethod
    def from_wire(cls, value: object) -> AttachmentObservationV1:
        obj = require_exact_keys(value, {"source", "target"})
        return cls(parse_uint(obj["source"]), parse_uint(obj["target"]))

    def to_wire(self) -> dict[str, object]:
        return {"source": uint_wire(self.source), "target": uint_wire(self.target)}


@dataclass(frozen=True, slots=True)
class FaceObservationV1:
    object: int
    face: str

    @classmethod
    def from_wire(cls, value: JsonValue) -> FaceObservationV1:
        obj = require_exact_keys(value, {"object", "face"})
        if obj["face"] not in FACE_VALUES:
            raise WireError("decode.invalid_json", "unknown public face")
        return cls(parse_uint(obj["object"]), str(obj["face"]))

    def to_wire(self) -> dict[str, JsonValue]:
        if self.face not in FACE_VALUES:
            raise WireError("encode.serialization", "unknown public face")
        return {"object": uint_wire(self.object), "face": self.face}


@dataclass(frozen=True, slots=True)
class MagicBasicLandObservationV1:
    schema_version: str
    active_player: int
    turn_number: str
    turn_position: SyntheticTurnPosition
    priority: SyntheticPriority
    pending_sba_ordering: MagicPendingSbaOrdering | None
    mana_pools: tuple[ManaPoolObservationV1, ...]
    counters: tuple[CounterObservationV1, ...]
    attachments: tuple[AttachmentObservationV1, ...]
    faces: tuple[FaceObservationV1, ...]

    @classmethod
    def from_wire(cls, value: object) -> MagicBasicLandObservationV1:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "active_player",
                "turn_number",
                "turn_position",
                "priority",
                "pending_sba_ordering",
                "mana_pools",
                "counters",
                "attachments",
                "faces",
            },
        )
        if obj["schema_version"] != MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1:
            raise WireError("decode.invalid_json", "unsupported basic-land observation")
        if not isinstance(obj["turn_number"], str):
            raise WireError("decode.invalid_json", "turn number must be a string")
        for key in ("mana_pools", "counters", "attachments", "faces"):
            if not isinstance(obj[key], list):
                raise WireError("decode.invalid_json", f"{key} must be an array")
        pending = obj["pending_sba_ordering"]
        result = cls(
            MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1,
            parse_uint(obj["active_player"]),
            obj["turn_number"],
            SyntheticTurnPosition.from_wire(obj["turn_position"]),
            SyntheticPriority.from_wire(obj["priority"]),
            None if pending is None else MagicPendingSbaOrdering.from_wire(pending),
            tuple(ManaPoolObservationV1.from_wire(item) for item in obj["mana_pools"]),
            tuple(CounterObservationV1.from_wire(item) for item in obj["counters"]),
            tuple(AttachmentObservationV1.from_wire(item) for item in obj["attachments"]),
            tuple(FaceObservationV1.from_wire(item) for item in obj["faces"]),
        )
        result.to_wire()
        return result

    def to_wire(self) -> dict[str, object]:
        from .canonical import parse_uint

        try:
            parse_uint(self.turn_number)
        except WireError as exc:
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "invalid turn number"
            ) from exc
        if self.schema_version != MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1:
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "unsupported payload schema"
            )
        if any(a.player >= b.player for a, b in pairwise(self.mana_pools)):
            raise WireError("semantic.magic_basic_land_observation_v1", "mana rows are not ordered")
        counter_keys = tuple(
            (
                item.object,
                COUNTER_KINDS.index(item.counter_kind)
                if item.counter_kind in COUNTER_KINDS
                else -1,
            )
            for item in self.counters
        )
        if any(a >= b for a, b in pairwise(counter_keys)):
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "counter rows are not ordered"
            )
        if any(a.source >= b.source for a, b in pairwise(self.attachments)):
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "attachment rows are not ordered"
            )
        if any(a.object >= b.object for a, b in pairwise(self.faces)):
            raise WireError("semantic.magic_basic_land_observation_v1", "face rows are not ordered")
        return {
            "active_player": uint_wire(self.active_player),
            "attachments": [item.to_wire() for item in self.attachments],
            "counters": [item.to_wire() for item in self.counters],
            "faces": [item.to_wire() for item in self.faces],
            "mana_pools": [item.to_wire() for item in self.mana_pools],
            "pending_sba_ordering": None
            if self.pending_sba_ordering is None
            else self.pending_sba_ordering.to_wire(),
            "priority": self.priority.to_wire(),
            "schema_version": MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1,
            "turn_number": self.turn_number,
            "turn_position": self.turn_position.to_wire(),
        }
