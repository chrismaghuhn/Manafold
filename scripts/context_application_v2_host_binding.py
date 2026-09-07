"""Deterministic Slice-6 composition boundary for ContextApplicationV2.

Task 4 owns verified member applicability and the cpa-level member closure.
HostBinding claim currentness and historical policy remain owned by the later
admission-composition task; this module does not invent a second lifecycle
authority.
"""

from __future__ import annotations

import sys
from collections.abc import Mapping
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Final, cast

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import AuthoritySourceResolver, ResolutionError
from authority_v2_validator import HostBindingClaimRecordStatus
from context_application_v2_resolver import (
    ContextApplicationV2ResolutionError,
    ContextApplicationV2Resolver,
)
from context_application_v2_supersession import (
    ContextApplicationV2CurrentnessError,
    ContextApplicationV2CurrentnessEvaluator,
    ContextApplicationV2CurrentnessResult,
)
from mtgml.authority import (
    ApplicationHostBindingV2,
    AuthorityIdentityV1,
    ContextApplicationAuthorityV2,
    ContextApplicationMemberV2,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionRecord,
    ContextAuthoritySourceBindingV2,
)
from mtgml.host_binding import ApplicationMemberKeyV1, CrossDeckHostBindingClaimV1
from mtgml.persistence import encode_canonical

HOST_INTEGRATION_INPUT_INVALID: Final = "HOST_INTEGRATION_INPUT_INVALID"
APPLICATION_CURRENTNESS_FAILED: Final = "APPLICATION_CURRENTNESS_FAILED"
APPLICATION_HOST_BINDING_DUPLICATE: Final = "APPLICATION_HOST_BINDING_DUPLICATE"
APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION: Final = "APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION"
APPLICATION_HOST_BINDING_INVALID: Final = "APPLICATION_HOST_BINDING_INVALID"
HOST_MEMBER_SET_MISMATCH: Final = "HOST_MEMBER_SET_MISMATCH"
HOST_RELATIONSHIP_MISMATCH: Final = "HOST_RELATIONSHIP_MISMATCH"
HOST_CLAIM_UNKNOWN: Final = "HOST_CLAIM_UNKNOWN"
HOST_MEMBER_APPLICABILITY_INVALID: Final = "HOST_MEMBER_APPLICABILITY_INVALID"


class ApplicationHostBindingStatus(StrEnum):
    QUALIFIED_CURRENT = "qualified_current"
    HISTORICAL_ONLY = "historical_only"


@dataclass(frozen=True)
class ApplicationHostBindingResult:
    application_id: AuthorityIdentityV1
    status: ApplicationHostBindingStatus
    host_binding_claim_ids: tuple[str, ...]


@dataclass(frozen=True)
class ContextApplicationV2HostBindingEvaluationResult:
    currentness: ContextApplicationV2CurrentnessResult
    qualified_current_application_record_ids: tuple[AuthorityIdentityV1, ...]
    application_host_binding_results: tuple[ApplicationHostBindingResult, ...]
    current_host_claim_ids: tuple[str, ...]


@dataclass(frozen=True)
class ContextApplicationV2HostBindingError(ValueError):
    """Stable, frozen Slice-6 structural/composition failure."""

    code: str
    location: str
    cause_code: str | None = None
    application_id: AuthorityIdentityV1 | None = None
    record_id: AuthorityIdentityV1 | None = None
    claim_id: str | None = None
    member_key: ApplicationMemberKeyV1 | None = None
    subject_ids: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        ValueError.__init__(self, f"{self.code} at {self.location}")


@dataclass(frozen=True)
class _ResolvedApplicationMember:
    member_key: ApplicationMemberKeyV1
    expected_host_relationship: str
    required: bool


@dataclass(frozen=True)
class _ApplicationMemberClosure:
    application_id: AuthorityIdentityV1
    record_id: AuthorityIdentityV1
    current: bool
    members: tuple[_ResolvedApplicationMember, ...]
    required_member_keys: tuple[ApplicationMemberKeyV1, ...]


def _identity_key(identity: AuthorityIdentityV1) -> bytes:
    return encode_canonical(identity.to_cbor())


def _member_key_bytes(member_key: ApplicationMemberKeyV1) -> bytes:
    return encode_canonical(member_key.to_cbor())


def _error(
    code: str,
    location: str,
    *,
    cause_code: str | None = None,
    application_id: AuthorityIdentityV1 | None = None,
    record_id: AuthorityIdentityV1 | None = None,
    claim_id: str | None = None,
    member_key: ApplicationMemberKeyV1 | None = None,
    subject_ids: tuple[str, ...] = (),
) -> ContextApplicationV2HostBindingError:
    return ContextApplicationV2HostBindingError(
        code=code,
        location=location,
        cause_code=cause_code,
        application_id=application_id,
        record_id=record_id,
        claim_id=claim_id,
        member_key=member_key,
        subject_ids=subject_ids,
    )


def _require_tuple(value: object, location: str) -> tuple[object, ...]:
    if not isinstance(value, tuple):
        raise _error(HOST_INTEGRATION_INPUT_INVALID, location)
    return value


def _require_canonical_items(
    values: tuple[object, ...],
    location: str,
) -> None:
    try:
        encoded_values: list[bytes] = []
        for value in values:
            to_cbor = getattr(value, "to_cbor", None)
            if not callable(to_cbor):
                raise _error(HOST_INTEGRATION_INPUT_INVALID, location)
            encoded_values.append(encode_canonical(to_cbor()))
        encoded = tuple(encoded_values)
    except (AttributeError, TypeError, ValueError) as exc:
        raise _error(HOST_INTEGRATION_INPUT_INVALID, location) from exc
    if encoded != tuple(sorted(encoded)) or len(set(encoded)) != len(encoded):
        raise _error(HOST_INTEGRATION_INPUT_INVALID, location)


def _require_identity_collection(
    values: tuple[object, ...],
    location: str,
) -> tuple[AuthorityIdentityV1, ...]:
    identities: list[AuthorityIdentityV1] = []
    for value in values:
        if not isinstance(value, AuthorityIdentityV1):
            raise _error(HOST_INTEGRATION_INPUT_INVALID, location)
        identities.append(value)
    return tuple(identities)


def _validate_link_member_union(
    link: ApplicationHostBindingV2,
    required_members: tuple[_ResolvedApplicationMember, ...],
    claims_by_id: Mapping[str, CrossDeckHostBindingClaimV1],
) -> None:
    """Validate G2's exact claim/member union without choosing currentness."""

    expected_by_key = {_member_key_bytes(member.member_key): member for member in required_members}
    if len(expected_by_key) != len(required_members):
        raise _error(
            HOST_MEMBER_SET_MISMATCH,
            "application_host_bindings_v2.host_binding_claim_ids",
            application_id=link.application_semantic_id,
        )

    actual_by_key: dict[bytes, tuple[str, CrossDeckHostBindingClaimV1]] = {}
    for claim_id in link.host_binding_claim_ids:
        claim = claims_by_id.get(claim_id)
        if claim is None:
            raise _error(
                HOST_CLAIM_UNKNOWN,
                "application_host_bindings_v2.host_binding_claim_ids",
                application_id=link.application_semantic_id,
                claim_id=claim_id,
                subject_ids=(claim_id,),
            )
        member_key = _member_key_bytes(claim.member_key)
        if member_key in actual_by_key:
            raise _error(
                HOST_MEMBER_SET_MISMATCH,
                "application_host_bindings_v2.host_binding_claim_ids",
                application_id=link.application_semantic_id,
                claim_id=claim_id,
                member_key=claim.member_key,
                subject_ids=tuple(link.host_binding_claim_ids),
            )
        expected = expected_by_key.get(member_key)
        if expected is None:
            raise _error(
                HOST_MEMBER_SET_MISMATCH,
                "application_host_bindings_v2.host_binding_claim_ids",
                application_id=link.application_semantic_id,
                claim_id=claim_id,
                member_key=claim.member_key,
                subject_ids=tuple(link.host_binding_claim_ids),
            )
        if claim.observed_host_relationship != expected.expected_host_relationship:
            raise _error(
                HOST_RELATIONSHIP_MISMATCH,
                "application_host_bindings_v2.host_binding_claim_ids",
                application_id=link.application_semantic_id,
                claim_id=claim_id,
                member_key=claim.member_key,
                subject_ids=(claim_id,),
            )
        actual_by_key[member_key] = (claim_id, claim)

    if set(actual_by_key) != set(expected_by_key):
        raise _error(
            HOST_MEMBER_SET_MISMATCH,
            "application_host_bindings_v2.host_binding_claim_ids",
            application_id=link.application_semantic_id,
            subject_ids=tuple(link.host_binding_claim_ids),
        )


class ContextApplicationV2HostBindingEvaluator:
    """Run the typed Slice-6 boundary with Slice-5 currentness first."""

    def __init__(self, source_resolver: AuthoritySourceResolver) -> None:
        self._source_resolver = source_resolver

    @staticmethod
    def _project_verified_member(
        member: ContextApplicationMemberV2,
        candidate_record: Mapping[str, object],
    ) -> _ResolvedApplicationMember:
        try:
            expected_host_relationship = member.context_binding_v1[3]
        except (IndexError, TypeError) as exc:
            raise _error(
                HOST_INTEGRATION_INPUT_INVALID,
                "context_application_v2_records.members.context_binding_v1",
            ) from exc
        if not isinstance(expected_host_relationship, str):
            raise _error(
                HOST_INTEGRATION_INPUT_INVALID,
                "context_application_v2_records.members.context_binding_v1.host_relationship",
            )
        required = (
            candidate_record.get("scope") == "cross_deck"
            and candidate_record.get("relation") == "directional_binary"
        )
        if required and expected_host_relationship == "not_applicable":
            raise _error(
                HOST_RELATIONSHIP_MISMATCH,
                "context_application_v2_records.members.context_binding_v1.host_relationship",
                member_key=ApplicationMemberKeyV1(
                    candidate_id=member.candidate_id,
                    candidate_identity_digest=(
                        member.candidate_identity_digest_reference.digest_bytes
                    ),
                    source_instance_id=member.source_instance_id,
                ),
            )
        return _ResolvedApplicationMember(
            member_key=ApplicationMemberKeyV1(
                candidate_id=member.candidate_id,
                candidate_identity_digest=member.candidate_identity_digest_reference.digest_bytes,
                source_instance_id=member.source_instance_id,
            ),
            expected_host_relationship=expected_host_relationship,
            required=required,
        )

    def _resolve_record_members(
        self,
        record: ContextApplicationV2Record,
        base_authority_binding: ContextAuthoritySourceBindingV2,
    ) -> tuple[_ResolvedApplicationMember, ...]:
        resolver = ContextApplicationV2Resolver(
            self._source_resolver,
            base_authority_binding=base_authority_binding,
        )
        projections: list[_ResolvedApplicationMember] = []
        for member in record.members:
            member_key = ApplicationMemberKeyV1(
                candidate_id=member.candidate_id,
                candidate_identity_digest=member.candidate_identity_digest_reference.digest_bytes,
                source_instance_id=member.source_instance_id,
            )
            try:
                resolved = resolver.resolve_member_source_instance(member)
            except (ContextApplicationV2ResolutionError, ResolutionError) as exc:
                raise _error(
                    HOST_MEMBER_APPLICABILITY_INVALID,
                    "context_application_v2_records.members",
                    cause_code=exc.code,
                    application_id=record.application_id,
                    record_id=record.record_id,
                    member_key=member_key,
                ) from exc
            projections.append(
                self._project_verified_member(member, resolved.candidate.candidate_record)
            )
        return tuple(projections)

    def _derive_application_closures(
        self,
        records: tuple[ContextApplicationV2Record, ...],
        currentness: ContextApplicationV2CurrentnessResult,
        base_authority_binding: ContextAuthoritySourceBindingV2,
    ) -> tuple[_ApplicationMemberClosure, ...]:
        current_record_keys = {
            _identity_key(record_id) for record_id in currentness.current_record_ids
        }
        groups: dict[bytes, list[ContextApplicationV2Record]] = {}
        for record in records:
            groups.setdefault(_identity_key(record.application_id), []).append(record)

        closures: list[_ApplicationMemberClosure] = []
        for application_key in sorted(groups):
            group = tuple(
                sorted(groups[application_key], key=lambda item: encode_canonical(item.to_cbor()))
            )
            current_records = tuple(
                record for record in group if _identity_key(record.record_id) in current_record_keys
            )
            if len(current_records) > 1:
                raise _error(
                    APPLICATION_CURRENTNESS_FAILED,
                    "context_application_v2_records.application_id",
                    cause_code="CURRENTNESS_AMBIGUOUS",
                    application_id=group[0].application_id,
                    subject_ids=tuple(record.record_id.as_text() for record in current_records),
                )
            selected = current_records[0] if current_records else group[0]
            members = self._resolve_record_members(selected, base_authority_binding)
            required_members = tuple(
                sorted(
                    (member for member in members if member.required),
                    key=lambda member: _member_key_bytes(member.member_key),
                )
            )
            closures.append(
                _ApplicationMemberClosure(
                    application_id=selected.application_id,
                    record_id=selected.record_id,
                    current=bool(current_records),
                    members=members,
                    required_member_keys=tuple(member.member_key for member in required_members),
                )
            )
        return tuple(sorted(closures, key=lambda item: _identity_key(item.application_id)))

    @staticmethod
    def _preflight(
        container: ContextApplicationAuthorityV2,
    ) -> tuple[
        tuple[ContextApplicationV2Record, ...],
        tuple[ContextApplicationV2SupersessionRecord, ...],
        tuple[ApplicationHostBindingV2, ...],
    ]:
        if not isinstance(container, ContextApplicationAuthorityV2):
            raise _error(HOST_INTEGRATION_INPUT_INVALID, "container")
        if not isinstance(container.base_authority_v1_binding, ContextAuthoritySourceBindingV2):
            raise _error(HOST_INTEGRATION_INPUT_INVALID, "base_authority_v1_binding")
        if not isinstance(container.candidate_universe_binding, ContextAuthoritySourceBindingV2):
            raise _error(HOST_INTEGRATION_INPUT_INVALID, "candidate_universe_binding")
        if container.host_binding_authority_v2_binding is not None and not isinstance(
            container.host_binding_authority_v2_binding,
            ContextAuthoritySourceBindingV2,
        ):
            raise _error(HOST_INTEGRATION_INPUT_INVALID, "host_binding_authority_v2_binding")

        source_bindings = _require_tuple(container.source_bindings, "source_bindings")
        application_records = _require_tuple(
            container.context_application_v2_records,
            "context_application_v2_records",
        )
        supersession_records = _require_tuple(
            container.context_application_v2_supersession_records,
            "context_application_v2_supersession_records",
        )
        application_links = _require_tuple(
            container.application_host_bindings_v2,
            "application_host_bindings_v2",
        )

        for value in source_bindings:
            if not isinstance(value, ContextAuthoritySourceBindingV2):
                raise _error(HOST_INTEGRATION_INPUT_INVALID, "source_bindings")
        _require_canonical_items(source_bindings, "source_bindings")

        for value in application_records:
            if not isinstance(value, ContextApplicationV2Record):
                raise _error(HOST_INTEGRATION_INPUT_INVALID, "context_application_v2_records")
        _require_canonical_items(application_records, "context_application_v2_records")
        application_record_values = tuple(
            cast(ContextApplicationV2Record, value) for value in application_records
        )
        application_record_ids = _require_identity_collection(
            tuple(record.record_id for record in application_record_values),
            "context_application_v2_records.record_id",
        )
        if len({_identity_key(value) for value in application_record_ids}) != len(
            application_record_ids
        ):
            raise _error(
                HOST_INTEGRATION_INPUT_INVALID,
                "context_application_v2_records.record_id",
            )

        for value in supersession_records:
            if not isinstance(value, ContextApplicationV2SupersessionRecord):
                raise _error(
                    HOST_INTEGRATION_INPUT_INVALID,
                    "context_application_v2_supersession_records",
                )
        _require_canonical_items(
            supersession_records,
            "context_application_v2_supersession_records",
        )
        supersession_record_values = tuple(
            cast(ContextApplicationV2SupersessionRecord, value) for value in supersession_records
        )
        supersession_record_ids = _require_identity_collection(
            tuple(record.record_id for record in supersession_record_values),
            "context_application_v2_supersession_records.record_id",
        )
        if len({_identity_key(value) for value in supersession_record_ids}) != len(
            supersession_record_ids
        ):
            raise _error(
                HOST_INTEGRATION_INPUT_INVALID,
                "context_application_v2_supersession_records.record_id",
            )

        for value in application_links:
            if not isinstance(value, ApplicationHostBindingV2):
                raise _error(HOST_INTEGRATION_INPUT_INVALID, "application_host_bindings_v2")
        _require_canonical_items(application_links, "application_host_bindings_v2")
        link_values = tuple(cast(ApplicationHostBindingV2, value) for value in application_links)
        application_ids = tuple(link.application_semantic_id for link in link_values)
        if len({_identity_key(value) for value in application_ids}) != len(application_ids):
            duplicate_id = next(
                identity
                for index, identity in enumerate(application_ids)
                if identity in application_ids[:index]
            )
            raise _error(
                APPLICATION_HOST_BINDING_DUPLICATE,
                "application_host_bindings_v2.application_semantic_id",
                application_id=duplicate_id,
                subject_ids=(duplicate_id.as_text(),),
            )

        return (
            application_record_values,
            supersession_record_values,
            link_values,
        )

    @staticmethod
    def _wrap_currentness_error(
        error: ContextApplicationV2CurrentnessError,
    ) -> ContextApplicationV2HostBindingError:
        subject_ids = tuple(
            identity.as_text()
            for identity in (*error.subject_record_ids, *error.subject_supersession_ids)
        )
        return _error(
            APPLICATION_CURRENTNESS_FAILED,
            error.location,
            cause_code=error.code,
            application_id=error.application_id,
            record_id=error.record_id,
            subject_ids=subject_ids,
        )

    def _evaluate_container(
        self,
        container: ContextApplicationAuthorityV2,
    ) -> ContextApplicationV2HostBindingEvaluationResult:
        records, supersessions, links = self._preflight(container)

        try:
            currentness = ContextApplicationV2CurrentnessEvaluator(
                self._source_resolver,
                base_authority_binding=container.base_authority_v1_binding,
            ).evaluate(records, supersessions)
        except ContextApplicationV2CurrentnessError as exc:
            raise self._wrap_currentness_error(exc) from exc

        known_application_ids = {record.application_id for record in records}
        for link in links:
            if link.application_semantic_id not in known_application_ids:
                raise _error(
                    APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION,
                    "application_host_bindings_v2.application_semantic_id",
                    application_id=link.application_semantic_id,
                    subject_ids=(link.application_semantic_id.as_text(),),
                )

        closures = self._derive_application_closures(
            records,
            currentness,
            container.base_authority_v1_binding,
        )
        links_by_application = {link.application_semantic_id: link for link in links}
        qualified_current_record_ids: list[AuthorityIdentityV1] = []
        application_results: list[ApplicationHostBindingResult] = []
        for closure in closures:
            link = links_by_application.get(closure.application_id)
            if not closure.required_member_keys:
                if link is not None:
                    raise _error(
                        HOST_MEMBER_SET_MISMATCH,
                        "application_host_bindings_v2.host_binding_claim_ids",
                        application_id=closure.application_id,
                        subject_ids=tuple(link.host_binding_claim_ids),
                    )
                if closure.current:
                    qualified_current_record_ids.append(closure.record_id)
                continue

            if link is None:
                if closure.current:
                    raise _error(
                        APPLICATION_HOST_BINDING_INVALID,
                        "application_host_bindings_v2",
                        application_id=closure.application_id,
                        record_id=closure.record_id,
                        subject_ids=tuple(
                            member_key.candidate_id for member_key in closure.required_member_keys
                        ),
                    )
                continue
            if len(link.host_binding_claim_ids) != len(closure.required_member_keys):
                raise _error(
                    HOST_MEMBER_SET_MISMATCH,
                    "application_host_bindings_v2.host_binding_claim_ids",
                    application_id=closure.application_id,
                    record_id=closure.record_id,
                    subject_ids=tuple(link.host_binding_claim_ids),
                )
            # Task 4 proves the G2 link shape only.  A required link is not
            # qualified until Task 5 supplies the admitted HBC claim index and
            # the exact claim/member closure has passed.

        return ContextApplicationV2HostBindingEvaluationResult(
            currentness=currentness,
            qualified_current_application_record_ids=tuple(
                sorted(qualified_current_record_ids, key=_identity_key)
            ),
            application_host_binding_results=tuple(application_results),
            current_host_claim_ids=(),
        )

    def evaluate(
        self,
        container: ContextApplicationAuthorityV2,
    ) -> ContextApplicationV2HostBindingEvaluationResult:
        return self._evaluate_container(container)


__all__ = [
    "APPLICATION_CURRENTNESS_FAILED",
    "APPLICATION_HOST_BINDING_DUPLICATE",
    "APPLICATION_HOST_BINDING_INVALID",
    "APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION",
    "HOST_CLAIM_UNKNOWN",
    "HOST_INTEGRATION_INPUT_INVALID",
    "HOST_MEMBER_APPLICABILITY_INVALID",
    "HOST_MEMBER_SET_MISMATCH",
    "HOST_RELATIONSHIP_MISMATCH",
    "ApplicationHostBindingResult",
    "ApplicationHostBindingStatus",
    "ContextApplicationV2HostBindingError",
    "ContextApplicationV2HostBindingEvaluationResult",
    "ContextApplicationV2HostBindingEvaluator",
    "HostBindingClaimRecordStatus",
]
