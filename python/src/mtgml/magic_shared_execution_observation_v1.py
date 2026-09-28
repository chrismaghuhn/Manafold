"""Rules-free public view for the bounded shared-execution Magic payload."""

from __future__ import annotations

from dataclasses import dataclass
from itertools import pairwise

from ._magic_basic_land_observation_v1 import (
    MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1,
    MagicBasicLandObservationV1,
)
from .canonical import parse_uint, require_exact_keys, uint_wire
from .decision_v4 import CostFactsV1, SafeTargetDescriptorV1
from .errors import WireError

MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1 = "magic-shared-execution-observation.v1"


def _u32(value: object, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**32 - 1:
        raise WireError("decode.invalid_json", f"{field} is outside u32")
    return value


def _i32(value: object, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not -(2**31) <= value < 2**31:
        raise WireError("decode.invalid_json", f"{field} is outside i32")
    return value


@dataclass(frozen=True, slots=True)
class PublicModeV1:
    mode_slot: int
    selected_mode: int

    @classmethod
    def from_wire(cls, value: object) -> PublicModeV1:
        obj = require_exact_keys(value, {"mode_slot", "selected_mode"})
        return cls(
            _u32(obj["mode_slot"], "mode_slot"),
            _u32(obj["selected_mode"], "selected_mode"),
        )

    def to_wire(self) -> dict[str, int]:
        _u32(self.mode_slot, "mode_slot")
        _u32(self.selected_mode, "selected_mode")
        return {"mode_slot": self.mode_slot, "selected_mode": self.selected_mode}


@dataclass(frozen=True, slots=True)
class SpellStackItemV1:
    controller: int
    card_object: int
    modes: tuple[PublicModeV1, ...]
    targets: tuple[SafeTargetDescriptorV1, ...]
    cost_facts: CostFactsV1

    @classmethod
    def from_wire(cls, value: object) -> SpellStackItemV1:
        obj = require_exact_keys(
            value,
            {"kind", "controller", "card_object", "modes", "targets", "cost_facts"},
        )
        if obj["kind"] != "spell":
            raise WireError("decode.invalid_json", "invalid spell stack item")
        if not isinstance(obj["modes"], list) or not isinstance(obj["targets"], list):
            raise WireError("decode.invalid_json", "stack modes and targets must be arrays")
        return cls(
            parse_uint(obj["controller"]),
            parse_uint(obj["card_object"]),
            tuple(PublicModeV1.from_wire(item) for item in obj["modes"]),
            tuple(SafeTargetDescriptorV1.from_wire(item) for item in obj["targets"]),
            CostFactsV1.from_wire(obj["cost_facts"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "kind": "spell",
            "controller": uint_wire(self.controller),
            "card_object": uint_wire(self.card_object),
            "modes": [item.to_wire() for item in self.modes],
            "targets": [item.to_wire() for item in self.targets],
            "cost_facts": self.cost_facts.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class ActivatedAbilityStackItemV1:
    controller: int
    source_object: int | None
    source_ability: int | None
    modes: tuple[PublicModeV1, ...]
    targets: tuple[SafeTargetDescriptorV1, ...]
    cost_facts: CostFactsV1

    @classmethod
    def from_wire(cls, value: object) -> ActivatedAbilityStackItemV1:
        obj = require_exact_keys(
            value,
            {
                "kind",
                "controller",
                "source_object",
                "source_ability",
                "modes",
                "targets",
                "cost_facts",
            },
        )
        if (
            obj["kind"] != "activated_ability"
            or not isinstance(obj["modes"], list)
            or not isinstance(obj["targets"], list)
        ):
            raise WireError("decode.invalid_json", "invalid activated ability stack item")
        return cls(
            parse_uint(obj["controller"]),
            None if obj["source_object"] is None else parse_uint(obj["source_object"]),
            None if obj["source_ability"] is None else parse_uint(obj["source_ability"]),
            tuple(PublicModeV1.from_wire(item) for item in obj["modes"]),
            tuple(SafeTargetDescriptorV1.from_wire(item) for item in obj["targets"]),
            CostFactsV1.from_wire(obj["cost_facts"]),
        )

    def validate(self) -> None:
        if self.source_ability is not None and self.source_object is None:
            raise WireError(
                "semantic.observation",
                "visible ability source requires visible source object",
            )
        slots = tuple(mode.mode_slot for mode in self.modes)
        if tuple(sorted(set(slots))) != slots:
            raise WireError("semantic.observation", "stack modes must be uniquely sorted by slot")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "kind": "activated_ability",
            "controller": uint_wire(self.controller),
            "source_object": None if self.source_object is None else uint_wire(self.source_object),
            "source_ability": None
            if self.source_ability is None
            else uint_wire(self.source_ability),
            "modes": [item.to_wire() for item in self.modes],
            "targets": [item.to_wire() for item in self.targets],
            "cost_facts": self.cost_facts.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class TriggeredAbilityStackItemV1:
    controller: int
    source_object: int | None
    source_ability: int | None
    targets: tuple[SafeTargetDescriptorV1, ...]

    @classmethod
    def from_wire(cls, value: object) -> TriggeredAbilityStackItemV1:
        obj = require_exact_keys(
            value,
            {"kind", "controller", "source_object", "source_ability", "targets"},
        )
        if obj["kind"] != "triggered_ability" or not isinstance(obj["targets"], list):
            raise WireError("decode.invalid_json", "invalid triggered ability stack item")
        return cls(
            parse_uint(obj["controller"]),
            None if obj["source_object"] is None else parse_uint(obj["source_object"]),
            None if obj["source_ability"] is None else parse_uint(obj["source_ability"]),
            tuple(SafeTargetDescriptorV1.from_wire(item) for item in obj["targets"]),
        )

    def validate(self) -> None:
        if self.source_ability is not None and self.source_object is None:
            raise WireError(
                "semantic.observation",
                "visible ability source requires visible source object",
            )

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "kind": "triggered_ability",
            "controller": uint_wire(self.controller),
            "source_object": None if self.source_object is None else uint_wire(self.source_object),
            "source_ability": None
            if self.source_ability is None
            else uint_wire(self.source_ability),
            "targets": [item.to_wire() for item in self.targets],
        }


PublicStackItemV1 = SpellStackItemV1 | ActivatedAbilityStackItemV1 | TriggeredAbilityStackItemV1


def public_stack_item_from_wire(value: object) -> PublicStackItemV1:
    if not isinstance(value, dict):
        raise WireError("decode.invalid_json", "public stack item must be an object")
    kind = value.get("kind")
    if kind == "spell":
        return SpellStackItemV1.from_wire(value)
    if kind == "activated_ability":
        return ActivatedAbilityStackItemV1.from_wire(value)
    if kind == "triggered_ability":
        return TriggeredAbilityStackItemV1.from_wire(value)
    raise WireError("decode.invalid_json", "unknown public stack item kind")


@dataclass(frozen=True, slots=True)
class PowerToughnessDeltaV1:
    power: int
    toughness: int

    @classmethod
    def from_wire(cls, value: object) -> PowerToughnessDeltaV1:
        obj = require_exact_keys(value, {"kind", "power", "toughness"})
        if obj["kind"] != "power_toughness_delta":
            raise WireError("decode.invalid_json", "invalid P/T temporary operation")
        return cls(_i32(obj["power"], "power"), _i32(obj["toughness"], "toughness"))

    def to_wire(self) -> dict[str, object]:
        _i32(self.power, "power")
        _i32(self.toughness, "toughness")
        return {"kind": "power_toughness_delta", "power": self.power, "toughness": self.toughness}


@dataclass(frozen=True, slots=True)
class GrantKeywordV1:
    keyword: str

    @classmethod
    def from_wire(cls, value: object) -> GrantKeywordV1:
        obj = require_exact_keys(value, {"kind", "keyword"})
        if (
            obj["kind"] != "grant_keyword"
            or not isinstance(obj["keyword"], str)
            or obj["keyword"] not in {"haste", "double_strike"}
        ):
            raise WireError("decode.invalid_json", "invalid temporary keyword grant")
        return cls(obj["keyword"])

    def to_wire(self) -> dict[str, str]:
        if not isinstance(self.keyword, str) or self.keyword not in {"haste", "double_strike"}:
            raise WireError("encode.serialization", "unknown temporary keyword")
        return {"kind": "grant_keyword", "keyword": self.keyword}


PublicTemporaryOperationV1 = PowerToughnessDeltaV1 | GrantKeywordV1


def temporary_operation_from_wire(value: object) -> PublicTemporaryOperationV1:
    if not isinstance(value, dict):
        raise WireError("decode.invalid_json", "temporary operation must be an object")
    kind = value.get("kind")
    if kind == "power_toughness_delta":
        return PowerToughnessDeltaV1.from_wire(value)
    if kind == "grant_keyword":
        return GrantKeywordV1.from_wire(value)
    raise WireError("decode.invalid_json", "unknown temporary operation")


@dataclass(frozen=True, slots=True)
class UntilEndOfTurnV1:
    turn_number: int

    @classmethod
    def from_wire(cls, value: object) -> UntilEndOfTurnV1:
        obj = require_exact_keys(value, {"kind", "turn_number"})
        if obj["kind"] != "until_end_of_turn":
            raise WireError("decode.invalid_json", "unknown temporary effect expiry")
        return cls(parse_uint(obj["turn_number"]))

    def to_wire(self) -> dict[str, object]:
        uint_wire(self.turn_number)
        return {"kind": "until_end_of_turn", "turn_number": uint_wire(self.turn_number)}


@dataclass(frozen=True, slots=True)
class PublicTemporaryEffectV1:
    affected_objects: tuple[int, ...]
    operation: PublicTemporaryOperationV1
    expiry: UntilEndOfTurnV1

    @classmethod
    def from_wire(cls, value: object) -> PublicTemporaryEffectV1:
        obj = require_exact_keys(value, {"affected_objects", "operation", "expiry"})
        if not isinstance(obj["affected_objects"], list):
            raise WireError("decode.invalid_json", "affected_objects must be an array")
        result = cls(
            tuple(parse_uint(item) for item in obj["affected_objects"]),
            temporary_operation_from_wire(obj["operation"]),
            UntilEndOfTurnV1.from_wire(obj["expiry"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if (
            not self.affected_objects
            or tuple(sorted(set(self.affected_objects))) != self.affected_objects
        ):
            raise WireError(
                "semantic.observation",
                "affected objects must be nonempty, unique, and sorted",
            )
        self.operation.to_wire()
        self.expiry.to_wire()

    def ordering_key(self) -> tuple[object, ...]:
        self.validate()
        operation_key: tuple[int, ...]
        if isinstance(self.operation, PowerToughnessDeltaV1):
            operation_key = (0, self.operation.power, self.operation.toughness)
        else:
            operation_key = (1, {"haste": 0, "double_strike": 1}[self.operation.keyword])
        return self.affected_objects, operation_key, self.expiry.turn_number

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "affected_objects": [uint_wire(item) for item in self.affected_objects],
            "operation": self.operation.to_wire(),
            "expiry": self.expiry.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class MagicSharedExecutionObservationV1:
    schema_version: str
    base_observation: MagicBasicLandObservationV1
    stack: tuple[PublicStackItemV1, ...]
    temporary_effects: tuple[PublicTemporaryEffectV1, ...]

    @classmethod
    def from_wire(cls, value: object) -> MagicSharedExecutionObservationV1:
        base_fields = {
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
        }
        obj = require_exact_keys(value, base_fields | {"stack", "temporary_effects"})
        if obj["schema_version"] != MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1:
            raise WireError("decode.invalid_json", "unsupported shared-execution payload V1")
        if not isinstance(obj["stack"], list) or not isinstance(obj["temporary_effects"], list):
            raise WireError("decode.invalid_json", "stack and temporary_effects must be arrays")
        base = dict(obj)
        del base["stack"]
        del base["temporary_effects"]
        base["schema_version"] = MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1
        result = cls(
            MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1,
            MagicBasicLandObservationV1.from_wire(base),
            tuple(public_stack_item_from_wire(item) for item in obj["stack"]),
            tuple(PublicTemporaryEffectV1.from_wire(item) for item in obj["temporary_effects"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1:
            raise WireError("semantic.observation", "unsupported payload V1")
        if self.base_observation.schema_version != MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1:
            raise WireError("semantic.observation", "invalid base observation")
        self.base_observation.to_wire()
        for item in self.stack:
            if isinstance(item, TriggeredAbilityStackItemV1):
                item.to_wire()
            if isinstance(item, SpellStackItemV1 | ActivatedAbilityStackItemV1):
                slots = tuple(mode.mode_slot for mode in item.modes)
                if tuple(sorted(set(slots))) != slots:
                    raise WireError(
                        "semantic.observation",
                        "stack modes must be uniquely sorted by slot",
                    )
                item.to_wire()
        keys = tuple(effect.ordering_key() for effect in self.temporary_effects)
        if any(left > right for left, right in pairwise(keys)):
            raise WireError(
                "semantic.observation",
                "temporary effects are not canonically ordered",
            )

    def to_wire(self) -> dict[str, object]:
        self.validate()
        result = self.base_observation.to_wire()
        result["schema_version"] = self.schema_version
        result["stack"] = [item.to_wire() for item in self.stack]
        result["temporary_effects"] = [item.to_wire() for item in self.temporary_effects]
        return result
