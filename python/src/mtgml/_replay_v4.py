from __future__ import annotations

from dataclasses import dataclass

from .canonical import (
    parse_u64_number,
    require_exact_keys,
    require_nonempty,
)
from .errors import WireError


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
