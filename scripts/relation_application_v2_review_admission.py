"""RPA V2 semantic plus current-theorem plus V4 review admission."""

from __future__ import annotations

import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import ResolutionError
from mtgml.authority import (
    RelationApplicationV2,
    RelationApplicationV2Record,
    ReviewAuthoritySourceBindingV4,
)
from relation_application_v2_resolver import RelationApplicationV2ResolutionError
from relation_application_v2_review_binding import (
    RelationApplicationV2ReviewBindingError,
    RelationApplicationV2ReviewBindingResult,
    bind_relation_application_v2_review,
    reconstruct_relation_application_v2_source_closure,
)
from relation_application_v2_validator import (
    RelationApplicationV2SemanticValidationError,
    RelationApplicationV2SemanticValidationResult,
    validate_relation_application_v2_semantics,
)


class RelationApplicationV2Currentness(Protocol):
    def require_current_relation_theorem(
        self, theorem_record_id: object
    ) -> Mapping[str, object]: ...

    def resolve_relation_theorem_record(
        self, theorem_record_id: object
    ) -> Mapping[str, object]: ...


class RelationApplicationV2ReviewAdmissionError(ValueError):
    """Stable RPA V2 admission failure."""

    def __init__(self, code: str, location: str, *, cause_code: str | None = None) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        super().__init__(f"{code} at {location}")


@dataclass(frozen=True)
class RelationApplicationV2ReviewAdmissionResult:
    record_id: str
    application_id: str
    semantic_validation: RelationApplicationV2SemanticValidationResult
    review_binding: RelationApplicationV2ReviewBindingResult


def admit_relation_application_v2_record(
    record: RelationApplicationV2Record,
    resolver: object,
    *,
    theorem_record: Mapping[str, object] | None = None,
    currentness: RelationApplicationV2Currentness,
    expected_source_bindings: tuple[ReviewAuthoritySourceBindingV4, ...] | None = None,
    require_current_theorem: bool = True,
) -> RelationApplicationV2ReviewAdmissionResult:
    if not isinstance(record, RelationApplicationV2Record):
        raise RelationApplicationV2ReviewAdmissionError("APPLICATION_INPUT_INVALID", "record")
    application = RelationApplicationV2(
        theorem_record_id_bytes=record.theorem_record_id.digest_bytes,
        terminal_disposition=record.terminal_disposition,
        members=record.members,
    )
    if application.identity() != record.application_id:
        raise RelationApplicationV2ReviewAdmissionError(
            "RELATION_APPLICATION_V2_IDENTITY_MISMATCH", "application_id"
        )
    try:
        if require_current_theorem:
            resolved_theorem = currentness.require_current_relation_theorem(
                record.theorem_record_id
            )
        else:
            resolver_for_historical = getattr(currentness, "resolve_relation_theorem_record", None)
            if not callable(resolver_for_historical):
                raise RelationApplicationV2ReviewAdmissionError(
                    "RELATION_APPLICATION_V2_THEOREM_MISMATCH", "theorem_record_id"
                )
            resolved_theorem = resolver_for_historical(record.theorem_record_id)
    except RelationApplicationV2ReviewAdmissionError:
        raise
    except RelationApplicationV2ResolutionError as exc:
        code = exc.code
        if code not in {
            "SUPERSEDED_AUTHORITY_USED",
            "RELATION_APPLICATION_V2_CURRENTNESS_FAILED",
            "RELATION_APPLICATION_V2_CURRENTNESS_AMBIGUOUS",
        }:
            code = "RELATION_APPLICATION_V2_CURRENTNESS_FAILED"
        raise RelationApplicationV2ReviewAdmissionError(
            code, "theorem_record_id", cause_code=exc.code
        ) from exc
    except ResolutionError as exc:
        code = exc.code
        if code not in {
            "SUPERSEDED_AUTHORITY_USED",
            "RELATION_APPLICATION_V2_CURRENTNESS_FAILED",
            "RELATION_APPLICATION_V2_CURRENTNESS_AMBIGUOUS",
        }:
            code = "RELATION_APPLICATION_V2_CURRENTNESS_FAILED"
        raise RelationApplicationV2ReviewAdmissionError(
            code, "theorem_record_id", cause_code=exc.code
        ) from exc
    except Exception as exc:
        raise RelationApplicationV2ReviewAdmissionError(
            "RELATION_APPLICATION_V2_CURRENTNESS_FAILED",
            "theorem_record_id",
            cause_code=type(exc).__name__,
        ) from exc
    if not isinstance(resolved_theorem, Mapping) or not resolved_theorem:
        raise RelationApplicationV2ReviewAdmissionError(
            "RELATION_APPLICATION_V2_THEOREM_MISMATCH", "theorem_record"
        )
    if theorem_record is not None and dict(theorem_record) != dict(resolved_theorem):
        raise RelationApplicationV2ReviewAdmissionError(
            "RELATION_APPLICATION_V2_THEOREM_MISMATCH", "theorem_record"
        )
    try:
        semantic = validate_relation_application_v2_semantics(
            application,
            resolver,
            theorem_record=resolved_theorem,
        )
    except RelationApplicationV2SemanticValidationError as exc:
        raise RelationApplicationV2ReviewAdmissionError(
            exc.code, exc.location, cause_code=exc.code
        ) from exc
    try:
        event = resolver.resolve_acceptance_event_leaf_v4(record.review_event_ref_v4)
        reconstructed_source_bindings = reconstruct_relation_application_v2_source_closure(
            record,
            resolver,
            event.reviewer_roster_ref,
        )
        if (
            expected_source_bindings is not None
            and tuple(expected_source_bindings) != reconstructed_source_bindings
        ):
            raise RelationApplicationV2ReviewBindingError(
                "RPA_V2_SOURCE_CLOSURE_MISMATCH",
                "review_event.source_binding_digests",
            )
    except RelationApplicationV2ReviewBindingError as exc:
        raise RelationApplicationV2ReviewAdmissionError(
            exc.code, exc.location, cause_code=exc.cause_code
        ) from exc
    except Exception as exc:
        raise RelationApplicationV2ReviewAdmissionError(
            "RPA_V2_SOURCE_CLOSURE_RECONSTRUCTION_FAILED",
            "review_event.source_binding_digests",
            cause_code=type(exc).__name__,
        ) from exc
    try:
        binding = bind_relation_application_v2_review(
            record,
            resolver,
            reconstructed_source_bindings,
        )
    except RelationApplicationV2ReviewBindingError as exc:
        raise RelationApplicationV2ReviewAdmissionError(
            exc.code, exc.location, cause_code=exc.cause_code
        ) from exc
    return RelationApplicationV2ReviewAdmissionResult(
        record_id=record.record_id.as_text(),
        application_id=record.application_id.as_text(),
        semantic_validation=semantic,
        review_binding=binding,
    )


__all__ = [
    "RelationApplicationV2Currentness",
    "RelationApplicationV2ReviewAdmissionError",
    "RelationApplicationV2ReviewAdmissionResult",
    "admit_relation_application_v2_record",
]
