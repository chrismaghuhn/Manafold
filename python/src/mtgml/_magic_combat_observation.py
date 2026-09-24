from __future__ import annotations

from dataclasses import dataclass

from ._magic_observation import MagicPendingSbaOrdering
from ._synthetic_observation import SyntheticPriority, SyntheticTurnPosition
from .canonical import parse_uint, require_exact_keys, uint_wire
from .errors import WireError

MAGIC_OBSERVATION_SCHEMA_V2 = "magic-combat-observation.v2"


@dataclass(frozen=True, slots=True)
class MagicCombatBlockerAssignmentV2:
    attacker: int
    blocker: int | None

    @classmethod
    def from_wire(cls, value: object) -> MagicCombatBlockerAssignmentV2:
        obj = require_exact_keys(value, {"attacker", "blocker"})
        blocker = None if obj["blocker"] is None else parse_uint(obj["blocker"])
        return cls(parse_uint(obj["attacker"]), blocker)

    def to_wire(self) -> dict[str, object]:
        return {
            "attacker": uint_wire(self.attacker),
            "blocker": None if self.blocker is None else uint_wire(self.blocker),
        }


@dataclass(frozen=True, slots=True)
class MagicCombatParticipationV2:
    defending_player: int
    attackers: tuple[int, ...]
    blockers: tuple[MagicCombatBlockerAssignmentV2, ...]

    @classmethod
    def from_wire(cls, value: object) -> MagicCombatParticipationV2:
        obj = require_exact_keys(value, {"defending_player", "attackers", "blockers"})
        if not isinstance(obj["attackers"], list) or not isinstance(obj["blockers"], list):
            raise WireError("decode.invalid_json", "combat members must be arrays")
        result = cls(
            parse_uint(obj["defending_player"]),
            tuple(parse_uint(item) for item in obj["attackers"]),
            tuple(MagicCombatBlockerAssignmentV2.from_wire(item) for item in obj["blockers"]),
        )
        result.to_wire()
        return result

    def to_wire(self) -> dict[str, object]:
        if (
            tuple(sorted(set(self.attackers))) != self.attackers
            or len(self.blockers) != len(self.attackers)
            or any(
                assignment.attacker != attacker or assignment.blocker is not None
                for assignment, attacker in zip(self.blockers, self.attackers)
            )
        ):
            raise WireError("semantic.magic_combat_observation", "invalid combat participation")
        return {
            "defending_player": uint_wire(self.defending_player),
            "attackers": [uint_wire(item) for item in self.attackers],
            "blockers": [item.to_wire() for item in self.blockers],
        }


@dataclass(frozen=True, slots=True)
class MagicObservationV2:
    schema_version: str
    active_player: int
    turn_number: str
    turn_position: SyntheticTurnPosition
    priority: SyntheticPriority
    pending_sba_ordering: MagicPendingSbaOrdering | None
    combat: MagicCombatParticipationV2 | None

    @classmethod
    def from_wire(cls, value: object) -> MagicObservationV2:
        obj = require_exact_keys(
            value,
            {
                "schema_version", "active_player", "turn_number", "turn_position",
                "priority", "pending_sba_ordering", "combat",
            },
        )
        if obj["schema_version"] != MAGIC_OBSERVATION_SCHEMA_V2:
            raise WireError("semantic.magic_combat_observation", "unsupported observation schema")
        turn_number = obj["turn_number"]
        if not isinstance(turn_number, str):
            raise WireError("decode.invalid_json", "turn number must be a string")
        parse_uint(turn_number)
        pending = obj["pending_sba_ordering"]
        combat = obj["combat"]
        result = cls(
            MAGIC_OBSERVATION_SCHEMA_V2,
            parse_uint(obj["active_player"]),
            turn_number,
            SyntheticTurnPosition.from_wire(obj["turn_position"]),
            SyntheticPriority.from_wire(obj["priority"]),
            None if pending is None else MagicPendingSbaOrdering.from_wire(pending),
            None if combat is None else MagicCombatParticipationV2.from_wire(combat),
        )
        result.to_wire()
        return result

    def to_wire(self) -> dict[str, object]:
        if self.schema_version != MAGIC_OBSERVATION_SCHEMA_V2:
            raise WireError("semantic.magic_combat_observation", "unsupported observation schema")
        uint_wire(self.active_player)
        parse_uint(self.turn_number)
        if self.combat is not None and (
            self.combat.defending_player == self.active_player
            or self.turn_position.kind != "combat"
            or self.turn_position.step not in {"declare_attackers", "end_of_combat"}
            or (self.turn_position.step == "end_of_combat" and self.combat.attackers)
        ):
            raise WireError("semantic.magic_combat_observation", "combat does not match turn position")
        result = {
            "schema_version": MAGIC_OBSERVATION_SCHEMA_V2,
            "active_player": uint_wire(self.active_player),
            "turn_number": self.turn_number,
            "turn_position": self.turn_position.to_wire(),
            "priority": self.priority.to_wire(),
            "pending_sba_ordering": None
            if self.pending_sba_ordering is None
            else self.pending_sba_ordering.to_wire(),
            "combat": None if self.combat is None else self.combat.to_wire(),
        }
        return result
