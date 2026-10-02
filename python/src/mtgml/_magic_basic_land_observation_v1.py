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


def _i64(value: object, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not -(2**63) <= value <= 2**63 - 1:
        raise WireError("decode.invalid_json", f"{label} is outside its i64 range")
    return value


@dataclass(frozen=True, slots=True)
class PlayerObservationV1:
    """A player's public totals: life, hand size and library size."""

    player: int
    life: int
    hand_count: int
    library_count: int

    @classmethod
    def from_wire(cls, value: object) -> PlayerObservationV1:
        obj = require_exact_keys(value, {"player", "life", "hand_count", "library_count"})
        return cls(
            parse_uint(obj["player"]),
            _i64(obj["life"], "life"),
            _u32(obj["hand_count"], "hand count"),
            _u32(obj["library_count"], "library count"),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "player": uint_wire(self.player),
            "life": _i64(self.life, "life"),
            "hand_count": _u32(self.hand_count, "hand count"),
            "library_count": _u32(self.library_count, "library count"),
        }


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
class PermanentObservationV1:
    """A permanent on the battlefield: who controls it, the turn since which
    that player has controlled it and, for a creature, its power and toughness
    (null for any other permanent). It also shows the damage marked on it, which
    is 0 for a permanent that is not a creature.

    A creature's power and toughness are its printed ones, which are also its
    current ones: no observation is made for a creature that an effect or a
    +1/+1 or -1/-1 counter could change.
    """

    object: int
    controller: int
    controlled_since_turn: int
    power: int | None
    toughness: int | None
    marked_damage: int

    @property
    def is_creature(self) -> bool:
        return self.power is not None

    @classmethod
    def from_wire(cls, value: JsonValue) -> PermanentObservationV1:
        obj = require_exact_keys(
            value,
            {
                "object",
                "controller",
                "controlled_since_turn",
                "power",
                "toughness",
                "marked_damage",
            },
        )
        return cls(
            parse_uint(obj["object"]),
            parse_uint(obj["controller"]),
            parse_uint(obj["controlled_since_turn"]),
            None if obj["power"] is None else _i64(obj["power"], "power"),
            None if obj["toughness"] is None else _i64(obj["toughness"], "toughness"),
            parse_uint(obj["marked_damage"]),
        )

    def to_wire(self) -> dict[str, JsonValue]:
        return {
            "object": uint_wire(self.object),
            "controller": uint_wire(self.controller),
            "controlled_since_turn": uint_wire(self.controlled_since_turn),
            "power": None if self.power is None else _i64(self.power, "power"),
            "toughness": None if self.toughness is None else _i64(self.toughness, "toughness"),
            "marked_damage": uint_wire(self.marked_damage),
        }


@dataclass(frozen=True, slots=True)
class BlockObservationV1:
    """A blocking creature and the attacker it blocks (CR 509.1g); the attacker
    is None when it left combat, and the creature then blocks nothing."""

    blocker: int
    attacker: int | None

    @classmethod
    def from_wire(cls, value: JsonValue) -> BlockObservationV1:
        obj = require_exact_keys(value, {"blocker", "attacker"})
        return cls(
            parse_uint(obj["blocker"]),
            None if obj["attacker"] is None else parse_uint(obj["attacker"]),
        )

    def to_wire(self) -> dict[str, JsonValue]:
        return {
            "blocker": uint_wire(self.blocker),
            "attacker": None if self.attacker is None else uint_wire(self.attacker),
        }


@dataclass(frozen=True, slots=True)
class DeclaredBlockObservationV1:
    """One answer of a block declaration in progress: the creature blocks the
    attacker, or, when the attacker is None, does not block."""

    blocker: int
    attacker: int | None

    @classmethod
    def from_wire(cls, value: JsonValue) -> DeclaredBlockObservationV1:
        obj = require_exact_keys(value, {"blocker", "attacker"})
        return cls(
            parse_uint(obj["blocker"]),
            None if obj["attacker"] is None else parse_uint(obj["attacker"]),
        )

    def to_wire(self) -> dict[str, JsonValue]:
        return {
            "blocker": uint_wire(self.blocker),
            "attacker": None if self.attacker is None else uint_wire(self.attacker),
        }


@dataclass(frozen=True, slots=True)
class AssignedDamageObservationV1:
    """One answer of a combat damage division in progress: how much of the
    attacker's damage the blocker is assigned (CR 510.1c)."""

    attacker: int
    blocker: int
    amount: int

    @classmethod
    def from_wire(cls, value: JsonValue) -> AssignedDamageObservationV1:
        obj = require_exact_keys(value, {"attacker", "blocker", "amount"})
        return cls(
            parse_uint(obj["attacker"]), parse_uint(obj["blocker"]), parse_uint(obj["amount"])
        )

    def to_wire(self) -> dict[str, JsonValue]:
        return {
            "attacker": uint_wire(self.attacker),
            "blocker": uint_wire(self.blocker),
            "amount": uint_wire(self.amount),
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
    players: tuple[PlayerObservationV1, ...]
    pending_sba_ordering: MagicPendingSbaOrdering | None
    pending_blocks: tuple[DeclaredBlockObservationV1, ...] | None
    pending_damage_assignment: tuple[AssignedDamageObservationV1, ...] | None
    mana_pools: tuple[ManaPoolObservationV1, ...]
    counters: tuple[CounterObservationV1, ...]
    attachments: tuple[AttachmentObservationV1, ...]
    faces: tuple[FaceObservationV1, ...]
    tapped: tuple[int, ...]
    permanents: tuple[PermanentObservationV1, ...]
    attacking: tuple[int, ...]
    blocked: tuple[int, ...]
    blocking: tuple[BlockObservationV1, ...]

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
                "players",
                "pending_sba_ordering",
                "pending_blocks",
                "pending_damage_assignment",
                "mana_pools",
                "counters",
                "attachments",
                "faces",
                "tapped",
                "permanents",
                "attacking",
                "blocked",
                "blocking",
            },
        )
        if obj["schema_version"] != MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1:
            raise WireError("decode.invalid_json", "unsupported basic-land observation")
        if not isinstance(obj["turn_number"], str):
            raise WireError("decode.invalid_json", "turn number must be a string")
        for key in (
            "players",
            "mana_pools",
            "counters",
            "attachments",
            "faces",
            "tapped",
            "permanents",
            "attacking",
            "blocked",
            "blocking",
        ):
            if not isinstance(obj[key], list):
                raise WireError("decode.invalid_json", f"{key} must be an array")
        for key in ("pending_blocks", "pending_damage_assignment"):
            if obj[key] is not None and not isinstance(obj[key], list):
                raise WireError("decode.invalid_json", f"{key} must be an array or null")
        pending = obj["pending_sba_ordering"]
        pending_blocks = obj["pending_blocks"]
        pending_damage = obj["pending_damage_assignment"]
        result = cls(
            MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1,
            parse_uint(obj["active_player"]),
            obj["turn_number"],
            SyntheticTurnPosition.from_wire(obj["turn_position"]),
            SyntheticPriority.from_wire(obj["priority"]),
            tuple(PlayerObservationV1.from_wire(item) for item in obj["players"]),
            None if pending is None else MagicPendingSbaOrdering.from_wire(pending),
            None
            if pending_blocks is None
            else tuple(DeclaredBlockObservationV1.from_wire(item) for item in pending_blocks),
            None
            if pending_damage is None
            else tuple(AssignedDamageObservationV1.from_wire(item) for item in pending_damage),
            tuple(ManaPoolObservationV1.from_wire(item) for item in obj["mana_pools"]),
            tuple(CounterObservationV1.from_wire(item) for item in obj["counters"]),
            tuple(AttachmentObservationV1.from_wire(item) for item in obj["attachments"]),
            tuple(FaceObservationV1.from_wire(item) for item in obj["faces"]),
            tuple(parse_uint(item) for item in obj["tapped"]),
            tuple(PermanentObservationV1.from_wire(item) for item in obj["permanents"]),
            tuple(parse_uint(item) for item in obj["attacking"]),
            tuple(parse_uint(item) for item in obj["blocked"]),
            tuple(BlockObservationV1.from_wire(item) for item in obj["blocking"]),
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
        if any(a.player >= b.player for a, b in pairwise(self.players)):
            raise WireError("semantic.magic_basic_land_observation_v1", "players are not ordered")
        if all(item.player != self.active_player for item in self.players):
            raise WireError("semantic.magic_basic_land_observation_v1", "active player not listed")
        if any(a >= b for a, b in pairwise(self.tapped)):
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "tapped permanents are not ordered"
            )
        if any(a.object >= b.object for a, b in pairwise(self.permanents)):
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "permanent rows are not ordered"
            )
        if any((item.power is None) != (item.toughness is None) for item in self.permanents):
            raise WireError(
                "semantic.magic_basic_land_observation_v1",
                "power and toughness are not both present or both null",
            )
        if any(a >= b for a, b in pairwise(self.attacking)):
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "attacking creatures are not ordered"
            )
        creature_ids = {item.object for item in self.permanents if item.is_creature}
        if any(item not in creature_ids for item in self.attacking):
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "an attacker is not a creature"
            )
        if any(a >= b for a, b in pairwise(self.blocked)):
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "blocked attackers are not ordered"
            )
        if any(item not in self.attacking for item in self.blocked):
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "a blocked attacker is not attacking"
            )
        if any(a.blocker >= b.blocker for a, b in pairwise(self.blocking)):
            raise WireError(
                "semantic.magic_basic_land_observation_v1", "block rows are not ordered"
            )
        if any(
            block.blocker not in creature_ids
            or (
                block.attacker is not None
                and (block.attacker not in self.attacking or block.attacker not in self.blocked)
            )
            for block in self.blocking
        ):
            raise WireError(
                "semantic.magic_basic_land_observation_v1",
                "a block does not name an attacking, blocked attacker and a creature",
            )
        if any(item.marked_damage != 0 and not item.is_creature for item in self.permanents):
            raise WireError(
                "semantic.magic_basic_land_observation_v1",
                "damage is marked on a permanent that is not a creature",
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
            "attacking": [uint_wire(item) for item in self.attacking],
            "blocked": [uint_wire(item) for item in self.blocked],
            "blocking": [item.to_wire() for item in self.blocking],
            "counters": [item.to_wire() for item in self.counters],
            "permanents": [item.to_wire() for item in self.permanents],
            "faces": [item.to_wire() for item in self.faces],
            "mana_pools": [item.to_wire() for item in self.mana_pools],
            "pending_blocks": None
            if self.pending_blocks is None
            else [item.to_wire() for item in self.pending_blocks],
            "pending_damage_assignment": None
            if self.pending_damage_assignment is None
            else [item.to_wire() for item in self.pending_damage_assignment],
            "pending_sba_ordering": None
            if self.pending_sba_ordering is None
            else self.pending_sba_ordering.to_wire(),
            "players": [item.to_wire() for item in self.players],
            "priority": self.priority.to_wire(),
            "schema_version": MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1,
            "tapped": [uint_wire(item) for item in self.tapped],
            "turn_number": self.turn_number,
            "turn_position": self.turn_position.to_wire(),
        }
