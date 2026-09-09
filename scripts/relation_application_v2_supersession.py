"""RPA V2 supersession admission and ADR-0043-style currentness."""

from __future__ import annotations

import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from mtgml.authority import (
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    RelationApplicationAuthorityV2,
    RelationApplicationV2Record,
    RelationApplicationV2SupersessionInputV2,
    RelationApplicationV2SupersessionRecord,
    RelationApplicationV2SupersessionRecordInputV1,
    RelationAuthoritySourceBindingV2,
    ReviewAuthoritySourceBindingV4,
    SupersessionReason,
)
from mtgml.persistence import encode_canonical
from relation_application_v2_review_admission import admit_relation_application_v2_record
from review_acceptance_v4 import bind_review_acceptance_v4


class RelationApplicationV2SupersessionError(ValueError):
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


class RelationApplicationV2CurrentnessResolver(Protocol):
    def require_current_relation_theorem(
        self, theorem_record_id: object
    ) -> Mapping[str, object]: ...


@dataclass(frozen=True)
class RelationApplicationV2SupersessionAdmissionResult:
    record_id: AuthorityIdentityV1
    supersession_id: AuthorityIdentityV1
    superseded_record_id: AuthorityIdentityV1
    replacement_record_id: AuthorityIdentityV1 | None
    reason_code: SupersessionReason
    exact_event_closure: tuple[ReviewAuthoritySourceBindingV4, ...]


@dataclass(frozen=True)
class RelationApplicationV2SupersessionEdge:
    supersession_id: AuthorityIdentityV1
    accepted_record_ids: tuple[AuthorityIdentityV1, ...]
    superseded_record_id: AuthorityIdentityV1
    replacement_record_id: AuthorityIdentityV1 | None
    reason_code: SupersessionReason


@dataclass(frozen=True)
class RelationApplicationV2CurrentnessResult:
    current_record_ids: tuple[AuthorityIdentityV1, ...]
    superseded_record_ids: tuple[AuthorityIdentityV1, ...]
    revoked_record_ids: tuple[AuthorityIdentityV1, ...]
    successor_edges: tuple[RelationApplicationV2SupersessionEdge, ...]


@dataclass(frozen=True)
class RelationApplicationAuthorityV2AdmissionResult:
    currentness: RelationApplicationV2CurrentnessResult
    source_closure: tuple[RelationAuthoritySourceBindingV2, ...]


def _identity_key(identity: AuthorityIdentityV1) -> bytes:
    return encode_canonical(identity.to_cbor())


def _record_key(record: RelationApplicationV2Record) -> bytes:
    return _identity_key(record.record_id)


def _require_exact_source_set(
    actual: Sequence[ReviewAuthoritySourceBindingV4],
    expected: Sequence[ReviewAuthoritySourceBindingV4],
) -> None:
    actual_bytes = tuple(encode_canonical(item.to_cbor()) for item in actual)
    if actual_bytes != tuple(sorted(actual_bytes)) or len(set(actual_bytes)) != len(actual_bytes):
        raise RelationApplicationV2SupersessionError(
            "RPA_V2_SOURCE_CLOSURE_MISMATCH", "review_event.source_binding_digests"
        )
    expected_bytes = tuple(sorted(encode_canonical(item.to_cbor()) for item in expected))
    if actual_bytes != expected_bytes:
        raise RelationApplicationV2SupersessionError(
            "RPA_V2_SOURCE_CLOSURE_MISMATCH", "review_event.source_binding_digests"
        )


def _require_structural_invariants(record: RelationApplicationV2SupersessionRecord) -> None:
    if record.record_id.kind is not AuthorityIdentityKind.RELATION_SUPERSESSION_RECORD_V2:
        raise RelationApplicationV2SupersessionError(
            "SUPERSESSION_RECORD_IDENTITY_MISMATCH", "record_id"
        )
    if record.supersession_id.kind is not AuthorityIdentityKind.RELATION_SUPERSESSION_V2:
        raise RelationApplicationV2SupersessionError(
            "SUPERSESSION_IDENTITY_MISMATCH", "supersession_id"
        )
    if record.superseded_record_id.kind is not AuthorityIdentityKind.RELATION_APPLICATION_RECORD_V2:
        raise RelationApplicationV2SupersessionError(
            "SUPERSESSION_INPUT_INVALID", "superseded_record_id"
        )
    if (
        record.replacement_record_id is not None
        and record.replacement_record_id.kind
        is not AuthorityIdentityKind.RELATION_APPLICATION_RECORD_V2
    ):
        raise RelationApplicationV2SupersessionError(
            "SUPERSESSION_REPLACEMENT_INVALID", "replacement_record_id"
        )
    if record.replacement_record_id is None:
        if record.reason_code is not SupersessionReason.AUTHORITY_REVOCATION:
            raise RelationApplicationV2SupersessionError(
                "SUPERSESSION_REASON_INVALID", "replacement_record_id"
            )
    elif record.reason_code is SupersessionReason.AUTHORITY_REVOCATION:
        raise RelationApplicationV2SupersessionError("SUPERSESSION_REASON_INVALID", "reason_code")


def admit_relation_application_v2_supersession_record(
    record: RelationApplicationV2SupersessionRecord,
    resolver: object,
    *,
    endpoints: Mapping[str, RelationApplicationV2Record],
) -> RelationApplicationV2SupersessionAdmissionResult:
    if not isinstance(record, RelationApplicationV2SupersessionRecord):
        raise RelationApplicationV2SupersessionError("SUPERSESSION_INPUT_INVALID", "record")
    _require_structural_invariants(record)
    superseded = endpoints.get(record.superseded_record_id.as_text())
    if superseded is None:
        raise RelationApplicationV2SupersessionError(
            "SUPERSEDED_RECORD_UNKNOWN", "superseded_record_id", record_id=record.record_id
        )
    replacement = (
        None
        if record.replacement_record_id is None
        else endpoints.get(record.replacement_record_id.as_text())
    )
    if record.replacement_record_id is not None and replacement is None:
        raise RelationApplicationV2SupersessionError(
            "REPLACEMENT_RECORD_UNKNOWN", "replacement_record_id", record_id=record.record_id
        )
    if record.replacement_record_id == record.superseded_record_id:
        raise RelationApplicationV2SupersessionError(
            "SELF_SUPERSESSION", "replacement_record_id", record_id=record.record_id
        )
    expected_id = RelationApplicationV2SupersessionInputV2(
        superseded_record_id_bytes=record.superseded_record_id.digest_bytes,
        replacement_record_id_bytes=(
            None
            if record.replacement_record_id is None
            else record.replacement_record_id.digest_bytes
        ),
        replacement_record_kind=(None if replacement is None else "relation_application_v2_record"),
        reason_code=record.reason_code,
        source_evidence_refs=record.source_evidence_refs,
    ).identity()
    if expected_id != record.supersession_id:
        raise RelationApplicationV2SupersessionError(
            "SUPERSESSION_IDENTITY_MISMATCH", "supersession_id", record_id=record.record_id
        )
    expected_record_id = RelationApplicationV2SupersessionRecordInputV1(
        record.supersession_id.digest_bytes,
        record.review_event_ref_v4,
    ).identity()
    if expected_record_id != record.record_id:
        raise RelationApplicationV2SupersessionError(
            "SUPERSESSION_RECORD_IDENTITY_MISMATCH", "record_id", record_id=record.record_id
        )
    try:
        event = resolver.resolve_acceptance_event_leaf_v4(record.review_event_ref_v4)
        subject = AcceptanceSubjectPayloadV4(
            AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_SUPERSESSION_RECORD,
            record.acceptance_free_subject_payload(),
        )
        expected_closure = resolver.expected_relation_application_v2_supersession_source_closure(
            record,
            event.reviewer_roster_ref,
            superseded,
            replacement,
        )
        bound = bind_review_acceptance_v4(
            subject,
            record.review_event_ref_v4,
            resolver,
            expected_closure,
        )
        _require_exact_source_set(bound.event.source_binding_digests, expected_closure)
    except RelationApplicationV2SupersessionError:
        raise
    except Exception as exc:
        raise RelationApplicationV2SupersessionError(
            "SUPERSESSION_REVIEW_ADMISSION_FAILED",
            "review_event",
            cause_code=getattr(exc, "code", type(exc).__name__),
            record_id=record.record_id,
            supersession_id=record.supersession_id,
        ) from exc
    return RelationApplicationV2SupersessionAdmissionResult(
        record.record_id,
        record.supersession_id,
        record.superseded_record_id,
        record.replacement_record_id,
        record.reason_code,
        tuple(expected_closure),
    )


def admit_relation_application_authority_v2(
    authority: RelationApplicationAuthorityV2,
    resolver: object,
    *,
    currentness: RelationApplicationV2CurrentnessResolver,
) -> RelationApplicationAuthorityV2AdmissionResult:
    if not isinstance(authority, RelationApplicationAuthorityV2):
        raise RelationApplicationV2SupersessionError("CURRENTNESS_INPUT_INVALID", "authority")
    try:
        closure = resolver.validate_relation_application_authority_v2_source_closure(authority)
    except RelationApplicationV2SupersessionError:
        raise
    except Exception as exc:
        raise RelationApplicationV2SupersessionError(
            "CONTAINER_SOURCE_CLOSURE_MISMATCH",
            "source_bindings",
            cause_code=getattr(exc, "code", type(exc).__name__),
        ) from exc
    actual_source_bytes = tuple(
        encode_canonical(binding.to_cbor()) for binding in authority.source_bindings
    )
    expected_source_bytes = tuple(encode_canonical(binding.to_cbor()) for binding in closure)
    if actual_source_bytes != expected_source_bytes:
        raise RelationApplicationV2SupersessionError(
            "CONTAINER_SOURCE_CLOSURE_MISMATCH", "source_bindings"
        )
    evaluator = RelationApplicationV2CurrentnessEvaluator(resolver, currentness=currentness)
    result = evaluator.evaluate(
        authority.relation_application_v2_records,
        authority.relation_application_v2_supersession_records,
    )
    return RelationApplicationAuthorityV2AdmissionResult(result, tuple(closure))


class RelationApplicationV2CurrentnessEvaluator:
    """Admit RPA V2 records and derive currentness without latest-wins."""

    def __init__(
        self, resolver: object, *, currentness: RelationApplicationV2CurrentnessResolver
    ) -> None:
        self._resolver = resolver
        self._currentness = currentness

    @staticmethod
    def _validate_duplicates(records: Sequence[object], location: str) -> None:
        ids = [getattr(record, "record_id", None) for record in records]
        if any(not isinstance(identity, AuthorityIdentityV1) for identity in ids):
            raise RelationApplicationV2SupersessionError("CURRENTNESS_INPUT_INVALID", location)
        keys = [_identity_key(identity) for identity in ids]
        if len(set(keys)) != len(keys):
            raise RelationApplicationV2SupersessionError("DUPLICATE_RECORD_ID", location)

    @staticmethod
    def _group_edges(
        admissions: Sequence[RelationApplicationV2SupersessionAdmissionResult],
    ) -> tuple[RelationApplicationV2SupersessionEdge, ...]:
        groups: dict[bytes, list[RelationApplicationV2SupersessionAdmissionResult]] = {}
        for admission in admissions:
            groups.setdefault(_identity_key(admission.supersession_id), []).append(admission)
        edges: list[RelationApplicationV2SupersessionEdge] = []
        for group in groups.values():
            ordered = tuple(sorted(group, key=lambda item: _identity_key(item.record_id)))
            first = ordered[0]
            for other in ordered[1:]:
                if (
                    other.superseded_record_id != first.superseded_record_id
                    or other.replacement_record_id != first.replacement_record_id
                    or other.reason_code is not first.reason_code
                ):
                    raise RelationApplicationV2SupersessionError(
                        "SUPERSESSION_CONFLICT", "supersession_id"
                    )
            edges.append(
                RelationApplicationV2SupersessionEdge(
                    first.supersession_id,
                    tuple(item.record_id for item in ordered),
                    first.superseded_record_id,
                    first.replacement_record_id,
                    first.reason_code,
                )
            )
        return tuple(sorted(edges, key=lambda edge: _identity_key(edge.supersession_id)))

    @staticmethod
    def _validate_edges(
        edges: Sequence[RelationApplicationV2SupersessionEdge],
        records: Mapping[str, RelationApplicationV2Record],
    ) -> tuple[set[str], dict[str, RelationApplicationV2SupersessionEdge]]:
        successors: dict[str, RelationApplicationV2SupersessionEdge] = {}
        superseded_sources: set[str] = set()
        for edge in edges:
            source = edge.superseded_record_id.as_text()
            if source not in records:
                raise RelationApplicationV2SupersessionError(
                    "SUPERSEDED_RECORD_UNKNOWN", "superseded_record_id"
                )
            if edge.replacement_record_id is not None:
                target = edge.replacement_record_id.as_text()
                if target not in records:
                    raise RelationApplicationV2SupersessionError(
                        "REPLACEMENT_RECORD_UNKNOWN", "replacement_record_id"
                    )
                if target == source:
                    raise RelationApplicationV2SupersessionError(
                        "SELF_SUPERSESSION", "replacement_record_id"
                    )
                superseded_sources.add(source)
            if source in successors:
                raise RelationApplicationV2SupersessionError(
                    "MULTIPLE_SUCCESSORS", "superseded_record_id"
                )
            successors[source] = edge
        return superseded_sources, successors

    @staticmethod
    def _validate_cycles(successors: Mapping[str, RelationApplicationV2SupersessionEdge]) -> None:
        for start in successors:
            seen: set[str] = set()
            current = start
            while current in successors and successors[current].replacement_record_id is not None:
                if current in seen:
                    raise RelationApplicationV2SupersessionError(
                        "SUPERSESSION_CYCLE", "supersession_graph"
                    )
                seen.add(current)
                current = successors[current].replacement_record_id.as_text()  # type: ignore[union-attr]

    def evaluate(
        self,
        records: Sequence[RelationApplicationV2Record],
        supersession_records: Sequence[RelationApplicationV2SupersessionRecord],
    ) -> RelationApplicationV2CurrentnessResult:
        self._validate_duplicates(records, "application_records.record_id")
        self._validate_duplicates(supersession_records, "supersession_records.record_id")
        record_map = {record.record_id.as_text(): record for record in records}
        admitted_records: list[RelationApplicationV2Record] = []
        for record in sorted(records, key=_record_key):
            try:
                admit_relation_application_v2_record(
                    record,
                    self._resolver,
                    currentness=self._currentness,
                )
                admitted_records.append(record)
            except Exception as exc:
                code = getattr(exc, "code", "APPLICATION_REVIEW_ADMISSION_FAILED")
                if code in {
                    "SUPERSEDED_AUTHORITY_USED",
                    "RELATION_APPLICATION_V2_CURRENTNESS_FAILED",
                    "RELATION_APPLICATION_V2_CURRENTNESS_AMBIGUOUS",
                }:
                    try:
                        admit_relation_application_v2_record(
                            record,
                            self._resolver,
                            currentness=self._currentness,
                            require_current_theorem=False,
                        )
                    except Exception as historical_exc:
                        raise RelationApplicationV2SupersessionError(
                            "APPLICATION_REVIEW_ADMISSION_FAILED",
                            "application_record",
                            cause_code=getattr(
                                historical_exc,
                                "code",
                                type(historical_exc).__name__,
                            ),
                        ) from historical_exc
                    continue
                raise RelationApplicationV2SupersessionError(
                    "APPLICATION_REVIEW_ADMISSION_FAILED", "application_record", cause_code=code
                ) from exc
        admissions: list[RelationApplicationV2SupersessionAdmissionResult] = []
        for record in sorted(supersession_records, key=_record_key):
            admissions.append(
                admit_relation_application_v2_supersession_record(
                    record,
                    self._resolver,
                    endpoints=record_map,
                )
            )
        edges = self._group_edges(admissions)
        superseded_sources, successors = self._validate_edges(edges, record_map)
        self._validate_cycles(successors)
        revoked_apps = {
            record_map[edge.superseded_record_id.as_text()].application_id.as_text()
            for edge in edges
            if edge.reason_code is SupersessionReason.AUTHORITY_REVOCATION
        }
        groups: dict[str, list[RelationApplicationV2Record]] = {}
        for record in admitted_records:
            groups.setdefault(record.application_id.as_text(), []).append(record)
        current: list[AuthorityIdentityV1] = []
        revoked: list[AuthorityIdentityV1] = []
        superseded: list[AuthorityIdentityV1] = []
        for application_id, group in groups.items():
            if application_id in revoked_apps:
                revoked.extend(record.record_id for record in group)
                continue
            candidates = [
                record for record in group if record.record_id.as_text() not in superseded_sources
            ]
            superseded.extend(
                record.record_id
                for record in group
                if record.record_id.as_text() in superseded_sources
            )
            if len(candidates) > 1:
                raise RelationApplicationV2SupersessionError(
                    "CURRENTNESS_AMBIGUOUS", "application_records.application_id"
                )
            if candidates:
                current.append(candidates[0].record_id)
        return RelationApplicationV2CurrentnessResult(
            tuple(sorted(current, key=_identity_key)),
            tuple(sorted(superseded, key=_identity_key)),
            tuple(sorted(revoked, key=_identity_key)),
            edges,
        )


__all__ = [
    "RelationApplicationAuthorityV2AdmissionResult",
    "RelationApplicationV2CurrentnessEvaluator",
    "RelationApplicationV2CurrentnessResult",
    "RelationApplicationV2SupersessionAdmissionResult",
    "RelationApplicationV2SupersessionEdge",
    "RelationApplicationV2SupersessionError",
    "admit_relation_application_authority_v2",
    "admit_relation_application_v2_supersession_record",
]
