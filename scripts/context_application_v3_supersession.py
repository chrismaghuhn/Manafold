"""ContextApplicationV3 supersession admission and derived currentness."""

from __future__ import annotations

from collections.abc import Callable, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Final

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
import sys

if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from mtgml.authority import (
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV3Record,
    ContextApplicationV3SupersessionInputV1,
    ContextApplicationV3SupersessionRecord,
    ContextApplicationV3SupersessionRecordInputV1,
    EvidenceRefV1,
    ReviewEventRefV4,
    SupersessionReason,
)
from mtgml.persistence import encode_canonical
from review_acceptance_v4 import ReviewAcceptanceV4BindingError, bind_review_acceptance_v4

SUPERSESSION_ERROR_CODES: Final = frozenset(
    {
        "CONTEXT_APPLICATION_V3_SUPERSESSION_INPUT_INVALID",
        "CONTEXT_APPLICATION_V3_SUPERSESSION_IDENTITY_MISMATCH",
        "CONTEXT_APPLICATION_V3_SUPERSESSION_RECORD_IDENTITY_MISMATCH",
        "CONTEXT_APPLICATION_V3_SUPERSESSION_REVIEW_ADMISSION_FAILED",
        "CONTEXT_APPLICATION_V3_SUPERSESSION_REASON_INVALID",
        "CONTEXT_APPLICATION_V3_SUPERSESSION_REPLACEMENT_INVALID",
    }
)


class ContextApplicationV3SupersessionError(ValueError):
    def __init__(self, code: str, location: str, *, cause_code: str | None = None) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        super().__init__(f"{code} at {location}")


@dataclass(frozen=True)
class ContextApplicationV3SupersessionAdmissionResult:
    record_id: AuthorityIdentityV1
    supersession_id: AuthorityIdentityV1
    superseded_record_id: AuthorityIdentityV1
    replacement_record_id: AuthorityIdentityV1 | None
    reason_code: SupersessionReason
    review_event_ref: ReviewEventRefV4
    event_id: str
    exact_event_closure: tuple[object, ...]


def _identity(value: object, kind: AuthorityIdentityKind, location: str) -> AuthorityIdentityV1:
    if not isinstance(value, AuthorityIdentityV1) or value.kind is not kind:
        raise ContextApplicationV3SupersessionError(
            "CONTEXT_APPLICATION_V3_SUPERSESSION_INPUT_INVALID", location
        )
    return value


def _validate_record_shape(record: ContextApplicationV3SupersessionRecord) -> None:
    _identity(record.record_id, AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V3, "record_id")
    _identity(
        record.supersession_id, AuthorityIdentityKind.CONTEXT_SUPERSESSION_V3, "supersession_id"
    )
    _identity(
        record.superseded_record_id,
        AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V3,
        "superseded_record_id",
    )
    if record.replacement_record_id is not None:
        _identity(
            record.replacement_record_id,
            AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V3,
            "replacement_record_id",
        )
    if (
        record.replacement_record_id is None
        and record.reason_code is not SupersessionReason.AUTHORITY_REVOCATION
    ):
        raise ContextApplicationV3SupersessionError(
            "CONTEXT_APPLICATION_V3_SUPERSESSION_REASON_INVALID", "replacement_record_id"
        )
    if (
        record.replacement_record_id is not None
        and record.reason_code is SupersessionReason.AUTHORITY_REVOCATION
    ):
        raise ContextApplicationV3SupersessionError(
            "CONTEXT_APPLICATION_V3_SUPERSESSION_REASON_INVALID", "reason_code"
        )
    if not isinstance(record.source_evidence_refs, tuple) or not record.source_evidence_refs:
        raise ContextApplicationV3SupersessionError(
            "CONTEXT_APPLICATION_V3_SUPERSESSION_INPUT_INVALID", "source_evidence_refs"
        )
    if not all(isinstance(item, EvidenceRefV1) for item in record.source_evidence_refs):
        raise ContextApplicationV3SupersessionError(
            "CONTEXT_APPLICATION_V3_SUPERSESSION_INPUT_INVALID", "source_evidence_refs"
        )
    keys = tuple(encode_canonical(item.to_cbor()) for item in record.source_evidence_refs)
    if keys != tuple(sorted(keys)) or len(set(keys)) != len(keys):
        raise ContextApplicationV3SupersessionError(
            "CONTEXT_APPLICATION_V3_SUPERSESSION_INPUT_INVALID", "source_evidence_refs"
        )
    if not isinstance(record.review_event_ref_v4, ReviewEventRefV4):
        raise ContextApplicationV3SupersessionError(
            "CONTEXT_APPLICATION_V3_SUPERSESSION_INPUT_INVALID", "review_event_ref_v4"
        )


class ContextApplicationV3SupersessionAdmissionValidator:
    """Admit one immutable cpsr.v3 record and its own V4 review."""

    def __init__(
        self,
        own_record_admitter: Callable[[ContextApplicationV3SupersessionRecord], object],
    ) -> None:
        self._own_record_admitter = own_record_admitter

    def admit(
        self, record: ContextApplicationV3SupersessionRecord
    ) -> ContextApplicationV3SupersessionAdmissionResult:
        if not isinstance(record, ContextApplicationV3SupersessionRecord):
            raise ContextApplicationV3SupersessionError(
                "CONTEXT_APPLICATION_V3_SUPERSESSION_INPUT_INVALID", "record"
            )
        _validate_record_shape(record)
        expected = ContextApplicationV3SupersessionInputV1(
            record.superseded_record_id.digest_bytes,
            None
            if record.replacement_record_id is None
            else record.replacement_record_id.digest_bytes,
            None if record.replacement_record_id is None else "context_application_v3_record",
            record.reason_code,
            record.source_evidence_refs,
        ).identity()
        if expected != record.supersession_id:
            raise ContextApplicationV3SupersessionError(
                "CONTEXT_APPLICATION_V3_SUPERSESSION_IDENTITY_MISMATCH", "supersession_id"
            )
        expected_record = ContextApplicationV3SupersessionRecordInputV1(
            record.supersession_id.digest_bytes, record.review_event_ref_v4
        ).identity()
        if expected_record != record.record_id:
            raise ContextApplicationV3SupersessionError(
                "CONTEXT_APPLICATION_V3_SUPERSESSION_RECORD_IDENTITY_MISMATCH", "record_id"
            )
        try:
            result = self._own_record_admitter(record)
            binding = getattr(result, "review_binding", result)
            event_id = getattr(binding, "event_id", None)
            exact_event_closure = getattr(binding, "exact_event_closure", None)
            if not isinstance(event_id, str) or not isinstance(exact_event_closure, tuple):
                raise TypeError("own V4 admission result is not a review binding")
        except Exception as exc:
            raise ContextApplicationV3SupersessionError(
                "CONTEXT_APPLICATION_V3_SUPERSESSION_REVIEW_ADMISSION_FAILED",
                "review_event_ref_v4",
                cause_code=getattr(exc, "code", type(exc).__name__),
            ) from exc
        return ContextApplicationV3SupersessionAdmissionResult(
            record.record_id,
            record.supersession_id,
            record.superseded_record_id,
            record.replacement_record_id,
            record.reason_code,
            record.review_event_ref_v4,
            event_id,
            exact_event_closure,
        )


def admit_context_application_v3_supersession_record(
    record: ContextApplicationV3SupersessionRecord,
    resolver: object,
) -> ContextApplicationV3SupersessionAdmissionResult:
    """Admit cpsr.v3 through the existing generic V4 binder."""

    _validate_record_shape(record)
    subject = AcceptanceSubjectPayloadV4(
        AcceptanceSubjectKindV4.CONTEXT_APPLICATION_V3_SUPERSESSION_RECORD,
        record.acceptance_free_subject_payload(),
    )
    try:
        event = resolver.resolve_acceptance_event_leaf_v4(record.review_event_ref_v4)
        expected = resolver.expected_context_application_v3_supersession_source_closure(
            record, event.reviewer_roster_ref
        )
        binding = bind_review_acceptance_v4(subject, record.review_event_ref_v4, resolver, expected)
    except ReviewAcceptanceV4BindingError as exc:
        raise ContextApplicationV3SupersessionError(
            "CONTEXT_APPLICATION_V3_SUPERSESSION_REVIEW_ADMISSION_FAILED",
            "review_event_ref_v4",
            cause_code=exc.code,
        ) from exc
    except Exception as exc:
        raise ContextApplicationV3SupersessionError(
            "CONTEXT_APPLICATION_V3_SUPERSESSION_REVIEW_ADMISSION_FAILED",
            "review_event_ref_v4",
            cause_code=getattr(exc, "code", type(exc).__name__),
        ) from exc
    return ContextApplicationV3SupersessionAdmissionResult(
        record.record_id,
        record.supersession_id,
        record.superseded_record_id,
        record.replacement_record_id,
        record.reason_code,
        record.review_event_ref_v4,
        binding.event_id,
        tuple(binding.exact_event_closure),
    )


@dataclass(frozen=True)
class ContextApplicationV3SupersessionEdge:
    supersession_id: AuthorityIdentityV1
    accepted_record_ids: tuple[AuthorityIdentityV1, ...]
    superseded_record_id: AuthorityIdentityV1
    replacement_record_id: AuthorityIdentityV1 | None
    reason_code: SupersessionReason


@dataclass(frozen=True)
class ContextApplicationV3CurrentnessResult:
    current_record_ids: tuple[AuthorityIdentityV1, ...]
    superseded_record_ids: tuple[AuthorityIdentityV1, ...]
    revoked_record_ids: tuple[AuthorityIdentityV1, ...]
    successor_edges: tuple[ContextApplicationV3SupersessionEdge, ...]


class ContextApplicationV3CurrentnessError(ValueError):
    def __init__(self, code: str, location: str, *, cause_code: str | None = None) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        super().__init__(f"{code} at {location}")


def _id_key(value: AuthorityIdentityV1) -> bytes:
    return encode_canonical(value.to_cbor())


class ContextApplicationV3CurrentnessEvaluator:
    """Derive V3 currentness from admitted immutable records and edges."""

    def __init__(
        self,
        record_admitter: Callable[[ContextApplicationV3Record], object],
        supersession_admitter: ContextApplicationV3SupersessionAdmissionValidator,
    ) -> None:
        self._record_admitter = record_admitter
        self._supersession_admitter = supersession_admitter

    def evaluate(
        self,
        application_records: Sequence[ContextApplicationV3Record],
        supersession_records: Sequence[ContextApplicationV3SupersessionRecord],
    ) -> ContextApplicationV3CurrentnessResult:
        records = tuple(application_records)
        supersessions = tuple(supersession_records)
        if any(not isinstance(item, ContextApplicationV3Record) for item in records):
            raise ContextApplicationV3CurrentnessError(
                "CURRENTNESS_INPUT_INVALID", "application_records"
            )
        if any(
            not isinstance(item, ContextApplicationV3SupersessionRecord) for item in supersessions
        ):
            raise ContextApplicationV3CurrentnessError(
                "CURRENTNESS_INPUT_INVALID", "supersession_records"
            )
        record_ids = [_id_key(item.record_id) for item in records]
        supersession_ids = [_id_key(item.record_id) for item in supersessions]
        if len(set(record_ids)) != len(record_ids) or len(set(supersession_ids)) != len(
            supersession_ids
        ):
            raise ContextApplicationV3CurrentnessError("DUPLICATE_RECORD_ID", "records")
        live_records: dict[bytes, ContextApplicationV3Record] = {}
        for record in sorted(records, key=lambda item: _id_key(item.record_id)):
            if self._record_admitter is not None:
                try:
                    self._record_admitter(record)
                except Exception:
                    continue
            live_records[_id_key(record.record_id)] = record
        admissions = []
        for record in sorted(supersessions, key=lambda item: _id_key(item.record_id)):
            try:
                admissions.append(self._supersession_admitter.admit(record))
            except ContextApplicationV3SupersessionError as exc:
                raise ContextApplicationV3CurrentnessError(
                    "SUPERSESSION_ADMISSION_FAILED", "supersession_record", cause_code=exc.code
                ) from exc
        edges_by_source: dict[bytes, ContextApplicationV3SupersessionEdge] = {}
        grouped: dict[bytes, list[object]] = {}
        for admission in admissions:
            grouped.setdefault(_id_key(admission.supersession_id), []).append(admission)
        for group in grouped.values():
            first = sorted(group, key=lambda item: _id_key(item.record_id))[0]
            edge = ContextApplicationV3SupersessionEdge(
                first.supersession_id,
                tuple(
                    item.record_id
                    for item in sorted(group, key=lambda item: _id_key(item.record_id))
                ),
                first.superseded_record_id,
                first.replacement_record_id,
                first.reason_code,
            )
            source_key = _id_key(edge.superseded_record_id)
            if source_key in edges_by_source:
                raise ContextApplicationV3CurrentnessError(
                    "MULTIPLE_SUCCESSORS", "supersession_graph"
                )
            if source_key not in live_records:
                raise ContextApplicationV3CurrentnessError(
                    "SUPERSEDED_RECORD_UNKNOWN", "superseded_record_id"
                )
            if edge.replacement_record_id is not None:
                replacement_key = _id_key(edge.replacement_record_id)
                if replacement_key not in live_records:
                    raise ContextApplicationV3CurrentnessError(
                        "REPLACEMENT_RECORD_UNKNOWN", "replacement_record_id"
                    )
                if replacement_key == source_key:
                    raise ContextApplicationV3CurrentnessError(
                        "SELF_SUPERSESSION", "replacement_record_id"
                    )
            edges_by_source[source_key] = edge
        for start in tuple(edges_by_source):
            seen: set[bytes] = set()
            current = start
            while (
                current in edges_by_source
                and edges_by_source[current].replacement_record_id is not None
            ):
                if current in seen:
                    raise ContextApplicationV3CurrentnessError(
                        "SUPERSESSION_CYCLE", "supersession_graph"
                    )
                seen.add(current)
                current = _id_key(edges_by_source[current].replacement_record_id)  # type: ignore[arg-type]
        revoked_apps = {
            _id_key(live_records[_id_key(edge.superseded_record_id)].application_id)
            for edge in edges_by_source.values()
            if edge.reason_code is SupersessionReason.AUTHORITY_REVOCATION
        }
        superseded = {
            _id_key(edge.superseded_record_id)
            for edge in edges_by_source.values()
            if edge.replacement_record_id is not None
        }
        groups: dict[bytes, list[ContextApplicationV3Record]] = {}
        for record in records:
            groups.setdefault(_id_key(record.application_id), []).append(record)
        current: list[AuthorityIdentityV1] = []
        for app_key, group in groups.items():
            if app_key in revoked_apps:
                continue
            candidates = tuple(
                record
                for record in group
                if _id_key(record.record_id) not in superseded
                and _id_key(record.record_id) in live_records
            )
            if len(candidates) > 1:
                raise ContextApplicationV3CurrentnessError(
                    "CURRENTNESS_AMBIGUOUS", "application_records.application_id"
                )
            if candidates:
                current.append(candidates[0].record_id)
        return ContextApplicationV3CurrentnessResult(
            tuple(sorted(current, key=_id_key)),
            tuple(
                sorted(
                    (
                        record.record_id
                        for record in records
                        if _id_key(record.record_id) in superseded
                    ),
                    key=_id_key,
                )
            ),
            tuple(
                sorted(
                    (
                        record.record_id
                        for record in records
                        if _id_key(record.application_id) in revoked_apps
                    ),
                    key=_id_key,
                )
            ),
            tuple(sorted(edges_by_source.values(), key=lambda edge: _id_key(edge.supersession_id))),
        )


__all__ = [
    "ContextApplicationV3CurrentnessError",
    "ContextApplicationV3CurrentnessEvaluator",
    "ContextApplicationV3CurrentnessResult",
    "ContextApplicationV3SupersessionAdmissionResult",
    "ContextApplicationV3SupersessionAdmissionValidator",
    "ContextApplicationV3SupersessionEdge",
    "ContextApplicationV3SupersessionError",
    "admit_context_application_v3_supersession_record",
]
