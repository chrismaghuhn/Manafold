from __future__ import annotations

from dataclasses import dataclass

from .canonical import parse_uint, require_digest, require_exact_keys, require_nonempty, uint_wire


@dataclass(frozen=True, slots=True)
class KernelIdentityV1:
    implementation_id: str
    semantic_version: str
    build_profile: str

    @classmethod
    def from_wire(cls, value: object) -> KernelIdentityV1:
        obj = require_exact_keys(value, {"implementation_id", "semantic_version", "build_profile"})
        return cls(
            *(
                require_nonempty(obj[key], key)
                for key in ("implementation_id", "semantic_version", "build_profile")
            )
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "build_profile": require_nonempty(self.build_profile, "build_profile"),
            "implementation_id": require_nonempty(self.implementation_id, "implementation_id"),
            "semantic_version": require_nonempty(self.semantic_version, "semantic_version"),
        }


@dataclass(frozen=True, slots=True)
class DeckIdentityV1:
    player: int
    deck_id: str
    digest: str

    @classmethod
    def from_wire(cls, value: object) -> DeckIdentityV1:
        obj = require_exact_keys(value, {"player", "deck_id", "digest"})
        return cls(
            parse_uint(obj["player"]),
            obj["deck_id"],  # Allow empty for now; validated at manifest level
            require_digest(obj["digest"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "deck_id": require_nonempty(self.deck_id, "deck_id"),
            "digest": require_digest(self.digest),
            "player": uint_wire(self.player),
        }
