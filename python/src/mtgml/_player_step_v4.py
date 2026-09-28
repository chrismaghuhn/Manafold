"""Rules-free decoder for detached PlayerStep V4 products."""

from __future__ import annotations

from dataclasses import dataclass

from ._events_v4 import ObservedEventEnvelopeV4
from ._player_step_v2 import PLAYER_SUBMISSION_CODES, PlayerStepSubmissionV1
from .canonical import require_exact_keys
from .decision_v4 import PlayerDecisionRequestV4
from .episode import EpisodeStatus
from .errors import WireError
from .observation_v3 import PlayerInformationStateV3

PLAYER_STEP_SCHEMA_V4 = "player-step.v4"


@dataclass(frozen=True, slots=True)
class PlayerStepV4:
    schema_version: str
    information_state: PlayerInformationStateV3
    observed_events: tuple[ObservedEventEnvelopeV4, ...]
    next_decision: PlayerDecisionRequestV4 | None
    status: EpisodeStatus
    submission: PlayerStepSubmissionV1

    @classmethod
    def from_wire(cls, value: object) -> PlayerStepV4:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "information_state",
                "observed_events",
                "next_decision",
                "status",
                "submission",
            },
        )
        if obj["schema_version"] != PLAYER_STEP_SCHEMA_V4 or not isinstance(
            obj["observed_events"], list
        ):
            raise WireError("decode.invalid_json", "unsupported player-step V4")
        result = cls(
            PLAYER_STEP_SCHEMA_V4,
            PlayerInformationStateV3.from_wire(obj["information_state"]),
            tuple(ObservedEventEnvelopeV4.from_wire(item) for item in obj["observed_events"]),
            None
            if obj["next_decision"] is None
            else PlayerDecisionRequestV4.from_wire(obj["next_decision"]),
            EpisodeStatus.from_wire(obj["status"]),
            PlayerStepSubmissionV1.from_wire(obj["submission"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        self.information_state.validate()
        cursor = self.information_state.next_visible_sequence
        previous_sequence: int | None = None
        for event in self.observed_events:
            if event.sequence >= cursor or (
                previous_sequence is not None and event.sequence <= previous_sequence
            ):
                raise WireError("semantic.player_step", "event sequence is invalid")
            previous_sequence = event.sequence
        if self.next_decision is not None and (
            self.next_decision.actor != self.information_state.perspective
            or self.next_decision.view_sequence != cursor
        ):
            raise WireError("semantic.player_step", "next decision is not bound to this endpoint")
        if self.status.kind != "running" and self.next_decision is not None:
            raise WireError("semantic.player_step", "closed episode exposes a decision")
        self.submission.validate()
        if self.submission.kind == "rejected":
            if self.observed_events:
                raise WireError("semantic.player_step", "rejected submission must carry no events")
            if self.submission.code == "episode_closed":
                valid = self.status.kind != "running" and self.next_decision is None
            elif self.submission.code == "unavailable_decision":
                valid = self.status.kind == "running" and self.next_decision is None
            elif self.submission.code in PLAYER_SUBMISSION_CODES - {
                "episode_closed",
                "unavailable_decision",
            }:
                valid = self.status.kind == "running" and self.next_decision is not None
            else:
                valid = False
            if not valid:
                raise WireError("semantic.player_step", "rejected submission contradicts the step")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "information_state": self.information_state.to_wire(),
            "next_decision": None if self.next_decision is None else self.next_decision.to_wire(),
            "observed_events": [event.to_wire() for event in self.observed_events],
            "schema_version": PLAYER_STEP_SCHEMA_V4,
            "status": self.status.to_wire(),
            "submission": self.submission.to_wire(),
        }
