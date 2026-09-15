from __future__ import annotations

from dataclasses import dataclass

from .canonical import parse_uint, require_digest, require_exact_keys, require_nonempty, uint_wire
from .errors import WireError


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
class ReplaySchemaVersionsV1:
    observation: str
    information_state: str
    decision: str
    decision_response: str
    observed_event: str
    player_step: str
    replay_step: str

    @classmethod
    def from_wire(cls, value: object) -> ReplaySchemaVersionsV1:
        keys = {
            "observation",
            "information_state",
            "decision",
            "decision_response",
            "observed_event",
            "player_step",
            "replay_step",
        }
        obj = require_exact_keys(value, keys)
        return cls(**{key: require_nonempty(obj[key], key) for key in keys})

    def to_wire(self) -> dict[str, object]:
        return {
            key: require_nonempty(getattr(self, key), key)
            for key in (
                "decision",
                "decision_response",
                "information_state",
                "observation",
                "observed_event",
                "player_step",
                "replay_step",
            )
        }


@dataclass(frozen=True, slots=True)
class RandomnessIdentityV1:
    algorithm_id: str
    derivation_version: str
    root_seed_hex: str

    @classmethod
    def from_wire(cls, value: object) -> RandomnessIdentityV1:
        obj = require_exact_keys(value, {"algorithm_id", "derivation_version", "root_seed_hex"})
        seed = obj["root_seed_hex"]
        if (
            not isinstance(seed, str)
            or len(seed) != 64
            or any(ch not in "0123456789abcdef" for ch in seed)
        ):
            raise WireError("semantic.replay_manifest", "root seed is not canonical hex")
        return cls(
            require_nonempty(obj["algorithm_id"], "algorithm_id"),
            require_nonempty(obj["derivation_version"], "derivation_version"),
            seed,
        )

    def to_wire(self) -> dict[str, object]:
        return RandomnessIdentityV1.from_wire(
            {
                "algorithm_id": self.algorithm_id,
                "derivation_version": self.derivation_version,
                "root_seed_hex": self.root_seed_hex,
            }
        )._raw()

    def _raw(self) -> dict[str, object]:
        return {
            "algorithm_id": self.algorithm_id,
            "derivation_version": self.derivation_version,
            "root_seed_hex": self.root_seed_hex,
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
