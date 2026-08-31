"""Rules-neutral, read-only source and locator resolution for M2.5.C.

This module verifies bytes and source identity before parsing or interpreting
them. It resolves repository artifacts and the externally configured REV3
package, including exact B2 source records; it does not classify candidates,
derive C semantics, or accept authority records.
"""

from __future__ import annotations

import base64
import csv
import hashlib
import io
import json
import os
import re
import sys
import zipfile
from collections.abc import Mapping
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from types import MappingProxyType
from typing import Literal, NoReturn, TypeAlias, cast

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from mtgml.authority import (
    ACCEPTANCE_EVENT_SCHEMA_V1,
    REVIEWER_ROSTER_SCHEMA_V1,
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKind,
    ReviewAcceptanceEventInputV1,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewerRosterV1,
    ReviewerV1,
    ReviewEventRefV1,
    ReviewMode,
    SourceBindingDigestV1,
)
from mtgml.persistence import (
    CANONICAL_CBOR_ID,
    DIGEST_ENVELOPE_ID,
    SHA256_ID,
    PersistenceValue,
    encode_canonical,
    encode_envelope,
    hash_envelope,
)

REV3_ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"
REV3_ARCHIVE_RELATIVE_PATH = Path("m2_5/Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip")
EXPECTED_REV3_ARCHIVE_SHA256 = "99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90"
REV3_PACKAGE_MANIFEST_MEMBER = "Manafold_M2_5_Package_Manifest_REV3.json"
REV3_PACKAGE_MANIFEST_SCHEMA = "manafold.m2.5.rev3.package-manifest.v1"
REV3_CENSUS_MEMBER = "derived/Pair_Interaction_Census_REV3.csv"
CANDIDATE_UNIVERSE_PATH = "sources/m2_5/closures/C/interaction_candidate_universe.v2.json"
CANDIDATE_UNIVERSE_SCHEMA = "manafold.m2.5.c.interaction-candidate-universe.v2"
CANDIDATE_UNIVERSE_MODEL_ID = "declared-interaction-model.v2"
DECLARED_MODEL_SCHEMA = "manafold.m2.5.c.declared-interaction-model.v2"
CANDIDATE_IDENTITY_DOMAIN = "manafold.m2.5.c.candidate-identity.v1"
CANDIDATE_IDENTITY_SCHEMA = "manafold.m2.5.c.candidate-identity-input.v1"
B2_CATALOG_PATH = "sources/m2_5/closures/B2/requirement_family_catalog.v1.json"
B2_CATALOG_SCHEMA = "manafold.m2.5.b2.requirement-family-catalog.v1"
B2_CLASSIFICATION_PATH = "sources/m2_5/closures/B2/card_semantic_classifications.v1.json"
B2_CLASSIFICATION_SCHEMA = "manafold.m2.5.b2.card-semantic-classifications.v1"
B2_CLOSURE_PATH = "sources/m2_5/closures/B2/classification_closure.v1.json"
B2_CLOSURE_SCHEMA = "manafold.m2.5.b2.classification-closure.v1"
B2_SOURCE_DOMAIN = "manafold.m2.5.b2.source-identity.v1"
B2_SOURCE_INPUT_SCHEMA = "manafold.m2.5.b2.source-identity-input.v1"
B2_REV3_CLASSIFICATION_DOMAIN = "manafold.m2.5.b2.rev3-classification-record-identity.v1"
B2_REV3_CLASSIFICATION_SCHEMA = "manafold.m2.5.b2.rev3-classification-record-identity-input.v1"
B2_CLASSIFICATION_DOMAIN = "manafold.m2.5.b2.classification-record-identity.v1"
B2_CLASSIFICATION_INPUT_SCHEMA = "manafold.m2.5.b2.classification-record-identity-input.v1"
B2_REVIEW_BASIS_SCHEMA = "manafold.m2.5.b2.review-basis.v1"
B2_PROVENANCE_SCHEMA = "manafold.m2.5.b2.provenance.v1"
B2_ORACLE_LOCATOR_SCHEMA = "manafold.m2.5.b2.oracle-field-locator.v1"
B2_AUTHORITY_LOCATOR_SCHEMA = "manafold.m2.5.b2.authority-byte-fragment-locator.v1"
B2_RULE_LOCATOR_SCHEMA = "manafold.m2.5.b2.comprehensive-rule-locator.v1"
B2_SEMANTIC_BOUNDARY_PREFIX = "B2_SEMANTIC_BOUNDARY_V1"
B2_SEMANTIC_BOUNDARY_FIELDS = (
    "family_id",
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
_B2_EVIDENCE_BASES = (
    "ORACLE_TEXT",
    "TYPE_LINE",
    "CARD_FACE",
    "STRUCTURAL_CARD_PROPERTY",
    "FORMAT_POLICY",
    "RULE_DERIVED",
)
_B2_CHANGE_KINDS = ("RETAINED", "ADDED", "REMOVED", "SUPERSEDED")
_B2_REVIEW_STATUSES = ("REVIEWED_CONFIRMED", "REVIEWED_CORRECTED")
_B2_BOUND_ARTIFACT_KEYS = {"path", "raw_sha256"}
B1_FINAL_CITATIONS_PATH = "sources/m2_5/closures/B1/official_authority_citations.v3.json"
B1_FINAL_CITATIONS_SCHEMA = "manafold.m2.5.b1.official-authority-citations.v3"
B1_FINAL_CLOSURE_PATH = "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json"
B1_FINAL_CLOSURE_SCHEMA = "manafold.m2.5.b1.official-authority-citation-closure.v2"
REV3_OFFICIAL_AUTHORITY_REGISTER_MEMBER = "source/official_authority_register_REV3.json"
B1_FINAL_AUTHORITY_IDS = (
    "banned_restricted",
    "commander_1v1",
    "commander_general",
    "commander_legends_release_notes",
    "comprehensive_rules",
    "kaldheim_release_notes",
    "magic_2013_release_notes",
)
_B1_FINAL_CITATION_KINDS = (
    "CR_RULE_IDENTIFIER",
    "POLICY_SECTION_LOCATOR",
    "RELEASE_NOTE_LOCATOR",
)
_CR_RULE_IDENTIFIER_RE = re.compile(r"^CR ([0-9]{3})(\.[0-9]+[a-z]?)?$")
REV3_MODEL_ID = "interaction-model.v1"
REV3_RESOLUTION_MEMBER = "inputs/deck_row_source_resolution_REV3.csv"
REV3_SOURCE_INDEX_MEMBER = "source/raw/source_record_index_REV3.csv"
REV3_SCOPE_MAP = {
    "INTRA_DECK": "intra_deck",
    "CROSS_DECK": "cross_deck",
    "UNARY_OR_HIGHER_ORDER": "unary_or_higher_order",
}
REV3_RELATION_MAP = {
    "UNORDERED_BINARY": "unordered_binary",
    "DIRECTIONAL_BINARY": "directional_binary",
    "DECLARED_CARD_TRIGGER": "declared_card_trigger",
}
REV3_SHAPES = frozenset(
    {
        ("INTRA_DECK", "UNORDERED_BINARY"),
        ("CROSS_DECK", "DIRECTIONAL_BINARY"),
        ("UNARY_OR_HIGHER_ORDER", "DECLARED_CARD_TRIGGER"),
    }
)
_LOWERCASE_UUID_RE = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")

REV3_SOURCE_COLUMNS = (
    "candidate_id",
    "model_id",
    "scope",
    "pair_id",
    "left_family_id",
    "right_family_id",
    "relation",
    "disposition",
    "disposition_reason",
    "supporting_requirement_ids",
)
SOURCE_CONTEXT_KEYS = (
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
CANDIDATE_SCOPES = frozenset({"cross_deck", "intra_deck", "unary_or_higher_order"})
CANDIDATE_RELATIONS = frozenset(
    {
        "declared_card_trigger",
        "directional_binary",
        "reviewed_higher_order",
        "unordered_binary",
    }
)
CANDIDATE_SOURCE_ORIGINS = frozenset({"rev3", "targeted_higher_order_review"})
RECONCILIATION_STATUSES = frozenset(
    {
        "unchanged",
        "stale_rev3_candidate",
        "removed_not_interaction",
        "merged_semantic_duplicate",
        "new_targeted_higher_order_candidate",
    }
)
RECONCILIATION_COUNT_KEYS = frozenset(
    {
        "unchanged",
        "stale_rev3_candidate",
        "removed_not_interaction",
        "merged_semantic_duplicate",
        "new_targeted_higher_order_candidate",
        "new_b2_derived",
    }
)
CANDIDATE_IDENTITY_KEYS = frozenset(
    {
        "envelope_id",
        "algorithm_id",
        "semantic_domain",
        "payload_codec_id",
        "input_schema_id",
        "digest_hex",
    }
)
CANDIDATE_RECORD_KEYS = frozenset(
    {
        "candidate_id",
        "candidate_identity",
        "source_origin",
        "scope",
        "relation",
        "participant_refs",
        "supporting_requirement_ids",
        "source_binding",
        "reconciliation_status",
        "reconciliation_reason",
    }
)
SOURCE_INSTANCE_RECORD_KEYS = frozenset(
    {
        "source_instance_id",
        "candidate_id",
        "source_binding",
        "participant_bindings",
        "source_context",
    }
)
REV3_SOURCE_BINDING_KEYS = frozenset(
    {
        "kind",
        "archive_member",
        "archive_member_sha256",
        "row_ordinal",
        "source_columns",
        "source_values",
    }
)
REV3_INPUT_BINDING_KEYS = frozenset(
    {"archive_member", "archive_member_sha256", "source_package_sha256"}
)
RAW_ARTIFACT_BINDING_KEYS = frozenset({"path", "raw_sha256"})
INPUT_BINDING_KEYS = frozenset(
    {
        "declared_model",
        "review_additions",
        "rev3_candidate_source",
        "b2_artifacts",
        "b1_final_artifacts",
    }
)
EXPECTED_STATIC_INPUT_PATHS = {
    "declared_model": "sources/m2_5/closures/C/declared_interaction_model.v2.json",
    "review_additions": "sources/m2_5/closures/C/interaction_review_additions.v2.json",
    "b2_artifacts": (
        "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
        "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
        "sources/m2_5/closures/B2/classification_closure.v1.json",
    ),
    "b1_final_artifacts": (
        "sources/m2_5/closures/B1/official_authority_citations.v3.json",
        "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json",
    ),
}

SourceKind: TypeAlias = Literal["repository", "rev3_archive"]
Locator: TypeAlias = tuple[str, str | int | None]
DigestInput: TypeAlias = str | bytes
ModelVocabularies: TypeAlias = tuple[
    frozenset[str],
    frozenset[str],
    Mapping[str, frozenset[str]],
]

_HEX64_RE = re.compile(r"^[0-9a-f]{64}$")
_EVENT_ID_RE = re.compile(r"^ae\.v1/[0-9a-f]{64}$")
_EVENT_LEAF_PATH_RE = re.compile(
    r"^sources/m2_5/authorities/review_acceptance_events/v1/[0-9a-f]{64}\.json$"
)
_VERIFIED_ARTIFACT_TOKEN = object()
_VERIFIED_CANDIDATE_TOKEN = object()
_VERIFIED_B2_FAMILY_TOKEN = object()
_VERIFIED_B2_CLASSIFICATION_TOKEN = object()
_VERIFIED_B2_ASSIGNMENT_TOKEN = object()
_VERIFIED_B2_BOUNDARY_TOKEN = object()
_VERIFIED_B1_AUTHORITY_TOKEN = object()
_VERIFIED_B1_CITATION_TOKEN = object()
_VERIFIED_B1_LOCATOR_TOKEN = object()


class ResolutionStatus(str, Enum):
    FAIL = "FAIL"
    BLOCKED = "BLOCKED"


class ResolutionError(Exception):
    """A source resolution failure with an explicit fail-closed status."""

    def __init__(self, status: ResolutionStatus, code: str, message: str) -> None:
        self.status = status
        self.code = code
        self.message = message
        super().__init__(f"[{status.value}:{code}] {message}")


@dataclass(frozen=True)
class ResolvedArtifact:
    source_kind: SourceKind
    path: str
    raw_bytes: bytes
    raw_sha256: str
    schema_or_null: str | None
    json_value: object | None
    _verification_token: object = field(default=None, repr=False, compare=False)


@dataclass(frozen=True)
class ResolvedLocator:
    artifact: ResolvedArtifact
    locator: Locator
    value: object


@dataclass(frozen=True)
class ResolvedCandidate:
    """An exact, source-bound candidate record with no semantic conclusion."""

    candidate_id: str
    candidate_identity: Mapping[str, object]
    candidate_universe: ResolvedArtifact
    candidate_universe_binding: SourceBindingDigestV1
    candidate_record: Mapping[str, object]
    source_binding: Mapping[str, object]
    _verification_token: object = field(default=None, repr=False, compare=False)


@dataclass(frozen=True)
class ResolvedSourceInstance:
    """An exact candidate/source-instance join after REV3 row verification."""

    candidate: ResolvedCandidate
    source_instance_id: str
    source_instance_record: Mapping[str, object]
    source_binding: Mapping[str, object]
    source_artifact: ResolvedArtifact


@dataclass(frozen=True)
class B2ArtifactBindingsV1:
    """The exact three-file B2 snapshot binding used by source resolution."""

    catalog: SourceBindingDigestV1
    classifications: SourceBindingDigestV1
    closure: SourceBindingDigestV1


@dataclass(frozen=True)
class ResolvedB2RequirementFamily:
    """One immutable family record from the digest-bound B2 catalog."""

    family_id: str
    artifact: ResolvedArtifact
    source_binding: SourceBindingDigestV1
    record: Mapping[str, object]
    boundary_fields: Mapping[str, str]
    _verification_token: object = field(default=None, repr=False, compare=False)


@dataclass(frozen=True)
class ResolvedB2Classification:
    """One immutable card classification from the digest-bound B2 artifact."""

    oracle_semantic_identity: str
    artifact: ResolvedArtifact
    source_binding: SourceBindingDigestV1
    record: Mapping[str, object]
    classification_identity: Mapping[str, object]
    _verification_token: object = field(default=None, repr=False, compare=False)


@dataclass(frozen=True)
class ResolvedB2Assignment:
    """One exact classification-to-family assignment edge."""

    classification: ResolvedB2Classification
    family: ResolvedB2RequirementFamily
    assignment: Mapping[str, object]
    _verification_token: object = field(default=None, repr=False, compare=False)


@dataclass(frozen=True)
class B2BoundaryReferenceV1:
    """The source-bound two-field B2 boundary reference."""

    family_id: str
    precise_semantic_definition: str


@dataclass(frozen=True)
class ResolvedB2Boundary:
    """One exact B2 family boundary, optionally joined to a classification edge."""

    family: ResolvedB2RequirementFamily
    boundary_ref: B2BoundaryReferenceV1
    assignment: ResolvedB2Assignment | None
    _verification_token: object = field(default=None, repr=False, compare=False)


@dataclass(frozen=True)
class B1FinalArtifactBindingsV1:
    """The exact repository bindings for the B1.Final snapshot."""

    citations: SourceBindingDigestV1
    closure: SourceBindingDigestV1


@dataclass(frozen=True)
class ResolvedB1FinalOfficialLocator:
    """The exact bytes selected by one B1.Final artifact-local locator."""

    artifact: ResolvedArtifact
    citation_kind: str
    locator: Mapping[str, object]
    resolved_bytes: bytes
    _verification_token: object = field(default=None, repr=False, compare=False)


@dataclass(frozen=True)
class ResolvedB1FinalAuthority:
    """One exact B1.Final authority record and its official source artifact."""

    authority_id: str
    artifact: ResolvedArtifact
    source_binding: SourceBindingDigestV1
    record: Mapping[str, object]
    official_artifact: ResolvedArtifact | None
    artifact_identity: Mapping[str, object]
    _verification_token: object = field(default=None, repr=False, compare=False)


@dataclass(frozen=True)
class ResolvedB1FinalCitation:
    """One exact B1.Final citation and its verified official fragment."""

    authority: ResolvedB1FinalAuthority
    citation_id: str
    citation: Mapping[str, object]
    official_locator: ResolvedB1FinalOfficialLocator
    _verification_token: object = field(default=None, repr=False, compare=False)


@dataclass(frozen=True)
class _B1FinalSnapshot:
    citations_artifact: ResolvedArtifact
    closure_artifact: ResolvedArtifact
    authorities_by_id: Mapping[str, Mapping[str, object]]
    citations_by_id: Mapping[str, tuple[str, Mapping[str, object]]]
    official_artifacts_by_id: Mapping[str, ResolvedArtifact | None]


@dataclass(frozen=True)
class _B2Snapshot:
    catalog_artifact: ResolvedArtifact
    classification_artifact: ResolvedArtifact
    closure_artifact: ResolvedArtifact
    families_by_id: Mapping[str, Mapping[str, object]]
    family_boundaries_by_id: Mapping[str, Mapping[str, str]]
    classifications_by_osi: Mapping[str, Mapping[str, object]]


@dataclass(frozen=True)
class _CandidateUniverseIndex:
    artifact: ResolvedArtifact
    rev3_input_binding: Mapping[str, object]
    candidates_by_id: Mapping[str, Mapping[str, object]]
    candidate_identities_by_digest: Mapping[str, Mapping[str, object]]
    candidate_bindings_by_id: Mapping[str, Mapping[str, object]]
    instances_by_id: Mapping[str, Mapping[str, object]]
    instances_by_candidate_id: Mapping[str, tuple[Mapping[str, object], ...]]


def _fail(code: str, message: str) -> NoReturn:
    raise ResolutionError(ResolutionStatus.FAIL, code, message)


def _blocked(code: str, message: str) -> NoReturn:
    raise ResolutionError(ResolutionStatus.BLOCKED, code, message)


def _digest_hex(value: DigestInput, label: str) -> str:
    if isinstance(value, bytes):
        if len(value) != 32:
            _fail("DIGEST_INVALID", f"{label} must contain exactly 32 bytes")
        return value.hex()
    if not isinstance(value, str) or _HEX64_RE.fullmatch(value) is None:
        _fail("DIGEST_INVALID", f"{label} must be lowercase SHA-256 hex")
    return value


def _relative_path(value: object, label: str) -> str:
    if not isinstance(value, str) or not value or "\x00" in value:
        _fail("PATH_INVALID", f"{label} must be a non-empty path")
    if (
        value.startswith(("/", "\\"))
        or "\\" in value
        or "://" in value
        or re.match(r"^[A-Za-z]:", value) is not None
    ):
        _fail("PATH_INVALID", f"{label} must be slash-separated and relative")
    parts = value.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        _fail("PATH_INVALID", f"{label} contains an invalid path segment")
    return value


def _json_object(value: object, label: str) -> dict[str, object]:
    if not isinstance(value, dict):
        _fail("JSON_OBJECT_REQUIRED", f"{label} must contain a JSON object")
    return cast(dict[str, object], value)


def _json_text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        _fail("JSON_FIELD_INVALID", f"{label} must be non-empty text")
    return value


def _json_digest(value: object, label: str) -> bytes:
    text = _json_text(value, label)
    if _HEX64_RE.fullmatch(text) is None:
        _fail("JSON_FIELD_INVALID", f"{label} must be SHA-256 hex")
    return bytes.fromhex(text)


def _verify_json_schema(raw: bytes, schema_or_null: str | None, path: str) -> object | None:
    if schema_or_null is None:
        return None
    try:
        value = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        _fail("JSON_INVALID", f"{path} is not valid UTF-8 JSON: {exc}")
    record = _json_object(value, path)
    if record.get("schema") != schema_or_null:
        _fail("SCHEMA_MISMATCH", f"{path} does not declare {schema_or_null!r}")
    return cast(object, value)


def _resolved_artifact(
    source_kind: SourceKind,
    path: str,
    raw: bytes,
    expected_raw_sha256: DigestInput,
    schema_or_null: str | None,
) -> ResolvedArtifact:
    expected = _digest_hex(expected_raw_sha256, f"{path} expected digest")
    actual = hashlib.sha256(raw).hexdigest()
    if actual != expected:
        _fail("SOURCE_DIGEST_MISMATCH", f"{path} has {actual}, expected {expected}")
    json_value = _verify_json_schema(raw, schema_or_null, path)
    return ResolvedArtifact(
        source_kind,
        path,
        raw,
        actual,
        schema_or_null,
        json_value,
        _VERIFIED_ARTIFACT_TOKEN,
    )


def _json_pointer_token(raw_token: str) -> str:
    result: list[str] = []
    index = 0
    while index < len(raw_token):
        character = raw_token[index]
        if character != "~":
            result.append(character)
            index += 1
            continue
        if index + 1 >= len(raw_token) or raw_token[index + 1] not in "01":
            _fail("LOCATOR_INVALID", "JSON Pointer contains an invalid escape")
        result.append("~" if raw_token[index + 1] == "0" else "/")
        index += 2
    return "".join(result)


def _json_pointer(value: object, pointer: object) -> object:
    if not isinstance(pointer, str) or (pointer and not pointer.startswith("/")):
        _fail("LOCATOR_INVALID", "JSON Pointer must be empty or begin with '/'")
    if pointer == "":
        return value
    current = value
    for raw_token in pointer[1:].split("/"):
        token = _json_pointer_token(raw_token)
        if isinstance(current, dict):
            if token not in current:
                _fail("LOCATOR_UNRESOLVED", f"JSON Pointer token {token!r} is absent")
            current = current[token]
        elif isinstance(current, list):
            if token == "0":
                index = 0
            elif re.fullmatch(r"[1-9][0-9]*", token) is not None:
                index = int(token)
            else:
                _fail("LOCATOR_UNRESOLVED", f"JSON Pointer array index {token!r} is invalid")
            if index >= len(current):
                _fail("LOCATOR_UNRESOLVED", f"JSON Pointer index {index} is out of range")
            current = current[index]
        else:
            _fail("LOCATOR_UNRESOLVED", "JSON Pointer traverses a scalar value")
    return current


def _locator(value: object) -> Locator:
    if not isinstance(value, tuple) or len(value) != 2:
        _fail("LOCATOR_INVALID", "locator must be a two-position tuple")
    kind, payload = value
    if kind == "whole_artifact":
        if payload is not None:
            _fail("LOCATOR_INVALID", "whole_artifact payload must be null")
    elif kind == "json_pointer":
        if not isinstance(payload, str):
            _fail("LOCATOR_INVALID", "json_pointer payload must be text")
    elif kind == "archive_member":
        _relative_path(payload, "archive member locator")
    elif kind == "event_id":
        if not isinstance(payload, str) or _EVENT_ID_RE.fullmatch(payload) is None:
            _fail("LOCATOR_INVALID", "event_id locator is not an acceptance-event ID")
    else:
        _fail("LOCATOR_INVALID", f"unknown locator variant {kind!r}")
    return cast(Locator, value)


def _exact_keys(value: Mapping[str, object], expected: set[str], label: str) -> None:
    if set(value) != expected:
        _fail("SCHEMA_MISMATCH", f"{label} fields are not exactly {sorted(expected)!r}")


def _freeze_json(value: object) -> object:
    if isinstance(value, dict):
        return MappingProxyType({key: _freeze_json(item) for key, item in value.items()})
    if isinstance(value, list):
        return tuple(_freeze_json(item) for item in value)
    return value


def _frozen_mapping(value: Mapping[str, object]) -> Mapping[str, object]:
    return cast(Mapping[str, object], _freeze_json(dict(value)))


def _b2_verified_json_document(
    artifact: ResolvedArtifact, label: str
) -> tuple[ResolvedArtifact, dict[str, object]]:
    if artifact._verification_token is not _VERIFIED_ARTIFACT_TOKEN:
        _fail("ARTIFACT_UNVERIFIED", f"{label} requires a resolver-verified artifact")
    verified = _resolved_artifact(
        artifact.source_kind,
        artifact.path,
        artifact.raw_bytes,
        artifact.raw_sha256,
        artifact.schema_or_null,
    )
    value = _verify_json_schema(verified.raw_bytes, verified.schema_or_null, verified.path)
    document = _json_object(value, label)
    immutable = ResolvedArtifact(
        verified.source_kind,
        verified.path,
        verified.raw_bytes,
        verified.raw_sha256,
        verified.schema_or_null,
        _freeze_json(value),
        _VERIFIED_ARTIFACT_TOKEN,
    )
    return immutable, document


def _b2_digest_reference(
    value: object, label: str, semantic_domain: str, input_schema_id: str
) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(
        record,
        {
            "envelope_id",
            "algorithm_id",
            "semantic_domain",
            "payload_codec_id",
            "input_schema_id",
            "digest_hex",
        },
        label,
    )
    expected = {
        "envelope_id": DIGEST_ENVELOPE_ID,
        "algorithm_id": SHA256_ID,
        "semantic_domain": semantic_domain,
        "payload_codec_id": CANONICAL_CBOR_ID,
        "input_schema_id": input_schema_id,
    }
    for key, expected_value in expected.items():
        if record.get(key) != expected_value:
            _fail("B2_IDENTITY_INVALID", f"{label}.{key} is not the accepted V1 value")
    _json_digest(record.get("digest_hex"), f"{label}.digest_hex")
    return dict(record)


def _b2_digest_for_payload(
    semantic_domain: str, input_schema_id: str, payload: list[object]
) -> bytes:
    return hash_envelope(
        encode_envelope(
            semantic_domain,
            input_schema_id,
            encode_canonical(cast(PersistenceValue, payload)),
        )
    )


def _b2_lower_variant(value: object, label: str) -> list[object]:
    return [_json_text(value, label).lower(), None]


def _b2_json_pointer_grammar(value: object, label: str) -> str:
    pointer = _json_text(value, label)
    if not pointer.startswith("/"):
        _fail("B2_LOCATOR_INVALID", f"{label} must begin with '/'")
    for token in pointer[1:].split("/"):
        _json_pointer_token(token)
    return pointer


def _b2_locator_to_cbor(value: object, label: str) -> list[object]:
    record = _json_object(value, label)
    version = _json_text(record.get("locator_version"), f"{label}.locator_version")
    if version == B2_ORACLE_LOCATOR_SCHEMA:
        _exact_keys(
            record,
            {
                "locator_version",
                "archive_artifact",
                "oracle_source_record_id",
                "raw_line_sha256",
                "json_pointer",
                "field_value_sha256",
            },
            label,
        )
        archive_artifact = _relative_path(
            record.get("archive_artifact"), f"{label}.archive_artifact"
        )
        source_id = _json_text(
            record.get("oracle_source_record_id"), f"{label}.oracle_source_record_id"
        )
        if _LOWERCASE_UUID_RE.fullmatch(source_id) is None:
            _fail("B2_LOCATOR_INVALID", f"{label}.oracle_source_record_id is not a lowercase UUID")
        pointer = _b2_json_pointer_grammar(record.get("json_pointer"), f"{label}.json_pointer")
        return [
            "oracle_field",
            [
                version,
                archive_artifact,
                source_id,
                _json_digest(record.get("raw_line_sha256"), f"{label}.raw_line_sha256"),
                pointer,
                _json_digest(record.get("field_value_sha256"), f"{label}.field_value_sha256"),
            ],
        ]
    if version == B2_AUTHORITY_LOCATOR_SCHEMA:
        _exact_keys(
            record,
            {
                "locator_version",
                "archive_artifact",
                "artifact_sha256",
                "byte_offset",
                "byte_length",
                "fragment_sha256",
            },
            label,
        )
        offset = record.get("byte_offset")
        length = record.get("byte_length")
        if (
            isinstance(offset, bool)
            or not isinstance(offset, int)
            or offset < 0
            or isinstance(length, bool)
            or not isinstance(length, int)
            or length <= 0
        ):
            _fail("B2_LOCATOR_INVALID", f"{label} byte range is invalid")
        return [
            "authority_byte_fragment",
            [
                version,
                _relative_path(record.get("archive_artifact"), f"{label}.archive_artifact"),
                _json_digest(record.get("artifact_sha256"), f"{label}.artifact_sha256"),
                offset,
                length,
                _json_digest(record.get("fragment_sha256"), f"{label}.fragment_sha256"),
            ],
        ]
    if version == B2_RULE_LOCATOR_SCHEMA:
        _exact_keys(
            record,
            {
                "locator_version",
                "archive_artifact",
                "artifact_sha256",
                "rule_identifier",
                "line_number",
                "line_sha256",
            },
            label,
        )
        line_number = record.get("line_number")
        if isinstance(line_number, bool) or not isinstance(line_number, int) or line_number <= 0:
            _fail("B2_LOCATOR_INVALID", f"{label}.line_number is invalid")
        return [
            "comprehensive_rule",
            [
                version,
                _relative_path(record.get("archive_artifact"), f"{label}.archive_artifact"),
                _json_digest(record.get("artifact_sha256"), f"{label}.artifact_sha256"),
                _json_text(record.get("rule_identifier"), f"{label}.rule_identifier"),
                line_number,
                _json_digest(record.get("line_sha256"), f"{label}.line_sha256"),
            ],
        ]
    _fail("B2_LOCATOR_INVALID", f"{label} has an unsupported locator version")


def _b2_locator_list(value: object, label: str, *, nonempty: bool) -> list[dict[str, object]]:
    raw = value
    if not isinstance(raw, list):
        _fail("B2_LOCATOR_INVALID", f"{label} must be an array")
    if nonempty and not raw:
        _fail("B2_LOCATOR_INVALID", f"{label} must be non-empty")
    locators = [
        _json_object(item, f"{label}[{index}]")
        for index, item in enumerate(cast(list[object], raw))
    ]
    keys = [_b2_locator_to_cbor(item, f"{label}[{index}]") for index, item in enumerate(locators)]
    encoded = [encode_canonical(cast(PersistenceValue, key)) for key in keys]
    if len(set(encoded)) != len(encoded) or encoded != sorted(encoded):
        _fail("B2_LOCATOR_ORDER_INVALID", f"{label} must be canonical and duplicate-free")
    return locators


def _b2_evidence_checksum(value: object, label: str) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(record, {"algorithm_id", "checksum_kind", "digest_hex", "input_schema_id"}, label)
    if (
        record.get("algorithm_id") != SHA256_ID
        or record.get("checksum_kind") != "EVIDENCE_CHECKSUM"
    ):
        _fail("B2_CHECKSUM_INVALID", f"{label} is not an evidence checksum")
    _json_text(record.get("input_schema_id"), f"{label}.input_schema_id")
    _json_digest(record.get("digest_hex"), f"{label}.digest_hex")
    return dict(record)


def _b2_validate_source_identity(value: object, label: str) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(
        record,
        {
            "archive_artifact",
            "normalized_record_sha256",
            "oracle_layout",
            "oracle_semantic_identity",
            "oracle_source_record_id",
            "source_record_raw_sha256",
        },
        label,
    )
    if record.get("archive_artifact") != "source/raw/oracle_cards_selected_REV3.jsonl":
        _fail(
            "B2_SOURCE_BINDING_INVALID",
            f"{label}.archive_artifact is not the admitted Oracle source",
        )
    for key in ("oracle_semantic_identity", "oracle_source_record_id"):
        identity = _json_text(record.get(key), f"{label}.{key}")
        if _LOWERCASE_UUID_RE.fullmatch(identity) is None:
            _fail("B2_SOURCE_BINDING_INVALID", f"{label}.{key} is not a lowercase UUID")
    _json_text(record.get("oracle_layout"), f"{label}.oracle_layout")
    _json_digest(record.get("source_record_raw_sha256"), f"{label}.source_record_raw_sha256")
    _json_digest(record.get("normalized_record_sha256"), f"{label}.normalized_record_sha256")
    return dict(record)


def _b2_source_identity_input(source: Mapping[str, object]) -> list[object]:
    return [
        B2_SOURCE_INPUT_SCHEMA,
        source["archive_artifact"],
        source["oracle_semantic_identity"],
        source["oracle_source_record_id"],
        source["oracle_layout"],
        bytes.fromhex(cast(str, source["source_record_raw_sha256"])),
        bytes.fromhex(cast(str, source["normalized_record_sha256"])),
    ]


def _b2_assignment_input(assignment: Mapping[str, object]) -> list[object]:
    return [
        assignment["requirement_family_id"],
        _b2_lower_variant(assignment["evidence_basis"], "assignment.evidence_basis"),
        [
            _b2_locator_to_cbor(locator, "assignment.evidence_locators")
            for locator in cast(list[Mapping[str, object]], assignment["evidence_locators"])
        ],
        assignment["review_rationale"],
    ]


def _b2_change_input(change: Mapping[str, object]) -> list[object]:
    return [
        change["family_id"],
        _b2_lower_variant(change["change_kind"], "classification change kind"),
        list(cast(tuple[str, ...], change["replacement_family_ids"])),
        change["rationale"],
        [
            _b2_locator_to_cbor(locator, "classification change evidence_locators")
            for locator in cast(list[Mapping[str, object]], change["evidence_locators"])
        ],
    ]


def _b2_validate_assignment(value: object, label: str) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(
        record,
        {"requirement_family_id", "evidence_basis", "evidence_locators", "review_rationale"},
        label,
    )
    _json_text(record.get("requirement_family_id"), f"{label}.requirement_family_id")
    if (
        _json_text(record.get("evidence_basis"), f"{label}.evidence_basis")
        not in _B2_EVIDENCE_BASES
    ):
        _fail("B2_ASSIGNMENT_INVALID", f"{label}.evidence_basis is not a closed V1 value")
    locators = _b2_locator_list(
        record.get("evidence_locators"), f"{label}.evidence_locators", nonempty=True
    )
    _json_text(record.get("review_rationale"), f"{label}.review_rationale")
    result = dict(record)
    result["evidence_locators"] = locators
    return result


def _b2_validate_change(value: object, label: str) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(
        record,
        {"family_id", "change_kind", "replacement_family_ids", "rationale", "evidence_locators"},
        label,
    )
    _json_text(record.get("family_id"), f"{label}.family_id")
    if _json_text(record.get("change_kind"), f"{label}.change_kind") not in _B2_CHANGE_KINDS:
        _fail("B2_CLASSIFICATION_INVALID", f"{label}.change_kind is not a closed V1 value")
    replacements = record.get("replacement_family_ids")
    if not isinstance(replacements, list) or any(
        not isinstance(item, str) or not item for item in replacements
    ):
        _fail("B2_CLASSIFICATION_INVALID", f"{label}.replacement_family_ids is invalid")
    replacement_values = [cast(str, item) for item in replacements]
    replacement_keys = [encode_canonical(item) for item in replacement_values]
    if len(set(replacement_keys)) != len(replacement_keys) or replacement_keys != sorted(
        replacement_keys
    ):
        _fail("B2_CLASSIFICATION_INVALID", f"{label}.replacement_family_ids is not canonical")
    _json_text(record.get("rationale"), f"{label}.rationale")
    locators = _b2_locator_list(
        record.get("evidence_locators"), f"{label}.evidence_locators", nonempty=True
    )
    result = dict(record)
    result["replacement_family_ids"] = replacement_values
    result["evidence_locators"] = locators
    return result


def _b2_validate_classification_record(value: object, label: str) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(
        record,
        {
            "classification_delta",
            "classification_identity",
            "oracle_semantic_identity",
            "previous_rev3_classification_identity",
            "provenance",
            "requirement_assignments",
            "review_basis",
            "review_status",
            "source_evidence_digest",
            "source_identity",
        },
        label,
    )
    osi = _json_text(record.get("oracle_semantic_identity"), f"{label}.oracle_semantic_identity")
    if _LOWERCASE_UUID_RE.fullmatch(osi) is None:
        _fail(
            "B2_CLASSIFICATION_INVALID", f"{label}.oracle_semantic_identity is not a lowercase UUID"
        )
    source = _b2_validate_source_identity(record.get("source_identity"), f"{label}.source_identity")
    if source["oracle_semantic_identity"] != osi:
        _fail("B2_SOURCE_BINDING_INVALID", f"{label}.source_identity binds another OSI")
    source_evidence = _b2_digest_reference(
        record.get("source_evidence_digest"),
        f"{label}.source_evidence_digest",
        B2_SOURCE_DOMAIN,
        B2_SOURCE_INPUT_SCHEMA,
    )
    expected_source_evidence = _b2_digest_for_payload(
        B2_SOURCE_DOMAIN,
        B2_SOURCE_INPUT_SCHEMA,
        _b2_source_identity_input(source),
    )
    if source_evidence["digest_hex"] != expected_source_evidence.hex():
        _fail(
            "B2_IDENTITY_MISMATCH", f"{label}.source_evidence_digest does not match source identity"
        )
    previous = _b2_digest_reference(
        record.get("previous_rev3_classification_identity"),
        f"{label}.previous_rev3_classification_identity",
        B2_REV3_CLASSIFICATION_DOMAIN,
        B2_REV3_CLASSIFICATION_SCHEMA,
    )
    review_status = _json_text(record.get("review_status"), f"{label}.review_status")
    if review_status not in _B2_REVIEW_STATUSES:
        _fail("B2_CLASSIFICATION_INVALID", f"{label}.review_status is not terminal")
    raw_assignments = record.get("requirement_assignments")
    if not isinstance(raw_assignments, list):
        _fail("B2_ASSIGNMENT_INVALID", f"{label}.requirement_assignments must be an array")
    assignments = [
        _b2_validate_assignment(item, f"{label}.requirement_assignments[{index}]")
        for index, item in enumerate(cast(list[object], raw_assignments))
    ]
    assignment_ids = [cast(str, item["requirement_family_id"]) for item in assignments]
    assignment_keys = [encode_canonical(item) for item in assignment_ids]
    if len(set(assignment_keys)) != len(assignment_keys) or assignment_keys != sorted(
        assignment_keys
    ):
        _fail("B2_ASSIGNMENT_ORDER_INVALID", f"{label}.requirement_assignments is not canonical")

    delta = _json_object(record.get("classification_delta"), f"{label}.classification_delta")
    _exact_keys(
        delta,
        {
            "added_family_ids",
            "changes",
            "removed_family_ids",
            "retained_family_ids",
            "superseded_family_ids",
        },
        f"{label}.classification_delta",
    )
    delta_result: dict[str, object] = {}
    for key in (
        "added_family_ids",
        "removed_family_ids",
        "retained_family_ids",
        "superseded_family_ids",
    ):
        values = delta.get(key)
        if not isinstance(values, list) or any(
            not isinstance(item, str) or not item for item in values
        ):
            _fail("B2_CLASSIFICATION_INVALID", f"{label}.classification_delta.{key} is invalid")
        texts = [cast(str, item) for item in values]
        keys = [encode_canonical(item) for item in texts]
        if len(set(keys)) != len(keys) or keys != sorted(keys):
            _fail(
                "B2_CLASSIFICATION_INVALID", f"{label}.classification_delta.{key} is not canonical"
            )
        delta_result[key] = texts
    raw_changes = delta.get("changes")
    if not isinstance(raw_changes, list):
        _fail("B2_CLASSIFICATION_INVALID", f"{label}.classification_delta.changes must be an array")
    changes = [
        _b2_validate_change(item, f"{label}.classification_delta.changes[{index}]")
        for index, item in enumerate(cast(list[object], raw_changes))
    ]
    change_keys = [
        encode_canonical(cast(PersistenceValue, [item["family_id"], item["change_kind"]]))
        for item in changes
    ]
    if len(set(change_keys)) != len(change_keys) or change_keys != sorted(change_keys):
        _fail("B2_CLASSIFICATION_INVALID", f"{label}.classification_delta.changes is not canonical")
    delta_result["changes"] = changes

    review_basis = _json_object(record.get("review_basis"), f"{label}.review_basis")
    _exact_keys(review_basis, {"evidence_locators", "review_method"}, f"{label}.review_basis")
    review_method = _json_text(
        review_basis.get("review_method"), f"{label}.review_basis.review_method"
    )
    if review_method != "SOURCE_GROUNDED_CARD_REVIEW":
        _fail("B2_CLASSIFICATION_INVALID", f"{label}.review_basis is not source-grounded")
    review_locators = _b2_locator_list(
        review_basis.get("evidence_locators"),
        f"{label}.review_basis.evidence_locators",
        nonempty=True,
    )
    if not any(
        _b2_locator_to_cbor(item, f"{label}.review_basis.evidence_locators")[0] == "oracle_field"
        for item in review_locators
    ):
        _fail("B2_CLASSIFICATION_INVALID", f"{label}.review_basis has no card-side locator")
    provenance = _json_object(record.get("provenance"), f"{label}.provenance")
    _exact_keys(provenance, {"provenance_method", "source_package_sha256"}, f"{label}.provenance")
    provenance_method = _json_text(
        provenance.get("provenance_method"), f"{label}.provenance.provenance_method"
    )
    if provenance_method != "SOURCE_GROUNDED_REVIEW_V1":
        _fail(
            "B2_CLASSIFICATION_INVALID",
            f"{label}.provenance.provenance_method is not source-grounded",
        )
    if (
        _json_digest(
            provenance.get("source_package_sha256"), f"{label}.provenance.source_package_sha256"
        ).hex()
        != EXPECTED_REV3_ARCHIVE_SHA256
    ):
        _fail(
            "B2_SOURCE_BINDING_INVALID",
            f"{label}.provenance source package differs from the pinned package",
        )

    expected_classification = _b2_digest_for_payload(
        B2_CLASSIFICATION_DOMAIN,
        B2_CLASSIFICATION_INPUT_SCHEMA,
        [
            B2_CLASSIFICATION_INPUT_SCHEMA,
            osi,
            bytes.fromhex(cast(str, source_evidence["digest_hex"])),
            _b2_lower_variant(review_status, f"{label}.review_status"),
            bytes.fromhex(cast(str, previous["digest_hex"])),
            [_b2_assignment_input(item) for item in assignments],
            [_b2_change_input(item) for item in changes],
            [
                B2_REVIEW_BASIS_SCHEMA,
                _b2_lower_variant(review_method, f"{label}.review_basis.review_method"),
                [
                    _b2_locator_to_cbor(item, f"{label}.review_basis.evidence_locators")
                    for item in review_locators
                ],
            ],
            [
                B2_PROVENANCE_SCHEMA,
                bytes.fromhex(EXPECTED_REV3_ARCHIVE_SHA256),
                _b2_lower_variant(provenance_method, f"{label}.provenance.provenance_method"),
            ],
        ],
    )
    classification_identity = _b2_digest_reference(
        record.get("classification_identity"),
        f"{label}.classification_identity",
        B2_CLASSIFICATION_DOMAIN,
        B2_CLASSIFICATION_INPUT_SCHEMA,
    )
    if classification_identity["digest_hex"] != expected_classification.hex():
        _fail("B2_IDENTITY_MISMATCH", f"{label}.classification_identity does not match its record")
    result = dict(record)
    result["source_identity"] = source
    result["requirement_assignments"] = assignments
    result["classification_delta"] = delta_result
    result["review_basis"] = {"evidence_locators": review_locators, "review_method": review_method}
    result["provenance"] = {
        "provenance_method": provenance_method,
        "source_package_sha256": EXPECTED_REV3_ARCHIVE_SHA256,
    }
    result["classification_identity"] = classification_identity
    result["source_evidence_digest"] = source_evidence
    result["previous_rev3_classification_identity"] = previous
    return result


def _b2_semantic_boundary(value: object, family_id: str, label: str) -> dict[str, str]:
    definition = _json_text(value, label)
    parts = definition.split("|")
    if (
        len(parts) != len(B2_SEMANTIC_BOUNDARY_FIELDS) + 1
        or parts[0] != B2_SEMANTIC_BOUNDARY_PREFIX
    ):
        _fail("B2_BOUNDARY_INVALID", f"{label} is not B2_SEMANTIC_BOUNDARY_V1")
    fields: dict[str, str] = {}
    for name, part in zip(B2_SEMANTIC_BOUNDARY_FIELDS, parts[1:], strict=True):
        key, separator, text = part.partition("=")
        if separator != "=" or key != name or not text or name in fields:
            _fail("B2_BOUNDARY_INVALID", f"{label} has an invalid field order or value")
        fields[name] = text
    if fields["family_id"] != family_id:
        _fail("B2_BOUNDARY_BINDING_MISMATCH", f"{label} binds family {fields['family_id']!r}")
    return fields


def _b2_validate_family_record(
    value: object, label: str
) -> tuple[dict[str, object], dict[str, str]]:
    """Validate only the typed family projection needed by source binding."""

    record = _json_object(value, label)
    common = {
        "canonical_name",
        "evidence_basis_allowed",
        "family_id",
        "family_origin",
        "lifecycle_relation",
        "precise_semantic_definition",
        "review_provenance",
        "status",
        "superseded_by",
        "supersession_reason",
        "terminal_assignable",
    }
    origin = _json_text(record.get("family_origin"), f"{label}.family_origin")
    if origin == "REV3_LEGACY":
        expected_keys = common | {"historical_definition", "historical_rev3"}
    elif origin == "B2_NEW":
        expected_keys = common
    else:
        _fail("B2_FAMILY_INVALID", f"{label}.family_origin is not closed")
    _exact_keys(record, expected_keys, label)

    family_id = _json_text(record.get("family_id"), f"{label}.family_id")
    _json_text(record.get("canonical_name"), f"{label}.canonical_name")
    boundary = _b2_semantic_boundary(
        record.get("precise_semantic_definition"),
        family_id,
        f"{label}.precise_semantic_definition",
    )
    allowed_basis = record.get("evidence_basis_allowed")
    if not isinstance(allowed_basis, list) or any(
        not isinstance(item, str) or not item for item in allowed_basis
    ):
        _fail("B2_FAMILY_INVALID", f"{label}.evidence_basis_allowed is not a text array")
    basis_keys = [encode_canonical(item) for item in allowed_basis]
    if len(set(basis_keys)) != len(basis_keys) or basis_keys != sorted(basis_keys):
        _fail("B2_FAMILY_INVALID", f"{label}.evidence_basis_allowed is not canonical")

    status = _json_text(record.get("status"), f"{label}.status")
    if status not in {"ACTIVE", "ACTIVE_UNASSIGNED", "SUPERSEDED", "RETIRED"}:
        _fail("B2_FAMILY_INVALID", f"{label}.status is not closed")
    assignable = record.get("terminal_assignable")
    if not isinstance(assignable, bool) or assignable != (status == "ACTIVE"):
        _fail("B2_FAMILY_INVALID", f"{label}.terminal_assignable disagrees with status")
    successors = record.get("superseded_by")
    if not isinstance(successors, list) or any(
        not isinstance(item, str) or not item for item in successors
    ):
        _fail("B2_FAMILY_INVALID", f"{label}.superseded_by is invalid")
    successor_keys = [encode_canonical(item) for item in successors]
    if len(set(successor_keys)) != len(successor_keys) or successor_keys != sorted(successor_keys):
        _fail("B2_FAMILY_INVALID", f"{label}.superseded_by is not canonical")
    reason = record.get("supersession_reason")
    if reason is not None and (not isinstance(reason, str) or not reason):
        _fail("B2_FAMILY_INVALID", f"{label}.supersession_reason is invalid")
    _json_text(record.get("lifecycle_relation"), f"{label}.lifecycle_relation")

    review = _json_object(record.get("review_provenance"), f"{label}.review_provenance")
    _exact_keys(
        review,
        {"evidence_locators", "review_basis", "review_status"},
        f"{label}.review_provenance",
    )
    _json_text(review.get("review_basis"), f"{label}.review_provenance.review_basis")
    _json_text(review.get("review_status"), f"{label}.review_provenance.review_status")
    evidence_locators = review.get("evidence_locators")
    if not isinstance(evidence_locators, list):
        _fail("B2_FAMILY_INVALID", f"{label}.review_provenance.evidence_locators is not an array")

    if origin == "REV3_LEGACY":
        _json_object(record.get("historical_rev3"), f"{label}.historical_rev3")
        _json_object(record.get("historical_definition"), f"{label}.historical_definition")

    result = dict(record)
    result["evidence_basis_allowed"] = [
        cast(str, item) for item in cast(list[object], allowed_basis)
    ]
    result["superseded_by"] = [cast(str, item) for item in cast(list[object], successors)]
    return result, boundary


def _b2_validate_catalog(
    value: object,
) -> dict[str, tuple[dict[str, object], dict[str, str]]]:
    """Index the B2 catalog; B2 semantic certification remains another gate."""

    catalog = _json_object(value, "B2 requirement family catalog")
    _exact_keys(
        catalog,
        {
            "catalog_family_count",
            "families",
            "legacy_family_count",
            "new_family_count",
            "rev3_catalog_sha256",
            "schema",
            "source_package_sha256",
        },
        "B2 requirement family catalog",
    )
    if (
        catalog.get("schema") != B2_CATALOG_SCHEMA
        or catalog.get("source_package_sha256") != EXPECTED_REV3_ARCHIVE_SHA256
    ):
        _fail(
            "B2_CATALOG_BINDING_INVALID",
            "B2 catalog schema or source package is not the accepted V1 value",
        )
    _json_digest(catalog.get("rev3_catalog_sha256"), "B2 catalog REV3 digest")
    raw_families = catalog.get("families")
    if not isinstance(raw_families, list):
        _fail("B2_CATALOG_INVALID", "B2 catalog families must be an array")
    if catalog.get("legacy_family_count") != 216:
        _fail("B2_CATALOG_INVALID", "B2 catalog legacy family count is not 216")
    if catalog.get("catalog_family_count") != len(raw_families):
        _fail("B2_CATALOG_INVALID", "B2 catalog family count is not closed")

    families: dict[str, tuple[dict[str, object], dict[str, str]]] = {}
    ids: list[str] = []
    for index, item in enumerate(cast(list[object], raw_families)):
        family, boundary = _b2_validate_family_record(item, f"B2 family[{index}]")
        family_id = cast(str, family["family_id"])
        if family_id in families:
            _fail("B2_FAMILY_AMBIGUOUS", f"B2 family {family_id!r} appears more than once")
        families[family_id] = (family, boundary)
        ids.append(family_id)
    if ids != sorted(ids, key=encode_canonical):
        _fail("B2_CATALOG_ORDER_INVALID", "B2 catalog families are not canonically ordered")
    new_count = sum(1 for family, _ in families.values() if family["family_origin"] == "B2_NEW")
    if (
        catalog.get("new_family_count") != new_count
        or catalog.get("catalog_family_count") != 216 + new_count
    ):
        _fail("B2_CATALOG_INVALID", "B2 catalog new-family counts are not closed")
    return families


def _b2_verify_closure_bindings(
    resolver: AuthoritySourceResolver,
    value: object,
    bindings: B2ArtifactBindingsV1,
) -> None:
    """Verify only closure-to-artifact bindings, never B2 certification state.

    Gate status and aggregate metrics are owned by the existing B2 verifier.
    This resolver deliberately does not reinterpret them.
    """

    closure = _json_object(value, "B2 classification closure")
    if (
        closure.get("schema") != B2_CLOSURE_SCHEMA
        or closure.get("source_package_sha256") != EXPECTED_REV3_ARCHIVE_SHA256
    ):
        _fail(
            "B2_CLOSURE_BINDING_INVALID",
            "B2 closure schema or source package is not the accepted V1 value",
        )
    bound = closure.get("bound_artifacts")
    if not isinstance(bound, list):
        _fail("B2_CLOSURE_INVALID", "B2 closure bound_artifacts must be an array")

    by_path: dict[str, str] = {}
    for index, item in enumerate(cast(list[object], bound)):
        record = _json_object(item, f"B2 closure bound_artifacts[{index}]")
        _exact_keys(record, _B2_BOUND_ARTIFACT_KEYS, f"B2 closure bound_artifacts[{index}]")
        path = _relative_path(record.get("path"), "B2 closure bound artifact path")
        if path in by_path:
            _fail("B2_CLOSURE_INVALID", f"B2 closure repeats bound artifact {path!r}")
        digest = _json_digest(record.get("raw_sha256"), f"B2 closure {path} digest").hex()
        by_path[path] = digest

    expected = {
        "requirement_family_catalog.v1.json": bindings.catalog.raw_sha256.hex(),
        "card_semantic_classifications.v1.json": bindings.classifications.raw_sha256.hex(),
    }
    for path, expected_digest in expected.items():
        if by_path.get(path) != expected_digest:
            _fail("B2_CLOSURE_BINDING_MISMATCH", f"B2 closure does not bind {path} bytes")
        resolver.resolve_repository_artifact(
            f"sources/m2_5/closures/B2/{path}",
            expected_digest,
            B2_CATALOG_SCHEMA
            if path.startswith("requirement_family_catalog")
            else B2_CLASSIFICATION_SCHEMA,
        )


def _b2_require_binding(
    binding: SourceBindingDigestV1, role: str, path: str, schema: str, label: str
) -> None:
    if not isinstance(binding, SourceBindingDigestV1):
        _fail("B2_SOURCE_BINDING_INVALID", f"{label} is not a SourceBindingDigestV1")
    if binding.artifact_role != role or binding.path != path or binding.schema_or_null != schema:
        _fail("B2_SOURCE_BINDING_INVALID", f"{label} does not use the admitted role/path/schema")


def _b2_require_bindings(bindings: B2ArtifactBindingsV1) -> None:
    if not isinstance(bindings, B2ArtifactBindingsV1):
        _fail("B2_SOURCE_BINDING_INVALID", "B2 resolution requires B2ArtifactBindingsV1")
    _b2_require_binding(
        bindings.catalog,
        "b2_catalog",
        B2_CATALOG_PATH,
        B2_CATALOG_SCHEMA,
        "B2 catalog binding",
    )
    _b2_require_binding(
        bindings.classifications,
        "b2_classifications",
        B2_CLASSIFICATION_PATH,
        B2_CLASSIFICATION_SCHEMA,
        "B2 classification binding",
    )
    _b2_require_binding(
        bindings.closure,
        "b2_closure",
        B2_CLOSURE_PATH,
        B2_CLOSURE_SCHEMA,
        "B2 closure binding",
    )


def _verified_json_value(artifact: ResolvedArtifact, label: str) -> tuple[ResolvedArtifact, object]:
    """Revalidate and parse bytes from a resolver-issued artifact."""

    if artifact._verification_token is not _VERIFIED_ARTIFACT_TOKEN:
        _fail("ARTIFACT_UNVERIFIED", f"{label} requires a resolver-verified artifact")
    verified = _resolved_artifact(
        artifact.source_kind,
        artifact.path,
        artifact.raw_bytes,
        artifact.raw_sha256,
        artifact.schema_or_null,
    )
    try:
        value = json.loads(verified.raw_bytes.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        _fail("JSON_INVALID", f"{label} is not valid UTF-8 JSON: {exc}")
    return verified, value


def _b1_verified_json_document(
    artifact: ResolvedArtifact, label: str
) -> tuple[ResolvedArtifact, dict[str, object]]:
    verified, value = _verified_json_value(artifact, label)
    document = _json_object(value, label)
    immutable = ResolvedArtifact(
        verified.source_kind,
        verified.path,
        verified.raw_bytes,
        verified.raw_sha256,
        verified.schema_or_null,
        _freeze_json(value),
        _VERIFIED_ARTIFACT_TOKEN,
    )
    return immutable, document


def _b1_require_binding(
    binding: SourceBindingDigestV1, role: str, path: str, schema: str, label: str
) -> None:
    if not isinstance(binding, SourceBindingDigestV1):
        _fail("B1_SOURCE_BINDING_INVALID", f"{label} is not a SourceBindingDigestV1")
    if binding.artifact_role != role or binding.path != path or binding.schema_or_null != schema:
        _fail("B1_SOURCE_BINDING_INVALID", f"{label} does not use the admitted role/path/schema")


def _b1_require_bindings(bindings: B1FinalArtifactBindingsV1) -> None:
    if not isinstance(bindings, B1FinalArtifactBindingsV1):
        _fail("B1_SOURCE_BINDING_INVALID", "B1.Final resolution requires B1FinalArtifactBindingsV1")
    _b1_require_binding(
        bindings.citations,
        "b1_final_citations",
        B1_FINAL_CITATIONS_PATH,
        B1_FINAL_CITATIONS_SCHEMA,
        "B1.Final citations binding",
    )
    _b1_require_binding(
        bindings.closure,
        "b1_final_closure",
        B1_FINAL_CLOSURE_PATH,
        B1_FINAL_CLOSURE_SCHEMA,
        "B1.Final closure binding",
    )


def _b1_artifact_identity(value: object, label: str) -> dict[str, object]:
    record = _json_object(value, label)
    for key in ("artifact_path", "artifact_sha256"):
        if key not in record:
            _fail("B1_AUTHORITY_INVALID", f"{label}.{key} is missing")
    path = record.get("artifact_path")
    digest = record.get("artifact_sha256")
    if path is None and digest is None:
        return dict(record)
    if path is None or digest is None:
        _fail("B1_AUTHORITY_INVALID", f"{label} has only one half of its source identity")
    normalized = dict(record)
    normalized["artifact_path"] = _relative_path(path, f"{label}.artifact_path")
    normalized["artifact_sha256"] = _json_digest(digest, f"{label}.artifact_sha256").hex()
    return normalized


def _b1_register_index(value: object) -> dict[str, Mapping[str, object]]:
    if not isinstance(value, list):
        _fail("B1_REGISTER_INVALID", "REV3 official-authority register must be an array")
    result: dict[str, Mapping[str, object]] = {}
    for index, item in enumerate(cast(list[object], value)):
        record = _json_object(item, f"REV3 authority register[{index}]")
        authority_id = _json_text(record.get("authority_id"), "REV3 authority ID")
        for key in ("artifact_path", "artifact_sha256"):
            if key not in record:
                _fail("B1_REGISTER_INVALID", f"REV3 authority register lacks {key}")
        if authority_id in result:
            _fail("B1_REGISTER_AMBIGUOUS", f"REV3 authority register repeats {authority_id!r}")
        result[authority_id] = record
    if set(result) != set(B1_FINAL_AUTHORITY_IDS):
        _fail(
            "B1_REGISTER_INVALID", "REV3 authority register universe is not the B1.Final universe"
        )
    return result


def _b1_validate_locator_shape(value: object, citation_kind: str, label: str) -> dict[str, object]:
    locator = _json_object(value, label)
    if citation_kind == "CR_RULE_IDENTIFIER":
        allowed = {
            "locator_kind",
            "line_number_1based",
            "heading_line_sha256",
            "heading_line_excerpt",
        }
        required = {"locator_kind", "line_number_1based", "heading_line_sha256"}
        if set(locator) not in (required, allowed):
            _fail("B1_LOCATOR_INVALID", f"{label} fields are not the admitted CR locator shape")
        if locator.get("locator_kind") != "RULE_HEADING_LINE":
            _fail("B1_LOCATOR_INVALID", f"{label}.locator_kind is not RULE_HEADING_LINE")
        line_number = locator.get("line_number_1based")
        if isinstance(line_number, bool) or not isinstance(line_number, int) or line_number < 1:
            _fail("B1_LOCATOR_INVALID", f"{label}.line_number_1based is invalid")
        _json_digest(locator.get("heading_line_sha256"), f"{label}.heading_line_sha256")
        if "heading_line_excerpt" in locator:
            _json_text(locator.get("heading_line_excerpt"), f"{label}.heading_line_excerpt")
    else:
        allowed = {
            "locator_kind",
            "byte_offset",
            "byte_length",
            "fragment_sha256",
            "section_heading_excerpt",
        }
        required = {"locator_kind", "byte_offset", "byte_length", "fragment_sha256"}
        if set(locator) not in (required, allowed):
            _fail("B1_LOCATOR_INVALID", f"{label} fields are not the admitted fragment shape")
        if locator.get("locator_kind") != "UNIQUE_BYTE_FRAGMENT":
            _fail("B1_LOCATOR_INVALID", f"{label}.locator_kind is not UNIQUE_BYTE_FRAGMENT")
        offset = locator.get("byte_offset")
        length = locator.get("byte_length")
        if (
            isinstance(offset, bool)
            or not isinstance(offset, int)
            or offset < 0
            or isinstance(length, bool)
            or not isinstance(length, int)
            or length <= 0
        ):
            _fail("B1_LOCATOR_INVALID", f"{label} byte range is invalid")
        _json_digest(locator.get("fragment_sha256"), f"{label}.fragment_sha256")
        if "section_heading_excerpt" in locator:
            _json_text(locator.get("section_heading_excerpt"), f"{label}.section_heading_excerpt")
    return dict(locator)


def _b1_validate_citation(value: object, label: str) -> dict[str, object]:
    citation = _json_object(value, label)
    for key in (
        "citation_id",
        "citation_kind",
        "rule_identifier",
        "artifact_local_locator",
        "why_required",
    ):
        if key not in citation:
            _fail("B1_CITATION_INVALID", f"{label}.{key} is missing")
    citation_id = _json_text(citation.get("citation_id"), f"{label}.citation_id")
    citation_kind = _json_text(citation.get("citation_kind"), f"{label}.citation_kind")
    if citation_kind not in _B1_FINAL_CITATION_KINDS:
        _fail("B1_CITATION_INVALID", f"{label}.citation_kind is not closed")
    rule_identifier = citation.get("rule_identifier")
    if citation_kind == "CR_RULE_IDENTIFIER":
        if (
            not isinstance(rule_identifier, str)
            or _CR_RULE_IDENTIFIER_RE.fullmatch(rule_identifier) is None
        ):
            _fail("B1_CITATION_INVALID", f"{label}.rule_identifier is not canonical")
    elif rule_identifier is not None:
        _fail("B1_CITATION_INVALID", f"{label} policy/release citation has a rule identifier")
    _json_text(citation.get("why_required"), f"{label}.why_required")
    locator = _b1_validate_locator_shape(
        citation.get("artifact_local_locator"), citation_kind, f"{label}.artifact_local_locator"
    )
    result = dict(citation)
    result["citation_id"] = citation_id
    result["citation_kind"] = citation_kind
    result["artifact_local_locator"] = locator
    return result


def _b1_verify_closure_binding(value: object, citations_artifact: ResolvedArtifact) -> None:
    closure = _json_object(value, "B1.Final citation closure")
    if closure.get("schema") != B1_FINAL_CLOSURE_SCHEMA:
        _fail("B1_CLOSURE_BINDING_INVALID", "B1.Final closure schema is not V2")
    bound = _json_object(closure.get("bound_evidence"), "B1.Final closure bound_evidence")
    citations_digest = bound.get("official_authority_citations.v3.json")
    if not isinstance(citations_digest, str) or citations_digest != citations_artifact.raw_sha256:
        _fail(
            "B1_CLOSURE_BINDING_MISMATCH", "B1.Final closure does not bind citation artifact bytes"
        )


def _b1_expected_register_identity(
    authority_id: str, register: Mapping[str, object]
) -> tuple[str | None, str | None]:
    register_id = _json_text(register.get("authority_id"), "REV3 register authority ID")
    if register_id != authority_id:
        _fail("B1_REGISTER_BINDING_MISMATCH", "REV3 register authority ID differs")
    path = register.get("artifact_path")
    digest = register.get("artifact_sha256")
    if path is None and digest is None:
        return None, None
    if path is None or digest is None:
        _fail("B1_REGISTER_INVALID", f"REV3 register entry for {authority_id!r} is incomplete")
    return (
        _relative_path(path, "REV3 registered artifact path"),
        _json_digest(digest, "REV3 registered artifact digest").hex(),
    )


def _candidate_identity_reference(value: object, label: str) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(record, set(CANDIDATE_IDENTITY_KEYS), label)
    expected_text = {
        "algorithm_id": "sha-256",
        "envelope_id": "mtgml.digest-envelope.v1",
        "input_schema_id": "manafold.m2.5.c.candidate-identity-input.v1",
        "payload_codec_id": "mtgml.canonical-cbor.v1",
        "semantic_domain": "manafold.m2.5.c.candidate-identity.v1",
    }
    for key, expected in expected_text.items():
        if record.get(key) != expected:
            _fail("CANDIDATE_IDENTITY_INVALID", f"{label}.{key} is not the V1 value")
    _json_digest(record.get("digest_hex"), f"{label}.digest_hex")
    return dict(record)


def _model_vocabularies(value: object) -> ModelVocabularies:
    model = _json_object(value, "declared interaction model")
    if model.get("schema") != DECLARED_MODEL_SCHEMA:
        _fail("MODEL_SCHEMA_MISMATCH", "declared interaction model schema is not V2")
    if model.get("model_id") != CANDIDATE_UNIVERSE_MODEL_ID:
        _fail("MODEL_IDENTITY_MISMATCH", "declared interaction model is not the C model")

    def string_set(field: str) -> frozenset[str]:
        raw = model.get(field)
        if (
            not isinstance(raw, list)
            or not raw
            or any(not isinstance(item, str) or not item for item in raw)
        ):
            _fail("MODEL_VOCABULARY_INVALID", f"declared interaction model {field} is invalid")
        values = [cast(str, item) for item in raw]
        if len(set(values)) != len(values):
            _fail(
                "MODEL_VOCABULARY_INVALID",
                f"declared interaction model {field} contains duplicates",
            )
        return frozenset(values)

    participant_kinds = string_set("participant_kind_vocabulary")
    participant_roles = string_set("participant_role_vocabulary")
    dimensions = model.get("context_dimensions")
    if dimensions != list(SOURCE_CONTEXT_KEYS):
        _fail(
            "MODEL_VOCABULARY_INVALID",
            "declared interaction model context dimensions differ from C V1",
        )
    raw_context = model.get("context_value_vocabulary")
    if not isinstance(raw_context, dict) or set(raw_context) != set(SOURCE_CONTEXT_KEYS):
        _fail(
            "MODEL_VOCABULARY_INVALID",
            "declared interaction model context vocabulary is incomplete",
        )
    context_values: dict[str, frozenset[str]] = {}
    for dimension in SOURCE_CONTEXT_KEYS:
        raw_values = raw_context.get(dimension)
        if (
            not isinstance(raw_values, list)
            or not raw_values
            or any(not isinstance(item, str) or not item for item in raw_values)
        ):
            _fail(
                "MODEL_VOCABULARY_INVALID",
                f"declared interaction model {dimension} vocabulary is invalid",
            )
        values = [cast(str, item) for item in raw_values]
        if len(set(values)) != len(values):
            _fail(
                "MODEL_VOCABULARY_INVALID",
                f"declared interaction model {dimension} vocabulary has duplicates",
            )
        context_values[dimension] = frozenset(values)
    return participant_kinds, participant_roles, context_values


def _participant_ref(
    value: object, label: str, participant_kinds: frozenset[str]
) -> dict[str, str]:
    record = _json_object(value, label)
    _exact_keys(record, {"participant_kind", "semantic_ref"}, label)
    participant_kind = _json_text(record.get("participant_kind"), f"{label}.participant_kind")
    if participant_kind not in participant_kinds:
        _fail("PARTICIPANT_BINDING_INVALID", f"{label}.participant_kind is not a V1 value")
    return {
        "participant_kind": participant_kind,
        "semantic_ref": _json_text(record.get("semantic_ref"), f"{label}.semantic_ref"),
    }


def _rev3_source_binding(value: object, label: str) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(record, set(REV3_SOURCE_BINDING_KEYS), label)
    if record.get("kind") != "rev3":
        _fail("SOURCE_KIND_UNSUPPORTED", f"{label}.kind is not the supported REV3 kind")
    archive_member = _json_text(record.get("archive_member"), f"{label}.archive_member")
    if archive_member != REV3_CENSUS_MEMBER:
        _fail("SOURCE_MEMBER_MISMATCH", f"{label}.archive_member is not the C census member")
    archive_member_sha256 = _json_digest(
        record.get("archive_member_sha256"), f"{label}.archive_member_sha256"
    ).hex()
    row_ordinal = record.get("row_ordinal")
    if isinstance(row_ordinal, bool) or not isinstance(row_ordinal, int) or row_ordinal < 0:
        _fail("REV3_ROW_ORDINAL_INVALID", f"{label}.row_ordinal must be a nonnegative integer")
    raw_columns = record.get("source_columns")
    if not isinstance(raw_columns, list) or any(not isinstance(item, str) for item in raw_columns):
        _fail("REV3_SOURCE_COLUMNS_MISMATCH", f"{label}.source_columns must be a text array")
    source_columns = [cast(str, item) for item in raw_columns]
    if source_columns != list(REV3_SOURCE_COLUMNS):
        _fail("REV3_SOURCE_COLUMNS_MISMATCH", f"{label}.source_columns differs from the C contract")
    raw_values = record.get("source_values")
    if not isinstance(raw_values, list) or any(not isinstance(item, str) for item in raw_values):
        _fail("REV3_SOURCE_VALUES_INVALID", f"{label}.source_values must be a text array")
    source_values = [cast(str, item) for item in raw_values]
    if len(source_values) != len(source_columns):
        _fail("REV3_SOURCE_VALUES_INVALID", f"{label}.source_values length differs from columns")
    return {
        "kind": "rev3",
        "archive_member": archive_member,
        "archive_member_sha256": archive_member_sha256,
        "row_ordinal": row_ordinal,
        "source_columns": source_columns,
        "source_values": source_values,
    }


def _candidate_identity_for_record(candidate: Mapping[str, object]) -> dict[str, str]:
    binding = cast(Mapping[str, object], candidate["source_binding"])
    participant_payload = []
    for ref in cast(list[Mapping[str, str]], candidate["participant_refs"]):
        participant_payload.append(
            [
                [ref["participant_kind"], None],
                ref["semantic_ref"],
            ]
        )
    payload = [
        [candidate["source_origin"], None],
        [candidate["scope"], None],
        [candidate["relation"], None],
        participant_payload,
        list(cast(list[str], candidate["supporting_requirement_ids"])),
        [
            ["rev3", None],
            [
                binding["archive_member"],
                bytes.fromhex(cast(str, binding["archive_member_sha256"])),
                binding["row_ordinal"],
                list(cast(list[str], binding["source_columns"])),
                list(cast(list[str], binding["source_values"])),
            ],
        ],
    ]
    digest_bytes = hash_envelope(
        encode_envelope(
            CANDIDATE_IDENTITY_DOMAIN,
            CANDIDATE_IDENTITY_SCHEMA,
            encode_canonical(cast(list[object], payload)),
        )
    )
    return {
        "envelope_id": DIGEST_ENVELOPE_ID,
        "algorithm_id": SHA256_ID,
        "semantic_domain": CANDIDATE_IDENTITY_DOMAIN,
        "payload_codec_id": CANONICAL_CBOR_ID,
        "input_schema_id": CANDIDATE_IDENTITY_SCHEMA,
        "digest_hex": digest_bytes.hex(),
    }


def _candidate_record(
    value: object, label: str, participant_kinds: frozenset[str]
) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(record, set(CANDIDATE_RECORD_KEYS), label)
    candidate_id = _json_text(record.get("candidate_id"), f"{label}.candidate_id")
    identity = _candidate_identity_reference(
        record.get("candidate_identity"), f"{label}.candidate_identity"
    )
    source_origin = _json_text(record.get("source_origin"), f"{label}.source_origin")
    if source_origin not in CANDIDATE_SOURCE_ORIGINS:
        _fail("CANDIDATE_SOURCE_ORIGIN_INVALID", f"{label}.source_origin is not a V1 value")
    if source_origin != "rev3":
        _fail("SOURCE_KIND_UNSUPPORTED", f"{label} is not a REV3-backed candidate")
    scope = _json_text(record.get("scope"), f"{label}.scope")
    if scope not in CANDIDATE_SCOPES:
        _fail("CANDIDATE_SHAPE_INVALID", f"{label}.scope is not a V1 value")
    relation = _json_text(record.get("relation"), f"{label}.relation")
    if relation not in CANDIDATE_RELATIONS:
        _fail("CANDIDATE_SHAPE_INVALID", f"{label}.relation is not a V1 value")
    raw_participants = record.get("participant_refs")
    if not isinstance(raw_participants, list):
        _fail("PARTICIPANT_BINDING_INVALID", f"{label}.participant_refs must be an array")
    participants = [
        _participant_ref(item, f"{label}.participant_refs[{index}]", participant_kinds)
        for index, item in enumerate(raw_participants)
    ]
    raw_supporting_ids = record.get("supporting_requirement_ids")
    if not isinstance(raw_supporting_ids, list) or any(
        not isinstance(item, str) or not item for item in raw_supporting_ids
    ):
        _fail("CANDIDATE_SUPPORTING_IDS_INVALID", f"{label}.supporting_requirement_ids is invalid")
    supporting_ids = [cast(str, item) for item in raw_supporting_ids]
    supporting_keys = [encode_canonical(item) for item in supporting_ids]
    if len(set(supporting_keys)) != len(supporting_keys):
        _fail(
            "CANDIDATE_SUPPORTING_IDS_DUPLICATE",
            f"{label}.supporting_requirement_ids has duplicates",
        )
    if supporting_keys != sorted(supporting_keys):
        _fail(
            "CANDIDATE_SUPPORTING_IDS_ORDER_INVALID",
            f"{label}.supporting_requirement_ids is not canonical",
        )
    source_binding = _rev3_source_binding(record.get("source_binding"), f"{label}.source_binding")
    source_values = cast(list[str], source_binding["source_values"])
    if source_values[0] != candidate_id:
        _fail("CANDIDATE_SOURCE_BINDING_MISMATCH", f"{label}.source_values[0] is not candidate_id")
    reconciliation_status = _json_text(
        record.get("reconciliation_status"), f"{label}.reconciliation_status"
    )
    if reconciliation_status not in RECONCILIATION_STATUSES:
        _fail("RECONCILIATION_STATUS_INVALID", f"{label}.reconciliation_status is not a V1 value")
    return {
        "candidate_id": candidate_id,
        "candidate_identity": identity,
        "source_origin": source_origin,
        "scope": scope,
        "relation": relation,
        "participant_refs": participants,
        "supporting_requirement_ids": supporting_ids,
        "source_binding": source_binding,
        "reconciliation_status": reconciliation_status,
        "reconciliation_reason": _json_text(
            record.get("reconciliation_reason"), f"{label}.reconciliation_reason"
        ),
    }


def _source_instance_record(
    value: object,
    label: str,
    participant_kinds: frozenset[str],
    participant_roles: frozenset[str],
    context_values: Mapping[str, frozenset[str]],
) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(record, set(SOURCE_INSTANCE_RECORD_KEYS), label)
    source_instance_id = _json_text(record.get("source_instance_id"), f"{label}.source_instance_id")
    candidate_id = _json_text(record.get("candidate_id"), f"{label}.candidate_id")
    source_binding = _rev3_source_binding(record.get("source_binding"), f"{label}.source_binding")
    raw_participants = record.get("participant_bindings")
    if not isinstance(raw_participants, list):
        _fail("PARTICIPANT_BINDING_INVALID", f"{label}.participant_bindings must be an array")
    participant_bindings: list[dict[str, object]] = []
    for index, item in enumerate(raw_participants):
        binding = _json_object(item, f"{label}.participant_bindings[{index}]")
        _exact_keys(binding, {"role", "participant_ref"}, f"{label}.participant_bindings[{index}]")
        role = _json_text(binding.get("role"), f"{label}.participant_bindings[{index}].role")
        if role not in participant_roles:
            _fail(
                "PARTICIPANT_BINDING_INVALID",
                f"{label}.participant_bindings[{index}].role is invalid",
            )
        participant_bindings.append(
            {
                "role": role,
                "participant_ref": _participant_ref(
                    binding.get("participant_ref"),
                    f"{label}.participant_bindings[{index}].participant_ref",
                    participant_kinds,
                ),
            }
        )
    source_context = _json_object(record.get("source_context"), f"{label}.source_context")
    _exact_keys(source_context, set(SOURCE_CONTEXT_KEYS), f"{label}.source_context")
    context: dict[str, str] = {}
    for key in SOURCE_CONTEXT_KEYS:
        context_value = _json_text(source_context.get(key), f"{label}.source_context.{key}")
        if context_value not in context_values[key]:
            _fail(
                "SOURCE_CONTEXT_VALUE_INVALID",
                f"{label}.source_context.{key} is not in the declared model vocabulary",
            )
        context[key] = context_value
    return {
        "source_instance_id": source_instance_id,
        "candidate_id": candidate_id,
        "source_binding": source_binding,
        "participant_bindings": participant_bindings,
        "source_context": context,
    }


def _source_binding_key(binding: Mapping[str, object]) -> tuple[object, ...]:
    return (
        binding["kind"],
        binding["archive_member"],
        binding["archive_member_sha256"],
        binding["row_ordinal"],
        tuple(cast(list[str], binding["source_columns"])),
        tuple(cast(list[str], binding["source_values"])),
    )


def _source_instance_tuple_bytes(record: Mapping[str, object]) -> bytes:
    binding = cast(Mapping[str, object], record["source_binding"])
    binding_payload = [
        ["rev3", None],
        [
            binding["archive_member"],
            bytes.fromhex(cast(str, binding["archive_member_sha256"])),
            binding["row_ordinal"],
            list(cast(list[str], binding["source_columns"])),
            list(cast(list[str], binding["source_values"])),
        ],
    ]
    participant_payload = []
    for item in cast(list[dict[str, object]], record["participant_bindings"]):
        participant_ref = cast(Mapping[str, str], item["participant_ref"])
        participant_payload.append(
            [
                [item["role"], None],
                [[participant_ref["participant_kind"], None], participant_ref["semantic_ref"]],
            ]
        )
    context = cast(Mapping[str, str], record["source_context"])
    context_payload = [[context[key], None] for key in SOURCE_CONTEXT_KEYS]
    return encode_canonical([binding_payload, participant_payload, context_payload])


def _source_instance_id(candidate_id: str, index: int) -> str:
    encoded = base64.urlsafe_b64encode(candidate_id.encode("utf-8")).decode("ascii").rstrip("=")
    return f"si.v1/{encoded}/{index}"


def _raw_artifact_binding(value: object, label: str, expected_path: str) -> dict[str, str]:
    record = _json_object(value, label)
    _exact_keys(record, set(RAW_ARTIFACT_BINDING_KEYS), label)
    path = _json_text(record.get("path"), f"{label}.path")
    if path != expected_path:
        _fail("CANDIDATE_UNIVERSE_INPUT_BINDING_MISMATCH", f"{label}.path is not the admitted path")
    return {
        "path": path,
        "raw_sha256": _json_digest(record.get("raw_sha256"), f"{label}.raw_sha256").hex(),
    }


def _candidate_universe_input_bindings(value: object) -> dict[str, object]:
    inputs = _json_object(value, "candidate universe input_bindings")
    _exact_keys(inputs, set(INPUT_BINDING_KEYS), "candidate universe input_bindings")
    declared_model = _raw_artifact_binding(
        inputs.get("declared_model"),
        "candidate universe declared_model",
        EXPECTED_STATIC_INPUT_PATHS["declared_model"],
    )
    review_additions = _raw_artifact_binding(
        inputs.get("review_additions"),
        "candidate universe review_additions",
        EXPECTED_STATIC_INPUT_PATHS["review_additions"],
    )
    rev3_record = _json_object(inputs.get("rev3_candidate_source"), "candidate universe rev3 input")
    _exact_keys(rev3_record, set(REV3_INPUT_BINDING_KEYS), "candidate universe rev3 input")
    if rev3_record.get("archive_member") != REV3_CENSUS_MEMBER:
        _fail(
            "CANDIDATE_UNIVERSE_INPUT_BINDING_MISMATCH",
            "candidate universe REV3 member is not the C census",
        )
    rev3_binding = {
        "archive_member": REV3_CENSUS_MEMBER,
        "archive_member_sha256": _json_digest(
            rev3_record.get("archive_member_sha256"), "candidate universe REV3 member digest"
        ).hex(),
        "source_package_sha256": _json_digest(
            rev3_record.get("source_package_sha256"), "candidate universe REV3 package digest"
        ).hex(),
    }
    result: dict[str, object] = {
        "declared_model": declared_model,
        "review_additions": review_additions,
        "rev3_candidate_source": rev3_binding,
    }
    for key in ("b2_artifacts", "b1_final_artifacts"):
        raw_items = inputs.get(key)
        if not isinstance(raw_items, list):
            _fail("CANDIDATE_UNIVERSE_INPUT_BINDING_INVALID", f"{key} must be an array")
        expected_paths = cast(tuple[str, ...], EXPECTED_STATIC_INPUT_PATHS[key])
        if len(raw_items) != len(expected_paths):
            _fail(
                "CANDIDATE_UNIVERSE_INPUT_BINDING_INVALID", f"{key} does not cover its fixed inputs"
            )
        result[key] = [
            _raw_artifact_binding(item, f"candidate universe {key}[{index}]", expected_paths[index])
            for index, item in enumerate(raw_items)
        ]
    return result


def _candidate_reconciliation_counts(candidates: list[Mapping[str, object]], value: object) -> None:
    counts = _json_object(value, "candidate reconciliation counts")
    _exact_keys(counts, set(RECONCILIATION_COUNT_KEYS), "candidate reconciliation counts")
    actual: dict[str, int] = {key: 0 for key in RECONCILIATION_COUNT_KEYS}
    for item in candidates:
        status = cast(str, item["reconciliation_status"])
        actual[status] += 1
    actual["new_b2_derived"] = 0
    expected = {
        key: counts[key]
        for key in RECONCILIATION_COUNT_KEYS
        if isinstance(counts[key], int) and not isinstance(counts[key], bool)
    }
    if len(expected) != len(RECONCILIATION_COUNT_KEYS) or expected != actual:
        _fail(
            "CANDIDATE_RECONCILIATION_COUNT_MISMATCH",
            "candidate reconciliation counts are not recomputed",
        )


def _candidate_universe_index(
    artifact: ResolvedArtifact, model_vocabularies: ModelVocabularies
) -> _CandidateUniverseIndex:
    universe = _json_object(artifact.json_value, "candidate universe")
    _exact_keys(
        universe,
        {
            "schema",
            "model_id",
            "input_bindings",
            "candidate_count",
            "candidate_reconciliation_counts",
            "source_instance_count",
            "candidates",
            "source_instances",
        },
        "candidate universe",
    )
    if universe.get("schema") != CANDIDATE_UNIVERSE_SCHEMA:
        _fail("SCHEMA_MISMATCH", "candidate universe schema is not V2")
    if universe.get("model_id") != CANDIDATE_UNIVERSE_MODEL_ID:
        _fail("MODEL_IDENTITY_MISMATCH", "candidate universe model is not the declared C model")
    rev3_input = _candidate_universe_input_bindings(universe.get("input_bindings"))
    participant_kinds, participant_roles, context_values = model_vocabularies
    raw_candidates = universe.get("candidates")
    if not isinstance(raw_candidates, list):
        _fail("CANDIDATE_UNIVERSE_INVALID", "candidate universe candidates must be an array")
    candidates_by_id: dict[str, Mapping[str, object]] = {}
    identities_by_digest: dict[str, Mapping[str, object]] = {}
    bindings_by_id: dict[str, Mapping[str, object]] = {}
    for index, raw_candidate in enumerate(raw_candidates):
        candidate = _candidate_record(raw_candidate, f"candidate[{index}]", participant_kinds)
        if _candidate_identity_for_record(candidate) != cast(
            dict[str, str], candidate["candidate_identity"]
        ):
            _fail(
                "CANDIDATE_IDENTITY_MISMATCH",
                f"candidate[{index}] identity does not match its fixed C preimage",
            )
        candidate_id = cast(str, candidate["candidate_id"])
        if candidate_id in candidates_by_id:
            _fail("DUPLICATE_CANDIDATE_ID", f"candidate {candidate_id!r} appears more than once")
        identity = cast(Mapping[str, object], candidate["candidate_identity"])
        digest_hex = cast(str, identity["digest_hex"])
        if digest_hex in identities_by_digest:
            _fail(
                "DUPLICATE_CANDIDATE_IDENTITY",
                f"candidate identity {digest_hex!r} appears more than once",
            )
        frozen_candidate = _frozen_mapping(candidate)
        candidates_by_id[candidate_id] = frozen_candidate
        identities_by_digest[digest_hex] = cast(
            Mapping[str, object], _frozen_mapping(dict(identity))
        )
        bindings_by_id[candidate_id] = cast(
            Mapping[str, object],
            _frozen_mapping(cast(Mapping[str, object], candidate["source_binding"])),
        )
    candidate_count = universe.get("candidate_count")
    if (
        isinstance(candidate_count, bool)
        or not isinstance(candidate_count, int)
        or candidate_count != len(candidates_by_id)
    ):
        _fail("CANDIDATE_COUNT_MISMATCH", "candidate_count does not match candidates")
    _candidate_reconciliation_counts(
        list(candidates_by_id.values()), universe.get("candidate_reconciliation_counts")
    )

    raw_instances = universe.get("source_instances")
    if not isinstance(raw_instances, list):
        _fail("SOURCE_INSTANCE_LEDGER_INVALID", "source_instances must be an array")
    instances_by_id: dict[str, Mapping[str, object]] = {}
    instances_by_candidate: dict[str, list[Mapping[str, object]]] = {}
    for index, raw_instance in enumerate(raw_instances):
        instance = _source_instance_record(
            raw_instance,
            f"source_instance[{index}]",
            participant_kinds,
            participant_roles,
            context_values,
        )
        instance_id = cast(str, instance["source_instance_id"])
        if instance_id in instances_by_id:
            _fail(
                "DUPLICATE_SOURCE_INSTANCE_ID",
                f"source instance {instance_id!r} appears more than once",
            )
        candidate_id = cast(str, instance["candidate_id"])
        if candidate_id not in candidates_by_id:
            _fail(
                "SOURCE_INSTANCE_CANDIDATE_MISMATCH",
                f"source instance {instance_id!r} has no candidate",
            )
        candidate = candidates_by_id[candidate_id]
        candidate_binding = cast(Mapping[str, object], candidate["source_binding"])
        instance_binding = cast(Mapping[str, object], instance["source_binding"])
        if _source_binding_key(candidate_binding) != _source_binding_key(instance_binding):
            _fail(
                "CANDIDATE_SOURCE_BINDING_MISMATCH",
                f"source instance {instance_id!r} binding differs from candidate",
            )
        candidate_refs = list(cast(tuple[Mapping[str, str], ...], candidate["participant_refs"]))
        instance_refs = [
            cast(Mapping[str, str], cast(Mapping[str, object], item)["participant_ref"])
            for item in cast(list[Mapping[str, object]], instance["participant_bindings"])
        ]
        if instance_refs != candidate_refs:
            _fail(
                "PARTICIPANT_BINDING_MISMATCH",
                f"source instance {instance_id!r} participants differ from candidate",
            )
        frozen_instance = _frozen_mapping(instance)
        instances_by_id[instance_id] = frozen_instance
        instances_by_candidate.setdefault(candidate_id, []).append(frozen_instance)

    for candidate_id in candidates_by_id:
        instances = instances_by_candidate.get(candidate_id, [])
        if not instances:
            _fail(
                "SOURCE_INSTANCE_COVERAGE_MISMATCH",
                f"candidate {candidate_id!r} has no source instance",
            )
        tuple_keys = [_source_instance_tuple_bytes(instance) for instance in instances]
        if len(set(tuple_keys)) != len(tuple_keys):
            _fail(
                "DUPLICATE_SOURCE_INSTANCE_TUPLE",
                f"candidate {candidate_id!r} has duplicate source tuples",
            )
        for index, tuple_key in enumerate(sorted(tuple_keys)):
            instance = instances[tuple_keys.index(tuple_key)]
            expected_id = _source_instance_id(candidate_id, index)
            if instance["source_instance_id"] != expected_id:
                _fail(
                    "SOURCE_INSTANCE_ID_MISMATCH",
                    f"source instance ID is not deterministic for {candidate_id!r}",
                )
    source_instance_count = universe.get("source_instance_count")
    if (
        isinstance(source_instance_count, bool)
        or not isinstance(source_instance_count, int)
        or source_instance_count != len(instances_by_id)
    ):
        _fail(
            "SOURCE_INSTANCE_COUNT_MISMATCH",
            "source_instance_count does not match source_instances",
        )
    return _CandidateUniverseIndex(
        artifact=artifact,
        rev3_input_binding=cast(
            Mapping[str, object],
            _frozen_mapping(cast(Mapping[str, object], rev3_input["rev3_candidate_source"])),
        ),
        candidates_by_id=candidates_by_id,
        candidate_identities_by_digest=identities_by_digest,
        candidate_bindings_by_id=bindings_by_id,
        instances_by_id=instances_by_id,
        instances_by_candidate_id={
            candidate_id: tuple(instances)
            for candidate_id, instances in instances_by_candidate.items()
        },
    )


def _parse_rev3_rows(raw: bytes, path: str) -> list[list[str]]:
    try:
        text = raw.decode("utf-8")
        rows = list(csv.reader(io.StringIO(text, newline=""), strict=True))
    except (UnicodeDecodeError, csv.Error) as exc:
        _fail("REV3_SOURCE_INVALID", f"{path} is not a strict UTF-8 CSV: {exc}")
    if not rows or rows[0] != list(REV3_SOURCE_COLUMNS):
        _fail("REV3_SOURCE_COLUMNS_MISMATCH", f"{path} has unexpected header columns")
    data_rows = rows[1:]
    for ordinal, row in enumerate(data_rows):
        if len(row) != len(REV3_SOURCE_COLUMNS):
            _fail(
                "REV3_SOURCE_COLUMNS_MISMATCH", f"{path} row {ordinal} has the wrong column count"
            )
    return data_rows


def _parse_keyed_csv(
    raw: bytes, path: str, required_columns: frozenset[str]
) -> list[dict[str, str]]:
    try:
        text = raw.decode("utf-8")
        rows = list(csv.reader(io.StringIO(text, newline=""), strict=True))
    except (UnicodeDecodeError, csv.Error) as exc:
        _fail("REV3_SOURCE_INVALID", f"{path} is not a strict UTF-8 CSV: {exc}")
    if not rows:
        _fail("REV3_SOURCE_COLUMNS_MISMATCH", f"{path} has no header")
    header = rows[0]
    if len(header) != len(set(header)) or not required_columns.issubset(header):
        _fail("REV3_SOURCE_COLUMNS_MISMATCH", f"{path} lacks required join columns")
    records: list[dict[str, str]] = []
    for ordinal, row in enumerate(rows[1:]):
        if len(row) != len(header):
            _fail(
                "REV3_SOURCE_COLUMNS_MISMATCH", f"{path} row {ordinal} has the wrong column count"
            )
        records.append(dict(zip(header, row, strict=True)))
    return records


def _reject_duplicate_json_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def _parse_supporting_requirement_ids(value: str, label: str) -> list[str]:
    if value.strip() != value:
        _fail("REV3_SUPPORTING_IDS_INVALID", f"{label} has surrounding whitespace")
    try:
        parsed = json.loads(value, object_pairs_hook=_reject_duplicate_json_keys)
    except (json.JSONDecodeError, ValueError) as exc:
        _fail("REV3_SUPPORTING_IDS_INVALID", f"{label} is not exact JSON: {exc}")
    if not isinstance(parsed, list) or any(
        not isinstance(item, str) or not item for item in parsed
    ):
        _fail("REV3_SUPPORTING_IDS_INVALID", f"{label} must be a non-empty text array")
    ids = [cast(str, item) for item in parsed]
    keys = [encode_canonical(item) for item in ids]
    if len(set(keys)) != len(keys):
        _fail("REV3_SUPPORTING_IDS_INVALID", f"{label} contains duplicate IDs")
    return sorted(ids, key=encode_canonical)


def _normalized_candidate_fields(row: Mapping[str, str]) -> dict[str, object]:
    if row.get("model_id") != REV3_MODEL_ID:
        _fail("REV3_MODEL_ID_MISMATCH", "REV3 candidate row model ID is not interaction-model.v1")
    raw_scope = row.get("scope")
    raw_relation = row.get("relation")
    if raw_scope not in REV3_SCOPE_MAP or raw_relation not in REV3_RELATION_MAP:
        _fail("REV3_SHAPE_MISMATCH", "REV3 candidate row has an unknown scope or relation")
    shape = (cast(str, raw_scope), cast(str, raw_relation))
    if shape not in REV3_SHAPES:
        _fail("REV3_SHAPE_MISMATCH", "REV3 candidate row has an unsupported scope/relation pair")
    candidate_id = row.get("candidate_id")
    pair_id = row.get("pair_id")
    left_family_id = row.get("left_family_id")
    right_family_id = row.get("right_family_id")
    supporting_requirement_ids = row.get("supporting_requirement_ids")
    if any(
        value is None or not value
        for value in (
            candidate_id,
            pair_id,
            left_family_id,
            right_family_id,
            supporting_requirement_ids,
        )
    ):
        _fail("REV3_SOURCE_NORMALIZATION_INVALID", "REV3 candidate row has an empty identity cell")
    candidate_id = cast(str, candidate_id)
    pair_id = cast(str, pair_id)
    left_family_id = cast(str, left_family_id)
    right_family_id = cast(str, right_family_id)
    if raw_relation == "DECLARED_CARD_TRIGGER":
        if pair_id != left_family_id or pair_id != right_family_id:
            _fail("REV3_TRIGGER_JOIN_INVALID", "card-trigger row participant cells differ")
        if _LOWERCASE_UUID_RE.fullmatch(pair_id) is None:
            _fail(
                "REV3_TRIGGER_JOIN_INVALID",
                "card-trigger row does not name a lowercase OSI UUID",
            )
        participants = [{"participant_kind": "card", "semantic_ref": pair_id}]
    else:
        participants = [
            {"participant_kind": "requirement_family", "semantic_ref": left_family_id},
            {"participant_kind": "requirement_family", "semantic_ref": right_family_id},
        ]
        if raw_relation == "UNORDERED_BINARY":
            if left_family_id.encode("utf-8") > right_family_id.encode("utf-8"):
                _fail(
                    "REV3_SOURCE_ROW_ORDER_INVALID",
                    "unordered REV3 family order is not canonical",
                )
            participants.sort(
                key=lambda item: encode_canonical(
                    [[item["participant_kind"], None], item["semantic_ref"]]
                )
            )
    supporting_ids = _parse_supporting_requirement_ids(
        cast(str, supporting_requirement_ids),
        "REV3 supporting_requirement_ids",
    )
    if raw_relation == "DECLARED_CARD_TRIGGER" and supporting_ids != [pair_id]:
        _fail("REV3_TRIGGER_JOIN_INVALID", "card-trigger supporting IDs do not equal the OSI")
    return {
        "candidate_id": candidate_id,
        "scope": REV3_SCOPE_MAP[cast(str, raw_scope)],
        "relation": REV3_RELATION_MAP[cast(str, raw_relation)],
        "participant_refs": participants,
        "supporting_requirement_ids": supporting_ids,
    }


class Rev3ArchiveStore:
    """Verified access to members of the pinned, non-extracted REV3 ZIP."""

    def __init__(
        self,
        raw: bytes,
        archive: zipfile.ZipFile,
        manifest_entries: dict[str, tuple[int, str]],
    ) -> None:
        self._raw = raw
        self._archive = archive
        self._manifest_entries = manifest_entries

    @property
    def archive_sha256(self) -> str:
        return hashlib.sha256(self._raw).hexdigest()

    @classmethod
    def from_root(
        cls,
        root: Path,
        expected_archive_sha256: str = EXPECTED_REV3_ARCHIVE_SHA256,
    ) -> Rev3ArchiveStore:
        configured_root = root.resolve()
        archive_path = (configured_root / REV3_ARCHIVE_RELATIVE_PATH).resolve()
        try:
            archive_path.relative_to(configured_root)
        except ValueError:
            _fail("REV3_ARCHIVE_PATH_INVALID", "REV3 archive path escapes its configured root")
        if not archive_path.is_file():
            _blocked("REV3_ARCHIVE_SOURCE_UNAVAILABLE", f"REV3 archive is missing: {archive_path}")
        try:
            raw = archive_path.read_bytes()
        except OSError as exc:
            _blocked("REV3_ARCHIVE_SOURCE_UNAVAILABLE", f"cannot read {archive_path}: {exc}")
        return cls.from_bytes(raw, expected_archive_sha256)

    @classmethod
    def from_bytes(
        cls,
        raw: bytes,
        expected_archive_sha256: str,
    ) -> Rev3ArchiveStore:
        expected = _digest_hex(expected_archive_sha256, "REV3 archive digest")
        actual = hashlib.sha256(raw).hexdigest()
        if actual != expected:
            _fail("REV3_ARCHIVE_DIGEST_MISMATCH", f"REV3 archive has {actual}, expected {expected}")
        try:
            archive = zipfile.ZipFile(io.BytesIO(raw))
        except (OSError, zipfile.BadZipFile) as exc:
            _fail("REV3_ARCHIVE_INVALID", f"REV3 archive is not a readable ZIP: {exc}")

        infos = archive.infolist()
        names = [info.filename for info in infos]
        if len(names) != len(set(names)):
            _fail("REV3_MEMBER_DUPLICATE", "REV3 archive contains duplicate member names")
        for info in infos:
            if info.is_dir():
                _fail(
                    "REV3_MEMBER_INVALID",
                    f"REV3 archive contains a directory member {info.filename!r}",
                )
            _relative_path(info.filename, "REV3 archive member")
        if REV3_PACKAGE_MANIFEST_MEMBER not in names:
            _fail("REV3_MANIFEST_MISSING", "REV3 package manifest is missing")
        try:
            manifest_raw = archive.read(REV3_PACKAGE_MANIFEST_MEMBER)
            manifest_value = json.loads(manifest_raw.decode("utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError, zipfile.BadZipFile) as exc:
            _fail("REV3_MANIFEST_INVALID", f"REV3 package manifest is unreadable: {exc}")
        manifest = _json_object(manifest_value, "REV3 package manifest")
        if manifest.get("schema") != REV3_PACKAGE_MANIFEST_SCHEMA:
            _fail("REV3_MANIFEST_SCHEMA_MISMATCH", "REV3 package manifest schema is not V1")
        entries = manifest.get("entries")
        excluded = manifest.get("manifest_excluded_paths")
        if not isinstance(entries, list) or not isinstance(excluded, list):
            _fail("REV3_MANIFEST_INVALID", "REV3 package manifest entries are not arrays")
        if manifest.get("manifest_excludes_self") is not True:
            _fail("REV3_MANIFEST_INVALID", "REV3 package manifest must exclude itself")

        manifest_entries: dict[str, tuple[int, str]] = {}
        for raw_entry in entries:
            entry = _json_object(raw_entry, "REV3 package manifest entry")
            _exact_keys(entry, {"bytes", "path", "sha256"}, "REV3 package manifest entry")
            path = _relative_path(entry.get("path"), "REV3 package manifest entry path")
            if path == REV3_PACKAGE_MANIFEST_MEMBER or path in manifest_entries:
                _fail("REV3_MEMBER_DUPLICATE", f"duplicate manifest member {path!r}")
            size = entry.get("bytes")
            if isinstance(size, bool) or not isinstance(size, int) or size < 0:
                _fail("REV3_MANIFEST_INVALID", f"invalid byte count for {path!r}")
            sha = _digest_hex(
                cast(DigestInput, entry.get("sha256", "")), f"manifest member {path} digest"
            )
            manifest_entries[path] = (size, sha)

        excluded_paths: set[str] = set()
        for raw_path in excluded:
            path = _relative_path(raw_path, "REV3 manifest exclusion")
            if path == REV3_PACKAGE_MANIFEST_MEMBER or path in excluded_paths:
                _fail("REV3_MEMBER_DUPLICATE", f"duplicate manifest exclusion {path!r}")
            excluded_paths.add(path)

        actual_members = set(names) - {REV3_PACKAGE_MANIFEST_MEMBER}
        expected_members = set(manifest_entries) | excluded_paths
        missing_declared = sorted(set(manifest_entries) - actual_members)
        if missing_declared:
            _fail(
                "REV3_MEMBER_MISSING", f"REV3 manifest member is missing: {missing_declared[0]!r}"
            )
        if actual_members != expected_members:
            missing = sorted(expected_members - actual_members)
            extra = sorted(actual_members - expected_members)
            _fail(
                "REV3_MEMBER_SET_MISMATCH",
                f"REV3 manifest/member set differs; missing={missing!r}, extra={extra!r}",
            )

        for path, (size, expected_sha) in manifest_entries.items():
            try:
                payload = archive.read(path)
            except (OSError, zipfile.BadZipFile) as exc:
                _fail("REV3_MEMBER_READ_FAILED", f"cannot read REV3 member {path!r}: {exc}")
            if len(payload) != size:
                _fail("REV3_MEMBER_SIZE_MISMATCH", f"REV3 member {path!r} has the wrong size")
            actual_sha = hashlib.sha256(payload).hexdigest()
            if actual_sha != expected_sha:
                _fail(
                    "REV3_MEMBER_DIGEST_MISMATCH",
                    f"REV3 member {path!r} has {actual_sha}, expected {expected_sha}",
                )
        return cls(raw, archive, manifest_entries)

    def expected_member_sha256(self, member_path: str) -> str:
        path = _relative_path(member_path, "REV3 member path")
        entry = self._manifest_entries.get(path)
        if entry is None:
            _fail("REV3_MEMBER_MISSING", f"REV3 member {path!r} is not in the manifest")
        return entry[1]

    def resolve_member(
        self,
        member_path: str,
        expected_raw_sha256: DigestInput,
        schema_or_null: str | None = None,
    ) -> ResolvedArtifact:
        path = _relative_path(member_path, "REV3 member path")
        entry = self._manifest_entries.get(path)
        if entry is None:
            _fail("REV3_MEMBER_MISSING", f"REV3 member {path!r} is not in the manifest")
        try:
            raw = self._archive.read(path)
        except (OSError, zipfile.BadZipFile) as exc:
            _fail("REV3_MEMBER_READ_FAILED", f"cannot read REV3 member {path!r}: {exc}")
        if hashlib.sha256(raw).hexdigest() != entry[1]:
            _fail(
                "REV3_MEMBER_DIGEST_MISMATCH",
                f"REV3 member {path!r} changed after package validation",
            )
        return _resolved_artifact("rev3_archive", path, raw, expected_raw_sha256, schema_or_null)

    def resolve_locator(
        self,
        locator: Locator,
        expected_raw_sha256: DigestInput,
        schema_or_null: str | None = None,
    ) -> ResolvedLocator:
        kind, payload = _locator(locator)
        if kind != "archive_member" or not isinstance(payload, str):
            _fail("LOCATOR_INVALID", "REV3 resolution requires an archive_member locator")
        artifact = self.resolve_member(payload, expected_raw_sha256, schema_or_null)
        return ResolvedLocator(artifact, locator, artifact.raw_bytes)


class AuthoritySourceResolver:
    """Resolve source bytes without granting them semantic authority."""

    def __init__(
        self,
        repo_root: Path,
        *,
        rev3_archive_root: Path | None = None,
        rev3_archive: Rev3ArchiveStore | None = None,
        expected_rev3_archive_sha256: str = EXPECTED_REV3_ARCHIVE_SHA256,
    ) -> None:
        self._repo_root = repo_root.resolve()
        self._rev3_archive_root = rev3_archive_root
        self._rev3_archive = rev3_archive
        self._expected_rev3_archive_sha256 = expected_rev3_archive_sha256
        if rev3_archive_root is not None and rev3_archive is not None:
            _fail(
                "CONFIGURATION_INVALID",
                "provide either rev3_archive_root or rev3_archive, not both",
            )

    def resolve_repository_artifact(
        self,
        path: str,
        expected_raw_sha256: DigestInput,
        schema_or_null: str | None,
    ) -> ResolvedArtifact:
        relative = _relative_path(path, "repository source path")
        candidate = (self._repo_root / Path(*relative.split("/"))).resolve()
        try:
            candidate.relative_to(self._repo_root)
        except ValueError:
            _fail(
                "REPOSITORY_PATH_ESCAPES",
                f"repository path escapes the configured root: {relative}",
            )
        if not candidate.is_file():
            _fail("REPOSITORY_SOURCE_MISSING", f"repository source is missing: {relative}")
        try:
            raw = candidate.read_bytes()
        except OSError as exc:
            _fail(
                "REPOSITORY_SOURCE_READ_FAILED", f"cannot read repository source {relative}: {exc}"
            )
        return _resolved_artifact("repository", relative, raw, expected_raw_sha256, schema_or_null)

    def _archive(self) -> Rev3ArchiveStore:
        if self._rev3_archive is None:
            root = self._rev3_archive_root
            if root is None:
                configured = os.environ.get(REV3_ARCHIVE_ENV_VAR)
                if not configured:
                    _blocked(
                        "REV3_ARCHIVE_SOURCE_UNAVAILABLE",
                        f"{REV3_ARCHIVE_ENV_VAR} is not configured",
                    )
                root = Path(configured)
            self._rev3_archive = Rev3ArchiveStore.from_root(
                root, self._expected_rev3_archive_sha256
            )
        return self._rev3_archive

    def resolve_rev3_member(
        self,
        member_path: str,
        expected_raw_sha256: DigestInput,
        schema_or_null: str | None = None,
    ) -> ResolvedArtifact:
        return self._archive().resolve_member(member_path, expected_raw_sha256, schema_or_null)

    def resolve_rev3_locator(
        self,
        locator: Locator,
        expected_raw_sha256: DigestInput,
        schema_or_null: str | None = None,
    ) -> ResolvedLocator:
        return self._archive().resolve_locator(locator, expected_raw_sha256, schema_or_null)

    def resolve_source_binding(self, binding: SourceBindingDigestV1) -> ResolvedArtifact:
        if binding.artifact_role == "rev3_source":
            return self.resolve_rev3_member(
                binding.path, binding.raw_sha256, binding.schema_or_null
            )
        return self.resolve_repository_artifact(
            binding.path, binding.raw_sha256, binding.schema_or_null
        )

    def _b1_snapshot(self, bindings: B1FinalArtifactBindingsV1) -> _B1FinalSnapshot:
        _b1_require_bindings(bindings)
        raw_citations = self.resolve_source_binding(bindings.citations)
        raw_closure = self.resolve_source_binding(bindings.closure)
        citations_artifact, citations_document = _b1_verified_json_document(
            raw_citations, "B1.Final citation artifact"
        )
        closure_artifact, closure_document = _b1_verified_json_document(
            raw_closure, "B1.Final citation closure"
        )
        _b1_verify_closure_binding(closure_document, citations_artifact)

        archive = self._archive()
        register_path = REV3_OFFICIAL_AUTHORITY_REGISTER_MEMBER
        register_artifact = self.resolve_rev3_member(
            register_path, archive.expected_member_sha256(register_path)
        )
        _, register_value = _verified_json_value(
            register_artifact, "REV3 official authority register"
        )
        register = _b1_register_index(register_value)

        if citations_document.get("schema") != B1_FINAL_CITATIONS_SCHEMA:
            _fail("B1_CITATIONS_BINDING_INVALID", "B1.Final citation schema is not V3")
        if citations_document.get("slice") != "B1_FINAL":
            _fail("B1_CITATIONS_BINDING_INVALID", "B1.Final citation slice marker is not V1")
        universe = _json_object(
            citations_document.get("input_universe"), "B1.Final citation input universe"
        )
        if universe.get("source_register") != register_path:
            _fail("B1_REGISTER_BINDING_MISMATCH", "B1.Final register path is not the REV3 register")
        if (
            _json_digest(universe.get("source_register_sha256"), "B1.Final register digest").hex()
            != register_artifact.raw_sha256
        ):
            _fail(
                "B1_REGISTER_BINDING_MISMATCH", "B1.Final register digest differs from REV3 bytes"
            )
        if (
            _json_digest(universe.get("archive_sha256"), "B1.Final archive digest").hex()
            != archive.archive_sha256
        ):
            _fail(
                "B1_ARCHIVE_BINDING_MISMATCH",
                "B1.Final archive binding differs from verified REV3 archive",
            )
        authority_ids = universe.get("authority_ids_in_order")
        if authority_ids != list(B1_FINAL_AUTHORITY_IDS) or universe.get("authority_count") != len(
            B1_FINAL_AUTHORITY_IDS
        ):
            _fail(
                "B1_AUTHORITY_UNIVERSE_INVALID",
                "B1.Final authority universe is not the closed V1 set",
            )

        raw_authorities = citations_document.get("authorities")
        if not isinstance(raw_authorities, list) or len(raw_authorities) != len(
            B1_FINAL_AUTHORITY_IDS
        ):
            _fail("B1_AUTHORITY_UNIVERSE_INVALID", "B1.Final authority record count is not seven")
        authorities: dict[str, Mapping[str, object]] = {}
        citations: dict[str, tuple[str, Mapping[str, object]]] = {}
        official_artifacts: dict[str, ResolvedArtifact | None] = {}
        for index, raw_authority in enumerate(cast(list[object], raw_authorities)):
            record = _json_object(raw_authority, f"B1.Final authority[{index}]")
            authority_id = _json_text(record.get("authority_id"), "B1.Final authority ID")
            if authority_id not in B1_FINAL_AUTHORITY_IDS:
                _fail("B1_AUTHORITY_INVALID", f"unknown B1.Final authority {authority_id!r}")
            if authority_id in authorities:
                _fail(
                    "B1_AUTHORITY_AMBIGUOUS",
                    f"B1.Final authority {authority_id!r} appears more than once",
                )
            register_entry = register.get(authority_id)
            if register_entry is None:
                _fail(
                    "B1_REGISTER_BINDING_MISMATCH",
                    f"authority {authority_id!r} is absent from the REV3 register",
                )
            identity = _b1_artifact_identity(
                record.get("artifact_identity"),
                f"B1.Final authority {authority_id}.artifact_identity",
            )
            status = _json_text(
                record.get("citation_status"), f"B1.Final authority {authority_id}.citation_status"
            )
            expected_path, expected_digest = _b1_expected_register_identity(
                authority_id, register_entry
            )
            actual_path = cast(str | None, identity.get("artifact_path"))
            actual_digest = cast(str | None, identity.get("artifact_sha256"))
            if actual_path != expected_path or actual_digest != expected_digest:
                _fail(
                    "B1_REGISTER_BINDING_MISMATCH",
                    f"authority {authority_id!r} differs from its REV3 register identity",
                )
            if status == "CITED":
                if actual_path is None or actual_digest is None:
                    _fail(
                        "B1_AUTHORITY_INVALID",
                        f"CITED authority {authority_id!r} lacks an official artifact",
                    )
                if register_entry.get("raw_artifact_available") is not True:
                    _fail(
                        "B1_REGISTER_BINDING_MISMATCH",
                        f"CITED authority {authority_id!r} is not register-available",
                    )
                official = self.resolve_rev3_member(actual_path, actual_digest)
                raw_citations = record.get("citations")
                if not isinstance(raw_citations, list) or not raw_citations:
                    _fail(
                        "B1_AUTHORITY_INVALID", f"CITED authority {authority_id!r} has no citations"
                    )
                for citation_index, raw_citation in enumerate(cast(list[object], raw_citations)):
                    citation = _b1_validate_citation(
                        raw_citation,
                        f"B1.Final authority {authority_id}.citation[{citation_index}]",
                    )
                    citation_id = cast(str, citation["citation_id"])
                    if citation_id in citations:
                        _fail(
                            "B1_CITATION_AMBIGUOUS",
                            f"B1.Final citation {citation_id!r} appears more than once",
                        )
                    citations[citation_id] = (
                        authority_id,
                        cast(Mapping[str, object], _frozen_mapping(citation)),
                    )
                official_artifacts[authority_id] = official
            elif status == "NOT_REQUIRED_WITH_PROOF":
                if actual_path is not None or actual_digest is not None:
                    _fail(
                        "B1_AUTHORITY_INVALID",
                        f"non-cited authority {authority_id!r} carries an official artifact",
                    )
                raw_citations = record.get("citations")
                if not isinstance(raw_citations, list) or raw_citations:
                    _fail(
                        "B1_AUTHORITY_INVALID",
                        f"non-cited authority {authority_id!r} carries citation nodes",
                    )
                official_artifacts[authority_id] = None
            else:
                _fail(
                    "B1_AUTHORITY_INVALID",
                    f"authority {authority_id!r} has an unsupported citation status",
                )
            normalized = dict(record)
            normalized["authority_id"] = authority_id
            normalized["artifact_identity"] = identity
            normalized["citations"] = record.get("citations")
            authorities[authority_id] = _frozen_mapping(normalized)

        if set(authorities) != set(B1_FINAL_AUTHORITY_IDS):
            _fail("B1_AUTHORITY_UNIVERSE_INVALID", "B1.Final authority set is incomplete")
        return _B1FinalSnapshot(
            citations_artifact=citations_artifact,
            closure_artifact=closure_artifact,
            authorities_by_id=authorities,
            citations_by_id=citations,
            official_artifacts_by_id=official_artifacts,
        )

    def resolve_b1_final_authority(
        self, authority_id: str, bindings: B1FinalArtifactBindingsV1
    ) -> ResolvedB1FinalAuthority:
        """Resolve one exact B1.Final authority record and source artifact."""

        requested_authority_id = _json_text(authority_id, "B1.Final authority ID")
        snapshot = self._b1_snapshot(bindings)
        record = snapshot.authorities_by_id.get(requested_authority_id)
        if record is None:
            _fail(
                "B1_AUTHORITY_NOT_FOUND",
                f"B1.Final authority {requested_authority_id!r} is not in the artifact",
            )
        identity = cast(Mapping[str, object], record["artifact_identity"])
        return ResolvedB1FinalAuthority(
            authority_id=requested_authority_id,
            artifact=snapshot.citations_artifact,
            source_binding=bindings.citations,
            record=record,
            official_artifact=snapshot.official_artifacts_by_id[requested_authority_id],
            artifact_identity=identity,
            _verification_token=_VERIFIED_B1_AUTHORITY_TOKEN,
        )

    def resolve_b1_final_citation(
        self,
        authority: ResolvedB1FinalAuthority,
        citation_id: str,
        bindings: B1FinalArtifactBindingsV1,
    ) -> ResolvedB1FinalCitation:
        """Resolve one exact citation under one resolver-verified authority."""

        if (
            not isinstance(authority, ResolvedB1FinalAuthority)
            or authority._verification_token is not _VERIFIED_B1_AUTHORITY_TOKEN
        ):
            _fail(
                "B1_AUTHORITY_UNVERIFIED",
                "B1.Final citation requires a resolver-verified authority",
            )
        _b1_require_bindings(bindings)
        if authority.source_binding != bindings.citations:
            _fail(
                "B1_AUTHORITY_BINDING_MISMATCH", "authority uses another B1.Final citation snapshot"
            )
        snapshot = self._b1_snapshot(bindings)
        persisted_authority = snapshot.authorities_by_id.get(authority.authority_id)
        if persisted_authority is None or dict(persisted_authority) != dict(authority.record):
            _fail("B1_AUTHORITY_BINDING_MISMATCH", "authority record changed after resolution")
        requested_citation_id = _json_text(citation_id, "B1.Final citation ID")
        resolved = snapshot.citations_by_id.get(requested_citation_id)
        if resolved is None:
            _fail(
                "B1_CITATION_NOT_FOUND",
                f"B1.Final citation {requested_citation_id!r} is not in the artifact",
            )
        owner, citation = resolved
        if owner != authority.authority_id:
            _fail(
                "B1_CITATION_AUTHORITY_MISMATCH",
                f"B1.Final citation {requested_citation_id!r} belongs to {owner!r}",
            )
        official = snapshot.official_artifacts_by_id[owner]
        if official is None:
            _fail(
                "B1_CITATION_ARTIFACT_UNAVAILABLE",
                "B1.Final citation has no official source artifact",
            )
        official_locator = self._resolve_b1_final_official_locator(
            owner, requested_citation_id, citation, official
        )
        return ResolvedB1FinalCitation(
            authority=authority,
            citation_id=requested_citation_id,
            citation=citation,
            official_locator=official_locator,
            _verification_token=_VERIFIED_B1_CITATION_TOKEN,
        )

    def resolve_b1_final_authority_citation(
        self,
        authority_id: str,
        citation_id: str,
        bindings: B1FinalArtifactBindingsV1,
    ) -> ResolvedB1FinalCitation:
        """Resolve an authority and one of its citations in one exact join."""

        authority = self.resolve_b1_final_authority(authority_id, bindings)
        return self.resolve_b1_final_citation(authority, citation_id, bindings)

    def _resolve_b1_final_official_locator(
        self,
        authority_id: str,
        citation_id: str,
        citation: Mapping[str, object],
        artifact: ResolvedArtifact,
    ) -> ResolvedB1FinalOfficialLocator:
        citation_kind = cast(str, citation["citation_kind"])
        locator = cast(Mapping[str, object], citation["artifact_local_locator"])
        if citation_kind == "CR_RULE_IDENTIFIER":
            identifier = cast(str, citation["rule_identifier"])
            match = _CR_RULE_IDENTIFIER_RE.fullmatch(identifier)
            if match is None:
                _fail(
                    "B1_CITATION_INVALID",
                    f"citation {citation_id!r} has a non-canonical CR identifier",
                )
            number = match.group(1) + (match.group(2) or "")
            try:
                lines = artifact.raw_bytes.decode("utf-8-sig").split("\n")
            except UnicodeDecodeError as exc:
                _fail("B1_OFFICIAL_ARTIFACT_INVALID", f"official CR artifact is not UTF-8: {exc}")
            line_number = cast(int, locator["line_number_1based"])
            if line_number > len(lines):
                _fail("B1_LOCATOR_UNRESOLVED", f"CR line {line_number} is outside {artifact.path}")
            stripped = lines[line_number - 1].strip()
            if number[-1:].isalpha():
                if not stripped.split()[0:1] or stripped.split()[0] != number:
                    _fail(
                        "B1_LOCATOR_BINDING_MISMATCH",
                        f"CR identifier {identifier!r} is not at line {line_number}",
                    )
            elif not stripped or stripped.split(maxsplit=1)[0].rstrip(".") != number:
                _fail(
                    "B1_LOCATOR_BINDING_MISMATCH",
                    f"CR identifier {identifier!r} is not at line {line_number}",
                )
            actual_digest = hashlib.sha256(stripped.encode("utf-8")).hexdigest()
            if actual_digest != cast(str, locator["heading_line_sha256"]):
                _fail(
                    "B1_LOCATOR_DIGEST_MISMATCH", f"CR heading digest differs at line {line_number}"
                )
            return ResolvedB1FinalOfficialLocator(
                artifact=artifact,
                citation_kind=citation_kind,
                locator=_frozen_mapping(dict(locator)),
                resolved_bytes=stripped.encode("utf-8"),
                _verification_token=_VERIFIED_B1_LOCATOR_TOKEN,
            )

        offset = cast(int, locator["byte_offset"])
        length = cast(int, locator["byte_length"])
        if offset + length > len(artifact.raw_bytes):
            _fail(
                "B1_LOCATOR_UNRESOLVED",
                f"byte fragment for {authority_id}/{citation_id} is out of range",
            )
        fragment = artifact.raw_bytes[offset : offset + length]
        if hashlib.sha256(fragment).hexdigest() != cast(str, locator["fragment_sha256"]):
            _fail(
                "B1_LOCATOR_DIGEST_MISMATCH",
                f"byte fragment digest differs for {authority_id}/{citation_id}",
            )
        occurrences = artifact.raw_bytes.count(fragment)
        if occurrences != 1:
            _fail(
                "B1_LOCATOR_AMBIGUOUS",
                f"byte fragment for {authority_id}/{citation_id} occurs {occurrences} times",
            )
        return ResolvedB1FinalOfficialLocator(
            artifact=artifact,
            citation_kind=citation_kind,
            locator=_frozen_mapping(dict(locator)),
            resolved_bytes=fragment,
            _verification_token=_VERIFIED_B1_LOCATOR_TOKEN,
        )

    def _b2_snapshot(self, bindings: B2ArtifactBindingsV1) -> _B2Snapshot:
        _b2_require_bindings(bindings)
        raw_catalog = self.resolve_source_binding(bindings.catalog)
        raw_classifications = self.resolve_source_binding(bindings.classifications)
        raw_closure = self.resolve_source_binding(bindings.closure)
        catalog_artifact, catalog_document = _b2_verified_json_document(
            raw_catalog, "B2 requirement family catalog"
        )
        classification_artifact, classification_document = _b2_verified_json_document(
            raw_classifications, "B2 card semantic classifications"
        )
        closure_artifact, closure_document = _b2_verified_json_document(
            raw_closure, "B2 classification closure"
        )
        _b2_verify_closure_bindings(self, closure_document, bindings)
        family_values = _b2_validate_catalog(catalog_document)
        if classification_document.get("schema") != B2_CLASSIFICATION_SCHEMA:
            _fail("B2_CLASSIFICATION_BINDING_INVALID", "B2 classification schema is not V1")
        if classification_document.get("source_package_sha256") != EXPECTED_REV3_ARCHIVE_SHA256:
            _fail("B2_CLASSIFICATION_BINDING_INVALID", "B2 classification package is not pinned")
        if classification_document.get("input_oracle_identity_count") != 402:
            _fail("B2_CLASSIFICATION_INVALID", "B2 classification identity count is not 402")
        raw_records = classification_document.get("classifications")
        if not isinstance(raw_records, list) or len(raw_records) != 402:
            _fail("B2_CLASSIFICATION_INVALID", "B2 classification record count is not 402")
        classifications: dict[str, Mapping[str, object]] = {}
        osi_order: list[str] = []
        for index, item in enumerate(cast(list[object], raw_records)):
            record = _b2_validate_classification_record(item, f"B2 classification[{index}]")
            osi = cast(str, record["oracle_semantic_identity"])
            if osi in classifications:
                _fail(
                    "B2_CLASSIFICATION_AMBIGUOUS",
                    f"B2 classification {osi!r} appears more than once",
                )
            classifications[osi] = _frozen_mapping(record)
            osi_order.append(osi)
        if osi_order != sorted(osi_order):
            _fail("B2_CLASSIFICATION_ORDER_INVALID", "B2 classifications are not sorted by OSI")
        for osi, classification in classifications.items():
            assignments = classification["requirement_assignments"]
            for assignment in cast(tuple[Mapping[str, object], ...], assignments):
                family_id = cast(str, assignment["requirement_family_id"])
                family_entry = family_values.get(family_id)
                if family_entry is None:
                    _fail(
                        "B2_ASSIGNMENT_FAMILY_MISMATCH",
                        f"B2 classification {osi!r} references unknown family {family_id!r}",
                    )
                family = family_entry[0]
                if family["status"] != "ACTIVE" or family["terminal_assignable"] is not True:
                    _fail(
                        "B2_ASSIGNMENT_FAMILY_MISMATCH",
                        f"B2 classification {osi!r} references non-active family {family_id!r}",
                    )
        frozen_families = {
            family_id: _frozen_mapping(family) for family_id, (family, _) in family_values.items()
        }
        frozen_boundaries = {
            family_id: cast(Mapping[str, str], _frozen_mapping(boundary))
            for family_id, (_, boundary) in family_values.items()
        }
        return _B2Snapshot(
            catalog_artifact=catalog_artifact,
            classification_artifact=classification_artifact,
            closure_artifact=closure_artifact,
            families_by_id=frozen_families,
            family_boundaries_by_id=frozen_boundaries,
            classifications_by_osi=classifications,
        )

    def resolve_b2_requirement_family(
        self, family_id: str, bindings: B2ArtifactBindingsV1
    ) -> ResolvedB2RequirementFamily:
        """Resolve one exact family record without interpreting its meaning."""

        requested_family_id = _json_text(family_id, "B2 family ID")
        snapshot = self._b2_snapshot(bindings)
        record = snapshot.families_by_id.get(requested_family_id)
        if record is None:
            _fail(
                "B2_FAMILY_NOT_FOUND",
                f"B2 family {requested_family_id!r} is not in the catalog",
            )
        boundary = snapshot.family_boundaries_by_id[requested_family_id]
        return ResolvedB2RequirementFamily(
            family_id=requested_family_id,
            artifact=snapshot.catalog_artifact,
            source_binding=bindings.catalog,
            record=record,
            boundary_fields=boundary,
            _verification_token=_VERIFIED_B2_FAMILY_TOKEN,
        )

    def resolve_b2_classification(
        self,
        oracle_semantic_identity: str,
        classification_identity: Mapping[str, object],
        bindings: B2ArtifactBindingsV1,
    ) -> ResolvedB2Classification:
        """Resolve one exact card classification and its persisted identity."""

        requested_osi = _json_text(oracle_semantic_identity, "B2 Oracle semantic identity")
        if _LOWERCASE_UUID_RE.fullmatch(requested_osi) is None:
            _fail(
                "B2_CLASSIFICATION_BINDING_INVALID",
                "B2 Oracle semantic identity is not a lowercase UUID",
            )
        requested_identity = _b2_digest_reference(
            classification_identity,
            "B2 classification identity",
            B2_CLASSIFICATION_DOMAIN,
            B2_CLASSIFICATION_INPUT_SCHEMA,
        )
        snapshot = self._b2_snapshot(bindings)
        record = snapshot.classifications_by_osi.get(requested_osi)
        if record is None:
            _fail(
                "B2_CLASSIFICATION_NOT_FOUND",
                f"B2 classification {requested_osi!r} is not in the artifact",
            )
        persisted_identity = cast(Mapping[str, object], record["classification_identity"])
        if dict(persisted_identity) != requested_identity:
            _fail(
                "B2_CLASSIFICATION_IDENTITY_MISMATCH",
                f"B2 classification {requested_osi!r} identity differs from the artifact",
            )
        return ResolvedB2Classification(
            oracle_semantic_identity=requested_osi,
            artifact=snapshot.classification_artifact,
            source_binding=bindings.classifications,
            record=record,
            classification_identity=_frozen_mapping(requested_identity),
            _verification_token=_VERIFIED_B2_CLASSIFICATION_TOKEN,
        )

    def resolve_b2_assignment(
        self,
        classification: ResolvedB2Classification,
        family_id: str,
        bindings: B2ArtifactBindingsV1,
    ) -> ResolvedB2Assignment:
        """Resolve one exact classification-to-family assignment edge."""

        if (
            not isinstance(classification, ResolvedB2Classification)
            or classification._verification_token is not _VERIFIED_B2_CLASSIFICATION_TOKEN
        ):
            _fail(
                "B2_CLASSIFICATION_UNVERIFIED",
                "B2 assignment requires a resolver-verified classification",
            )
        _b2_require_bindings(bindings)
        if classification.source_binding != bindings.classifications:
            _fail(
                "B2_CLASSIFICATION_BINDING_MISMATCH",
                "classification uses another B2 artifact binding",
            )
        snapshot = self._b2_snapshot(bindings)
        persisted = snapshot.classifications_by_osi.get(classification.oracle_semantic_identity)
        if persisted is None:
            _fail("B2_CLASSIFICATION_NOT_FOUND", "classification is no longer in the B2 artifact")
        if dict(cast(Mapping[str, object], persisted["classification_identity"])) != dict(
            classification.classification_identity
        ):
            _fail(
                "B2_CLASSIFICATION_IDENTITY_MISMATCH",
                "classification identity changed after resolution",
            )
        requested_family_id = _json_text(family_id, "B2 assignment family ID")
        raw_assignments = cast(
            tuple[Mapping[str, object], ...], persisted["requirement_assignments"]
        )
        matches = [
            (index, assignment)
            for index, assignment in enumerate(raw_assignments)
            if assignment["requirement_family_id"] == requested_family_id
        ]
        if not matches:
            _fail(
                "B2_ASSIGNMENT_NOT_FOUND",
                f"B2 classification {classification.oracle_semantic_identity!r} has no "
                f"{requested_family_id!r} assignment",
            )
        if len(matches) != 1:
            _fail(
                "B2_ASSIGNMENT_AMBIGUOUS",
                f"B2 classification {classification.oracle_semantic_identity!r} has multiple "
                f"{requested_family_id!r} assignments",
            )
        _, assignment = matches[0]
        family_record = snapshot.families_by_id.get(requested_family_id)
        if family_record is None:
            _fail(
                "B2_ASSIGNMENT_FAMILY_MISMATCH", "B2 assignment family is absent from the catalog"
            )
        if family_record["status"] != "ACTIVE" or family_record["terminal_assignable"] is not True:
            _fail(
                "B2_ASSIGNMENT_FAMILY_MISMATCH", "B2 assignment family is not terminally assignable"
            )
        family = ResolvedB2RequirementFamily(
            family_id=requested_family_id,
            artifact=snapshot.catalog_artifact,
            source_binding=bindings.catalog,
            record=family_record,
            boundary_fields=snapshot.family_boundaries_by_id[requested_family_id],
            _verification_token=_VERIFIED_B2_FAMILY_TOKEN,
        )
        return ResolvedB2Assignment(
            classification=classification,
            family=family,
            assignment=_frozen_mapping(dict(assignment)),
            _verification_token=_VERIFIED_B2_ASSIGNMENT_TOKEN,
        )

    def resolve_b2_boundary(
        self,
        family: ResolvedB2RequirementFamily,
        boundary_ref: B2BoundaryReferenceV1,
        assignment: ResolvedB2Assignment | None = None,
    ) -> ResolvedB2Boundary:
        """Resolve an exact B2 boundary reference and optional assignment join."""

        if (
            not isinstance(family, ResolvedB2RequirementFamily)
            or family._verification_token is not _VERIFIED_B2_FAMILY_TOKEN
        ):
            _fail("B2_FAMILY_UNVERIFIED", "B2 boundary requires a resolver-verified family")
        if not isinstance(boundary_ref, B2BoundaryReferenceV1):
            _fail("B2_BOUNDARY_INVALID", "B2 boundary requires B2BoundaryReferenceV1")
        if boundary_ref.family_id != family.family_id:
            _fail(
                "B2_BOUNDARY_BINDING_MISMATCH",
                "B2 boundary family ID differs from the catalog record",
            )
        definition = _json_text(
            boundary_ref.precise_semantic_definition,
            "B2 precise semantic definition",
        )
        if definition != family.record["precise_semantic_definition"]:
            _fail(
                "B2_BOUNDARY_BINDING_MISMATCH",
                "B2 boundary definition differs from the catalog record",
            )
        if assignment is not None:
            if (
                not isinstance(assignment, ResolvedB2Assignment)
                or assignment._verification_token is not _VERIFIED_B2_ASSIGNMENT_TOKEN
            ):
                _fail("B2_ASSIGNMENT_UNVERIFIED", "B2 boundary assignment is not resolver-verified")
            if assignment.family.family_id != family.family_id:
                _fail(
                    "B2_BOUNDARY_BINDING_MISMATCH",
                    "B2 boundary is joined to another assignment family",
                )
            if assignment.family.source_binding != family.source_binding:
                _fail(
                    "B2_BOUNDARY_BINDING_MISMATCH",
                    "B2 boundary and assignment use different catalog snapshots",
                )
            if family.record["status"] != "ACTIVE":
                _fail("B2_BOUNDARY_BINDING_MISMATCH", "a card-derived B2 boundary must be active")
        return ResolvedB2Boundary(
            family=family,
            boundary_ref=boundary_ref,
            assignment=assignment,
            _verification_token=_VERIFIED_B2_BOUNDARY_TOKEN,
        )

    def _candidate_universe(self, binding: SourceBindingDigestV1) -> _CandidateUniverseIndex:
        if binding.artifact_role != "candidate_universe":
            _fail(
                "CANDIDATE_UNIVERSE_BINDING_INVALID",
                "candidate resolution requires a candidate_universe binding",
            )
        if (
            binding.path != CANDIDATE_UNIVERSE_PATH
            or binding.schema_or_null != CANDIDATE_UNIVERSE_SCHEMA
        ):
            _fail(
                "CANDIDATE_UNIVERSE_BINDING_INVALID",
                "candidate resolution requires the admitted candidate-universe path and schema",
            )
        artifact = self.resolve_source_binding(binding)
        universe = _json_object(artifact.json_value, "candidate universe")
        inputs = _candidate_universe_input_bindings(universe.get("input_bindings"))
        declared_model = cast(Mapping[str, str], inputs["declared_model"])
        model_binding = SourceBindingDigestV1(
            artifact_role="declared_model",
            path=declared_model["path"],
            schema_or_null=DECLARED_MODEL_SCHEMA,
            raw_sha256=bytes.fromhex(declared_model["raw_sha256"]),
        )
        model_artifact = self.resolve_source_binding(model_binding)
        model_vocabularies = _model_vocabularies(model_artifact.json_value)
        return _candidate_universe_index(artifact, model_vocabularies)

    def resolve_candidate(
        self,
        candidate_id: str,
        candidate_identity: Mapping[str, object],
        candidate_universe_binding: SourceBindingDigestV1,
    ) -> ResolvedCandidate:
        """Resolve one exact candidate from the verified C candidate ledger."""

        requested_candidate_id = _json_text(candidate_id, "candidate_id")
        requested_identity = _candidate_identity_reference(candidate_identity, "candidate_identity")
        index = self._candidate_universe(candidate_universe_binding)
        record = index.candidates_by_id.get(requested_candidate_id)
        if record is None:
            _fail(
                "CANDIDATE_BINDING_MISMATCH",
                f"candidate {requested_candidate_id!r} is not in the candidate universe",
            )
        persisted_identity = cast(Mapping[str, object], record["candidate_identity"])
        if dict(persisted_identity) != requested_identity:
            _fail(
                "CANDIDATE_IDENTITY_MISMATCH",
                f"candidate {requested_candidate_id!r} identity differs from the ledger",
            )
        source_binding = index.candidate_bindings_by_id[requested_candidate_id]
        return ResolvedCandidate(
            candidate_id=requested_candidate_id,
            candidate_identity=_frozen_mapping(requested_identity),
            candidate_universe=index.artifact,
            candidate_universe_binding=candidate_universe_binding,
            candidate_record=record,
            source_binding=source_binding,
            _verification_token=_VERIFIED_CANDIDATE_TOKEN,
        )

    def _verify_declared_card_trigger_join(
        self, archive: Rev3ArchiveStore, oracle_semantic_identity: str
    ) -> None:
        resolution_path = REV3_RESOLUTION_MEMBER
        resolution_artifact = self.resolve_rev3_member(
            resolution_path, archive.expected_member_sha256(resolution_path)
        )
        resolutions = _parse_keyed_csv(
            resolution_artifact.raw_bytes,
            resolution_path,
            frozenset(
                {
                    "oracle_semantic_identity",
                    "deck_row_id",
                    "source_row_id",
                    "source_snapshot_file",
                    "source_line_number",
                    "oracle_source_record_id",
                }
            ),
        )
        selected = [
            row
            for row in resolutions
            if row["oracle_semantic_identity"] == oracle_semantic_identity
        ]
        if len(selected) != 1 or any(
            not selected[0][column]
            for column in (
                "deck_row_id",
                "source_row_id",
                "source_snapshot_file",
                "source_line_number",
                "oracle_source_record_id",
            )
        ):
            _fail(
                "REV3_TRIGGER_JOIN_INVALID",
                f"OSI {oracle_semantic_identity!r} does not resolve exactly once",
            )
        source_record_id = selected[0]["oracle_source_record_id"]
        index_path = REV3_SOURCE_INDEX_MEMBER
        index_artifact = self.resolve_rev3_member(
            index_path, archive.expected_member_sha256(index_path)
        )
        index_rows = _parse_keyed_csv(
            index_artifact.raw_bytes,
            index_path,
            frozenset({"oracle_semantic_identity", "source_record_id"}),
        )
        matches = [
            row
            for row in index_rows
            if row["oracle_semantic_identity"] == oracle_semantic_identity
            and row["source_record_id"] == source_record_id
        ]
        if len(matches) != 1:
            _fail(
                "REV3_TRIGGER_JOIN_INVALID",
                f"OSI {oracle_semantic_identity!r} raw source join is not unique",
            )

    def _verify_candidate_normalization(
        self,
        candidate: Mapping[str, object],
        source_artifact: ResolvedArtifact,
        rows: list[list[str]],
        row_ordinal: int,
        archive: Rev3ArchiveStore,
    ) -> None:
        row = dict(zip(REV3_SOURCE_COLUMNS, rows[row_ordinal], strict=True))
        normalized = _normalized_candidate_fields(row)
        if normalized["relation"] == "declared_card_trigger":
            self._verify_declared_card_trigger_join(archive, cast(str, row["pair_id"]))
        if candidate["candidate_id"] != normalized["candidate_id"]:
            _fail(
                "REV3_CANDIDATE_NORMALIZATION_MISMATCH",
                f"{source_artifact.path} row {row_ordinal} has a different candidate_id",
            )
        if candidate["scope"] != normalized["scope"]:
            _fail(
                "REV3_CANDIDATE_NORMALIZATION_MISMATCH",
                f"{source_artifact.path} row {row_ordinal} has a different scope",
            )
        if candidate["relation"] != normalized["relation"]:
            _fail(
                "REV3_CANDIDATE_NORMALIZATION_MISMATCH",
                f"{source_artifact.path} row {row_ordinal} has a different relation",
            )
        candidate_participants = list(
            cast(tuple[Mapping[str, str], ...], candidate["participant_refs"])
        )
        if candidate_participants != normalized["participant_refs"]:
            _fail(
                "REV3_CANDIDATE_NORMALIZATION_MISMATCH",
                f"{source_artifact.path} row {row_ordinal} has different participants",
            )
        candidate_supporting_ids = list(
            cast(tuple[str, ...], candidate["supporting_requirement_ids"])
        )
        if candidate_supporting_ids != normalized["supporting_requirement_ids"]:
            _fail(
                "REV3_CANDIDATE_NORMALIZATION_MISMATCH",
                f"{source_artifact.path} row {row_ordinal} has different supporting IDs",
            )

    def resolve_source_instance(
        self,
        candidate: ResolvedCandidate,
        source_instance_id: str,
    ) -> ResolvedSourceInstance:
        """Resolve one exact REV3 source instance owned by ``candidate``."""

        if (
            not isinstance(candidate, ResolvedCandidate)
            or candidate._verification_token is not _VERIFIED_CANDIDATE_TOKEN
        ):
            _fail(
                "CANDIDATE_UNVERIFIED",
                "source-instance resolution requires a resolver-verified candidate",
            )
        requested_instance_id = _json_text(source_instance_id, "source_instance_id")
        index = self._candidate_universe(candidate.candidate_universe_binding)
        persisted_candidate = index.candidates_by_id.get(candidate.candidate_id)
        if persisted_candidate is None:
            _fail(
                "CANDIDATE_BINDING_MISMATCH",
                f"candidate {candidate.candidate_id!r} disappeared from the candidate universe",
            )
        if dict(cast(Mapping[str, object], persisted_candidate["candidate_identity"])) != dict(
            candidate.candidate_identity
        ):
            _fail(
                "CANDIDATE_IDENTITY_MISMATCH",
                f"candidate {candidate.candidate_id!r} identity changed after resolution",
            )
        instance = index.instances_by_id.get(requested_instance_id)
        if instance is None:
            _fail(
                "SOURCE_INSTANCE_BINDING_MISMATCH",
                f"source instance {requested_instance_id!r} is not in the candidate universe",
            )
        if instance["candidate_id"] != candidate.candidate_id:
            _fail(
                "SOURCE_INSTANCE_CANDIDATE_MISMATCH",
                f"source instance {requested_instance_id!r} belongs to another candidate",
            )
        candidate_binding = cast(Mapping[str, object], persisted_candidate["source_binding"])
        instance_binding = cast(Mapping[str, object], instance["source_binding"])
        if _source_binding_key(candidate_binding) != _source_binding_key(instance_binding):
            _fail(
                "CANDIDATE_SOURCE_BINDING_MISMATCH",
                f"source instance {requested_instance_id!r} does not copy the candidate binding",
            )
        rev3_input = index.rev3_input_binding
        if (
            rev3_input["archive_member"] != candidate_binding["archive_member"]
            or rev3_input["archive_member_sha256"] != candidate_binding["archive_member_sha256"]
        ):
            _fail(
                "CANDIDATE_SOURCE_BINDING_MISMATCH",
                "candidate binding differs from the candidate-universe REV3 input binding",
            )
        archive = self._archive()
        if archive.archive_sha256 != rev3_input["source_package_sha256"]:
            _fail(
                "REV3_ARCHIVE_BINDING_MISMATCH",
                "configured REV3 archive differs from the candidate-universe package binding",
            )
        source_artifact = self.resolve_rev3_member(
            cast(str, instance_binding["archive_member"]),
            cast(str, instance_binding["archive_member_sha256"]),
        )
        rows = _parse_rev3_rows(source_artifact.raw_bytes, source_artifact.path)
        row_ordinal = cast(int, instance_binding["row_ordinal"])
        if row_ordinal >= len(rows):
            _fail(
                "REV3_ROW_ORDINAL_OUT_OF_RANGE",
                f"REV3 row ordinal {row_ordinal} is outside {len(rows)} rows",
            )
        expected_values = list(cast(tuple[str, ...], instance_binding["source_values"]))
        actual_values = rows[row_ordinal]
        if actual_values != expected_values:
            _fail(
                "REV3_SOURCE_ROW_MISMATCH",
                f"REV3 row {row_ordinal} does not match the persisted source values",
            )
        self._verify_candidate_normalization(
            persisted_candidate,
            source_artifact,
            rows,
            row_ordinal,
            archive,
        )
        return ResolvedSourceInstance(
            candidate=candidate,
            source_instance_id=requested_instance_id,
            source_instance_record=instance,
            source_binding=instance_binding,
            source_artifact=source_artifact,
        )

    def resolve_candidate_source_instance(
        self,
        candidate_id: str,
        candidate_identity: Mapping[str, object],
        source_instance_id: str,
        candidate_universe_binding: SourceBindingDigestV1,
    ) -> ResolvedSourceInstance:
        """Resolve and join one candidate with its exact REV3 source instance."""

        candidate = self.resolve_candidate(
            candidate_id, candidate_identity, candidate_universe_binding
        )
        return self.resolve_source_instance(candidate, source_instance_id)

    def resolve_locator(self, artifact: ResolvedArtifact, locator: Locator) -> ResolvedLocator:
        artifact = self._reverify_artifact(artifact)
        kind, payload = _locator(locator)
        if kind == "whole_artifact":
            value = artifact.json_value if artifact.json_value is not None else artifact.raw_bytes
            return ResolvedLocator(artifact, locator, value)
        if kind == "json_pointer":
            if artifact.json_value is None:
                _fail("LOCATOR_INVALID", "json_pointer requires a JSON artifact schema")
            return ResolvedLocator(artifact, locator, _json_pointer(artifact.json_value, payload))
        if kind == "event_id" and isinstance(payload, str):
            if artifact.source_kind != "repository":
                _fail("LOCATOR_INVALID", "event_id locators require a repository event leaf")
            try:
                reference = ReviewEventRefV1(
                    path=artifact.path,
                    raw_sha256=bytes.fromhex(artifact.raw_sha256),
                    event_id=payload,
                )
            except ValueError as exc:
                _fail("ACCEPTANCE_EVENT_ID_MISMATCH", str(exc))
            resolved = self.resolve_acceptance_event_leaf(reference)
            return ResolvedLocator(artifact, locator, cast(dict[str, object], resolved.json_value))
        _fail("LOCATOR_INVALID", "archive_member locators require resolve_rev3_locator")

    def resolve_acceptance_event_leaf(self, reference: ReviewEventRefV1) -> ResolvedArtifact:
        if _EVENT_LEAF_PATH_RE.fullmatch(reference.path) is None:
            _fail("ACCEPTANCE_EVENT_PATH_INVALID", "acceptance event path is not a V1 leaf path")
        artifact = self.resolve_repository_artifact(
            reference.path, reference.raw_sha256, ACCEPTANCE_EVENT_SCHEMA_V1
        )
        event = _json_object(artifact.json_value, "acceptance event leaf")
        if event.get("event_id") != reference.event_id:
            _fail(
                "ACCEPTANCE_EVENT_ID_MISMATCH",
                "acceptance event locator does not match the leaf event_id",
            )
        expected_path = (
            "sources/m2_5/authorities/review_acceptance_events/v1/"
            + reference.event_id.removeprefix("ae.v1/")
            + ".json"
        )
        if reference.path != expected_path:
            _fail("ACCEPTANCE_EVENT_PATH_INVALID", "acceptance event path is not bound to event_id")
        self._verify_acceptance_event_identity(event, reference.event_id)
        return artifact

    def resolve_reviewer_roster_leaf(self, reference: ReviewerRosterRefV1) -> ResolvedArtifact:
        artifact = self.resolve_repository_artifact(
            reference.path, reference.raw_sha256, REVIEWER_ROSTER_SCHEMA_V1
        )
        roster = _json_object(artifact.json_value, "reviewer roster leaf")
        _exact_keys(roster, {"schema", "reviewers"}, "reviewer roster leaf")
        raw_reviewers = roster.get("reviewers")
        if not isinstance(raw_reviewers, list):
            _fail("REVIEWER_ROSTER_INVALID", "reviewer roster reviewers must be an array")
        reviewers: list[ReviewerV1] = []
        for raw_reviewer in raw_reviewers:
            record = _json_object(raw_reviewer, "reviewer roster entry")
            _exact_keys(record, {"reviewer_id", "roles"}, "reviewer roster entry")
            roles = record.get("roles")
            if not isinstance(roles, list) or any(not isinstance(role, str) for role in roles):
                _fail("REVIEWER_ROSTER_INVALID", "reviewer roster roles must be text")
            try:
                reviewers.append(
                    ReviewerV1(
                        _json_text(record.get("reviewer_id"), "reviewer ID"),
                        tuple(cast(list[str], roles)),
                    )
                )
            except (TypeError, ValueError) as exc:
                _fail("REVIEWER_ROSTER_INVALID", str(exc))
        try:
            ReviewerRosterV1(tuple(reviewers))
        except (TypeError, ValueError) as exc:
            _fail("REVIEWER_ROSTER_INVALID", str(exc))
        return artifact

    def _verify_acceptance_event_identity(
        self,
        event: Mapping[str, object],
        expected_event_id: str,
    ) -> None:
        _exact_keys(
            event,
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
            "acceptance event leaf",
        )
        if event.get("decision") != "human_accepted":
            _fail("ACCEPTANCE_EVENT_INVALID", "acceptance event decision is not human_accepted")
        if event.get("checklist_id") != "interaction-authority-review-checklist.v1":
            _fail("ACCEPTANCE_EVENT_INVALID", "acceptance event checklist is not the V1 contract")
        roster_record = _json_object(event.get("reviewer_roster_ref"), "reviewer roster reference")
        _exact_keys(roster_record, {"path", "schema", "raw_sha256"}, "reviewer roster reference")
        try:
            roster_ref = ReviewerRosterRefV1(
                path=_json_text(roster_record.get("path"), "reviewer roster path"),
                schema=_json_text(roster_record.get("schema"), "reviewer roster schema"),
                raw_sha256=_json_digest(roster_record.get("raw_sha256"), "reviewer roster digest"),
            )
            raw_bindings = event.get("reviewer_role_bindings")
            if not isinstance(raw_bindings, list):
                raise ValueError("reviewer role bindings must be an array")
            role_binding_records = []
            for item in raw_bindings:
                record = _json_object(item, "reviewer role binding")
                _exact_keys(record, {"reviewer_id", "roles"}, "reviewer role binding")
                role_binding_records.append(record)
            role_bindings = tuple(
                ReviewerRoleBindingV1(
                    reviewer_id=_json_text(record.get("reviewer_id"), "reviewer ID"),
                    roles=tuple(cast(list[str], record.get("roles"))),
                )
                for record in role_binding_records
            )
            raw_sources = event.get("source_binding_digests")
            if not isinstance(raw_sources, list):
                raise ValueError("source binding digests must be an array")
            source_binding_records = []
            for item in raw_sources:
                record = _json_object(item, "source binding")
                _exact_keys(
                    record,
                    {"artifact_role", "path", "schema_or_null", "raw_sha256"},
                    "source binding",
                )
                source_binding_records.append(record)
            source_bindings = tuple(
                SourceBindingDigestV1(
                    artifact_role=_json_text(record.get("artifact_role"), "artifact role"),
                    path=_json_text(record.get("path"), "source binding path"),
                    schema_or_null=cast(str | None, record.get("schema_or_null")),
                    raw_sha256=_json_digest(record.get("raw_sha256"), "source binding digest"),
                )
                for record in source_binding_records
            )
            raw_evidence = event.get("review_evidence_refs")
            if not isinstance(raw_evidence, list):
                raise ValueError("review evidence references must be an array")
            evidence_records = []
            for item in raw_evidence:
                record = _json_object(item, "acceptance evidence")
                _exact_keys(record, {"path", "raw_sha256", "locator"}, "acceptance evidence")
                evidence_records.append(record)
            evidence = tuple(
                AcceptanceEvidenceRefV1(
                    path=_json_text(record.get("path"), "acceptance evidence path"),
                    raw_sha256=_json_digest(record.get("raw_sha256"), "acceptance evidence digest"),
                    locator=self._wire_locator(record.get("locator")),
                )
                for record in evidence_records
            )
            candidate = ReviewAcceptanceEventInputV1(
                subject_kind=AcceptanceSubjectKind(
                    _json_text(event.get("subject_kind"), "subject kind")
                ),
                subject_payload_digest=_json_digest(
                    event.get("subject_payload_digest"), "subject payload digest"
                ),
                reviewer_roster_ref=roster_ref,
                reviewer_role_bindings=role_bindings,
                review_mode=ReviewMode(_json_text(event.get("review_mode"), "review mode")),
                source_binding_digests=source_bindings,
                review_evidence_refs=evidence,
            )
            actual_event_id = candidate.identity().as_text()
        except ResolutionError as exc:
            _fail("ACCEPTANCE_EVENT_INVALID", exc.message)
        except (TypeError, ValueError) as exc:
            _fail("ACCEPTANCE_EVENT_INVALID", str(exc))
        if actual_event_id != expected_event_id:
            _fail(
                "ACCEPTANCE_EVENT_ID_MISMATCH",
                f"acceptance event bytes derive {actual_event_id}, expected {expected_event_id}",
            )

    @staticmethod
    def _wire_locator(value: object) -> Locator:
        record = _json_object(value, "locator")
        kind = _json_text(record.get("kind"), "locator kind")
        if kind == "whole_artifact":
            expected_keys = {"kind"}
        elif kind in {"json_pointer", "archive_member", "event_id"}:
            expected_keys = {"kind", "value"}
        else:
            _fail("LOCATOR_INVALID", f"unknown locator variant {kind!r}")
        _exact_keys(record, expected_keys, "locator")
        payload = record.get("value") if "value" in record else None
        return _locator((kind, cast(str | int | None, payload)))

    @staticmethod
    def _reverify_artifact(artifact: ResolvedArtifact) -> ResolvedArtifact:
        if artifact._verification_token is not _VERIFIED_ARTIFACT_TOKEN:
            _fail("ARTIFACT_UNVERIFIED", "locator resolution requires a resolver-verified artifact")
        return _resolved_artifact(
            artifact.source_kind,
            artifact.path,
            artifact.raw_bytes,
            artifact.raw_sha256,
            artifact.schema_or_null,
        )


__all__ = [
    "ACCEPTANCE_EVENT_SCHEMA_V1",
    "B1_FINAL_CITATIONS_PATH",
    "B1_FINAL_CITATIONS_SCHEMA",
    "B1_FINAL_CLOSURE_PATH",
    "B1_FINAL_CLOSURE_SCHEMA",
    "B2_CATALOG_PATH",
    "B2_CATALOG_SCHEMA",
    "B2_CLASSIFICATION_PATH",
    "B2_CLASSIFICATION_SCHEMA",
    "B2_CLOSURE_PATH",
    "B2_CLOSURE_SCHEMA",
    "CANDIDATE_UNIVERSE_PATH",
    "CANDIDATE_UNIVERSE_SCHEMA",
    "EXPECTED_REV3_ARCHIVE_SHA256",
    "REV3_CENSUS_MEMBER",
    "REV3_OFFICIAL_AUTHORITY_REGISTER_MEMBER",
    "REV3_SOURCE_COLUMNS",
    "AuthoritySourceResolver",
    "B1FinalArtifactBindingsV1",
    "B2ArtifactBindingsV1",
    "B2BoundaryReferenceV1",
    "ResolutionError",
    "ResolutionStatus",
    "ResolvedB1FinalAuthority",
    "ResolvedB1FinalCitation",
    "ResolvedB1FinalOfficialLocator",
    "ResolvedB2Assignment",
    "ResolvedB2Boundary",
    "ResolvedB2Classification",
    "ResolvedB2RequirementFamily",
    "ResolvedCandidate",
    "ResolvedSourceInstance",
    "Rev3ArchiveStore",
]
