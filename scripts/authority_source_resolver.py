"""Rules-neutral, read-only source and locator resolution for M2.5.C.

This module verifies bytes and source identity before parsing or interpreting
them. It resolves repository artifacts and the externally configured REV3
package only; it does not classify candidates, derive C semantics, or accept
authority records.
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
from mtgml.persistence import encode_canonical

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


def _candidate_identity(value: object, label: str) -> dict[str, object]:
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


def _candidate_record(
    value: object, label: str, participant_kinds: frozenset[str]
) -> dict[str, object]:
    record = _json_object(value, label)
    _exact_keys(record, set(CANDIDATE_RECORD_KEYS), label)
    candidate_id = _json_text(record.get("candidate_id"), f"{label}.candidate_id")
    identity = _candidate_identity(record.get("candidate_identity"), f"{label}.candidate_identity")
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
        requested_identity = _candidate_identity(candidate_identity, "candidate_identity")
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
    "CANDIDATE_UNIVERSE_PATH",
    "CANDIDATE_UNIVERSE_SCHEMA",
    "EXPECTED_REV3_ARCHIVE_SHA256",
    "REV3_CENSUS_MEMBER",
    "REV3_SOURCE_COLUMNS",
    "AuthoritySourceResolver",
    "ResolutionError",
    "ResolutionStatus",
    "ResolvedCandidate",
    "ResolvedSourceInstance",
    "Rev3ArchiveStore",
]
