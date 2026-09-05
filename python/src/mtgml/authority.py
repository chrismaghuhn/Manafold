"""Rules-neutral M2.5.C authority contract and identity primitives.

This module owns the fixed V1 persistence shapes and the additive structural
V2/V3 context-application contract vocabulary needed by the authority
boundary. It does not resolve sources, validate Magic semantics, classify C
candidates, or derive interaction classes.
"""

from __future__ import annotations

import re
from collections.abc import Callable
from dataclasses import dataclass
from enum import Enum
from typing import Final, NoReturn, Protocol, TypeAlias, cast

from .persistence import (
    CANONICAL_CBOR_ID,
    DIGEST_ENVELOPE_ID,
    SHA256_ID,
    PersistenceValue,
    encode_canonical,
    encode_envelope,
    hash_envelope,
)

AUTHORITY_SCHEMA_V1: Final = "manafold.m2.5.c.interaction-review-authority.v1"
ACCEPTANCE_EVENT_SCHEMA_V1: Final = "manafold.m2.5.c.review-acceptance-event.v1"
REVIEWER_ROSTER_SCHEMA_V1: Final = "manafold.m2.5.c.reviewer-roster.v1"
ACCEPTANCE_CHECKLIST_V1: Final = "interaction-authority-review-checklist.v1"
SUPERSESSION_RECORD_SCHEMA_V1: Final = "manafold.m2.5.c.supersession-record.v1"
CONTEXT_AUTHORITY_SCHEMA_V2: Final = "manafold.m2.5.c.context-application-authority.v2"
CONTEXT_APPLICATION_INPUT_SCHEMA_V2: Final = "manafold.m2.5.c.context-application-input.v2"
CONTEXT_APPLICATION_RECORD_INPUT_SCHEMA_V2: Final = (
    "manafold.m2.5.c.context-application-record-input.v2"
)
CONTEXT_SUPERSESSION_INPUT_SCHEMA_V2: Final = (
    "manafold.m2.5.c.context-application-v2-supersession-input.v2"
)
CONTEXT_SUPERSESSION_RECORD_INPUT_SCHEMA_V2: Final = (
    "manafold.m2.5.c.context-application-supersession-record-input.v2"
)
ACCEPTANCE_SUBJECT_SCHEMA_V3: Final = "manafold.m2.5.c.acceptance-subject-payload.v3"
ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V3: Final = "manafold.m2.5.c.acceptance-subject-payload-input.v3"
ACCEPTANCE_EVENT_SCHEMA_V3: Final = "manafold.m2.5.c.review-acceptance-event.v3"
ACCEPTANCE_EVENT_INPUT_SCHEMA_V3: Final = "manafold.m2.5.c.review-acceptance-event-input.v3"
ACCEPTANCE_CHECKLIST_V2: Final = "interaction-authority-review-checklist.v2"

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
_EXPECTED_STATIC_PATH_BY_ROLE: Final = {
    "declared_model": "sources/m2_5/closures/C/declared_interaction_model.v2.json",
    "b2_catalog": "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
    "b2_classifications": "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
    "b2_closure": "sources/m2_5/closures/B2/classification_closure.v1.json",
    "b1_final_citations": "sources/m2_5/closures/B1/official_authority_citations.v3.json",
    "b1_final_closure": ("sources/m2_5/closures/B1/official_authority_citation_closure.v2.json"),
    "candidate_universe": "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
}
_CONTEXT_AUTHORITY_STATIC_BINDING_REGISTRY_V2: Final = {
    "base_authority_v1": (
        "sources/m2_5/authorities/interaction_review_authority.v1.json",
        "manafold.m2.5.c.interaction-review-authority.v1",
    ),
    "declared_model": (
        "sources/m2_5/closures/C/declared_interaction_model.v2.json",
        "manafold.m2.5.c.declared-interaction-model.v2",
    ),
    "candidate_universe": (
        "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
        "manafold.m2.5.c.interaction-candidate-universe.v2",
    ),
    "rev3_candidate_census": ("derived/Pair_Interaction_Census_REV3.csv", None),
    "rev3_pair_aggregates": ("derived/Pair_Requirement_Aggregates_REV3.json", None),
    "rev3_card_requirement_map": ("derived/Card_Requirement_Map_REV3.csv", None),
    "rev3_deck_row_source_resolution": ("inputs/deck_row_source_resolution_REV3.csv", None),
    "rev3_osi_source_records": ("source/raw/oracle_cards_selected_REV3.jsonl", None),
    "rev3_source_index": ("source/raw/source_record_index_REV3.csv", None),
    "b2_catalog": (
        "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
        "manafold.m2.5.b2.requirement-family-catalog.v1",
    ),
    "b2_classifications": (
        "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
        "manafold.m2.5.b2.card-semantic-classifications.v1",
    ),
    "b2_closure": (
        "sources/m2_5/closures/B2/classification_closure.v1.json",
        "manafold.m2.5.b2.classification-closure.v1",
    ),
    "b1_final_citations": (
        "sources/m2_5/closures/B1/official_authority_citations.v3.json",
        "manafold.m2.5.b1.official-authority-citations.v3",
    ),
    "b1_final_closure": (
        "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json",
        "manafold.m2.5.b1.official-authority-citation-closure.v2",
    ),
    "host_binding_authority_v2": (
        "sources/m2_5/authorities/interaction_review_authority.v2.json",
        "manafold.m2.5.c.interaction-review-authority.v2",
    ),
}
_CONTEXT_AUTHORITY_LEAF_BINDING_REGISTRY_V2: Final = {
    "reviewer_roster_leaf": (
        r"sources/m2_5/authorities/reviewer_rosters/v1/[0-9a-f]{64}\.json",
        REVIEWER_ROSTER_SCHEMA_V1,
    ),
    "acceptance_event_leaf_v3": (
        r"sources/m2_5/authorities/review_acceptance_events/v3/[0-9a-f]{64}\.json",
        ACCEPTANCE_EVENT_SCHEMA_V3,
    ),
    "host_binding_claim_record": (
        r"sources/m2_5/authorities/cross_deck_host_binding_claims/v1/[0-9a-f]{64}\.json",
        "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1",
    ),
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

AuthorityValue: TypeAlias = PersistenceValue
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
    CONTEXT_APPLICATION_V2 = "context_application_v2"
    CONTEXT_APPLICATION_RECORD_V2 = "context_application_record_v2"
    CONTEXT_SUPERSESSION_V2 = "context_supersession_v2"
    CONTEXT_SUPERSESSION_RECORD_V2 = "context_supersession_record_v2"
    ACCEPTANCE_SUBJECT_V3 = "acceptance_subject_v3"
    REVIEW_ACCEPTANCE_EVENT_V3 = "review_acceptance_event_v3"


class ContextBridgeRelationV2(str, Enum):
    EXACT_MATCH = "exact_match"
    REVIEWED_DIVERGENCE = "reviewed_divergence"


class AcceptanceSubjectKindV3(str, Enum):
    CONTEXT_APPLICATION_V2_RECORD = "context_application_v2_record"
    CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD = "context_application_v2_supersession_record"


class ContextAuthorityArtifactRoleV2(str, Enum):
    BASE_AUTHORITY_V1 = "base_authority_v1"
    DECLARED_MODEL = "declared_model"
    CANDIDATE_UNIVERSE = "candidate_universe"
    REV3_CANDIDATE_CENSUS = "rev3_candidate_census"
    REV3_PAIR_AGGREGATES = "rev3_pair_aggregates"
    REV3_CARD_REQUIREMENT_MAP = "rev3_card_requirement_map"
    REV3_DECK_ROW_SOURCE_RESOLUTION = "rev3_deck_row_source_resolution"
    REV3_OSI_SOURCE_RECORDS = "rev3_osi_source_records"
    REV3_SOURCE_INDEX = "rev3_source_index"
    B2_CATALOG = "b2_catalog"
    B2_CLASSIFICATIONS = "b2_classifications"
    B2_CLOSURE = "b2_closure"
    B1_FINAL_CITATIONS = "b1_final_citations"
    B1_FINAL_CLOSURE = "b1_final_closure"
    REVIEWER_ROSTER_LEAF = "reviewer_roster_leaf"
    ACCEPTANCE_EVENT_LEAF_V3 = "acceptance_event_leaf_v3"
    HOST_BINDING_AUTHORITY_V2 = "host_binding_authority_v2"
    HOST_BINDING_CLAIM_RECORD = "host_binding_claim_record"


CONTEXT_AUTHORITY_SOURCE_ROLES_V2: Final = tuple(
    role.value for role in ContextAuthorityArtifactRoleV2
)


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
    AuthorityIdentityKind.CONTEXT_APPLICATION_V2: _IdentitySpec(
        "cpa.v2/",
        "manafold.m2.5.c.context-application.v2",
        CONTEXT_APPLICATION_INPUT_SCHEMA_V2,
    ),
    AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2: _IdentitySpec(
        "cpar.v2/",
        "manafold.m2.5.c.context-application-record.v2",
        CONTEXT_APPLICATION_RECORD_INPUT_SCHEMA_V2,
    ),
    AuthorityIdentityKind.CONTEXT_SUPERSESSION_V2: _IdentitySpec(
        "cps.v2/",
        "manafold.m2.5.c.context-application-supersession.v2",
        CONTEXT_SUPERSESSION_INPUT_SCHEMA_V2,
    ),
    AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V2: _IdentitySpec(
        "cpsr.v2/",
        "manafold.m2.5.c.context-application-supersession-record.v2",
        CONTEXT_SUPERSESSION_RECORD_INPUT_SCHEMA_V2,
    ),
    AuthorityIdentityKind.ACCEPTANCE_SUBJECT_V3: _IdentitySpec(
        "asp.v3/",
        ACCEPTANCE_SUBJECT_SCHEMA_V3,
        ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V3,
    ),
    AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT_V3: _IdentitySpec(
        "ae.v3/",
        ACCEPTANCE_EVENT_SCHEMA_V3,
        ACCEPTANCE_EVENT_INPUT_SCHEMA_V3,
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
    AuthorityIdentityKind.CONTEXT_APPLICATION_V2: 3,
    AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2: 3,
    AuthorityIdentityKind.CONTEXT_SUPERSESSION_V2: 7,
    AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V2: 3,
    AuthorityIdentityKind.ACCEPTANCE_SUBJECT_V3: 3,
    AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT_V3: 10,
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
    _validate_kind_payload(kind, fields)
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


def _locator_to_wire(locator: LocatorV1) -> dict[str, object]:
    kind, payload = locator
    wire: dict[str, object] = {"kind": kind}
    if payload is not None:
        wire["value"] = payload
    return wire


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
        expected_path = _EXPECTED_STATIC_PATH_BY_ROLE.get(self.artifact_role)
        if expected_path is not None and self.path != expected_path:
            raise AuthorityContractError("source binding path does not match its artifact role")
        if (
            self.artifact_role == "acceptance_event_leaf"
            and re.fullmatch(
                r"sources/m2_5/authorities/review_acceptance_events/v1/[0-9a-f]{64}\.json",
                self.path,
            )
            is None
        ):
            raise AuthorityContractError("acceptance event leaf path is not a V1 leaf path")
        if (
            self.artifact_role == "reviewer_roster_leaf"
            and re.fullmatch(
                r"sources/m2_5/authorities/reviewer_rosters/v1/[0-9a-f]{64}\.json",
                self.path,
            )
            is None
        ):
            raise AuthorityContractError("reviewer roster leaf path is not a V1 leaf path")
        _require_digest_bytes(self.raw_sha256, "source binding digest")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.artifact_role, self.path, self.schema_or_null, self.raw_sha256]

    def to_wire(self) -> dict[str, object]:
        return {
            "artifact_role": self.artifact_role,
            "path": self.path,
            "schema_or_null": self.schema_or_null,
            "raw_sha256": self.raw_sha256.hex(),
        }


@dataclass(frozen=True)
class B2FamilyRefV1:
    family_id: str
    lifecycle: str
    assignment_role: str

    def __post_init__(self) -> None:
        if not self.family_id:
            raise AuthorityContractError("B2 family ID must be non-empty")
        if self.lifecycle not in _B2_LIFECYCLES:
            raise AuthorityContractError("B2 family lifecycle is not closed in V1")
        if self.assignment_role not in _B2_ASSIGNMENT_ROLES:
            raise AuthorityContractError("B2 family assignment role is not closed in V1")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.family_id, self.lifecycle, self.assignment_role]


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

    def to_wire(self) -> dict[str, object]:
        return {
            "authority_kind": self.authority_kind,
            "path": self.path,
            "locator": _locator_to_wire(self.locator),
            "raw_sha256": self.raw_sha256.hex(),
        }


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

    def to_wire(self) -> dict[str, object]:
        return {
            "path": self.path,
            "raw_sha256": self.raw_sha256.hex(),
            "locator": _locator_to_wire(self.locator),
        }


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

    def to_wire(self) -> dict[str, object]:
        return {
            "path": self.path,
            "raw_sha256": self.raw_sha256.hex(),
            "locator": {"kind": "event_id", "value": self.event_id},
        }


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

    def to_wire(self) -> dict[str, object]:
        return {
            "path": self.path,
            "schema": self.schema,
            "raw_sha256": self.raw_sha256.hex(),
        }


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

    def to_wire(self) -> dict[str, object]:
        return {"reviewer_id": self.reviewer_id, "roles": list(self.roles)}


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
class ReviewAcceptanceEventInputV1:
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


@dataclass(frozen=True)
class ReviewAcceptanceEventLeafV1:
    event_id: AuthorityIdentityV1
    subject_kind: AcceptanceSubjectKind
    subject_payload_digest: bytes
    decision: str
    reviewer_roster_ref: ReviewerRosterRefV1
    reviewer_role_bindings: tuple[ReviewerRoleBindingV1, ...]
    review_mode: ReviewMode
    checklist_id: str
    source_binding_digests: tuple[SourceBindingDigestV1, ...]
    review_evidence_refs: tuple[AcceptanceEvidenceRefV1, ...]

    @classmethod
    def from_input(cls, event: ReviewAcceptanceEventInputV1) -> ReviewAcceptanceEventLeafV1:
        return cls(
            event_id=event.identity(),
            subject_kind=event.subject_kind,
            subject_payload_digest=event.subject_payload_digest,
            decision="human_accepted",
            reviewer_roster_ref=event.reviewer_roster_ref,
            reviewer_role_bindings=event.reviewer_role_bindings,
            review_mode=event.review_mode,
            checklist_id=ACCEPTANCE_CHECKLIST_V1,
            source_binding_digests=event.source_binding_digests,
            review_evidence_refs=event.review_evidence_refs,
        )

    def __post_init__(self) -> None:
        if self.event_id.kind is not AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT:
            raise AuthorityContractError("acceptance leaf ID has the wrong identity kind")
        if self.decision != "human_accepted":
            raise AuthorityContractError("acceptance leaf decision is not human_accepted")
        if self.checklist_id != ACCEPTANCE_CHECKLIST_V1:
            raise AuthorityContractError("acceptance leaf checklist is not the V1 contract")
        if self.as_input().identity() != self.event_id:
            raise AuthorityContractError("acceptance leaf event ID does not match its input")

    def as_input(self) -> ReviewAcceptanceEventInputV1:
        return ReviewAcceptanceEventInputV1(
            subject_kind=self.subject_kind,
            subject_payload_digest=self.subject_payload_digest,
            reviewer_roster_ref=self.reviewer_roster_ref,
            reviewer_role_bindings=self.reviewer_role_bindings,
            review_mode=self.review_mode,
            source_binding_digests=self.source_binding_digests,
            review_evidence_refs=self.review_evidence_refs,
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "event_id": self.event_id.as_text(),
            "schema": ACCEPTANCE_EVENT_SCHEMA_V1,
            "subject_kind": self.subject_kind.value,
            "subject_payload_digest": self.subject_payload_digest.hex(),
            "decision": self.decision,
            "reviewer_roster_ref": self.reviewer_roster_ref.to_wire(),
            "reviewer_role_bindings": [
                binding.to_wire() for binding in self.reviewer_role_bindings
            ],
            "review_mode": self.review_mode.value,
            "checklist_id": self.checklist_id,
            "source_binding_digests": [
                binding.to_wire() for binding in self.source_binding_digests
            ],
            "review_evidence_refs": [evidence.to_wire() for evidence in self.review_evidence_refs],
        }

    def to_cbor(self) -> list[AuthorityValue]:
        return self.as_input().semantic_input()


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


_RELATION_CHANNELS: Final = (
    "participant_boundary",
    "event_or_effect_causality",
    "target_or_choice",
    "zone_or_object_identity",
    "control_or_ownership",
    "replacement_or_layer",
    "trigger_or_lki",
    "information_or_visibility",
    "ordering_or_temporal",
    "decision_actor",
    "format_and_declared_scope",
)
_ARITIES: Final = ("unary", "binary", "higher_order")
_DIRECTIONALITIES: Final = ("directed", "none", "symmetric")
_HOST_RELATIONSHIPS: Final = ("cross_host", "not_applicable", "same_host")
_SCOPES: Final = ("cross_deck", "intra_deck", "unary_or_higher_order")
_RELATIONS: Final = (
    "declared_card_trigger",
    "directional_binary",
    "reviewed_higher_order",
    "unordered_binary",
)
_OPERATIONS: Final = (
    "reads",
    "changes_characteristic",
    "changes_eligibility",
    "changes_target_legality",
    "changes_controller",
    "changes_ownership",
    "changes_zone",
    "creates_object",
    "copies_value",
    "replaces_event",
    "triggers_ability",
    "orders_event",
    "supplies_choice",
)
_PROOF_KINDS: Final = (
    "positive_interaction",
    "positive_separation",
    "model_bound_scope",
)
_SCOPE_REASONS: Final = (
    "unbounded_n_way_not_representable",
    "undeclared_participant_kind",
    "undeclared_relation_shape",
    "undeclared_outcome_surface",
)
_REVIEW_DOMAINS: Final = (
    "triggers_and_lki",
    "replacement_layers_and_dependency",
    "copy_and_token_creation",
    "target_legality_protection_and_identity",
    "control_and_ownership",
    "commander_and_format",
    "hidden_information_and_visibility",
    "ordering_and_temporal_dependencies",
    "source_versus_affected_identity",
    "controller_owner_and_decision_actor",
    "higher_order_interactions",
)
_APPLICABILITY: Final = ("applicable", "not_applicable")
_TERMINAL_DISPOSITIONS: Final = (
    "required_interaction",
    "not_an_interaction_with_proof",
    "out_of_declared_scope_with_reason",
)
_PRECONDITION_KINDS: Final = (
    "candidate_relation_shape",
    "participant_binding",
    "b2_boundary",
    "source_context",
    "temporal_semantic",
    "class_projection",
)
_SEPARATION_KINDS: Final = (
    "boundary_disjointness",
    "closed_channel_exclusion",
    "independent_effect_separation",
)
_REQUIRED_CONCLUSIONS: Final = ("separated", "not_relevant")
_RECORD_KINDS: Final = tuple(kind.value for kind in RecordKind)
_SUBJECT_KINDS: Final = tuple(kind.value for kind in AcceptanceSubjectKind)
_SLOT_KINDS: Final = ("context_dimension", "temporal_semantic")
_PARTICIPANT_ROLES: Final = (
    "affected",
    "controller",
    "copied_source",
    "copy_result",
    "decision_actor",
    "destination_zone",
    "origin_zone",
    "ordered_participant",
    "owner",
    "replacement_actor",
    "source",
    "target",
    "trigger_source",
)
_PARTICIPANT_KINDS: Final = (
    "ability",
    "card",
    "copiable_value",
    "deck",
    "effect",
    "event",
    "object",
    "permanent",
    "player",
    "requirement_family",
    "source_instance",
    "spell",
    "token",
    "zone",
)
_CONTEXT_DIMENSIONS: Final = (
    "zone",
    "visibility",
    "timing",
    "temporal_order",
    "source_affected_relation",
    "control_ownership_relation",
    "replacement_layer_relation",
    "trigger_lki_relation",
    "information_relation",
    "decision_actor_relation",
)
_TEMPORAL_SEMANTICS: Final = (
    "trigger_order",
    "dependency_order",
    "duration",
    "replacement_order",
)
_CONTEXT_VALUES: Final[dict[str, tuple[str, ...]]] = {
    "zone": (
        "battlefield",
        "command_zone",
        "exile",
        "graveyard",
        "hand",
        "library",
        "outside_game",
        "stack",
        "zone_agnostic",
        "not_applicable",
    ),
    "visibility": (
        "controller_only",
        "hidden_to_actor",
        "identity_hidden",
        "not_applicable",
        "owner_only",
        "private",
        "public",
    ),
    "timing": (
        "activation_time",
        "cast_time",
        "combat_time",
        "continuous_effect",
        "not_applicable",
        "resolution_time",
        "state_based_check",
        "trigger_time",
        "turn_boundary",
        "zone_change_time",
    ),
    "temporal_order": (
        "after",
        "before",
        "during",
        "not_applicable",
        "sequential",
        "simultaneous",
        "until",
        "while",
    ),
    "source_affected_relation": (
        "both_affected",
        "no_effect_relation",
        "not_applicable",
        "source_affected",
        "source_affects_other",
    ),
    "control_ownership_relation": (
        "control_changes",
        "cross_controller",
        "cross_owner",
        "not_applicable",
        "ownership_changes",
        "same_controller",
        "same_owner",
    ),
    "replacement_layer_relation": (
        "copy_layer",
        "control_layer",
        "layer_dependency",
        "no_replacement_or_layer",
        "not_applicable",
        "pt_layer",
        "replacement_" + "effect",
        "type_layer",
        "zone_change_replacement",
    ),
    "trigger_lki_relation": (
        "intervening_if",
        "last_known_information",
        "no_trigger_lki",
        "not_applicable",
        "trigger_condition",
        "triggered_event",
    ),
    "information_relation": (
        "hidden_identity",
        "known_to_controller",
        "known_to_owner",
        "no_information_dependency",
        "not_applicable",
        "private_look",
        "public_identity",
        "random_unknown",
    ),
    "decision_actor_relation": (
        "active_player",
        "controller",
        "no_decision",
        "not_applicable",
        "opponent",
        "owner",
        "rules_forced",
        "target_player",
    ),
}
_TEMPORAL_VALUES: Final[dict[str, tuple[str, ...]]] = {
    "trigger_order": ("deferred", "immediate", "no_temporal_dependency", "not_applicable"),
    "dependency_order": (
        "dependency_ordered",
        "no_temporal_dependency",
        "not_applicable",
    ),
    "duration": ("duration_limited", "indefinite", "not_applicable", "until_event"),
    "replacement_order": (
        "after_effect",
        "before_effect",
        "no_temporal_dependency",
        "not_applicable",
        "same_event",
    ),
}
_B2_LIFECYCLES: Final = ("active", "active_unassigned")
_B2_ASSIGNMENT_ROLES: Final = ("primary", "supporting")
_CLASS_PROJECTION_POSITIONS: Final = (
    "arity",
    "directionality",
    "participant_roles",
    "host_relationship",
    "context_dimensions",
    "temporal_semantics",
    "b2_family_refs",
    "b2_boundary_refs",
    "b1_final_citation_refs",
)
_DOMAIN_BOUNDARY_FIELDS: Final = (
    "includes",
    "excludes",
    "objects",
    "action_or_event",
    "timing",
    "zone_visibility",
    "eligibility_condition_duration",
    "targets_choices",
    "ownership_control",
    "numeric_scaling_counters",
    "information_identity_effect",
    "rule_dependency",
)
_CONTEXT_SLOT_SEQUENCE: Final = tuple(
    [("context_dimension", name) for name in _CONTEXT_DIMENSIONS]
    + [("temporal_semantic", name) for name in _TEMPORAL_SEMANTICS]
)


def _fail(message: str) -> NoReturn:
    raise AuthorityContractError(message)


def _array(value: object, label: str, length: int | None = None) -> list[object]:
    if not isinstance(value, list):
        _fail(f"{label} must be an array")
    result = cast(list[object], value)
    if length is not None and len(result) != length:
        _fail(f"{label} must contain exactly {length} fields")
    return result


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        _fail(f"{label} must be non-empty text")
    return value


def _any_text(value: object, label: str) -> str:
    if not isinstance(value, str):
        _fail(f"{label} must be text")
    return value


def _uint32(value: object, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**32 - 1:
        _fail(f"{label} must be a u32")
    return value


def _bytes32(value: object, label: str) -> bytes:
    return _require_digest_bytes(value, label)


def _enum(value: object, allowed: tuple[str, ...], label: str) -> str:
    value = _any_text(value, label)
    if value not in allowed:
        _fail(f"{label} is not a closed V1 value")
    return value


def _canonical_array(
    values: list[object],
    validator: Callable[[object], None],
    label: str,
) -> None:
    encoded: list[bytes] = []
    for value in values:
        validator(value)
        encoded.append(encode_canonical(cast(PersistenceValue, value)))
    if encoded != sorted(encoded):
        _fail(f"{label} must be in canonical order")
    if len(set(encoded)) != len(encoded):
        _fail(f"{label} must be duplicate-free")


def _ordered_enum_array(
    values: list[object],
    allowed: tuple[str, ...],
    label: str,
) -> None:
    seen = [_enum(value, allowed, label) for value in values]
    if len(set(seen)) != len(seen):
        _fail(f"{label} must be duplicate-free")
    if seen != [value for value in allowed if value in seen]:
        _fail(f"{label} must preserve the closed vocabulary order")


def _exact_ordered_enum_array(
    value: object,
    allowed: tuple[str, ...],
    label: str,
) -> None:
    values = _array(value, label, len(allowed))
    actual = [_enum(item, allowed, label) for item in values]
    if actual != list(allowed):
        _fail(f"{label} must contain the complete closed vocabulary in order")


def _validate_slot_value_vector(
    value: object,
    slot_names: tuple[str, ...],
    vocabularies: dict[str, tuple[str, ...]],
    label: str,
) -> None:
    values = _array(value, label, len(slot_names))
    for name, observed in zip(slot_names, values, strict=True):
        _enum(observed, vocabularies[name], f"{label}.{name}")


def _validate_cbor_value(value: object, label: str = "value") -> None:
    if value is None or isinstance(value, bool | int | bytes | str):
        return
    if isinstance(value, list):
        for index, child in enumerate(value):
            _validate_cbor_value(child, f"{label}[{index}]")
        return
    _fail(f"{label} uses a forbidden value form")


def _validate_locator_array(value: object, *, acceptance: bool) -> None:
    locator = _array(value, "locator", 2)
    kind = _any_text(locator[0], "locator kind")
    payload = locator[1]
    if kind == "whole_artifact":
        if payload is not None:
            _fail("whole_artifact locator payload must be null")
    elif kind == "json_pointer":
        _require_locator(("json_pointer", payload), acceptance=acceptance)
    elif kind == "archive_member":
        _require_locator(("archive_member", payload), acceptance=acceptance)
    elif kind == "event_id":
        _require_locator(("event_id", payload), acceptance=acceptance)
    else:
        _fail("locator variant is not closed in V1")


def _validate_digest_reference(value: object, label: str = "digest reference") -> None:
    reference = _array(value, label, 6)
    _any_text(reference[0], f"{label} envelope")
    _any_text(reference[1], f"{label} algorithm")
    _any_text(reference[2], f"{label} domain")
    _any_text(reference[3], f"{label} codec")
    _any_text(reference[4], f"{label} schema")
    _bytes32(reference[5], f"{label} digest")


def _validate_source_binding_array(value: object) -> None:
    binding = _array(value, "source binding", 4)
    role = _any_text(binding[0], "source binding artifact role")
    path = _any_text(binding[1], "source binding path")
    schema = binding[2]
    digest = _bytes32(binding[3], "source binding digest")
    if not isinstance(schema, str | None):
        _fail("source binding schema_or_null must be text or null")
    SourceBindingDigestV1(role, path, schema, digest)


def _validate_evidence_ref_array(value: object) -> None:
    reference = _array(value, "evidence reference", 4)
    authority_kind = _any_text(reference[0], "evidence authority kind")
    path = _any_text(reference[1], "evidence path")
    locator = _array(reference[2], "evidence locator", 2)
    _validate_locator_array(locator, acceptance=False)
    _bytes32(reference[3], "evidence digest")
    EvidenceRefV1(
        authority_kind,
        path,
        (cast(str, locator[0]), cast(str | int | None, locator[1])),
        cast(bytes, reference[3]),
    )


def _validate_evidence_refs(value: object, label: str = "evidence references") -> None:
    refs = _array(value, label)
    _canonical_array(refs, _validate_evidence_ref_array, label)


def _validate_nonempty_evidence_refs(
    value: object,
    label: str = "evidence references",
) -> None:
    refs = _array(value, label)
    if not refs:
        _fail(f"{label} must be non-empty")
    _canonical_array(refs, _validate_evidence_ref_array, label)


def _validate_acceptance_evidence_ref_array(value: object) -> None:
    reference = _array(value, "acceptance evidence reference", 3)
    path = _any_text(reference[0], "acceptance evidence path")
    digest = _bytes32(reference[1], "acceptance evidence digest")
    locator = _array(reference[2], "acceptance evidence locator", 2)
    _validate_locator_array(locator, acceptance=True)
    AcceptanceEvidenceRefV1(
        path,
        digest,
        (cast(str, locator[0]), cast(str | int | None, locator[1])),
    )


def _validate_acceptance_evidence_refs(value: object) -> None:
    refs = _array(value, "review evidence references")
    if not refs:
        _fail("review evidence references must be non-empty")
    _canonical_array(refs, _validate_acceptance_evidence_ref_array, "review evidence references")


def _validate_review_event_ref_array(value: object) -> None:
    reference = _array(value, "review event reference", 3)
    path = _any_text(reference[0], "review event path")
    digest = _bytes32(reference[1], "review event digest")
    locator = _array(reference[2], "review event locator", 2)
    if locator[0] != "event_id":
        _fail("review event locator must be event_id")
    event_id = _any_text(locator[1], "review event ID")
    ReviewEventRefV1(path, digest, event_id)


def _validate_roster_ref_array(value: object) -> None:
    reference = _array(value, "reviewer roster reference", 3)
    ReviewerRosterRefV1(
        _any_text(reference[0], "reviewer roster path"),
        _any_text(reference[1], "reviewer roster schema"),
        _bytes32(reference[2], "reviewer roster digest"),
    )


def _validate_roles(value: object, label: str = "roles") -> None:
    roles = _array(value, label)
    values = [_enum(role, REVIEWER_ROLES, label) for role in roles]
    if values != sorted(values) or len(set(values)) != len(values):
        _fail(f"{label} must be sorted and duplicate-free")


def _validate_role_binding(value: object) -> None:
    binding = _array(value, "reviewer role binding", 2)
    _text(binding[0], "reviewer ID")
    _validate_roles(binding[1])


def _validate_participant_role(value: object, label: str = "participant role") -> None:
    role = _array(value, label, 4)
    _uint32(role[0], f"{label} position")
    _enum(role[1], _PARTICIPANT_ROLES, f"{label} role")
    _enum(role[2], _PARTICIPANT_KINDS, f"{label} participant kind")
    _text(role[3], f"{label} semantic reference")


def _validate_participant_roles(
    value: object,
    directionality: str | None = None,
) -> None:
    roles = _array(value, "participant roles")
    if not roles:
        _fail("participant roles must be non-empty")
    keys: list[bytes] = []
    for index, role in enumerate(roles):
        _validate_participant_role(role)
        fields = cast(list[object], role)
        if fields[0] != index:
            _fail("participant role positions must be the ordered 0..n-1 sequence")
        if directionality == "symmetric":
            keys.append(encode_canonical(cast(PersistenceValue, [fields[2], fields[3], fields[1]])))
    if directionality == "symmetric":
        if keys != sorted(keys):
            _fail("symmetric participant roles must be in canonical order")
        if len(set(keys)) != len(keys):
            _fail("symmetric participant roles must be duplicate-free")


def _validate_b2_boundary_ref(value: object) -> None:
    boundary = _array(value, "B2 boundary reference", 2)
    _text(boundary[0], "B2 family ID")
    _text(boundary[1], "B2 semantic definition")


def _validate_b2_boundary_refs(value: object) -> None:
    refs = _array(value, "B2 boundary references")
    _canonical_array(refs, _validate_b2_boundary_ref, "B2 boundary references")


def _validate_b2_family_ref(value: object) -> None:
    family = _array(value, "B2 family reference", 3)
    _text(family[0], "B2 family ID")
    _enum(family[1], _B2_LIFECYCLES, "B2 family lifecycle")
    _enum(family[2], _B2_ASSIGNMENT_ROLES, "B2 family assignment role")


def _validate_b2_family_refs(value: object) -> None:
    refs = _array(value, "B2 family references")
    _canonical_array(refs, _validate_b2_family_ref, "B2 family references")


def _validate_b1_citation_ref(value: object) -> None:
    citation = _array(value, "B1.Final citation reference", 2)
    _text(citation[0], "B1 authority ID")
    _text(citation[1], "B1 citation ID")


def _validate_b1_citation_refs(value: object) -> None:
    refs = _array(value, "B1.Final citation references")
    _canonical_array(refs, _validate_b1_citation_ref, "B1.Final citation references")


def _validate_context_slot_value(
    slot_kind: object,
    slot_name: object,
    observed_value: object,
) -> None:
    kind = _enum(slot_kind, _SLOT_KINDS, "context slot kind")
    if kind == "context_dimension":
        name = _enum(slot_name, _CONTEXT_DIMENSIONS, "context slot name")
        _enum(observed_value, _CONTEXT_VALUES[name], "context slot value")
    else:
        name = _enum(slot_name, _TEMPORAL_SEMANTICS, "temporal slot name")
        _enum(observed_value, _TEMPORAL_VALUES[name], "temporal slot value")


def _validate_context_slot_attestation(
    value: object,
    expected: tuple[str, str] | None = None,
) -> None:
    fields = _array(value, "context slot attestation", 5)
    _validate_context_slot_value(fields[0], fields[1], fields[2])
    if expected is not None and (fields[0], fields[1]) != expected:
        _fail("context slot attestations must use the fixed slot order")
    _validate_nonempty_evidence_refs(fields[3], "context slot evidence")
    _text(fields[4], "context slot rationale")


def _validate_model_boundary_locator(value: object) -> None:
    locator = _array(value, "model boundary locator", 2)
    if locator[0] == "coverage_scope":
        if locator[1] is not None:
            _fail("coverage_scope locator payload must be null")
    elif locator[0] == "excluded_claim":
        _uint32(locator[1], "excluded claim index")
    else:
        _fail("model boundary locator variant is not closed")


def _validate_positive_boundary_fact(value: object) -> None:
    tagged = _array(value, "positive boundary fact", 2)
    kind = _any_text(tagged[0], "positive boundary fact kind")
    payload = _array(tagged[1], "positive boundary fact payload")
    if kind == "b2_boundary":
        if len(payload) != 4:
            _fail("B2 positive boundary fact must contain four fields")
        _text(payload[0], "B2 positive boundary family ID")
        _enum(payload[1], _B2_LIFECYCLES, "B2 positive boundary lifecycle")
        _enum(payload[2], _B2_ASSIGNMENT_ROLES, "B2 positive boundary assignment role")
        _text(payload[3], "B2 positive boundary definition")
    elif kind in {"rev3_locator", "b2_locator"}:
        if len(payload) != 3:
            _fail("source positive boundary fact must contain three fields")
        _text(payload[0], "source positive boundary path")
        _bytes32(payload[1], "source positive boundary digest")
        _validate_locator_array(payload[2], acceptance=False)
    elif kind == "b1_citation":
        _validate_b1_citation_ref(payload)
    elif kind == "context_slot":
        if len(payload) != 3:
            _fail("context positive boundary fact must contain three fields")
        _validate_context_slot_value(payload[0], payload[1], payload[2])
    elif kind == "model_boundary":
        if len(payload) != 3:
            _fail("model positive boundary fact must contain three fields")
        _text(payload[0], "boundary model ID")
        _text(payload[1], "boundary model version")
        _validate_model_boundary_locator(payload[2])
    else:
        _fail("positive boundary fact variant is not closed")


def _validate_positive_boundary_facts(value: object, label: str) -> None:
    facts = _array(value, label)
    if not facts:
        _fail(f"{label} must be non-empty")
    _canonical_array(facts, _validate_positive_boundary_fact, label)


def _validate_class_projection(value: object) -> None:
    projection = _array(value, "class projection", 9)
    _enum(projection[0], _ARITIES, "class arity")
    directionality = _enum(projection[1], _DIRECTIONALITIES, "class directionality")
    _validate_participant_roles(projection[2], directionality)
    _enum(projection[3], _HOST_RELATIONSHIPS, "class host relationship")
    _validate_slot_value_vector(
        projection[4], _CONTEXT_DIMENSIONS, _CONTEXT_VALUES, "context dimension values"
    )
    _validate_slot_value_vector(
        projection[5], _TEMPORAL_SEMANTICS, _TEMPORAL_VALUES, "temporal semantic values"
    )
    _validate_b2_family_refs(projection[6])
    _validate_b2_boundary_refs(projection[7])
    _validate_b1_citation_refs(projection[8])


def _validate_candidate_shape(value: object) -> None:
    shape = _array(value, "candidate shape", 5)
    _enum(shape[0], _SCOPES, "candidate scope")
    _enum(shape[1], _RELATIONS, "candidate relation")
    _enum(shape[2], _ARITIES, "candidate arity")
    _enum(shape[3], _DIRECTIONALITIES, "candidate directionality")
    _uint32(shape[4], "participant count")


def _validate_model_boundary_ref(value: object) -> None:
    reference = _array(value, "model boundary reference", 4)
    _text(reference[0], "model boundary path")
    if reference[1] != "manafold.m2.5.c.declared-interaction-model.v2":
        _fail("model boundary schema is not the declared model V2 contract")
    _bytes32(reference[2], "model boundary digest")
    _validate_model_boundary_locator(reference[3])


def _validate_precondition_payload(kind: str, payload: object) -> None:
    values = _array(payload, f"{kind} precondition payload")
    if kind == "candidate_relation_shape":
        if len(values) != 4:
            _fail("candidate relation shape precondition must contain four fields")
        _enum(values[0], _SCOPES, "candidate relation scope")
        _enum(values[1], _RELATIONS, "candidate relation")
        _enum(values[2], _DIRECTIONALITIES, "candidate relation directionality")
        _enum(values[3], _HOST_RELATIONSHIPS, "candidate relation host relationship")
    elif kind == "participant_binding":
        if len(values) != 4:
            _fail("participant binding precondition must contain four fields")
        _uint32(values[0], "participant binding position")
        _enum(values[1], _PARTICIPANT_ROLES, "participant binding role")
        _enum(values[2], _PARTICIPANT_KINDS, "participant binding participant kind")
        _text(values[3], "participant binding semantic reference")
    elif kind == "b2_boundary":
        if len(values) != 4:
            _fail("B2 boundary precondition must contain four fields")
        _text(values[0], "B2 family ID")
        _enum(values[1], _B2_LIFECYCLES, "B2 lifecycle")
        _enum(values[2], _B2_ASSIGNMENT_ROLES, "B2 assignment role")
        _text(values[3], "B2 definition")
    elif kind in {"source_context", "temporal_semantic"}:
        if len(values) != 2:
            _fail(f"{kind} precondition must contain two fields")
        if kind == "source_context":
            dimension = _enum(values[0], _CONTEXT_DIMENSIONS, "source context dimension")
            _enum(values[1], _CONTEXT_VALUES[dimension], "source context value")
        else:
            dimension = _enum(values[0], _TEMPORAL_SEMANTICS, "temporal semantic dimension")
            _enum(values[1], _TEMPORAL_VALUES[dimension], "temporal semantic value")
    elif kind == "class_projection":
        _validate_class_projection(values)


def _validate_preconditions(value: object) -> None:
    preconditions = _array(value, "preconditions")
    seen: set[str] = set()
    ordering_keys: list[bytes] = []
    for precondition in preconditions:
        fields = _array(precondition, "precondition", 2)
        precondition_id = _text(fields[0], "precondition ID")
        if precondition_id in seen:
            _fail("precondition IDs must be unique")
        seen.add(precondition_id)
        ordering_keys.append(encode_canonical(precondition_id))
        tagged = _array(fields[1], "precondition tagged payload", 2)
        kind = _enum(tagged[0], _PRECONDITION_KINDS, "precondition kind")
        _validate_precondition_payload(kind, tagged[1])
    if ordering_keys != sorted(ordering_keys):
        _fail("precondition IDs must be in canonical order")


def _validate_causal_chain(value: object) -> None:
    chain = _array(value, "causal chain")
    if not chain:
        _fail("causal chain must be non-empty")
    for ordinal, edge in enumerate(chain):
        fields = _array(edge, "causal chain edge", 7)
        if _uint32(fields[0], "causal edge ordinal") != ordinal:
            _fail("causal edge ordinals must be exactly 0..n-1")
        _uint32(fields[1], "causal edge source role position")
        _enum(fields[2], _OPERATIONS, "causal edge operation")
        _validate_b2_boundary_refs(fields[3])
        _validate_optional_uint32(fields[4], "causal edge event/effect role position")
        _validate_optional_uint32(fields[5], "causal edge destination role position")
        _validate_b1_citation_refs(fields[6])


def _validate_optional_uint32(value: object, label: str) -> None:
    if value is not None:
        _uint32(value, label)


def _validate_required_channels(value: object) -> None:
    _ordered_enum_array(
        _array(value, "required relation channels"),
        _RELATION_CHANNELS,
        "required relation channels",
    )


def _validate_separation_obligations(value: object) -> None:
    obligations = _array(value, "separation obligations", len(_RELATION_CHANNELS))
    channels: list[str] = []
    for obligation in obligations:
        fields = _array(obligation, "separation obligation", 2)
        channels.append(_enum(fields[0], _RELATION_CHANNELS, "separation channel"))
        _enum(fields[1], _REQUIRED_CONCLUSIONS, "required separation conclusion")
    if channels != list(_RELATION_CHANNELS):
        _fail("separation obligations must cover every channel in order")


def _validate_relation_proof_payload(value: object) -> None:
    tagged = _array(value, "relation proof payload", 2)
    kind = _enum(tagged[0], _PROOF_KINDS, "relation proof kind")
    fields = _array(tagged[1], f"{kind} proof payload")
    if kind == "positive_interaction":
        if len(fields) != 3:
            _fail("positive interaction payload must contain three fields")
        _validate_causal_chain(fields[0])
        _validate_required_channels(fields[1])
        if fields[2] is not None:
            _validate_class_projection(fields[2])
    elif kind == "positive_separation":
        if len(fields) != 2:
            _fail("positive separation payload must contain two fields")
        _enum(fields[0], _SEPARATION_KINDS, "separation kind")
        _validate_separation_obligations(fields[1])
    else:
        if len(fields) != 4:
            _fail("model-bound scope payload must contain four fields")
        _validate_model_boundary_ref(fields[0])
        _enum(fields[1], _SCOPE_REASONS, "scope reason")
        _validate_candidate_shape(fields[2])
        _validate_evidence_refs(fields[3], "positive boundary evidence references")
        if not _array(fields[3], "positive boundary evidence references"):
            _fail("positive boundary evidence references must be non-empty")


def _validate_relation_binding(value: object) -> None:
    fields = _array(value, "relation binding", 5)
    directionality = _enum(fields[2], _DIRECTIONALITIES, "relation directionality")
    _enum(fields[0], _SCOPES, "relation scope")
    _enum(fields[1], _RELATIONS, "relation name")
    _enum(fields[3], _HOST_RELATIONSHIPS, "relation host relationship")
    _validate_participant_roles(fields[4], directionality)


def _validate_candidate_universe_binding(value: object) -> None:
    fields = _array(value, "candidate universe binding", 3)
    _text(fields[0], "candidate universe path")
    _text(fields[1], "candidate universe schema")
    _bytes32(fields[2], "candidate universe digest")


def _validate_domain_binding(value: object) -> None:
    fields = _array(value, "domain binding", 2)
    _enum(fields[0], _REVIEW_DOMAINS, "review domain")
    _enum(fields[1], _APPLICABILITY, "applicability")


def _validate_context_binding(value: object) -> None:
    fields = _array(value, "context binding", 4)
    _enum(fields[0], _ARITIES, "context arity")
    directionality = _enum(fields[1], _DIRECTIONALITIES, "context directionality")
    _validate_participant_roles(fields[2], directionality)
    _enum(fields[3], _HOST_RELATIONSHIPS, "context host relationship")


def _validate_precondition_attestations(value: object) -> None:
    attestations = _array(value, "precondition attestations")
    ordering_keys: list[bytes] = []
    for attestation in attestations:
        fields = _array(attestation, "precondition attestation", 4)
        precondition_id = _text(fields[0], "attestation precondition ID")
        ordering_keys.append(encode_canonical(precondition_id))
        _validate_cbor_value(fields[1], "observed precondition payload")
        _validate_nonempty_evidence_refs(fields[2], "member evidence references")
        _text(fields[3], "precondition equivalence rationale")
    if ordering_keys != sorted(ordering_keys):
        _fail("precondition IDs must be in canonical order")
    if len(set(ordering_keys)) != len(ordering_keys):
        _fail("precondition attestations must be duplicate-free")


def _validate_application_members(
    value: object,
    validator: Callable[[object], None],
    label: str,
) -> None:
    members = _array(value, label)
    ordering_keys: list[bytes] = []
    for member in members:
        validator(member)
        fields = _array(member, f"{label} member")
        identity = _array(fields[1], f"{label} member candidate identity", 6)
        digest = _bytes32(identity[5], f"{label} member candidate digest")
        source_instance_id = _text(fields[2], f"{label} member source instance ID")
        ordering_keys.append(encode_canonical([digest, source_instance_id]))
    if ordering_keys != sorted(ordering_keys):
        _fail(f"{label} must be sorted by candidate digest and source instance ID")
    if len(set(ordering_keys)) != len(ordering_keys):
        _fail(f"{label} must be duplicate-free")


def _validate_class_projection_equivalence(value: object) -> None:
    fields = _array(value, "class projection equivalence", 6)
    _validate_class_projection(fields[0])
    _validate_class_projection(fields[1])
    if encode_canonical(cast(PersistenceValue, fields[0])) != encode_canonical(
        cast(PersistenceValue, fields[1])
    ):
        _fail("theorem and member class projections must be identical")
    _exact_ordered_enum_array(fields[2], _CLASS_PROJECTION_POSITIONS, "equal class positions")
    semantic_claim = _array(fields[3], "semantic claim relation", 2)
    if semantic_claim[0] != "same_theorem_semantic_id":
        _fail("class semantic claim relation is not closed")
    _bytes32(semantic_claim[1], "theorem semantic digest")
    _validate_nonempty_evidence_refs(fields[4], "class equivalence evidence")
    _text(fields[5], "class equivalence rationale")


def _validate_channel_coverage(value: object) -> None:
    fields = _array(value, "channel coverage", 6)
    _enum(fields[0], _RELATION_CHANNELS, "covered channel")
    _enum(fields[1], _REQUIRED_CONCLUSIONS, "channel coverage conclusion")
    _validate_positive_boundary_facts(fields[2], "positive boundary facts")
    _validate_nonempty_evidence_refs(fields[3], "channel source evidence")
    _validate_b1_citation_refs(fields[4])
    _text(fields[5], "channel coverage rationale")


def _validate_channel_coverages(value: object) -> None:
    coverages = _array(value, "channel coverages", len(_RELATION_CHANNELS))
    channels: list[str] = []
    for coverage in coverages:
        _validate_channel_coverage(coverage)
        channels.append(cast(str, _array(coverage, "channel coverage")[0]))
    if channels != list(_RELATION_CHANNELS):
        _fail("channel coverages must cover every relation channel in order")


def _validate_scope_attestation(value: object) -> None:
    fields = _array(value, "scope boundary attestation", 6)
    _text(fields[0], "scope model ID")
    _text(fields[1], "scope model version")
    _validate_model_boundary_ref(fields[2])
    _enum(fields[3], _SCOPE_REASONS, "scope reason")
    _validate_candidate_shape(fields[4])
    _validate_evidence_refs(fields[5], "positive boundary evidence references")
    if not cast(list[object], fields[5]):
        _fail("positive boundary evidence references must be non-empty")


def _validate_member_proof_attestation(value: object) -> None:
    tagged = _array(value, "member proof attestation", 2)
    kind = _enum(tagged[0], _PROOF_KINDS, "member proof kind")
    fields = _array(tagged[1], f"{kind} member proof")
    if kind == "positive_interaction":
        if len(fields) != 2:
            _fail("positive interaction member proof must contain two fields")
        ordinals = _array(fields[0], "causal chain ordinals")
        for ordinal, value in enumerate(ordinals):
            if _uint32(value, "causal chain ordinal") != ordinal:
                _fail("causal chain ordinals must be exactly 0..n-1")
        if fields[1] is not None:
            _validate_class_projection_equivalence(fields[1])
    elif kind == "positive_separation":
        if len(fields) != 1:
            _fail("positive separation member proof must contain one field")
        _validate_channel_coverages(fields[0])
    else:
        if len(fields) != 1:
            _fail("scope member proof must contain one field")
        _validate_scope_attestation(fields[0])


def _validate_relation_member(value: object) -> None:
    fields = _array(value, "relation application member", 8)
    _text(fields[0], "candidate ID")
    _validate_digest_reference(fields[1], "candidate identity")
    _text(fields[2], "source instance ID")
    _validate_candidate_universe_binding(fields[3])
    _validate_relation_binding(fields[4])
    _validate_precondition_attestations(fields[5])
    _validate_nonempty_evidence_refs(fields[6], "member evidence references")
    _validate_member_proof_attestation(fields[7])


def _validate_domain_criterion(value: object) -> None:
    fields = _array(value, "domain criterion", 2)
    kind = _any_text(fields[0], "domain criterion kind")
    payload = _array(fields[1], "domain criterion payload")
    if kind in {"channel_implicated", "channel_excluded"}:
        if len(payload) != 2:
            _fail("channel domain criterion must contain two fields")
        _enum(payload[0], _RELATION_CHANNELS, "domain criterion channel")
        _validate_positive_boundary_fact(payload[1])
    elif kind == "rule_domain_required":
        if len(payload) != 2:
            _fail("required rule-domain criterion must contain two fields")
        _validate_b1_citation_ref(payload[0])
        covered = _array(payload[1], "covered boundary fields")
        fields = [
            _enum(
                field,
                _DOMAIN_BOUNDARY_FIELDS,
                "covered boundary field",
            )
            for field in covered
        ]
        if not fields or fields != sorted(
            fields,
            key=lambda field: _DOMAIN_BOUNDARY_FIELDS.index(field),
        ):
            _fail("covered boundary fields must be a non-empty ordered subsequence")
    elif kind == "rule_domain_excluded":
        if len(payload) != 2:
            _fail("excluded rule-domain criterion must contain two fields")
        excluded = _array(payload[0], "excluded rule-domain ID", 2)
        if excluded[0] == "b1_final_citation":
            _validate_b1_citation_ref(excluded[1])
        elif excluded[0] == "review_domain":
            _enum(excluded[1], _REVIEW_DOMAINS, "excluded review domain")
        else:
            _fail("excluded rule-domain ID variant is not closed")
        _validate_positive_boundary_fact(payload[1])
    else:
        _fail("domain criterion variant is not closed")


def _validate_domain_criterion_array(value: object) -> None:
    criteria = _array(value, "domain criteria")
    channel_names: set[str] = set()
    for criterion in criteria:
        _validate_domain_criterion(criterion)
        fields = _array(criterion, "domain criterion", 2)
        if fields[0] in {"channel_implicated", "channel_excluded"}:
            payload = _array(fields[1], "domain criterion payload", 2)
            channel = cast(str, payload[0])
            if channel in channel_names:
                _fail("domain criterion channels must be unique")
            channel_names.add(channel)


def _validate_domain_member(value: object) -> None:
    fields = _array(value, "domain application member", 8)
    _text(fields[0], "candidate ID")
    _validate_digest_reference(fields[1], "candidate identity")
    _text(fields[2], "source instance ID")
    _validate_candidate_universe_binding(fields[3])
    _validate_domain_binding(fields[4])
    _validate_precondition_attestations(fields[5])
    _validate_nonempty_evidence_refs(fields[6], "member evidence references")
    attestation = _array(fields[7], "domain member attestation", 1)
    criteria = _array(attestation[0], "criterion attestations")
    for index, criterion in enumerate(criteria):
        criterion_fields = _array(criterion, "criterion attestation", 4)
        if _uint32(criterion_fields[0], "criterion index") != index:
            _fail("criterion attestations must use the contiguous theorem order")
        _validate_domain_criterion(criterion_fields[1])
        _validate_nonempty_evidence_refs(criterion_fields[2], "criterion evidence")
        _text(criterion_fields[3], "criterion equivalence rationale")


def _validate_context_member(value: object) -> None:
    fields = _array(value, "context application member", 8)
    _text(fields[0], "candidate ID")
    _validate_digest_reference(fields[1], "candidate identity")
    _text(fields[2], "source instance ID")
    _validate_candidate_universe_binding(fields[3])
    _validate_context_binding(fields[4])
    _validate_precondition_attestations(fields[5])
    _validate_nonempty_evidence_refs(fields[6], "member evidence references")
    attestation = _array(fields[7], "context member attestation", 1)
    slots = _array(attestation[0], "context slot attestations", len(_CONTEXT_SLOT_SEQUENCE))
    for slot, expected in zip(slots, _CONTEXT_SLOT_SEQUENCE, strict=True):
        _validate_context_slot_attestation(slot, expected)


def _validate_record_input(value: list[object], *, application: bool) -> None:
    if application:
        if len(value) != 3:
            _fail("application record input must contain three fields")
        _text(value[0], "application record schema")
        _bytes32(value[1], "application ID")
        _validate_review_event_ref_array(value[2])
    else:
        if len(value) != 5:
            _fail("theorem record input must contain five fields")
        _text(value[0], "theorem record schema")
        _bytes32(value[1], "theorem ID")
        _validate_nonempty_evidence_refs(value[2], "theorem source evidence")
        _validate_review_event_ref_array(value[3])
        _text(value[4], "theorem semantic rationale")


def _validate_supersession_input(value: list[object]) -> None:
    if len(value) != 8:
        _fail("supersession input must contain eight fields")
    _text(value[0], "supersession schema")
    _bytes32(value[1], "superseded record ID")
    replacement = value[2]
    if replacement is not None:
        _bytes32(replacement, "replacement record ID")
    kind = _enum(value[3], _RECORD_KINDS, "superseded record kind")
    replacement_kind = value[4]
    if replacement is None:
        if replacement_kind is not None or value[5] != "authority_revocation":
            _fail("revocation supersession must null both replacement fields")
    else:
        if _enum(replacement_kind, _RECORD_KINDS, "replacement record kind") != kind:
            _fail("supersession replacement kind must match")
        if value[5] == "authority_revocation":
            _fail("authority_revocation cannot carry a replacement")
    _enum(
        value[5],
        (
            "semantic_correction",
            "source_revision",
            "model_revision",
            "authority_revocation",
        ),
        "supersession reason",
    )
    _validate_nonempty_evidence_refs(value[6], "supersession source evidence")
    _validate_review_event_ref_array(value[7])


def _validate_acceptance_subject_input(value: list[object]) -> None:
    if len(value) != 3:
        _fail("acceptance subject input must contain three fields")
    _text(value[0], "acceptance subject schema")
    kind = _enum(value[1], _SUBJECT_KINDS, "acceptance subject kind")
    payload = _array(value[2], "acceptance subject payload")
    if kind.endswith("_theorem_record"):
        if len(payload) != 3:
            _fail("theorem-record subject payload must contain three fields")
        _bytes32(payload[0], "subject theorem ID")
        _validate_nonempty_evidence_refs(payload[1], "subject source evidence")
        _text(payload[2], "subject semantic rationale")
    elif kind.endswith("_application_record"):
        if len(payload) != 1:
            _fail("application-record subject payload must contain one field")
        _bytes32(payload[0], "subject application ID")
    else:
        if len(payload) != 6:
            _fail("supersession subject payload must contain six fields")
        _bytes32(payload[0], "subject superseded ID")
        if payload[1] is not None:
            _bytes32(payload[1], "subject replacement ID")
        kind_value = _enum(payload[2], _RECORD_KINDS, "subject superseded kind")
        replacement_kind = payload[3]
        if payload[1] is None:
            if replacement_kind is not None:
                _fail("subject revocation must null replacement kind")
        elif _enum(replacement_kind, _RECORD_KINDS, "subject replacement kind") != kind_value:
            _fail("subject replacement kind must match")
        _enum(
            payload[4],
            (
                "semantic_correction",
                "source_revision",
                "model_revision",
                "authority_revocation",
            ),
            "subject supersession reason",
        )
        _validate_nonempty_evidence_refs(payload[5], "subject supersession evidence")


def _validate_acceptance_event_input(value: list[object]) -> None:
    if len(value) != 10:
        _fail("acceptance event input must contain ten fields")
    _text(value[0], "acceptance event schema")
    _enum(value[1], _SUBJECT_KINDS, "acceptance event subject kind")
    _bytes32(value[2], "acceptance event subject digest")
    if value[3] != "human_accepted":
        _fail("acceptance event decision must be human_accepted")
    _validate_roster_ref_array(value[4])
    bindings = _array(value[5], "reviewer role bindings")
    if not bindings:
        _fail("reviewer role bindings must be non-empty")
    for binding in bindings:
        _validate_role_binding(binding)
    reviewer_ids = [cast(str, _array(binding, "reviewer role binding")[0]) for binding in bindings]
    if reviewer_ids != sorted(reviewer_ids) or len(set(reviewer_ids)) != len(reviewer_ids):
        _fail("reviewer role bindings must be sorted and duplicate-free")
    _enum(value[6], ("multi_reviewer", "solo_separate_self_review"), "review mode")
    if value[7] != ACCEPTANCE_CHECKLIST_V1:
        _fail("acceptance checklist is not the V1 contract")
    source_bindings = _array(value[8], "acceptance source bindings")
    if not source_bindings:
        _fail("acceptance source bindings must be non-empty")
    for binding in source_bindings:
        _validate_source_binding_array(binding)
    if any(
        cast(str, _array(binding, "source binding")[0]) == "acceptance_event_leaf"
        for binding in source_bindings
    ):
        _fail("acceptance event cannot bind its own leaf")
    if not any(
        cast(str, _array(binding, "source binding")[0]) == "declared_model"
        for binding in source_bindings
    ):
        _fail("acceptance event must bind the declared model")
    roster_ref = _array(value[4], "reviewer roster reference", 3)
    expected_roster = [
        "reviewer_roster_leaf",
        roster_ref[0],
        roster_ref[1],
        roster_ref[2],
    ]
    if not any(
        _array(binding, "source binding", 4) == expected_roster for binding in source_bindings
    ):
        _fail("acceptance event must bind its reviewer roster leaf")
    _canonical_array(source_bindings, _validate_source_binding_array, "acceptance source bindings")
    _validate_acceptance_evidence_refs(value[9])


def _validate_kind_payload(kind: AuthorityIdentityKind, fields: list[AuthorityValue]) -> None:
    values = cast(list[object], fields)
    if kind is AuthorityIdentityKind.RELATION_THEOREM:
        _text(values[1], "model ID")
        _enum(values[2], _PROOF_KINDS, "proof kind")
        _enum(values[3], _ARITIES, "arity")
        _enum(values[4], _RELATIONS, "relation")
        directionality = _enum(values[5], _DIRECTIONALITIES, "directionality")
        _enum(values[6], _HOST_RELATIONSHIPS, "host relationship")
        _validate_participant_roles(values[7], directionality)
        _validate_preconditions(values[8])
        _validate_relation_proof_payload(values[9])
        _validate_b2_boundary_refs(values[10])
        _validate_b1_citation_refs(values[11])
    elif kind is AuthorityIdentityKind.DOMAIN_THEOREM:
        _text(values[1], "domain model ID")
        _enum(values[2], _REVIEW_DOMAINS, "review domain")
        _enum(values[3], _APPLICABILITY, "applicability")
        _validate_domain_criterion_array(values[4])
        _validate_preconditions(values[5])
        _validate_b2_boundary_refs(values[6])
        _validate_b1_citation_refs(values[7])
    elif kind is AuthorityIdentityKind.CONTEXT_THEOREM:
        _text(values[1], "context model ID")
        _validate_context_binding(values[2])
        _validate_slot_value_vector(
            values[3], _CONTEXT_DIMENSIONS, _CONTEXT_VALUES, "context dimension values"
        )
        _validate_slot_value_vector(
            values[4], _TEMPORAL_SEMANTICS, _TEMPORAL_VALUES, "temporal semantic values"
        )
        _validate_preconditions(values[5])
        _validate_b2_boundary_refs(values[6])
        _validate_b1_citation_refs(values[7])
    elif kind in {
        AuthorityIdentityKind.RELATION_THEOREM_RECORD,
        AuthorityIdentityKind.DOMAIN_THEOREM_RECORD,
        AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
    }:
        _validate_record_input(values, application=False)
    elif kind is AuthorityIdentityKind.RELATION_APPLICATION:
        _bytes32(values[1], "relation theorem record ID")
        _enum(values[2], _TERMINAL_DISPOSITIONS, "terminal disposition")
        _validate_application_members(
            values[3], _validate_relation_member, "relation application members"
        )
    elif kind is AuthorityIdentityKind.DOMAIN_APPLICATION:
        _bytes32(values[1], "domain theorem record ID")
        _enum(values[2], _REVIEW_DOMAINS, "review domain")
        _enum(values[3], _APPLICABILITY, "applicability")
        _validate_application_members(
            values[4], _validate_domain_member, "domain application members"
        )
    elif kind is AuthorityIdentityKind.CONTEXT_APPLICATION:
        _bytes32(values[1], "context theorem record ID")
        _validate_application_members(
            values[2], _validate_context_member, "context application members"
        )
    elif kind in {
        AuthorityIdentityKind.RELATION_APPLICATION_RECORD,
        AuthorityIdentityKind.DOMAIN_APPLICATION_RECORD,
        AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD,
    }:
        _validate_record_input(values, application=True)
    elif kind in {
        AuthorityIdentityKind.RELATION_SUPERSESSION,
        AuthorityIdentityKind.DOMAIN_SUPERSESSION,
        AuthorityIdentityKind.CONTEXT_SUPERSESSION,
    }:
        _validate_supersession_input(values)
    elif kind is AuthorityIdentityKind.ACCEPTANCE_SUBJECT:
        _validate_acceptance_subject_input(values)
    elif kind is AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT:
        _validate_acceptance_event_input(values)
    elif kind is AuthorityIdentityKind.CONTEXT_APPLICATION_V2:
        _validate_context_application_v2_input(values)
    elif kind is AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2:
        _validate_context_application_record_v2_input(values)
    elif kind is AuthorityIdentityKind.CONTEXT_SUPERSESSION_V2:
        _validate_context_supersession_v2_input(values)
    elif kind is AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V2:
        _validate_context_supersession_record_v2_input(values)
    elif kind is AuthorityIdentityKind.ACCEPTANCE_SUBJECT_V3:
        _validate_acceptance_subject_v3_input(values)
    elif kind is AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT_V3:
        _validate_acceptance_event_v3_input(values)


@dataclass(frozen=True)
class DigestReferenceV1:
    """The complete six-field digest-envelope reference used by V2 inputs."""

    envelope_id: str
    algorithm_id: str
    semantic_domain: str
    payload_codec_id: str
    input_schema_id: str
    digest_bytes: bytes

    def __post_init__(self) -> None:
        if self.envelope_id != DIGEST_ENVELOPE_ID:
            raise AuthorityContractError("digest reference envelope is not V1")
        if self.algorithm_id != SHA256_ID:
            raise AuthorityContractError("digest reference algorithm is not sha-256")
        if self.payload_codec_id != CANONICAL_CBOR_ID:
            raise AuthorityContractError("digest reference codec is not canonical CBOR V1")
        if not isinstance(self.semantic_domain, str) or not self.semantic_domain:
            raise AuthorityContractError("digest reference semantic domain must be non-empty")
        if not isinstance(self.input_schema_id, str) or not self.input_schema_id:
            raise AuthorityContractError("digest reference input schema must be non-empty")
        _require_digest_bytes(self.digest_bytes, "digest reference digest")

    @classmethod
    def from_identity(cls, identity: AuthorityIdentityV1) -> DigestReferenceV1:
        return cls(
            envelope_id=DIGEST_ENVELOPE_ID,
            algorithm_id=SHA256_ID,
            semantic_domain=identity.semantic_domain,
            payload_codec_id=CANONICAL_CBOR_ID,
            input_schema_id=identity.input_schema_id,
            digest_bytes=identity.digest_bytes,
        )

    @classmethod
    def from_cbor(cls, value: object) -> DigestReferenceV1:
        fields = _array(value, "digest reference", 6)
        return cls(
            envelope_id=_any_text(fields[0], "digest reference envelope"),
            algorithm_id=_any_text(fields[1], "digest reference algorithm"),
            semantic_domain=_any_text(fields[2], "digest reference semantic domain"),
            payload_codec_id=_any_text(fields[3], "digest reference codec"),
            input_schema_id=_any_text(fields[4], "digest reference input schema"),
            digest_bytes=_bytes32(fields[5], "digest reference digest"),
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.envelope_id,
            self.algorithm_id,
            self.semantic_domain,
            self.payload_codec_id,
            self.input_schema_id,
            self.digest_bytes,
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "envelope_id": self.envelope_id,
            "algorithm_id": self.algorithm_id,
            "semantic_domain": self.semantic_domain,
            "payload_codec_id": self.payload_codec_id,
            "input_schema_id": self.input_schema_id,
            "digest_hex": self.digest_bytes.hex(),
        }


def _identity_to_wire(identity: AuthorityIdentityV1) -> dict[str, object]:
    return DigestReferenceV1.from_identity(identity).to_wire()


def _persistence_value_to_wire(value: AuthorityValue) -> object:
    if isinstance(value, bytes):
        return value.hex()
    if isinstance(value, list):
        return [_persistence_value_to_wire(child) for child in value]
    return value


def _precondition_to_wire(value: AuthorityValue) -> dict[str, object]:
    fields = _array(value, "V1 precondition attestation", 4)
    return {
        "precondition_id": _text(fields[0], "V1 precondition ID"),
        "observed_value": _persistence_value_to_wire(cast(AuthorityValue, fields[1])),
        "evidence_refs": [
            EvidenceRefV1(
                authority_kind=cast(str, _array(ref, "V1 evidence reference", 4)[0]),
                path=cast(str, _array(ref, "V1 evidence reference", 4)[1]),
                locator=(
                    cast(str, _array(_array(ref, "V1 evidence reference", 4)[2], "locator", 2)[0]),
                    cast(
                        str | int | None,
                        _array(_array(ref, "V1 evidence reference", 4)[2], "locator", 2)[1],
                    ),
                ),
                raw_sha256=cast(bytes, _array(ref, "V1 evidence reference", 4)[3]),
            ).to_wire()
            for ref in _array(fields[2], "V1 precondition evidence")
        ],
        "equivalence_rationale": _text(fields[3], "V1 precondition rationale"),
    }


def _typed_canonical_items(
    values: tuple[_CborConvertible, ...],
    label: str,
    *,
    allow_empty: bool = True,
) -> None:
    if not allow_empty and not values:
        raise AuthorityContractError(f"{label} must be non-empty")
    encoded = [encode_canonical(value.to_cbor()) for value in values]
    if encoded != sorted(encoded):
        raise AuthorityContractError(f"{label} must be in canonical order")
    if len(set(encoded)) != len(encoded):
        raise AuthorityContractError(f"{label} must be duplicate-free")


def _require_evidence_tuple(
    values: tuple[EvidenceRefV1, ...],
    label: str,
    *,
    allow_empty: bool = True,
) -> None:
    if any(not isinstance(value, EvidenceRefV1) for value in values):
        raise AuthorityContractError(f"{label} contains a non-V1 evidence reference")
    _typed_canonical_items(values, label, allow_empty=allow_empty)


@dataclass(frozen=True)
class ContextSlotBridgeAttestationV2:
    slot_name: str
    source_value: str
    reviewed_value: str
    relation: ContextBridgeRelationV2
    evidence_refs: tuple[EvidenceRefV1, ...]
    rationale: str

    def __post_init__(self) -> None:
        _validate_context_slot_value("context_dimension", self.slot_name, self.source_value)
        _validate_context_slot_value("context_dimension", self.slot_name, self.reviewed_value)
        if not isinstance(self.relation, ContextBridgeRelationV2):
            raise AuthorityContractError("context bridge relation is not closed in V2")
        _require_evidence_tuple(self.evidence_refs, "context slot evidence", allow_empty=False)
        _text(self.rationale, "context slot rationale")

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.slot_name,
            self.source_value,
            self.reviewed_value,
            self.relation.value,
            [reference.to_cbor() for reference in self.evidence_refs],
            self.rationale,
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "slot_name": self.slot_name,
            "source_value": self.source_value,
            "reviewed_value": self.reviewed_value,
            "relation": self.relation.value,
            "evidence_refs": [reference.to_wire() for reference in self.evidence_refs],
            "rationale": self.rationale,
        }


@dataclass(frozen=True)
class TemporalSlotAttestationV2:
    slot_name: str
    reviewed_value: str
    evidence_refs: tuple[EvidenceRefV1, ...]
    rationale: str

    def __post_init__(self) -> None:
        _validate_context_slot_value("temporal_semantic", self.slot_name, self.reviewed_value)
        _require_evidence_tuple(self.evidence_refs, "temporal slot evidence", allow_empty=False)
        _text(self.rationale, "temporal slot rationale")

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.slot_name,
            self.reviewed_value,
            [reference.to_cbor() for reference in self.evidence_refs],
            self.rationale,
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "slot_name": self.slot_name,
            "reviewed_value": self.reviewed_value,
            "evidence_refs": [reference.to_wire() for reference in self.evidence_refs],
            "rationale": self.rationale,
        }


@dataclass(frozen=True)
class ContextMemberBridgeAttestationV2:
    context: tuple[ContextSlotBridgeAttestationV2, ...]
    temporal: tuple[TemporalSlotAttestationV2, ...]

    def __post_init__(self) -> None:
        if len(self.context) != len(_CONTEXT_DIMENSIONS):
            raise AuthorityContractError("V2 bridge must contain exactly ten context slots")
        if len(self.temporal) != len(_TEMPORAL_SEMANTICS):
            raise AuthorityContractError("V2 bridge must contain exactly four temporal slots")
        if any(not isinstance(slot, ContextSlotBridgeAttestationV2) for slot in self.context):
            raise AuthorityContractError("V2 context bridge contains an invalid slot")
        if any(not isinstance(slot, TemporalSlotAttestationV2) for slot in self.temporal):
            raise AuthorityContractError("V2 temporal bridge contains an invalid slot")
        if tuple(slot.slot_name for slot in self.context) != _CONTEXT_DIMENSIONS:
            raise AuthorityContractError("V2 context slots must use the V1 canonical order")
        if tuple(slot.slot_name for slot in self.temporal) != _TEMPORAL_SEMANTICS:
            raise AuthorityContractError("V2 temporal slots must use the V1 canonical order")

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            [slot.to_cbor() for slot in self.context],
            [slot.to_cbor() for slot in self.temporal],
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "context_slot_attestations": [slot.to_wire() for slot in self.context],
            "temporal_slot_attestations": [slot.to_wire() for slot in self.temporal],
        }


@dataclass(frozen=True)
class ContextApplicationMemberV2:
    candidate_id: str
    candidate_identity_digest_reference: DigestReferenceV1
    source_instance_id: str
    candidate_universe_binding: list[AuthorityValue]
    context_binding_v1: list[AuthorityValue]
    precondition_attestations_v1: list[AuthorityValue]
    member_evidence_refs: tuple[EvidenceRefV1, ...]
    context_member_bridge_attestation_v2: ContextMemberBridgeAttestationV2

    def __post_init__(self) -> None:
        _text(self.candidate_id, "candidate ID")
        if not isinstance(self.candidate_identity_digest_reference, DigestReferenceV1):
            raise AuthorityContractError("candidate identity must be a full DigestReferenceV1")
        _text(self.source_instance_id, "source instance ID")
        _validate_candidate_universe_binding(self.candidate_universe_binding)
        _validate_context_binding(self.context_binding_v1)
        _validate_precondition_attestations(self.precondition_attestations_v1)
        _require_evidence_tuple(self.member_evidence_refs, "member evidence", allow_empty=False)
        if not isinstance(
            self.context_member_bridge_attestation_v2,
            ContextMemberBridgeAttestationV2,
        ):
            raise AuthorityContractError("member bridge attestation is not V2")

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.candidate_id,
            self.candidate_identity_digest_reference.to_cbor(),
            self.source_instance_id,
            self.candidate_universe_binding,
            self.context_binding_v1,
            self.precondition_attestations_v1,
            [reference.to_cbor() for reference in self.member_evidence_refs],
            self.context_member_bridge_attestation_v2.to_cbor(),
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "candidate_id": self.candidate_id,
            "candidate_identity": self.candidate_identity_digest_reference.to_wire(),
            "source_instance_id": self.source_instance_id,
            "candidate_universe_binding": {
                "path": self.candidate_universe_binding[0],
                "schema": self.candidate_universe_binding[1],
                "raw_sha256": cast(bytes, self.candidate_universe_binding[2]).hex(),
            },
            "context_binding": {
                "arity": self.context_binding_v1[0],
                "directionality": self.context_binding_v1[1],
                "participant_roles": self.context_binding_v1[2],
                "host_relationship": self.context_binding_v1[3],
            },
            "precondition_attestations": [
                _precondition_to_wire(precondition)
                for precondition in self.precondition_attestations_v1
            ],
            "member_evidence_refs": [
                reference.to_wire() for reference in self.member_evidence_refs
            ],
            "context_member_attestation": self.context_member_bridge_attestation_v2.to_wire(),
        }


def _validate_context_slot_bridge_attestation(value: object, expected: str) -> None:
    fields = _array(value, "V2 context slot bridge attestation", 6)
    _validate_context_slot_value("context_dimension", fields[0], fields[1])
    _validate_context_slot_value("context_dimension", fields[0], fields[2])
    _enum(
        fields[3],
        (
            ContextBridgeRelationV2.EXACT_MATCH.value,
            ContextBridgeRelationV2.REVIEWED_DIVERGENCE.value,
        ),
        "V2 context bridge relation",
    )
    if fields[0] != expected:
        _fail("V2 context slots must use the V1 canonical order")
    _validate_nonempty_evidence_refs(fields[4], "V2 context slot evidence")
    _text(fields[5], "V2 context slot rationale")


def _validate_temporal_slot_attestation(value: object, expected: str) -> None:
    fields = _array(value, "V2 temporal slot attestation", 4)
    _validate_context_slot_value("temporal_semantic", fields[0], fields[1])
    if fields[0] != expected:
        _fail("V2 temporal slots must use the V1 canonical order")
    _validate_nonempty_evidence_refs(fields[2], "V2 temporal slot evidence")
    _text(fields[3], "V2 temporal slot rationale")


def _validate_context_member_bridge_v2(value: object) -> None:
    fields = _array(value, "V2 member bridge", 2)
    context = _array(fields[0], "V2 context slot bridge", len(_CONTEXT_DIMENSIONS))
    for slot, expected in zip(
        context,
        _CONTEXT_DIMENSIONS,
        strict=True,
    ):
        _validate_context_slot_bridge_attestation(slot, expected)
    temporal = _array(fields[1], "V2 temporal slot bridge", len(_TEMPORAL_SEMANTICS))
    for slot, expected in zip(temporal, _TEMPORAL_SEMANTICS, strict=True):
        _validate_temporal_slot_attestation(slot, expected)


def _validate_context_application_member_v2(value: object) -> None:
    fields = _array(value, "V2 context application member", 8)
    _text(fields[0], "V2 candidate ID")
    DigestReferenceV1.from_cbor(fields[1])
    _text(fields[2], "V2 source instance ID")
    _validate_candidate_universe_binding(fields[3])
    _validate_context_binding(fields[4])
    _validate_precondition_attestations(fields[5])
    _validate_nonempty_evidence_refs(fields[6], "V2 member evidence references")
    _validate_context_member_bridge_v2(fields[7])


def _validate_context_application_members_v2(value: object) -> None:
    members = _array(value, "V2 context application members")
    if not members:
        _fail("V2 context application members must be non-empty")
    ordering_keys: list[bytes] = []
    for member in members:
        _validate_context_application_member_v2(member)
        fields = _array(member, "V2 context application member", 8)
        identity = DigestReferenceV1.from_cbor(fields[1])
        ordering_keys.append(encode_canonical([identity.digest_bytes, fields[2]]))
    if ordering_keys != sorted(ordering_keys):
        _fail("V2 context application members must use the V1 digest/source order")
    if len(set(ordering_keys)) != len(ordering_keys):
        _fail("V2 context application members must be duplicate-free")


@dataclass(frozen=True)
class ContextAuthoritySourceBindingV2:
    artifact_role: str
    path: str
    schema: str | None
    raw_sha256: bytes

    def __post_init__(self) -> None:
        if self.artifact_role not in CONTEXT_AUTHORITY_SOURCE_ROLES_V2:
            raise AuthorityContractError("context authority source role is not closed in V2")
        _require_repo_relative_path(self.path, "context authority source path")
        if self.artifact_role in _CONTEXT_AUTHORITY_STATIC_BINDING_REGISTRY_V2:
            expected_path, expected_schema = _CONTEXT_AUTHORITY_STATIC_BINDING_REGISTRY_V2[
                self.artifact_role
            ]
            if self.path != expected_path or self.schema != expected_schema:
                raise AuthorityContractError("context authority source role/path/schema mismatch")
        else:
            pattern, expected_schema = _CONTEXT_AUTHORITY_LEAF_BINDING_REGISTRY_V2[
                self.artifact_role
            ]
            if re.fullmatch(pattern, self.path) is None or self.schema != expected_schema:
                raise AuthorityContractError("context authority leaf path/schema mismatch")
        _require_digest_bytes(self.raw_sha256, "context authority source digest")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.artifact_role, self.path, self.schema, self.raw_sha256]

    def to_wire(self) -> dict[str, object]:
        return {
            "artifact_role": self.artifact_role,
            "path": self.path,
            "schema": self.schema,
            "raw_sha256": self.raw_sha256.hex(),
        }


def _validate_context_source_binding_array(value: object) -> None:
    fields = _array(value, "V2 context authority source binding", 4)
    schema = fields[2]
    if not isinstance(schema, str | None):
        _fail("V2 context authority source schema must be text or null")
    ContextAuthoritySourceBindingV2(
        artifact_role=_any_text(fields[0], "V2 context authority source role"),
        path=_any_text(fields[1], "V2 context authority source path"),
        schema=schema,
        raw_sha256=_bytes32(fields[3], "V2 context authority source digest"),
    )


def _validate_context_source_bindings(value: object, label: str) -> None:
    bindings = _array(value, label)
    _canonical_array(bindings, _validate_context_source_binding_array, label)
    keys: list[tuple[str, str]] = []
    for binding in bindings:
        fields = _array(binding, label + " binding", 4)
        key = (cast(str, fields[0]), cast(str, fields[1]))
        if key in keys:
            _fail(f"{label} must not duplicate a role/path pair")
        keys.append(key)


@dataclass(frozen=True)
class ReviewEventRefV3:
    path: str
    raw_sha256: bytes
    event_id: str

    def __post_init__(self) -> None:
        _require_repo_relative_path(self.path, "V3 review event path")
        _require_digest_bytes(self.raw_sha256, "V3 review event raw digest")
        if re.fullmatch(r"ae\.v3/[0-9a-f]{64}", self.event_id) is None:
            raise AuthorityContractError("V3 review event ID has the wrong namespace")
        expected_path = (
            "sources/m2_5/authorities/review_acceptance_events/v3/"
            + self.event_id.removeprefix("ae.v3/")
            + ".json"
        )
        if self.path != expected_path:
            raise AuthorityContractError("V3 review event path is not bound to its event ID")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.path, self.raw_sha256, ["event_id", self.event_id]]

    def to_wire(self) -> dict[str, object]:
        return {
            "event_id": self.event_id,
            "path": self.path,
            "raw_sha256": self.raw_sha256.hex(),
        }


def _validate_review_event_ref_v3_array(value: object) -> None:
    fields = _array(value, "V3 review event reference", 3)
    locator = _array(fields[2], "V3 review event locator", 2)
    if locator[0] != "event_id":
        _fail("V3 review event locator must be event_id")
    ReviewEventRefV3(
        path=_any_text(fields[0], "V3 review event path"),
        raw_sha256=_bytes32(fields[1], "V3 review event raw digest"),
        event_id=_any_text(locator[1], "V3 review event ID"),
    )


@dataclass(frozen=True)
class ContextApplicationV2InputV1:
    theorem_record_id_bytes: bytes
    members: tuple[ContextApplicationMemberV2, ...]

    def __post_init__(self) -> None:
        _require_digest_bytes(self.theorem_record_id_bytes, "V2 theorem record ID")
        if not self.members:
            raise AuthorityContractError("V2 application input must contain members")
        _validate_context_application_members_v2([member.to_cbor() for member in self.members])

    def semantic_input(self) -> list[AuthorityValue]:
        return [
            CONTEXT_APPLICATION_INPUT_SCHEMA_V2,
            self.theorem_record_id_bytes,
            [member.to_cbor() for member in self.members],
        ]

    def identity(self) -> AuthorityIdentityV1:
        return compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_APPLICATION_V2,
            self.semantic_input(),
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return self.semantic_input()


@dataclass(frozen=True)
class ContextApplicationV2RecordInputV1:
    context_application_id_bytes: bytes
    review_event_ref_v3: ReviewEventRefV3

    def __post_init__(self) -> None:
        _require_digest_bytes(self.context_application_id_bytes, "V2 application ID")
        if not isinstance(self.review_event_ref_v3, ReviewEventRefV3):
            raise AuthorityContractError("V3 review event reference is required")

    def semantic_input(self) -> list[AuthorityValue]:
        return [
            CONTEXT_APPLICATION_RECORD_INPUT_SCHEMA_V2,
            self.context_application_id_bytes,
            self.review_event_ref_v3.to_cbor(),
        ]

    def identity(self) -> AuthorityIdentityV1:
        return compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2,
            self.semantic_input(),
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return self.semantic_input()


@dataclass(frozen=True)
class ContextApplicationV2Record:
    record_id: AuthorityIdentityV1
    application_id: AuthorityIdentityV1
    theorem_record_id: AuthorityIdentityV1
    members: tuple[ContextApplicationMemberV2, ...]
    review_event_ref_v3: ReviewEventRefV3

    @classmethod
    def from_parts(
        cls,
        application_id: AuthorityIdentityV1,
        theorem_record_id: AuthorityIdentityV1,
        members: tuple[ContextApplicationMemberV2, ...],
        review_event_ref_v3: ReviewEventRefV3,
    ) -> ContextApplicationV2Record:
        record_id = ContextApplicationV2RecordInputV1(
            context_application_id_bytes=application_id.digest_bytes,
            review_event_ref_v3=review_event_ref_v3,
        ).identity()
        return cls(record_id, application_id, theorem_record_id, members, review_event_ref_v3)

    def __post_init__(self) -> None:
        if self.record_id.kind is not AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2:
            raise AuthorityContractError("V2 application record ID has the wrong kind")
        if self.application_id.kind is not AuthorityIdentityKind.CONTEXT_APPLICATION_V2:
            raise AuthorityContractError("V2 application ID has the wrong kind")
        if self.theorem_record_id.kind is not AuthorityIdentityKind.CONTEXT_THEOREM_RECORD:
            raise AuthorityContractError("V1 context theorem record ID has the wrong kind")
        if not self.members:
            raise AuthorityContractError("V2 application record must contain members")
        _validate_context_application_members_v2([member.to_cbor() for member in self.members])
        expected = ContextApplicationV2RecordInputV1(
            self.application_id.digest_bytes,
            self.review_event_ref_v3,
        ).identity()
        if expected != self.record_id:
            raise AuthorityContractError("V2 application record ID does not match its input")

    def acceptance_free_subject_payload(self) -> list[AuthorityValue]:
        return [
            AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD.value,
            self.application_id.digest_bytes,
            self.theorem_record_id.digest_bytes,
            [member.to_cbor() for member in self.members],
        ]

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.record_id.to_cbor(),
            self.application_id.to_cbor(),
            self.theorem_record_id.to_cbor(),
            [member.to_cbor() for member in self.members],
            ["human_accepted", self.review_event_ref_v3.to_cbor()],
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "record_id": self.record_id.as_text(),
            "application_id": self.application_id.as_text(),
            "theorem_record_id": self.theorem_record_id.as_text(),
            "members": [member.to_wire() for member in self.members],
            "acceptance": {
                "decision": "human_accepted",
                "review_event_ref": self.review_event_ref_v3.to_wire(),
            },
        }


def _validate_context_application_v2_input(values: list[object]) -> None:
    if len(values) != 3:
        _fail("V2 context application input must contain three fields")
    _text(values[0], "V2 context application schema")
    _bytes32(values[1], "V2 theorem record ID")
    _validate_context_application_members_v2(values[2])


def _validate_context_application_record_v2_input(values: list[object]) -> None:
    if len(values) != 3:
        _fail("V2 application record input must contain three fields")
    _text(values[0], "V2 application record schema")
    _bytes32(values[1], "V2 application ID")
    _validate_review_event_ref_v3_array(values[2])


@dataclass(frozen=True)
class ContextApplicationV2SupersessionInputV2:
    superseded_record_id_bytes: bytes
    replacement_record_id_bytes: bytes | None
    replacement_record_kind: str | None
    reason_code: SupersessionReason
    source_evidence_refs: tuple[EvidenceRefV1, ...]

    def __post_init__(self) -> None:
        _require_digest_bytes(self.superseded_record_id_bytes, "superseded V2 record ID")
        if self.replacement_record_id_bytes is not None:
            _require_digest_bytes(self.replacement_record_id_bytes, "replacement V2 record ID")
        if self.replacement_record_id_bytes is None:
            if self.replacement_record_kind is not None:
                raise AuthorityContractError("V2 revocation replacement kind must be null")
            if self.reason_code is not SupersessionReason.AUTHORITY_REVOCATION:
                raise AuthorityContractError("V2 null replacement requires revocation")
        else:
            if self.replacement_record_kind != "context_application_v2_record":
                raise AuthorityContractError("V2 replacement kind must be the same record kind")
            if self.reason_code is SupersessionReason.AUTHORITY_REVOCATION:
                raise AuthorityContractError("V2 revocation cannot carry a replacement")
        if not isinstance(self.reason_code, SupersessionReason):
            raise AuthorityContractError("V2 supersession reason is not closed")
        _require_evidence_tuple(
            self.source_evidence_refs, "V2 supersession evidence", allow_empty=False
        )

    def semantic_input(self) -> list[AuthorityValue]:
        return [
            CONTEXT_SUPERSESSION_INPUT_SCHEMA_V2,
            self.superseded_record_id_bytes,
            self.replacement_record_id_bytes,
            "context_application_v2_record",
            self.replacement_record_kind,
            self.reason_code.value,
            [reference.to_cbor() for reference in self.source_evidence_refs],
        ]

    def identity(self) -> AuthorityIdentityV1:
        return compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_SUPERSESSION_V2,
            self.semantic_input(),
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return self.semantic_input()

    def acceptance_free_subject_payload(self) -> list[AuthorityValue]:
        return [
            AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD.value,
            self.identity().digest_bytes,
            self.superseded_record_id_bytes,
            self.replacement_record_id_bytes,
            "context_application_v2_record",
            self.replacement_record_kind,
            self.reason_code.value,
            [reference.to_cbor() for reference in self.source_evidence_refs],
        ]


def _validate_context_supersession_v2_input(values: list[object]) -> None:
    if len(values) != 7:
        _fail("V2 supersession input must contain seven fields")
    _text(values[0], "V2 supersession schema")
    _bytes32(values[1], "superseded V2 record ID")
    replacement = values[2]
    if replacement is not None:
        _bytes32(replacement, "replacement V2 record ID")
    if values[3] != "context_application_v2_record":
        _fail("V2 supersession record kind is not closed")
    replacement_kind = values[4]
    if replacement is None:
        if (
            replacement_kind is not None
            or values[5] != SupersessionReason.AUTHORITY_REVOCATION.value
        ):
            _fail("V2 revocation replacement fields are inconsistent")
    elif replacement_kind != "context_application_v2_record":
        _fail("V2 supersession replacement kind must match")
    elif values[5] == SupersessionReason.AUTHORITY_REVOCATION.value:
        _fail("V2 revocation cannot carry a replacement")
    _enum(values[5], tuple(reason.value for reason in SupersessionReason), "V2 supersession reason")
    _validate_nonempty_evidence_refs(values[6], "V2 supersession source evidence")


@dataclass(frozen=True)
class ContextApplicationV2SupersessionRecordInputV1:
    supersession_id_bytes: bytes
    review_event_ref_v3: ReviewEventRefV3

    def __post_init__(self) -> None:
        _require_digest_bytes(self.supersession_id_bytes, "V2 supersession ID")
        if not isinstance(self.review_event_ref_v3, ReviewEventRefV3):
            raise AuthorityContractError("V3 review event reference is required")

    def semantic_input(self) -> list[AuthorityValue]:
        return [
            CONTEXT_SUPERSESSION_RECORD_INPUT_SCHEMA_V2,
            self.supersession_id_bytes,
            self.review_event_ref_v3.to_cbor(),
        ]

    def identity(self) -> AuthorityIdentityV1:
        return compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V2,
            self.semantic_input(),
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return self.semantic_input()


def _validate_context_supersession_record_v2_input(values: list[object]) -> None:
    if len(values) != 3:
        _fail("V2 supersession record input must contain three fields")
    _text(values[0], "V2 supersession record schema")
    _bytes32(values[1], "V2 supersession ID")
    _validate_review_event_ref_v3_array(values[2])


@dataclass(frozen=True)
class ContextApplicationV2SupersessionRecord:
    record_id: AuthorityIdentityV1
    supersession_id: AuthorityIdentityV1
    superseded_record_id: AuthorityIdentityV1
    replacement_record_id: AuthorityIdentityV1 | None
    reason_code: SupersessionReason
    source_evidence_refs: tuple[EvidenceRefV1, ...]
    review_event_ref_v3: ReviewEventRefV3

    @classmethod
    def from_parts(
        cls,
        supersession_id: AuthorityIdentityV1,
        superseded_record_id: AuthorityIdentityV1,
        replacement_record_id: AuthorityIdentityV1 | None,
        reason_code: SupersessionReason,
        source_evidence_refs: tuple[EvidenceRefV1, ...],
        review_event_ref_v3: ReviewEventRefV3,
    ) -> ContextApplicationV2SupersessionRecord:
        record_id = ContextApplicationV2SupersessionRecordInputV1(
            supersession_id_bytes=supersession_id.digest_bytes,
            review_event_ref_v3=review_event_ref_v3,
        ).identity()
        return cls(
            record_id=record_id,
            supersession_id=supersession_id,
            superseded_record_id=superseded_record_id,
            replacement_record_id=replacement_record_id,
            reason_code=reason_code,
            source_evidence_refs=source_evidence_refs,
            review_event_ref_v3=review_event_ref_v3,
        )

    def __post_init__(self) -> None:
        if self.record_id.kind is not AuthorityIdentityKind.CONTEXT_SUPERSESSION_RECORD_V2:
            raise AuthorityContractError("V2 supersession record ID has the wrong kind")
        if self.supersession_id.kind is not AuthorityIdentityKind.CONTEXT_SUPERSESSION_V2:
            raise AuthorityContractError("V2 supersession ID has the wrong kind")
        if (
            self.superseded_record_id.kind
            is not AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2
        ):
            raise AuthorityContractError("superseded V2 record ID has the wrong kind")
        if (
            self.replacement_record_id is not None
            and self.replacement_record_id.kind
            is not AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2
        ):
            raise AuthorityContractError("replacement V2 record ID has the wrong kind")
        if self.replacement_record_id is None:
            if self.reason_code is not SupersessionReason.AUTHORITY_REVOCATION:
                raise AuthorityContractError("V2 null replacement requires revocation")
        elif self.reason_code is SupersessionReason.AUTHORITY_REVOCATION:
            raise AuthorityContractError("V2 revocation cannot carry a replacement")
        if not isinstance(self.reason_code, SupersessionReason):
            raise AuthorityContractError("V2 supersession reason is not closed")
        _require_evidence_tuple(
            self.source_evidence_refs, "V2 supersession evidence", allow_empty=False
        )
        expected_supersession = ContextApplicationV2SupersessionInputV2(
            superseded_record_id_bytes=self.superseded_record_id.digest_bytes,
            replacement_record_id_bytes=(
                None
                if self.replacement_record_id is None
                else self.replacement_record_id.digest_bytes
            ),
            replacement_record_kind=(
                None if self.replacement_record_id is None else "context_application_v2_record"
            ),
            reason_code=self.reason_code,
            source_evidence_refs=self.source_evidence_refs,
        ).identity()
        if expected_supersession != self.supersession_id:
            raise AuthorityContractError("V2 supersession ID does not match its input")
        expected = ContextApplicationV2SupersessionRecordInputV1(
            self.supersession_id.digest_bytes,
            self.review_event_ref_v3,
        ).identity()
        if expected != self.record_id:
            raise AuthorityContractError("V2 supersession record ID does not match its input")

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.record_id.to_cbor(),
            self.supersession_id.to_cbor(),
            self.superseded_record_id.to_cbor(),
            (None if self.replacement_record_id is None else self.replacement_record_id.to_cbor()),
            "context_application_v2_record",
            (None if self.replacement_record_id is None else "context_application_v2_record"),
            self.reason_code.value,
            [reference.to_cbor() for reference in self.source_evidence_refs],
            ["human_accepted", self.review_event_ref_v3.to_cbor()],
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "record_id": self.record_id.as_text(),
            "supersession_id": self.supersession_id.as_text(),
            "superseded_record_id": self.superseded_record_id.as_text(),
            "replacement_record_id": (
                None if self.replacement_record_id is None else self.replacement_record_id.as_text()
            ),
            "superseded_record_kind": "context_application_v2_record",
            "replacement_record_kind": (
                None if self.replacement_record_id is None else "context_application_v2_record"
            ),
            "reason_code": self.reason_code.value,
            "source_evidence_refs": [
                reference.to_wire() for reference in self.source_evidence_refs
            ],
            "acceptance": {
                "decision": "human_accepted",
                "review_event_ref": self.review_event_ref_v3.to_wire(),
            },
        }

    def acceptance_free_subject_payload(self) -> list[AuthorityValue]:
        return [
            AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD.value,
            self.supersession_id.digest_bytes,
            self.superseded_record_id.digest_bytes,
            (
                None
                if self.replacement_record_id is None
                else self.replacement_record_id.digest_bytes
            ),
            "context_application_v2_record",
            (None if self.replacement_record_id is None else "context_application_v2_record"),
            self.reason_code.value,
            [reference.to_cbor() for reference in self.source_evidence_refs],
        ]


@dataclass(frozen=True)
class AcceptanceSubjectPayloadV3:
    subject_kind: AcceptanceSubjectKindV3
    subject_payload: list[AuthorityValue]

    def __post_init__(self) -> None:
        if not isinstance(self.subject_kind, AcceptanceSubjectKindV3):
            raise AuthorityContractError("V3 acceptance subject kind is not closed")
        _array(self.subject_payload, "V3 acceptance subject payload")

    def semantic_input(self) -> list[AuthorityValue]:
        return [ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V3, self.subject_kind.value, self.subject_payload]

    def identity(self) -> AuthorityIdentityV1:
        return compute_authority_identity(
            AuthorityIdentityKind.ACCEPTANCE_SUBJECT_V3,
            self.semantic_input(),
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.subject_kind.value, self.subject_payload]


def _validate_acceptance_subject_v3_input(values: list[object]) -> None:
    if len(values) != 3:
        _fail("V3 acceptance subject input must contain three fields")
    _text(values[0], "V3 acceptance subject schema")
    subject_kind = _enum(
        values[1],
        tuple(kind.value for kind in AcceptanceSubjectKindV3),
        "V3 acceptance subject kind",
    )
    payload = _array(values[2], "V3 acceptance subject payload")
    if subject_kind == AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD.value:
        if len(payload) != 4:
            _fail("V3 record subject payload must contain four fields")
        if payload[0] != subject_kind:
            _fail("V3 record subject kind marker is inconsistent")
        _bytes32(payload[1], "V3 subject application ID")
        _bytes32(payload[2], "V3 subject theorem record ID")
        _validate_context_application_members_v2(payload[3])
    else:
        if len(payload) != 8:
            _fail("V3 supersession subject payload must contain eight fields")
        if payload[0] != subject_kind:
            _fail("V3 supersession subject kind marker is inconsistent")
        _bytes32(payload[1], "V3 subject supersession ID")
        _bytes32(payload[2], "V3 subject superseded record ID")
        if payload[3] is not None:
            _bytes32(payload[3], "V3 subject replacement record ID")
        if payload[4] != "context_application_v2_record":
            _fail("V3 subject superseded kind is not closed")
        if payload[3] is None:
            if payload[5] is not None:
                _fail("V3 revocation subject replacement kind must be null")
        elif payload[5] != "context_application_v2_record":
            _fail("V3 subject replacement kind must match")
        if payload[6] == SupersessionReason.AUTHORITY_REVOCATION.value:
            _fail("V3 revocation cannot carry a replacement")
        _enum(payload[6], tuple(reason.value for reason in SupersessionReason), "V3 subject reason")
        _validate_nonempty_evidence_refs(payload[7], "V3 subject supersession evidence")


@dataclass(frozen=True)
class ReviewAcceptanceEventInputV3:
    subject_kind: AcceptanceSubjectKindV3
    subject_payload_digest_reference: DigestReferenceV1
    reviewer_roster_ref: ReviewerRosterRefV1
    reviewer_role_bindings: tuple[ReviewerRoleBindingV1, ...]
    review_mode: ReviewMode
    source_binding_digests: tuple[ContextAuthoritySourceBindingV2, ...]
    review_evidence_refs: tuple[AcceptanceEvidenceRefV1, ...]

    def __post_init__(self) -> None:
        if not isinstance(self.subject_kind, AcceptanceSubjectKindV3):
            raise AuthorityContractError("V3 acceptance subject kind is not closed")
        if self.subject_payload_digest_reference.semantic_domain != ACCEPTANCE_SUBJECT_SCHEMA_V3:
            raise AuthorityContractError("V3 subject digest has the wrong semantic domain")
        if (
            self.subject_payload_digest_reference.input_schema_id
            != ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V3
        ):
            raise AuthorityContractError("V3 subject digest has the wrong input schema")
        if not self.reviewer_role_bindings:
            raise AuthorityContractError("V3 acceptance requires reviewer role bindings")
        if any(
            not isinstance(binding, ReviewerRoleBindingV1)
            for binding in self.reviewer_role_bindings
        ):
            raise AuthorityContractError("V3 reviewer role binding is not V1")
        reviewer_ids = tuple(binding.reviewer_id for binding in self.reviewer_role_bindings)
        if reviewer_ids != tuple(sorted(reviewer_ids)) or len(set(reviewer_ids)) != len(
            reviewer_ids
        ):
            raise AuthorityContractError("V3 reviewer bindings must be sorted and duplicate-free")
        if not self.source_binding_digests:
            raise AuthorityContractError("V3 acceptance requires source bindings")
        if any(
            not isinstance(binding, ContextAuthoritySourceBindingV2)
            for binding in self.source_binding_digests
        ):
            raise AuthorityContractError("V3 source binding is not the V2 type")
        _typed_canonical_items(self.source_binding_digests, "V3 source bindings", allow_empty=False)
        source_keys = [
            (binding.artifact_role, binding.path) for binding in self.source_binding_digests
        ]
        if len(set(source_keys)) != len(source_keys):
            raise AuthorityContractError("V3 source bindings must not duplicate a role/path pair")
        if any(
            not isinstance(evidence, AcceptanceEvidenceRefV1)
            for evidence in self.review_evidence_refs
        ):
            raise AuthorityContractError("V3 review evidence is not a V1 acceptance reference")
        _typed_canonical_items(self.review_evidence_refs, "V3 review evidence", allow_empty=False)

    def semantic_input(self) -> list[AuthorityValue]:
        return [
            ACCEPTANCE_EVENT_INPUT_SCHEMA_V3,
            self.subject_kind.value,
            self.subject_payload_digest_reference.to_cbor(),
            "human_accepted",
            self.reviewer_roster_ref.to_cbor(),
            [binding.to_cbor() for binding in self.reviewer_role_bindings],
            self.review_mode.value,
            ACCEPTANCE_CHECKLIST_V2,
            [binding.to_cbor() for binding in self.source_binding_digests],
            [evidence.to_cbor() for evidence in self.review_evidence_refs],
        ]

    def identity(self) -> AuthorityIdentityV1:
        return compute_authority_identity(
            AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT_V3,
            self.semantic_input(),
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return self.semantic_input()


def _validate_acceptance_event_v3_input(values: list[object]) -> None:
    if len(values) != 10:
        _fail("V3 acceptance event input must contain ten fields")
    _text(values[0], "V3 acceptance event schema")
    _enum(
        values[1],
        tuple(kind.value for kind in AcceptanceSubjectKindV3),
        "V3 acceptance event subject kind",
    )
    subject_reference = DigestReferenceV1.from_cbor(values[2])
    if subject_reference.semantic_domain != ACCEPTANCE_SUBJECT_SCHEMA_V3:
        _fail("V3 acceptance event subject digest has the wrong domain")
    if subject_reference.input_schema_id != ACCEPTANCE_SUBJECT_INPUT_SCHEMA_V3:
        _fail("V3 acceptance event subject digest has the wrong schema")
    if values[3] != "human_accepted":
        _fail("V3 acceptance event decision is not human_accepted")
    _validate_roster_ref_array(values[4])
    bindings = _array(values[5], "V3 reviewer role bindings")
    if not bindings:
        _fail("V3 reviewer role bindings must be non-empty")
    reviewer_ids: list[str] = []
    for binding in bindings:
        _validate_role_binding(binding)
        reviewer_ids.append(cast(str, _array(binding, "V3 reviewer role binding")[0]))
    if reviewer_ids != sorted(reviewer_ids) or len(set(reviewer_ids)) != len(reviewer_ids):
        _fail("V3 reviewer role bindings must be sorted and duplicate-free")
    _enum(values[6], ("multi_reviewer", "solo_separate_self_review"), "V3 review mode")
    if values[7] != ACCEPTANCE_CHECKLIST_V2:
        _fail("V3 acceptance checklist is not the V2 contract")
    _validate_context_source_bindings(values[8], "V3 acceptance source bindings")
    if not _array(values[8], "V3 acceptance source bindings"):
        _fail("V3 acceptance source bindings must be non-empty")
    _validate_acceptance_evidence_refs(values[9])


@dataclass(frozen=True)
class ReviewAcceptanceEventLeafV3:
    event_id: AuthorityIdentityV1
    subject_kind: AcceptanceSubjectKindV3
    subject_payload_digest_reference: DigestReferenceV1
    reviewer_roster_ref: ReviewerRosterRefV1
    reviewer_role_bindings: tuple[ReviewerRoleBindingV1, ...]
    review_mode: ReviewMode
    source_binding_digests: tuple[ContextAuthoritySourceBindingV2, ...]
    review_evidence_refs: tuple[AcceptanceEvidenceRefV1, ...]

    @classmethod
    def from_input(cls, event: ReviewAcceptanceEventInputV3) -> ReviewAcceptanceEventLeafV3:
        return cls(
            event_id=event.identity(),
            subject_kind=event.subject_kind,
            subject_payload_digest_reference=event.subject_payload_digest_reference,
            reviewer_roster_ref=event.reviewer_roster_ref,
            reviewer_role_bindings=event.reviewer_role_bindings,
            review_mode=event.review_mode,
            source_binding_digests=event.source_binding_digests,
            review_evidence_refs=event.review_evidence_refs,
        )

    def __post_init__(self) -> None:
        if self.event_id.kind is not AuthorityIdentityKind.REVIEW_ACCEPTANCE_EVENT_V3:
            raise AuthorityContractError("V3 acceptance event ID has the wrong kind")
        if self.as_input().identity() != self.event_id:
            raise AuthorityContractError("V3 acceptance event ID does not match its input")

    def as_input(self) -> ReviewAcceptanceEventInputV3:
        return ReviewAcceptanceEventInputV3(
            subject_kind=self.subject_kind,
            subject_payload_digest_reference=self.subject_payload_digest_reference,
            reviewer_roster_ref=self.reviewer_roster_ref,
            reviewer_role_bindings=self.reviewer_role_bindings,
            review_mode=self.review_mode,
            source_binding_digests=self.source_binding_digests,
            review_evidence_refs=self.review_evidence_refs,
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return self.as_input().semantic_input()

    def to_wire(self) -> dict[str, object]:
        event = self.as_input()
        return {
            "event_id": self.event_id.as_text(),
            "schema": ACCEPTANCE_EVENT_SCHEMA_V3,
            "subject_kind": event.subject_kind.value,
            "subject_payload_digest": event.subject_payload_digest_reference.to_wire(),
            "decision": "human_accepted",
            "reviewer_roster_ref": event.reviewer_roster_ref.to_wire(),
            "reviewer_role_bindings": [
                binding.to_wire() for binding in event.reviewer_role_bindings
            ],
            "review_mode": event.review_mode.value,
            "checklist_id": ACCEPTANCE_CHECKLIST_V2,
            "source_binding_digests": [
                binding.to_wire() for binding in event.source_binding_digests
            ],
            "review_evidence_refs": [evidence.to_wire() for evidence in event.review_evidence_refs],
        }


@dataclass(frozen=True)
class ApplicationHostBindingV2:
    application_kind: str
    application_semantic_id: AuthorityIdentityV1
    host_binding_claim_ids: tuple[str, ...]

    def __post_init__(self) -> None:
        if self.application_kind != "context_application":
            raise AuthorityContractError("V2 application host binding kind is not closed")
        if self.application_semantic_id.kind is not AuthorityIdentityKind.CONTEXT_APPLICATION_V2:
            raise AuthorityContractError("V2 application host binding ID has the wrong kind")
        if not self.host_binding_claim_ids:
            raise AuthorityContractError("V2 application host binding claims must be non-empty")
        if any(
            re.fullmatch(r"hbc\.v1/[0-9a-f]{64}", claim) is None
            for claim in self.host_binding_claim_ids
        ):
            raise AuthorityContractError("V2 host binding claim ID is not a V1 identity")
        encoded = [encode_canonical(claim) for claim in self.host_binding_claim_ids]
        if encoded != sorted(encoded) or len(set(encoded)) != len(encoded):
            raise AuthorityContractError("V2 host binding claims must be sorted and duplicate-free")

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.application_kind,
            self.application_semantic_id.as_text(),
            list(self.host_binding_claim_ids),
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "application_kind": self.application_kind,
            "application_semantic_id": self.application_semantic_id.as_text(),
            "host_binding_claim_ids": list(self.host_binding_claim_ids),
        }


@dataclass(frozen=True)
class ContextApplicationAuthorityV2:
    base_authority_v1_binding: ContextAuthoritySourceBindingV2
    host_binding_authority_v2_binding: ContextAuthoritySourceBindingV2 | None
    candidate_universe_binding: ContextAuthoritySourceBindingV2
    source_bindings: tuple[ContextAuthoritySourceBindingV2, ...]
    context_application_v2_records: tuple[ContextApplicationV2Record, ...]
    context_application_v2_supersession_records: tuple[ContextApplicationV2SupersessionRecord, ...]
    application_host_bindings_v2: tuple[ApplicationHostBindingV2, ...]

    def __post_init__(self) -> None:
        if any(
            not isinstance(binding, ContextAuthoritySourceBindingV2)
            for binding in self.source_bindings
        ):
            raise AuthorityContractError("V2 authority source binding has the wrong type")
        _typed_canonical_items(self.source_bindings, "V2 authority source bindings")
        keys = [(binding.artifact_role, binding.path) for binding in self.source_bindings]
        if len(set(keys)) != len(keys):
            raise AuthorityContractError("V2 authority source bindings must be unique")
        if any(
            not isinstance(record, ContextApplicationV2Record)
            for record in self.context_application_v2_records
        ):
            raise AuthorityContractError("V2 authority application record has the wrong type")
        if any(
            not isinstance(record, ContextApplicationV2SupersessionRecord)
            for record in self.context_application_v2_supersession_records
        ):
            raise AuthorityContractError("V2 authority supersession record has the wrong type")
        if any(
            not isinstance(binding, ApplicationHostBindingV2)
            for binding in self.application_host_bindings_v2
        ):
            raise AuthorityContractError("V2 application host binding has the wrong type")
        _typed_canonical_items(
            self.context_application_v2_records, "V2 authority application records"
        )
        _typed_canonical_items(
            self.context_application_v2_supersession_records,
            "V2 authority supersession records",
        )
        _typed_canonical_items(
            self.application_host_bindings_v2, "V2 authority application host bindings"
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "schema": CONTEXT_AUTHORITY_SCHEMA_V2,
            "base_authority_v1_binding": self.base_authority_v1_binding.to_wire(),
            "host_binding_authority_v2_binding": (
                None
                if self.host_binding_authority_v2_binding is None
                else self.host_binding_authority_v2_binding.to_wire()
            ),
            "candidate_universe_binding": self.candidate_universe_binding.to_wire(),
            "source_bindings": [binding.to_wire() for binding in self.source_bindings],
            "context_application_v2_records": [
                record.to_wire() for record in self.context_application_v2_records
            ],
            "context_application_v2_supersession_records": [
                record.to_wire() for record in self.context_application_v2_supersession_records
            ],
            "application_host_bindings_v2": [
                binding.to_wire() for binding in self.application_host_bindings_v2
            ],
        }
