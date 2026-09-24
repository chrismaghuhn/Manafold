from __future__ import annotations

from dataclasses import dataclass

from .canonical import parse_uint, require_exact_keys, uint_wire
from .errors import WireError

SYNTHETIC_OBSERVATION_SCHEMA_V1 = "synthetic-m3-observation.v1"

SYNTHETIC_BEGINNING_STEPS = frozenset({"untap", "upkeep", "draw"})
SYNTHETIC_COMBAT_STEPS = frozenset(
    {
        "beginning_of_combat",
        "declare_attackers",
        "declare_blockers",
        "combat_damage",
        "end_of_combat",
    }
)
SYNTHETIC_ENDING_STEPS = frozenset({"end_step", "cleanup"})
SYNTHETIC_TURN_KINDS = frozenset(
    {"beginning", "precombat_main", "combat", "postcombat_main", "ending"}
)
SYNTHETIC_PRIORITY_KINDS = frozenset({"none", "held_by"})


@dataclass(frozen=True, slots=True)
class SyntheticTurnPosition:
    kind: str
    step: str | None = None

    @classmethod
    def from_wire(cls, value: object) -> SyntheticTurnPosition:
        if not isinstance(value, dict):
            raise WireError("decode.invalid_json", "turn position must be an object")
        kind = value.get("kind")
        if kind not in SYNTHETIC_TURN_KINDS:
            raise WireError("decode.invalid_json", "unknown turn-position kind")
        kind_str = str(kind)
        if kind_str == "beginning":
            obj = require_exact_keys(value, {"kind", "step"})
            step = obj["step"]
            if step not in SYNTHETIC_BEGINNING_STEPS:
                raise WireError("decode.invalid_json", "unknown beginning step")
            return cls("beginning", str(step))
        if kind_str == "combat":
            obj = require_exact_keys(value, {"kind", "step"})
            step = obj["step"]
            if step not in SYNTHETIC_COMBAT_STEPS:
                raise WireError("decode.invalid_json", "unknown combat step")
            return cls("combat", str(step))
        if kind_str == "ending":
            obj = require_exact_keys(value, {"kind", "step"})
            step = obj["step"]
            if step not in SYNTHETIC_ENDING_STEPS:
                raise WireError("decode.invalid_json", "unknown ending step")
            return cls("ending", str(step))
        # precombat_main / postcombat_main carry no step field.
        require_exact_keys(value, {"kind"})
        return cls(kind_str, None)

    def to_wire(self) -> dict[str, object]:
        # Reuse the reader as exact structural validation.
        if self.kind == "beginning":
            if self.step not in SYNTHETIC_BEGINNING_STEPS:
                raise WireError("decode.invalid_json", "unknown beginning step")
            result: dict[str, object] = {"kind": "beginning", "step": self.step}
        elif self.kind == "combat":
            if self.step not in SYNTHETIC_COMBAT_STEPS:
                raise WireError("decode.invalid_json", "unknown combat step")
            result = {"kind": "combat", "step": self.step}
        elif self.kind == "ending":
            if self.step not in SYNTHETIC_ENDING_STEPS:
                raise WireError("decode.invalid_json", "unknown ending step")
            result = {"kind": "ending", "step": self.step}
        elif self.kind in {"precombat_main", "postcombat_main"}:
            if self.step is not None:
                raise WireError("decode.invalid_json", "main phase must not carry a step")
            result = {"kind": self.kind}
        else:
            raise WireError("decode.invalid_json", "unknown turn-position kind")
        SyntheticTurnPosition.from_wire(result)
        return result


@dataclass(frozen=True, slots=True)
class SyntheticPriority:
    kind: str
    player: int | None = None

    @classmethod
    def from_wire(cls, value: object) -> SyntheticPriority:
        if not isinstance(value, dict):
            raise WireError("decode.invalid_json", "priority must be an object")
        kind = value.get("kind")
        if kind not in SYNTHETIC_PRIORITY_KINDS:
            raise WireError("decode.invalid_json", "unknown priority kind")
        if kind == "none":
            require_exact_keys(value, {"kind"})
            return cls("none", None)
        obj = require_exact_keys(value, {"kind", "player"})
        return cls("held_by", parse_uint(obj["player"]))

    def to_wire(self) -> dict[str, object]:
        if self.kind == "none":
            if self.player is not None:
                raise WireError("decode.invalid_json", "none priority carries no player")
            result: dict[str, object] = {"kind": "none"}
        elif self.kind == "held_by":
            if self.player is None:
                raise WireError("decode.invalid_json", "held_by priority needs a player")
            result = {"kind": "held_by", "player": uint_wire(self.player)}
        else:
            raise WireError("decode.invalid_json", "unknown priority kind")
        SyntheticPriority.from_wire(result)
        return result


@dataclass(frozen=True, slots=True)
class SyntheticObservation:
    schema_version: str
    active_player: int
    turn_number: str
    turn_position: SyntheticTurnPosition
    priority: SyntheticPriority

    @classmethod
    def from_wire(cls, value: object) -> SyntheticObservation:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "active_player",
                "turn_number",
                "turn_position",
                "priority",
            },
        )
        schema_raw = obj["schema_version"]
        if not isinstance(schema_raw, str):
            raise WireError("decode.invalid_json", "unsupported synthetic observation schema")
        if schema_raw != SYNTHETIC_OBSERVATION_SCHEMA_V1:
            raise WireError(
                "semantic.synthetic_m3_observation", "unsupported synthetic observation schema"
            )
        active_player = parse_uint(obj["active_player"])
        turn_number_raw = obj["turn_number"]
        # turn_number is a canonical u64 decimal string on the wire. A
        # non-string is a shape failure; a string that is not canonical
        # u64 decimal is a synthetic observation identity failure.
        if not isinstance(turn_number_raw, str):
            raise WireError("decode.invalid_json", "turn number must be a string")
        try:
            parse_uint(turn_number_raw)
        except WireError as exc:
            raise WireError("semantic.synthetic_m3_observation", exc.message) from exc
        result = cls(
            SYNTHETIC_OBSERVATION_SCHEMA_V1,
            active_player,
            str(turn_number_raw),
            SyntheticTurnPosition.from_wire(obj["turn_position"]),
            SyntheticPriority.from_wire(obj["priority"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if not isinstance(self.schema_version, str):
            raise WireError("decode.invalid_json", "unsupported synthetic observation schema")
        if self.schema_version != SYNTHETIC_OBSERVATION_SCHEMA_V1:
            raise WireError(
                "semantic.synthetic_m3_observation", "unsupported synthetic observation schema"
            )
        uint_wire(self.active_player)
        if not isinstance(self.turn_number, str):
            raise WireError("decode.invalid_json", "turn number must be a string")
        try:
            parse_uint(self.turn_number)
        except WireError as exc:
            raise WireError("semantic.synthetic_m3_observation", exc.message) from exc
        self.turn_position.to_wire()
        self.priority.to_wire()

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "active_player": uint_wire(self.active_player),
            "priority": self.priority.to_wire(),
            "schema_version": SYNTHETIC_OBSERVATION_SCHEMA_V1,
            "turn_number": str(self.turn_number),
            "turn_position": self.turn_position.to_wire(),
        }
