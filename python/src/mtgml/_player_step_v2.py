from __future__ import annotations

from dataclasses import dataclass

from .canonical import require_exact_keys
from .errors import WireError

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
