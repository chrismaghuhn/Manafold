from __future__ import annotations

from dataclasses import dataclass

from ._events_v2 import ObservedEventEnvelopeV2
from ._information_v2 import PlayerInformationStateV2
from .canonical import require_exact_keys
from .decision import PlayerDecisionRequestV2
from .episode import EpisodeStatus
from .errors import WireError

PLAYER_STEP_SCHEMA_V2 = "player-step.v2"

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
                if self.status.kind == "running" or self.next_decision is not None:
                    raise WireError(
                        "semantic.player_step", "episode_closed rejection has an invalid product"
                    )
            elif self.submission.code == "unavailable_decision":
                if self.status.kind != "running" or self.next_decision is not None:
                    raise WireError(
                        "semantic.player_step",
                        "unavailable decision has an invalid product",
                    )
            elif self.submission.code in {
                "stale_decision",
                "invalid_answer",
                "invalid_candidate",
                "duplicate_assignment",
                "invalid_cardinality",
                "invalid_number",
                "invalid_order",
            } and (self.status.kind != "running" or self.next_decision is None):
                raise WireError(
                    "semantic.player_step",
                    "actor-bound rejection must carry the current decision",
                )

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "information_state": self.information_state.to_wire(),
            # Mirrors Rust PlayerStepV2: Option serializes as an explicit null,
            # so decision-less steps canonicalize identically in both languages.
            "next_decision": None if self.next_decision is None else self.next_decision.to_wire(),
            "observed_events": [event.to_wire() for event in self.observed_events],
            "schema_version": PLAYER_STEP_SCHEMA_V2,
            "status": self.status.to_wire(),
            "submission": self.submission.to_wire(),
        }
