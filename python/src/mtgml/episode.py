from __future__ import annotations

from dataclasses import dataclass

from ._generated_contract_vocab import PlayerResult as PlayerResult
from ._generated_contract_vocab import TerminalReason as TerminalReason
from ._generated_contract_vocab import TruncationReason as TruncationReason
from .canonical import parse_uint, require_exact_keys, uint_wire
from .errors import WireError


@dataclass(frozen=True, slots=True)
class PlayerOutcome:
    player: int
    result: PlayerResult

    @classmethod
    def from_wire(cls, value: object) -> PlayerOutcome:
        obj = require_exact_keys(value, {"player", "result"})
        try:
            result = PlayerResult(obj["result"])
        except (TypeError, ValueError) as exc:
            raise WireError("decode.invalid_json", "unknown player result") from exc
        return cls(parse_uint(obj["player"]), result)

    def to_wire(self) -> dict[str, object]:
        return {"player": uint_wire(self.player), "result": self.result.value}


@dataclass(frozen=True, slots=True)
class EpisodeStatus:
    kind: str
    reason: TerminalReason | TruncationReason | None = None
    players: tuple[PlayerOutcome, ...] = ()

    @classmethod
    def running(cls) -> EpisodeStatus:
        return cls("running")

    @classmethod
    def from_wire(cls, value: object) -> EpisodeStatus:
        if not isinstance(value, dict):
            raise WireError("decode.invalid_json", "episode status must be an object")
        kind = value.get("kind")
        if kind == "running":
            require_exact_keys(value, {"kind"})
            return cls.running()
        obj = require_exact_keys(value, {"kind", "reason", "players"})
        if not isinstance(obj["players"], list):
            raise WireError("decode.invalid_json", "players must be an array")
        players = tuple(PlayerOutcome.from_wire(item) for item in obj["players"])
        if len({outcome.player for outcome in players}) != len(players):
            raise WireError("semantic.episode_status", "duplicate player outcome")
        try:
            if kind == "terminal":
                reason: TerminalReason | TruncationReason = TerminalReason(obj["reason"])
            elif kind == "truncated":
                reason = TruncationReason(obj["reason"])
            else:
                raise ValueError
        except (TypeError, ValueError) as exc:
            raise WireError("decode.invalid_json", "unknown episode status or reason") from exc
        return cls(str(kind), reason, players)

    def to_wire(self) -> dict[str, object]:
        if self.kind == "running":
            if self.reason is not None or self.players:
                raise WireError("semantic.episode_status", "running status has terminal fields")
            return {"kind": "running"}
        if self.kind not in {"terminal", "truncated"} or self.reason is None:
            raise WireError("semantic.episode_status", "invalid episode status")
        if self.kind == "terminal" and not isinstance(self.reason, TerminalReason):
            raise WireError("semantic.episode_status", "terminal status has truncation reason")
        if self.kind == "truncated" and not isinstance(self.reason, TruncationReason):
            raise WireError("semantic.episode_status", "truncated status has terminal reason")
        if len({outcome.player for outcome in self.players}) != len(self.players):
            raise WireError("semantic.episode_status", "duplicate player outcome")
        return {
            "kind": self.kind,
            "players": [outcome.to_wire() for outcome in self.players],
            "reason": self.reason.value,
        }
