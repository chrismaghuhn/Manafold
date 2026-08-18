from __future__ import annotations

from dataclasses import dataclass

from .canonical import parse_u64_number, parse_uint, require_canonical_base64, require_digest, require_exact_keys, require_nonempty, uint_wire
from .decision import PlayerDecisionRequest
from .episode import EpisodeStatus
from .errors import WireError
from .events import ObservedEventEnvelope

OBSERVATION_SCHEMA = "observation-envelope.v1"
INFORMATION_STATE_SCHEMA = "information-state-envelope.v1"
PLAYER_STEP_SCHEMA = "player-step.v1"


@dataclass(frozen=True, slots=True)
class ObservationEnvelope:
    schema_version: str
    perspective: int
    state_revision: int
    payload_codec: str
    payload_base64: str
    digest: str

    @classmethod
    def from_wire(cls, value: object) -> "ObservationEnvelope":
        obj = require_exact_keys(value, {"schema_version", "perspective", "state_revision", "payload_codec", "payload_base64", "digest"})
        if obj["schema_version"] != OBSERVATION_SCHEMA:
            raise WireError("decode.invalid_json", "unsupported observation schema")
        return cls(
            OBSERVATION_SCHEMA,
            parse_uint(obj["perspective"]),
            parse_uint(obj["state_revision"]),
            require_nonempty(obj["payload_codec"], "payload_codec"),
            require_canonical_base64(obj["payload_base64"]),
            require_digest(obj["digest"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "digest": require_digest(self.digest),
            "payload_base64": require_canonical_base64(self.payload_base64),
            "payload_codec": require_nonempty(self.payload_codec, "payload_codec"),
            "perspective": uint_wire(self.perspective),
            "schema_version": self.schema_version,
            "state_revision": uint_wire(self.state_revision),
        }


@dataclass(frozen=True, slots=True)
class InformationStateEnvelope:
    schema_version: str
    perspective: int
    state_revision: int
    current_observation: ObservationEnvelope
    public_history_length: int
    private_history_length: int
    digest: str

    @classmethod
    def from_wire(cls, value: object) -> "InformationStateEnvelope":
        obj = require_exact_keys(value, {"schema_version", "perspective", "state_revision", "current_observation", "public_history_length", "private_history_length", "digest"})
        if obj["schema_version"] != INFORMATION_STATE_SCHEMA:
            raise WireError("decode.invalid_json", "unsupported information-state schema")
        result = cls(
            INFORMATION_STATE_SCHEMA,
            parse_uint(obj["perspective"]),
            parse_uint(obj["state_revision"]),
            ObservationEnvelope.from_wire(obj["current_observation"]),
            parse_u64_number(obj["public_history_length"]),
            parse_u64_number(obj["private_history_length"]),
            require_digest(obj["digest"]),
        )
        if result.current_observation.perspective != result.perspective or result.current_observation.state_revision != result.state_revision:
            raise WireError("semantic.information_state", "current observation disagrees with information state")
        return result

    def to_wire(self) -> dict[str, object]:
        if self.current_observation.perspective != self.perspective or self.current_observation.state_revision != self.state_revision:
            raise WireError("semantic.information_state", "current observation disagrees with information state")
        return {
            "current_observation": self.current_observation.to_wire(),
            "digest": require_digest(self.digest),
            "perspective": uint_wire(self.perspective),
            "private_history_length": parse_u64_number(self.private_history_length),
            "public_history_length": parse_u64_number(self.public_history_length),
            "schema_version": self.schema_version,
            "state_revision": uint_wire(self.state_revision),
        }


@dataclass(frozen=True, slots=True)
class PlayerStep:
    schema_version: str
    information_state: InformationStateEnvelope
    observed_events: tuple[ObservedEventEnvelope, ...]
    next_decision: PlayerDecisionRequest | None
    status: EpisodeStatus

    @classmethod
    def from_wire(cls, value: object) -> "PlayerStep":
        obj = require_exact_keys(value, {"schema_version", "information_state", "observed_events", "status"}, {"next_decision"})
        if obj["schema_version"] != PLAYER_STEP_SCHEMA or not isinstance(obj["observed_events"], list):
            raise WireError("decode.invalid_json", "unsupported player-step schema")
        result = cls(
            PLAYER_STEP_SCHEMA,
            InformationStateEnvelope.from_wire(obj["information_state"]),
            tuple(ObservedEventEnvelope.from_wire(item) for item in obj["observed_events"]),
            PlayerDecisionRequest.from_wire(obj["next_decision"]) if "next_decision" in obj else None,
            EpisodeStatus.from_wire(obj["status"]),
        )
        result.validate()
        return result

    @property
    def observation(self) -> ObservationEnvelope:
        return self.information_state.current_observation

    def validate(self) -> None:
        perspective = self.information_state.perspective
        revision = self.information_state.state_revision
        if any(event.state_revision > revision for event in self.observed_events):
            raise WireError("semantic.player_step", "event belongs to a future revision")
        if self.next_decision is not None and (
            self.next_decision.actor != perspective or self.next_decision.state_revision != revision
        ):
            raise WireError("semantic.player_step", "next decision is not for this endpoint")
        if self.status.kind != "running" and self.next_decision is not None:
            raise WireError("semantic.player_step", "completed episode exposes a decision")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        result: dict[str, object] = {
            "information_state": self.information_state.to_wire(),
            "observed_events": [event.to_wire() for event in self.observed_events],
            "schema_version": self.schema_version,
            "status": self.status.to_wire(),
        }
        if self.next_decision is not None:
            result["next_decision"] = self.next_decision.to_wire()
        return result
