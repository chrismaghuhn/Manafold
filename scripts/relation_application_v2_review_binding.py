"""RPA-specific adapter over the generic ADR-0045 V4 review binder."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from mtgml.authority import (
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    DigestReferenceV1,
    RelationApplicationV2Record,
    ReviewAcceptanceEventLeafV4,
    ReviewAuthoritySourceBindingV4,
    ReviewEventRefV4,
)
from review_acceptance_v4 import (
    ReviewAcceptanceV4Binding,
    ReviewAcceptanceV4BindingError,
    ReviewAcceptanceV4Resolver,
    bind_review_acceptance_v4,
)


class RelationApplicationV2ReviewBindingError(ValueError):
    """Stable RPA V2 review-binding failure."""

    def __init__(self, code: str, location: str, *, cause_code: str | None = None) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        super().__init__(f"{code} at {location}")


@dataclass(frozen=True)
class RelationApplicationV2ReviewBindingResult:
    record_id: str
    application_id: str
    subject_kind: AcceptanceSubjectKindV4
    subject_digest_reference: DigestReferenceV1
    review_event_ref: ReviewEventRefV4
    event: ReviewAcceptanceEventLeafV4
    exact_event_closure: tuple[ReviewAuthoritySourceBindingV4, ...]


def bind_relation_application_v2_review(
    record: RelationApplicationV2Record,
    resolver: ReviewAcceptanceV4Resolver,
    expected_source_bindings: tuple[ReviewAuthoritySourceBindingV4, ...],
) -> RelationApplicationV2ReviewBindingResult:
    if not isinstance(record, RelationApplicationV2Record):
        raise RelationApplicationV2ReviewBindingError("APPLICATION_INPUT_INVALID", "record")
    subject = AcceptanceSubjectPayloadV4(
        AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
        record.acceptance_free_subject_payload(),
    )
    try:
        bound: ReviewAcceptanceV4Binding = bind_review_acceptance_v4(
            subject,
            record.review_event_ref_v4,
            resolver,
            expected_source_bindings,
        )
    except ReviewAcceptanceV4BindingError as exc:
        code_map = {
            "SUBJECT_KIND_MISMATCH": "RPA_V2_SUBJECT_KIND_MISMATCH",
            "SUBJECT_DIGEST_MISMATCH": "RPA_V2_SUBJECT_DIGEST_MISMATCH",
            "SOURCE_CLOSURE_MISMATCH": "RPA_V2_SOURCE_CLOSURE_MISMATCH",
            "REVIEWER_ROLE_MISSING": "REVIEWER_ROLE_MISSING",
            "REVIEW_EVIDENCE_MISSING": "REVIEW_EVIDENCE_MISSING",
        }
        raise RelationApplicationV2ReviewBindingError(
            code_map.get(exc.code, "RPA_V2_REVIEW_BINDING_INVALID"),
            "review_event",
            cause_code=exc.code,
        ) from exc
    return RelationApplicationV2ReviewBindingResult(
        record_id=record.record_id.as_text(),
        application_id=record.application_id.as_text(),
        subject_kind=subject.subject_kind,
        subject_digest_reference=DigestReferenceV1.from_identity(subject.identity()),
        review_event_ref=record.review_event_ref_v4,
        event=bound.event,
        exact_event_closure=expected_source_bindings,
    )


__all__ = [
    "RelationApplicationV2ReviewBindingError",
    "RelationApplicationV2ReviewBindingResult",
    "bind_relation_application_v2_review",
]
