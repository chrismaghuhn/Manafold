"""Typed, rules-neutral host-binding contracts for M2.5.C.

This module owns the new host-binding V2 underlay only.  It does not alter
the accepted V1 authority contracts, resolve source bytes, classify C
candidates, or decide relation/domain/context semantics.
"""

from __future__ import annotations

import re
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from enum import StrEnum
from typing import Final, TypeAlias, cast

from .authority import (
    AcceptanceEvidenceRefV1,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewMode,
)
from .persistence import (
    CANONICAL_CBOR_ID,
    DIGEST_ENVELOPE_ID,
    SHA256_ID,
    PersistenceValue,
    encode_canonical,
    encode_envelope,
    hash_envelope,
)

HOST_BINDING_AUTHORITY_SCHEMA_V2: Final = "manafold.m2.5.c.interaction-review-authority.v2"
HOST_BINDING_CLAIM_SCHEMA_V1: Final = "manafold.m2.5.c.cross-deck-host-binding-claim.v1"
HOST_BINDING_CLAIM_INPUT_SCHEMA_V1: Final = "manafold.m2.5.c.cross-deck-host-binding-claim-input.v1"
HOST_BINDING_CLAIM_RECORD_SCHEMA_V1: Final = (
    "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1"
)
HOST_BINDING_CLAIM_RECORD_INPUT_SCHEMA_V1: Final = (
    "manafold.m2.5.c.cross-deck-host-binding-claim-record-input.v1"
)
HOST_BINDING_SUPERSESSION_INPUT_SCHEMA_V1: Final = (
    "manafold.m2.5.c.cross-deck-host-binding-supersession-input.v1"
)
HOST_BINDING_ACCEPTANCE_EVENT_SCHEMA_V2: Final = "manafold.m2.5.c.review-acceptance-event.v2"
HOST_BINDING_ACCEPTANCE_SUBJECT_SCHEMA_V2: Final = "manafold.m2.5.c.acceptance-subject-payload.v2"
HOST_BINDING_CHECKLIST_V1: Final = "cross-deck-host-binding-review-checklist.v1"
HOST_BINDING_ACCEPTANCE_EVENT_INPUT_SCHEMA_V2: Final = (
    "manafold.m2.5.c.review-acceptance-event-input.v2"
)
HOST_BINDING_ACCEPTANCE_SUBJECT_KINDS: Final = (
    "cross_deck_host_binding_claim_record_v1",
    "cross_deck_host_binding_claim_supersession_v1",
)

DISCOVERY_SIDES: Final = ("rev3_left_family", "rev3_right_family")
HOST_RELATIONSHIPS: Final = ("same_host", "cross_host")
APPLICATION_KINDS: Final = (
    "relation_application",
    "domain_application",
    "context_application",
)

V2_SOURCE_ROLES: Final = (
    "base_authority_v1",
    "candidate_universe",
    "declared_model",
    "rev3_candidate_census",
    "rev3_pair_aggregates",
    "rev3_card_requirement_map",
    "rev3_deck_row_source_resolution",
    "rev3_osi_source_records",
    "rev3_source_index",
    "b2_catalog",
    "b2_classifications",
    "b2_closure",
    "reviewer_roster_leaf",
    "acceptance_event_leaf_v2",
    "host_binding_claim_record",
)

_SOURCE_ROLE_PATHS: Final[dict[str, tuple[str, str | None]]] = {
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
    "reviewer_roster_leaf": (
        "sources/m2_5/authorities/reviewer_rosters/v1/<sha256>.json",
        "manafold.m2.5.c.reviewer-roster.v1",
    ),
    "acceptance_event_leaf_v2": (
        "sources/m2_5/authorities/review_acceptance_events/v2/<sha256>.json",
        "manafold.m2.5.c.review-acceptance-event.v2",
    ),
    "host_binding_claim_record": (
        "sources/m2_5/authorities/cross_deck_host_binding_claims/v1/<sha256>.json",
        HOST_BINDING_CLAIM_RECORD_SCHEMA_V1,
    ),
}

_HEX64_RE: Final = re.compile(r"^[0-9a-f]{64}$")
_IDENTITY_RE: Final = re.compile(r"^(rpa|dpa|cpa)\.v1/[0-9a-f]{64}$")
_CLAIM_ID_RE: Final = re.compile(r"^hbc\.v1/[0-9a-f]{64}$")

AuthorityValue: TypeAlias = PersistenceValue
LocatorV2: TypeAlias = tuple[str, str | int | None]


class HostBindingContractError(ValueError):
    """Raised when a host-binding value violates its closed contract."""


class HostBindingIdentityKind(StrEnum):
    CROSS_DECK_HOST_BINDING_CLAIM = "cross_deck_host_binding_claim"
    CROSS_DECK_HOST_BINDING_CLAIM_RECORD = "cross_deck_host_binding_claim_record"
    CROSS_DECK_HOST_BINDING_SUPERSESSION = "cross_deck_host_binding_supersession"
    REVIEW_ACCEPTANCE_EVENT_V2 = "review_acceptance_event_v2"


@dataclass(frozen=True)
class HostBindingIdentityV1:
    kind: HostBindingIdentityKind
    digest_bytes: bytes

    def __post_init__(self) -> None:
        if not isinstance(self.kind, HostBindingIdentityKind):
            raise HostBindingContractError("host-binding identity kind is not closed")
        _bytes32(self.digest_bytes, "host-binding identity digest")

    @property
    def prefix(self) -> str:
        return _IDENTITY_SPECS[self.kind][0]

    @property
    def semantic_domain(self) -> str:
        return _IDENTITY_SPECS[self.kind][1]

    @property
    def input_schema_id(self) -> str:
        return _IDENTITY_SPECS[self.kind][2]

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

    def to_wire(self) -> dict[str, object]:
        return {
            "envelope_id": DIGEST_ENVELOPE_ID,
            "algorithm_id": SHA256_ID,
            "semantic_domain": self.semantic_domain,
            "payload_codec_id": CANONICAL_CBOR_ID,
            "input_schema_id": self.input_schema_id,
            "digest_hex": self.digest_bytes.hex(),
        }


_IDENTITY_SPECS: Final[dict[HostBindingIdentityKind, tuple[str, str, str]]] = {
    HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_CLAIM: (
        "hbc.v1/",
        "manafold.m2.5.c.cross-deck-host-binding-claim.v1",
        HOST_BINDING_CLAIM_INPUT_SCHEMA_V1,
    ),
    HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_CLAIM_RECORD: (
        "hbcr.v1/",
        "manafold.m2.5.c.cross-deck-host-binding-claim-record.v1",
        HOST_BINDING_CLAIM_RECORD_INPUT_SCHEMA_V1,
    ),
    HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_SUPERSESSION: (
        "hbcs.v1/",
        "manafold.m2.5.c.cross-deck-host-binding-supersession.v1",
        HOST_BINDING_SUPERSESSION_INPUT_SCHEMA_V1,
    ),
    HostBindingIdentityKind.REVIEW_ACCEPTANCE_EVENT_V2: (
        "ae.v2/",
        "manafold.m2.5.c.review-acceptance-event.v2",
        HOST_BINDING_ACCEPTANCE_EVENT_INPUT_SCHEMA_V2,
    ),
}


def _nonempty_text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise HostBindingContractError(f"{label} must be non-empty text")
    return value


def _bytes32(value: object, label: str) -> bytes:
    if not isinstance(value, bytes) or len(value) != 32:
        raise HostBindingContractError(f"{label} must contain exactly 32 bytes")
    return value


def _u32(value: object, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**32 - 1:
        raise HostBindingContractError(f"{label} must be a u32")
    return value


def _relative_path(value: object, label: str) -> str:
    path = _nonempty_text(value, label)
    if (
        path.startswith(("/", "\\"))
        or "\\" in path
        or ":" in path.split("/", 1)[0]
        or "://" in path
        or any(part in {"", ".", ".."} for part in path.split("/"))
    ):
        raise HostBindingContractError(f"{label} must be a repository-relative slash path")
    return path


def _canonical_unique(values: Sequence[object], label: str) -> tuple[object, ...]:
    encoded = [encode_canonical(cast(PersistenceValue, value)) for value in values]
    if encoded != sorted(encoded):
        raise HostBindingContractError(f"{label} must be in canonical order")
    if len(set(encoded)) != len(encoded):
        raise HostBindingContractError(f"{label} must be duplicate-free")
    return tuple(values)


def _identity_from_payload(
    kind: HostBindingIdentityKind, payload: list[AuthorityValue]
) -> HostBindingIdentityV1:
    expected_schema = _IDENTITY_SPECS[kind][2]
    if not payload or payload[0] != expected_schema:
        raise HostBindingContractError("host-binding identity input schema is incorrect")
    expected_arity = {
        HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_CLAIM: 5,
        HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_CLAIM_RECORD: 3,
        HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_SUPERSESSION: 7,
        HostBindingIdentityKind.REVIEW_ACCEPTANCE_EVENT_V2: 10,
    }[kind]
    if len(payload) != expected_arity:
        raise HostBindingContractError("host-binding identity input has the wrong arity")
    encoded = encode_canonical(payload)
    envelope = encode_envelope(
        _IDENTITY_SPECS[kind][1],
        _IDENTITY_SPECS[kind][2],
        encoded,
    )
    return HostBindingIdentityV1(kind, hash_envelope(envelope))


@dataclass(frozen=True)
class ApplicationMemberKeyV1:
    candidate_id: str
    candidate_identity_digest: bytes
    source_instance_id: str

    def __post_init__(self) -> None:
        _nonempty_text(self.candidate_id, "candidate ID")
        _bytes32(self.candidate_identity_digest, "candidate identity digest")
        _nonempty_text(self.source_instance_id, "source instance ID")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.candidate_id, self.candidate_identity_digest, self.source_instance_id]

    def to_wire(self) -> dict[str, object]:
        return {
            "candidate_id": self.candidate_id,
            "candidate_identity_digest": self.candidate_identity_digest.hex(),
            "source_instance_id": self.source_instance_id,
        }


@dataclass(frozen=True)
class DiscoveryHostRefV1:
    host_kind: str
    host_id: str

    def __post_init__(self) -> None:
        if self.host_kind != "rev3_deck":
            raise HostBindingContractError("discovery host kind is not closed")
        _nonempty_text(self.host_id, "discovery host ID")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.host_kind, self.host_id]

    def to_wire(self) -> dict[str, object]:
        return {"host_kind": self.host_kind, "host_id": self.host_id}


@dataclass(frozen=True)
class HostBindingEvidenceRefV2:
    artifact_role: str
    path: str
    schema_or_null: str | None
    raw_sha256: bytes
    locator: LocatorV2

    def __post_init__(self) -> None:
        if self.artifact_role not in V2_SOURCE_ROLES:
            raise HostBindingContractError("host-binding source role is not closed")
        expected_path, expected_schema = _SOURCE_ROLE_PATHS[self.artifact_role]
        if "<sha256>" not in expected_path and self.path != expected_path:
            raise HostBindingContractError("host-binding source path does not match its role")
        if "<sha256>" in expected_path:
            pattern = re.escape(expected_path).replace("<sha256>", r"[0-9a-f]{64}")
            if re.fullmatch(pattern, self.path) is None:
                raise HostBindingContractError("content-addressed source path is invalid")
        _relative_path(self.path, "host-binding evidence path")
        if expected_schema is not None and self.schema_or_null != expected_schema:
            raise HostBindingContractError("host-binding evidence schema does not match its role")
        if expected_schema is None and self.schema_or_null is not None:
            raise HostBindingContractError("raw REV3 evidence must have a null schema")
        _bytes32(self.raw_sha256, "host-binding evidence digest")
        _validate_locator(self.locator)

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.artifact_role,
            self.path,
            self.schema_or_null,
            self.raw_sha256,
            list(self.locator),
        ]

    def to_wire(self) -> dict[str, object]:
        kind, payload = self.locator
        locator: dict[str, object] = {"kind": kind}
        if payload is not None:
            locator["value"] = payload
        return {
            "artifact_role": self.artifact_role,
            "path": self.path,
            "schema_or_null": self.schema_or_null,
            "raw_sha256": self.raw_sha256.hex(),
            "locator": locator,
        }


def _validate_locator(value: object) -> LocatorV2:
    if not isinstance(value, tuple) or len(value) != 2:
        raise HostBindingContractError("host-binding locator must be a tagged tuple")
    kind, payload = value
    if kind == "whole_artifact":
        if payload is not None:
            raise HostBindingContractError("whole_artifact locator payload must be null")
    elif kind in {"csv_row", "jsonl_line"}:
        _u32(payload, f"{kind} locator index")
    elif kind == "jsonl_record":
        _nonempty_text(payload, "jsonl record locator")
    elif kind == "json_pointer":
        pointer = _nonempty_text(payload, "JSON Pointer locator")
        if not pointer.startswith("/"):
            raise HostBindingContractError("JSON Pointer locator must begin with '/'")
    else:
        raise HostBindingContractError("host-binding locator kind is not closed")
    return cast(LocatorV2, value)


@dataclass(frozen=True)
class CrossDeckParticipantDiscoveryHostBindingV1:
    member_key: ApplicationMemberKeyV1
    participant_position: int
    participant_ref: str
    discovery_side: str
    discovery_host: DiscoveryHostRefV1
    mapping_evidence_refs: tuple[HostBindingEvidenceRefV2, ...]

    def __post_init__(self) -> None:
        if not isinstance(self.member_key, ApplicationMemberKeyV1):
            raise HostBindingContractError("discovery binding member key is invalid")
        _u32(self.participant_position, "participant position")
        _nonempty_text(self.participant_ref, "participant reference")
        if self.discovery_side not in DISCOVERY_SIDES:
            raise HostBindingContractError("discovery side is not closed")
        if not isinstance(self.discovery_host, DiscoveryHostRefV1):
            raise HostBindingContractError("discovery host is invalid")
        if not self.mapping_evidence_refs:
            raise HostBindingContractError("discovery mapping evidence must be non-empty")
        if any(
            not isinstance(reference, HostBindingEvidenceRefV2)
            or reference.artifact_role != "rev3_card_requirement_map"
            for reference in self.mapping_evidence_refs
        ):
            raise HostBindingContractError("discovery mappings must use the REV3 map role")
        _canonical_unique(
            [reference.to_cbor() for reference in self.mapping_evidence_refs],
            "discovery mapping evidence",
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.member_key.to_cbor(),
            self.participant_position,
            self.participant_ref,
            self.discovery_side,
            self.discovery_host.to_cbor(),
            [reference.to_cbor() for reference in self.mapping_evidence_refs],
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "member_key": self.member_key.to_wire(),
            "participant_position": self.participant_position,
            "participant_ref": self.participant_ref,
            "discovery_side": self.discovery_side,
            "discovery_host": self.discovery_host.to_wire(),
            "mapping_evidence_refs": [
                reference.to_wire() for reference in self.mapping_evidence_refs
            ],
        }


@dataclass(frozen=True)
class HostRealizationWitnessV1:
    discovery_mapping_ref: HostBindingEvidenceRefV2
    deck_row_ref: HostBindingEvidenceRefV2
    osi_ref: HostBindingEvidenceRefV2
    b2_assignment_refs: tuple[HostBindingEvidenceRefV2, ...]

    def __post_init__(self) -> None:
        if self.discovery_mapping_ref.artifact_role != "rev3_card_requirement_map":
            raise HostBindingContractError("witness discovery mapping role is invalid")
        if self.deck_row_ref.artifact_role != "rev3_deck_row_source_resolution":
            raise HostBindingContractError("witness deck row role is invalid")
        if self.osi_ref.artifact_role != "rev3_osi_source_records":
            raise HostBindingContractError("witness OSI role is invalid")
        if not self.b2_assignment_refs:
            raise HostBindingContractError("witness B2 assignments must be non-empty")
        if any(
            reference.artifact_role != "b2_classifications" for reference in self.b2_assignment_refs
        ):
            raise HostBindingContractError("witness B2 assignment role is invalid")
        _canonical_unique(
            [reference.to_cbor() for reference in self.b2_assignment_refs],
            "witness B2 assignments",
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.discovery_mapping_ref.to_cbor(),
            self.deck_row_ref.to_cbor(),
            self.osi_ref.to_cbor(),
            [reference.to_cbor() for reference in self.b2_assignment_refs],
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "discovery_mapping_ref": self.discovery_mapping_ref.to_wire(),
            "deck_row_ref": self.deck_row_ref.to_wire(),
            "osi_ref": self.osi_ref.to_wire(),
            "b2_assignment_refs": [reference.to_wire() for reference in self.b2_assignment_refs],
        }


@dataclass(frozen=True)
class ParticipantHostRealizationV1:
    member_key: ApplicationMemberKeyV1
    participant_position: int
    participant_ref: str
    host: DiscoveryHostRefV1
    witnesses: tuple[HostRealizationWitnessV1, ...]

    def __post_init__(self) -> None:
        if not isinstance(self.member_key, ApplicationMemberKeyV1):
            raise HostBindingContractError("realization member key is invalid")
        _u32(self.participant_position, "realization participant position")
        _nonempty_text(self.participant_ref, "realization participant reference")
        if not isinstance(self.host, DiscoveryHostRefV1):
            raise HostBindingContractError("realization host is invalid")
        if not self.witnesses:
            raise HostBindingContractError("realization witnesses must be non-empty")
        if any(not isinstance(witness, HostRealizationWitnessV1) for witness in self.witnesses):
            raise HostBindingContractError("realization witness is invalid")
        _canonical_unique(
            [witness.to_cbor() for witness in self.witnesses],
            "realization witnesses",
        )

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.member_key.to_cbor(),
            self.participant_position,
            self.participant_ref,
            self.host.to_cbor(),
            [witness.to_cbor() for witness in self.witnesses],
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "member_key": self.member_key.to_wire(),
            "participant_position": self.participant_position,
            "participant_ref": self.participant_ref,
            "host": self.host.to_wire(),
            "witnesses": [witness.to_wire() for witness in self.witnesses],
        }


@dataclass(frozen=True)
class CrossDeckHostBindingClaimV1:
    member_key: ApplicationMemberKeyV1
    discovery_bindings: tuple[CrossDeckParticipantDiscoveryHostBindingV1, ...]
    participant_host_realizations: tuple[ParticipantHostRealizationV1, ...]
    observed_host_relationship: str

    def __post_init__(self) -> None:
        if not isinstance(self.member_key, ApplicationMemberKeyV1):
            raise HostBindingContractError("claim member key is invalid")
        if not self.discovery_bindings:
            raise HostBindingContractError("claim discovery bindings must be non-empty")
        if not self.participant_host_realizations:
            raise HostBindingContractError("claim realizations must be non-empty")
        if self.observed_host_relationship not in HOST_RELATIONSHIPS:
            raise HostBindingContractError("observed host relationship is not closed")

        binding_positions = [item.participant_position for item in self.discovery_bindings]
        realization_positions = [
            item.participant_position for item in self.participant_host_realizations
        ]
        expected_positions = list(range(len(binding_positions)))
        if binding_positions != expected_positions or realization_positions != expected_positions:
            raise HostBindingContractError(
                "claim participant positions must be complete and in position order"
            )
        if len(set(binding_positions)) != len(binding_positions):
            raise HostBindingContractError("claim discovery positions must be unique")
        if len(set(realization_positions)) != len(realization_positions):
            raise HostBindingContractError("claim realization positions must be unique")

        for binding, realization in zip(
            self.discovery_bindings, self.participant_host_realizations, strict=True
        ):
            if binding.member_key != self.member_key or realization.member_key != self.member_key:
                raise HostBindingContractError("claim member key is not repeated exactly")
            if binding.participant_ref != realization.participant_ref:
                raise HostBindingContractError("claim participant reference differs")
            if binding.discovery_host != realization.host:
                raise HostBindingContractError(
                    "participant realization host differs from discovery host"
                )
            discovery_refs = {
                encode_canonical(reference.to_cbor()) for reference in binding.mapping_evidence_refs
            }
            witness_refs = {
                encode_canonical(witness.discovery_mapping_ref.to_cbor())
                for witness in realization.witnesses
            }
            if len(witness_refs) != len(realization.witnesses):
                raise HostBindingContractError(
                    "one discovery mapping cannot back multiple witnesses"
                )
            if discovery_refs != witness_refs:
                raise HostBindingContractError(
                    "witness discovery mappings do not exactly cover discovery bindings"
                )

        hosts = {
            realization.host.to_cbor()[1] for realization in self.participant_host_realizations
        }
        expected_relationship = "same_host" if len(hosts) == 1 else "cross_host"
        if self.observed_host_relationship != expected_relationship:
            raise HostBindingContractError(
                "observed host relationship differs from selected realizations"
            )

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.member_key.to_cbor(),
            [binding.to_cbor() for binding in self.discovery_bindings],
            [realization.to_cbor() for realization in self.participant_host_realizations],
            self.observed_host_relationship,
        ]

    def semantic_input(self) -> list[AuthorityValue]:
        return [
            HOST_BINDING_CLAIM_INPUT_SCHEMA_V1,
            self.member_key.to_cbor(),
            [binding.to_cbor() for binding in self.discovery_bindings],
            [realization.to_cbor() for realization in self.participant_host_realizations],
            self.observed_host_relationship,
        ]

    def identity(self) -> HostBindingIdentityV1:
        return _identity_from_payload(
            HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_CLAIM,
            self.semantic_input(),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "claim_id": self.identity().as_text(),
            "member_key": self.member_key.to_wire(),
            "discovery_bindings": [binding.to_wire() for binding in self.discovery_bindings],
            "participant_host_realizations": [
                realization.to_wire() for realization in self.participant_host_realizations
            ],
            "observed_host_relationship": self.observed_host_relationship,
        }


@dataclass(frozen=True)
class ApplicationHostBindingV1:
    application_kind: str
    application_semantic_id: str
    host_binding_claim_ids: tuple[str, ...]

    def __post_init__(self) -> None:
        if self.application_kind not in APPLICATION_KINDS:
            raise HostBindingContractError("application kind is not closed")
        if _IDENTITY_RE.fullmatch(self.application_semantic_id) is None:
            raise HostBindingContractError("application semantic ID is not a V1 application ID")
        expected_prefix = {
            "relation_application": "rpa.v1/",
            "domain_application": "dpa.v1/",
            "context_application": "cpa.v1/",
        }[self.application_kind]
        if not self.application_semantic_id.startswith(expected_prefix):
            raise HostBindingContractError("application semantic ID does not match its kind")
        if not self.host_binding_claim_ids:
            raise HostBindingContractError("application host-binding claims must be non-empty")
        if any(_CLAIM_ID_RE.fullmatch(value) is None for value in self.host_binding_claim_ids):
            raise HostBindingContractError("host-binding claim ID is invalid")
        if tuple(sorted(self.host_binding_claim_ids)) != self.host_binding_claim_ids:
            raise HostBindingContractError("host-binding claim IDs must be canonical")
        if len(set(self.host_binding_claim_ids)) != len(self.host_binding_claim_ids):
            raise HostBindingContractError("host-binding claim IDs must be duplicate-free")

    def to_cbor(self) -> list[AuthorityValue]:
        return [
            self.application_kind,
            self.application_semantic_id,
            list(self.host_binding_claim_ids),
        ]

    def to_wire(self) -> dict[str, object]:
        return {
            "application_kind": self.application_kind,
            "application_semantic_id": self.application_semantic_id,
            "host_binding_claim_ids": list(self.host_binding_claim_ids),
        }


@dataclass(frozen=True)
class HostBindingAcceptanceEventRefV2:
    path: str
    raw_sha256: bytes
    event_id: str

    def __post_init__(self) -> None:
        if re.fullmatch(r"ae\.v2/[0-9a-f]{64}", self.event_id) is None:
            raise HostBindingContractError("V2 acceptance event ID is invalid")
        expected_path = (
            "sources/m2_5/authorities/review_acceptance_events/v2/"
            + self.event_id.removeprefix("ae.v2/")
            + ".json"
        )
        if self.path != expected_path:
            raise HostBindingContractError("V2 acceptance event path is not ID-bound")
        _bytes32(self.raw_sha256, "V2 acceptance event digest")

    def to_cbor(self) -> list[AuthorityValue]:
        return [self.path, self.raw_sha256, ["event_id", self.event_id]]

    def to_wire(self) -> dict[str, object]:
        return {
            "path": self.path,
            "raw_sha256": self.raw_sha256.hex(),
            "event_id": self.event_id,
        }


@dataclass(frozen=True)
class HostBindingSourceBindingV2:
    artifact_role: str
    path: str
    schema_or_null: str | None
    raw_sha256: bytes

    def __post_init__(self) -> None:
        if self.artifact_role not in V2_SOURCE_ROLES:
            raise HostBindingContractError("V2 source role is not closed")
        expected_path, expected_schema = _SOURCE_ROLE_PATHS[self.artifact_role]
        if "<sha256>" not in expected_path and self.path != expected_path:
            raise HostBindingContractError("V2 source path does not match its role")
        if "<sha256>" in expected_path:
            pattern = re.escape(expected_path).replace("<sha256>", r"[0-9a-f]{64}")
            if re.fullmatch(pattern, self.path) is None:
                raise HostBindingContractError("V2 content-addressed source path is invalid")
        _relative_path(self.path, "V2 source path")
        if expected_schema is not None and self.schema_or_null != expected_schema:
            raise HostBindingContractError("V2 source schema does not match its role")
        if expected_schema is None and self.schema_or_null is not None:
            raise HostBindingContractError("raw V2 REV3 source schema must be null")
        _bytes32(self.raw_sha256, "V2 source digest")

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
class HostBindingAcceptanceEventInputV2:
    subject_kind: str
    subject_payload_digest: bytes
    reviewer_roster_ref: ReviewerRosterRefV1
    reviewer_role_bindings: tuple[ReviewerRoleBindingV1, ...]
    review_mode: ReviewMode
    checklist_id: str
    source_binding_digests: tuple[HostBindingSourceBindingV2, ...]
    review_evidence_refs: tuple[AcceptanceEvidenceRefV1, ...]

    def __post_init__(self) -> None:
        if self.subject_kind not in HOST_BINDING_ACCEPTANCE_SUBJECT_KINDS:
            raise HostBindingContractError("V2 host-binding acceptance subject is not closed")
        _bytes32(self.subject_payload_digest, "V2 acceptance subject digest")
        if not isinstance(self.reviewer_roster_ref, ReviewerRosterRefV1):
            raise HostBindingContractError("V2 reviewer roster reference is invalid")
        if not self.reviewer_role_bindings:
            raise HostBindingContractError("V2 acceptance reviewer bindings are empty")
        reviewer_ids = tuple(binding.reviewer_id for binding in self.reviewer_role_bindings)
        if reviewer_ids != tuple(sorted(reviewer_ids)) or len(set(reviewer_ids)) != len(
            reviewer_ids
        ):
            raise HostBindingContractError("V2 reviewer bindings are not canonical")
        if self.review_mode not in {
            ReviewMode.MULTI_REVIEWER,
            ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
        }:
            raise HostBindingContractError("V2 review mode is not closed")
        if self.checklist_id != HOST_BINDING_CHECKLIST_V1:
            raise HostBindingContractError("V2 host-binding checklist is not admitted")
        if not self.source_binding_digests:
            raise HostBindingContractError("V2 acceptance source bindings are empty")
        _canonical_unique(
            [binding.to_cbor() for binding in self.source_binding_digests],
            "V2 acceptance source bindings",
        )
        roles = {binding.artifact_role for binding in self.source_binding_digests}
        if not {"declared_model", "reviewer_roster_leaf"}.issubset(roles):
            raise HostBindingContractError("V2 acceptance lacks model or roster binding")
        if not self.review_evidence_refs:
            raise HostBindingContractError("V2 acceptance evidence is empty")
        _canonical_unique(
            [reference.to_cbor() for reference in self.review_evidence_refs],
            "V2 acceptance evidence",
        )

    def semantic_input(self) -> list[AuthorityValue]:
        return [
            HOST_BINDING_ACCEPTANCE_EVENT_INPUT_SCHEMA_V2,
            self.subject_kind,
            self.subject_payload_digest,
            "human_accepted",
            self.reviewer_roster_ref.to_cbor(),
            [binding.to_cbor() for binding in self.reviewer_role_bindings],
            self.review_mode.value,
            self.checklist_id,
            [binding.to_cbor() for binding in self.source_binding_digests],
            [reference.to_cbor() for reference in self.review_evidence_refs],
        ]

    def identity(self) -> HostBindingIdentityV1:
        return _identity_from_payload(
            HostBindingIdentityKind.REVIEW_ACCEPTANCE_EVENT_V2,
            self.semantic_input(),
        )


@dataclass(frozen=True)
class HostBindingAcceptanceEventLeafV2:
    event_id: HostBindingIdentityV1
    subject_kind: str
    subject_payload_digest: bytes
    reviewer_roster_ref: ReviewerRosterRefV1
    reviewer_role_bindings: tuple[ReviewerRoleBindingV1, ...]
    review_mode: ReviewMode
    checklist_id: str
    source_binding_digests: tuple[HostBindingSourceBindingV2, ...]
    review_evidence_refs: tuple[AcceptanceEvidenceRefV1, ...]

    @classmethod
    def from_input(
        cls, event: HostBindingAcceptanceEventInputV2
    ) -> HostBindingAcceptanceEventLeafV2:
        return cls(
            event_id=event.identity(),
            subject_kind=event.subject_kind,
            subject_payload_digest=event.subject_payload_digest,
            reviewer_roster_ref=event.reviewer_roster_ref,
            reviewer_role_bindings=event.reviewer_role_bindings,
            review_mode=event.review_mode,
            checklist_id=event.checklist_id,
            source_binding_digests=event.source_binding_digests,
            review_evidence_refs=event.review_evidence_refs,
        )

    def __post_init__(self) -> None:
        if self.event_id.kind is not HostBindingIdentityKind.REVIEW_ACCEPTANCE_EVENT_V2:
            raise HostBindingContractError("V2 acceptance event ID has the wrong kind")
        expected = HostBindingAcceptanceEventInputV2(
            subject_kind=self.subject_kind,
            subject_payload_digest=self.subject_payload_digest,
            reviewer_roster_ref=self.reviewer_roster_ref,
            reviewer_role_bindings=self.reviewer_role_bindings,
            review_mode=self.review_mode,
            checklist_id=self.checklist_id,
            source_binding_digests=self.source_binding_digests,
            review_evidence_refs=self.review_evidence_refs,
        ).identity()
        if expected != self.event_id:
            raise HostBindingContractError("V2 acceptance event ID differs from its input")

    def to_wire(self) -> dict[str, object]:
        return {
            "event_id": self.event_id.as_text(),
            "schema": HOST_BINDING_ACCEPTANCE_EVENT_SCHEMA_V2,
            "subject_kind": self.subject_kind,
            "subject_payload_digest": self.subject_payload_digest.hex(),
            "decision": "human_accepted",
            "reviewer_roster_ref": self.reviewer_roster_ref.to_wire(),
            "reviewer_role_bindings": [
                binding.to_wire() for binding in self.reviewer_role_bindings
            ],
            "review_mode": self.review_mode.value,
            "checklist_id": self.checklist_id,
            "source_binding_digests": [
                binding.to_wire() for binding in self.source_binding_digests
            ],
            "review_evidence_refs": [
                reference.to_wire() for reference in self.review_evidence_refs
            ],
        }


@dataclass(frozen=True)
class CrossDeckHostBindingClaimRecordV1:
    claim: CrossDeckHostBindingClaimV1
    acceptance_event_ref: HostBindingAcceptanceEventRefV2

    def record_identity(self) -> HostBindingIdentityV1:
        return _identity_from_payload(
            HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_CLAIM_RECORD,
            [
                HOST_BINDING_CLAIM_RECORD_INPUT_SCHEMA_V1,
                self.claim.identity().digest_bytes,
                self.acceptance_event_ref.to_cbor(),
            ],
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "record_kind": "cross_deck_host_binding_claim_record_v1",
            "record_id": self.record_identity().to_wire(),
            "claim_id": self.claim.identity().to_wire(),
            "claim": self.claim.to_wire(),
            "acceptance": {
                "decision": "human_accepted",
                "review_event_ref": self.acceptance_event_ref.to_wire(),
            },
        }


@dataclass(frozen=True)
class CrossDeckHostBindingClaimSupersessionV1:
    superseded_record_id: HostBindingIdentityV1
    replacement_record_id: HostBindingIdentityV1 | None
    reason_code: str
    source_evidence_refs: tuple[HostBindingEvidenceRefV2, ...]
    acceptance_event_ref: HostBindingAcceptanceEventRefV2

    def __post_init__(self) -> None:
        expected_kind = HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_CLAIM_RECORD
        if self.superseded_record_id.kind is not expected_kind:
            raise HostBindingContractError("superseded host-binding record kind is invalid")
        if (
            self.replacement_record_id is not None
            and self.replacement_record_id.kind is not expected_kind
        ):
            raise HostBindingContractError("replacement host-binding record kind is invalid")
        if self.replacement_record_id == self.superseded_record_id:
            raise HostBindingContractError("host-binding record cannot supersede itself")
        if self.replacement_record_id is None:
            if self.reason_code != "authority_revocation":
                raise HostBindingContractError("null replacement requires authority_revocation")
        elif self.reason_code == "authority_revocation":
            raise HostBindingContractError("authority_revocation requires null replacement")
        if self.reason_code not in {
            "semantic_correction",
            "source_revision",
            "authority_revocation",
        }:
            raise HostBindingContractError("host-binding supersession reason is not closed")
        if not self.source_evidence_refs:
            raise HostBindingContractError("host-binding supersession evidence is empty")
        _canonical_unique(
            [reference.to_cbor() for reference in self.source_evidence_refs],
            "host-binding supersession evidence",
        )

    def semantic_input(self) -> list[AuthorityValue]:
        return [
            HOST_BINDING_SUPERSESSION_INPUT_SCHEMA_V1,
            self.superseded_record_id.digest_bytes,
            (
                None
                if self.replacement_record_id is None
                else self.replacement_record_id.digest_bytes
            ),
            "cross_deck_host_binding_claim_record_v1",
            (
                None
                if self.replacement_record_id is None
                else "cross_deck_host_binding_claim_record_v1"
            ),
            self.reason_code,
            [reference.to_cbor() for reference in self.source_evidence_refs],
        ]

    def identity(self) -> HostBindingIdentityV1:
        return _identity_from_payload(
            HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_SUPERSESSION,
            self.semantic_input(),
        )

    def to_wire(self) -> dict[str, object]:
        return {
            "supersession_id": self.identity().to_wire(),
            "superseded_record_id": self.superseded_record_id.to_wire(),
            "replacement_record_id": (
                None if self.replacement_record_id is None else self.replacement_record_id.to_wire()
            ),
            "superseded_record_kind": "cross_deck_host_binding_claim_record_v1",
            "replacement_record_kind": (
                None
                if self.replacement_record_id is None
                else "cross_deck_host_binding_claim_record_v1"
            ),
            "reason_code": self.reason_code,
            "source_evidence_refs": [
                reference.to_wire() for reference in self.source_evidence_refs
            ],
            "acceptance": {
                "decision": "human_accepted",
                "review_event_ref": self.acceptance_event_ref.to_wire(),
            },
        }


def _wire_object(value: object, keys: set[str], label: str) -> dict[str, object]:
    if not isinstance(value, Mapping) or set(value) != keys:
        raise HostBindingContractError(f"{label} fields are not exactly {sorted(keys)!r}")
    return {cast(str, key): item for key, item in value.items()}


def _wire_digest(value: object, label: str) -> bytes:
    text = _nonempty_text(value, label)
    if _HEX64_RE.fullmatch(text) is None:
        raise HostBindingContractError(f"{label} must be lowercase SHA-256 hex")
    return bytes.fromhex(text)


def host_binding_source_binding_from_wire(value: object) -> HostBindingSourceBindingV2:
    record = _wire_object(
        value,
        {"artifact_role", "path", "schema_or_null", "raw_sha256"},
        "V2 source binding",
    )
    schema = record["schema_or_null"]
    if not isinstance(schema, str | None):
        raise HostBindingContractError("V2 source schema must be text or null")
    return HostBindingSourceBindingV2(
        artifact_role=_nonempty_text(record["artifact_role"], "V2 artifact role"),
        path=_nonempty_text(record["path"], "V2 source path"),
        schema_or_null=schema,
        raw_sha256=_wire_digest(record["raw_sha256"], "V2 source digest"),
    )


def host_binding_event_ref_from_wire(value: object) -> HostBindingAcceptanceEventRefV2:
    record = _wire_object(value, {"path", "raw_sha256", "event_id"}, "V2 event reference")
    return HostBindingAcceptanceEventRefV2(
        path=_nonempty_text(record["path"], "V2 event path"),
        raw_sha256=_wire_digest(record["raw_sha256"], "V2 event digest"),
        event_id=_nonempty_text(record["event_id"], "V2 event ID"),
    )


def _acceptance_evidence_from_wire(value: object) -> AcceptanceEvidenceRefV1:
    record = _wire_object(
        value,
        {"path", "raw_sha256", "locator"},
        "V2 review evidence reference",
    )
    locator_record = record["locator"]
    if not isinstance(locator_record, Mapping):
        raise HostBindingContractError("V2 review evidence locator must be an object")
    kind = _nonempty_text(locator_record.get("kind"), "V2 review evidence locator kind")
    if kind == "whole_artifact":
        if set(locator_record) != {"kind"}:
            raise HostBindingContractError("whole_artifact evidence locator is not closed")
        payload: str | None = None
    else:
        if set(locator_record) != {"kind", "value"}:
            raise HostBindingContractError("V2 review evidence locator is not closed")
        raw_payload = locator_record["value"]
        if not isinstance(raw_payload, str):
            raise HostBindingContractError("V2 review evidence locator value must be text")
        payload = raw_payload
    if kind not in {"whole_artifact", "json_pointer", "archive_member"}:
        raise HostBindingContractError("V2 review evidence locator kind is not accepted")
    return AcceptanceEvidenceRefV1(
        path=_nonempty_text(record["path"], "V2 review evidence path"),
        raw_sha256=_wire_digest(record["raw_sha256"], "V2 review evidence digest"),
        locator=(kind, payload),
    )


def host_binding_acceptance_event_from_wire(
    value: object,
) -> HostBindingAcceptanceEventInputV2:
    record = _wire_object(
        value,
        {
            "event_id",
            "schema",
            "subject_kind",
            "subject_payload_digest",
            "decision",
            "reviewer_roster_ref",
            "reviewer_role_bindings",
            "review_mode",
            "checklist_id",
            "source_binding_digests",
            "review_evidence_refs",
        },
        "V2 acceptance event leaf",
    )
    if record["schema"] != HOST_BINDING_ACCEPTANCE_EVENT_SCHEMA_V2:
        raise HostBindingContractError("V2 acceptance event schema is invalid")
    if record["decision"] != "human_accepted":
        raise HostBindingContractError("V2 acceptance event is not human_accepted")
    roster_record = _wire_object(
        record["reviewer_roster_ref"],
        {"path", "schema", "raw_sha256"},
        "V2 reviewer roster reference",
    )
    reviewer_roster_ref = ReviewerRosterRefV1(
        path=_nonempty_text(roster_record["path"], "V2 reviewer roster path"),
        schema=_nonempty_text(roster_record["schema"], "V2 reviewer roster schema"),
        raw_sha256=_wire_digest(roster_record["raw_sha256"], "V2 reviewer roster digest"),
    )
    raw_roles = record["reviewer_role_bindings"]
    raw_sources = record["source_binding_digests"]
    raw_evidence = record["review_evidence_refs"]
    if not isinstance(raw_roles, list) or not isinstance(raw_sources, list):
        raise HostBindingContractError("V2 acceptance roles and sources must be arrays")
    if not isinstance(raw_evidence, list):
        raise HostBindingContractError("V2 acceptance evidence must be an array")
    role_bindings: list[ReviewerRoleBindingV1] = []
    for item in raw_roles:
        role_record = _wire_object(item, {"reviewer_id", "roles"}, "V2 reviewer role binding")
        roles = role_record["roles"]
        if not isinstance(roles, list) or any(not isinstance(role, str) for role in roles):
            raise HostBindingContractError("V2 reviewer roles must be a string array")
        role_bindings.append(
            ReviewerRoleBindingV1(
                reviewer_id=_nonempty_text(role_record["reviewer_id"], "V2 reviewer ID"),
                roles=tuple(cast(list[str], roles)),
            )
        )
    try:
        review_mode = ReviewMode(_nonempty_text(record["review_mode"], "V2 review mode"))
    except ValueError as exc:
        raise HostBindingContractError("V2 review mode is not closed") from exc
    event_input = HostBindingAcceptanceEventInputV2(
        subject_kind=_nonempty_text(record["subject_kind"], "V2 subject kind"),
        subject_payload_digest=_wire_digest(record["subject_payload_digest"], "V2 subject digest"),
        reviewer_roster_ref=reviewer_roster_ref,
        reviewer_role_bindings=tuple(role_bindings),
        review_mode=review_mode,
        checklist_id=_nonempty_text(record["checklist_id"], "V2 checklist ID"),
        source_binding_digests=tuple(
            host_binding_source_binding_from_wire(item) for item in raw_sources
        ),
        review_evidence_refs=tuple(_acceptance_evidence_from_wire(item) for item in raw_evidence),
    )
    expected_id = event_input.identity().as_text()
    if record["event_id"] != expected_id:
        raise HostBindingContractError("V2 acceptance event ID differs from its input")
    return event_input


def host_binding_identity_from_wire(
    value: object, expected_kind: HostBindingIdentityKind
) -> HostBindingIdentityV1:
    record = _wire_object(
        value,
        {
            "envelope_id",
            "algorithm_id",
            "semantic_domain",
            "payload_codec_id",
            "input_schema_id",
            "digest_hex",
        },
        "host-binding identity",
    )
    prefix, domain, schema = _IDENTITY_SPECS[expected_kind]
    del prefix
    if (
        record["envelope_id"] != DIGEST_ENVELOPE_ID
        or record["algorithm_id"] != SHA256_ID
        or record["semantic_domain"] != domain
        or record["payload_codec_id"] != CANONICAL_CBOR_ID
        or record["input_schema_id"] != schema
    ):
        raise HostBindingContractError("host-binding identity metadata differs from its kind")
    return HostBindingIdentityV1(
        expected_kind,
        _wire_digest(record["digest_hex"], "host-binding identity digest"),
    )


def _wire_locator(value: object, label: str) -> LocatorV2:
    record = cast(dict[str, object], value)
    if not isinstance(value, Mapping):
        raise HostBindingContractError(f"{label} must be an object")
    kind = _nonempty_text(record.get("kind"), f"{label}.kind")
    if kind == "whole_artifact":
        keys = {"kind"}
        payload: str | int | None = None
    else:
        keys = {"kind", "value"}
        payload = cast(str | int | None, record.get("value"))
    if set(record) != keys:
        raise HostBindingContractError(f"{label} fields are not closed")
    return _validate_locator((kind, payload))


def host_binding_evidence_from_wire(value: object) -> HostBindingEvidenceRefV2:
    record = _wire_object(
        value,
        {"artifact_role", "path", "schema_or_null", "raw_sha256", "locator"},
        "host-binding evidence reference",
    )
    schema = record["schema_or_null"]
    if not isinstance(schema, str | None):
        raise HostBindingContractError("host-binding evidence schema must be text or null")
    return HostBindingEvidenceRefV2(
        artifact_role=_nonempty_text(record["artifact_role"], "artifact role"),
        path=_nonempty_text(record["path"], "evidence path"),
        schema_or_null=schema,
        raw_sha256=_wire_digest(record["raw_sha256"], "evidence digest"),
        locator=_wire_locator(record["locator"], "evidence locator"),
    )


def application_member_key_from_wire(value: object) -> ApplicationMemberKeyV1:
    record = _wire_object(
        value,
        {"candidate_id", "candidate_identity_digest", "source_instance_id"},
        "application member key",
    )
    return ApplicationMemberKeyV1(
        candidate_id=_nonempty_text(record["candidate_id"], "candidate ID"),
        candidate_identity_digest=_wire_digest(
            record["candidate_identity_digest"], "candidate identity digest"
        ),
        source_instance_id=_nonempty_text(record["source_instance_id"], "source instance ID"),
    )


def discovery_host_from_wire(value: object) -> DiscoveryHostRefV1:
    record = _wire_object(value, {"host_kind", "host_id"}, "discovery host")
    return DiscoveryHostRefV1(
        host_kind=_nonempty_text(record["host_kind"], "host kind"),
        host_id=_nonempty_text(record["host_id"], "host ID"),
    )


def discovery_binding_from_wire(
    value: object,
) -> CrossDeckParticipantDiscoveryHostBindingV1:
    record = _wire_object(
        value,
        {
            "member_key",
            "participant_position",
            "participant_ref",
            "discovery_side",
            "discovery_host",
            "mapping_evidence_refs",
        },
        "discovery binding",
    )
    raw_refs = record["mapping_evidence_refs"]
    if not isinstance(raw_refs, list):
        raise HostBindingContractError("mapping evidence references must be an array")
    return CrossDeckParticipantDiscoveryHostBindingV1(
        member_key=application_member_key_from_wire(record["member_key"]),
        participant_position=_u32(record["participant_position"], "participant position"),
        participant_ref=_nonempty_text(record["participant_ref"], "participant reference"),
        discovery_side=_nonempty_text(record["discovery_side"], "discovery side"),
        discovery_host=discovery_host_from_wire(record["discovery_host"]),
        mapping_evidence_refs=tuple(host_binding_evidence_from_wire(item) for item in raw_refs),
    )


def host_realization_witness_from_wire(value: object) -> HostRealizationWitnessV1:
    record = _wire_object(
        value,
        {"discovery_mapping_ref", "deck_row_ref", "osi_ref", "b2_assignment_refs"},
        "host realization witness",
    )
    raw_refs = record["b2_assignment_refs"]
    if not isinstance(raw_refs, list):
        raise HostBindingContractError("B2 assignment references must be an array")
    return HostRealizationWitnessV1(
        discovery_mapping_ref=host_binding_evidence_from_wire(record["discovery_mapping_ref"]),
        deck_row_ref=host_binding_evidence_from_wire(record["deck_row_ref"]),
        osi_ref=host_binding_evidence_from_wire(record["osi_ref"]),
        b2_assignment_refs=tuple(host_binding_evidence_from_wire(item) for item in raw_refs),
    )


def participant_realization_from_wire(value: object) -> ParticipantHostRealizationV1:
    record = _wire_object(
        value,
        {"member_key", "participant_position", "participant_ref", "host", "witnesses"},
        "participant host realization",
    )
    raw_witnesses = record["witnesses"]
    if not isinstance(raw_witnesses, list):
        raise HostBindingContractError("host realization witnesses must be an array")
    return ParticipantHostRealizationV1(
        member_key=application_member_key_from_wire(record["member_key"]),
        participant_position=_u32(
            record["participant_position"], "realization participant position"
        ),
        participant_ref=_nonempty_text(record["participant_ref"], "realization participant ref"),
        host=discovery_host_from_wire(record["host"]),
        witnesses=tuple(host_realization_witness_from_wire(item) for item in raw_witnesses),
    )


def host_binding_claim_from_wire(value: object) -> CrossDeckHostBindingClaimV1:
    record = _wire_object(
        value,
        {
            "claim_id",
            "member_key",
            "discovery_bindings",
            "participant_host_realizations",
            "observed_host_relationship",
        },
        "host-binding claim",
    )
    raw_discovery = record["discovery_bindings"]
    raw_realizations = record["participant_host_realizations"]
    if not isinstance(raw_discovery, list) or not isinstance(raw_realizations, list):
        raise HostBindingContractError("claim bindings and realizations must be arrays")
    claim = CrossDeckHostBindingClaimV1(
        member_key=application_member_key_from_wire(record["member_key"]),
        discovery_bindings=tuple(discovery_binding_from_wire(item) for item in raw_discovery),
        participant_host_realizations=tuple(
            participant_realization_from_wire(item) for item in raw_realizations
        ),
        observed_host_relationship=_nonempty_text(
            record["observed_host_relationship"], "observed host relationship"
        ),
    )
    if record["claim_id"] != claim.identity().as_text():
        raise HostBindingContractError("claim_id does not match the canonical claim identity")
    return claim


def host_binding_claim_record_from_wire(
    value: object,
) -> CrossDeckHostBindingClaimRecordV1:
    record = _wire_object(
        value,
        {"record_kind", "record_id", "claim_id", "claim", "acceptance"},
        "host-binding claim record",
    )
    if record["record_kind"] != "cross_deck_host_binding_claim_record_v1":
        raise HostBindingContractError("host-binding record kind is invalid")
    claim = host_binding_claim_from_wire(record["claim"])
    claim_identity = host_binding_identity_from_wire(
        record["claim_id"],
        HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_CLAIM,
    )
    if claim_identity != claim.identity():
        raise HostBindingContractError("claim record claim ID differs from its claim")
    acceptance = _wire_object(
        record["acceptance"],
        {"decision", "review_event_ref"},
        "host-binding claim acceptance",
    )
    if acceptance["decision"] != "human_accepted":
        raise HostBindingContractError("host-binding claim is not human accepted")
    parsed = CrossDeckHostBindingClaimRecordV1(
        claim=claim,
        acceptance_event_ref=host_binding_event_ref_from_wire(acceptance["review_event_ref"]),
    )
    record_identity = host_binding_identity_from_wire(
        record["record_id"],
        HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_CLAIM_RECORD,
    )
    if record_identity != parsed.record_identity():
        raise HostBindingContractError("claim record ID differs from its canonical input")
    return parsed


def host_binding_claim_supersession_from_wire(
    value: object,
) -> CrossDeckHostBindingClaimSupersessionV1:
    record = _wire_object(
        value,
        {
            "supersession_id",
            "superseded_record_id",
            "replacement_record_id",
            "superseded_record_kind",
            "replacement_record_kind",
            "reason_code",
            "source_evidence_refs",
            "acceptance",
        },
        "host-binding claim supersession",
    )
    expected_kind = HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_CLAIM_RECORD
    superseded_record_id = host_binding_identity_from_wire(
        record["superseded_record_id"], expected_kind
    )
    replacement_value = record["replacement_record_id"]
    replacement_record_id = (
        None
        if replacement_value is None
        else host_binding_identity_from_wire(replacement_value, expected_kind)
    )
    if record["superseded_record_kind"] != "cross_deck_host_binding_claim_record_v1":
        raise HostBindingContractError("host-binding superseded record kind is invalid")
    replacement_kind = record["replacement_record_kind"]
    if replacement_kind not in {None, "cross_deck_host_binding_claim_record_v1"}:
        raise HostBindingContractError("host-binding replacement record kind is invalid")
    if (replacement_value is None) != (replacement_kind is None):
        raise HostBindingContractError("host-binding replacement ID and kind must agree")
    raw_evidence = record["source_evidence_refs"]
    if not isinstance(raw_evidence, list):
        raise HostBindingContractError("host-binding supersession evidence must be an array")
    acceptance = _wire_object(
        record["acceptance"],
        {"decision", "review_event_ref"},
        "host-binding supersession acceptance",
    )
    if acceptance["decision"] != "human_accepted":
        raise HostBindingContractError("host-binding supersession is not human accepted")
    parsed = CrossDeckHostBindingClaimSupersessionV1(
        superseded_record_id=superseded_record_id,
        replacement_record_id=replacement_record_id,
        reason_code=_nonempty_text(record["reason_code"], "supersession reason"),
        source_evidence_refs=tuple(host_binding_evidence_from_wire(item) for item in raw_evidence),
        acceptance_event_ref=host_binding_event_ref_from_wire(acceptance["review_event_ref"]),
    )
    expected_identity = host_binding_identity_from_wire(
        record["supersession_id"],
        HostBindingIdentityKind.CROSS_DECK_HOST_BINDING_SUPERSESSION,
    )
    if expected_identity != parsed.identity():
        raise HostBindingContractError("host-binding supersession identity is invalid")
    return parsed


def application_host_binding_from_wire(value: object) -> ApplicationHostBindingV1:
    record = _wire_object(
        value,
        {"application_kind", "application_semantic_id", "host_binding_claim_ids"},
        "application host binding",
    )
    raw_claim_ids = record["host_binding_claim_ids"]
    if not isinstance(raw_claim_ids, list) or any(
        not isinstance(item, str) for item in raw_claim_ids
    ):
        raise HostBindingContractError("application host-binding claim IDs must be text array")
    return ApplicationHostBindingV1(
        application_kind=_nonempty_text(record["application_kind"], "application kind"),
        application_semantic_id=_nonempty_text(
            record["application_semantic_id"], "application semantic ID"
        ),
        host_binding_claim_ids=tuple(cast(list[str], raw_claim_ids)),
    )


__all__ = [
    "APPLICATION_KINDS",
    "HOST_BINDING_ACCEPTANCE_EVENT_INPUT_SCHEMA_V2",
    "HOST_BINDING_ACCEPTANCE_EVENT_SCHEMA_V2",
    "HOST_BINDING_ACCEPTANCE_SUBJECT_KINDS",
    "HOST_BINDING_ACCEPTANCE_SUBJECT_SCHEMA_V2",
    "HOST_BINDING_AUTHORITY_SCHEMA_V2",
    "HOST_BINDING_CHECKLIST_V1",
    "HOST_BINDING_CLAIM_INPUT_SCHEMA_V1",
    "HOST_BINDING_CLAIM_RECORD_INPUT_SCHEMA_V1",
    "HOST_BINDING_CLAIM_RECORD_SCHEMA_V1",
    "HOST_BINDING_CLAIM_SCHEMA_V1",
    "HOST_BINDING_SUPERSESSION_INPUT_SCHEMA_V1",
    "HOST_RELATIONSHIPS",
    "V2_SOURCE_ROLES",
    "ApplicationHostBindingV1",
    "ApplicationMemberKeyV1",
    "CrossDeckHostBindingClaimRecordV1",
    "CrossDeckHostBindingClaimSupersessionV1",
    "CrossDeckHostBindingClaimV1",
    "CrossDeckParticipantDiscoveryHostBindingV1",
    "DiscoveryHostRefV1",
    "HostBindingAcceptanceEventInputV2",
    "HostBindingAcceptanceEventLeafV2",
    "HostBindingAcceptanceEventRefV2",
    "HostBindingContractError",
    "HostBindingEvidenceRefV2",
    "HostBindingIdentityKind",
    "HostBindingIdentityV1",
    "HostBindingSourceBindingV2",
    "HostRealizationWitnessV1",
    "ParticipantHostRealizationV1",
    "application_host_binding_from_wire",
    "application_member_key_from_wire",
    "discovery_binding_from_wire",
    "discovery_host_from_wire",
    "host_binding_acceptance_event_from_wire",
    "host_binding_claim_from_wire",
    "host_binding_claim_record_from_wire",
    "host_binding_claim_supersession_from_wire",
    "host_binding_event_ref_from_wire",
    "host_binding_evidence_from_wire",
    "host_binding_identity_from_wire",
    "host_binding_source_binding_from_wire",
    "host_realization_witness_from_wire",
    "participant_realization_from_wire",
]
