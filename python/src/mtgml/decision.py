from __future__ import annotations

from dataclasses import dataclass

from .canonical import parse_uint, require_exact_keys, require_nonempty, uint_wire
from .errors import WireError

PLAYER_DECISION_REQUEST_SCHEMA = "player-decision-request.v1"
DECISION_RESPONSE_SCHEMA = "decision-response.v1"

_ALLOWED_VISIBILITY = {"public", "acting_player_only", "mixed"}
_ALLOWED_DECISIONS = {"choose_one", "choose_many", "choose_number", "order"}
_ALLOWED_INTENTS = {
    "pass_priority",
    "cast_spell",
    "activate_ability",
    "select_object",
    "select_player",
    "select_mode",
    "choose_boolean",
    "declare_number",
    "confirm",
}


@dataclass(frozen=True, slots=True)
class CandidateIntent:
    kind: str
    payload: tuple[tuple[str, object], ...] = ()

    @classmethod
    def from_wire(cls, value: object) -> CandidateIntent:
        if not isinstance(value, dict) or value.get("kind") not in _ALLOWED_INTENTS:
            raise WireError("decode.invalid_json", "unknown candidate intent")
        kind = str(value["kind"])
        fields: dict[str, set[str]] = {
            "pass_priority": set(),
            "cast_spell": {"object"},
            "activate_ability": {"ability"},
            "select_object": {"object"},
            "select_player": {"player"},
            "select_mode": {"mode_index"},
            "choose_boolean": {"value"},
            "declare_number": {"value"},
            "confirm": set(),
        }
        obj = require_exact_keys(value, {"kind"} | fields[kind])
        payload: list[tuple[str, object]] = []
        for key in sorted(fields[kind]):
            raw = obj[key]
            if key in {"object", "ability", "player"}:
                parsed: object = parse_uint(raw)
            elif key == "mode_index":
                if isinstance(raw, bool) or not isinstance(raw, int) or raw < 0 or raw > 2**32 - 1:
                    raise WireError("decode.invalid_json", "mode_index is outside u32")
                parsed = raw
            elif kind == "choose_boolean":
                if not isinstance(raw, bool):
                    raise WireError("decode.invalid_json", "boolean choice must be boolean")
                parsed = raw
            elif kind == "declare_number":
                if isinstance(raw, bool) or not isinstance(raw, int) or not -(2**63) <= raw < 2**63:
                    raise WireError("decode.invalid_json", "declared number is outside i64")
                parsed = raw
            else:
                parsed = raw
            payload.append((key, parsed))
        return cls(kind, tuple(payload))

    def to_wire(self) -> dict[str, object]:
        if self.kind not in _ALLOWED_INTENTS:
            raise WireError("encode.serialization", "unknown candidate intent")
        result: dict[str, object] = {"kind": self.kind}
        for key, value in self.payload:
            result[key] = uint_wire(int(value)) if key in {"object", "ability", "player"} else value
        # Reuse the reader as exact structural validation.
        CandidateIntent.from_wire(result)
        return result


@dataclass(frozen=True, slots=True)
class ActionCandidate:
    candidate_id: str
    semantic_key: str
    intent: CandidateIntent

    @classmethod
    def from_wire(cls, value: object) -> ActionCandidate:
        obj = require_exact_keys(value, {"candidate_id", "semantic_key", "intent"})
        return cls(
            require_nonempty(obj["candidate_id"], "candidate_id"),
            require_nonempty(obj["semantic_key"], "semantic_key"),
            CandidateIntent.from_wire(obj["intent"]),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "candidate_id": require_nonempty(self.candidate_id, "candidate_id"),
            "intent": self.intent.to_wire(),
            "semantic_key": require_nonempty(self.semantic_key, "semantic_key"),
        }


@dataclass(frozen=True, slots=True)
class DecisionSpec:
    kind: str
    minimum: int | None = None
    maximum: int | None = None

    @classmethod
    def from_wire(cls, value: object) -> DecisionSpec:
        if not isinstance(value, dict) or value.get("kind") not in _ALLOWED_DECISIONS:
            raise WireError("decode.invalid_json", "unknown decision kind")
        kind = str(value["kind"])
        if kind == "choose_one":
            require_exact_keys(value, {"kind"})
            return cls(kind)
        obj = require_exact_keys(value, {"kind", "minimum", "maximum"})
        minimum, maximum = obj["minimum"], obj["maximum"]
        if (
            isinstance(minimum, bool)
            or isinstance(maximum, bool)
            or not isinstance(minimum, int)
            or not isinstance(maximum, int)
        ):
            raise WireError("decode.invalid_json", "decision bounds must be integers")
        if kind in {"choose_many", "order"} and (minimum < 0 or maximum < 0 or maximum > 2**32 - 1):
            raise WireError("decode.invalid_json", "selection bounds are outside u32")
        if kind == "choose_number" and not (
            -(2**63) <= minimum < 2**63 and -(2**63) <= maximum < 2**63
        ):
            raise WireError("decode.invalid_json", "numeric bounds are outside i64")
        return cls(kind, minimum, maximum)

    def validate(self, candidate_count: int) -> None:
        if self.kind != "choose_one" and (self.minimum is None or self.maximum is None):
            raise WireError("semantic.decision", "decision bounds are absent")
        if self.minimum is not None and self.maximum is not None and self.minimum > self.maximum:
            raise WireError("semantic.decision", "decision bounds are inverted")
        effective_minimum = 1 if self.kind == "choose_one" else self.minimum
        if (
            self.kind in {"choose_one", "choose_many", "order"}
            and effective_minimum is not None
            and effective_minimum > candidate_count
        ):
            raise WireError("semantic.decision", "minimum cannot be satisfied")

    def to_wire(self) -> dict[str, object]:
        result: dict[str, object] = {"kind": self.kind}
        if self.kind != "choose_one":
            result["maximum"] = self.maximum
            result["minimum"] = self.minimum
        DecisionSpec.from_wire(result)
        return result


@dataclass(frozen=True, slots=True)
class PlayerDecisionRequest:
    schema_version: str
    decision_id: int
    state_revision: int
    actor: int
    visibility: str
    decision: DecisionSpec
    candidates: tuple[ActionCandidate, ...]

    @classmethod
    def from_wire(cls, value: object) -> PlayerDecisionRequest:
        obj = require_exact_keys(
            value,
            {
                "schema_version",
                "decision_id",
                "state_revision",
                "actor",
                "visibility",
                "decision",
                "candidates",
            },
        )
        if (
            obj["schema_version"] != PLAYER_DECISION_REQUEST_SCHEMA
            or obj["visibility"] not in _ALLOWED_VISIBILITY
        ):
            raise WireError("decode.invalid_json", "unsupported decision schema or visibility")
        if not isinstance(obj["candidates"], list):
            raise WireError("decode.invalid_json", "candidates must be an array")
        result = cls(
            PLAYER_DECISION_REQUEST_SCHEMA,
            parse_uint(obj["decision_id"]),
            parse_uint(obj["state_revision"]),
            parse_uint(obj["actor"]),
            str(obj["visibility"]),
            DecisionSpec.from_wire(obj["decision"]),
            tuple(ActionCandidate.from_wire(item) for item in obj["candidates"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        ids = [candidate.candidate_id for candidate in self.candidates]
        keys = [candidate.semantic_key for candidate in self.candidates]
        if len(set(ids)) != len(ids):
            raise WireError("semantic.decision", "candidate IDs are not unique")
        if len(set(keys)) != len(keys):
            raise WireError("semantic.decision", "semantic keys are not unique")
        self.decision.validate(len(self.candidates))

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "actor": uint_wire(self.actor),
            "candidates": [candidate.to_wire() for candidate in self.candidates],
            "decision": self.decision.to_wire(),
            "decision_id": uint_wire(self.decision_id),
            "schema_version": self.schema_version,
            "state_revision": uint_wire(self.state_revision),
            "visibility": self.visibility,
        }


@dataclass(frozen=True, slots=True)
class CandidateAssignment:
    candidate_id: str
    ordinal: int | None = None

    @classmethod
    def from_wire(cls, value: object) -> CandidateAssignment:
        obj = require_exact_keys(value, {"candidate_id"}, {"ordinal"})
        ordinal = obj.get("ordinal")
        if ordinal is not None and (
            isinstance(ordinal, bool)
            or not isinstance(ordinal, int)
            or not 0 <= ordinal <= 2**32 - 1
        ):
            raise WireError("decode.invalid_json", "ordinal is outside u32")
        return cls(require_nonempty(obj["candidate_id"], "candidate_id"), ordinal)

    def to_wire(self) -> dict[str, object]:
        result: dict[str, object] = {
            "candidate_id": require_nonempty(self.candidate_id, "candidate_id")
        }
        if self.ordinal is not None:
            result["ordinal"] = self.ordinal
        CandidateAssignment.from_wire(result)
        return result


@dataclass(frozen=True, slots=True)
class DecisionResponse:
    schema_version: str
    decision_id: int
    state_revision: int
    assignments: tuple[CandidateAssignment, ...]

    @classmethod
    def from_wire(cls, value: object) -> DecisionResponse:
        obj = require_exact_keys(
            value, {"schema_version", "decision_id", "state_revision", "assignments"}
        )
        if obj["schema_version"] != DECISION_RESPONSE_SCHEMA or not isinstance(
            obj["assignments"], list
        ):
            raise WireError("decode.invalid_json", "unsupported response schema or assignments")
        result = cls(
            DECISION_RESPONSE_SCHEMA,
            parse_uint(obj["decision_id"]),
            parse_uint(obj["state_revision"]),
            tuple(CandidateAssignment.from_wire(item) for item in obj["assignments"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        ids = [assignment.candidate_id for assignment in self.assignments]
        if len(set(ids)) != len(ids):
            raise WireError(
                "semantic.decision_response",
                "assignments contain duplicate candidate IDs",
            )

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "assignments": [assignment.to_wire() for assignment in self.assignments],
            "decision_id": uint_wire(self.decision_id),
            "schema_version": self.schema_version,
            "state_revision": uint_wire(self.state_revision),
        }
