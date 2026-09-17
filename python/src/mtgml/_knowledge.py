from __future__ import annotations

from dataclasses import dataclass
from itertools import pairwise

from .canonical import parse_uint, require_exact_keys, uint_wire
from .errors import WireError


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
