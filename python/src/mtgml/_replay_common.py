from __future__ import annotations

from dataclasses import dataclass

from .canonical import (
    parse_u64_number,
    parse_uint,
    require_digest,
    require_exact_keys,
    require_nonempty,
    uint_wire,
)
from .errors import WireError
from .persistence import _VALID_PROGRAM_KINDS


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


@dataclass(frozen=True, slots=True)
class RandomnessIdentityV2:
    contract_id: str
    root_seed_hex: str

    @classmethod
    def from_wire(cls, value: object) -> RandomnessIdentityV2:
        obj = require_exact_keys(value, {"contract_id", "root_seed_hex"})
        return cls(
            require_nonempty(obj["contract_id"], "contract_id"),
            require_digest(obj["root_seed_hex"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "contract_id": require_nonempty(self.contract_id, "contract_id"),
            "root_seed_hex": require_digest(self.root_seed_hex),
        }


@dataclass(frozen=True, slots=True)
class EnvironmentLimitCountersV4:
    decisions_submitted: int
    accepted_transitions: int
    rule_events_emitted: int
    resource_units_consumed: int
    wall_clock_elapsed_millis: int

    @classmethod
    def from_wire(cls, value: object) -> EnvironmentLimitCountersV4:
        obj = require_exact_keys(
            value,
            {
                "decisions_submitted",
                "accepted_transitions",
                "rule_events_emitted",
                "resource_units_consumed",
                "wall_clock_elapsed_millis",
            },
        )
        return cls(
            parse_u64_number(obj["decisions_submitted"]),
            parse_u64_number(obj["accepted_transitions"]),
            parse_u64_number(obj["rule_events_emitted"]),
            parse_u64_number(obj["resource_units_consumed"]),
            parse_u64_number(obj["wall_clock_elapsed_millis"]),
        )

    def to_wire(self) -> dict[str, object]:
        values: dict[str, object] = {
            "decisions_submitted": self.decisions_submitted,
            "accepted_transitions": self.accepted_transitions,
            "rule_events_emitted": self.rule_events_emitted,
            "resource_units_consumed": self.resource_units_consumed,
            "wall_clock_elapsed_millis": self.wall_clock_elapsed_millis,
        }
        for name, value in values.items():
            if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**64 - 1:
                raise WireError("encode.serialization", f"{name} is outside u64")
        return values

    def as_dict(self) -> dict[str, int]:
        return {
            "decisions_submitted": self.decisions_submitted,
            "accepted_transitions": self.accepted_transitions,
            "rule_events_emitted": self.rule_events_emitted,
            "resource_units_consumed": self.resource_units_consumed,
            "wall_clock_elapsed_millis": self.wall_clock_elapsed_millis,
        }


@dataclass(frozen=True, slots=True)
class CheckpointCodecIdentityV4:
    codec_id: str
    semantic_version: str

    @classmethod
    def from_wire(cls, value: object) -> CheckpointCodecIdentityV4:
        obj = require_exact_keys(value, {"codec_id", "semantic_version"})
        return cls(
            require_nonempty(obj["codec_id"], "codec_id"),
            require_nonempty(obj["semantic_version"], "semantic_version"),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "codec_id": require_nonempty(self.codec_id, "codec_id"),
            "semantic_version": require_nonempty(self.semantic_version, "semantic_version"),
        }


@dataclass(frozen=True, slots=True)
class ExecutionIdentityV1:
    program_kind: str
    semantic_contract_id: str

    @classmethod
    def from_wire(cls, value: object) -> ExecutionIdentityV1:
        obj = require_exact_keys(value, {"program_kind", "semantic_contract_id"})
        result = cls(
            program_kind=require_nonempty(obj["program_kind"], "program_kind"),
            semantic_contract_id=require_digest(obj["semantic_contract_id"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        require_nonempty(self.program_kind, "program_kind")
        if self.program_kind not in _VALID_PROGRAM_KINDS:
            raise WireError("decode.invalid_json", "unknown execution program kind")
        require_digest(self.semantic_contract_id)

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "program_kind": self.program_kind,
            "semantic_contract_id": self.semantic_contract_id,
        }
