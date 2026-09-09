"""Generic, rules-neutral V4 record-to-review binding.

This module binds an already typed V4 subject to its already typed V4 event.
It deliberately does not admit RPA or Context records and does not resolve
filesystem semantics; those adapters belong to later slices.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol

from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectPayloadV4,
    ReviewAcceptanceEventLeafV4,
    ReviewAuthoritySourceBindingV4,
    ReviewEventRefV4,
    validate_v4_event_source_bindings,
)
from mtgml.persistence import encode_canonical


class ReviewAcceptanceV4BindingError(ValueError):
    """Stable generic V4 record-to-review binding failure."""

    def __init__(self, code: str, message: str) -> None:
        self.code = code
        super().__init__(message)


class ReviewAcceptanceV4Resolver(Protocol):
    def resolve_acceptance_event_leaf_v4(
        self, reference: ReviewEventRefV4
    ) -> ReviewAcceptanceEventLeafV4: ...

    def resolve_v4_source_binding(self, binding: ReviewAuthoritySourceBindingV4) -> object: ...

    def resolve_v4_acceptance_evidence(self, evidence: AcceptanceEvidenceRefV1) -> object: ...


@dataclass(frozen=True)
class ReviewAcceptanceV4Binding:
    subject: AcceptanceSubjectPayloadV4
    event: ReviewAcceptanceEventLeafV4
    event_ref: ReviewEventRefV4


def bind_review_acceptance_v4(
    subject: AcceptanceSubjectPayloadV4,
    event_ref: ReviewEventRefV4,
    resolver: ReviewAcceptanceV4Resolver,
    expected_source_bindings: tuple[ReviewAuthoritySourceBindingV4, ...],
) -> ReviewAcceptanceV4Binding:
    """Resolve and bind one exact V4 subject/event pair and source closure."""

    event = resolver.resolve_acceptance_event_leaf_v4(event_ref)
    if event.subject_kind is not subject.subject_kind:
        raise ReviewAcceptanceV4BindingError(
            "SUBJECT_KIND_MISMATCH", "V4 subject kind does not match the event"
        )
    expected_digest = subject.identity().digest_bytes
    if event.subject_payload_digest_reference.digest_bytes != expected_digest:
        raise ReviewAcceptanceV4BindingError(
            "SUBJECT_DIGEST_MISMATCH", "V4 event subject digest does not match the subject"
        )
    required_roles = {
        "architecture_maintainer",
        "rules_authority_maintainer",
        "conformance_maintainer",
        "information_safety_reviewer",
    }
    observed_roles = {role for binding in event.reviewer_role_bindings for role in binding.roles}
    missing_roles = required_roles - observed_roles
    if missing_roles:
        raise ReviewAcceptanceV4BindingError(
            "REVIEWER_ROLE_MISSING",
            "V4 event reviewer_role_bindings omit required roles",
        )
    if not event.review_evidence_refs:
        raise ReviewAcceptanceV4BindingError(
            "REVIEW_EVIDENCE_MISSING", "V4 event review_evidence_refs must be non-empty"
        )
    validate_v4_event_source_bindings(event.event_id, event.source_binding_digests)
    actual = [encode_canonical(binding.to_cbor()) for binding in event.source_binding_digests]
    expected = [encode_canonical(binding.to_cbor()) for binding in expected_source_bindings]
    if actual != expected:
        raise ReviewAcceptanceV4BindingError(
            "SOURCE_CLOSURE_MISMATCH", "V4 event source bindings do not equal the expected closure"
        )
    for binding in expected_source_bindings:
        resolver.resolve_v4_source_binding(binding)
    for evidence in event.review_evidence_refs:
        resolver.resolve_v4_acceptance_evidence(evidence)
    return ReviewAcceptanceV4Binding(subject=subject, event=event, event_ref=event_ref)
