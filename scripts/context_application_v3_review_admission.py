"""ContextApplicationV3 semantic and V4 review admission."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from context_application_v3_resolver import ContextApplicationV3Resolver
from context_application_v3_review_binding import (
    ContextApplicationV3ReviewBindingError,
    ContextApplicationV3ReviewBindingResult,
    bind_context_application_v3_review,
)
from context_application_v3_validator import (
    ContextApplicationV3SemanticValidationError,
    ContextApplicationV3SemanticValidationResult,
    ContextApplicationV3SemanticValidator,
)
from mtgml.authority import ContextApplicationV3Record


class ContextApplicationV3ReviewAdmissionError(ValueError):
    def __init__(self, code: str, location: str, *, cause_code: str | None = None) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        super().__init__(f"{code} at {location}")


@dataclass(frozen=True)
class ContextApplicationV3ReviewAdmissionResult:
    record_id: str
    application_id: str
    semantic_validation: ContextApplicationV3SemanticValidationResult
    review_binding: ContextApplicationV3ReviewBindingResult


def admit_context_application_v3_record(
    record: ContextApplicationV3Record,
    resolver: ContextApplicationV3Resolver,
) -> ContextApplicationV3ReviewAdmissionResult:
    if not isinstance(record, ContextApplicationV3Record):
        raise ContextApplicationV3ReviewAdmissionError("APPLICATION_INPUT_INVALID", "record")
    try:
        semantic = ContextApplicationV3SemanticValidator(resolver).validate(record)
    except ContextApplicationV3SemanticValidationError as exc:
        raise ContextApplicationV3ReviewAdmissionError(
            exc.code,
            exc.location,
            cause_code=exc.cause_code,
        ) from exc
    try:
        binding = bind_context_application_v3_review(record, resolver)
    except ContextApplicationV3ReviewBindingError as exc:
        raise ContextApplicationV3ReviewAdmissionError(
            exc.code,
            exc.location,
            cause_code=exc.cause_code,
        ) from exc
    return ContextApplicationV3ReviewAdmissionResult(
        record_id=record.record_id.as_text(),
        application_id=record.application_id.as_text(),
        semantic_validation=semantic,
        review_binding=binding,
    )


__all__ = [
    "ContextApplicationV3ReviewAdmissionError",
    "ContextApplicationV3ReviewAdmissionResult",
    "admit_context_application_v3_record",
]
