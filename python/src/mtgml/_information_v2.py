from __future__ import annotations

from dataclasses import dataclass

from ._knowledge import PlayerKnownObjectV1
from ._observation_v1 import ObservationEnvelope
from .canonical import parse_uint, require_digest, require_exact_keys, uint_wire
from .errors import WireError

INFORMATION_STATE_SCHEMA_V2 = "information-state-envelope.v2"


@dataclass(frozen=True, slots=True)
class InformationStateDigestInputV2:
    schema_version: str
    perspective: int
    state_revision: int
    current_observation: ObservationEnvelope
    next_visible_sequence: int
    retained_knowledge: tuple[PlayerKnownObjectV1, ...]

    @classmethod
    def from_wire(cls, value: object) -> InformationStateDigestInputV2:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "perspective",
                "state_revision",
                "current_observation",
                "next_visible_sequence",
                "retained_knowledge",
            },
        )
        if obj["schema_version"] != "information-state-digest-input.v2" or not isinstance(
            obj["retained_knowledge"], list
        ):
            raise WireError("decode.invalid_json", "unsupported information digest input")
        return cls(
            "information-state-digest-input.v2",
            parse_uint(obj["perspective"]),
            parse_uint(obj["state_revision"]),
            ObservationEnvelope.from_wire(obj["current_observation"]),
            parse_uint(obj["next_visible_sequence"]),
            tuple(PlayerKnownObjectV1.from_wire(item) for item in obj["retained_knowledge"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "current_observation": self.current_observation.to_wire(),
            "next_visible_sequence": uint_wire(self.next_visible_sequence),
            "perspective": uint_wire(self.perspective),
            "retained_knowledge": [item.to_wire() for item in self.retained_knowledge],
            "schema_version": "information-state-digest-input.v2",
            "state_revision": uint_wire(self.state_revision),
        }


@dataclass(frozen=True, slots=True)
class PlayerInformationStateV2:
    schema_version: str
    perspective: int
    state_revision: int
    current_observation: ObservationEnvelope
    next_visible_sequence: int
    retained_knowledge: tuple[PlayerKnownObjectV1, ...]
    digest: str

    @classmethod
    def from_wire(cls, value: object) -> PlayerInformationStateV2:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "perspective",
                "state_revision",
                "current_observation",
                "next_visible_sequence",
                "retained_knowledge",
                "digest",
            },
        )
        if obj["schema_version"] != INFORMATION_STATE_SCHEMA_V2 or not isinstance(
            obj["retained_knowledge"], list
        ):
            raise WireError("decode.invalid_json", "unsupported information-state V2")
        result = cls(
            INFORMATION_STATE_SCHEMA_V2,
            parse_uint(obj["perspective"]),
            parse_uint(obj["state_revision"]),
            ObservationEnvelope.from_wire(obj["current_observation"]),
            parse_uint(obj["next_visible_sequence"]),
            tuple(PlayerKnownObjectV1.from_wire(item) for item in obj["retained_knowledge"]),
            require_digest(obj["digest"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if (
            self.current_observation.perspective != self.perspective
            or self.current_observation.state_revision != self.state_revision
        ):
            raise WireError("semantic.information_state", "observation identity differs")
        ids = [item.opaque_object_id for item in self.retained_knowledge]
        if ids != sorted(ids) or len(ids) != len(set(ids)):
            raise WireError("semantic.information_state", "retained knowledge is not canonical")
        for item in self.retained_knowledge:
            item.validate(self.next_visible_sequence)
        from .wire import compute_information_state_digest_v2

        _, expected = compute_information_state_digest_v2(self.digest_input())
        if self.digest != expected:
            raise WireError(
                "semantic.information_state",
                "information-state digest does not match its semantic payload",
            )

    def digest_input(self) -> InformationStateDigestInputV2:
        return InformationStateDigestInputV2(
            "information-state-digest-input.v2",
            self.perspective,
            self.state_revision,
            self.current_observation,
            self.next_visible_sequence,
            self.retained_knowledge,
        )

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "current_observation": self.current_observation.to_wire(),
            "digest": require_digest(self.digest),
            "next_visible_sequence": uint_wire(self.next_visible_sequence),
            "perspective": uint_wire(self.perspective),
            "retained_knowledge": [item.to_wire() for item in self.retained_knowledge],
            "schema_version": INFORMATION_STATE_SCHEMA_V2,
            "state_revision": uint_wire(self.state_revision),
        }
