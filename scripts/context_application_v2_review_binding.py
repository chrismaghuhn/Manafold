"""Shared read-only V3 review binding for ContextApplicationV2 subjects."""

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
from mtgml.authority import (
    AcceptanceSubjectKindV3,
    AcceptanceSubjectPayloadV3,
    AuthorityIdentityV1,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionRecord,
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
class ContextApplicationV2V3ReviewBindingResult:
    subject_kind: AcceptanceSubjectKindV3
    subject_digest_reference: DigestReferenceV1
    review_event_ref: ReviewEventRefV3
    event_id: str
    exact_event_closure: tuple[ContextAuthoritySourceBindingV2, ...]
    reviewer_roster_ref: ReviewerRosterRefV1
    required_roles: tuple[str, ...]
    review_mode: ReviewMode


class ContextApplicationV2V3ReviewBindingError(ValueError):
    """Stable failure from the shared mechanical V3 binding seam."""

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
        super().__init__(f"{code} at {location}")


def _expected_subject_kind(
    subject: object,
) -> AcceptanceSubjectKindV3:
    if isinstance(subject, ContextApplicationV2Record):
        return AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD
    if isinstance(subject, ContextApplicationV2SupersessionRecord):
        return AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD
    raise ContextApplicationV2V3ReviewBindingError(
        "APPLICATION_INPUT_INVALID",
        "subject",
    )


def _resolve_event(
    resolver: ContextApplicationV2Resolver,
    reference: ReviewEventRefV3,
) -> ResolvedReviewAcceptanceEventV3:
    try:
        return resolver.resolve_review_event_leaf_v3(reference)
    except ContextApplicationV2ResolutionError as exc:
        raise ContextApplicationV2V3ReviewBindingError(
            exc.code or "V3_EVENT_REFERENCE_INVALID",
            "review_event",
            cause_code=exc.cause_code or exc.code,
        ) from exc
    except ResolutionError as exc:
        raise ContextApplicationV2V3ReviewBindingError(
            "V3_EVENT_SOURCE_INVALID",
            "review_event",
            cause_code=exc.code,
        ) from exc


def _expected_closure(
    resolver: ContextApplicationV2Resolver,
    subject: ContextApplicationV2Record | ContextApplicationV2SupersessionRecord,
    event: ResolvedReviewAcceptanceEventV3,
) -> tuple[ContextAuthoritySourceBindingV2, ...]:
    try:
        expected = resolver.expected_acceptance_source_closure_v3(
            subject,
            event.event.reviewer_roster_ref,
        )
        require_exact_source_set(
            event.event.source_binding_digests,
            expected,
        )
        return expected
    except ContextApplicationV2ResolutionError as exc:
        raise ContextApplicationV2V3ReviewBindingError(
            "V3_SOURCE_CLOSURE_MISMATCH",
            "event.source_binding_digests",
            cause_code=exc.code,
        ) from exc
    except ResolutionError as exc:
        raise ContextApplicationV2V3ReviewBindingError(
            "V3_SOURCE_CLOSURE_MISMATCH",
            "event.source_binding_digests",
            cause_code=exc.code,
        ) from exc


def _validate_roster_and_roles(
    source_resolver: AuthoritySourceResolver,
    event: ReviewAcceptanceEventInputV3,
) -> None:
    try:
        roster = resolve_reviewer_roster(
            source_resolver,
            event.reviewer_roster_ref,
        )
    except ResolutionError as exc:
        raise ContextApplicationV2V3ReviewBindingError(
            "REVIEWER_ROSTER_INVALID",
            "event.reviewer_roster_ref",
            cause_code=exc.code,
        ) from exc
    except (TypeError, ValueError) as exc:
        raise ContextApplicationV2V3ReviewBindingError(
            "REVIEWER_ROSTER_INVALID",
            "event.reviewer_roster_ref",
        ) from exc

    bindings = event.reviewer_role_bindings
    reviewer_ids = tuple(binding.reviewer_id for binding in bindings)
    if len(set(reviewer_ids)) != len(reviewer_ids):
        raise ContextApplicationV2V3ReviewBindingError(
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
        raise ContextApplicationV2V3ReviewBindingError(
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
            raise ContextApplicationV2V3ReviewBindingError(
                code,
                f"event.reviewer_role_bindings.{role}",
                missing_role=role,
            )


def admit_v3_review_binding(
    subject: ContextApplicationV2Record | ContextApplicationV2SupersessionRecord,
    source_resolver: AuthoritySourceResolver,
    *,
    base_authority_binding: ContextAuthoritySourceBindingV2,
) -> ContextApplicationV2V3ReviewBindingResult:
    expected_kind = _expected_subject_kind(subject)
    reference = subject.review_event_ref_v3
    resolver = ContextApplicationV2Resolver(
        source_resolver,
        base_authority_binding=base_authority_binding,
    )
    resolved_event = _resolve_event(resolver, reference)
    if resolved_event.event.subject_kind is not expected_kind:
        raise ContextApplicationV2V3ReviewBindingError(
            "V3_SUBJECT_KIND_MISMATCH",
            "event.subject_kind",
        )

    subject_payload = AcceptanceSubjectPayloadV3(
        subject_kind=expected_kind,
        subject_payload=subject.acceptance_free_subject_payload(),
    )
    expected_reference = DigestReferenceV1.from_identity(subject_payload.identity())
    if expected_reference != resolved_event.event.subject_payload_digest_reference:
        raise ContextApplicationV2V3ReviewBindingError(
            "V3_SUBJECT_DIGEST_MISMATCH",
            "event.subject_payload_digest_reference",
        )

    expected_closure = _expected_closure(resolver, subject, resolved_event)
    _validate_roster_and_roles(source_resolver, resolved_event.event)
    if not resolved_event.event.review_evidence_refs:
        raise ContextApplicationV2V3ReviewBindingError(
            "REVIEW_EVIDENCE_MISSING",
            "event.review_evidence_refs",
        )

    return ContextApplicationV2V3ReviewBindingResult(
        subject_kind=expected_kind,
        subject_digest_reference=expected_reference,
        review_event_ref=reference,
        event_id=resolved_event.event_id,
        exact_event_closure=expected_closure,
        reviewer_roster_ref=resolved_event.event.reviewer_roster_ref,
        required_roles=REQUIRED_V2_ROLES,
        review_mode=resolved_event.event.review_mode,
    )


__all__ = [
    "REQUIRED_V2_ROLES",
    "ContextApplicationV2V3ReviewBindingError",
    "ContextApplicationV2V3ReviewBindingResult",
    "admit_v3_review_binding",
]
