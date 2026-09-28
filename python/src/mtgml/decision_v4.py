"""Rules-free typed mirror for the detached Decision request V4 contract."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from itertools import pairwise
from typing import Any

from .canonical import parse_uint, require_exact_keys, uint_wire
from .decision import DecisionResponseV3, DecisionSpec
from .errors import WireError

PLAYER_DECISION_REQUEST_V4_SCHEMA = "player-decision-request.v4"
_VISIBILITY = {"public", "acting_player_only", "mixed"}
_EVENT_RANK = {
    "spell_cast": 0,
    "ability_activated": 1,
    "target_became": 2,
    "object_entered": 3,
    "object_left_or_died": 4,
    "beginning_of_combat": 5,
    "attack_declared": 6,
    "card_drawn": 7,
    "counter_changed": 8,
    "damage_applied": 9,
    "life_changed": 10,
}
_ZONE_RANK = {
    "library": 0,
    "hand": 1,
    "battlefield": 2,
    "graveyard": 3,
    "exile": 4,
    "stack": 5,
    "command": 6,
    "ante": 7,
    "outside": 8,
}
_COUNTER_RANK = {"plus_one_plus_one": 0, "minus_one_minus_one": 1, "lore": 2}
_DAMAGE_RANK = {"combat": 0, "noncombat": 1}
_LIFE_CAUSE_RANK = {"damage": 0, "non_damage": 1}
_CANDIDATE_RANK = {
    "pass_priority": 0,
    "play_land": 1,
    "cast_spell": 2,
    "activate_ability": 3,
    "select_object": 4,
    "select_player": 5,
    "select_mode": 6,
    "choose_boolean": 7,
    "declare_number": 8,
    "confirm": 9,
    "select_cost_route": 10,
    "select_mana_source": 11,
    "finalize_mana_production": 12,
    "select_mana_payment": 13,
    "select_trigger": 14,
}
_PURPOSE_CANDIDATES = {
    "priority_action": {"pass_priority", "play_land", "cast_spell", "activate_ability"},
    "attacker_declaration": {"select_object"},
    "sba_graveyard_order": {"select_object"},
    "cast_cost_route": {"select_cost_route"},
    "mode_selection": {"select_mode"},
    "target_selection": {"select_object", "select_player"},
    "cost_operand_selection": {"select_object"},
    "mana_production_choice": {"select_mana_source", "finalize_mana_production"},
    "mana_payment": {"select_mana_payment"},
    "optional_cost_payment": {"choose_boolean"},
    "ability_action": {"activate_ability"},
    "trigger_order": {"select_trigger"},
    "trigger_target": {"select_object", "select_player"},
    "synthetic_assembly": {"select_object"},
}
_PURPOSE_DOMAINS = {
    "priority_action": {"choose_one"},
    "attacker_declaration": {"choose_many"},
    "sba_graveyard_order": {"order"},
    "cast_cost_route": {"choose_one"},
    "mode_selection": {"choose_one", "choose_many"},
    "target_selection": {"choose_one", "choose_many"},
    "cost_operand_selection": {"choose_one"},
    "mana_production_choice": {"choose_one"},
    "mana_payment": {"choose_one"},
    "optional_cost_payment": {"choose_one"},
    "ability_action": {"choose_one"},
    "trigger_order": {"order"},
    "trigger_target": {"choose_one", "choose_many"},
    "synthetic_assembly": {"choose_one", "choose_number", "choose_many", "order"},
}


def _u32(value: object, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**32 - 1:
        raise WireError("decode.invalid_json", f"{field} is outside u32")
    return value


def _i64(value: object, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not -(2**63) <= value < 2**63:
        raise WireError("decode.invalid_json", f"{field} is outside i64")
    return value


def _nullable_u64(value: object, field: str) -> int | None:
    return None if value is None else parse_uint(value)


def _nullable_wire(value: int | None) -> str | None:
    return None if value is None else uint_wire(value)


def _array(value: object, field: str) -> list[object]:
    if not isinstance(value, list):
        raise WireError("decode.invalid_json", f"{field} must be an array")
    return value


def _opt_key(value: int | None) -> tuple[int, int]:
    return (0, 0) if value is None else (1, value)


@dataclass(frozen=True, slots=True)
class SafeTargetDescriptorV1:
    kind: str
    object_id: int | None = None
    player_id: int | None = None
    stack_position_from_top: int | None = None

    @classmethod
    def from_wire(cls, value: object) -> SafeTargetDescriptorV1:
        if not isinstance(value, dict):
            raise WireError("decode.invalid_json", "target descriptor must be an object")
        kind = value.get("kind")
        if kind == "object":
            obj = require_exact_keys(value, {"kind", "object"})
            return cls(kind, object_id=parse_uint(obj["object"]))
        if kind == "player":
            obj = require_exact_keys(value, {"kind", "player"})
            return cls(kind, player_id=parse_uint(obj["player"]))
        if kind == "stack_item":
            obj = require_exact_keys(value, {"kind", "stack_position_from_top"})
            return cls(
                kind,
                stack_position_from_top=_u32(
                    obj["stack_position_from_top"], "stack_position_from_top"
                ),
            )
        raise WireError("decode.invalid_json", "unknown SafeTargetDescriptorV1 variant")

    def validate(self) -> None:
        if (
            self.kind == "object"
            and self.object_id is not None
            and self.player_id is None
            and self.stack_position_from_top is None
        ):
            return
        if (
            self.kind == "player"
            and self.player_id is not None
            and self.object_id is None
            and self.stack_position_from_top is None
        ):
            return
        if (
            self.kind == "stack_item"
            and self.stack_position_from_top is not None
            and self.object_id is None
            and self.player_id is None
        ):
            _u32(self.stack_position_from_top, "stack_position_from_top")
            return
        raise WireError("semantic.decision", "invalid SafeTargetDescriptorV1 payload")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        if self.kind == "object":
            assert self.object_id is not None
            return {"kind": self.kind, "object": uint_wire(self.object_id)}
        if self.kind == "player":
            assert self.player_id is not None
            return {"kind": self.kind, "player": uint_wire(self.player_id)}
        assert self.stack_position_from_top is not None
        return {
            "kind": self.kind,
            "stack_position_from_top": self.stack_position_from_top,
        }

    def ordering_key(self) -> tuple[int, int]:
        self.validate()
        if self.kind == "object":
            assert self.object_id is not None
            return 0, self.object_id
        if self.kind == "player":
            assert self.player_id is not None
            return 1, self.player_id
        assert self.stack_position_from_top is not None
        return 2, self.stack_position_from_top


@dataclass(frozen=True, slots=True)
class CostFactsV1:
    selected_route_kind: str | None
    route_id: int | None
    paid_additional_cost_ids: tuple[int, ...]

    @classmethod
    def from_wire(cls, value: object) -> CostFactsV1:
        obj = require_exact_keys(value, {"selected_route", "paid_additional_cost_ids"})
        route = obj["selected_route"]
        route_kind: str | None = None
        route_id: int | None = None
        if route is not None:
            if not isinstance(route, dict):
                raise WireError("decode.invalid_json", "selected_route must be an object or null")
            route_kind = route.get("kind")
            if route_kind == "normal":
                require_exact_keys(route, {"kind"})
            elif route_kind == "alternative":
                route_obj = require_exact_keys(route, {"kind", "route_id"})
                route_id = _u32(route_obj["route_id"], "route_id")
            else:
                raise WireError("decode.invalid_json", "unknown cost route")
        raw_ids = _array(obj["paid_additional_cost_ids"], "paid_additional_cost_ids")
        ids = tuple(_u32(item, "paid_additional_cost_id") for item in raw_ids)
        result = cls(route_kind, route_id, ids)
        result.validate()
        return result

    def validate(self) -> None:
        if self.selected_route_kind not in {None, "normal", "alternative"}:
            raise WireError("semantic.decision", "unknown selected cost route")
        if (self.selected_route_kind == "alternative") != (self.route_id is not None):
            raise WireError("semantic.decision", "alternative route ID relation is invalid")
        if tuple(sorted(set(self.paid_additional_cost_ids))) != self.paid_additional_cost_ids:
            raise WireError("semantic.decision", "paid cost IDs must be sorted and unique")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        if self.selected_route_kind is None:
            route: dict[str, object] | None = None
        elif self.selected_route_kind == "normal":
            route = {"kind": "normal"}
        else:
            assert self.route_id is not None
            route = {"kind": "alternative", "route_id": self.route_id}
        return {
            "paid_additional_cost_ids": list(self.paid_additional_cost_ids),
            "selected_route": route,
        }

    def ordering_key(self) -> tuple[object, ...]:
        self.validate()
        route_key = (
            (0, 0)
            if self.selected_route_kind is None
            else ((1, 0) if self.selected_route_kind == "normal" else (2, self.route_id))
        )
        return route_key, self.paid_additional_cost_ids


@dataclass(frozen=True, slots=True)
class PrintedManaSymbolsV1:
    colored_wubrg_counts: tuple[int, int, int, int, int]
    colorless_count: int
    generic_count: int

    @classmethod
    def from_wire(cls, value: object) -> PrintedManaSymbolsV1:
        obj = require_exact_keys(
            value,
            {"colored_wubrg_counts", "colorless_count", "generic_count"},
        )
        colors = tuple(
            _u32(item, "colored_wubrg_count")
            for item in _array(obj["colored_wubrg_counts"], "colored_wubrg_counts")
        )
        if len(colors) != 5:
            raise WireError(
                "decode.invalid_json", "colored W/U/B/R/G counts must have five entries"
            )
        return cls(
            colors,
            _u32(obj["colorless_count"], "colorless_count"),
            _u32(obj["generic_count"], "generic_count"),
        )

    def to_wire(self) -> dict[str, object]:
        colors = tuple(_u32(item, "colored_wubrg_count") for item in self.colored_wubrg_counts)
        if len(colors) != 5:
            raise WireError(
                "encode.serialization", "colored W/U/B/R/G counts must have five entries"
            )
        return {
            "colored_wubrg_counts": list(colors),
            "colorless_count": _u32(self.colorless_count, "colorless_count"),
            "generic_count": _u32(self.generic_count, "generic_count"),
        }


@dataclass(frozen=True, slots=True)
class CostRouteDescriptorV1:
    route_class: str
    printed_mana_symbols: PrintedManaSymbolsV1
    profile_local_option_ordinal: int | None

    @classmethod
    def from_wire(cls, value: object) -> CostRouteDescriptorV1:
        obj = require_exact_keys(
            value,
            {"route_class", "printed_mana_symbols", "profile_local_option_ordinal"},
        )
        route_class = obj["route_class"]
        if not isinstance(route_class, str):
            raise WireError("decode.invalid_json", "unknown cost route class")
        ordinal = (
            None
            if obj["profile_local_option_ordinal"] is None
            else _u32(obj["profile_local_option_ordinal"], "profile_local_option_ordinal")
        )
        if route_class not in {"normal", "alternative"}:
            raise WireError("decode.invalid_json", "unknown cost route class")
        result = cls(
            route_class,
            PrintedManaSymbolsV1.from_wire(obj["printed_mana_symbols"]),
            ordinal,
        )
        result.validate()
        return result

    def validate(self) -> None:
        if not isinstance(self.route_class, str) or self.route_class not in {
            "normal",
            "alternative",
        }:
            raise WireError("semantic.decision", "unknown cost route class")
        if (self.route_class == "normal") != (self.profile_local_option_ordinal is None):
            raise WireError("semantic.decision", "cost route class and ordinal disagree")
        if self.profile_local_option_ordinal is not None:
            _u32(self.profile_local_option_ordinal, "profile_local_option_ordinal")
        self.printed_mana_symbols.to_wire()

    def ordering_key(self) -> tuple[int, int]:
        self.validate()
        if self.route_class == "normal":
            return 0, 0
        assert self.profile_local_option_ordinal is not None
        return 1, self.profile_local_option_ordinal

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "route_class": self.route_class,
            "printed_mana_symbols": self.printed_mana_symbols.to_wire(),
            "profile_local_option_ordinal": self.profile_local_option_ordinal,
        }


@dataclass(frozen=True, slots=True)
class AttackerFactV1:
    attacker: int
    defending_player: int

    @classmethod
    def from_wire(cls, value: object) -> AttackerFactV1:
        obj = require_exact_keys(value, {"attacker", "defending_player"})
        return cls(parse_uint(obj["attacker"]), parse_uint(obj["defending_player"]))

    def to_wire(self) -> dict[str, str]:
        return {
            "attacker": uint_wire(self.attacker),
            "defending_player": uint_wire(self.defending_player),
        }


@dataclass(frozen=True, slots=True)
class SpellCastSubjectV1:
    actor: int
    spell_source_object: int | None
    creature_spell: bool
    cost_facts: CostFactsV1

    @classmethod
    def from_wire(cls, value: object) -> SpellCastSubjectV1:
        obj = require_exact_keys(
            value, {"kind", "actor", "spell_source_object", "creature_spell", "cost_facts"}
        )
        if obj["kind"] != "spell_cast" or not isinstance(obj["creature_spell"], bool):
            raise WireError("decode.invalid_json", "invalid spell_cast subject")
        return cls(
            parse_uint(obj["actor"]),
            _nullable_u64(obj["spell_source_object"], "spell_source_object"),
            obj["creature_spell"],
            CostFactsV1.from_wire(obj["cost_facts"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "kind": "spell_cast",
            "actor": uint_wire(self.actor),
            "spell_source_object": _nullable_wire(self.spell_source_object),
            "creature_spell": self.creature_spell,
            "cost_facts": self.cost_facts.to_wire(),
        }

    def ordering_key(self) -> tuple[object, ...]:
        return (
            self.actor,
            _opt_key(self.spell_source_object),
            self.creature_spell,
            self.cost_facts.ordering_key(),
        )


@dataclass(frozen=True, slots=True)
class AbilityActivatedSubjectV1:
    actor: int
    activated_source_object: int
    activated_source_ability: int
    cost_facts: CostFactsV1
    targets: tuple[SafeTargetDescriptorV1, ...]

    @classmethod
    def from_wire(cls, value: object) -> AbilityActivatedSubjectV1:
        obj = require_exact_keys(
            value,
            {
                "kind",
                "actor",
                "activated_source_object",
                "activated_source_ability",
                "cost_facts",
                "targets",
            },
        )
        if obj["kind"] != "ability_activated":
            raise WireError("decode.invalid_json", "invalid ability_activated subject")
        return cls(
            parse_uint(obj["actor"]),
            parse_uint(obj["activated_source_object"]),
            parse_uint(obj["activated_source_ability"]),
            CostFactsV1.from_wire(obj["cost_facts"]),
            tuple(
                SafeTargetDescriptorV1.from_wire(item) for item in _array(obj["targets"], "targets")
            ),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "kind": "ability_activated",
            "actor": uint_wire(self.actor),
            "activated_source_object": uint_wire(self.activated_source_object),
            "activated_source_ability": uint_wire(self.activated_source_ability),
            "cost_facts": self.cost_facts.to_wire(),
            "targets": [item.to_wire() for item in self.targets],
        }

    def ordering_key(self) -> tuple[object, ...]:
        return (
            self.actor,
            self.activated_source_object,
            self.activated_source_ability,
            tuple(item.ordering_key() for item in self.targets),
            self.cost_facts.ordering_key(),
        )


@dataclass(frozen=True, slots=True)
class TargetBecameSubjectV1:
    actor: int
    target: SafeTargetDescriptorV1

    @classmethod
    def from_wire(cls, value: object) -> TargetBecameSubjectV1:
        obj = require_exact_keys(value, {"kind", "actor", "target"})
        if obj["kind"] != "target_became":
            raise WireError("decode.invalid_json", "invalid target_became subject")
        return cls(parse_uint(obj["actor"]), SafeTargetDescriptorV1.from_wire(obj["target"]))

    def to_wire(self) -> dict[str, object]:
        return {
            "kind": "target_became",
            "actor": uint_wire(self.actor),
            "target": self.target.to_wire(),
        }

    def ordering_key(self) -> tuple[object, ...]:
        return self.actor, self.target.ordering_key()


@dataclass(frozen=True, slots=True)
class ObjectEnteredSubjectV1:
    object_id: int | None

    @classmethod
    def from_wire(cls, value: object) -> ObjectEnteredSubjectV1:
        obj = require_exact_keys(value, {"kind", "object"})
        if obj["kind"] != "object_entered":
            raise WireError("decode.invalid_json", "invalid object_entered subject")
        return cls(_nullable_u64(obj["object"], "object"))

    def to_wire(self) -> dict[str, object]:
        return {"kind": "object_entered", "object": _nullable_wire(self.object_id)}

    def ordering_key(self) -> tuple[object, ...]:
        return (_opt_key(self.object_id),)


@dataclass(frozen=True, slots=True)
class ObjectLeftOrDiedSubjectV1:
    last_known_object: int | None
    destination: str

    @classmethod
    def from_wire(cls, value: object) -> ObjectLeftOrDiedSubjectV1:
        obj = require_exact_keys(value, {"kind", "last_known_object", "destination"})
        if obj["kind"] != "object_left_or_died" or obj["destination"] not in _ZONE_RANK:
            raise WireError("decode.invalid_json", "invalid object_left_or_died subject")
        return cls(
            _nullable_u64(obj["last_known_object"], "last_known_object"), str(obj["destination"])
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "kind": "object_left_or_died",
            "last_known_object": _nullable_wire(self.last_known_object),
            "destination": self.destination,
        }

    def ordering_key(self) -> tuple[object, ...]:
        return _opt_key(self.last_known_object), _ZONE_RANK[self.destination]


@dataclass(frozen=True, slots=True)
class BeginningOfCombatSubjectV1:
    active_player: int
    turn_number: int

    @classmethod
    def from_wire(cls, value: object) -> BeginningOfCombatSubjectV1:
        obj = require_exact_keys(value, {"kind", "active_player", "turn_number"})
        if obj["kind"] != "beginning_of_combat":
            raise WireError("decode.invalid_json", "invalid beginning_of_combat subject")
        return cls(parse_uint(obj["active_player"]), parse_uint(obj["turn_number"]))

    def to_wire(self) -> dict[str, object]:
        return {
            "kind": "beginning_of_combat",
            "active_player": uint_wire(self.active_player),
            "turn_number": uint_wire(self.turn_number),
        }

    def ordering_key(self) -> tuple[int, int]:
        return self.active_player, self.turn_number


@dataclass(frozen=True, slots=True)
class AttackDeclaredSubjectV1:
    controller: int
    attackers: tuple[AttackerFactV1, ...]

    @classmethod
    def from_wire(cls, value: object) -> AttackDeclaredSubjectV1:
        obj = require_exact_keys(value, {"kind", "controller", "attackers"})
        if obj["kind"] != "attack_declared":
            raise WireError("decode.invalid_json", "invalid attack_declared subject")
        return cls(
            parse_uint(obj["controller"]),
            tuple(AttackerFactV1.from_wire(item) for item in _array(obj["attackers"], "attackers")),
        )

    def validate(self) -> None:
        ids = tuple(item.attacker for item in self.attackers)
        if tuple(sorted(set(ids))) != ids:
            raise WireError(
                "semantic.decision", "attackers must be uniquely sorted by opaque identity"
            )

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "kind": "attack_declared",
            "controller": uint_wire(self.controller),
            "attackers": [item.to_wire() for item in self.attackers],
        }

    def ordering_key(self) -> tuple[object, ...]:
        self.validate()
        return self.controller, tuple(
            (item.attacker, item.defending_player) for item in self.attackers
        )


@dataclass(frozen=True, slots=True)
class CardDrawnSubjectV1:
    player: int

    @classmethod
    def from_wire(cls, value: object) -> CardDrawnSubjectV1:
        obj = require_exact_keys(value, {"kind", "player"})
        if obj["kind"] != "card_drawn":
            raise WireError("decode.invalid_json", "invalid card_drawn subject")
        return cls(parse_uint(obj["player"]))

    def to_wire(self) -> dict[str, object]:
        return {"kind": "card_drawn", "player": uint_wire(self.player)}

    def ordering_key(self) -> tuple[int]:
        return (self.player,)


@dataclass(frozen=True, slots=True)
class CounterChangedSubjectV1:
    object_id: int | None
    counter_kind: str
    before: int
    after: int

    @classmethod
    def from_wire(cls, value: object) -> CounterChangedSubjectV1:
        obj = require_exact_keys(value, {"kind", "object", "counter_kind", "before", "after"})
        if obj["kind"] != "counter_changed" or obj["counter_kind"] not in _COUNTER_RANK:
            raise WireError("decode.invalid_json", "invalid counter_changed subject")
        return cls(
            _nullable_u64(obj["object"], "object"),
            str(obj["counter_kind"]),
            _u32(obj["before"], "before"),
            _u32(obj["after"], "after"),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "kind": "counter_changed",
            "object": _nullable_wire(self.object_id),
            "counter_kind": self.counter_kind,
            "before": self.before,
            "after": self.after,
        }

    def ordering_key(self) -> tuple[object, ...]:
        return _opt_key(self.object_id), _COUNTER_RANK[self.counter_kind], self.before, self.after


@dataclass(frozen=True, slots=True)
class DamageRecipientV1:
    kind: str
    object_id: int | None = None
    player_id: int | None = None

    @classmethod
    def from_wire(cls, value: object) -> DamageRecipientV1:
        target = SafeTargetDescriptorV1.from_wire(value)
        if target.kind == "object":
            return cls("object", object_id=target.object_id)
        if target.kind == "player":
            return cls("player", player_id=target.player_id)
        raise WireError("decode.invalid_json", "damage recipient cannot be a stack item")

    def to_wire(self) -> dict[str, object]:
        return SafeTargetDescriptorV1(self.kind, self.object_id, self.player_id).to_wire()

    def ordering_key(self) -> tuple[int, int]:
        return SafeTargetDescriptorV1(self.kind, self.object_id, self.player_id).ordering_key()


@dataclass(frozen=True, slots=True)
class DamageAppliedSubjectV1:
    source_object: int | None
    recipient: DamageRecipientV1
    amount: int
    damage_kind: str

    @classmethod
    def from_wire(cls, value: object) -> DamageAppliedSubjectV1:
        obj = require_exact_keys(
            value, {"kind", "source_object", "recipient", "amount", "damage_kind"}
        )
        if obj["kind"] != "damage_applied" or obj["damage_kind"] not in _DAMAGE_RANK:
            raise WireError("decode.invalid_json", "invalid damage_applied subject")
        return cls(
            _nullable_u64(obj["source_object"], "source_object"),
            DamageRecipientV1.from_wire(obj["recipient"]),
            _u32(obj["amount"], "amount"),
            str(obj["damage_kind"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "kind": "damage_applied",
            "source_object": _nullable_wire(self.source_object),
            "recipient": self.recipient.to_wire(),
            "amount": self.amount,
            "damage_kind": self.damage_kind,
        }

    def ordering_key(self) -> tuple[object, ...]:
        return (
            _opt_key(self.source_object),
            self.recipient.ordering_key(),
            self.amount,
            _DAMAGE_RANK[self.damage_kind],
        )


@dataclass(frozen=True, slots=True)
class LifeChangedSubjectV1:
    player: int
    before: int
    after: int
    cause: str

    @classmethod
    def from_wire(cls, value: object) -> LifeChangedSubjectV1:
        obj = require_exact_keys(value, {"kind", "player", "before", "after", "cause"})
        if obj["kind"] != "life_changed" or obj["cause"] not in _LIFE_CAUSE_RANK:
            raise WireError("decode.invalid_json", "invalid life_changed subject")
        return cls(
            parse_uint(obj["player"]),
            _i64(obj["before"], "before"),
            _i64(obj["after"], "after"),
            str(obj["cause"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "kind": "life_changed",
            "player": uint_wire(self.player),
            "before": self.before,
            "after": self.after,
            "cause": self.cause,
        }

    def ordering_key(self) -> tuple[object, ...]:
        return self.player, self.before, self.after, _LIFE_CAUSE_RANK[self.cause]


SafeTriggerSubjectV1 = (
    SpellCastSubjectV1
    | AbilityActivatedSubjectV1
    | TargetBecameSubjectV1
    | ObjectEnteredSubjectV1
    | ObjectLeftOrDiedSubjectV1
    | BeginningOfCombatSubjectV1
    | AttackDeclaredSubjectV1
    | CardDrawnSubjectV1
    | CounterChangedSubjectV1
    | DamageAppliedSubjectV1
    | LifeChangedSubjectV1
)
_SUBJECT_DECODERS: dict[str, Callable[[object], SafeTriggerSubjectV1]] = {
    "spell_cast": SpellCastSubjectV1.from_wire,
    "ability_activated": AbilityActivatedSubjectV1.from_wire,
    "target_became": TargetBecameSubjectV1.from_wire,
    "object_entered": ObjectEnteredSubjectV1.from_wire,
    "object_left_or_died": ObjectLeftOrDiedSubjectV1.from_wire,
    "beginning_of_combat": BeginningOfCombatSubjectV1.from_wire,
    "attack_declared": AttackDeclaredSubjectV1.from_wire,
    "card_drawn": CardDrawnSubjectV1.from_wire,
    "counter_changed": CounterChangedSubjectV1.from_wire,
    "damage_applied": DamageAppliedSubjectV1.from_wire,
    "life_changed": LifeChangedSubjectV1.from_wire,
}


@dataclass(frozen=True, slots=True)
class SafeTriggerDescriptorV1:
    source_object: int | None
    source_ability: int | None
    event_kind: str
    subject: SafeTriggerSubjectV1

    @classmethod
    def from_wire(cls, value: object) -> SafeTriggerDescriptorV1:
        obj = require_exact_keys(
            value, {"source_object", "source_ability", "event_kind", "subject"}
        )
        event_kind = obj["event_kind"]
        decoder = _SUBJECT_DECODERS.get(event_kind)
        if decoder is None:
            raise WireError("decode.invalid_json", "unknown trigger event kind")
        result = cls(
            _nullable_u64(obj["source_object"], "source_object"),
            _nullable_u64(obj["source_ability"], "source_ability"),
            event_kind,
            decoder(obj["subject"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        subject_kind = next(
            (
                event_kind
                for event_kind, subject_type in (
                    ("spell_cast", SpellCastSubjectV1),
                    ("ability_activated", AbilityActivatedSubjectV1),
                    ("target_became", TargetBecameSubjectV1),
                    ("object_entered", ObjectEnteredSubjectV1),
                    ("object_left_or_died", ObjectLeftOrDiedSubjectV1),
                    ("beginning_of_combat", BeginningOfCombatSubjectV1),
                    ("attack_declared", AttackDeclaredSubjectV1),
                    ("card_drawn", CardDrawnSubjectV1),
                    ("counter_changed", CounterChangedSubjectV1),
                    ("damage_applied", DamageAppliedSubjectV1),
                    ("life_changed", LifeChangedSubjectV1),
                )
                if isinstance(self.subject, subject_type)
            ),
            None,
        )
        if self.event_kind not in _EVENT_RANK or subject_kind != self.event_kind:
            raise WireError("semantic.decision", "trigger event and subject tags differ")
        if self.source_ability is not None and self.source_object is None:
            raise WireError(
                "semantic.decision",
                "visible source ability requires its visible source object",
            )
        if isinstance(self.subject, AttackDeclaredSubjectV1):
            self.subject.validate()

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "event_kind": self.event_kind,
            "source_ability": _nullable_wire(self.source_ability),
            "source_object": _nullable_wire(self.source_object),
            "subject": self.subject.to_wire(),
        }

    def ordering_key(self) -> tuple[object, ...]:
        self.validate()
        return (
            _EVENT_RANK[self.event_kind],
            self.subject.ordering_key(),
            _opt_key(self.source_object),
            _opt_key(self.source_ability),
        )


@dataclass(frozen=True, slots=True)
class CandidateIntentV4:
    kind: str
    object_id: int | None = None
    player_id: int | None = None
    ability_id: int | None = None
    mode_index: int | None = None
    boolean_value: bool | None = None
    number_value: int | None = None
    cost_route_descriptor: CostRouteDescriptorV1 | None = None
    source_object_id: int | None = None
    source_ability_id: int | None = None
    produced_buckets: tuple[int, ...] | None = None
    spent_buckets: tuple[int, ...] | None = None
    trigger: SafeTriggerDescriptorV1 | None = None

    @classmethod
    def from_wire(cls, value: object) -> CandidateIntentV4:
        if (
            not isinstance(value, dict)
            or not isinstance(value.get("kind"), str)
            or value["kind"] not in _CANDIDATE_RANK
        ):
            raise WireError("decode.invalid_json", "unknown candidate V4 intent")
        kind = value["kind"]
        fields = {
            "pass_priority": set(),
            "play_land": {"object"},
            "cast_spell": {"object"},
            "activate_ability": {"ability"},
            "select_object": {"object"},
            "select_player": {"player"},
            "select_mode": {"mode_index"},
            "choose_boolean": {"value"},
            "declare_number": {"value"},
            "confirm": set(),
            "select_cost_route": {"descriptor"},
            "select_mana_source": {"source", "ability", "produced_buckets"},
            "finalize_mana_production": set(),
            "select_mana_payment": {"spent_buckets"},
            "select_trigger": {"trigger"},
        }[kind]
        obj = require_exact_keys(value, {"kind", *fields})
        if kind in {"play_land", "cast_spell", "select_object"}:
            return cls(kind, object_id=parse_uint(obj["object"]))
        if kind in {"activate_ability"}:
            return cls(kind, ability_id=parse_uint(obj["ability"]))
        if kind == "select_player":
            return cls(kind, player_id=parse_uint(obj["player"]))
        if kind == "select_mode":
            return cls(kind, mode_index=_u32(obj["mode_index"], "mode_index"))
        if kind == "choose_boolean":
            if not isinstance(obj["value"], bool):
                raise WireError("decode.invalid_json", "value must be boolean")
            return cls(kind, boolean_value=obj["value"])
        if kind == "declare_number":
            return cls(kind, number_value=_i64(obj["value"], "value"))
        if kind == "select_cost_route":
            return cls(
                kind,
                cost_route_descriptor=CostRouteDescriptorV1.from_wire(obj["descriptor"]),
            )
        if kind == "select_mana_source":
            buckets = tuple(
                _u32(item, "produced_buckets")
                for item in _array(obj["produced_buckets"], "produced_buckets")
            )
            if len(buckets) != 12:
                raise WireError("decode.invalid_json", "produced_buckets must contain 12 counts")
            return cls(
                kind,
                source_object_id=parse_uint(obj["source"]),
                source_ability_id=parse_uint(obj["ability"]),
                produced_buckets=buckets,
            )
        if kind == "select_mana_payment":
            buckets = tuple(
                _u32(item, "spent_buckets")
                for item in _array(obj["spent_buckets"], "spent_buckets")
            )
            if len(buckets) != 12:
                raise WireError("decode.invalid_json", "spent_buckets must contain 12 counts")
            return cls(kind, spent_buckets=buckets)
        if kind == "select_trigger":
            return cls(kind, trigger=SafeTriggerDescriptorV1.from_wire(obj["trigger"]))
        return cls(kind)

    def to_wire(self) -> dict[str, object]:
        result: dict[str, object] = {"kind": self.kind}
        fields = {
            "play_land": ("object", self.object_id),
            "cast_spell": ("object", self.object_id),
            "select_object": ("object", self.object_id),
            "activate_ability": ("ability", self.ability_id),
            "select_player": ("player", self.player_id),
            "select_mode": ("mode_index", self.mode_index),
            "choose_boolean": ("value", self.boolean_value),
            "declare_number": ("value", self.number_value),
            "select_mana_source": ("source", self.source_object_id),
        }
        if self.kind in fields:
            field, value = fields[self.kind]
            if value is None:
                raise WireError("encode.serialization", "candidate payload is absent")
            result[field] = (
                uint_wire(value) if field in {"object", "ability", "player", "source"} else value
            )
        if self.kind == "select_cost_route":
            if self.cost_route_descriptor is None:
                raise WireError("encode.serialization", "cost route descriptor is absent")
            result["descriptor"] = self.cost_route_descriptor.to_wire()
        if self.kind == "select_mana_source":
            if self.source_ability_id is None or self.produced_buckets is None:
                raise WireError("encode.serialization", "mana source payload is incomplete")
            result["ability"] = uint_wire(self.source_ability_id)
            result["produced_buckets"] = list(self.produced_buckets)
        elif self.kind == "select_mana_payment":
            if self.spent_buckets is None:
                raise WireError("encode.serialization", "mana payment payload is absent")
            result["spent_buckets"] = list(self.spent_buckets)
        elif self.kind == "select_trigger":
            if self.trigger is None:
                raise WireError("encode.serialization", "trigger descriptor is absent")
            result["trigger"] = self.trigger.to_wire()
        self.from_wire(result)
        return result

    def ordering_key(self) -> tuple[object, ...]:
        rank = _CANDIDATE_RANK.get(self.kind)
        if rank is None:
            raise WireError("semantic.decision", "unknown candidate ordering variant")
        value: object = ()
        if self.kind in {"play_land", "cast_spell", "select_object"}:
            if self.object_id is None:
                raise WireError("semantic.decision", "candidate object is absent")
            value = self.object_id
        elif self.kind in {"activate_ability", "select_player"}:
            item = self.ability_id if self.kind == "activate_ability" else self.player_id
            if item is None:
                raise WireError("semantic.decision", "candidate identity is absent")
            value = item
        elif self.kind == "select_mode":
            value = _u32(self.mode_index, "mode_index")
        elif self.kind == "choose_boolean":
            if self.boolean_value is None:
                raise WireError("semantic.decision", "candidate boolean is absent")
            value = int(self.boolean_value)
        elif self.kind == "declare_number":
            if self.number_value is None:
                raise WireError("semantic.decision", "candidate number is absent")
            value = _i64(self.number_value, "number_value")
        elif self.kind == "select_cost_route":
            if self.cost_route_descriptor is None:
                raise WireError("semantic.decision", "cost route descriptor is absent")
            value = self.cost_route_descriptor.ordering_key()
        elif self.kind == "select_mana_source":
            if (
                self.source_object_id is None
                or self.source_ability_id is None
                or self.produced_buckets is None
            ):
                raise WireError("semantic.decision", "mana source candidate is incomplete")
            value = (self.source_object_id, self.source_ability_id, self.produced_buckets)
        elif self.kind == "select_mana_payment":
            if self.spent_buckets is None:
                raise WireError("semantic.decision", "payment vector is absent")
            value = tuple(_u32(item, "spent_buckets") for item in self.spent_buckets)
        elif self.kind == "select_trigger":
            if self.trigger is None:
                raise WireError("semantic.decision", "trigger descriptor is absent")
            value = self.trigger.ordering_key()
        return rank, value


@dataclass(frozen=True, slots=True)
class VisibleCandidateV4:
    candidate_id: int
    intent: CandidateIntentV4

    @classmethod
    def from_wire(cls, value: object) -> VisibleCandidateV4:
        obj = require_exact_keys(value, {"candidate_id", "intent"})
        return cls(
            _u32(obj["candidate_id"], "candidate_id"), CandidateIntentV4.from_wire(obj["intent"])
        )

    def to_wire(self) -> dict[str, object]:
        return {"candidate_id": self.candidate_id, "intent": self.intent.to_wire()}


@dataclass(frozen=True, slots=True)
class DecisionPurposeV4:
    kind: str
    mode_slot: int | None = None
    target_slot: int | None = None
    cost_slot: int | None = None
    operation: str | None = None
    counter_kind: str | None = None
    count: int | None = None
    profile_local_cost_id: int | None = None
    stage: str | None = None

    @classmethod
    def from_wire(cls, value: object) -> DecisionPurposeV4:
        if not isinstance(value, dict):
            raise WireError("decode.invalid_json", "decision purpose must be an object")
        kind = value.get("kind")
        fields: dict[str, set[str]] = {
            "priority_action": set(),
            "attacker_declaration": set(),
            "sba_graveyard_order": set(),
            "cast_cost_route": set(),
            "mode_selection": {"mode_slot"},
            "target_selection": {"target_slot"},
            "cost_operand_selection": {"cost_slot", "operation", "counter_kind", "count"},
            "mana_production_choice": set(),
            "mana_payment": set(),
            "optional_cost_payment": {"profile_local_cost_id"},
            "ability_action": set(),
            "trigger_order": set(),
            "trigger_target": {"target_slot"},
            "synthetic_assembly": {"stage"},
        }
        if kind not in fields:
            raise WireError("decode.invalid_json", "unknown DecisionPurposeV4")
        obj = require_exact_keys(value, {"kind", *fields[kind]})
        kwargs: dict[str, Any] = {"kind": kind}
        for field in ("mode_slot", "target_slot", "cost_slot", "count", "profile_local_cost_id"):
            if field in obj:
                kwargs[field] = _u32(obj[field], field)
        for field in ("operation", "counter_kind", "stage"):
            if field in obj:
                kwargs[field] = obj[field]
        result = cls(**kwargs)
        result.validate()
        return result

    def validate(self) -> None:
        if self.kind not in _PURPOSE_CANDIDATES:
            raise WireError("semantic.decision", "unknown DecisionPurposeV4")
        if self.mode_slot is not None:
            _u32(self.mode_slot, "mode_slot")
        if self.target_slot is not None:
            _u32(self.target_slot, "target_slot")
        if self.cost_slot is not None:
            _u32(self.cost_slot, "cost_slot")
        if self.count is not None:
            _u32(self.count, "count")
        if self.profile_local_cost_id is not None:
            _u32(self.profile_local_cost_id, "profile_local_cost_id")
        if self.kind == "cost_operand_selection" and (
            self.operation != "put_counters"
            or self.counter_kind not in _COUNTER_RANK
            or self.count is None
        ):
            raise WireError("semantic.decision", "invalid cost operand purpose")
        if self.kind in {"mode_selection", "target_selection", "trigger_target"}:
            slot = self.mode_slot if self.kind == "mode_selection" else self.target_slot
            if slot is None:
                raise WireError("semantic.decision", "decision slot is absent")
        if self.kind == "optional_cost_payment" and self.profile_local_cost_id is None:
            raise WireError("semantic.decision", "optional cost ID is absent")
        if self.kind == "synthetic_assembly" and self.stage not in {
            "entry",
            "choose_count",
            "choose_members",
            "order_members",
        }:
            raise WireError("semantic.decision", "unknown synthetic assembly stage")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        all_payload_fields = {
            "mode_slot",
            "target_slot",
            "cost_slot",
            "operation",
            "counter_kind",
            "count",
            "profile_local_cost_id",
            "stage",
        }
        present_fields = {field for field in all_payload_fields if getattr(self, field) is not None}
        allowed_fields = {
            "priority_action": set(),
            "attacker_declaration": set(),
            "sba_graveyard_order": set(),
            "cast_cost_route": set(),
            "mode_selection": {"mode_slot"},
            "target_selection": {"target_slot"},
            "cost_operand_selection": {
                "cost_slot",
                "operation",
                "counter_kind",
                "count",
            },
            "mana_production_choice": set(),
            "mana_payment": set(),
            "optional_cost_payment": {"profile_local_cost_id"},
            "ability_action": set(),
            "trigger_order": set(),
            "trigger_target": {"target_slot"},
            "synthetic_assembly": {"stage"},
        }[self.kind]
        if present_fields != allowed_fields:
            raise WireError("encode.serialization", "decision purpose has missing or extra fields")
        result: dict[str, object] = {"kind": self.kind}
        for field in (
            "mode_slot",
            "target_slot",
            "cost_slot",
            "operation",
            "counter_kind",
            "count",
            "profile_local_cost_id",
            "stage",
        ):
            value = getattr(self, field)
            if value is not None:
                result[field] = value
        return result


@dataclass(frozen=True, slots=True)
class PlayerDecisionRequestV4:
    schema_version: str
    player_decision_id: int
    view_sequence: int
    actor: int
    visibility: str
    decision_domain_v2: DecisionSpec
    purpose: DecisionPurposeV4
    parent_player_decision_id: int | None
    candidates: tuple[VisibleCandidateV4, ...]

    @classmethod
    def from_wire(cls, value: object) -> PlayerDecisionRequestV4:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "player_decision_id",
                "view_sequence",
                "actor",
                "visibility",
                "decision_domain_v2",
                "purpose",
                "parent_player_decision_id",
                "candidates",
            },
        )
        if (
            obj["schema_version"] != PLAYER_DECISION_REQUEST_V4_SCHEMA
            or obj["visibility"] not in _VISIBILITY
        ):
            raise WireError("decode.invalid_json", "unsupported request V4 schema or visibility")
        result = cls(
            PLAYER_DECISION_REQUEST_V4_SCHEMA,
            parse_uint(obj["player_decision_id"]),
            parse_uint(obj["view_sequence"]),
            parse_uint(obj["actor"]),
            str(obj["visibility"]),
            DecisionSpec.from_wire(obj["decision_domain_v2"]),
            DecisionPurposeV4.from_wire(obj["purpose"]),
            _nullable_u64(obj["parent_player_decision_id"], "parent_player_decision_id"),
            tuple(
                VisibleCandidateV4.from_wire(item)
                for item in _array(obj["candidates"], "candidates")
            ),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if (
            self.schema_version != PLAYER_DECISION_REQUEST_V4_SCHEMA
            or self.visibility not in _VISIBILITY
        ):
            raise WireError("semantic.decision", "unsupported request V4 identity or visibility")
        uint_wire(self.player_decision_id)
        uint_wire(self.view_sequence)
        uint_wire(self.actor)
        _nullable_wire(self.parent_player_decision_id)
        self.purpose.validate()
        if self.decision_domain_v2.kind not in _PURPOSE_DOMAINS[self.purpose.kind]:
            raise WireError("semantic.decision", "decision domain is incompatible with purpose")
        if (
            self.purpose.kind
            in {
                "trigger_order",
                "attacker_declaration",
                "sba_graveyard_order",
                "cast_cost_route",
            }
            and self.visibility != "acting_player_only"
        ):
            raise WireError("semantic.decision", "private decision request is not actor-only")
        if self.purpose.kind == "synthetic_assembly":
            if self.visibility != "public":
                raise WireError("semantic.decision", "synthetic assembly request is not public")
            stage = self.purpose.stage
            if stage is None:
                raise WireError("semantic.decision", "synthetic assembly stage is absent")
            stage_domain = {
                "entry": "choose_one",
                "choose_count": "choose_number",
                "choose_members": "choose_many",
                "order_members": "order",
            }[stage]
            if self.decision_domain_v2.kind != stage_domain:
                raise WireError("semantic.decision", "synthetic assembly stage/domain mismatch")
            if stage_domain == "choose_number" and self.candidates:
                raise WireError(
                    "semantic.decision", "synthetic count decision cannot have candidates"
                )
            if stage_domain != "choose_number" and any(
                candidate.intent.kind != "select_object" for candidate in self.candidates
            ):
                raise WireError(
                    "semantic.decision",
                    "synthetic assembly candidate must select an object",
                )
        self.decision_domain_v2.validate(len(self.candidates))
        if self.decision_domain_v2.kind == "choose_number" and self.candidates:
            raise WireError("semantic.decision", "choose_number cannot contain candidates")
        ids = tuple(candidate.candidate_id for candidate in self.candidates)
        if ids != tuple(range(len(ids))):
            raise WireError("semantic.decision", "candidate IDs must be dense from zero")
        for candidate in self.candidates:
            _u32(candidate.candidate_id, "candidate_id")
            candidate.intent.to_wire()
        keys = tuple(candidate.intent.ordering_key() for candidate in self.candidates)
        if any(left >= right for left, right in pairwise(keys)):
            raise WireError("semantic.decision", "candidate ordering is not canonical")
        allowed_intents = _PURPOSE_CANDIDATES[self.purpose.kind]
        if any(candidate.intent.kind not in allowed_intents for candidate in self.candidates):
            raise WireError("semantic.decision", "candidate intent is incompatible with purpose")
        ability_sources: dict[int, int] = {}
        for candidate in self.candidates:
            intent = candidate.intent
            if intent.kind != "select_trigger" or intent.trigger is None:
                continue
            descriptor = intent.trigger
            pairs: list[tuple[int | None, int | None]] = [
                (descriptor.source_ability, descriptor.source_object)
            ]
            if isinstance(descriptor.subject, AbilityActivatedSubjectV1):
                pairs.append(
                    (
                        descriptor.subject.activated_source_ability,
                        descriptor.subject.activated_source_object,
                    )
                )
            for ability_id, object_id in pairs:
                if ability_id is None or object_id is None:
                    continue
                previous = ability_sources.setdefault(ability_id, object_id)
                if previous != object_id:
                    raise WireError(
                        "semantic.decision",
                        "opaque ability identity maps to conflicting source objects",
                    )

    def validate_response(self, response: DecisionResponseV3) -> None:
        response.validate()
        self.validate()
        if (
            response.player_decision_id != self.player_decision_id
            or response.view_sequence != self.view_sequence
        ):
            raise WireError(
                "semantic.decision_response", "request identity or view sequence mismatch"
            )
        ids = {candidate.candidate_id for candidate in self.candidates}
        answer = response.answer
        expected = {
            "choose_one": "select_one",
            "choose_many": "select_many",
            "choose_number": "choose_number",
            "order": "order",
        }[self.decision_domain_v2.kind]
        if answer.kind != expected:
            raise WireError("semantic.decision_response", "answer domain mismatch")
        if answer.kind == "select_one" and answer.candidate_id not in ids:
            raise WireError("semantic.decision_response", "unknown candidate")
        if answer.kind in {"select_many", "order"}:
            selected = answer.candidate_ids
            if any(candidate_id not in ids for candidate_id in selected) or len(
                set(selected)
            ) != len(selected):
                raise WireError("semantic.decision_response", "unknown or duplicate candidate")
            if answer.kind == "select_many" and any(a >= b for a, b in pairwise(selected)):
                raise WireError("semantic.decision_response", "SelectMany IDs are not ascending")
            minimum = self.decision_domain_v2.minimum or 0
            maximum = self.decision_domain_v2.maximum
            if maximum is not None and not minimum <= len(selected) <= maximum:
                raise WireError("semantic.decision_response", "answer cardinality is out of bounds")
        if answer.kind == "choose_number":
            number_minimum = self.decision_domain_v2.minimum
            number_maximum = self.decision_domain_v2.maximum
            if (
                number_minimum is None
                or number_maximum is None
                or answer.value is None
                or not number_minimum <= answer.value <= number_maximum
            ):
                raise WireError("semantic.decision_response", "numeric answer is out of bounds")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "actor": uint_wire(self.actor),
            "candidates": [item.to_wire() for item in self.candidates],
            "decision_domain_v2": self.decision_domain_v2.to_wire(),
            "parent_player_decision_id": _nullable_wire(self.parent_player_decision_id),
            "player_decision_id": uint_wire(self.player_decision_id),
            "purpose": self.purpose.to_wire(),
            "schema_version": self.schema_version,
            "view_sequence": uint_wire(self.view_sequence),
            "visibility": self.visibility,
        }
