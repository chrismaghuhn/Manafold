"""Mechanical player-facing Decision V3 request mirror.

This module validates request identity, ordering and response membership. It
does not decide which land is legal or execute a game action.
"""

from __future__ import annotations

from dataclasses import dataclass
from itertools import pairwise

from .canonical import parse_uint, require_exact_keys, uint_wire
from .decision import (
    DECISION_RESPONSE_V2_SCHEMA,
    DecisionResponseV2,
    DecisionSpec,
    _validate_candidate_capacity,
)
from .errors import WireError

PLAYER_DECISION_REQUEST_V3_SCHEMA = "player-decision-request.v3"
_ALLOWED_VISIBILITY = {"public", "acting_player_only", "mixed"}
_ALLOWED_INTENTS = {
    "pass_priority",
    "play_land",
    "cast_spell",
    "activate_ability",
    "select_object",
    "select_player",
    "select_mode",
    "choose_boolean",
    "declare_number",
    "confirm",
}
_RESPONSE_KIND_BY_DECISION_KIND = {
    "choose_one": "select_one",
    "choose_many": "select_many",
    "choose_number": "choose_number",
    "order": "order",
}


@dataclass(frozen=True, slots=True)
class CandidateIntentV3:
    kind: str
    value: int | bool | None = None

    @classmethod
    def from_wire(cls, value: object) -> CandidateIntentV3:
        if not isinstance(value, dict) or value.get("kind") not in _ALLOWED_INTENTS:
            raise WireError("decode.invalid_json", "unknown candidate V3 intent")
        kind = str(value["kind"])
        field = {
            "play_land": "object",
            "cast_spell": "object",
            "activate_ability": "ability",
            "select_object": "object",
            "select_player": "player",
            "select_mode": "mode_index",
            "choose_boolean": "value",
            "declare_number": "value",
        }.get(kind)
        obj = require_exact_keys(value, {"kind"} if field is None else {"kind", field})
        if field is None:
            return cls(kind)
        raw = obj[field]
        if field in {"object", "ability", "player"}:
            parsed: int | bool = parse_uint(raw)
        elif field == "mode_index":
            if isinstance(raw, bool) or not isinstance(raw, int) or not 0 <= raw <= 2**32 - 1:
                raise WireError("decode.invalid_json", "mode_index is outside u32")
            parsed = raw
        elif kind == "choose_boolean":
            if not isinstance(raw, bool):
                raise WireError("decode.invalid_json", "boolean choice must be boolean")
            parsed = raw
        else:
            if isinstance(raw, bool) or not isinstance(raw, int) or not -(2**63) <= raw < 2**63:
                raise WireError("decode.invalid_json", "declared number is outside i64")
            parsed = raw
        return cls(kind, parsed)

    def to_wire(self) -> dict[str, object]:
        if self.kind not in _ALLOWED_INTENTS:
            raise WireError("encode.serialization", "unknown candidate V3 intent")
        result: dict[str, object] = {"kind": self.kind}
        field = {
            "play_land": "object",
            "cast_spell": "object",
            "activate_ability": "ability",
            "select_object": "object",
            "select_player": "player",
            "select_mode": "mode_index",
            "choose_boolean": "value",
            "declare_number": "value",
        }.get(self.kind)
        if field is not None:
            if self.value is None:
                raise WireError("encode.serialization", "candidate intent payload is absent")
            result[field] = (
                uint_wire(self.value) if field in {"object", "ability", "player"} else self.value
            )
        elif self.value is not None:
            raise WireError("encode.serialization", "candidate intent has an unexpected payload")
        CandidateIntentV3.from_wire(result)
        return result


@dataclass(frozen=True, slots=True)
class VisibleCandidateV3:
    candidate_id: int
    intent: CandidateIntentV3

    @classmethod
    def from_wire(cls, value: object) -> VisibleCandidateV3:
        obj = require_exact_keys(value, {"candidate_id", "intent"})
        candidate_id = obj["candidate_id"]
        if (
            isinstance(candidate_id, bool)
            or not isinstance(candidate_id, int)
            or not 0 <= candidate_id <= 2**32 - 1
        ):
            raise WireError("decode.invalid_json", "candidate_id is outside u32")
        return cls(candidate_id, CandidateIntentV3.from_wire(obj["intent"]))

    def to_wire(self) -> dict[str, object]:
        if (
            isinstance(self.candidate_id, bool)
            or not isinstance(self.candidate_id, int)
            or not 0 <= self.candidate_id <= 2**32 - 1
        ):
            raise WireError("encode.serialization", "candidate_id is outside u32")
        return {"candidate_id": self.candidate_id, "intent": self.intent.to_wire()}


def _ordering_key(intent: CandidateIntentV3) -> tuple[int, int]:
    ranks = {
        "pass_priority": 0,
        "play_land": 1,
        "cast_spell": 2,
        "activate_ability": 3,
        "select_object": 4,
        "select_player": 5,
        "select_mode": 6,
        "choose_boolean": 7,
        "declare_number": 8,
        "confirm": 9,
    }
    if intent.kind not in ranks:
        raise WireError("semantic.decision", "unknown candidate ordering variant")
    value = intent.value
    if value is None:
        payload = 0
    elif isinstance(value, bool):
        payload = int(value)
    else:
        payload = value
    return ranks[intent.kind], payload


@dataclass(frozen=True, slots=True)
class PlayerDecisionRequestV3:
    schema_version: str
    player_decision_id: int
    state_revision: int
    actor: int
    visibility: str
    decision: DecisionSpec
    candidates: tuple[VisibleCandidateV3, ...]

    @classmethod
    def from_wire(cls, value: object) -> PlayerDecisionRequestV3:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "player_decision_id",
                "state_revision",
                "actor",
                "visibility",
                "decision",
                "candidates",
            },
        )
        if (
            obj["schema_version"] != PLAYER_DECISION_REQUEST_V3_SCHEMA
            or obj["visibility"] not in _ALLOWED_VISIBILITY
        ):
            raise WireError("decode.invalid_json", "unsupported decision V3 schema or visibility")
        if not isinstance(obj["candidates"], list):
            raise WireError("decode.invalid_json", "candidates must be an array")
        result = cls(
            PLAYER_DECISION_REQUEST_V3_SCHEMA,
            parse_uint(obj["player_decision_id"]),
            parse_uint(obj["state_revision"]),
            parse_uint(obj["actor"]),
            str(obj["visibility"]),
            DecisionSpec.from_wire(obj["decision"]),
            tuple(VisibleCandidateV3.from_wire(item) for item in obj["candidates"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != PLAYER_DECISION_REQUEST_V3_SCHEMA:
            raise WireError("semantic.decision", "unsupported decision V3 schema")
        if self.visibility not in _ALLOWED_VISIBILITY:
            raise WireError("semantic.decision", "unknown decision visibility")
        if (
            isinstance(self.actor, bool)
            or not isinstance(self.actor, int)
            or not 0 <= self.actor < 2**64
        ):
            raise WireError("semantic.decision", "actor is outside u64")
        self.decision.to_wire()
        _validate_candidate_capacity(len(self.candidates))
        self.decision.validate(len(self.candidates))
        if self.decision.kind == "choose_number" and self.candidates:
            raise WireError("semantic.decision", "choose_number cannot contain candidates")
        for candidate in self.candidates:
            candidate.to_wire()
        if [candidate.candidate_id for candidate in self.candidates] != list(
            range(len(self.candidates))
        ):
            raise WireError("semantic.decision", "candidate IDs must be dense from zero")
        keys = [_ordering_key(candidate.intent) for candidate in self.candidates]
        if any(left >= right for left, right in pairwise(keys)):
            raise WireError("semantic.decision", "candidate ordering is not canonical")

    def validate_response(self, response: DecisionResponseV2) -> None:
        if response.schema_version != DECISION_RESPONSE_V2_SCHEMA:
            raise WireError("decode.invalid_json", "unsupported response V2 schema")
        if response.player_decision_id != self.player_decision_id:
            raise WireError("semantic.decision_response", "decision identity mismatch")
        if response.state_revision != self.state_revision:
            raise WireError("semantic.decision_response", "state revision mismatch")
        self.validate()
        ids = {candidate.candidate_id for candidate in self.candidates}
        answer = response.answer
        if answer.kind != _RESPONSE_KIND_BY_DECISION_KIND[self.decision.kind]:
            raise WireError("semantic.decision_response", "response domain mismatch")
        if self.decision.kind == "choose_one":
            if answer.candidate_id not in ids:
                raise WireError("semantic.decision_response", "unknown candidate")
            return
        if self.decision.kind in {"choose_many", "order"}:
            values = answer.candidate_ids
            if any(candidate_id not in ids for candidate_id in values):
                raise WireError("semantic.decision_response", "unknown candidate")
            if len(set(values)) != len(values):
                raise WireError("semantic.decision_response", "duplicate candidate")
            if self.decision.kind == "choose_many" and any(
                left >= right for left, right in pairwise(values)
            ):
                raise WireError("semantic.decision_response", "SelectMany IDs are not ascending")
            assert self.decision.minimum is not None and self.decision.maximum is not None
            if not self.decision.minimum <= len(values) <= self.decision.maximum:
                raise WireError("semantic.decision_response", "answer cardinality is out of bounds")
            return
        if self.decision.kind == "choose_number":
            assert self.decision.minimum is not None and self.decision.maximum is not None
            if (
                answer.value is None
                or not self.decision.minimum <= answer.value <= self.decision.maximum
            ):
                raise WireError("semantic.decision_response", "numeric answer is out of bounds")
            return
        raise WireError("semantic.decision_response", "response domain mismatch")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "actor": uint_wire(self.actor),
            "candidates": [candidate.to_wire() for candidate in self.candidates],
            "decision": self.decision.to_wire(),
            "player_decision_id": uint_wire(self.player_decision_id),
            "schema_version": self.schema_version,
            "state_revision": uint_wire(self.state_revision),
            "visibility": self.visibility,
        }
