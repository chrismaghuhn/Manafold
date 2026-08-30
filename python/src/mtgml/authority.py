"""Rules-neutral M2.5.C authority contract and identity primitives.

This module owns only the fixed V1 persistence shapes needed by the authority
boundary. It does not resolve sources, validate Magic semantics, classify C
candidates, or derive interaction classes.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from enum import Enum
from typing import Final, Protocol, TypeAlias

from .persistence import (
    CANONICAL_CBOR_ID,
    DIGEST_ENVELOPE_ID,
    SHA256_ID,
    encode_canonical,
    encode_envelope,
    hash_envelope,
)

AUTHORITY_SCHEMA_V1: Final = "manafold.m2.5.c.interaction-review-authority.v1"
ACCEPTANCE_EVENT_SCHEMA_V1: Final = "manafold.m2.5.c.review-acceptance-event.v1"
REVIEWER_ROSTER_SCHEMA_V1: Final = "manafold.m2.5.c.reviewer-roster.v1"
ACCEPTANCE_CHECKLIST_V1: Final = "interaction-authority-review-checklist.v1"
SUPERSESSION_RECORD_SCHEMA_V1: Final = "manafold.m2.5.c.supersession-record.v1"

_RAW_REV3_PATHS: Final = frozenset(
    {
        "derived/Pair_Interaction_Census_REV3.csv",
        "inputs/deck_row_source_resolution_REV3.csv",
        "source/raw/source_record_index_REV3.csv",
        "source/raw/oracle_cards_selected_REV3.jsonl",
    }
)
_ARTIFACT_ROLES: Final = frozenset(
    {
        "declared_model",
        "rev3_source",
        "b2_catalog",
        "b2_classifications",
        "b2_closure",
        "b1_final_citations",
        "b1_final_closure",
        "candidate_universe",
        "acceptance_event_leaf",
        "reviewer_roster_leaf",
    }
)
_EXPECTED_SCHEMA_BY_ROLE: Final = {
    "declared_model": "manafold.m2.5.c.declared-interaction-model.v2",
    "b2_catalog": "manafold.m2.5.b2.requirement-family-catalog.v1",
    "b2_classifications": "manafold.m2.5.b2.card-semantic-classifications.v1",
    "b2_closure": "manafold.m2.5.b2.classification-closure.v1",
    "b1_final_citations": "manafold.m2.5.b1.official-authority-citations.v3",
    "b1_final_closure": "manafold.m2.5.b1.official-authority-citation-closure.v2",
    "candidate_universe": "manafold.m2.5.c.interaction-candidate-universe.v2",
    "acceptance_event_leaf": ACCEPTANCE_EVENT_SCHEMA_V1,
    "reviewer_roster_leaf": REVIEWER_ROSTER_SCHEMA_V1,
}
_AUTHORITY_KINDS: Final = frozenset(
    {
        "model",
        "rev3",
        "b2",
        "b1_final",
        "c_candidate",
        "reviewer_roster",
        "acceptance_event",
    }
)
REVIEWER_ROLES: Final = (
    "project_owner",
    "architecture_maintainer",
    "rules_authority_maintainer",
    "information_safety_reviewer",
    "conformance_maintainer",
)
_REVIEWER_ROLE_SET: Final = frozenset(REVIEWER_ROLES)

AuthorityValue: TypeAlias = bool | int | bytes | str | list["AuthorityValue"] | None
LocatorV1: TypeAlias = tuple[str, str | int | None]


class _CborConvertible(Protocol):
    def to_cbor(self) -> list[AuthorityValue]: ...


class AuthorityContractError(ValueError):
    """Raised when a foundational authority value violates its V1 contract."""


class AuthorityIdentityKind(str, Enum):
    RELATION_THEOREM = "relation_theorem"
    RELATION_THEOREM_RECORD = "relation_theorem_record"
    RELATION_APPLICATION = "relation_application"
    RELATION_APPLICATION_RECORD = "relation_application_record"
    RELATION_SUPERSESSION = "relation_supersession"
    DOMAIN_THEOREM = "domain_theorem"
    DOMAIN_THEOREM_RECORD = "domain_theorem_record"
    DOMAIN_APPLICATION = "domain_application"
    DOMAIN_APPLICATION_RECORD = "domain_application_record"
    DOMAIN_SUPERSESSION = "domain_supersession"
    CONTEXT_THEOREM = "context_theorem"
    CONTEXT_THEOREM_RECORD = "context_theorem_record"
    CONTEXT_APPLICATION = "context_application"
    CONTEXT_APPLICATION_RECORD = "context_application_record"
    CONTEXT_SUPERSESSION = "context_supersession"
    ACCEPTANCE_SUBJECT = "acceptance_subject"
    REVIEW_ACCEPTANCE_EVENT = "review_acceptance_event"


class AcceptanceSubjectKind(str, Enum):
    RELATION_THEOREM_RECORD = "relation_theorem_record"
    DOMAIN_THEOREM_RECORD = "domain_theorem_record"
    CONTEXT_THEOREM_RECORD = "context_theorem_record"
    RELATION_APPLICATION_RECORD = "relation_application_record"
    DOMAIN_APPLICATION_RECORD = "domain_application_record"
    CONTEXT_APPLICATION_RECORD = "context_application_record"
    SUPERSESSION_RECORD = "supersession_record"


class ReviewMode(str, Enum):
    MULTI_REVIEWER = "multi_reviewer"
    SOLO_SEPARATE_SELF_REVIEW = "solo_separate_self_review"


@dataclass(frozen=True)
class AcceptanceSubjectPayloadV1:
    subject_kind: AcceptanceSubjectKind
    subject_payload: list[AuthorityValue]

    def __post_init__(self) -> None:
        if not isinstance(self.subject_kind, AcceptanceSubjectKind):
            raise AuthorityContractError("acceptance subject kind is not closed in V1")
        if not isinstance(self.subject_payload, list):
            raise AuthorityContractError("acceptance subject payload must be an array")

    def semantic_input(self) -> list[AuthorityValue]:
        return [
            _IDENTITY_SPECS[AuthorityIdentityKind.ACCEPTANCE_SUBJECT].input_schema_id,
            self.subject_kind.value,
            self.subject_payload,
        ]

    def identity(self) -> AuthorityIdentityV1:
        return compute_authority_identity(
            AuthorityIdentityKind.ACCEPTANCE_SUBJECT,
            self.semantic_input(),
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.subject_kind.value, self.subject_payload]


@dataclass(frozen=True)
class _IdentitySpec:
    prefix: str
    semantic_domain: str
    input_schema_id: str


_IDENTITY_SPECS: Final[dict[AuthorityIdentityKind, _IdentitySpec]] = {
    AuthorityIdentityKind.RELATION_THEOREM: _IdentitySpec(
        "rp.v1/",
        "manafold.m2.5.c.relation-proof.v1",
        "manafold.m2.5.c.relation-proof-input.v1",
    ),
    AuthorityIdentityKind.RELATION_THEOREM_RECORD: _IdentitySpec(
        "rpr.v1/",
        "manafold.m2.5.c.relation-proof-record.v1",
        "manafold.m2.5.c.relation-proof-record-input.v1",
    ),
    AuthorityIdentityKind.RELATION_APPLICATION: _IdentitySpec(
        "rpa.v1/",
        "manafold.m2.5.c.relation-application.v1",
        "manafold.m2.5.c.relation-application-input.v1",
    ),
    AuthorityIdentityKind.RELATION_APPLICATION_RECORD: _IdentitySpec(
        "rpar.v1/",
        "manafold.m2.5.c.relation-application-record.v1",
        "manafold.m2.5.c.relation-application-record-input.v1",
    ),
    AuthorityIdentityKind.RELATION_SUPERSESSION: _IdentitySpec(
        "rps.v1/",
        "manafold.m2.5.c.relation-supersession.v1",
        "manafold.m2.5.c.relation-supersession-input.v1",
    ),
    AuthorityIdentityKind.DOMAIN_THEOREM: _IdentitySpec(
        "dp.v1/",
        "manafold.m2.5.c.domain-proof.v1",
        "manafold.m2.5.c.domain-proof-input.v1",
    ),
    AuthorityIdentityKind.DOMAIN_THEOREM_RECORD: _IdentitySpec(
        "dpr.v1/",
        "manafold.m2.5.c.domain-proof-record.v1",
        "manafold.m2.5.c.domain-proof-record-input.v1",
    ),
    AuthorityIdentityKind.DOMAIN_APPLICATION: _IdentitySpec(
        "dpa.v1/",
        "manafold.m2.5.c.domain-application.v1",
        "manafold.m2.5.c.domain-application-input.v1",
    ),
    AuthorityIdentityKind.DOMAIN_APPLICATION_RECORD: _IdentitySpec(
        "dpar.v1/",
        "manafold.m2.5.c.domain-application-record.v1",
        "manafold.m2.5.c.domain-application-record-input.v1",
    ),
    AuthorityIdentityKind.DOMAIN_SUPERSESSION: _IdentitySpec(
        "dps.v1/",
        "manafold.m2.5.c.domain-supersession.v1",
        "manafold.m2.5.c.domain-supersession-input.v1",
    ),
    AuthorityIdentityKind.CONTEXT_THEOREM: _IdentitySpec(
        "cp.v1/",
        "manafold.m2.5.c.context-proof.v1",
        "manafold.m2.5.c.context-proof-input.v1",
    ),
    AuthorityIdentityKind.CONTEXT_THEOREM_RECORD: _IdentitySpec(
        "cpr.v1/",
        "manafold.m2.5.c.context-proof-record.v1",
        "manafold.m2.5.c.context-proof-record-input.v1",
    ),
    AuthorityIdentityKind.CONTEXT_APPLICATION: _IdentitySpec(
        "cpa.v1/",
        "manafold.m2.5.c.context-application.v1",
        "manafold.m2.5.c.context-application-input.v1",
    ),
    AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD: _IdentitySpec(
        "cpar.v1/",
        "manafold.m2.5.c.context-application-record.v1",
        "manafold.m2.5.c.context-application-record-input.v1",
    ),
    AuthorityIdentityKind.CONTEXT_SUPERSESSION: _IdentitySpec(
        "cps.v1/",
        "manafold.m2.5.c.context-supersession.v1",
        "manafold.m2.5.c.context-supersession-input.v1",
    ),
    AuthorityIdentityKind.ACCEPTANCE_SUBJECT: _IdentitySpec(
        "asp.v1/",
        "manafold.m2.5.c.acceptance-subject-payload.v1",
        "manafold.m2.5.c.acceptance-subject-payload-input.v1",
    ),
    AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT: _IdentitySpec(
        "ae.v1/",
        "manafold.m2.5.c.review-acceptance-event.v1",
        "manafold.m2.5.c.review-acceptance-event-input.v1",
    ),
}
_IDENTITY_ARITIES: Final[dict[AuthorityIdentityKind, int]] = {
    AuthorityIdentityKind.RELATION_THEOREM: 12,
    AuthorityIdentityKind.RELATION_THEOREM_RECORD: 5,
    AuthorityIdentityKind.RELATION_APPLICATION: 4,
    AuthorityIdentityKind.RELATION_APPLICATION_RECORD: 3,
    AuthorityIdentityKind.RELATION_SUPERSESSION: 8,
    AuthorityIdentityKind.DOMAIN_THEOREM: 8,
    AuthorityIdentityKind.DOMAIN_THEOREM_RECORD: 5,
    AuthorityIdentityKind.DOMAIN_APPLICATION: 5,
    AuthorityIdentityKind.DOMAIN_APPLICATION_RECORD: 3,
    AuthorityIdentityKind.DOMAIN_SUPERSESSION: 8,
    AuthorityIdentityKind.CONTEXT_THEOREM: 8,
    AuthorityIdentityKind.CONTEXT_THEOREM_RECORD: 5,
    AuthorityIdentityKind.CONTEXT_APPLICATION: 3,
    AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD: 3,
    AuthorityIdentityKind.CONTEXT_SUPERSESSION: 8,
    AuthorityIdentityKind.ACCEPTANCE_SUBJECT: 3,
    AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT: 10,
}


@dataclass(frozen=True)
class AuthorityIdentityV1:
    kind: AuthorityIdentityKind
    digest_bytes: bytes

    def __post_init__(self) -> None:
        if not isinstance(self.kind, AuthorityIdentityKind):
            raise AuthorityContractError("identity kind is not a closed V1 variant")
        _require_digest_bytes(self.digest_bytes, "identity digest")

    @property
    def prefix(self) -> str:
        return _IDENTITY_SPECS[self.kind].prefix

    @property
    def semantic_domain(self) -> str:
        return _IDENTITY_SPECS[self.kind].semantic_domain

    @property
    def input_schema_id(self) -> str:
        return _IDENTITY_SPECS[self.kind].input_schema_id

    def as_text(self) -> str:
        return self.prefix + self.digest_bytes.hex()

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            DIGEST_ENVELOPE_ID,
            SHA256_ID,
            self.semantic_domain,
            CANONICAL_CBOR_ID,
            self.input_schema_id,
            self.digest_bytes,
        ]


def canonical_identity_input(
    kind: AuthorityIdentityKind,
    fields: list[AuthorityValue],
) -> list[AuthorityValue]:
    if not isinstance(kind, AuthorityIdentityKind):
        raise AuthorityContractError("identity kind is not a closed V1 variant")
    if not isinstance(fields, list):
        raise AuthorityContractError("authority identity preimage must be an array")
    expected = _IDENTITY_ARITIES[kind]
    if len(fields) != expected:
        raise AuthorityContractError(
            f"{kind.value} identity preimage must contain exactly {expected} fields"
        )
    if fields[0] != _IDENTITY_SPECS[kind].input_schema_id:
        raise AuthorityContractError("identity preimage schema field does not match its kind")
    return fields


def compute_authority_identity(
    kind: AuthorityIdentityKind,
    payload: list[AuthorityValue],
) -> AuthorityIdentityV1:
    payload = canonical_identity_input(kind, payload)
    canonical_payload = encode_canonical(payload)
    envelope = encode_envelope(
        _IDENTITY_SPECS[kind].semantic_domain,
        _IDENTITY_SPECS[kind].input_schema_id,
        canonical_payload,
    )
    return AuthorityIdentityV1(kind, hash_envelope(envelope))


def _require_digest_bytes(value: object, label: str) -> bytes:
    if not isinstance(value, bytes) or len(value) != 32:
        raise AuthorityContractError(f"{label} must contain exactly 32 bytes")
    return value


def _require_repo_relative_path(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise AuthorityContractError(f"{label} must be a non-empty path")
    if (
        value.startswith(("/", "\\"))
        or ":" in value.split("/", 1)[0]
        or "\\" in value
        or "://" in value
    ):
        raise AuthorityContractError(f"{label} must be repository-relative")
    segments = value.split("/")
    if any(segment in {"", ".", ".."} for segment in segments):
        raise AuthorityContractError(f"{label} contains an invalid path segment")
    return value


def _require_locator(value: object, *, acceptance: bool) -> LocatorV1:
    if not isinstance(value, tuple) or len(value) != 2:
        raise AuthorityContractError("locator must be a two-position tagged tuple")
    kind, payload = value
    if kind == "whole_artifact":
        if payload is not None:
            raise AuthorityContractError("whole_artifact locator payload must be null")
    elif kind == "json_pointer":
        if not isinstance(payload, str) or re.fullmatch(r"(?:/([^~/]|~0|~1)*)*", payload) is None:
            raise AuthorityContractError("json_pointer locator payload must be text")
    elif kind == "archive_member":
        if not isinstance(payload, str):
            raise AuthorityContractError("archive_member locator payload must be text")
        _require_repo_relative_path(payload, "archive member")
    elif kind == "event_id":
        if (
            acceptance
            or not isinstance(payload, str)
            or re.fullmatch(r"ae\.v1/[0-9a-f]{64}", payload) is None
        ):
            raise AuthorityContractError("event_id locator is not valid here")
    else:
        raise AuthorityContractError("locator variant is not closed in V1")
    return value


def _require_canonical_sequence(values: tuple[_CborConvertible, ...], label: str) -> None:
    encoded = [encode_canonical(value.to_cbor()) for value in values]
    if encoded != sorted(encoded):
        raise AuthorityContractError(f"{label} must be in canonical order")
    if len(set(encoded)) != len(encoded):
        raise AuthorityContractError(f"{label} must be duplicate-free")


@dataclass(frozen=True)
class SourceBindingDigestV1:
    artifact_role: str
    path: str
    schema_or_null: str | None
    raw_sha256: bytes

    def __post_init__(self) -> None:
        if self.artifact_role not in _ARTIFACT_ROLES:
            raise AuthorityContractError("artifact role is not closed in V1")
        _require_repo_relative_path(self.path, "source binding path")
        if self.path in _RAW_REV3_PATHS:
            if self.artifact_role != "rev3_source" or self.schema_or_null is not None:
                raise AuthorityContractError(
                    "raw REV3 source bindings require role=rev3_source and schema_or_null=null"
                )
        elif self.artifact_role == "rev3_source":
            if (
                self.path != "inputs/interaction_model_v1.json"
                or self.schema_or_null != "interaction-model.v1"
            ):
                raise AuthorityContractError("REV3 source binding path/schema is not admitted")
        elif self.schema_or_null != _EXPECTED_SCHEMA_BY_ROLE.get(self.artifact_role):
            raise AuthorityContractError("source binding schema does not match its artifact role")
        _require_digest_bytes(self.raw_sha256, "source binding digest")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.artifact_role, self.path, self.schema_or_null, self.raw_sha256]


@dataclass(frozen=True)
class EvidenceRefV1:
    authority_kind: str
    path: str
    locator: LocatorV1
    raw_sha256: bytes

    def __post_init__(self) -> None:
        if self.authority_kind not in _AUTHORITY_KINDS:
            raise AuthorityContractError("authority kind is not closed in V1")
        _require_repo_relative_path(self.path, "evidence path")
        _require_locator(self.locator, acceptance=False)
        _require_digest_bytes(self.raw_sha256, "evidence digest")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.authority_kind, self.path, list(self.locator), self.raw_sha256]


@dataclass(frozen=True)
class AcceptanceEvidenceRefV1:
    path: str
    raw_sha256: bytes
    locator: LocatorV1

    def __post_init__(self) -> None:
        _require_repo_relative_path(self.path, "acceptance evidence path")
        _require_digest_bytes(self.raw_sha256, "acceptance evidence digest")
        _require_locator(self.locator, acceptance=True)

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.path, self.raw_sha256, list(self.locator)]


@dataclass(frozen=True)
class ReviewEventRefV1:
    path: str
    raw_sha256: bytes
    event_id: str

    def __post_init__(self) -> None:
        _require_repo_relative_path(self.path, "review event path")
        _require_digest_bytes(self.raw_sha256, "review event digest")
        if re.fullmatch(r"ae\.v1/[0-9a-f]{64}", self.event_id) is None:
            raise AuthorityContractError("review event ID has the wrong V1 namespace")
        expected_path = (
            "sources/m2_5/authorities/review_acceptance_events/v1/"
            + self.event_id.removeprefix("ae.v1/")
            + ".json"
        )
        if self.path != expected_path:
            raise AuthorityContractError("review event path is not bound to its event ID")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.path, self.raw_sha256, ["event_id", self.event_id]]


@dataclass(frozen=True)
class AcceptanceV1:
    review_event_ref: ReviewEventRefV1

    def to_cbor(self) -> list[AuthorityValue]:
        return ["human_accepted", self.review_event_ref.to_cbor()]


@dataclass(frozen=True)
class ReviewerRosterRefV1:
    path: str
    schema: str
    raw_sha256: bytes

    def __post_init__(self) -> None:
        _require_repo_relative_path(self.path, "reviewer roster path")
        if self.schema != REVIEWER_ROSTER_SCHEMA_V1:
            raise AuthorityContractError("reviewer roster schema is not the V1 contract")
        _require_digest_bytes(self.raw_sha256, "reviewer roster digest")
        expected_path = (
            "sources/m2_5/authorities/reviewer_rosters/v1/" + self.raw_sha256.hex() + ".json"
        )
        if self.path != expected_path:
            raise AuthorityContractError("reviewer roster path is not bound to its raw digest")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.path, self.schema, self.raw_sha256]


@dataclass(frozen=True)
class ReviewerRoleBindingV1:
    reviewer_id: str
    roles: tuple[str, ...]

    def __post_init__(self) -> None:
        if not isinstance(self.reviewer_id, str) or not self.reviewer_id:
            raise AuthorityContractError("reviewer ID must be non-empty")
        if any(role not in _REVIEWER_ROLE_SET for role in self.roles):
            raise AuthorityContractError("reviewer role is not closed in V1")
        if tuple(sorted(self.roles)) != self.roles:
            raise AuthorityContractError("reviewer roles must be in canonical order")
        if len(set(self.roles)) != len(self.roles):
            raise AuthorityContractError("reviewer roles must be duplicate-free")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.reviewer_id, list(self.roles)]


@dataclass(frozen=True)
class ReviewerV1:
    reviewer_id: str
    roles: tuple[str, ...]

    def __post_init__(self) -> None:
        if not isinstance(self.reviewer_id, str) or not self.reviewer_id:
            raise AuthorityContractError("reviewer ID must be non-empty")
        if any(role not in _REVIEWER_ROLE_SET for role in self.roles):
            raise AuthorityContractError("reviewer role is not closed in V1")
        if tuple(sorted(self.roles)) != self.roles:
            raise AuthorityContractError("reviewer roles must be in canonical order")
        if len(set(self.roles)) != len(self.roles):
            raise AuthorityContractError("reviewer roles must be duplicate-free")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.reviewer_id, list(self.roles)]


@dataclass(frozen=True)
class ReviewerRosterV1:
    reviewers: tuple[ReviewerV1, ...]

    def __post_init__(self) -> None:
        if not self.reviewers:
            raise AuthorityContractError("reviewer roster must contain a reviewer")
        reviewer_ids = tuple(reviewer.reviewer_id for reviewer in self.reviewers)
        if reviewer_ids != tuple(sorted(reviewer_ids)):
            raise AuthorityContractError("reviewers must be in canonical order")
        if len(set(reviewer_ids)) != len(reviewer_ids):
            raise AuthorityContractError("reviewers must be duplicate-free")

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            REVIEWER_ROSTER_SCHEMA_V1,
            [reviewer.to_cbor() for reviewer in self.reviewers],
        ]


@dataclass(frozen=True)
class ReviewAcceptanceEventV1:
    subject_kind: AcceptanceSubjectKind
    subject_payload_digest: bytes
    reviewer_roster_ref: ReviewerRosterRefV1
    reviewer_role_bindings: tuple[ReviewerRoleBindingV1, ...]
    review_mode: ReviewMode
    source_binding_digests: tuple[SourceBindingDigestV1, ...]
    review_evidence_refs: tuple[AcceptanceEvidenceRefV1, ...]

    def __post_init__(self) -> None:
        if not isinstance(self.subject_kind, AcceptanceSubjectKind):
            raise AuthorityContractError("acceptance subject kind is not closed in V1")
        _require_digest_bytes(self.subject_payload_digest, "acceptance subject digest")
        if not self.reviewer_role_bindings:
            raise AuthorityContractError("acceptance requires reviewer role bindings")
        reviewer_ids = tuple(binding.reviewer_id for binding in self.reviewer_role_bindings)
        if reviewer_ids != tuple(sorted(reviewer_ids)):
            raise AuthorityContractError("reviewer bindings must be in canonical order")
        if len(set(reviewer_ids)) != len(reviewer_ids):
            raise AuthorityContractError("reviewer bindings must be duplicate-free")
        if not self.source_binding_digests:
            raise AuthorityContractError("acceptance requires source bindings")
        if not self.review_evidence_refs:
            raise AuthorityContractError("acceptance requires review evidence")
        if any(
            binding.artifact_role == "acceptance_event_leaf"
            for binding in self.source_binding_digests
        ):
            raise AuthorityContractError("acceptance event cannot bind its own leaf")
        if not any(
            binding.artifact_role == "declared_model" for binding in self.source_binding_digests
        ):
            raise AuthorityContractError("acceptance event must bind the declared model")
        expected_roster_binding = SourceBindingDigestV1(
            artifact_role="reviewer_roster_leaf",
            path=self.reviewer_roster_ref.path,
            schema_or_null=self.reviewer_roster_ref.schema,
            raw_sha256=self.reviewer_roster_ref.raw_sha256,
        )
        if expected_roster_binding not in self.source_binding_digests:
            raise AuthorityContractError("acceptance event must bind its reviewer roster leaf")
        _require_canonical_sequence(self.source_binding_digests, "source bindings")
        _require_canonical_sequence(self.review_evidence_refs, "review evidence")

    def semantic_input(self) -> list[AuthorityValue]:
        return [
            _IDENTITY_SPECS[AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT].input_schema_id,
            self.subject_kind.value,
            self.subject_payload_digest,
            "human_accepted",
            self.reviewer_roster_ref.to_cbor(),
            [binding.to_cbor() for binding in self.reviewer_role_bindings],
            self.review_mode.value,
            "interaction-authority-review-checklist.v1",
            [binding.to_cbor() for binding in self.source_binding_digests],
            [evidence.to_cbor() for evidence in self.review_evidence_refs],
        ]

    def identity(self) -> AuthorityIdentityV1:
        return compute_authority_identity(
            AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT,
            self.semantic_input(),
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return self.semantic_input()


class RecordKind(str, Enum):
    RELATION_THEOREM_RECORD = "relation_theorem_record"
    RELATION_APPLICATION_RECORD = "relation_application_record"
    DOMAIN_THEOREM_RECORD = "domain_theorem_record"
    DOMAIN_APPLICATION_RECORD = "domain_application_record"
    CONTEXT_THEOREM_RECORD = "context_theorem_record"
    CONTEXT_APPLICATION_RECORD = "context_application_record"


class SupersessionReason(str, Enum):
    SEMANTIC_CORRECTION = "semantic_correction"
    SOURCE_REVISION = "source_revision"
    MODEL_REVISION = "model_revision"
    AUTHORITY_REVOCATION = "authority_revocation"


_SUPERSESSION_ID_KIND: Final[dict[RecordKind, AuthorityIdentityKind]] = {
    RecordKind.RELATION_THEOREM_RECORD: AuthorityIdentityKind.RELATION_SUPERSESSION,
    RecordKind.RELATION_APPLICATION_RECORD: AuthorityIdentityKind.RELATION_SUPERSESSION,
    RecordKind.DOMAIN_THEOREM_RECORD: AuthorityIdentityKind.DOMAIN_SUPERSESSION,
    RecordKind.DOMAIN_APPLICATION_RECORD: AuthorityIdentityKind.DOMAIN_SUPERSESSION,
    RecordKind.CONTEXT_THEOREM_RECORD: AuthorityIdentityKind.CONTEXT_SUPERSESSION,
    RecordKind.CONTEXT_APPLICATION_RECORD: AuthorityIdentityKind.CONTEXT_SUPERSESSION,
}


@dataclass(frozen=True)
class SupersessionRecordV1:
    superseded_record_id: AuthorityIdentityV1
    replacement_record_id: AuthorityIdentityV1 | None
    superseded_record_kind: RecordKind
    replacement_record_kind: RecordKind | None
    reason_code: SupersessionReason
    source_evidence_refs: tuple[EvidenceRefV1, ...]
    review_event_ref: ReviewEventRefV1

    def __post_init__(self) -> None:
        expected = _record_identity_kind(self.superseded_record_kind)
        if self.superseded_record_id.kind is not expected:
            raise AuthorityContractError("superseded ID kind does not match record kind")
        if self.replacement_record_id is None:
            if self.replacement_record_kind is not None:
                raise AuthorityContractError("revocation replacement kind must be null")
            if self.reason_code is not SupersessionReason.AUTHORITY_REVOCATION:
                raise AuthorityContractError("null replacement requires authority_revocation")
        else:
            if self.replacement_record_kind is None:
                raise AuthorityContractError("replacement ID requires replacement kind")
            if self.replacement_record_kind is not self.superseded_record_kind:
                raise AuthorityContractError("supersession replacement kind must match")
            if self.replacement_record_id.kind is not _record_identity_kind(
                self.replacement_record_kind
            ):
                raise AuthorityContractError("replacement ID kind does not match record kind")
            if self.reason_code is SupersessionReason.AUTHORITY_REVOCATION:
                raise AuthorityContractError("authority_revocation requires a null replacement")
        if not self.source_evidence_refs:
            raise AuthorityContractError("supersession requires source evidence")
        _require_canonical_sequence(self.source_evidence_refs, "supersession evidence")

    def semantic_input(self) -> list[AuthorityValue]:
        replacement = (
            None if self.replacement_record_id is None else self.replacement_record_id.digest_bytes
        )
        return [
            _IDENTITY_SPECS[_SUPERSESSION_ID_KIND[self.superseded_record_kind]].input_schema_id,
            self.superseded_record_id.digest_bytes,
            replacement,
            self.superseded_record_kind.value,
            (None if self.replacement_record_kind is None else self.replacement_record_kind.value),
            self.reason_code.value,
            [ref.to_cbor() for ref in self.source_evidence_refs],
            self.review_event_ref.to_cbor(),
        ]

    def identity(self) -> AuthorityIdentityV1:
        return compute_authority_identity(
            _SUPERSESSION_ID_KIND[self.superseded_record_kind],
            self.semantic_input(),
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return self.semantic_input()


def _record_identity_kind(kind: RecordKind) -> AuthorityIdentityKind:
    mapping = {
        RecordKind.RELATION_THEOREM_RECORD: AuthorityIdentityKind.RELATION_THEOREM_RECORD,
        RecordKind.RELATION_APPLICATION_RECORD: AuthorityIdentityKind.RELATION_APPLICATION_RECORD,
        RecordKind.DOMAIN_THEOREM_RECORD: AuthorityIdentityKind.DOMAIN_THEOREM_RECORD,
        RecordKind.DOMAIN_APPLICATION_RECORD: AuthorityIdentityKind.DOMAIN_APPLICATION_RECORD,
        RecordKind.CONTEXT_THEOREM_RECORD: AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
        RecordKind.CONTEXT_APPLICATION_RECORD: AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD,
    }
    return mapping[kind]
