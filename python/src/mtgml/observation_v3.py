"""Detached perspective-safe observation and information-state V2/V3 DTOs."""

from __future__ import annotations

import base64
import hashlib
from dataclasses import dataclass

from ._knowledge import PlayerKnownObjectV1
from ._observation_v1 import observation_digest_from_payload
from .canonical import (
    canonical_json_bytes,
    parse_uint,
    require_canonical_base64,
    require_digest,
    require_exact_keys,
    require_nonempty,
    uint_wire,
)
from .errors import WireError

OBSERVATION_SCHEMA_V2 = "observation-envelope.v2"
INFORMATION_STATE_SCHEMA_V3 = "information-state-envelope.v3"
INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3 = "information-state-digest-input.v3"


@dataclass(frozen=True, slots=True)
class ObservationEnvelopeV2:
    schema_version: str
    perspective: int
    view_sequence: int
    payload_codec: str
    payload_base64: str
    digest: str

    @classmethod
    def from_wire(cls, value: object) -> ObservationEnvelopeV2:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "perspective",
                "view_sequence",
                "payload_codec",
                "payload_base64",
                "digest",
            },
        )
        if obj["schema_version"] != OBSERVATION_SCHEMA_V2:
            raise WireError("decode.invalid_json", "unsupported observation V2 schema")
        result = cls(
            OBSERVATION_SCHEMA_V2,
            parse_uint(obj["perspective"]),
            parse_uint(obj["view_sequence"]),
            require_nonempty(obj["payload_codec"], "payload_codec"),
            require_canonical_base64(obj["payload_base64"]),
            require_digest(obj["digest"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != OBSERVATION_SCHEMA_V2:
            raise WireError("semantic.observation", "unsupported observation V2 schema")
        payload = require_canonical_base64(self.payload_base64)
        decoded = base64.b64decode(payload, validate=True)
        if observation_digest_from_payload(decoded) != self.digest:
            raise WireError("semantic.observation", "observation digest mismatch")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "digest": self.digest,
            "payload_base64": self.payload_base64,
            "payload_codec": self.payload_codec,
            "perspective": uint_wire(self.perspective),
            "schema_version": self.schema_version,
            "view_sequence": uint_wire(self.view_sequence),
        }


@dataclass(frozen=True, slots=True)
class InformationStateDigestInputV3:
    schema_version: str
    perspective: int
    current_observation: ObservationEnvelopeV2
    next_visible_sequence: int
    retained_knowledge: tuple[PlayerKnownObjectV1, ...]

    @classmethod
    def from_wire(cls, value: object) -> InformationStateDigestInputV3:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "perspective",
                "current_observation",
                "next_visible_sequence",
                "retained_knowledge",
            },
        )
        if obj["schema_version"] != INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3:
            raise WireError("decode.invalid_json", "unsupported information digest input V3")
        if not isinstance(obj["retained_knowledge"], list):
            raise WireError("decode.invalid_json", "retained_knowledge must be an array")
        result = cls(
            INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3,
            parse_uint(obj["perspective"]),
            ObservationEnvelopeV2.from_wire(obj["current_observation"]),
            parse_uint(obj["next_visible_sequence"]),
            tuple(PlayerKnownObjectV1.from_wire(item) for item in obj["retained_knowledge"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3:
            raise WireError("semantic.information_state", "unsupported digest input V3")
        if (
            self.current_observation.perspective != self.perspective
            or self.current_observation.view_sequence != self.next_visible_sequence
        ):
            raise WireError(
                "semantic.information_state", "observation cursor or perspective differs"
            )
        ids = tuple(item.opaque_object_id for item in self.retained_knowledge)
        if any(value == 0 for value in ids) or tuple(sorted(set(ids))) != ids:
            raise WireError("semantic.information_state", "retained knowledge is not canonical")
        for item in self.retained_knowledge:
            item.validate(self.next_visible_sequence)

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "current_observation": self.current_observation.to_wire(),
            "next_visible_sequence": uint_wire(self.next_visible_sequence),
            "perspective": uint_wire(self.perspective),
            "retained_knowledge": [item.to_wire() for item in self.retained_knowledge],
            "schema_version": self.schema_version,
        }


def compute_information_state_digest_v3(
    input_value: InformationStateDigestInputV3,
) -> tuple[bytes, str]:
    payload = canonical_json_bytes(input_value.to_wire())
    digest = hashlib.sha256(b"mtgml.information-state-digest.v3\0" + payload).hexdigest()
    return payload, digest


@dataclass(frozen=True, slots=True)
class PlayerInformationStateV3:
    schema_version: str
    perspective: int
    current_observation: ObservationEnvelopeV2
    next_visible_sequence: int
    retained_knowledge: tuple[PlayerKnownObjectV1, ...]
    digest: str

    @classmethod
    def from_wire(cls, value: object) -> PlayerInformationStateV3:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "perspective",
                "current_observation",
                "next_visible_sequence",
                "retained_knowledge",
                "digest",
            },
        )
        if obj["schema_version"] != INFORMATION_STATE_SCHEMA_V3:
            raise WireError("decode.invalid_json", "unsupported information-state V3")
        if not isinstance(obj["retained_knowledge"], list):
            raise WireError("decode.invalid_json", "retained_knowledge must be an array")
        result = cls(
            INFORMATION_STATE_SCHEMA_V3,
            parse_uint(obj["perspective"]),
            ObservationEnvelopeV2.from_wire(obj["current_observation"]),
            parse_uint(obj["next_visible_sequence"]),
            tuple(PlayerKnownObjectV1.from_wire(item) for item in obj["retained_knowledge"]),
            require_digest(obj["digest"]),
        )
        result.validate()
        return result

    def digest_input(self) -> InformationStateDigestInputV3:
        return InformationStateDigestInputV3(
            INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3,
            self.perspective,
            self.current_observation,
            self.next_visible_sequence,
            self.retained_knowledge,
        )

    def validate(self) -> None:
        if self.schema_version != INFORMATION_STATE_SCHEMA_V3:
            raise WireError("semantic.information_state", "unsupported information-state V3")
        self.current_observation.validate()
        if (
            self.current_observation.perspective != self.perspective
            or self.current_observation.view_sequence != self.next_visible_sequence
        ):
            raise WireError(
                "semantic.information_state", "observation cursor or perspective differs"
            )
        input_value = self.digest_input()
        _, expected = compute_information_state_digest_v3(input_value)
        if self.digest != expected:
            raise WireError("semantic.information_state", "information-state digest mismatch")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "current_observation": self.current_observation.to_wire(),
            "digest": self.digest,
            "next_visible_sequence": uint_wire(self.next_visible_sequence),
            "perspective": uint_wire(self.perspective),
            "retained_knowledge": [item.to_wire() for item in self.retained_knowledge],
            "schema_version": self.schema_version,
        }
