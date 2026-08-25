"""Shape-only submission encoder for decision-response.v2.

Mirrors the Rust boundary split: ``decode_submission`` judges shape,
canonical form, and schema identity only; response-local semantics
(duplicate order ids, ascending select_many, membership, cardinality vs
request, numeric bounds) are owned by the endpoint and are deliberately
NOT checked here. Structural violations fail closed with mechanical
adapter codes.
"""

from __future__ import annotations

from typing import Final, NoReturn

from ..canonical import canonical_json_bytes
from ..decision import DECISION_RESPONSE_V2_SCHEMA, DecisionAnswerV2, DecisionResponseV2
from .protocol import INVALID_PARAMS, AdapterError

_U32_MAXIMUM: Final = 2**32 - 1
_U64_MAXIMUM: Final = 2**64 - 1
_I64_MINIMUM: Final = -(2**63)
_I64_MAXIMUM: Final = 2**63 - 1


def _fail(message: str) -> NoReturn:
    raise AdapterError(INVALID_PARAMS, message)


def _u32_scalar(value: object, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0 or value > _U32_MAXIMUM:
        _fail(f"{label} is not a u32 scalar")
    return value


def _i64_scalar(value: object, label: str) -> int:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or value < _I64_MINIMUM
        or value > _I64_MAXIMUM
    ):
        _fail(f"{label} is not an i64 scalar")
    return value


def _u64_identity(value: object, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0 or value > _U64_MAXIMUM:
        _fail(f"{label} is not a u64 identity scalar")
    return value


def _answer_wire(answer: DecisionAnswerV2) -> dict[str, object]:
    kind = answer.kind
    if kind == "select_one":
        if answer.candidate_ids or answer.value is not None:
            _fail("select_one carries foreign answer fields")
        if answer.candidate_id is None:
            _fail("select_one lacks candidate_id")
        return {"candidate_id": _u32_scalar(answer.candidate_id, "candidate_id"), "kind": kind}
    if kind in {"select_many", "order"}:
        if answer.candidate_id is not None or answer.value is not None:
            _fail(f"{kind} carries foreign answer fields")
        ids = [_u32_scalar(item, "candidate_ids entry") for item in answer.candidate_ids]
        return {"candidate_ids": ids, "kind": kind}
    if kind == "choose_number":
        if answer.candidate_id is not None or answer.candidate_ids:
            _fail("choose_number carries foreign answer fields")
        if answer.value is None:
            _fail("choose_number lacks value")
        return {"kind": kind, "value": _i64_scalar(answer.value, "value")}
    _fail("unknown DecisionAnswerV2 variant")


def encode_decision_response_submission_v2(response: DecisionResponseV2) -> bytes:
    if response.schema_version != DECISION_RESPONSE_V2_SCHEMA:
        _fail("unsupported response schema identity")
    player_decision_id = _u64_identity(response.player_decision_id, "player_decision_id")
    state_revision = _u64_identity(response.state_revision, "state_revision")
    wire: dict[str, object] = {
        "answer": _answer_wire(response.answer),
        "player_decision_id": str(player_decision_id),
        "schema_version": DECISION_RESPONSE_V2_SCHEMA,
        "state_revision": str(state_revision),
    }
    return canonical_json_bytes(wire)
