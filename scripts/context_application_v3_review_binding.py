"""Generic V4 review binding adapter for ContextApplicationV3 records."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from context_application_v3_resolver import (
    ContextApplicationV3ResolutionError,
    ContextApplicationV3Resolver,
)
from mtgml.authority import (
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    ContextApplicationV3Record,
    DigestReferenceV1,
    ReviewEventRefV4,
)
from review_acceptance_v4 import ReviewAcceptanceV4BindingError, bind_review_acceptance_v4


class ContextApplicationV3ReviewBindingError(ValueError):
    def __init__(self, code: str, location: str, *, cause_code: str | None = None) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        super().__init__(f"{code} at {location}")


@dataclass(frozen=True)
class ContextApplicationV3ReviewBindingResult:
    record_id: str
    application_id: str
    subject_digest_reference: DigestReferenceV1
    review_event_ref: ReviewEventRefV4
    event_id: str
    exact_event_closure: tuple[object, ...]


def bind_context_application_v3_review(
    record: ContextApplicationV3Record,
    resolver: ContextApplicationV3Resolver,
) -> ContextApplicationV3ReviewBindingResult:
    if not isinstance(record, ContextApplicationV3Record):
        raise ContextApplicationV3ReviewBindingError("APPLICATION_INPUT_INVALID", "record")
    subject = AcceptanceSubjectPayloadV4(
        AcceptanceSubjectKindV4.CONTEXT_APPLICATION_V3_RECORD,
        record.acceptance_free_subject_payload(),
    )
    try:
        event = resolver.resolve_acceptance_event_leaf_v4(record.review_event_ref_v4)
        expected = resolver.expected_context_application_v3_source_closure(
            record,
            event.reviewer_roster_ref,
        )
        bound = bind_review_acceptance_v4(
            subject,
            record.review_event_ref_v4,
            resolver,
            expected,
        )
    except ReviewAcceptanceV4BindingError as exc:
        mapping = {
            "SUBJECT_KIND_MISMATCH": "CONTEXT_APPLICATION_V3_SUBJECT_KIND_MISMATCH",
            "SUBJECT_DIGEST_MISMATCH": "CONTEXT_APPLICATION_V3_SUBJECT_DIGEST_MISMATCH",
            "SOURCE_CLOSURE_MISMATCH": "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
        }
        raise ContextApplicationV3ReviewBindingError(
            mapping.get(exc.code, "CONTEXT_APPLICATION_V3_REVIEW_BINDING_INVALID"),
            "review_event",
            cause_code=exc.code,
        ) from exc
    except ContextApplicationV3ResolutionError as exc:
        raise ContextApplicationV3ReviewBindingError(
            exc.code,
            exc.location,
            cause_code=exc.cause_code,
        ) from exc
    except Exception as exc:
        raise ContextApplicationV3ReviewBindingError(
            "CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH",
            "review_event.source_binding_digests",
            cause_code=type(exc).__name__,
        ) from exc
    return ContextApplicationV3ReviewBindingResult(
        record_id=record.record_id.as_text(),
        application_id=record.application_id.as_text(),
        subject_digest_reference=DigestReferenceV1.from_identity(subject.identity()),
        review_event_ref=record.review_event_ref_v4,
        event_id=bound.event.event_id.as_text(),
        exact_event_closure=tuple(expected),
    )


__all__ = [
    "ContextApplicationV3ReviewBindingError",
    "ContextApplicationV3ReviewBindingResult",
    "bind_context_application_v3_review",
]
