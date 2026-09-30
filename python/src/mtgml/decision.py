from __future__ import annotations

from dataclasses import dataclass

from .canonical import parse_uint, require_exact_keys, uint_wire
from .errors import WireError

DECISION_RESPONSE_V3_SCHEMA = "decision-response.v3"


_ALLOWED_DECISIONS = {"choose_one", "choose_many", "choose_number", "order"}


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


def _parse_candidate_id(value: object) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**32 - 1:
        raise WireError("decode.invalid_json", "candidate_id is outside u32")
    return value


@dataclass(frozen=True, slots=True)
class DecisionAnswerV2:
    kind: str
    candidate_id: int | None = None
    candidate_ids: tuple[int, ...] = ()
    value: int | None = None

    @classmethod
    def from_wire(cls, value: object) -> DecisionAnswerV2:
        if not isinstance(value, dict) or value.get("kind") not in {
            "select_one",
            "select_many",
            "choose_number",
            "order",
        }:
            raise WireError("decode.invalid_json", "unknown DecisionAnswerV2 variant")
        kind = str(value["kind"])
        if kind == "select_one":
            obj = require_exact_keys(value, {"kind", "candidate_id"})
            return cls(kind, candidate_id=_parse_candidate_id(obj["candidate_id"]))
        if kind in {"select_many", "order"}:
            obj = require_exact_keys(value, {"kind", "candidate_ids"})
            if not isinstance(obj["candidate_ids"], list):
                raise WireError("decode.invalid_json", "candidate_ids must be an array")
            return cls(
                kind,
                candidate_ids=tuple(_parse_candidate_id(item) for item in obj["candidate_ids"]),
            )
        obj = require_exact_keys(value, {"kind", "value"})
        raw = obj["value"]
        if isinstance(raw, bool) or not isinstance(raw, int) or not -(2**63) <= raw < 2**63:
            raise WireError("decode.invalid_json", "numeric answer is outside i64")
        return cls(kind, value=raw)

    def validate(self) -> None:
        if self.kind == "select_many" and any(
            left >= right
            for left, right in zip(self.candidate_ids, self.candidate_ids[1:], strict=False)
        ):
            raise WireError("semantic.decision_response", "SelectMany IDs are not ascending")
        if self.kind == "order" and len(set(self.candidate_ids)) != len(self.candidate_ids):
            raise WireError("semantic.decision_response", "Order contains duplicate IDs")

    def to_wire(self) -> dict[str, object]:
        self.validate()
        if self.kind == "select_one":
            return {"candidate_id": _parse_candidate_id(self.candidate_id), "kind": self.kind}
        if self.kind in {"select_many", "order"}:
            return {
                "candidate_ids": [_parse_candidate_id(item) for item in self.candidate_ids],
                "kind": self.kind,
            }
        if self.kind == "choose_number":
            if (
                self.value is None
                or isinstance(self.value, bool)
                or not -(2**63) <= self.value < 2**63
            ):
                raise WireError("encode.serialization", "numeric answer is outside i64")
            return {"kind": self.kind, "value": self.value}
        raise WireError("encode.serialization", "unknown DecisionAnswerV2 variant")


@dataclass(frozen=True, slots=True)
class DecisionResponseV3:
    """Perspective-safe response bound to a visible request and view cursor."""

    schema_version: str
    player_decision_id: int
    view_sequence: int
    answer: DecisionAnswerV2

    @classmethod
    def from_wire(cls, value: object) -> DecisionResponseV3:
        obj = require_exact_keys(
            value, {"schema_version", "player_decision_id", "view_sequence", "answer"}
        )
        if obj["schema_version"] != DECISION_RESPONSE_V3_SCHEMA:
            raise WireError("decode.invalid_json", "unsupported response V3 schema")
        result = cls(
            DECISION_RESPONSE_V3_SCHEMA,
            parse_uint(obj["player_decision_id"]),
            parse_uint(obj["view_sequence"]),
            DecisionAnswerV2.from_wire(obj["answer"]),
        )
        result.validate()
        return result

    def validate(self) -> None:
        if self.schema_version != DECISION_RESPONSE_V3_SCHEMA:
            raise WireError("semantic.decision_response", "unsupported response V3 schema")
        if not 0 <= self.player_decision_id < 2**64 or not 0 <= self.view_sequence < 2**64:
            raise WireError("semantic.decision_response", "response identity is outside u64")
        self.answer.validate()

    def to_wire(self) -> dict[str, object]:
        self.validate()
        return {
            "answer": self.answer.to_wire(),
            "player_decision_id": uint_wire(self.player_decision_id),
            "schema_version": self.schema_version,
            "view_sequence": uint_wire(self.view_sequence),
        }
