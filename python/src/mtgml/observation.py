from __future__ import annotations

from dataclasses import dataclass
from itertools import pairwise

from ._generated_contract_vocab import OBSERVED_EVENT_KINDS, ZONE_KINDS
from .canonical import (
    parse_u64_number,
    parse_uint,
    require_canonical_base64,
    require_digest,
    require_exact_keys,
    require_nonempty,
    uint_wire,
)
from .decision import PlayerDecisionRequest, PlayerDecisionRequestV2
from .episode import EpisodeStatus
from .errors import WireError
from .events import ObservedEventEnvelope

OBSERVATION_SCHEMA = "observation-envelope.v1"
INFORMATION_STATE_SCHEMA = "information-state-envelope.v1"
PLAYER_STEP_SCHEMA = "player-step.v1"
INFORMATION_STATE_SCHEMA_V2 = "information-state-envelope.v2"
OBSERVED_EVENT_SCHEMA_V2 = "observed-event-envelope.v2"
PLAYER_STEP_SCHEMA_V2 = "player-step.v2"


@dataclass(frozen=True, slots=True)
class ObservationEnvelope:
    schema_version: str
    perspective: int
    state_revision: int
    payload_codec: str
    payload_base64: str
    digest: str

    @classmethod
    def from_wire(cls, value: object) -> ObservationEnvelope:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "perspective",
                "state_revision",
                "payload_codec",
                "payload_base64",
                "digest",
            },
        )
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
    def from_wire(cls, value: object) -> InformationStateEnvelope:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "perspective",
                "state_revision",
                "current_observation",
                "public_history_length",
                "private_history_length",
                "digest",
            },
        )
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
        if (
            result.current_observation.perspective != result.perspective
            or result.current_observation.state_revision != result.state_revision
        ):
            raise WireError(
                "semantic.information_state",
                "current observation disagrees with information state",
            )
        return result

    def to_wire(self) -> dict[str, object]:
        if (
            self.current_observation.perspective != self.perspective
            or self.current_observation.state_revision != self.state_revision
        ):
            raise WireError(
                "semantic.information_state",
                "current observation disagrees with information state",
            )
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
    def from_wire(cls, value: object) -> PlayerStep:
        obj = require_exact_keys(
            value,
            {"schema_version", "information_state", "observed_events", "status"},
            {"next_decision"},
        )
        if obj["schema_version"] != PLAYER_STEP_SCHEMA or not isinstance(
            obj["observed_events"], list
        ):
            raise WireError("decode.invalid_json", "unsupported player-step schema")
        result = cls(
            PLAYER_STEP_SCHEMA,
            InformationStateEnvelope.from_wire(obj["information_state"]),
            tuple(ObservedEventEnvelope.from_wire(item) for item in obj["observed_events"]),
            (
                PlayerDecisionRequest.from_wire(obj["next_decision"])
                if "next_decision" in obj
                else None
            ),
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


@dataclass(frozen=True, slots=True)
class PlayerKnownLocationV1:
    zone: str
    player: int | None

    @classmethod
    def from_wire(cls, value: object) -> PlayerKnownLocationV1:
        obj = require_exact_keys(value, {"zone", "player"})
        if obj["zone"] not in {
            "library",
            "hand",
            "battlefield",
            "graveyard",
            "exile",
            "stack",
            "command",
            "ante",
            "outside",
        }:
            raise WireError("decode.invalid_json", "unknown public location zone")
        player = None if obj["player"] is None else parse_uint(obj["player"])
        return cls(str(obj["zone"]), player)

    def to_wire(self) -> dict[str, object]:
        return {
            "player": None if self.player is None else uint_wire(self.player),
            "zone": self.zone,
        }


@dataclass(frozen=True, slots=True)
class PlayerKnowledgeProvenanceV1:
    kind: str
    channel: str | None = None
    sequence: int | None = None
    cause: str | None = None

    @classmethod
    def from_wire(cls, value: object) -> PlayerKnowledgeProvenanceV1:
        if not isinstance(value, dict) or value.get("kind") not in {
            "initial_configuration",
            "observed",
        }:
            raise WireError("decode.invalid_json", "unknown knowledge provenance")
        if value["kind"] == "initial_configuration":
            require_exact_keys(value, {"kind"})
            return cls("initial_configuration")
        obj = require_exact_keys(value, {"kind", "channel", "sequence", "cause"})
        if obj["channel"] not in {"public", "private"} or obj["cause"] not in {
            "public_event",
            "private_look",
            "explicit_reveal",
            "own_private_identity",
        }:
            raise WireError("decode.invalid_json", "unknown knowledge provenance detail")
        return cls(
            "observed",
            str(obj["channel"]),
            parse_uint(obj["sequence"]),
            str(obj["cause"]),
        )

    def to_wire(self) -> dict[str, object]:
        if self.kind == "initial_configuration":
            require_exact_keys({"kind": self.kind}, {"kind"})
            return {"kind": self.kind}
        if (
            self.kind != "observed"
            or self.channel is None
            or self.sequence is None
            or self.cause is None
        ):
            raise WireError("encode.serialization", "invalid knowledge provenance")
        return {
            "cause": self.cause,
            "channel": self.channel,
            "kind": self.kind,
            "sequence": uint_wire(self.sequence),
        }


@dataclass(frozen=True, slots=True)
class PlayerKnownLocationFactV1:
    location: PlayerKnownLocationV1
    provenance: PlayerKnowledgeProvenanceV1

    @classmethod
    def from_wire(cls, value: object) -> PlayerKnownLocationFactV1:
        obj = require_exact_keys(value, {"location", "provenance"})
        return cls(
            PlayerKnownLocationV1.from_wire(obj["location"]),
            PlayerKnowledgeProvenanceV1.from_wire(obj["provenance"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "location": self.location.to_wire(),
            "provenance": self.provenance.to_wire(),
        }


@dataclass(frozen=True, slots=True)
class PlayerKnowledgeInvalidationV1:
    provenance: PlayerKnowledgeProvenanceV1
    reason: str

    @classmethod
    def from_wire(cls, value: object) -> PlayerKnowledgeInvalidationV1:
        obj = require_exact_keys(value, {"provenance", "reason"})
        if obj["reason"] not in {
            "hidden_transition",
            "randomization",
            "shuffle",
            "explicit_forget",
        }:
            raise WireError("decode.invalid_json", "unknown invalidation reason")
        return cls(PlayerKnowledgeProvenanceV1.from_wire(obj["provenance"]), str(obj["reason"]))

    def to_wire(self) -> dict[str, object]:
        return {"provenance": self.provenance.to_wire(), "reason": self.reason}


@dataclass(frozen=True, slots=True)
class PlayerKnownObjectV1:
    kind: str
    opaque_object_id: int
    known_definition: int | None
    current_known_location_fact: PlayerKnownLocationFactV1 | None = None
    last_known_location_fact: PlayerKnownLocationFactV1 | None = None
    historical_locations: tuple[PlayerKnownLocationFactV1, ...] = ()
    acquisition: PlayerKnowledgeProvenanceV1 | None = None
    invalidation: PlayerKnowledgeInvalidationV1 | None = None

    @classmethod
    def from_wire(cls, value: object) -> PlayerKnownObjectV1:
        if not isinstance(value, dict) or value.get("kind") not in {"active", "retired"}:
            raise WireError("decode.invalid_json", "unknown retained knowledge kind")
        kind = str(value["kind"])
        if kind == "active":
            obj = require_exact_keys(
                value,
                {
                    "kind",
                    "opaque_object_id",
                    "known_definition",
                    "current_known_location_fact",
                    "historical_locations",
                    "acquisition",
                },
            )
            if not isinstance(obj["historical_locations"], list):
                raise WireError("decode.invalid_json", "historical locations must be an array")
            return cls(
                kind,
                parse_uint(obj["opaque_object_id"]),
                None if obj["known_definition"] is None else parse_uint(obj["known_definition"]),
                None
                if obj["current_known_location_fact"] is None
                else PlayerKnownLocationFactV1.from_wire(obj["current_known_location_fact"]),
                historical_locations=tuple(
                    PlayerKnownLocationFactV1.from_wire(item)
                    for item in obj["historical_locations"]
                ),
                acquisition=PlayerKnowledgeProvenanceV1.from_wire(obj["acquisition"]),
            )
        obj = require_exact_keys(
            value,
            {
                "kind",
                "opaque_object_id",
                "known_definition",
                "last_known_location_fact",
                "historical_locations",
                "acquisition",
                "invalidation",
            },
        )
        if not isinstance(obj["historical_locations"], list):
            raise WireError("decode.invalid_json", "historical locations must be an array")
        return cls(
            kind,
            parse_uint(obj["opaque_object_id"]),
            None if obj["known_definition"] is None else parse_uint(obj["known_definition"]),
            last_known_location_fact=None
            if obj["last_known_location_fact"] is None
            else PlayerKnownLocationFactV1.from_wire(obj["last_known_location_fact"]),
            historical_locations=tuple(
                PlayerKnownLocationFactV1.from_wire(item) for item in obj["historical_locations"]
            ),
            acquisition=PlayerKnowledgeProvenanceV1.from_wire(obj["acquisition"]),
            invalidation=PlayerKnowledgeInvalidationV1.from_wire(obj["invalidation"]),
        )

    def validate(self, next_visible_sequence: int | None = None) -> None:
        if self.opaque_object_id == 0 or self.acquisition is None:
            raise WireError("semantic.information_state", "retained object identity is invalid")
        if self.kind == "active" and self.invalidation is not None:
            raise WireError("semantic.information_state", "active object is invalidated")
        if self.kind == "retired" and self.invalidation is None:
            raise WireError("semantic.information_state", "retired object lacks invalidation")
        sequences = [
            fact.provenance.sequence
            for fact in self.historical_locations
            if fact.provenance.sequence is not None
        ]
        if any(left >= right for left, right in pairwise(sequences)):
            raise WireError("semantic.information_state", "historical sequences are not increasing")
        for fact in self.historical_locations:
            _validate_provenance(fact.provenance, next_visible_sequence)
        if next_visible_sequence is not None:
            _validate_provenance(self.acquisition, next_visible_sequence)
            if self.current_known_location_fact is not None:
                _validate_provenance(
                    self.current_known_location_fact.provenance, next_visible_sequence
                )
            if self.last_known_location_fact is not None:
                _validate_provenance(
                    self.last_known_location_fact.provenance, next_visible_sequence
                )
            if self.invalidation is not None:
                # Retirement must be an observed fact: it records an
                # explicit reason *and visible sequence*.
                if self.invalidation.provenance.kind != "observed":
                    raise WireError(
                        "semantic.information_state",
                        "invalidation provenance must be an observed fact",
                    )
                _validate_provenance(self.invalidation.provenance, next_visible_sequence)

    def to_wire(self) -> dict[str, object]:
        self.validate()
        if self.kind == "active":
            return {
                "acquisition": self.acquisition.to_wire(),  # type: ignore[union-attr]
                "current_known_location_fact": None
                if self.current_known_location_fact is None
                else self.current_known_location_fact.to_wire(),
                "historical_locations": [fact.to_wire() for fact in self.historical_locations],
                "kind": "active",
                "known_definition": None
                if self.known_definition is None
                else uint_wire(self.known_definition),
                "opaque_object_id": uint_wire(self.opaque_object_id),
            }
        return {
            "acquisition": self.acquisition.to_wire(),  # type: ignore[union-attr]
            "historical_locations": [fact.to_wire() for fact in self.historical_locations],
            "invalidation": self.invalidation.to_wire(),  # type: ignore[union-attr]
            "kind": "retired",
            "known_definition": None
            if self.known_definition is None
            else uint_wire(self.known_definition),
            "last_known_location_fact": None
            if self.last_known_location_fact is None
            else self.last_known_location_fact.to_wire(),
            "opaque_object_id": uint_wire(self.opaque_object_id),
        }


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


def _validate_provenance(
    provenance: PlayerKnowledgeProvenanceV1, next_visible_sequence: int | None
) -> None:
    if next_visible_sequence is None:
        return
    sequence = provenance.sequence
    if sequence is not None and sequence >= next_visible_sequence:
        raise WireError(
            "semantic.information_state",
            "observed provenance sequence is not below the next visible sequence",
        )
    if provenance.kind == "initial_configuration":
        return
    accepted = {
        ("public", "public_event"),
        ("public", "explicit_reveal"),
        ("private", "private_look"),
        ("private", "own_private_identity"),
    }
    if (provenance.channel, provenance.cause) not in accepted:
        raise WireError(
            "semantic.information_state",
            "knowledge provenance cause is not accepted for its channel",
        )


_EVENT_V2_REQUIRED_KEYS: dict[str, set[str]] = {
    "object_moved": {"kind", "from", "to"},
    "object_ceased_to_exist": {"kind", "object"},
    "life_changed": {"kind", "player", "from", "to"},
    "object_tapped": {"kind", "object", "tapped"},
    "decision_available": {"kind", "actor"},
    "random_outcome_visible": {"kind", "label", "exclusive_upper_bound", "value"},
    "public_outcome": {"kind", "code"},
}
_EVENT_V2_UINT_KEYS = frozenset({"old_object", "new_object", "object", "player", "actor"})


@dataclass(frozen=True, slots=True)
class ObservedEventV2:
    kind: str
    payload: tuple[tuple[str, object], ...]

    @classmethod
    def from_wire(cls, value: object) -> ObservedEventV2:
        if not isinstance(value, dict) or value.get("kind") not in OBSERVED_EVENT_KINDS:
            raise WireError("decode.invalid_json", "unknown observed event V2 kind")
        kind = str(value["kind"])
        optional = {"old_object", "new_object"} if kind == "object_moved" else set()
        obj = require_exact_keys(value, _EVENT_V2_REQUIRED_KEYS[kind], optional)
        payload: dict[str, object] = {}
        if kind == "object_moved":
            if obj["from"] not in ZONE_KINDS or obj["to"] not in ZONE_KINDS:
                raise WireError("decode.invalid_json", "unknown zone kind")
            payload["from"] = str(obj["from"])
            payload["to"] = str(obj["to"])
            for key in ("old_object", "new_object"):
                payload[key] = None if obj.get(key) is None else parse_uint(obj[key])
        elif kind == "object_ceased_to_exist":
            payload["object"] = parse_uint(obj["object"])
        elif kind == "life_changed":
            payload["player"] = parse_uint(obj["player"])
            for key in ("from", "to"):
                raw = obj[key]
                if isinstance(raw, bool) or not isinstance(raw, int) or not -(2**63) <= raw < 2**63:
                    raise WireError("decode.invalid_json", "life value is outside i64")
                payload[key] = raw
        elif kind == "object_tapped":
            payload["object"] = parse_uint(obj["object"])
            if not isinstance(obj["tapped"], bool):
                raise WireError("decode.invalid_json", "tapped must be boolean")
            payload["tapped"] = obj["tapped"]
        elif kind == "decision_available":
            payload["actor"] = parse_uint(obj["actor"])
        elif kind == "random_outcome_visible":
            label = obj["label"]
            if not isinstance(label, str):
                raise WireError("decode.invalid_json", "label must be a string")
            upper = parse_u64_number(obj["exclusive_upper_bound"])
            outcome = parse_u64_number(obj["value"])
            if not label or upper == 0 or outcome >= upper:
                raise WireError(
                    "semantic.observed_event",
                    "random outcome is outside its declared range",
                )
            payload.update(label=label, exclusive_upper_bound=upper, value=outcome)
        else:
            code = obj["code"]
            if not isinstance(code, str):
                raise WireError("decode.invalid_json", "code must be a string")
            if not code:
                raise WireError("semantic.observed_event", "observed event code is empty")
            payload["code"] = code
        return cls(kind, tuple(sorted(payload.items())))

    def to_wire(self) -> dict[str, object]:
        result: dict[str, object] = {"kind": self.kind}
        for key, value in self.payload:
            result[key] = (
                uint_wire(value)  # type: ignore[arg-type]  # guaranteed int by wire contract
                if key in _EVENT_V2_UINT_KEYS and value is not None
                else value
            )
        ObservedEventV2.from_wire(result)
        return result


@dataclass(frozen=True, slots=True)
class ObservedEventEnvelopeV2:
    schema_version: str
    sequence: int
    state_revision: int
    event: ObservedEventV2

    @classmethod
    def from_wire(cls, value: object) -> ObservedEventEnvelopeV2:
        obj = require_exact_keys(value, {"schema_version", "sequence", "state_revision", "event"})
        if obj["schema_version"] != OBSERVED_EVENT_SCHEMA_V2:
            raise WireError("decode.invalid_json", "unsupported observed event V2")
        return cls(
            OBSERVED_EVENT_SCHEMA_V2,
            parse_uint(obj["sequence"]),
            parse_uint(obj["state_revision"]),
            ObservedEventV2.from_wire(obj["event"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "event": self.event.to_wire(),
            "schema_version": OBSERVED_EVENT_SCHEMA_V2,
            "sequence": uint_wire(self.sequence),
            "state_revision": uint_wire(self.state_revision),
        }


PLAYER_SUBMISSION_CODES = frozenset(
    {
        "stale_decision",
        "unavailable_decision",
        "invalid_answer",
        "invalid_candidate",
        "duplicate_assignment",
        "invalid_cardinality",
        "invalid_number",
        "invalid_order",
        "episode_closed",
    }
)


@dataclass(frozen=True, slots=True)
class PlayerStepSubmissionV1:
    kind: str
    code: str | None = None

    @classmethod
    def from_wire(cls, value: object) -> PlayerStepSubmissionV1:
        if not isinstance(value, dict):
            raise WireError("decode.invalid_json", "submission must be an object")
        kind = value.get("kind")
        if kind == "accepted":
            require_exact_keys(value, {"kind"})
            return cls("accepted")
        if kind == "rejected":
            obj = require_exact_keys(value, {"kind", "code"})
            code = obj["code"]
            if code not in PLAYER_SUBMISSION_CODES:
                raise WireError("decode.invalid_json", "unknown submission code")
            return cls("rejected", str(code))
        raise WireError("decode.invalid_json", "unknown submission outcome kind")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        if self.kind == "accepted":
            return {"kind": "accepted"}
        return {"kind": "rejected", "code": self.code}

    def validate(self) -> None:
        if self.kind == "accepted":
            if self.code is not None:
                raise WireError("encode.serialization", "accepted submission must not carry a code")
        elif self.kind == "rejected":
            if self.code is None or self.code not in PLAYER_SUBMISSION_CODES:
                raise WireError(
                    "encode.serialization", "rejected submission carries an invalid code"
                )
        else:
            raise WireError("encode.serialization", "unknown submission kind")


@dataclass(frozen=True, slots=True)
class PlayerStepV2:
    schema_version: str
    information_state: PlayerInformationStateV2
    observed_events: tuple[ObservedEventEnvelopeV2, ...]
    next_decision: PlayerDecisionRequestV2 | None
    status: EpisodeStatus
    submission: PlayerStepSubmissionV1

    @classmethod
    def from_wire(cls, value: object) -> PlayerStepV2:
        obj = require_exact_keys(
            value,
            {"schema_version", "information_state", "observed_events", "status", "submission"},
            {"next_decision"},
        )
        if obj["schema_version"] != PLAYER_STEP_SCHEMA_V2 or not isinstance(
            obj["observed_events"], list
        ):
            raise WireError("decode.invalid_json", "unsupported player-step V2")
        result = cls(
            PLAYER_STEP_SCHEMA_V2,
            PlayerInformationStateV2.from_wire(obj["information_state"]),
            tuple(ObservedEventEnvelopeV2.from_wire(item) for item in obj["observed_events"]),
            None
            if obj.get("next_decision") is None
            else PlayerDecisionRequestV2.from_wire(obj["next_decision"]),
            EpisodeStatus.from_wire(obj["status"]),
            PlayerStepSubmissionV1.from_wire(obj["submission"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        self.information_state.validate()
        revision = self.information_state.state_revision
        cursor = self.information_state.next_visible_sequence
        previous_sequence = None
        for event in self.observed_events:
            # One accepted transition owns exactly one revision.
            if event.state_revision != revision:
                raise WireError("semantic.player_step", "event belongs to a different revision")
            # Visible sequences never reach the step's own next-unused cursor.
            if event.sequence >= cursor:
                raise WireError(
                    "semantic.player_step", "event sequence is not below the visible cursor"
                )
            if previous_sequence is not None and event.sequence <= previous_sequence:
                raise WireError(
                    "semantic.player_step",
                    "event sequences must be strictly increasing",
                )
            previous_sequence = event.sequence
        if self.next_decision is not None and (
            self.next_decision.actor != self.information_state.perspective
            or self.next_decision.state_revision != revision
        ):
            raise WireError("semantic.player_step", "next decision is not for this endpoint")
        if self.status.kind != "running" and self.next_decision is not None:
            raise WireError("semantic.player_step", "completed episode exposes a decision")
        # ML_ENVIRONMENT.md rejection invariants.
        self.submission.validate()
        if self.submission.kind == "rejected":
            if self.observed_events:
                raise WireError(
                    "semantic.player_step",
                    "rejected submission must carry an empty event batch",
                )
            if self.submission.code == "episode_closed":
                if self.status.kind == "running":
                    raise WireError(
                        "semantic.player_step",
                        "episode_closed rejection requires a non-running status",
                    )
            elif self.status.kind != "running":
                raise WireError(
                    "semantic.player_step",
                    "typed rejection requires a running episode",
                )

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "information_state": self.information_state.to_wire(),
            # Mirrors Rust PlayerStepV2: Option serializes as an explicit null,
            # so decision-less steps canonicalize identically in both languages.
            "next_decision": None
            if self.next_decision is None
            else self.next_decision.to_wire(),
            "observed_events": [event.to_wire() for event in self.observed_events],
            "schema_version": PLAYER_STEP_SCHEMA_V2,
            "status": self.status.to_wire(),
            "submission": self.submission.to_wire(),
        }
