"""Read-only V3 review admission for ContextApplicationV2 records."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Final

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import AuthoritySourceResolver, ResolutionError
from context_application_v2_resolver import (
    ContextApplicationV2ResolutionError,
    ContextApplicationV2Resolver,
    ResolvedReviewAcceptanceEventV3,
    require_exact_source_set,
)
from context_application_v2_validator import (
    ContextApplicationV2SemanticValidationError,
    ContextApplicationV2SemanticValidator,
)
from mtgml.authority import (
    AcceptanceSubjectKindV3,
    AcceptanceSubjectPayloadV3,
    AuthorityIdentityV1,
    ContextApplicationV2Record,
    ContextAuthoritySourceBindingV2,
    DigestReferenceV1,
    ReviewAcceptanceEventInputV3,
    ReviewerRosterRefV1,
    ReviewEventRefV3,
    ReviewMode,
)
from reviewer_role_binding import (
    ReviewerRoleBindingValidationError,
    resolve_reviewer_roster,
    validate_reviewer_binding_against_roster,
)

REQUIRED_V2_ROLES: Final = (
    "architecture_maintainer",
    "rules_authority_maintainer",
    "conformance_maintainer",
    "information_safety_reviewer",
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
        self._resolver = ContextApplicationV2Resolver(
            source_resolver,
            base_authority_binding=base_authority_binding,
        )
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

        resolved_event = self._resolve_event(record.review_event_ref_v3)
        if (
            resolved_event.event.subject_kind
            is not AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD
        ):
            raise ContextApplicationV2ReviewAdmissionError(
                "V3_SUBJECT_KIND_MISMATCH",
                "event.subject_kind",
            )

        subject = AcceptanceSubjectPayloadV3(
            subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
            subject_payload=record.acceptance_free_subject_payload(),
        )
        expected_subject_reference = DigestReferenceV1.from_identity(subject.identity())
        if expected_subject_reference != resolved_event.event.subject_payload_digest_reference:
            raise ContextApplicationV2ReviewAdmissionError(
                "V3_SUBJECT_DIGEST_MISMATCH",
                "event.subject_payload_digest_reference",
            )

        expected_closure = self._expected_closure(record, resolved_event)
        self._validate_roster_and_roles(resolved_event.event)

        if not resolved_event.event.review_evidence_refs:
            raise ContextApplicationV2ReviewAdmissionError(
                "REVIEW_EVIDENCE_MISSING",
                "event.review_evidence_refs",
            )

        return ContextApplicationV2ReviewAdmissionResult(
            record_id=record.record_id,
            application_id=record.application_id,
            subject_digest_reference=expected_subject_reference,
            review_event_ref=record.review_event_ref_v3,
            event_id=resolved_event.event_id,
            exact_event_closure=expected_closure,
            reviewer_roster_ref=resolved_event.event.reviewer_roster_ref,
            required_roles=REQUIRED_V2_ROLES,
            review_mode=resolved_event.event.review_mode,
        )

    def _resolve_event(self, reference: ReviewEventRefV3) -> ResolvedReviewAcceptanceEventV3:
        try:
            return self._resolver.resolve_review_event_leaf_v3(reference)
        except ContextApplicationV2ResolutionError as exc:
            raise ContextApplicationV2ReviewAdmissionError(
                exc.code or "V3_EVENT_REFERENCE_INVALID",
                "review_event",
                cause_code=exc.code,
            ) from exc
        except ResolutionError as exc:
            raise ContextApplicationV2ReviewAdmissionError(
                "V3_EVENT_SOURCE_INVALID",
                "review_event",
                cause_code=exc.code,
            ) from exc

    def _expected_closure(
        self,
        record: ContextApplicationV2Record,
        resolved_event: ResolvedReviewAcceptanceEventV3,
    ) -> tuple[ContextAuthoritySourceBindingV2, ...]:
        try:
            expected = self._resolver.expected_acceptance_source_closure_v3(
                record,
                resolved_event.event.reviewer_roster_ref,
            )
            require_exact_source_set(
                resolved_event.event.source_binding_digests,
                expected,
            )
            return expected
        except ContextApplicationV2ResolutionError as exc:
            raise ContextApplicationV2ReviewAdmissionError(
                "V3_SOURCE_CLOSURE_MISMATCH",
                "event.source_binding_digests",
                cause_code=exc.code,
            ) from exc
        except ResolutionError as exc:
            raise ContextApplicationV2ReviewAdmissionError(
                "V3_SOURCE_CLOSURE_MISMATCH",
                "event.source_binding_digests",
                cause_code=exc.code,
            ) from exc

    def _validate_roster_and_roles(self, event: ReviewAcceptanceEventInputV3) -> None:
        try:
            roster = resolve_reviewer_roster(
                self._source_resolver,
                event.reviewer_roster_ref,
            )
        except ResolutionError as exc:
            raise ContextApplicationV2ReviewAdmissionError(
                "REVIEWER_ROSTER_INVALID",
                "event.reviewer_roster_ref",
                cause_code=exc.code,
            ) from exc
        except (TypeError, ValueError) as exc:
            raise ContextApplicationV2ReviewAdmissionError(
                "REVIEWER_ROSTER_INVALID",
                "event.reviewer_roster_ref",
            ) from exc

        bindings = event.reviewer_role_bindings
        reviewer_ids = tuple(binding.reviewer_id for binding in bindings)
        if len(set(reviewer_ids)) != len(reviewer_ids):
            raise ContextApplicationV2ReviewAdmissionError(
                "REVIEWER_DUPLICATE",
                "event.reviewer_role_bindings",
            )

        try:
            for binding in bindings:
                validate_reviewer_binding_against_roster(binding, roster)
        except ReviewerRoleBindingValidationError as exc:
            code = (
                "REVIEWER_BINDING_NOT_IN_ROSTER"
                if exc.reason == "not_in_roster"
                else "REVIEWER_BINDING_INVALID"
            )
            raise ContextApplicationV2ReviewAdmissionError(
                code,
                f"event.reviewer_role_bindings.{exc.reviewer_id}",
            ) from exc

        role_union = frozenset(role for binding in bindings for role in binding.roles)
        for role in REQUIRED_V2_ROLES:
            if role not in role_union:
                code = (
                    "INFORMATION_SAFETY_REVIEWER_REQUIRED"
                    if role == "information_safety_reviewer"
                    else "REVIEWER_ROLE_MISSING"
                )
                raise ContextApplicationV2ReviewAdmissionError(
                    code,
                    f"event.reviewer_role_bindings.{role}",
                    missing_role=role,
                )


__all__ = [
    "REQUIRED_V2_ROLES",
    "ContextApplicationV2ReviewAdmissionError",
    "ContextApplicationV2ReviewAdmissionResult",
    "ContextApplicationV2ReviewAdmissionValidator",
]
