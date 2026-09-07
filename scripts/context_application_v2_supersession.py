"""Read-only ContextApplicationV2 supersession admission and currentness."""

from __future__ import annotations

import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Final, cast

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import AuthoritySourceResolver
from context_application_v2_review_admission import (
    ContextApplicationV2ReviewAdmissionError,
    ContextApplicationV2ReviewAdmissionValidator,
)
from context_application_v2_review_binding import (
    ContextApplicationV2V3ReviewBindingError,
    admit_v3_review_binding,
)
from mtgml.authority import (
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionInputV2,
    ContextApplicationV2SupersessionRecord,
    ContextApplicationV2SupersessionRecordInputV1,
    ContextAuthoritySourceBindingV2,
    DigestReferenceV1,
    EvidenceRefV1,
    ReviewerRosterRefV1,
    ReviewEventRefV3,
    ReviewMode,
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
                None if record.replacement_record_id is None else "context_application_v2_record"
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


@dataclass(frozen=True)
class ContextApplicationV2SupersessionEdge:
    supersession_id: AuthorityIdentityV1
    accepted_record_ids: tuple[AuthorityIdentityV1, ...]
    superseded_record_id: AuthorityIdentityV1
    replacement_record_id: AuthorityIdentityV1 | None
    reason_code: SupersessionReason


@dataclass(frozen=True)
class ContextApplicationV2CurrentnessResult:
    current_record_ids: tuple[AuthorityIdentityV1, ...]
    superseded_record_ids: tuple[AuthorityIdentityV1, ...]
    revoked_record_ids: tuple[AuthorityIdentityV1, ...]
    successor_edges: tuple[ContextApplicationV2SupersessionEdge, ...]


class ContextApplicationV2CurrentnessError(ValueError):
    """Stable, structured failure from graph admission or currentness."""

    def __init__(
        self,
        code: str,
        location: str,
        *,
        cause_code: str | None = None,
        cause_location: str | None = None,
        record_id: AuthorityIdentityV1 | None = None,
        supersession_id: AuthorityIdentityV1 | None = None,
        application_id: AuthorityIdentityV1 | None = None,
        subject_record_ids: tuple[AuthorityIdentityV1, ...] = (),
        subject_supersession_ids: tuple[AuthorityIdentityV1, ...] = (),
        cycle_path: tuple[AuthorityIdentityV1, ...] = (),
    ) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        self.cause_location = cause_location
        self.record_id = record_id
        self.supersession_id = supersession_id
        self.application_id = application_id
        self.subject_record_ids = subject_record_ids
        self.subject_supersession_ids = subject_supersession_ids
        self.cycle_path = cycle_path
        super().__init__(f"{code} at {location}")


def _identity_key(identity: AuthorityIdentityV1) -> bytes:
    return encode_canonical(identity.to_cbor())


def _record_key(record: ContextApplicationV2Record) -> bytes:
    return _identity_key(record.record_id)


def _require_sequence(value: object, location: str) -> tuple[object, ...]:
    if isinstance(value, str | bytes | bytearray | Mapping) or not isinstance(value, Sequence):
        raise ContextApplicationV2CurrentnessError(
            "CURRENTNESS_INPUT_INVALID",
            location,
        )
    return tuple(value)


def _sorted_ids(values: Sequence[AuthorityIdentityV1]) -> tuple[AuthorityIdentityV1, ...]:
    return tuple(sorted(values, key=_identity_key))


def _edge_key(
    edge: ContextApplicationV2SupersessionEdge,
) -> tuple[bytes, bytes, bytes, bytes]:
    return (
        _identity_key(edge.superseded_record_id),
        b"" if edge.replacement_record_id is None else _identity_key(edge.replacement_record_id),
        edge.reason_code.value.encode("utf-8"),
        _identity_key(edge.supersession_id),
    )


def _normalize_cycle(
    cycle: tuple[AuthorityIdentityV1, ...],
) -> tuple[AuthorityIdentityV1, ...]:
    rotations = tuple(cycle[index:] + cycle[:index] for index in range(len(cycle)))
    return min(rotations, key=lambda path: tuple(_identity_key(item) for item in path))


class ContextApplicationV2CurrentnessEvaluator:
    """Admit immutable records, validate their graph, and derive currentness."""

    def __init__(
        self,
        source_resolver: AuthoritySourceResolver,
        *,
        base_authority_binding: ContextAuthoritySourceBindingV2,
    ) -> None:
        self._source_resolver = source_resolver
        self._base_binding = base_authority_binding

    def _admit_application_records(
        self,
        records: tuple[ContextApplicationV2Record, ...],
    ) -> dict[bytes, ContextApplicationV2Record]:
        validator = ContextApplicationV2ReviewAdmissionValidator(
            self._source_resolver,
            base_authority_binding=self._base_binding,
        )
        admitted: dict[bytes, ContextApplicationV2Record] = {}
        for record in sorted(records, key=_record_key):
            key = _record_key(record)
            if key in admitted:
                raise ContextApplicationV2CurrentnessError(
                    "DUPLICATE_RECORD_ID",
                    "application_records.record_id",
                    record_id=record.record_id,
                    subject_record_ids=(record.record_id,),
                )
            try:
                validator.admit(record)
            except ContextApplicationV2ReviewAdmissionError as exc:
                raise ContextApplicationV2CurrentnessError(
                    "APPLICATION_REVIEW_ADMISSION_FAILED",
                    "application_record",
                    cause_code=exc.code,
                    cause_location=exc.location,
                    record_id=record.record_id,
                    application_id=record.application_id,
                    subject_record_ids=(record.record_id,),
                ) from exc
            admitted[key] = record
        return admitted

    def _admit_supersession_records(
        self,
        records: tuple[ContextApplicationV2SupersessionRecord, ...],
    ) -> tuple[ContextApplicationV2SupersessionAdmissionResult, ...]:
        validator = ContextApplicationV2SupersessionAdmissionValidator(
            self._source_resolver,
            base_authority_binding=self._base_binding,
        )
        ordered = tuple(sorted(records, key=lambda record: _identity_key(record.record_id)))
        seen_record_ids: set[bytes] = set()
        results: list[ContextApplicationV2SupersessionAdmissionResult] = []
        for record in ordered:
            record_key = _identity_key(record.record_id)
            if record_key in seen_record_ids:
                raise ContextApplicationV2CurrentnessError(
                    "DUPLICATE_RECORD_ID",
                    "supersession_records.record_id",
                    record_id=record.record_id,
                    supersession_id=record.supersession_id,
                    subject_record_ids=(record.record_id,),
                )
            seen_record_ids.add(record_key)
            try:
                results.append(validator.admit(record))
            except ContextApplicationV2SupersessionError as exc:
                raise ContextApplicationV2CurrentnessError(
                    "SUPERSESSION_ADMISSION_FAILED",
                    "supersession_record",
                    cause_code=exc.code,
                    cause_location=exc.location,
                    record_id=record.record_id,
                    supersession_id=record.supersession_id,
                    subject_record_ids=(
                        record.record_id,
                        record.superseded_record_id,
                    ),
                ) from exc
        return tuple(results)

    @staticmethod
    def _group_edges(
        admissions: tuple[ContextApplicationV2SupersessionAdmissionResult, ...],
    ) -> tuple[ContextApplicationV2SupersessionEdge, ...]:
        groups: dict[bytes, list[ContextApplicationV2SupersessionAdmissionResult]] = {}
        for admission in admissions:
            groups.setdefault(_identity_key(admission.supersession_id), []).append(admission)
        edges: list[ContextApplicationV2SupersessionEdge] = []
        for group in groups.values():
            ordered = tuple(sorted(group, key=lambda item: _identity_key(item.record_id)))
            first = ordered[0]
            edges.append(
                ContextApplicationV2SupersessionEdge(
                    supersession_id=first.supersession_id,
                    accepted_record_ids=tuple(item.record_id for item in ordered),
                    superseded_record_id=first.superseded_record_id,
                    replacement_record_id=first.replacement_record_id,
                    reason_code=first.reason_code,
                )
            )
        return tuple(sorted(edges, key=_edge_key))

    @staticmethod
    def _validate_edges(
        edges: tuple[ContextApplicationV2SupersessionEdge, ...],
        application_by_id: dict[bytes, ContextApplicationV2Record],
    ) -> tuple[set[bytes], dict[bytes, ContextApplicationV2SupersessionEdge]]:
        successor_edges: dict[bytes, ContextApplicationV2SupersessionEdge] = {}
        replacement_sources: set[bytes] = set()
        for edge in edges:
            source_key = _identity_key(edge.superseded_record_id)
            if source_key not in application_by_id:
                raise ContextApplicationV2CurrentnessError(
                    "SUPERSEDED_RECORD_UNKNOWN",
                    "supersession_edge.superseded_record_id",
                    record_id=edge.superseded_record_id,
                    supersession_id=edge.supersession_id,
                    subject_record_ids=(edge.superseded_record_id,),
                    subject_supersession_ids=(edge.supersession_id,),
                )
            if edge.replacement_record_id is not None:
                replacement_key = _identity_key(edge.replacement_record_id)
                if replacement_key not in application_by_id:
                    raise ContextApplicationV2CurrentnessError(
                        "REPLACEMENT_RECORD_UNKNOWN",
                        "supersession_edge.replacement_record_id",
                        record_id=edge.replacement_record_id,
                        supersession_id=edge.supersession_id,
                        subject_record_ids=(edge.replacement_record_id,),
                        subject_supersession_ids=(edge.supersession_id,),
                    )
                if replacement_key == source_key:
                    raise ContextApplicationV2CurrentnessError(
                        "SELF_SUPERSESSION",
                        "supersession_edge.replacement_record_id",
                        record_id=edge.superseded_record_id,
                        supersession_id=edge.supersession_id,
                        subject_record_ids=(edge.superseded_record_id,),
                        subject_supersession_ids=(edge.supersession_id,),
                    )
                replacement_sources.add(source_key)

            existing = successor_edges.get(source_key)
            if existing is not None:
                subject_supersession_ids = tuple(
                    sorted(
                        (existing.supersession_id, edge.supersession_id),
                        key=_identity_key,
                    )
                )
                raise ContextApplicationV2CurrentnessError(
                    "MULTIPLE_SUCCESSORS",
                    "supersession_edge.superseded_record_id",
                    record_id=edge.superseded_record_id,
                    supersession_id=subject_supersession_ids[0],
                    subject_record_ids=(edge.superseded_record_id,),
                    subject_supersession_ids=subject_supersession_ids,
                )
            successor_edges[source_key] = edge
        return replacement_sources, successor_edges

    @staticmethod
    def _validate_cycles(
        successor_edges: dict[bytes, ContextApplicationV2SupersessionEdge],
    ) -> None:
        successor_map = {
            source_key: edge.replacement_record_id
            for source_key, edge in successor_edges.items()
            if edge.replacement_record_id is not None
        }
        nodes = tuple(
            sorted(
                (
                    edge.superseded_record_id
                    for edge in successor_edges.values()
                    if edge.replacement_record_id is not None
                ),
                key=_identity_key,
            )
        )
        cycles: list[tuple[AuthorityIdentityV1, ...]] = []
        for start in nodes:
            path: list[AuthorityIdentityV1] = []
            positions: dict[bytes, int] = {}
            current = start
            while _identity_key(current) in successor_map:
                current_key = _identity_key(current)
                if current_key in positions:
                    cycles.append(_normalize_cycle(tuple(path[positions[current_key] :])))
                    break
                positions[current_key] = len(path)
                path.append(current)
                current = successor_map[current_key]  # type: ignore[assignment]
        if not cycles:
            return
        cycle = min(cycles, key=lambda path: tuple(_identity_key(item) for item in path))
        cycle_edges = tuple(successor_edges[_identity_key(record_id)] for record_id in cycle)
        subject_supersession_ids = tuple(
            sorted((edge.supersession_id for edge in cycle_edges), key=_identity_key)
        )
        raise ContextApplicationV2CurrentnessError(
            "SUPERSESSION_CYCLE",
            "supersession_graph",
            record_id=cycle[0],
            supersession_id=subject_supersession_ids[0],
            subject_record_ids=cycle,
            subject_supersession_ids=subject_supersession_ids,
            cycle_path=cycle,
        )

    def evaluate(
        self,
        application_records: Sequence[ContextApplicationV2Record],
        supersession_records: Sequence[ContextApplicationV2SupersessionRecord],
    ) -> ContextApplicationV2CurrentnessResult:
        raw_applications = _require_sequence(application_records, "application_records")
        raw_supersessions = _require_sequence(
            supersession_records,
            "supersession_records",
        )
        if any(not isinstance(record, ContextApplicationV2Record) for record in raw_applications):
            raise ContextApplicationV2CurrentnessError(
                "CURRENTNESS_INPUT_INVALID",
                "application_records",
            )
        if any(
            not isinstance(record, ContextApplicationV2SupersessionRecord)
            for record in raw_supersessions
        ):
            raise ContextApplicationV2CurrentnessError(
                "CURRENTNESS_INPUT_INVALID",
                "supersession_records",
            )

        applications = tuple(cast(ContextApplicationV2Record, item) for item in raw_applications)
        supersessions = tuple(
            cast(ContextApplicationV2SupersessionRecord, item) for item in raw_supersessions
        )
        application_by_id = self._admit_application_records(applications)
        admissions = self._admit_supersession_records(supersessions)
        edges = self._group_edges(admissions)
        replacement_sources, successor_edges = self._validate_edges(edges, application_by_id)
        self._validate_cycles(successor_edges)

        revoked_application_ids = {
            application_by_id[_identity_key(edge.superseded_record_id)].application_id
            for edge in edges
            if edge.reason_code is SupersessionReason.AUTHORITY_REVOCATION
        }
        revoked_application_keys = {
            _identity_key(application_id) for application_id in revoked_application_ids
        }
        revoked_record_ids = {
            _identity_key(record.record_id)
            for record in applications
            if _identity_key(record.application_id) in revoked_application_keys
        }
        groups: dict[bytes, list[ContextApplicationV2Record]] = {}
        for record in sorted(applications, key=_record_key):
            groups.setdefault(_identity_key(record.application_id), []).append(record)

        current_record_ids: list[AuthorityIdentityV1] = []
        for application_key in sorted(groups):
            group = groups[application_key]
            if application_key in revoked_application_keys:
                candidates: tuple[ContextApplicationV2Record, ...] = ()
            else:
                candidates = tuple(
                    record
                    for record in group
                    if _identity_key(record.record_id) not in replacement_sources
                )
            if len(candidates) > 1:
                candidate_ids = _sorted_ids(tuple(record.record_id for record in candidates))
                raise ContextApplicationV2CurrentnessError(
                    "CURRENTNESS_AMBIGUOUS",
                    "application_records.application_id",
                    application_id=group[0].application_id,
                    subject_record_ids=candidate_ids,
                )
            if candidates:
                current_record_ids.append(candidates[0].record_id)

        return ContextApplicationV2CurrentnessResult(
            current_record_ids=_sorted_ids(tuple(current_record_ids)),
            superseded_record_ids=_sorted_ids(
                tuple(
                    record.record_id
                    for record in applications
                    if _identity_key(record.record_id) in replacement_sources
                )
            ),
            revoked_record_ids=_sorted_ids(
                tuple(
                    record.record_id
                    for record in applications
                    if _identity_key(record.record_id) in revoked_record_ids
                )
            ),
            successor_edges=tuple(sorted(edges, key=_edge_key)),
        )


__all__ = [
    "ContextApplicationV2CurrentnessError",
    "ContextApplicationV2CurrentnessEvaluator",
    "ContextApplicationV2CurrentnessResult",
    "ContextApplicationV2SupersessionAdmissionResult",
    "ContextApplicationV2SupersessionAdmissionValidator",
    "ContextApplicationV2SupersessionEdge",
    "ContextApplicationV2SupersessionError",
]
