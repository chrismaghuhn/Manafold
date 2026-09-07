"""Read-only ContextApplicationV2 supersession admission and currentness."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Final

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import AuthoritySourceResolver
from context_application_v2_review_binding import (
    ContextApplicationV2V3ReviewBindingError,
    admit_v3_review_binding,
)
from mtgml.authority import (
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV2SupersessionInputV2,
    ContextApplicationV2SupersessionRecord,
    ContextApplicationV2SupersessionRecordInputV1,
    ContextAuthoritySourceBindingV2,
    DigestReferenceV1,
    EvidenceRefV1,
    ReviewEventRefV3,
    ReviewMode,
    ReviewerRosterRefV1,
    SupersessionReason,
)
from mtgml.persistence import encode_canonical


SUPERSESSION_ERROR_CODES: Final = frozenset(
    {
        "SUPERSESSION_INPUT_INVALID",
        "SUPERSESSION_IDENTITY_MISMATCH",
        "SUPERSESSION_RECORD_IDENTITY_MISMATCH",
        "SUPERSESSION_REVIEW_ADMISSION_FAILED",
        "SUPERSESSION_REASON_INVALID",
        "SUPERSESSION_REPLACEMENT_INVALID",
    }
)


@dataclass(frozen=True)
class ContextApplicationV2SupersessionAdmissionResult:
    record_id: AuthorityIdentityV1
    supersession_id: AuthorityIdentityV1
    superseded_record_id: AuthorityIdentityV1
    replacement_record_id: AuthorityIdentityV1 | None
    reason_code: SupersessionReason
    subject_digest_reference: DigestReferenceV1
    review_event_ref: ReviewEventRefV3
    event_id: str
    exact_event_closure: tuple[ContextAuthoritySourceBindingV2, ...]
    reviewer_roster_ref: ReviewerRosterRefV1
    required_roles: tuple[str, ...]
    review_mode: ReviewMode


class ContextApplicationV2SupersessionError(ValueError):
    """Stable read-only per-record supersession admission failure."""

    def __init__(
        self,
        code: str,
        location: str,
        *,
        cause_code: str | None = None,
        record_id: AuthorityIdentityV1 | None = None,
        supersession_id: AuthorityIdentityV1 | None = None,
    ) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        self.record_id = record_id
        self.supersession_id = supersession_id
        super().__init__(f"{code} at {location}")


def _identity_kind_valid(
    value: object,
    kind: AuthorityIdentityKind,
) -> bool:
    return isinstance(value, AuthorityIdentityV1) and value.kind is kind


def _require_structural_invariants(record: ContextApplicationV2SupersessionRecord) -> None:
    if not _identity_kind_valid(
        record.record_id,
        AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V2,
    ):
        raise ContextApplicationV2SupersessionError(
            "SUPERSESSION_RECORD_IDENTITY_MISMATCH",
            "record_id",
            record_id=record.record_id
            if isinstance(record.record_id, AuthorityIdentityV1)
            else None,
        )
    if not _identity_kind_valid(
        record.supersession_id,
        AuthorityIdentityKind.CONTEXT_SUPERSESSION_V2,
    ):
        raise ContextApplicationV2SupersessionError(
            "SUPERSESSION_IDENTITY_MISMATCH",
            "supersession_id",
            supersession_id=record.supersession_id
            if isinstance(record.supersession_id, AuthorityIdentityV1)
            else None,
        )
    if not _identity_kind_valid(
        record.superseded_record_id,
        AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2,
    ):
        raise ContextApplicationV2SupersessionError(
            "SUPERSESSION_INPUT_INVALID",
            "superseded_record_id",
            record_id=record.record_id,
            supersession_id=record.supersession_id,
        )
    if record.replacement_record_id is not None and not _identity_kind_valid(
        record.replacement_record_id,
        AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2,
    ):
        raise ContextApplicationV2SupersessionError(
            "SUPERSESSION_REPLACEMENT_INVALID",
            "replacement_record_id",
            record_id=record.record_id,
            supersession_id=record.supersession_id,
        )
    if not isinstance(record.reason_code, SupersessionReason):
        raise ContextApplicationV2SupersessionError(
            "SUPERSESSION_REASON_INVALID",
            "reason_code",
            record_id=record.record_id,
            supersession_id=record.supersession_id,
        )
    if record.replacement_record_id is None:
        if record.reason_code is not SupersessionReason.AUTHORITY_REVOCATION:
            raise ContextApplicationV2SupersessionError(
                "SUPERSESSION_REASON_INVALID",
                "replacement_record_id",
                record_id=record.record_id,
                supersession_id=record.supersession_id,
            )
    elif record.reason_code is SupersessionReason.AUTHORITY_REVOCATION:
        raise ContextApplicationV2SupersessionError(
            "SUPERSESSION_REASON_INVALID",
            "reason_code",
            record_id=record.record_id,
            supersession_id=record.supersession_id,
        )
    if not isinstance(record.source_evidence_refs, tuple) or not record.source_evidence_refs:
        raise ContextApplicationV2SupersessionError(
            "SUPERSESSION_INPUT_INVALID",
            "source_evidence_refs",
            record_id=record.record_id,
            supersession_id=record.supersession_id,
        )
    if not all(isinstance(reference, EvidenceRefV1) for reference in record.source_evidence_refs):
        raise ContextApplicationV2SupersessionError(
            "SUPERSESSION_INPUT_INVALID",
            "source_evidence_refs",
            record_id=record.record_id,
            supersession_id=record.supersession_id,
        )
    evidence_keys = tuple(
        encode_canonical(reference.to_cbor()) for reference in record.source_evidence_refs
    )
    if evidence_keys != tuple(sorted(evidence_keys)) or len(set(evidence_keys)) != len(
        evidence_keys
    ):
        raise ContextApplicationV2SupersessionError(
            "SUPERSESSION_INPUT_INVALID",
            "source_evidence_refs",
            record_id=record.record_id,
            supersession_id=record.supersession_id,
        )
    if not isinstance(record.review_event_ref_v3, ReviewEventRefV3):
        raise ContextApplicationV2SupersessionError(
            "SUPERSESSION_INPUT_INVALID",
            "review_event_ref_v3",
            record_id=record.record_id,
            supersession_id=record.supersession_id,
        )


class ContextApplicationV2SupersessionAdmissionValidator:
    """Admit one immutable supersession record without graph interpretation."""

    def __init__(
        self,
        source_resolver: AuthoritySourceResolver,
        *,
        base_authority_binding: ContextAuthoritySourceBindingV2,
    ) -> None:
        self._source_resolver = source_resolver
        self._base_binding = base_authority_binding

    def admit(
        self,
        record: ContextApplicationV2SupersessionRecord,
    ) -> ContextApplicationV2SupersessionAdmissionResult:
        if not isinstance(record, ContextApplicationV2SupersessionRecord):
            raise ContextApplicationV2SupersessionError(
                "SUPERSESSION_INPUT_INVALID",
                "record",
            )

        _require_structural_invariants(record)
        semantic_input = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=record.superseded_record_id.digest_bytes,
            replacement_record_id_bytes=(
                None
                if record.replacement_record_id is None
                else record.replacement_record_id.digest_bytes
            ),
            replacement_record_kind=(
                None
                if record.replacement_record_id is None
                else "context_application_v2_record"
            ),
            reason_code=record.reason_code,
            source_evidence_refs=record.source_evidence_refs,
        )
        expected_supersession_id = semantic_input.identity()
        if expected_supersession_id != record.supersession_id:
            raise ContextApplicationV2SupersessionError(
                "SUPERSESSION_IDENTITY_MISMATCH",
                "supersession_id",
                supersession_id=record.supersession_id,
            )

        expected_record_id = ContextApplicationV2SupersessionRecordInputV1(
            record.supersession_id.digest_bytes,
            record.review_event_ref_v3,
        ).identity()
        if expected_record_id != record.record_id:
            raise ContextApplicationV2SupersessionError(
                "SUPERSESSION_RECORD_IDENTITY_MISMATCH",
                "record_id",
                record_id=record.record_id,
            )

        try:
            binding = admit_v3_review_binding(
                record,
                self._source_resolver,
                base_authority_binding=self._base_binding,
            )
        except ContextApplicationV2V3ReviewBindingError as exc:
            raise ContextApplicationV2SupersessionError(
                "SUPERSESSION_REVIEW_ADMISSION_FAILED",
                exc.location,
                cause_code=exc.code,
                record_id=record.record_id,
                supersession_id=record.supersession_id,
            ) from exc

        return ContextApplicationV2SupersessionAdmissionResult(
            record_id=record.record_id,
            supersession_id=record.supersession_id,
            superseded_record_id=record.superseded_record_id,
            replacement_record_id=record.replacement_record_id,
            reason_code=record.reason_code,
            subject_digest_reference=binding.subject_digest_reference,
            review_event_ref=binding.review_event_ref,
            event_id=binding.event_id,
            exact_event_closure=binding.exact_event_closure,
            reviewer_roster_ref=binding.reviewer_roster_ref,
            required_roles=binding.required_roles,
            review_mode=binding.review_mode,
        )


__all__ = [
    "ContextApplicationV2SupersessionAdmissionResult",
    "ContextApplicationV2SupersessionAdmissionValidator",
    "ContextApplicationV2SupersessionError",
]
