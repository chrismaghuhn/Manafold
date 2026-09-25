from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from ._magic_observation import MagicPendingSbaOrdering
from ._synthetic_observation import SyntheticPriority, SyntheticTurnPosition
from .canonical import parse_uint, require_exact_keys, uint_wire
from .errors import WireError

MAGIC_OBSERVATION_SCHEMA_V4 = "magic-combat-observation.v4"


class MagicBlockedStatusV4(str, Enum):
    BLOCKED = "blocked"
    UNBLOCKED = "unblocked"


@dataclass(frozen=True, slots=True)
class MagicPlayerLifeV4:
    player: int
    life: int
    has_lost: bool

    @classmethod
    def from_wire(cls, value: object) -> MagicPlayerLifeV4:
        obj = require_exact_keys(value, {"player", "life", "has_lost"})
        life = obj["life"]
        if (
            isinstance(life, bool)
            or not isinstance(life, int)
            or not -(2**63) <= life < 2**63
            or not isinstance(obj["has_lost"], bool)
            or (obj["has_lost"] and life > 0)
        ):
            raise WireError("decode.invalid_json", "life total must be a signed 64-bit integer")
        return cls(parse_uint(obj["player"]), life, obj["has_lost"])

    def to_wire(self) -> dict[str, object]:
        if not -(2**63) <= self.life < 2**63:
            raise WireError("semantic.magic_combat_observation_v4", "life total is out of range")
        if self.has_lost and self.life > 0:
            raise WireError("semantic.magic_combat_observation_v4", "lost player has positive life")
        return {"player": uint_wire(self.player), "life": self.life, "has_lost": self.has_lost}


@dataclass(frozen=True, slots=True)
class MagicMarkedDamageV4:
    creature: int
    amount: str

    @classmethod
    def from_wire(cls, value: object) -> MagicMarkedDamageV4:
        obj = require_exact_keys(value, {"creature", "amount"})
        amount = obj["amount"]
        if not isinstance(amount, str) or parse_uint(amount) == 0:
            raise WireError(
                "semantic.magic_combat_observation_v4", "marked damage must be positive"
            )
        return cls(parse_uint(obj["creature"]), amount)

    def to_wire(self) -> dict[str, object]:
        if parse_uint(self.amount) == 0:
            raise WireError(
                "semantic.magic_combat_observation_v4", "marked damage must be positive"
            )
        return {"creature": uint_wire(self.creature), "amount": self.amount}


@dataclass(frozen=True, slots=True)
class MagicCombatBlockerAssignmentV4:
    attacker: int
    status: MagicBlockedStatusV4
    blocker: int | None

    @classmethod
    def from_wire(cls, value: object) -> MagicCombatBlockerAssignmentV4:
        obj = require_exact_keys(value, {"attacker", "status", "blocker"})
        try:
            status = MagicBlockedStatusV4(obj["status"])
        except (TypeError, ValueError):
            raise WireError(
                "semantic.magic_combat_observation_v4", "unknown blocked status"
            ) from None
        blocker = None if obj["blocker"] is None else parse_uint(obj["blocker"])
        return cls(parse_uint(obj["attacker"]), status, blocker)

    def to_wire(self) -> dict[str, object]:
        if not isinstance(self.status, MagicBlockedStatusV4):
            raise WireError("semantic.magic_combat_observation_v4", "unknown blocked status")
        if self.status == MagicBlockedStatusV4.UNBLOCKED and self.blocker is not None:
            raise WireError("semantic.magic_combat_observation_v4", "invalid blocker relation")
        return {
            "attacker": uint_wire(self.attacker),
            "status": self.status.value,
            "blocker": None if self.blocker is None else uint_wire(self.blocker),
        }


@dataclass(frozen=True, slots=True)
class MagicCombatParticipationV4:
    defending_player: int
    attackers: tuple[int, ...]
    blockers: tuple[MagicCombatBlockerAssignmentV4, ...]

    @classmethod
    def from_wire(cls, value: object) -> MagicCombatParticipationV4:
        obj = require_exact_keys(value, {"defending_player", "attackers", "blockers"})
        if not isinstance(obj["attackers"], list) or not isinstance(obj["blockers"], list):
            raise WireError("decode.invalid_json", "combat members must be arrays")
        result = cls(
            parse_uint(obj["defending_player"]),
            tuple(parse_uint(item) for item in obj["attackers"]),
            tuple(MagicCombatBlockerAssignmentV4.from_wire(item) for item in obj["blockers"]),
        )
        result.to_wire()
        return result

    def to_wire(self) -> dict[str, object]:
        assigned = tuple(entry.blocker for entry in self.blockers if entry.blocker is not None)
        if (
            tuple(sorted(set(self.attackers))) != self.attackers
            or len(self.blockers) != len(self.attackers)
            or any(
                entry.attacker != attacker
                for entry, attacker in zip(self.blockers, self.attackers, strict=True)
            )
            or len(set(assigned)) != len(assigned)
            or bool(set(assigned).intersection(self.attackers))
        ):
            raise WireError("semantic.magic_combat_observation_v4", "invalid combat participation")
        return {
            "defending_player": uint_wire(self.defending_player),
            "attackers": [uint_wire(item) for item in self.attackers],
            "blockers": [entry.to_wire() for entry in self.blockers],
        }


@dataclass(frozen=True, slots=True)
class MagicObservationV4:
    schema_version: str
    active_player: int
    turn_number: str
    turn_position: SyntheticTurnPosition
    priority: SyntheticPriority
    player_life: tuple[MagicPlayerLifeV4, ...]
    marked_damage: tuple[MagicMarkedDamageV4, ...]
    pending_sba_ordering: MagicPendingSbaOrdering | None
    combat: MagicCombatParticipationV4 | None

    @classmethod
    def from_wire(cls, value: object) -> MagicObservationV4:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "active_player",
                "turn_number",
                "turn_position",
                "priority",
                "player_life",
                "marked_damage",
                "pending_sba_ordering",
                "combat",
            },
        )
        if obj["schema_version"] != MAGIC_OBSERVATION_SCHEMA_V4:
            raise WireError(
                "semantic.magic_combat_observation_v4", "unsupported observation schema"
            )
        if not isinstance(obj["turn_number"], str):
            raise WireError("decode.invalid_json", "turn number must be a string")
        if not isinstance(obj["player_life"], list) or not isinstance(obj["marked_damage"], list):
            raise WireError("decode.invalid_json", "life and marked damage must be arrays")
        pending = obj["pending_sba_ordering"]
        combat = obj["combat"]
        result = cls(
            MAGIC_OBSERVATION_SCHEMA_V4,
            parse_uint(obj["active_player"]),
            obj["turn_number"],
            SyntheticTurnPosition.from_wire(obj["turn_position"]),
            SyntheticPriority.from_wire(obj["priority"]),
            tuple(MagicPlayerLifeV4.from_wire(item) for item in obj["player_life"]),
            tuple(MagicMarkedDamageV4.from_wire(item) for item in obj["marked_damage"]),
            None if pending is None else MagicPendingSbaOrdering.from_wire(pending),
            None if combat is None else MagicCombatParticipationV4.from_wire(combat),
        )
        result.to_wire()
        return result

    def to_wire(self) -> dict[str, object]:
        if self.schema_version != MAGIC_OBSERVATION_SCHEMA_V4:
            raise WireError(
                "semantic.magic_combat_observation_v4", "unsupported observation schema"
            )
        uint_wire(self.active_player)
        parse_uint(self.turn_number)
        if (
            len(self.player_life) != 2
            or tuple(sorted(item.player for item in self.player_life))
            != tuple(item.player for item in self.player_life)
            or len({item.player for item in self.player_life}) != 2
            or tuple(sorted(item.creature for item in self.marked_damage))
            != tuple(item.creature for item in self.marked_damage)
            or len({item.creature for item in self.marked_damage}) != len(self.marked_damage)
        ):
            raise WireError("semantic.magic_combat_observation_v4", "noncanonical public state")
        if self.combat is not None and self.combat.defending_player == self.active_player:
            raise WireError("semantic.magic_combat_observation_v4", "invalid defending player")
        return {
            "schema_version": MAGIC_OBSERVATION_SCHEMA_V4,
            "active_player": uint_wire(self.active_player),
            "turn_number": self.turn_number,
            "turn_position": self.turn_position.to_wire(),
            "priority": self.priority.to_wire(),
            "player_life": [item.to_wire() for item in self.player_life],
            "marked_damage": [item.to_wire() for item in self.marked_damage],
            "pending_sba_ordering": None
            if self.pending_sba_ordering is None
            else self.pending_sba_ordering.to_wire(),
            "combat": None if self.combat is None else self.combat.to_wire(),
        }
