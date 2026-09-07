"""Read-only V3 review admission for ContextApplicationV2 records."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import AuthoritySourceResolver, ResolutionError
from context_application_v2_review_binding import (
    REQUIRED_V2_ROLES,
    ContextApplicationV2V3ReviewBindingError,
    admit_v3_review_binding,
)
from context_application_v2_validator import (
    ContextApplicationV2SemanticValidationError,
    ContextApplicationV2SemanticValidator,
)
from mtgml.authority import (
    AuthorityIdentityV1,
    ContextApplicationV2Record,
    ContextAuthoritySourceBindingV2,
    DigestReferenceV1,
    ReviewerRosterRefV1,
    ReviewEventRefV3,
    ReviewMode,
)


@dataclass(frozen=True)
class ContextApplicationV2ReviewAdmissionResult:
    record_id: AuthorityIdentityV1
    application_id: AuthorityIdentityV1
    subject_digest_reference: DigestReferenceV1
    review_event_ref: ReviewEventRefV3
    event_id: str
    exact_event_closure: tuple[ContextAuthoritySourceBindingV2, ...]
    reviewer_roster_ref: ReviewerRosterRefV1
    required_roles: tuple[str, ...]
    review_mode: ReviewMode


class ContextApplicationV2ReviewAdmissionError(ValueError):
    """Stable, read-only admission failure."""

    def __init__(
        self,
        code: str,
        location: str,
        *,
        cause_code: str | None = None,
        missing_role: str | None = None,
    ) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        self.missing_role = missing_role
        self.message = f"{code} at {location}"
        super().__init__(self.message)


class ContextApplicationV2ReviewAdmissionValidator:
    """Compose Slice-3 semantics with exact V3 review admission."""

    def __init__(
        self,
        source_resolver: AuthoritySourceResolver,
        *,
        base_authority_binding: ContextAuthoritySourceBindingV2,
    ) -> None:
        self._source_resolver = source_resolver
        self._base_binding = base_authority_binding
        self._semantic_validator = ContextApplicationV2SemanticValidator(
            source_resolver,
            base_authority_binding=base_authority_binding,
        )

    def admit(
        self,
        record: ContextApplicationV2Record,
    ) -> ContextApplicationV2ReviewAdmissionResult:
        if not isinstance(record, ContextApplicationV2Record):
            raise ContextApplicationV2ReviewAdmissionError(
                "APPLICATION_INPUT_INVALID",
                "record",
            )

        try:
            self._semantic_validator.validate(record)
        except ContextApplicationV2SemanticValidationError as exc:
            raise ContextApplicationV2ReviewAdmissionError(
                "SEMANTIC_VALIDATION_FAILED",
                exc.location,
                cause_code=exc.code,
            ) from exc
        except ResolutionError as exc:
            raise ContextApplicationV2ReviewAdmissionError(
                "SEMANTIC_VALIDATION_FAILED",
                "record",
                cause_code=exc.code,
            ) from exc

        try:
            binding = admit_v3_review_binding(
                record,
                self._source_resolver,
                base_authority_binding=self._base_binding,
            )
        except ContextApplicationV2V3ReviewBindingError as exc:
            raise ContextApplicationV2ReviewAdmissionError(
                exc.code,
                exc.location,
                cause_code=exc.cause_code,
                missing_role=exc.missing_role,
            ) from exc

        return ContextApplicationV2ReviewAdmissionResult(
            record_id=record.record_id,
            application_id=record.application_id,
            subject_digest_reference=binding.subject_digest_reference,
            review_event_ref=binding.review_event_ref,
            event_id=binding.event_id,
            exact_event_closure=binding.exact_event_closure,
            reviewer_roster_ref=binding.reviewer_roster_ref,
            required_roles=binding.required_roles,
            review_mode=binding.review_mode,
        )


__all__ = [
    "REQUIRED_V2_ROLES",
    "ContextApplicationV2ReviewAdmissionError",
    "ContextApplicationV2ReviewAdmissionResult",
    "ContextApplicationV2ReviewAdmissionValidator",
]
