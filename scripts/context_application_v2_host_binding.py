"""Structural Slice-6 composition entrypoint for ContextApplicationV2.

Task 3 owns the typed boundary and the currentness-first ordering.  Member
applicability and HostBinding claim composition are deliberately added by the
later Slice-6 tasks; this module must not invent a second lifecycle authority.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Final, cast

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import AuthoritySourceResolver
from authority_v2_validator import HostBindingClaimRecordStatus
from context_application_v2_supersession import (
    ContextApplicationV2CurrentnessError,
    ContextApplicationV2CurrentnessEvaluator,
    ContextApplicationV2CurrentnessResult,
)
from mtgml.authority import (
    ApplicationHostBindingV2,
    AuthorityIdentityV1,
    ContextApplicationAuthorityV2,
    ContextApplicationV2Record,
    ContextApplicationV2SupersessionRecord,
    ContextAuthoritySourceBindingV2,
)
from mtgml.host_binding import ApplicationMemberKeyV1
from mtgml.persistence import encode_canonical

HOST_INTEGRATION_INPUT_INVALID: Final = "HOST_INTEGRATION_INPUT_INVALID"
APPLICATION_CURRENTNESS_FAILED: Final = "APPLICATION_CURRENTNESS_FAILED"
APPLICATION_HOST_BINDING_DUPLICATE: Final = "APPLICATION_HOST_BINDING_DUPLICATE"
APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION: Final = "APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION"


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


def _identity_key(identity: AuthorityIdentityV1) -> bytes:
    return encode_canonical(identity.to_cbor())


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


class ContextApplicationV2HostBindingEvaluator:
    """Run the typed Slice-6 boundary with Slice-5 currentness first."""

    def __init__(self, source_resolver: AuthoritySourceResolver) -> None:
        self._source_resolver = source_resolver

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

        return ContextApplicationV2HostBindingEvaluationResult(
            currentness=currentness,
            qualified_current_application_record_ids=(),
            application_host_binding_results=(),
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
    "APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION",
    "HOST_INTEGRATION_INPUT_INVALID",
    "ApplicationHostBindingResult",
    "ApplicationHostBindingStatus",
    "ContextApplicationV2HostBindingError",
    "ContextApplicationV2HostBindingEvaluationResult",
    "ContextApplicationV2HostBindingEvaluator",
    "HostBindingClaimRecordStatus",
]
