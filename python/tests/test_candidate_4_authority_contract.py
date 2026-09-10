from __future__ import annotations

import base64
import copy
import hashlib
import json
import os
import sys
import unittest
from collections.abc import Mapping
from dataclasses import dataclass, replace
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from authority_source_resolver import AuthoritySourceResolver
from authority_v2_validator import HostBindingAuthorityV2ReadModel
from authority_validator import validate_relation_member_proof_v1_against_theorem
from context_application_v3_host_binding import (
    ContextApplicationV3HostBindingError,
    admit_host_binding_authority_v2,
    validate_application_host_binding_v3,
)
from context_application_v3_resolver import (
    ContextApplicationV3AuthorityResolver,
    ContextApplicationV3ResolutionError,
    ContextApplicationV3Resolver,
    ContextApplicationV3RpaResolver,
)
from context_application_v3_review_admission import (
    ContextApplicationV3ReviewAdmissionError,
    admit_context_application_v3_record,
)
from context_application_v3_validator import (
    ContextApplicationV3SemanticValidationError,
    ContextApplicationV3SemanticValidator,
)
from mtgml.authority import (
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKindV4,
    AcceptanceSubjectPayloadV4,
    ApplicationHostBindingV3,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationAuthorityV3,
    ContextApplicationMemberV3,
    ContextApplicationV3InputV1,
    ContextApplicationV3Record,
    ContextAuthoritySourceBindingV2,
    ContextAuthoritySourceBindingV3,
    ContextBridgeRelationV2,
    ContextMemberBridgeAttestationV2,
    ContextSlotBridgeAttestationV2,
    DigestReferenceV1,
    EvidenceRefV1,
    ParticipantRoleBridgeEntryV1,
    ParticipantRoleBridgeV1,
    RelationApplicationAuthorityV2,
    RelationApplicationMemberV2,
    RelationApplicationV2,
    RelationApplicationV2Record,
    RelationAuthoritySourceBindingV2,
    ReviewAcceptanceEventInputV4,
    ReviewAcceptanceEventLeafV4,
    ReviewAuthoritySourceBindingV4,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV4,
    ReviewMode,
    SourceBindingDigestV1,
    TemporalSlotAttestationV2,
)
from mtgml.host_binding import (
    ApplicationMemberKeyV1,
    CrossDeckHostBindingClaimRecordV1,
    CrossDeckHostBindingClaimV1,
    CrossDeckParticipantDiscoveryHostBindingV1,
    DiscoveryHostRefV1,
    HostBindingAcceptanceEventInputV2,
    HostBindingAcceptanceEventLeafV2,
    HostBindingAcceptanceEventRefV2,
    HostBindingEvidenceRefV2,
    HostBindingSourceBindingV2,
    HostRealizationWitnessV1,
    ParticipantHostRealizationV1,
)
from mtgml.persistence import encode_canonical
from relation_application_v2_review_admission import admit_relation_application_v2_record
from relation_application_v2_validator import (
    RelationApplicationV2SemanticValidationError,
    validate_relation_application_v2_semantics,
)
from test_context_application_v2_host_binding import _cross_host_claim

CANDIDATE_ID = "CROSS_DECK|P3|cap.mass_destruction|cap.death_trigger|DIRECTIONAL_BINARY"
CANDIDATE_DIGEST = bytes.fromhex("af33dd4f0b65103102828bfec8ebd23196b1685282ee6ef9f0dc4690e3a6420b")
CANDIDATE_UNIVERSE_PATH = "sources/m2_5/closures/C/interaction_candidate_universe.v2.json"
CANDIDATE_UNIVERSE_SCHEMA = "manafold.m2.5.c.interaction-candidate-universe.v2"
CANDIDATE_UNIVERSE_DIGEST = bytes.fromhex(
    "1f8761af56f8b44c5e51d8cb9fcff79dd95dd56a98bfc6793e2ca8860050c532"
)
SOURCE_INSTANCE_ID = (
    "si.v1/"
    + base64.urlsafe_b64encode(CANDIDATE_ID.encode("utf-8")).decode("ascii").rstrip("=")
    + "/0"
)
ZERO = bytes(32)
REV3_CARD_REQUIREMENT_MAP_PATH = "derived/Card_Requirement_Map_REV3.csv"
REV3_CARD_REQUIREMENT_MAP_DIGEST = bytes.fromhex(
    "07af07fa0a45785cd497db616343569786212f58f6aa3e61e5f143fc1e23bfe7"
)
REV3_DECK_ROW_SOURCE_RESOLUTION_PATH = "inputs/deck_row_source_resolution_REV3.csv"
REV3_DECK_ROW_SOURCE_RESOLUTION_DIGEST = bytes.fromhex(
    "611a5d1de9ee8560d52ef434666d02d6d0906d321e5c41974b4588d163e508da"
)
REV3_OSI_SOURCE_RECORDS_PATH = "source/raw/oracle_cards_selected_REV3.jsonl"
REV3_OSI_SOURCE_RECORDS_DIGEST = bytes.fromhex(
    "0392cf3d9c4f8c27fd1a12722889594dfb79e0f9f1b92764db8d577e98e08b2b"
)
REV3_PAIR_AGGREGATES_PATH = "derived/Pair_Requirement_Aggregates_REV3.json"
REV3_PAIR_AGGREGATES_DIGEST = bytes.fromhex(
    "9fec921f3a29548f9638c3708fffa37fb4900991fa88ec005c7232d3830e7f74"
)
REV3_ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"
REQUIRED_ROLES = (
    "architecture_maintainer",
    "conformance_maintainer",
    "information_safety_reviewer",
    "rules_authority_maintainer",
)
CONTEXT_DIMENSIONS = (
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
TEMPORAL_DIMENSIONS = ("trigger_order", "dependency_order", "duration", "replacement_order")


def _fixture(name: str) -> dict[str, object]:
    return json.loads(
        (ROOT / "conformance" / "fixtures" / "authority" / name).read_text(encoding="utf-8")
    )


def _candidate_identity() -> DigestReferenceV1:
    return DigestReferenceV1(
        "mtgml.digest-envelope.v1",
        "sha-256",
        "manafold.m2.5.c.candidate-identity.v1",
        "mtgml.canonical-cbor.v1",
        "manafold.m2.5.c.candidate-identity-input.v1",
        CANDIDATE_DIGEST,
    )


def _evidence() -> EvidenceRefV1:
    return EvidenceRefV1(
        "model", "sources/review-evidence.json", ("whole_artifact", None), b"e" * 32
    )


def _member_evidence() -> tuple[EvidenceRefV1, ...]:
    return (_evidence(),)


def _candidate_binding() -> list[object]:
    return [
        CANDIDATE_UNIVERSE_PATH,
        CANDIDATE_UNIVERSE_SCHEMA,
        CANDIDATE_UNIVERSE_DIGEST,
    ]


def _repo_digest(relative_path: str) -> bytes:
    return hashlib.sha256((ROOT / Path(*relative_path.split("/"))).read_bytes()).digest()


def _plain(value: object) -> object:
    if isinstance(value, Mapping):
        return {str(key): _plain(child) for key, child in value.items()}
    if isinstance(value, list | tuple):
        return [_plain(child) for child in value]
    return value


def _bridge(
    *,
    historical: tuple[str, str] = ("ordered_participant", "ordered_participant"),
    reviewed: tuple[str, str] = ("source", "affected"),
    participant_kind: str = "requirement_family",
    semantic_refs: tuple[str, str] = ("cap.mass_destruction", "cap.death_trigger"),
) -> ParticipantRoleBridgeV1:
    return ParticipantRoleBridgeV1(
        (
            ParticipantRoleBridgeEntryV1(
                0, participant_kind, semantic_refs[0], historical[0], reviewed[0]
            ),
            ParticipantRoleBridgeEntryV1(
                1, participant_kind, semantic_refs[1], historical[1], reviewed[1]
            ),
        )
    )


def _relation_binding(reviewed: tuple[str, str] = ("source", "affected")) -> list[object]:
    return [
        "cross_deck",
        "directional_binary",
        "directed",
        "cross_host",
        [
            [0, reviewed[0], "requirement_family", "cap.mass_destruction"],
            [1, reviewed[1], "requirement_family", "cap.death_trigger"],
        ],
    ]


def _context_binding(reviewed: tuple[str, str] = ("source", "affected")) -> list[object]:
    return [
        "binary",
        "directed",
        [
            [0, reviewed[0], "requirement_family", "cap.mass_destruction"],
            [1, reviewed[1], "requirement_family", "cap.death_trigger"],
        ],
        "cross_host",
    ]


def _rpa_theorem(roles: tuple[str, str] = ("source", "affected")) -> dict[str, object]:
    return {
        "proof_kind": "positive_interaction",
        "subject": {
            "arity": "binary",
            "relation": "directional_binary",
            "directionality": "directed",
            "participant_roles": [
                {
                    "position": 0,
                    "role": roles[0],
                    "participant_kind": "requirement_family",
                    "semantic_ref": "cap.mass_destruction",
                },
                {
                    "position": 1,
                    "role": roles[1],
                    "participant_kind": "requirement_family",
                    "semantic_ref": "cap.death_trigger",
                },
            ],
            "host_relationship": "cross_host",
        },
        "proof_payload": {
            "kind": "positive_interaction",
            "causal_chain": [{}],
            "class_projection_template": None,
        },
        "preconditions": [],
    }


def _context_preconditions() -> list[dict[str, object]]:
    values = [
        {
            "precondition_id": "shape",
            "precondition_kind": "candidate_relation_shape",
            "payload": ["cross_deck", "directional_binary", "directed", "cross_host"],
        },
        {
            "precondition_id": "participant",
            "precondition_kind": "participant_binding",
            "payload": [0, "ordered_participant", "requirement_family", "cap.mass_destruction"],
        },
        {
            "precondition_id": "b2",
            "precondition_kind": "b2_boundary",
            "payload": ["family.a", "active", "classification", "definition"],
        },
        {
            "precondition_id": "source",
            "precondition_kind": "source_context",
            "payload": ["zone", "not_applicable"],
        },
        {
            "precondition_id": "temporal",
            "precondition_kind": "temporal_semantic",
            "payload": ["trigger_order", "not_applicable"],
        },
        {
            "precondition_id": "projection",
            "precondition_kind": "class_projection",
            "payload": [
                "binary",
                "directed",
                [],
                ["not_applicable"] * 10,
                ["not_applicable"] * 4,
                [],
                [],
                [],
                [],
            ],
        },
    ]
    return sorted(values, key=lambda item: encode_canonical(str(item["precondition_id"])))


def _context_theorem() -> dict[str, object]:
    return {
        "subject_shape": {
            "arity": "binary",
            "directionality": "directed",
            "participant_roles": [
                {
                    "position": 0,
                    "role": "source",
                    "participant_kind": "requirement_family",
                    "semantic_ref": "cap.mass_destruction",
                },
                {
                    "position": 1,
                    "role": "affected",
                    "participant_kind": "requirement_family",
                    "semantic_ref": "cap.death_trigger",
                },
            ],
            "host_relationship": "cross_host",
        },
        "preconditions": _context_preconditions(),
        "context_dimensions": [
            "battlefield",
            "not_applicable",
            "not_applicable",
            "not_applicable",
            "not_applicable",
            "not_applicable",
            "not_applicable",
            "not_applicable",
            "not_applicable",
            "not_applicable",
        ],
        "temporal_semantics": ["not_applicable"] * 4,
    }


def _context_bridge() -> ContextMemberBridgeAttestationV2:
    context = tuple(
        ContextSlotBridgeAttestationV2(
            name,
            "not_applicable",
            "battlefield" if name == "zone" else "not_applicable",
            ContextBridgeRelationV2.REVIEWED_DIVERGENCE
            if name == "zone"
            else ContextBridgeRelationV2.EXACT_MATCH,
            _member_evidence(),
            "synthetic Candidate-4 context bridge",
        )
        for name in CONTEXT_DIMENSIONS
    )
    temporal = tuple(
        TemporalSlotAttestationV2(
            name, "not_applicable", _member_evidence(), "synthetic temporal bridge"
        )
        for name in TEMPORAL_DIMENSIONS
    )
    return ContextMemberBridgeAttestationV2(context, temporal)


def _precondition_attestations() -> list[object]:
    return [
        [
            item["precondition_id"],
            item["payload"],
            [_evidence().to_cbor()],
            "synthetic Candidate-4 precondition",
        ]
        for item in _context_preconditions()
    ]


def _rpa_member(
    *,
    bridge: ParticipantRoleBridgeV1 | None = None,
    relation: list[object] | None = None,
    candidate_id: str = CANDIDATE_ID,
    identity: DigestReferenceV1 | None = None,
    source_instance_id: str = SOURCE_INSTANCE_ID,
) -> RelationApplicationMemberV2:
    return RelationApplicationMemberV2(
        candidate_id=candidate_id,
        candidate_identity_digest_reference=identity or _candidate_identity(),
        source_instance_id=source_instance_id,
        candidate_universe_binding=_candidate_binding(),
        reviewed_relation_binding_v1=relation or _relation_binding(),
        participant_role_bridge_v1=bridge or _bridge(),
        precondition_attestations_v1=[],
        member_evidence_refs=_member_evidence(),
        member_proof_attestation_v1=["positive_interaction", [[0], None]],
    )


def _context_member(
    *,
    rpa_id: bytes,
    identity: DigestReferenceV1 | None = None,
    candidate_id: str = CANDIDATE_ID,
    source_instance_id: str = SOURCE_INSTANCE_ID,
    bridge: ContextMemberBridgeAttestationV2 | None = None,
) -> ContextApplicationMemberV3:
    return ContextApplicationMemberV3(
        candidate_id=candidate_id,
        candidate_identity_digest_reference=identity or _candidate_identity(),
        source_instance_id=source_instance_id,
        candidate_universe_binding=_candidate_binding(),
        reviewed_context_binding_v1=_context_binding(),
        relation_application_v2_id_bytes=rpa_id,
        precondition_attestations_v1=_precondition_attestations(),
        member_evidence_refs=_member_evidence(),
        context_member_bridge_attestation_v2=bridge or _context_bridge(),
    )


def _source_record(
    *,
    roles: tuple[str, str] = ("ordered_participant", "ordered_participant"),
    refs: tuple[str, str] = ("cap.mass_destruction", "cap.death_trigger"),
) -> dict[str, object]:
    participants = [
        {
            "role": role,
            "participant_ref": {"participant_kind": "requirement_family", "semantic_ref": ref},
        }
        for role, ref in zip(roles, refs, strict=True)
    ]
    return {
        "relation_binding": {"participant_bindings": participants},
        "participant_bindings": participants,
        "source_context": {name: "not_applicable" for name in CONTEXT_DIMENSIONS},
    }


def _candidate_record() -> dict[str, object]:
    return {
        "scope": "cross_deck",
        "relation": "directional_binary",
        "participant_refs": [
            {"participant_kind": "requirement_family", "semantic_ref": "cap.mass_destruction"},
            {"participant_kind": "requirement_family", "semantic_ref": "cap.death_trigger"},
        ],
    }


def _resolved_source(
    *,
    roles: tuple[str, str] = ("ordered_participant", "ordered_participant"),
    refs: tuple[str, str] = ("cap.mass_destruction", "cap.death_trigger"),
) -> object:
    return SimpleNamespace(
        candidate=SimpleNamespace(candidate_record=_candidate_record()),
        source_instance_record=_source_record(roles=roles, refs=refs),
    )


def _binding_v4(role: str, digest: bytes) -> ReviewAuthoritySourceBindingV4:
    paths: dict[str, tuple[str, str | None]] = {
        "declared_model": (
            "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            "manafold.m2.5.c.declared-interaction-model.v2",
        ),
        "candidate_universe": (
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
        ),
        "acceptance_event_leaf_v1": (
            "sources/m2_5/authorities/review_acceptance_events/v1/" + "1" * 64 + ".json",
            "manafold.m2.5.c.review-acceptance-event.v1",
        ),
        "reviewer_roster_leaf": (
            "sources/m2_5/authorities/reviewer_rosters/v1/" + digest.hex() + ".json",
            "manafold.m2.5.c.reviewer-roster.v1",
        ),
    }
    path, schema = paths[role]
    return ReviewAuthoritySourceBindingV4(role, path, schema, digest)


def _roster(digest: bytes) -> ReviewerRosterRefV1:
    return ReviewerRosterRefV1(
        "sources/m2_5/authorities/reviewer_rosters/v1/" + digest.hex() + ".json",
        "manafold.m2.5.c.reviewer-roster.v1",
        digest,
    )


def _event(
    subject_kind: AcceptanceSubjectKindV4,
    payload: list[object],
    roster: ReviewerRosterRefV1,
    sources: tuple[ReviewAuthoritySourceBindingV4, ...],
) -> ReviewAcceptanceEventLeafV4:
    subject = AcceptanceSubjectPayloadV4(subject_kind, payload)
    return ReviewAcceptanceEventLeafV4.from_input(
        ReviewAcceptanceEventInputV4(
            subject_kind,
            DigestReferenceV1.from_identity(subject.identity()),
            roster,
            (ReviewerRoleBindingV1("candidate-4-reviewer", REQUIRED_ROLES),),
            ReviewMode.MULTI_REVIEWER,
            sources,
            (
                AcceptanceEvidenceRefV1(
                    "sources/review-evidence.json", b"e" * 32, ("whole_artifact", None)
                ),
            ),
        )
    )


@dataclass
class Candidate4Bundle:
    rpa_member: RelationApplicationMemberV2
    rpa_record: RelationApplicationV2Record
    rpa_event: ReviewAcceptanceEventLeafV4
    rpa_authority: RelationApplicationAuthorityV2
    context_member: ContextApplicationMemberV3
    context_record: ContextApplicationV3Record
    context_event: ReviewAcceptanceEventLeafV4
    context_authority: ContextApplicationAuthorityV3
    context_resolver: ContextApplicationV3Resolver
    relation_resolver: object
    host_read_model: HostBindingAuthorityV2ReadModel
    host_link: ApplicationHostBindingV3
    context_closure: tuple[ReviewAuthoritySourceBindingV4, ...]
    rpa_closure: tuple[ReviewAuthoritySourceBindingV4, ...]
    source_instance_snapshot: dict[str, object]
    source_resolver: Candidate4SourceResolver


class Candidate4SourceResolver(AuthoritySourceResolver):
    """Real repository/REV3 resolver with digest-checked test artifacts overlaid."""

    def __init__(self) -> None:
        super().__init__(ROOT)
        self._bound_artifacts: dict[str, SimpleNamespace] = {}

    def bind_json_artifact(
        self,
        path: str,
        schema: str,
        value: Mapping[str, object],
    ) -> bytes:
        raw = (json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n").encode("utf-8")
        return self.bind_raw_artifact(path, schema, raw, json_value=json.loads(raw))

    def bind_raw_artifact(
        self,
        path: str,
        schema: str | None,
        raw: bytes,
        *,
        json_value: object | None = None,
    ) -> bytes:
        digest = hashlib.sha256(raw).digest()
        self._bound_artifacts[path] = SimpleNamespace(
            source_kind="synthetic-conformance",
            path=path,
            raw_bytes=raw,
            raw_sha256=digest.hex(),
            json_value=json_value,
            schema=schema,
        )
        return digest

    def resolve_repository_artifact(
        self,
        path: str,
        expected_raw_sha256: object,
        schema_or_null: str | None,
    ) -> object:
        artifact = self._bound_artifacts.get(path)
        if artifact is not None:
            expected_digest = (
                expected_raw_sha256
                if isinstance(expected_raw_sha256, bytes)
                else bytes.fromhex(str(expected_raw_sha256))
            )
            if expected_digest.hex() != artifact.raw_sha256:
                raise ValueError(f"synthetic artifact digest mismatch for {path}")
            if schema_or_null != artifact.schema:
                raise ValueError(f"synthetic artifact schema mismatch for {path}")
            return artifact
        return super().resolve_repository_artifact(path, expected_raw_sha256, schema_or_null)


_CANDIDATE4_SOURCE_RESOLVER: Candidate4SourceResolver | None = None
_CANDIDATE4_RESOLVED_SOURCE: object | None = None


def _candidate4_source_resolver() -> Candidate4SourceResolver:
    global _CANDIDATE4_SOURCE_RESOLVER
    if _CANDIDATE4_SOURCE_RESOLVER is None:
        _CANDIDATE4_SOURCE_RESOLVER = Candidate4SourceResolver()
    return _CANDIDATE4_SOURCE_RESOLVER


def _semantic_source_view(resolved: object) -> object:
    """Keep real resolver bytes/identity while exposing its immutable facts as wire mappings."""

    raw = resolved.source_instance_record
    participants = [
        {
            "position": position,
            "role": item["role"],
            "participant_ref": dict(item["participant_ref"]),
        }
        for position, item in enumerate(raw["participant_bindings"])
    ]
    record = {
        "source_instance_id": raw["source_instance_id"],
        "candidate_id": raw["candidate_id"],
        "source_binding": dict(raw["source_binding"]),
        "participant_bindings": participants,
        "relation_binding": {"participant_bindings": participants},
        "source_context": dict(raw["source_context"]),
    }
    return SimpleNamespace(
        candidate=resolved.candidate,
        source_instance_record=record,
        source_binding=resolved.source_binding,
        source_artifact=resolved.source_artifact,
    )


class SyntheticRpaResolver:
    def __init__(
        self,
        record: RelationApplicationV2Record,
        event: ReviewAcceptanceEventLeafV4,
        source: object,
        theorem: Mapping[str, object],
        relation_sources: tuple[RelationAuthoritySourceBindingV2, ...],
        closure: tuple[ReviewAuthoritySourceBindingV4, ...],
        source_resolver: Candidate4SourceResolver,
    ) -> None:
        self.authority: object | None = None
        self.record = record
        self.event = event
        self.source = source
        self.theorem = theorem
        self.relation_sources = relation_sources
        self.closure = closure
        self.source_resolver = source_resolver
        self.source_override: object | None = None
        self.source_view = _semantic_source_view(source)
        self.roster_seen: list[ReviewerRosterRefV1] = []

    def resolve_candidate_source_instance(self, *_args: object) -> object:
        if len(_args) >= 3:
            candidate_id, candidate_identity, source_instance_id = _args[:3]
            if candidate_id != CANDIDATE_ID or source_instance_id != SOURCE_INSTANCE_ID:
                raise ValueError("candidate/source instance substitution")
            if (
                not isinstance(candidate_identity, Mapping)
                or candidate_identity.get("digest_hex") != CANDIDATE_DIGEST.hex()
            ):
                raise ValueError("candidate identity mismatch")
        if self.source_override is not None:
            return self.source_override
        return self.source_view

    def validate_relation_member_proof_v1(
        self, member: Mapping[str, object], theorem: Mapping[str, object], label: str
    ) -> None:
        validate_relation_member_proof_v1_against_theorem(member, theorem, label)

    def require_current_relation_theorem(self, _theorem_id: object) -> Mapping[str, object]:
        return self.theorem

    def resolve_relation_theorem_record(self, _theorem_id: object) -> Mapping[str, object]:
        return self.theorem

    def resolve_acceptance_event_leaf_v4(self, _reference: object) -> ReviewAcceptanceEventLeafV4:
        return self.event

    def resolve_v4_source_binding(self, _binding: object) -> object:
        return object()

    def resolve_v4_acceptance_evidence(self, _evidence: object) -> object:
        return object()

    def expected_relation_application_v2_source_closure(
        self, _record: object, roster: ReviewerRosterRefV1
    ) -> tuple[ReviewAuthoritySourceBindingV4, ...]:
        self.roster_seen.append(roster)
        if roster != self.event.reviewer_roster_ref:
            raise AssertionError("RPA closure received the wrong reviewer roster")
        return self.closure

    def validate_relation_application_authority_v2_source_closure(
        self, _authority: object
    ) -> tuple[RelationAuthoritySourceBindingV2, ...]:
        return self.relation_sources


class SyntheticV1Validator:
    def __init__(self) -> None:
        self.last_member: Mapping[str, object] | None = None

    def _validate_context_member_source_contract_v1(
        self,
        member: Mapping[str, object],
        _theorem: Mapping[str, object],
        _resolved: object,
        _label: str,
    ) -> None:
        self.last_member = member
        roles = [item["role"] for item in member["context_binding"]["participant_roles"]]
        if roles != ["ordered_participant", "ordered_participant"]:
            raise ValueError("historical participant roles were rewritten")
        if "member_proof_attestation" not in member:
            raise ValueError("linked RPA proof was not projected")


class SyntheticContextV2Resolver:
    def __init__(self, model: ContextAuthoritySourceBindingV2) -> None:
        self.model = model

    def _base_context(
        self, _binding: object
    ) -> tuple[object, ContextAuthoritySourceBindingV2, tuple[object, ...], object]:
        return ({}, self.model, (), ())

    def _candidate_provenance(
        self, _member: object
    ) -> tuple[tuple[object, ...], tuple[object, ...]]:
        return ((), ())

    def _member_evidence(self, _member: object) -> tuple[object, ...]:
        return ()

    def _collect_evidence(self, _evidence: object) -> tuple[tuple[object, ...], set[str], bool]:
        return ((), set(), False)

    def _walk_v1_dependencies(
        self, _theorem: Mapping[str, object]
    ) -> tuple[tuple[object, ...], set[str], bool]:
        return ((), set(), False)

    def resolve_member_source_instance(self, _member: object) -> object:
        raise AssertionError(
            "synthetic V2 resolver should be replaced by Context V3 source resolution"
        )


class SyntheticContextResolver(ContextApplicationV3Resolver):
    def __init__(
        self,
        rpa_member_resolver: ContextApplicationV3RpaResolver,
        source: object,
        theorem: Mapping[str, object],
        candidate: Mapping[str, object],
        rpa_event: ReviewAcceptanceEventLeafV4,
        source_resolver: Candidate4SourceResolver,
        context_event: ReviewAcceptanceEventLeafV4 | None = None,
    ) -> None:
        base_v2 = ContextAuthoritySourceBindingV2(
            "base_authority_v1",
            "sources/m2_5/authorities/interaction_review_authority.v1.json",
            "manafold.m2.5.c.interaction-review-authority.v1",
            b"b" * 32,
        )
        super().__init__(
            source_resolver,
            base_authority_binding=base_v2,
            rpa_member_resolver=rpa_member_resolver,
        )
        self._rpa_member_resolver = rpa_member_resolver
        self._source = source
        self._theorem = theorem
        self._candidate = candidate
        self._candidate_source_resolver = source_resolver
        self._source_view = _semantic_source_view(source)
        self._rpa_event = rpa_event
        self._v1_validator = SyntheticV1Validator()
        self._v2_resolver = SyntheticContextV2Resolver(
            ContextAuthoritySourceBindingV2(
                "declared_model",
                "sources/m2_5/closures/C/declared_interaction_model.v2.json",
                "manafold.m2.5.c.declared-interaction-model.v2",
                _repo_digest("sources/m2_5/closures/C/declared_interaction_model.v2.json"),
            )
        )
        self._context_event = context_event
        self._v1_leaf = _binding_v4("acceptance_event_leaf_v1", b"1" * 32)

    def _validated_base(self) -> tuple[SyntheticV1Validator, Mapping[str, object]]:
        return self._v1_validator, {}

    def resolve_current_context_theorem(self, _theorem_id: object) -> Mapping[str, object]:
        return self._theorem

    def resolve_member_source_instance(self, _member: ContextApplicationMemberV3) -> object:
        if (
            _member.candidate_id != CANDIDATE_ID
            or _member.source_instance_id != SOURCE_INSTANCE_ID
            or _member.candidate_identity_digest_reference.digest_bytes != CANDIDATE_DIGEST
        ):
            raise ValueError("candidate/source instance substitution")
        return self._source_view

    def resolve_candidate_records(
        self, _record: ContextApplicationV3Record
    ) -> dict[str, Mapping[str, object]]:
        resolved = self.resolve_member_source_instance(_record.members[0])
        return {CANDIDATE_ID: resolved.candidate.candidate_record}

    def resolve_acceptance_event_leaf_v4(self, reference: object) -> ReviewAcceptanceEventLeafV4:
        if getattr(reference, "event_id", None) == self._rpa_event.event_id.as_text():
            return self._rpa_event
        if self._context_event is None:
            raise AssertionError("context event not installed")
        return self._context_event

    def resolve_v4_source_binding(self, _binding: object) -> object:
        return object()

    def resolve_v4_acceptance_evidence(self, _evidence: object) -> object:
        return object()

    def _review_event_ref_v1(self, _theorem: Mapping[str, object]) -> object:
        return object()

    def _v1_event_closure(
        self, _reference: object, seen: set[str] | None = None
    ) -> list[ReviewAuthoritySourceBindingV4]:
        return [self._v1_leaf]


def _v3_binding(binding: ReviewAuthoritySourceBindingV4) -> ContextAuthoritySourceBindingV3:
    return ContextAuthoritySourceBindingV3(
        binding.artifact_role, binding.path, binding.schema, binding.raw_sha256
    )


def _relation_source_binding(
    role: str, path: str, schema: str | None, digest: bytes
) -> RelationAuthoritySourceBindingV2:
    return RelationAuthoritySourceBindingV2(role, path, schema, digest)


def _synthetic_base_authority(
    source_resolver: Candidate4SourceResolver,
) -> HostBindingSourceBindingV2:
    """Create one digest-bound, test-only V1 envelope for HBC admission."""

    model_path = "sources/m2_5/closures/C/declared_interaction_model.v2.json"
    model_digest = _repo_digest(model_path)
    document = {
        "schema": "manafold.m2.5.c.interaction-review-authority.v1",
        "model_binding": {
            "path": model_path,
            "raw_sha256": model_digest.hex(),
            "model_id": "declared-interaction-model.v2",
            "model_version": "2",
        },
        "source_bindings": [
            {
                "authority_kind": "model",
                "artifact_role": "declared_model",
                "path": model_path,
                "schema_or_null": "manafold.m2.5.c.declared-interaction-model.v2",
                "raw_sha256": model_digest.hex(),
            }
        ],
        "relation_proofs": [],
        "relation_applications": [],
        "domain_proofs": [],
        "domain_applications": [],
        "context_proofs": [],
        "context_applications": [],
        "supersession_records": [],
    }
    path = "sources/m2_5/authorities/interaction_review_authority.v1.json"
    digest = source_resolver.bind_json_artifact(
        path, "manafold.m2.5.c.interaction-review-authority.v1", document
    )
    return HostBindingSourceBindingV2(
        "base_authority_v1",
        path,
        "manafold.m2.5.c.interaction-review-authority.v1",
        digest,
    )


def _candidate4_host_claim(
    source_resolver: Candidate4SourceResolver,
    member_key: ApplicationMemberKeyV1,
    base_binding: HostBindingSourceBindingV2,
    model_binding: HostBindingSourceBindingV2,
    candidate_binding: HostBindingSourceBindingV2,
) -> tuple[
    CrossDeckHostBindingClaimRecordV1,
    tuple[HostBindingSourceBindingV2, ...],
]:
    map_binding = HostBindingSourceBindingV2(
        "rev3_card_requirement_map",
        REV3_CARD_REQUIREMENT_MAP_PATH,
        None,
        REV3_CARD_REQUIREMENT_MAP_DIGEST,
    )
    deck_binding = HostBindingSourceBindingV2(
        "rev3_deck_row_source_resolution",
        REV3_DECK_ROW_SOURCE_RESOLUTION_PATH,
        None,
        REV3_DECK_ROW_SOURCE_RESOLUTION_DIGEST,
    )
    osi_binding = HostBindingSourceBindingV2(
        "rev3_osi_source_records",
        REV3_OSI_SOURCE_RECORDS_PATH,
        None,
        REV3_OSI_SOURCE_RECORDS_DIGEST,
    )
    pair_binding = HostBindingSourceBindingV2(
        "rev3_pair_aggregates",
        REV3_PAIR_AGGREGATES_PATH,
        None,
        REV3_PAIR_AGGREGATES_DIGEST,
    )
    b2_catalog = HostBindingSourceBindingV2(
        "b2_catalog",
        "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
        "manafold.m2.5.b2.requirement-family-catalog.v1",
        _repo_digest("sources/m2_5/closures/B2/requirement_family_catalog.v1.json"),
    )
    b2_classifications = HostBindingSourceBindingV2(
        "b2_classifications",
        "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
        "manafold.m2.5.b2.card-semantic-classifications.v1",
        _repo_digest("sources/m2_5/closures/B2/card_semantic_classifications.v1.json"),
    )
    b2_closure = HostBindingSourceBindingV2(
        "b2_closure",
        "sources/m2_5/closures/B2/classification_closure.v1.json",
        "manafold.m2.5.b2.classification-closure.v1",
        _repo_digest("sources/m2_5/closures/B2/classification_closure.v1.json"),
    )

    participant_facts = (
        (
            0,
            "cap.mass_destruction",
            "rev3_left_family",
            "Buckle Up",
            15,
            30,
            349,
            340,
        ),
        (
            1,
            "cap.death_trigger",
            "rev3_right_family",
            "Upgrades Unleashed",
            432,
            421,
            344,
            0,
        ),
    )
    discoveries: list[CrossDeckParticipantDiscoveryHostBindingV1] = []
    realizations: list[ParticipantHostRealizationV1] = []
    for (
        position,
        participant_ref,
        side,
        host_id,
        map_row,
        deck_row,
        osi_line,
        b2_row,
    ) in participant_facts:
        mapping_ref = HostBindingEvidenceRefV2(
            "rev3_card_requirement_map",
            REV3_CARD_REQUIREMENT_MAP_PATH,
            None,
            REV3_CARD_REQUIREMENT_MAP_DIGEST,
            ("csv_row", map_row),
        )
        deck_ref = HostBindingEvidenceRefV2(
            "rev3_deck_row_source_resolution",
            REV3_DECK_ROW_SOURCE_RESOLUTION_PATH,
            None,
            REV3_DECK_ROW_SOURCE_RESOLUTION_DIGEST,
            ("csv_row", deck_row),
        )
        osi_ref = HostBindingEvidenceRefV2(
            "rev3_osi_source_records",
            REV3_OSI_SOURCE_RECORDS_PATH,
            None,
            REV3_OSI_SOURCE_RECORDS_DIGEST,
            ("jsonl_line", osi_line),
        )
        b2_ref = HostBindingEvidenceRefV2(
            "b2_classifications",
            b2_classifications.path,
            b2_classifications.schema_or_null,
            b2_classifications.raw_sha256,
            ("json_pointer", f"/classifications/{b2_row}"),
        )
        host = DiscoveryHostRefV1("rev3_deck", host_id)
        discoveries.append(
            CrossDeckParticipantDiscoveryHostBindingV1(
                member_key,
                position,
                participant_ref,
                side,
                host,
                (mapping_ref,),
            )
        )
        realizations.append(
            ParticipantHostRealizationV1(
                member_key,
                position,
                participant_ref,
                host,
                (HostRealizationWitnessV1(mapping_ref, deck_ref, osi_ref, (b2_ref,)),),
            )
        )
    claim = CrossDeckHostBindingClaimV1(
        member_key,
        tuple(discoveries),
        tuple(realizations),
        "cross_host",
    )

    roster_raw = (
        json.dumps(
            {
                "schema": "manafold.m2.5.c.reviewer-roster.v1",
                "reviewers": [
                    {
                        "reviewer_id": "candidate4-host-reviewer",
                        "roles": ["architecture_maintainer", "project_owner"],
                    }
                ],
            },
            separators=(",", ":"),
            sort_keys=True,
        ).encode("utf-8")
        + b"\n"
    )
    roster_path = (
        "sources/m2_5/authorities/reviewer_rosters/v1/"
        + hashlib.sha256(roster_raw).hexdigest()
        + ".json"
    )
    roster_digest = source_resolver.bind_raw_artifact(
        roster_path,
        "manafold.m2.5.c.reviewer-roster.v1",
        roster_raw,
        json_value=json.loads(roster_raw),
    )
    roster_ref = ReviewerRosterRefV1(
        roster_path, "manafold.m2.5.c.reviewer-roster.v1", roster_digest
    )
    evidence_raw = b"candidate-4 host-binding synthetic review evidence\n"
    evidence_path = "conformance/fixtures/authority/candidate_4_host_review.txt"
    evidence_digest = source_resolver.bind_raw_artifact(evidence_path, None, evidence_raw)
    evidence_ref = AcceptanceEvidenceRefV1(evidence_path, evidence_digest, ("whole_artifact", None))
    event_sources = tuple(
        sorted(
            (
                model_binding,
                candidate_binding,
                pair_binding,
                map_binding,
                deck_binding,
                osi_binding,
                b2_catalog,
                b2_classifications,
                b2_closure,
                HostBindingSourceBindingV2(
                    "reviewer_roster_leaf",
                    roster_path,
                    "manafold.m2.5.c.reviewer-roster.v1",
                    roster_digest,
                ),
            ),
            key=lambda item: encode_canonical(item.to_cbor()),
        )
    )
    event_input = HostBindingAcceptanceEventInputV2(
        "cross_deck_host_binding_claim_record_v1",
        claim.identity().digest_bytes,
        roster_ref,
        (
            ReviewerRoleBindingV1(
                "candidate4-host-reviewer", ("architecture_maintainer", "project_owner")
            ),
        ),
        ReviewMode.MULTI_REVIEWER,
        "cross-deck-host-binding-review-checklist.v1",
        event_sources,
        (evidence_ref,),
    )
    event = HostBindingAcceptanceEventLeafV2.from_input(event_input)
    event_path = (
        "sources/m2_5/authorities/review_acceptance_events/v2/"
        + event.event_id.digest_bytes.hex()
        + ".json"
    )
    event_raw = (json.dumps(event.to_wire(), separators=(",", ":"), sort_keys=True) + "\n").encode(
        "utf-8"
    )
    event_digest = source_resolver.bind_raw_artifact(
        event_path,
        "manafold.m2.5.c.review-acceptance-event.v2",
        event_raw,
        json_value=json.loads(event_raw),
    )
    event_ref = HostBindingAcceptanceEventRefV2(event_path, event_digest, event.event_id.as_text())
    record = CrossDeckHostBindingClaimRecordV1(claim, event_ref)
    return record, tuple(
        sorted(
            (
                base_binding,
                *event_sources,
                HostBindingSourceBindingV2(
                    "acceptance_event_leaf_v2",
                    event_path,
                    "manafold.m2.5.c.review-acceptance-event.v2",
                    event_digest,
                ),
            ),
            key=lambda item: encode_canonical(item.to_cbor()),
        )
    )


def build_bundle() -> Candidate4Bundle:
    global _CANDIDATE4_RESOLVED_SOURCE
    if not os.environ.get(REV3_ARCHIVE_ENV_VAR):
        raise unittest.SkipTest(
            f"{REV3_ARCHIVE_ENV_VAR} is unavailable; real Candidate-4 source probe is blocked"
        )
    source_resolver = _candidate4_source_resolver()
    if _CANDIDATE4_RESOLVED_SOURCE is None:
        _CANDIDATE4_RESOLVED_SOURCE = source_resolver.resolve_candidate_source_instance(
            CANDIDATE_ID,
            _candidate_identity().to_wire(),
            SOURCE_INSTANCE_ID,
            SourceBindingDigestV1(
                "candidate_universe",
                CANDIDATE_UNIVERSE_PATH,
                CANDIDATE_UNIVERSE_SCHEMA,
                CANDIDATE_UNIVERSE_DIGEST,
            ),
        )
    source = _CANDIDATE4_RESOLVED_SOURCE
    candidate = source.candidate.candidate_record
    base_v2 = _synthetic_base_authority(source_resolver)
    rpa_theorem_id = AuthorityIdentityV1(AuthorityIdentityKind.RELATION_THEOREM_RECORD, b"t" * 32)
    rpa_app = RelationApplicationV2(
        rpa_theorem_id.digest_bytes, "required_interaction", (_rpa_member(),)
    )
    rpa_roster = _roster(b"r" * 32)
    rpa_sources = tuple(
        sorted(
            (
                _binding_v4("candidate_universe", CANDIDATE_UNIVERSE_DIGEST),
                _binding_v4(
                    "declared_model",
                    _repo_digest("sources/m2_5/closures/C/declared_interaction_model.v2.json"),
                ),
                _binding_v4("acceptance_event_leaf_v1", b"1" * 32),
                _binding_v4("reviewer_roster_leaf", rpa_roster.raw_sha256),
            ),
            key=lambda item: encode_canonical(item.to_cbor()),
        )
    )
    rpa_event = _event(
        AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
        [
            "relation_application_v2_record",
            rpa_app.identity().digest_bytes,
            rpa_theorem_id.digest_bytes,
            "required_interaction",
            [rpa_app.members[0].to_cbor()],
        ],
        rpa_roster,
        rpa_sources,
    )
    rpa_ref = ReviewEventRefV4(
        "sources/m2_5/authorities/review_acceptance_events/v4/"
        + rpa_event.event_id.digest_bytes.hex()
        + ".json",
        b"a" * 32,
        rpa_event.event_id.as_text(),
    )
    rpa_record = RelationApplicationV2Record.from_parts(
        rpa_app.identity(), rpa_theorem_id, "required_interaction", rpa_app.members, rpa_ref
    )
    relation_sources = tuple(
        sorted(
            (
                _relation_source_binding(
                    "base_authority_v1",
                    "sources/m2_5/authorities/interaction_review_authority.v1.json",
                    "manafold.m2.5.c.interaction-review-authority.v1",
                    base_v2.raw_sha256,
                ),
                _relation_source_binding(
                    "candidate_universe",
                    "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
                    "manafold.m2.5.c.interaction-candidate-universe.v2",
                    CANDIDATE_UNIVERSE_DIGEST,
                ),
                _relation_source_binding(
                    "declared_model",
                    "sources/m2_5/closures/C/declared_interaction_model.v2.json",
                    "manafold.m2.5.c.declared-interaction-model.v2",
                    _repo_digest("sources/m2_5/closures/C/declared_interaction_model.v2.json"),
                ),
                _relation_source_binding(
                    "rev3_pair_aggregates",
                    REV3_PAIR_AGGREGATES_PATH,
                    None,
                    REV3_PAIR_AGGREGATES_DIGEST,
                ),
                _relation_source_binding(
                    "rev3_card_requirement_map",
                    REV3_CARD_REQUIREMENT_MAP_PATH,
                    None,
                    REV3_CARD_REQUIREMENT_MAP_DIGEST,
                ),
                _relation_source_binding(
                    "rev3_deck_row_source_resolution",
                    REV3_DECK_ROW_SOURCE_RESOLUTION_PATH,
                    None,
                    REV3_DECK_ROW_SOURCE_RESOLUTION_DIGEST,
                ),
                _relation_source_binding(
                    "rev3_osi_source_records",
                    REV3_OSI_SOURCE_RECORDS_PATH,
                    None,
                    REV3_OSI_SOURCE_RECORDS_DIGEST,
                ),
                _relation_source_binding(
                    "b2_catalog",
                    "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
                    "manafold.m2.5.b2.requirement-family-catalog.v1",
                    _repo_digest("sources/m2_5/closures/B2/requirement_family_catalog.v1.json"),
                ),
                _relation_source_binding(
                    "b2_classifications",
                    "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
                    "manafold.m2.5.b2.card-semantic-classifications.v1",
                    _repo_digest("sources/m2_5/closures/B2/card_semantic_classifications.v1.json"),
                ),
                _relation_source_binding(
                    "b2_closure",
                    "sources/m2_5/closures/B2/classification_closure.v1.json",
                    "manafold.m2.5.b2.classification-closure.v1",
                    _repo_digest("sources/m2_5/closures/B2/classification_closure.v1.json"),
                ),
            ),
            key=lambda item: encode_canonical(item.to_cbor()),
        )
    )
    rpa_resolver = SyntheticRpaResolver(
        rpa_record,
        rpa_event,
        source,
        _rpa_theorem(),
        relation_sources,
        rpa_sources,
        source_resolver,
    )
    rpa_authority = RelationApplicationAuthorityV2(
        next(item for item in relation_sources if item.artifact_role == "base_authority_v1"),
        next(item for item in relation_sources if item.artifact_role == "candidate_universe"),
        relation_sources,
        (rpa_record,),
        (),
    )
    rpa_resolver.authority = rpa_authority
    rpa_path = (
        "sources/m2_5/authorities/relation_application_authority/v2/"
        "relation_application_authority.v2.json"
    )
    rpa_digest = source_resolver.bind_json_artifact(
        rpa_path,
        "manafold.m2.5.c.relation-application-authority.v2",
        rpa_authority.to_wire(),
    )
    base = ContextAuthoritySourceBindingV3(
        "base_authority_v1",
        base_v2.path,
        base_v2.schema_or_null,
        base_v2.raw_sha256,
    )
    candidate_binding = ContextAuthoritySourceBindingV3(
        "candidate_universe",
        CANDIDATE_UNIVERSE_PATH,
        CANDIDATE_UNIVERSE_SCHEMA,
        CANDIDATE_UNIVERSE_DIGEST,
    )
    relation_projection = ContextAuthoritySourceBindingV3(
        "relation_authority_v2",
        rpa_path,
        "manafold.m2.5.c.relation-application-authority.v2",
        rpa_digest,
    )
    claim_key = ApplicationMemberKeyV1(CANDIDATE_ID, CANDIDATE_DIGEST, SOURCE_INSTANCE_ID)
    host_record, host_sources = _candidate4_host_claim(
        source_resolver,
        claim_key,
        base_v2,
        HostBindingSourceBindingV2(
            "declared_model",
            "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            "manafold.m2.5.c.declared-interaction-model.v2",
            _repo_digest("sources/m2_5/closures/C/declared_interaction_model.v2.json"),
        ),
        HostBindingSourceBindingV2(
            "candidate_universe",
            CANDIDATE_UNIVERSE_PATH,
            CANDIDATE_UNIVERSE_SCHEMA,
            CANDIDATE_UNIVERSE_DIGEST,
        ),
    )
    host_path = "sources/m2_5/authorities/interaction_review_authority.v2.json"
    host_document = {
        "schema": "manafold.m2.5.c.interaction-review-authority.v2",
        "base_authority_v1_binding": base_v2.to_wire(),
        "source_bindings": [item.to_wire() for item in host_sources],
        "cross_deck_host_binding_claim_records": [host_record.to_wire()],
        "cross_deck_host_binding_claim_supersession_records": [],
        "application_host_bindings": [],
    }
    host_digest = source_resolver.bind_json_artifact(
        host_path,
        "manafold.m2.5.c.interaction-review-authority.v2",
        host_document,
    )
    host_projection = ContextAuthoritySourceBindingV3(
        "host_binding_authority_v2",
        host_path,
        "manafold.m2.5.c.interaction-review-authority.v2",
        host_digest,
    )
    host_model = admit_host_binding_authority_v2(source_resolver, host_projection)
    rpa_member_resolver = ContextApplicationV3RpaResolver(
        rpa_authority, rpa_resolver, currentness=rpa_resolver
    )
    context_member = _context_member(rpa_id=rpa_app.identity().digest_bytes)
    context_theorem_id = AuthorityIdentityV1(
        AuthorityIdentityKind.CONTEXT_THEOREM_RECORD, b"k" * 32
    )
    context_app = ContextApplicationV3InputV1(context_theorem_id.digest_bytes, (context_member,))
    context_record_placeholder = ContextApplicationV3Record.from_parts(
        context_app.identity(),
        context_theorem_id,
        context_app.members,
        ReviewEventRefV4(
            "sources/m2_5/authorities/review_acceptance_events/v4/" + "0" * 64 + ".json",
            b"0" * 32,
            "ae.v4/" + "0" * 64,
        ),
    )
    context_resolver = SyntheticContextResolver(
        rpa_member_resolver,
        source,
        _context_theorem(),
        candidate,
        rpa_event,
        source_resolver,
    )
    context_roster = _roster(b"s" * 32)
    context_closure = context_resolver.expected_context_application_v3_source_closure(
        context_record_placeholder, context_roster
    )
    context_event = _event(
        AcceptanceSubjectKindV4.CONTEXT_APPLICATION_V3_RECORD,
        context_record_placeholder.acceptance_free_subject_payload(),
        context_roster,
        context_closure,
    )
    context_ref = ReviewEventRefV4(
        "sources/m2_5/authorities/review_acceptance_events/v4/"
        + context_event.event_id.digest_bytes.hex()
        + ".json",
        b"d" * 32,
        context_event.event_id.as_text(),
    )
    context_record = ContextApplicationV3Record.from_parts(
        context_app.identity(), context_theorem_id, context_app.members, context_ref
    )
    context_resolver._context_event = context_event
    claim = host_record.claim
    claim_id = claim.identity().as_text()
    host_link = ApplicationHostBindingV3(
        "context_application_v3", context_record.application_id, (claim_id,)
    )
    context_source_values: dict[bytes, ContextAuthoritySourceBindingV3] = {
        encode_canonical(item.to_cbor()): item
        for item in (base, candidate_binding, relation_projection, host_projection)
    }
    context_event_binding = ContextAuthoritySourceBindingV3(
        "acceptance_event_leaf_v4",
        context_ref.path,
        "manafold.m2.5.c.review-acceptance-event.v4",
        context_ref.raw_sha256,
    )
    context_source_values[encode_canonical(context_event_binding.to_cbor())] = context_event_binding
    for item in context_closure:
        context_source_values[encode_canonical(_v3_binding(item).to_cbor())] = _v3_binding(item)
    for item in relation_sources:
        projected = ContextAuthoritySourceBindingV3(
            item.artifact_role, item.path, item.schema, item.raw_sha256
        )
        context_source_values[encode_canonical(projected.to_cbor())] = projected
    for item in host_sources:
        projected = ContextAuthoritySourceBindingV3(
            item.artifact_role, item.path, item.schema_or_null, item.raw_sha256
        )
        context_source_values[encode_canonical(projected.to_cbor())] = projected
    context_authority = ContextApplicationAuthorityV3(
        base,
        candidate_binding,
        tuple(
            sorted(
                context_source_values.values(), key=lambda item: encode_canonical(item.to_cbor())
            )
        ),
        relation_projection,
        relation_sources,
        host_projection,
        host_sources,
        (context_record,),
        (),
        (host_link,),
    )
    source_snapshot = _plain(source.source_instance_record)
    return Candidate4Bundle(
        rpa_app.members[0],
        rpa_record,
        rpa_event,
        rpa_authority,
        context_member,
        context_record,
        context_event,
        context_authority,
        context_resolver,
        rpa_resolver,
        host_model,
        host_link,
        context_closure,
        rpa_sources,
        source_snapshot,
        source_resolver,
    )


class Candidate4AuthorityContractTests(unittest.TestCase):
    def test_synthetic_authority_artifacts_require_exact_bound_digest(self) -> None:
        resolver = Candidate4SourceResolver()
        path = (
            "sources/m2_5/authorities/relation_application_authority/v2/"
            "relation_application_authority.v2.json"
        )
        schema = "manafold.m2.5.c.relation-application-authority.v2"
        digest = resolver.bind_json_artifact(path, schema, {"schema": schema})
        self.assertIsNotNone(resolver.resolve_repository_artifact(path, digest, schema))
        with self.assertRaises(ValueError):
            resolver.resolve_repository_artifact(path, b"q" * 32, schema)

    def test_exact_candidate_4_lock_and_full_positive_path(self) -> None:
        lock = _fixture("candidate_4_role_bridge_golden.v1.json")
        self.assertEqual(lock["candidate_id"], CANDIDATE_ID)
        self.assertEqual(lock["candidate_identity"], CANDIDATE_DIGEST.hex())
        self.assertEqual(lock["rev3_row_ordinal"], 6463)
        self.assertEqual(lock["candidate_universe_raw_sha256"], CANDIDATE_UNIVERSE_DIGEST.hex())
        self.assertEqual(lock["source_instance_id"], SOURCE_INSTANCE_ID)
        self.assertEqual(lock["source_binding"]["row_ordinal"], 6463)
        self.assertEqual(
            lock["source_binding"]["archive_member_sha256"],
            "82f9312113bb1007ad6562d454c515f85dbc1e0d7a471f7b1c6793725aea45d4",
        )
        bundle = build_bundle()
        source_before = copy.deepcopy(bundle.source_instance_snapshot)
        admitted_claims = dict(bundle.host_read_model.admitted_claims_by_id)
        current_claims = dict(bundle.host_read_model.current_claims_by_id)
        claim_id = bundle.host_link.host_binding_claim_ids[0]
        self.assertIn(claim_id, admitted_claims)
        self.assertIs(current_claims[claim_id], admitted_claims[claim_id])
        self.assertTrue(
            any(
                record_ids
                for _claim, record_ids in bundle.host_read_model.claim_record_ids_by_claim_id
            )
        )
        rpa_admission = admit_relation_application_v2_record(
            bundle.rpa_record, bundle.relation_resolver, currentness=bundle.relation_resolver
        )
        self.assertTrue(rpa_admission.semantic_validation.valid)
        context_admission = admit_context_application_v3_record(
            bundle.context_record, bundle.context_resolver
        )
        self.assertTrue(context_admission.semantic_validation.valid)
        currentness = ContextApplicationV3AuthorityResolver(
            bundle.context_resolver
        ).evaluate_authority(
            bundle.context_authority,
            relation_authority=bundle.rpa_authority,
            v2_records=(),
            v2_supersession_records=(),
        )
        self.assertEqual(
            tuple(item.as_text() for item in currentness.current_record_ids),
            (bundle.context_record.record_id.as_text(),),
        )
        self.assertEqual(
            bundle.rpa_member.participant_role_bridge_v1.entries[0].historical_source_role,
            "ordered_participant",
        )
        self.assertEqual(
            bundle.rpa_member.participant_role_bridge_v1.entries[0].reviewed_role, "source"
        )
        self.assertEqual(
            bundle.rpa_member.participant_role_bridge_v1.entries[1].reviewed_role, "affected"
        )
        source_roundtrip = bundle.source_resolver.resolve_candidate_source_instance(
            CANDIDATE_ID,
            _candidate_identity().to_wire(),
            SOURCE_INSTANCE_ID,
            SourceBindingDigestV1(
                "candidate_universe",
                CANDIDATE_UNIVERSE_PATH,
                CANDIDATE_UNIVERSE_SCHEMA,
                CANDIDATE_UNIVERSE_DIGEST,
            ),
        )
        self.assertEqual(source_before, _plain(source_roundtrip.source_instance_record))
        self.assertEqual(source_roundtrip.source_binding["row_ordinal"], 6463)
        self.assertEqual(
            source_roundtrip.source_binding["archive_member_sha256"],
            "82f9312113bb1007ad6562d454c515f85dbc1e0d7a471f7b1c6793725aea45d4",
        )
        self.assertEqual(
            source_roundtrip.candidate.candidate_universe.raw_sha256,
            CANDIDATE_UNIVERSE_DIGEST.hex(),
        )

    def test_deterministic_repeatability_and_golden_identities(self) -> None:
        first = build_bundle()
        second = build_bundle()

        def values(item: AuthorityIdentityV1) -> tuple[str, object]:
            return item.as_text(), item.to_cbor()

        self.assertEqual(
            values(first.rpa_record.application_id), values(second.rpa_record.application_id)
        )
        self.assertEqual(
            first.rpa_record.record_id.as_text(), second.rpa_record.record_id.as_text()
        )
        self.assertEqual(
            first.context_record.application_id.as_text(),
            second.context_record.application_id.as_text(),
        )
        self.assertEqual(
            first.context_record.record_id.as_text(), second.context_record.record_id.as_text()
        )
        self.assertEqual(
            encode_canonical(first.rpa_member.participant_role_bridge_v1.to_cbor()),
            encode_canonical(second.rpa_member.participant_role_bridge_v1.to_cbor()),
        )
        self.assertEqual(
            tuple(item.to_cbor() for item in first.context_closure),
            tuple(item.to_cbor() for item in second.context_closure),
        )
        rpa_golden = _fixture("candidate_4_rpa_v2_golden.v1.json")
        context_golden = _fixture("candidate_4_context_v3_golden.v1.json")
        self.assertEqual(rpa_golden["rpa_v2"], first.rpa_record.application_id.as_text())
        self.assertEqual(rpa_golden["rpar_v2"], first.rpa_record.record_id.as_text())
        self.assertEqual(
            rpa_golden["bridge_cbor_hex"],
            encode_canonical(first.rpa_member.participant_role_bridge_v1.to_cbor()).hex(),
        )
        self.assertEqual(
            rpa_golden["subject_identity"],
            first.rpa_event.subject_payload_digest_reference.digest_bytes.hex(),
        )
        self.assertEqual(context_golden["cpa_v3"], first.context_record.application_id.as_text())
        self.assertEqual(context_golden["cpar_v3"], first.context_record.record_id.as_text())
        self.assertEqual(
            context_golden["subject_identity"],
            first.context_event.subject_payload_digest_reference.digest_bytes.hex(),
        )
        self.assertEqual(
            context_golden["application_host_binding_v3_cbor_hex"],
            encode_canonical(first.host_link.to_cbor()).hex(),
        )

    def test_negative_matrix_is_executable(self) -> None:
        matrix = _fixture("candidate_4_negative_matrix.v1.json")
        self.assertEqual({case["case_id"] for case in matrix["cases"]}, set("ABCDEFGHIJKLMNOPQRST"))
        for case in matrix["cases"]:
            with self.subTest(case=case["case_id"]):
                self._run_negative(str(case["mutation"]))

    def _run_negative(self, mutation: str) -> None:
        bundle = build_bundle()
        if mutation in {
            "candidate_identity_mismatch",
            "wrong_candidate_substitution",
            "bridge_historical_role_mismatch",
            "bridge_reviewed_role_mismatch",
            "bridge_participant_kind_mismatch",
            "bridge_semantic_ref_mismatch",
            "position_role_inference",
            "reviewed_role_mutation",
        }:
            member = bundle.rpa_member
            if mutation == "candidate_identity_mismatch":
                member = replace(
                    member,
                    candidate_identity_digest_reference=DigestReferenceV1(
                        "mtgml.digest-envelope.v1",
                        "sha-256",
                        "manafold.m2.5.c.candidate-identity.v1",
                        "mtgml.canonical-cbor.v1",
                        "manafold.m2.5.c.candidate-identity-input.v1",
                        b"x" * 32,
                    ),
                )
            elif mutation == "wrong_candidate_substitution":
                member = replace(
                    member, candidate_id="CROSS_DECK|P3|wrong|candidate|DIRECTIONAL_BINARY"
                )
            elif mutation in {"historical_source_role_mutation", "bridge_historical_role_mismatch"}:
                member = replace(
                    member,
                    participant_role_bridge_v1=_bridge(
                        historical=("source", "ordered_participant")
                    ),
                )
            elif mutation == "bridge_reviewed_role_mismatch":
                member = replace(
                    member, participant_role_bridge_v1=_bridge(reviewed=("affected", "source"))
                )
            elif mutation == "bridge_participant_kind_mismatch":
                member = replace(
                    member, participant_role_bridge_v1=_bridge(participant_kind="card")
                )
            elif mutation == "bridge_semantic_ref_mismatch":
                member = replace(
                    member,
                    participant_role_bridge_v1=_bridge(
                        semantic_refs=("cap.other", "cap.death_trigger")
                    ),
                )
            elif mutation == "position_role_inference":
                member = replace(
                    member, participant_role_bridge_v1=_bridge(historical=("source", "affected"))
                )
            else:
                member = replace(
                    member, reviewed_relation_binding_v1=_relation_binding(("affected", "source"))
                )
            application = RelationApplicationV2(
                bundle.rpa_record.theorem_record_id.digest_bytes, "required_interaction", (member,)
            )
            with self.assertRaises(RelationApplicationV2SemanticValidationError):
                validate_relation_application_v2_semantics(
                    application, bundle.relation_resolver, theorem_record=_rpa_theorem()
                )
            return
        if mutation == "historical_source_role_mutation":
            bundle.relation_resolver.source_override = _resolved_source(
                roles=("source", "ordered_participant")
            )
            application = RelationApplicationV2(
                bundle.rpa_record.theorem_record_id.digest_bytes,
                "required_interaction",
                (bundle.rpa_member,),
            )
            with self.assertRaises(RelationApplicationV2SemanticValidationError):
                validate_relation_application_v2_semantics(
                    application,
                    bundle.relation_resolver,
                    theorem_record=_rpa_theorem(),
                )
            return
        if mutation == "stale_rpa":
            from relation_application_v2_resolver import RelationApplicationV2ResolutionError

            for status, code in (
                ("stale", "SUPERSEDED_AUTHORITY_USED"),
                ("revoked", "SUPERSEDED_AUTHORITY_USED"),
                ("ambiguous", "RELATION_APPLICATION_V2_CURRENTNESS_AMBIGUOUS"),
            ):
                with (
                    self.subTest(status=status),
                    self.assertRaises(ContextApplicationV3ResolutionError),
                ):
                    ContextApplicationV3RpaResolver(
                        bundle.rpa_authority,
                        bundle.relation_resolver,
                        currentness=SimpleNamespace(
                            require_current_relation_theorem=lambda _id, code=code: (
                                (_ for _ in ()).throw(
                                    RelationApplicationV2ResolutionError(code, "theorem_record_id")
                                )
                            )
                        ),
                    )
            return
        if mutation == "wrong_rpa_identity":
            member = replace(bundle.context_member, relation_application_v2_id_bytes=b"z" * 32)
            app = ContextApplicationV3InputV1(
                bundle.context_record.theorem_record_id.digest_bytes, (member,)
            )
            bad = ContextApplicationV3Record.from_parts(
                app.identity(),
                bundle.context_record.theorem_record_id,
                app.members,
                bundle.context_record.review_event_ref_v4,
            )
            with self.assertRaises(ContextApplicationV3SemanticValidationError):
                ContextApplicationV3SemanticValidator(bundle.context_resolver).validate(bad)
            return
        if mutation == "invalid_v4_admission":
            old = bundle.context_resolver._context_event
            bundle.context_resolver._context_event = _event(
                AcceptanceSubjectKindV4.RELATION_APPLICATION_V2_RECORD,
                [
                    "relation_application_v2_record",
                    b"x" * 32,
                    b"y" * 32,
                    "required_interaction",
                    [bundle.rpa_member.to_cbor()],
                ],
                _roster(b"s" * 32),
                bundle.context_closure,
            )
            try:
                with self.assertRaises(ContextApplicationV3ReviewAdmissionError):
                    admit_context_application_v3_record(
                        bundle.context_record, bundle.context_resolver
                    )
            finally:
                bundle.context_resolver._context_event = old
            return
        if mutation in {
            "missing_context_source",
            "extra_relation_source",
            "shared_snapshot_mismatch",
        }:
            authority = bundle.context_authority
            if mutation == "missing_context_source":
                source_bindings = authority.source_bindings[:-1]
            elif mutation == "extra_relation_source":
                relation_sources = tuple(
                    sorted(
                        [
                            *authority.relation_source_bindings,
                            _relation_source_binding(
                                "rev3_source_index",
                                "source/raw/source_record_index_REV3.csv",
                                None,
                                b"x" * 32,
                            ),
                        ],
                        key=lambda item: encode_canonical(item.to_cbor()),
                    )
                )
                authority = replace(authority, relation_source_bindings=relation_sources)
                source_bindings = authority.source_bindings
            else:
                bad_base = ContextAuthoritySourceBindingV3(
                    "base_authority_v1",
                    authority.base_authority_v1_binding.path,
                    authority.base_authority_v1_binding.schema,
                    b"x" * 32,
                )
                source_bindings = tuple(
                    bad_base if item.artifact_role == "base_authority_v1" else item
                    for item in authority.source_bindings
                )
                authority = replace(
                    authority,
                    base_authority_v1_binding=bad_base,
                    source_bindings=tuple(
                        sorted(source_bindings, key=lambda item: encode_canonical(item.to_cbor()))
                    ),
                )
            if mutation == "missing_context_source":
                authority = replace(authority, source_bindings=source_bindings)
            with self.assertRaises(ContextApplicationV3ResolutionError):
                ContextApplicationV3AuthorityResolver(
                    bundle.context_resolver
                ).validate_source_closure(
                    authority,
                    relation_authority=bundle.rpa_authority,
                    host_read_model=bundle.host_read_model,
                )
            return
        if mutation in {"missing_host_link", "stale_host_claim", "wrong_host_member_key"}:
            if mutation == "missing_host_link":

                class EmptyV2Currentness:
                    def __init__(self, *_args: object, **_kwargs: object) -> None:
                        pass

                    def evaluate(self, *_args: object, **_kwargs: object) -> object:
                        return SimpleNamespace(current_record_ids=())

                no_host_source_roles = {
                    "host_binding_authority_v2",
                    "host_binding_claim_record",
                }
                no_host_authority = replace(
                    bundle.context_authority,
                    source_bindings=tuple(
                        item
                        for item in bundle.context_authority.source_bindings
                        if item.artifact_role not in no_host_source_roles
                    ),
                    host_binding_authority_v2_binding=None,
                    host_binding_source_bindings=(),
                    application_host_bindings_v3=(),
                )
                with (
                    patch(
                        "context_application_v2_supersession.ContextApplicationV2CurrentnessEvaluator",
                        EmptyV2Currentness,
                    ),
                    self.assertRaises(ContextApplicationV3ResolutionError),
                ):
                    ContextApplicationV3AuthorityResolver(
                        bundle.context_resolver
                    ).evaluate_authority(
                        no_host_authority,
                        relation_authority=bundle.rpa_authority,
                        v2_records=(),
                        v2_supersession_records=(),
                    )
            elif mutation == "stale_host_claim":
                stale = replace(bundle.host_read_model, current_claims_by_id=())
                with self.assertRaises(ContextApplicationV3HostBindingError):
                    validate_application_host_binding_v3(
                        bundle.context_record,
                        bundle.host_link,
                        stale,
                        bundle.context_resolver.resolve_candidate_records(bundle.context_record),
                    )
            else:
                wrong = _cross_host_claim(
                    ApplicationMemberKeyV1(CANDIDATE_ID, b"z" * 32, SOURCE_INSTANCE_ID)
                )
                bad = replace(
                    bundle.host_read_model,
                    admitted_claims_by_id=((bundle.host_link.host_binding_claim_ids[0], wrong),),
                    current_claims_by_id=((bundle.host_link.host_binding_claim_ids[0], wrong),),
                )
                with self.assertRaises(ContextApplicationV3HostBindingError):
                    validate_application_host_binding_v3(
                        bundle.context_record,
                        bundle.host_link,
                        bad,
                        bundle.context_resolver.resolve_candidate_records(bundle.context_record),
                    )
            return
        if mutation == "v2_v3_current_collision":
            key = (CANDIDATE_DIGEST, SOURCE_INSTANCE_ID)
            with self.assertRaises(ContextApplicationV3ResolutionError):
                ContextApplicationV3AuthorityResolver.require_cross_version_eligibility(
                    v2_member_facts=((key, ("ordered_participant",), ("ordered_participant",)),),
                    v3_member_facts=((key, ("ordered_participant",), ("source",)),),
                )
            return
        if mutation == "divergent_member_on_v2_path":
            key = (CANDIDATE_DIGEST, SOURCE_INSTANCE_ID)
            with self.assertRaises(ContextApplicationV3ResolutionError):
                ContextApplicationV3AuthorityResolver.require_cross_version_eligibility(
                    v2_member_facts=((key, ("ordered_participant",), ("source",)),),
                    v3_member_facts=(),
                )
            return
        raise AssertionError(f"unhandled mutation {mutation}")


if __name__ == "__main__":
    unittest.main()
