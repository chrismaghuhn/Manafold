"""Fail-closed validation of the M2.5.C review-authority graph.

This module validates the persisted V1 authority structures and delegates all
source access to ``AuthoritySourceResolver``.  It never derives a C
classification, evaluates Magic rules, or treats source presence as semantic
authority.
"""

from __future__ import annotations

import io
import json
import re
import sys
import zipfile
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType
from typing import Final, TypeAlias, cast

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import (
    B1_FINAL_CITATIONS_SCHEMA,
    B1_FINAL_CLOSURE_SCHEMA,
    B2_CATALOG_SCHEMA,
    B2_CLASSIFICATION_SCHEMA,
    B2_CLOSURE_SCHEMA,
    CANDIDATE_UNIVERSE_SCHEMA,
    DECLARED_MODEL_SCHEMA,
    AuthoritySourceResolver,
    B1FinalArtifactBindingsV1,
    B2ArtifactBindingsV1,
    B2BoundaryReferenceV1,
    ResolutionError,
    ResolutionStatus,
    ResolvedSourceInstance,
)
from mtgml.authority import (
    ACCEPTANCE_EVENT_SCHEMA_V1,
    AUTHORITY_SCHEMA_V1,
    REVIEWER_ROSTER_SCHEMA_V1,
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKind,
    AcceptanceSubjectPayloadV1,
    AuthorityContractError,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    B2FamilyRefV1,
    EvidenceRefV1,
    RecordKind,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewerRosterV1,
    ReviewerV1,
    ReviewEventRefV1,
    SourceBindingDigestV1,
    SupersessionReason,
    SupersessionRecordV1,
    compute_authority_identity,
)
from mtgml.persistence import PersistenceValue, encode_canonical

JsonObject: TypeAlias = dict[str, object]
CborValue: TypeAlias = PersistenceValue

_HEX64_RE: Final = re.compile(r"^[0-9a-f]{64}$")
_EVENT_ID_RE: Final = re.compile(r"^ae\.v1/[0-9a-f]{64}$")
_ROSTER_PATH_RE: Final = re.compile(
    r"^sources/m2_5/authorities/reviewer_rosters/v1/[0-9a-f]{64}\.json$"
)
_EVENT_PATH_RE: Final = re.compile(
    r"^sources/m2_5/authorities/review_acceptance_events/v1/[0-9a-f]{64}\.json$"
)
_CANDIDATE_IDENTITY_DOMAIN: Final = "manafold.m2.5.c.candidate-identity.v1"
_CANDIDATE_IDENTITY_SCHEMA: Final = "manafold.m2.5.c.candidate-identity-input.v1"
_CANDIDATE_IDENTITY_KEYS: Final = frozenset(
    {
        "envelope_id",
        "algorithm_id",
        "semantic_domain",
        "payload_codec_id",
        "input_schema_id",
        "digest_hex",
    }
)
_SOURCE_BINDING_KIND_BY_ROLE: Final = {
    "declared_model": "model",
    "rev3_source": "rev3",
    "b2_catalog": "b2",
    "b2_classifications": "b2",
    "b2_closure": "b2",
    "b1_final_citations": "b1_final",
    "b1_final_closure": "b1_final",
    "candidate_universe": "c_candidate",
    "acceptance_event_leaf": "acceptance_event",
    "reviewer_roster_leaf": "reviewer_roster",
}
_STATIC_ROLE_BY_PATH: Final = {
    "sources/m2_5/closures/C/declared_interaction_model.v2.json": "declared_model",
    "sources/m2_5/closures/B2/requirement_family_catalog.v1.json": "b2_catalog",
    "sources/m2_5/closures/B2/card_semantic_classifications.v1.json": "b2_classifications",
    "sources/m2_5/closures/B2/classification_closure.v1.json": "b2_closure",
    "sources/m2_5/closures/B1/official_authority_citations.v3.json": "b1_final_citations",
    "sources/m2_5/closures/B1/official_authority_citation_closure.v2.json": "b1_final_closure",
    "sources/m2_5/closures/C/interaction_candidate_universe.v2.json": "candidate_universe",
}
_RAW_REV3_PATHS: Final = frozenset(
    {
        "derived/Pair_Interaction_Census_REV3.csv",
        "inputs/deck_row_source_resolution_REV3.csv",
        "source/raw/source_record_index_REV3.csv",
        "source/raw/oracle_cards_selected_REV3.jsonl",
    }
)
_SOURCE_CONTEXT_KEYS: Final = (
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
_SOURCE_RELATION_SHAPES: Final = {
    "declared_card_trigger": ("unary", "none"),
    "directional_binary": ("binary", "directed"),
    "unordered_binary": ("binary", "symmetric"),
}
_RECORD_KIND_TO_RECORD_ID_KIND: Final = {
    RecordKind.RELATION_THEOREM_RECORD: AuthorityIdentityKind.RELATION_THEOREM_RECORD,
    RecordKind.RELATION_APPLICATION_RECORD: AuthorityIdentityKind.RELATION_APPLICATION_RECORD,
    RecordKind.DOMAIN_THEOREM_RECORD: AuthorityIdentityKind.DOMAIN_THEOREM_RECORD,
    RecordKind.DOMAIN_APPLICATION_RECORD: AuthorityIdentityKind.DOMAIN_APPLICATION_RECORD,
    RecordKind.CONTEXT_THEOREM_RECORD: AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
    RecordKind.CONTEXT_APPLICATION_RECORD: AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD,
}
_RECORD_KIND_TO_SEMANTIC_KIND: Final = {
    RecordKind.RELATION_THEOREM_RECORD: AuthorityIdentityKind.RELATION_THEOREM,
    RecordKind.RELATION_APPLICATION_RECORD: AuthorityIdentityKind.RELATION_APPLICATION,
    RecordKind.DOMAIN_THEOREM_RECORD: AuthorityIdentityKind.DOMAIN_THEOREM,
    RecordKind.DOMAIN_APPLICATION_RECORD: AuthorityIdentityKind.DOMAIN_APPLICATION,
    RecordKind.CONTEXT_THEOREM_RECORD: AuthorityIdentityKind.CONTEXT_THEOREM,
    RecordKind.CONTEXT_APPLICATION_RECORD: AuthorityIdentityKind.CONTEXT_APPLICATION,
}
_RECORD_KIND_TO_SUPERSESSION_KIND: Final = {
    RecordKind.RELATION_THEOREM_RECORD: AuthorityIdentityKind.RELATION_SUPERSESSION,
    RecordKind.RELATION_APPLICATION_RECORD: AuthorityIdentityKind.RELATION_SUPERSESSION,
    RecordKind.DOMAIN_THEOREM_RECORD: AuthorityIdentityKind.DOMAIN_SUPERSESSION,
    RecordKind.DOMAIN_APPLICATION_RECORD: AuthorityIdentityKind.DOMAIN_SUPERSESSION,
    RecordKind.CONTEXT_THEOREM_RECORD: AuthorityIdentityKind.CONTEXT_SUPERSESSION,
    RecordKind.CONTEXT_APPLICATION_RECORD: AuthorityIdentityKind.CONTEXT_SUPERSESSION,
}
_REQUIRED_ROLE_BY_RECORD_KIND: Final = frozenset(
    {"architecture_maintainer", "rules_authority_maintainer"}
)


@dataclass(frozen=True)
class AuthorityValidationResult:
    """Immutable result for one fully validated authority graph."""

    valid: bool
    counts: Mapping[str, int]


@dataclass(frozen=True)
class _ValidatedRecord:
    kind: RecordKind
    record: Mapping[str, object]
    semantic_id: AuthorityIdentityV1 | None
    record_id: AuthorityIdentityV1
    event_ref: ReviewEventRefV1


@dataclass(frozen=True)
class _SourceRegistry:
    by_key: Mapping[bytes, SourceBindingDigestV1]

    def require(self, binding: SourceBindingDigestV1, label: str) -> None:
        if encode_canonical(binding.to_cbor()) not in self.by_key:
            _fail("SOURCE_BINDING_CLOSURE_MISMATCH", f"{label} is absent from source_bindings")


def _fail(code: str, message: str) -> None:
    raise ResolutionError(ResolutionStatus.FAIL, code, message)


def _object(value: object, label: str) -> JsonObject:
    if not isinstance(value, Mapping):
        _fail("AUTHORITY_SHAPE_INVALID", f"{label} must be an object")
    return cast(JsonObject, value)


def _exact(value: object, keys: set[str] | frozenset[str], label: str) -> JsonObject:
    record = _object(value, label)
    if set(record) != set(keys):
        _fail("AUTHORITY_SHAPE_INVALID", f"{label} fields are not exactly {sorted(keys)!r}")
    return record


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        _fail("AUTHORITY_VALUE_INVALID", f"{label} must be non-empty text")
    return value


def _digest(value: object, label: str) -> bytes:
    text = _text(value, label)
    if _HEX64_RE.fullmatch(text) is None:
        _fail("AUTHORITY_VALUE_INVALID", f"{label} must be lowercase SHA-256 hex")
    return bytes.fromhex(text)


def _uint32(value: object, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**32 - 1:
        _fail("AUTHORITY_VALUE_INVALID", f"{label} must be a u32")
    return value


def _array(value: object, label: str, length: int | None = None) -> list[object]:
    if not isinstance(value, list):
        _fail("AUTHORITY_SHAPE_INVALID", f"{label} must be an array")
    result = cast(list[object], value)
    if length is not None and len(result) != length:
        _fail("AUTHORITY_SHAPE_INVALID", f"{label} must contain exactly {length} values")
    return result


def _cbor_value(value: object, label: str) -> CborValue:
    if value is None or isinstance(value, bool | int | str):
        return cast(CborValue, value)
    if isinstance(value, list):
        return [_cbor_value(item, f"{label}[{index}]") for index, item in enumerate(value)]
    _fail("AUTHORITY_VALUE_INVALID", f"{label} is not a JSON-representable CBOR value")


def _wire_locator(value: object, *, acceptance: bool, label: str) -> tuple[str, str | None]:
    record = _object(value, label)
    kind = _text(record.get("kind"), f"{label}.kind")
    if kind == "whole_artifact":
        if set(record) != {"kind"}:
            _fail("AUTHORITY_SHAPE_INVALID", f"{label} whole_artifact has extra fields")
        return kind, None
    if kind in {"json_pointer", "archive_member", "event_id"}:
        if set(record) != {"kind", "value"}:
            _fail("AUTHORITY_SHAPE_INVALID", f"{label} fields are not closed")
        payload = _text(record.get("value"), f"{label}.value")
        if kind == "event_id" and acceptance:
            _fail("AUTHORITY_VALUE_INVALID", "acceptance evidence cannot use event_id")
        return kind, payload
    _fail("AUTHORITY_VALUE_INVALID", f"{label}.kind is not a closed locator variant")


def _strict_json_pointer(value: object, pointer: str, label: str) -> object:
    if pointer and not pointer.startswith("/"):
        _fail("EVIDENCE_LOCATOR_INVALID", f"{label} must begin with '/'")
    current = value
    if not pointer:
        return current
    for raw_token in pointer[1:].split("/"):
        token_parts: list[str] = []
        index = 0
        while index < len(raw_token):
            if raw_token[index] != "~":
                token_parts.append(raw_token[index])
                index += 1
                continue
            if index + 1 >= len(raw_token) or raw_token[index + 1] not in "01":
                _fail("EVIDENCE_LOCATOR_INVALID", f"{label} contains an invalid escape")
            token_parts.append("~" if raw_token[index + 1] == "0" else "/")
            index += 2
        token = "".join(token_parts)
        if isinstance(current, dict):
            if token not in current:
                _fail("EVIDENCE_LOCATOR_UNRESOLVED", f"{label} names a missing object key")
            current = current[token]
        elif isinstance(current, list):
            if token == "-" or not re.fullmatch(r"0|[1-9][0-9]*", token):
                _fail("EVIDENCE_LOCATOR_INVALID", f"{label} names an invalid array index")
            position = int(token)
            if position >= len(current):
                _fail("EVIDENCE_LOCATOR_UNRESOLVED", f"{label} array index is out of range")
            current = current[position]
        else:
            _fail("EVIDENCE_LOCATOR_UNRESOLVED", f"{label} traverses a scalar value")
    return current


def _identity_ref(value: object, kind: AuthorityIdentityKind, label: str) -> AuthorityIdentityV1:
    record = _exact(
        value,
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
    zero = AuthorityIdentityV1(kind, bytes(32))
    expected = {
        "envelope_id": "mtgml.digest-envelope.v1",
        "algorithm_id": "sha-256",
        "semantic_domain": zero.semantic_domain,
        "payload_codec_id": "mtgml.canonical-cbor.v1",
        "input_schema_id": zero.input_schema_id,
    }
    for key, expected_value in expected.items():
        if record.get(key) != expected_value:
            _fail("AUTHORITY_IDENTITY_INVALID", f"{label}.{key} does not match its identity kind")
    return AuthorityIdentityV1(kind, _digest(record.get("digest_hex"), f"{label}.digest_hex"))


def _candidate_identity(value: object, label: str) -> tuple[JsonObject, list[CborValue]]:
    record = _exact(value, _CANDIDATE_IDENTITY_KEYS, label)
    expected = {
        "envelope_id": "mtgml.digest-envelope.v1",
        "algorithm_id": "sha-256",
        "semantic_domain": _CANDIDATE_IDENTITY_DOMAIN,
        "payload_codec_id": "mtgml.canonical-cbor.v1",
        "input_schema_id": _CANDIDATE_IDENTITY_SCHEMA,
    }
    for key, expected_value in expected.items():
        if record.get(key) != expected_value:
            _fail("CANDIDATE_IDENTITY_INVALID", f"{label}.{key} is not the V1 value")
    digest_bytes = _digest(record.get("digest_hex"), f"{label}.digest_hex")
    return record, [
        cast(CborValue, record["envelope_id"]),
        cast(CborValue, record["algorithm_id"]),
        cast(CborValue, record["semantic_domain"]),
        cast(CborValue, record["payload_codec_id"]),
        cast(CborValue, record["input_schema_id"]),
        digest_bytes,
    ]


def _participant(value: object, label: str) -> list[CborValue]:
    record = _exact(value, {"position", "role", "participant_kind", "semantic_ref"}, label)
    return [
        _uint32(record.get("position"), f"{label}.position"),
        _text(record.get("role"), f"{label}.role"),
        _text(record.get("participant_kind"), f"{label}.participant_kind"),
        _text(record.get("semantic_ref"), f"{label}.semantic_ref"),
    ]


def _subject(value: object, label: str) -> list[CborValue]:
    record = _exact(
        value,
        {"arity", "relation", "directionality", "participant_roles", "host_relationship"},
        label,
    )
    participants = [
        _participant(item, f"{label}.participant_roles[{index}]")
        for index, item in enumerate(_array(record.get("participant_roles"), "participant roles"))
    ]
    return [
        _text(record.get("arity"), f"{label}.arity"),
        _text(record.get("relation"), f"{label}.relation"),
        _text(record.get("directionality"), f"{label}.directionality"),
        participants,
        _text(record.get("host_relationship"), f"{label}.host_relationship"),
    ]


def _candidate_shape(value: object, label: str) -> list[CborValue]:
    record = _exact(
        value,
        {"scope", "relation", "arity", "directionality", "participant_count"},
        label,
    )
    return [
        _text(record.get("scope"), f"{label}.scope"),
        _text(record.get("relation"), f"{label}.relation"),
        _text(record.get("arity"), f"{label}.arity"),
        _text(record.get("directionality"), f"{label}.directionality"),
        _uint32(record.get("participant_count"), f"{label}.participant_count"),
    ]


def _model_boundary_locator(value: object, label: str) -> list[CborValue]:
    record = _object(value, label)
    kind = _text(record.get("kind"), f"{label}.kind")
    if kind == "coverage_scope":
        if set(record) != {"kind"}:
            _fail("AUTHORITY_SHAPE_INVALID", f"{label} coverage_scope has extra fields")
        return [kind, None]
    if kind == "excluded_claim":
        if set(record) != {"kind", "index"}:
            _fail("AUTHORITY_SHAPE_INVALID", f"{label} excluded_claim fields are not closed")
        return [kind, _uint32(record.get("index"), f"{label}.index")]
    _fail("AUTHORITY_VALUE_INVALID", f"{label}.kind is not closed")


def _model_boundary_ref(value: object, label: str) -> list[CborValue]:
    record = _exact(value, {"path", "schema", "raw_sha256", "locator"}, label)
    return [
        _text(record.get("path"), f"{label}.path"),
        _text(record.get("schema"), f"{label}.schema"),
        _digest(record.get("raw_sha256"), f"{label}.raw_sha256"),
        _model_boundary_locator(record.get("locator"), f"{label}.locator"),
    ]


def _b2_boundary_ref(value: object, label: str) -> list[CborValue]:
    record = _exact(value, {"family_id", "precise_semantic_definition"}, label)
    return [
        _text(record.get("family_id"), f"{label}.family_id"),
        _text(record.get("precise_semantic_definition"), f"{label}.precise_semantic_definition"),
    ]


def _b1_citation_ref(value: object, label: str) -> list[CborValue]:
    record = _exact(value, {"authority_id", "citation_id"}, label)
    return [
        _text(record.get("authority_id"), f"{label}.authority_id"),
        _text(record.get("citation_id"), f"{label}.citation_id"),
    ]


def _evidence_ref(value: object, label: str) -> tuple[EvidenceRefV1, list[CborValue]]:
    record = _exact(value, {"authority_kind", "path", "locator", "raw_sha256"}, label)
    locator = _wire_locator(record.get("locator"), acceptance=False, label=f"{label}.locator")
    reference = EvidenceRefV1(
        _text(record.get("authority_kind"), f"{label}.authority_kind"),
        _text(record.get("path"), f"{label}.path"),
        locator,
        _digest(record.get("raw_sha256"), f"{label}.raw_sha256"),
    )
    return reference, reference.to_cbor()


def _evidence_refs(
    value: object, label: str, *, nonempty: bool = False
) -> tuple[tuple[EvidenceRefV1, ...], list[CborValue]]:
    raw = _array(value, label)
    if nonempty and not raw:
        _fail("EVIDENCE_MISSING", f"{label} must be non-empty")
    parsed = [_evidence_ref(item, f"{label}[{index}]") for index, item in enumerate(raw)]
    encoded = [encode_canonical(item[1]) for item in parsed]
    if encoded != sorted(encoded) or len(set(encoded)) != len(encoded):
        _fail("NONCANONICAL_EVIDENCE", f"{label} must be sorted and duplicate-free")
    return tuple(item[0] for item in parsed), [item[1] for item in parsed]


def _participant_arrays(value: object, label: str) -> list[list[CborValue]]:
    return [
        _participant(item, f"{label}[{index}]") for index, item in enumerate(_array(value, label))
    ]


def _class_projection(value: object, label: str) -> list[CborValue]:
    record = _exact(
        value,
        {
            "arity",
            "directionality",
            "participant_roles",
            "host_relationship",
            "context_dimensions",
            "temporal_semantics",
            "b2_family_refs",
            "b2_boundary_refs",
            "b1_final_citation_refs",
        },
        label,
    )
    return [
        _text(record.get("arity"), f"{label}.arity"),
        _text(record.get("directionality"), f"{label}.directionality"),
        _participant_arrays(record.get("participant_roles"), f"{label}.participant_roles"),
        _text(record.get("host_relationship"), f"{label}.host_relationship"),
        [
            _text(item, f"{label}.context_dimensions[{index}]")
            for index, item in enumerate(
                _array(record.get("context_dimensions"), "context dimensions")
            )
        ],
        [
            _text(item, f"{label}.temporal_semantics[{index}]")
            for index, item in enumerate(
                _array(record.get("temporal_semantics"), "temporal semantics")
            )
        ],
        [
            _b2_family_ref(item, f"{label}.b2_family_refs[{index}]")
            for index, item in enumerate(_array(record.get("b2_family_refs"), "B2 family refs"))
        ],
        [
            _b2_boundary_ref(item, f"{label}.b2_boundary_refs[{index}]")
            for index, item in enumerate(_array(record.get("b2_boundary_refs"), "B2 boundary refs"))
        ],
        [
            _b1_citation_ref(item, f"{label}.b1_final_citation_refs[{index}]")
            for index, item in enumerate(
                _array(record.get("b1_final_citation_refs"), "B1 citation refs")
            )
        ],
    ]


def _b2_family_ref(value: object, label: str) -> list[CborValue]:
    record = _exact(value, {"family_id", "lifecycle", "assignment_role"}, label)
    ref = B2FamilyRefV1(
        _text(record.get("family_id"), f"{label}.family_id"),
        _text(record.get("lifecycle"), f"{label}.lifecycle"),
        _text(record.get("assignment_role"), f"{label}.assignment_role"),
    )
    return ref.to_cbor()


def _precondition(value: object, label: str) -> tuple[str, list[CborValue], list[CborValue]]:
    record = _exact(value, {"precondition_id", "precondition_kind", "payload"}, label)
    identifier = _text(record.get("precondition_id"), f"{label}.precondition_id")
    kind = _text(record.get("precondition_kind"), f"{label}.precondition_kind")
    payload = _cbor_value(record.get("payload"), f"{label}.payload")
    return identifier, [kind, payload], [identifier, [kind, payload]]


def _preconditions(value: object, label: str) -> tuple[tuple[JsonObject, ...], list[CborValue]]:
    raw = _array(value, label)
    parsed: list[JsonObject] = []
    arrays: list[list[CborValue]] = []
    identifiers: list[str] = []
    for index, item in enumerate(raw):
        record = _exact(
            item, {"precondition_id", "precondition_kind", "payload"}, f"{label}[{index}]"
        )
        _, _, array = _precondition(record, f"{label}[{index}]")
        parsed.append(record)
        identifiers.append(cast(str, array[0]))
        arrays.append(array)
    ordering = [encode_canonical(identifier) for identifier in identifiers]
    if ordering != sorted(ordering) or len(set(identifiers)) != len(identifiers):
        _fail("NONCANONICAL_PRECONDITIONS", f"{label} must be sorted and duplicate-free")
    return tuple(parsed), arrays


def _causal_edge(value: object, label: str) -> list[CborValue]:
    record = _exact(
        value,
        {
            "ordinal",
            "from_role_position",
            "operation",
            "through_boundary_refs",
            "event_or_effect_role_position",
            "to_role_position",
            "b1_final_citation_refs",
        },
        label,
    )
    event_position = record.get("event_or_effect_role_position")
    destination = record.get("to_role_position")
    return [
        _uint32(record.get("ordinal"), f"{label}.ordinal"),
        _uint32(record.get("from_role_position"), f"{label}.from_role_position"),
        _text(record.get("operation"), f"{label}.operation"),
        [
            _b2_boundary_ref(item, f"{label}.through_boundary_refs[{index}]")
            for index, item in enumerate(
                _array(record.get("through_boundary_refs"), "through boundary refs")
            )
        ],
        None if event_position is None else _uint32(event_position, f"{label}.event position"),
        None if destination is None else _uint32(destination, f"{label}.destination position"),
        [
            _b1_citation_ref(item, f"{label}.b1_final_citation_refs[{index}]")
            for index, item in enumerate(
                _array(record.get("b1_final_citation_refs"), "causal B1 refs")
            )
        ],
    ]


def _positive_boundary_fact(value: object, label: str) -> list[CborValue]:
    record = _object(value, label)
    kind = _text(record.get("kind"), f"{label}.kind")
    if kind == "b2_boundary":
        record = _exact(
            record,
            {"kind", "family_id", "lifecycle", "assignment_role", "precise_semantic_definition"},
            label,
        )
        return [
            kind,
            [
                _text(record.get("family_id"), f"{label}.family_id"),
                _text(record.get("lifecycle"), f"{label}.lifecycle"),
                _text(record.get("assignment_role"), f"{label}.assignment_role"),
                _text(record.get("precise_semantic_definition"), f"{label}.definition"),
            ],
        ]
    if kind in {"rev3_locator", "b2_locator"}:
        record = _exact(record, {"kind", "path", "raw_sha256", "locator"}, label)
        return [
            kind,
            [
                _text(record.get("path"), f"{label}.path"),
                _digest(record.get("raw_sha256"), f"{label}.raw_sha256"),
                list(
                    _wire_locator(record.get("locator"), acceptance=False, label=f"{label}.locator")
                ),
            ],
        ]
    if kind == "b1_citation":
        record = _exact(record, {"kind", "citation"}, label)
        return [kind, _b1_citation_ref(record.get("citation"), f"{label}.citation")]
    if kind == "context_slot":
        record = _exact(record, {"kind", "slot_kind", "slot_name", "observed_value"}, label)
        return [
            kind,
            [
                _text(record.get("slot_kind"), f"{label}.slot_kind"),
                _text(record.get("slot_name"), f"{label}.slot_name"),
                _cbor_value(record.get("observed_value"), f"{label}.observed_value"),
            ],
        ]
    if kind == "model_boundary":
        record = _exact(
            record, {"kind", "model_id", "model_version", "model_boundary_locator"}, label
        )
        return [
            kind,
            [
                _text(record.get("model_id"), f"{label}.model_id"),
                _text(record.get("model_version"), f"{label}.model_version"),
                _model_boundary_locator(record.get("model_boundary_locator"), f"{label}.locator"),
            ],
        ]
    _fail("AUTHORITY_VALUE_INVALID", f"{label}.kind is not a closed positive fact")


def _proof_payload(value: object, label: str) -> list[CborValue]:
    record = _object(value, label)
    kind = _text(record.get("kind"), f"{label}.kind")
    if kind == "positive_interaction":
        record = _exact(
            record,
            {"kind", "causal_chain", "required_relation_channels", "class_projection_template"},
            label,
        )
        chain = [
            _causal_edge(item, f"{label}.causal_chain[{index}]")
            for index, item in enumerate(_array(record.get("causal_chain"), "causal chain"))
        ]
        projection = record.get("class_projection_template")
        return [
            kind,
            [
                chain,
                [
                    _text(item, f"{label}.required_relation_channels[{index}]")
                    for index, item in enumerate(
                        _array(record.get("required_relation_channels"), "required channels")
                    )
                ],
                None
                if projection is None
                else _class_projection(projection, f"{label}.class projection"),
            ],
        ]
    if kind == "positive_separation":
        record = _exact(record, {"kind", "separation_kind", "separation_obligations"}, label)
        obligations = []
        for index, item in enumerate(
            _array(record.get("separation_obligations"), "separation obligations")
        ):
            obligation = _exact(
                item, {"channel", "required_conclusion"}, f"{label}.obligation[{index}]"
            )
            obligations.append(
                [
                    _text(obligation.get("channel"), "separation channel"),
                    _text(obligation.get("required_conclusion"), "separation conclusion"),
                ]
            )
        return [kind, [_text(record.get("separation_kind"), "separation kind"), obligations]]
    if kind == "model_bound_scope":
        record = _exact(
            record,
            {
                "kind",
                "model_boundary_ref",
                "reason_code",
                "observed_candidate_shape",
                "positive_boundary_evidence_refs",
            },
            label,
        )
        _, evidence = _evidence_refs(
            record.get("positive_boundary_evidence_refs"),
            f"{label}.positive_boundary_evidence_refs",
            nonempty=True,
        )
        return [
            kind,
            [
                _model_boundary_ref(
                    record.get("model_boundary_ref"), f"{label}.model_boundary_ref"
                ),
                _text(record.get("reason_code"), f"{label}.reason_code"),
                _candidate_shape(
                    record.get("observed_candidate_shape"), f"{label}.observed_candidate_shape"
                ),
                evidence,
            ],
        ]
    _fail("AUTHORITY_VALUE_INVALID", f"{label}.kind is not a closed proof variant")


def _relation_binding(value: object, label: str) -> list[CborValue]:
    record = _exact(
        value,
        {"scope", "relation", "directionality", "host_relationship", "participant_bindings"},
        label,
    )
    return [
        _text(record.get("scope"), f"{label}.scope"),
        _text(record.get("relation"), f"{label}.relation"),
        _text(record.get("directionality"), f"{label}.directionality"),
        _text(record.get("host_relationship"), f"{label}.host_relationship"),
        _participant_arrays(record.get("participant_bindings"), f"{label}.participant_bindings"),
    ]


def _candidate_universe_binding(value: object, label: str) -> list[CborValue]:
    record = _exact(value, {"path", "schema", "raw_sha256"}, label)
    return [
        _text(record.get("path"), f"{label}.path"),
        _text(record.get("schema"), f"{label}.schema"),
        _digest(record.get("raw_sha256"), f"{label}.raw_sha256"),
    ]


def _domain_binding(value: object, label: str) -> list[CborValue]:
    record = _exact(value, {"review_domain", "applicability"}, label)
    return [
        _text(record.get("review_domain"), f"{label}.review_domain"),
        _text(record.get("applicability"), f"{label}.applicability"),
    ]


def _context_binding(value: object, label: str) -> list[CborValue]:
    record = _exact(
        value,
        {"arity", "directionality", "participant_roles", "host_relationship"},
        label,
    )
    return [
        _text(record.get("arity"), f"{label}.arity"),
        _text(record.get("directionality"), f"{label}.directionality"),
        _participant_arrays(record.get("participant_roles"), f"{label}.participant_roles"),
        _text(record.get("host_relationship"), f"{label}.host_relationship"),
    ]


def _precondition_attestations(
    value: object, label: str
) -> tuple[tuple[JsonObject, ...], list[CborValue]]:
    raw = _array(value, label)
    records: list[JsonObject] = []
    arrays: list[list[CborValue]] = []
    identifiers: list[str] = []
    for index, item in enumerate(raw):
        record = _exact(
            item,
            {"precondition_id", "observed_value", "evidence_refs", "equivalence_rationale"},
            f"{label}[{index}]",
        )
        identifier = _text(record.get("precondition_id"), "precondition attestation ID")
        evidence, evidence_arrays = _evidence_refs(
            record.get("evidence_refs"), f"{label}[{index}].evidence_refs", nonempty=True
        )
        del evidence
        records.append(record)
        identifiers.append(identifier)
        arrays.append(
            [
                identifier,
                _cbor_value(record.get("observed_value"), "observed precondition value"),
                evidence_arrays,
                _text(record.get("equivalence_rationale"), "precondition rationale"),
            ]
        )
    ordering = [encode_canonical(identifier) for identifier in identifiers]
    if ordering != sorted(ordering) or len(set(identifiers)) != len(identifiers):
        _fail(
            "NONCANONICAL_PRECONDITION_ATTESTATIONS", f"{label} must be sorted and duplicate-free"
        )
    return tuple(records), arrays


def _class_projection_equivalence(value: object, label: str) -> list[CborValue]:
    record = _exact(
        value,
        {
            "theorem_projection",
            "member_projection",
            "equal_positions",
            "semantic_claim_relation",
            "evidence_refs",
            "rationale",
        },
        label,
    )
    claim = _exact(
        record.get("semantic_claim_relation"),
        {"kind", "theorem_semantic_digest"},
        f"{label}.semantic_claim_relation",
    )
    _, evidence = _evidence_refs(
        record.get("evidence_refs"), f"{label}.evidence_refs", nonempty=True
    )
    return [
        _class_projection(record.get("theorem_projection"), f"{label}.theorem_projection"),
        _class_projection(record.get("member_projection"), f"{label}.member_projection"),
        [
            _text(item, f"{label}.equal_positions[{index}]")
            for index, item in enumerate(_array(record.get("equal_positions"), "equal positions"))
        ],
        [
            _text(claim.get("kind"), f"{label}.claim.kind"),
            _digest(claim.get("theorem_semantic_digest"), f"{label}.claim.digest"),
        ],
        evidence,
        _text(record.get("rationale"), f"{label}.rationale"),
    ]


def _channel_coverage(value: object, label: str) -> list[CborValue]:
    record = _exact(
        value,
        {
            "channel",
            "coverage",
            "positive_boundary_facts",
            "source_evidence_refs",
            "b1_final_citation_refs",
            "rationale",
        },
        label,
    )
    _, evidence = _evidence_refs(
        record.get("source_evidence_refs"), f"{label}.source_evidence_refs", nonempty=True
    )
    return [
        _text(record.get("channel"), f"{label}.channel"),
        _text(record.get("coverage"), f"{label}.coverage"),
        [
            _positive_boundary_fact(item, f"{label}.positive_boundary_facts[{index}]")
            for index, item in enumerate(
                _array(record.get("positive_boundary_facts"), "boundary facts")
            )
        ],
        evidence,
        [
            _b1_citation_ref(item, f"{label}.b1_final_citation_refs[{index}]")
            for index, item in enumerate(
                _array(record.get("b1_final_citation_refs"), "coverage B1 refs")
            )
        ],
        _text(record.get("rationale"), f"{label}.rationale"),
    ]


def _scope_attestation(value: object, label: str) -> list[CborValue]:
    record = _exact(
        value,
        {
            "model_id",
            "model_version",
            "model_boundary_ref",
            "reason_code",
            "observed_candidate_shape",
            "positive_boundary_evidence_refs",
        },
        label,
    )
    _, evidence = _evidence_refs(
        record.get("positive_boundary_evidence_refs"),
        f"{label}.positive_boundary_evidence_refs",
        nonempty=True,
    )
    return [
        _text(record.get("model_id"), f"{label}.model_id"),
        _text(record.get("model_version"), f"{label}.model_version"),
        _model_boundary_ref(record.get("model_boundary_ref"), f"{label}.model_boundary_ref"),
        _text(record.get("reason_code"), f"{label}.reason_code"),
        _candidate_shape(
            record.get("observed_candidate_shape"), f"{label}.observed_candidate_shape"
        ),
        evidence,
    ]


def _member_proof(value: object, label: str) -> list[CborValue]:
    record = _object(value, label)
    kind = _text(record.get("kind"), f"{label}.kind")
    if kind == "positive_interaction":
        record = _exact(
            record, {"kind", "causal_chain_ordinals", "class_projection_equivalence"}, label
        )
        projection = record.get("class_projection_equivalence")
        return [
            kind,
            [
                [
                    _uint32(item, f"{label}.causal_chain_ordinals[{index}]")
                    for index, item in enumerate(
                        _array(record.get("causal_chain_ordinals"), "ordinals")
                    )
                ],
                None
                if projection is None
                else _class_projection_equivalence(
                    projection, f"{label}.class_projection_equivalence"
                ),
            ],
        ]
    if kind == "positive_separation":
        record = _exact(record, {"kind", "channel_coverages"}, label)
        return [
            kind,
            [
                _channel_coverage(item, f"{label}.channel_coverages[{index}]")
                for index, item in enumerate(
                    _array(record.get("channel_coverages"), "channel coverages")
                )
            ],
        ]
    if kind == "model_bound_scope":
        record = _exact(record, {"kind", "scope_boundary_attestation"}, label)
        return [
            kind,
            [
                _scope_attestation(
                    record.get("scope_boundary_attestation"), f"{label}.scope_boundary_attestation"
                )
            ],
        ]
    _fail("AUTHORITY_VALUE_INVALID", f"{label}.kind is not a closed member-proof variant")


def _relation_member(value: object, label: str) -> tuple[JsonObject, list[CborValue]]:
    record = _exact(
        value,
        {
            "candidate_id",
            "candidate_identity",
            "source_instance_id",
            "candidate_universe_binding",
            "relation_binding",
            "precondition_attestations",
            "member_evidence_refs",
            "member_proof_attestation",
        },
        label,
    )
    _, candidate_identity = _candidate_identity(
        record.get("candidate_identity"), f"{label}.candidate_identity"
    )
    _, preconditions = _precondition_attestations(
        record.get("precondition_attestations"), f"{label}.precondition_attestations"
    )
    _, member_evidence = _evidence_refs(
        record.get("member_evidence_refs"), f"{label}.member_evidence_refs", nonempty=True
    )
    return record, [
        _text(record.get("candidate_id"), f"{label}.candidate_id"),
        candidate_identity,
        _text(record.get("source_instance_id"), f"{label}.source_instance_id"),
        _candidate_universe_binding(
            record.get("candidate_universe_binding"), f"{label}.candidate_universe_binding"
        ),
        _relation_binding(record.get("relation_binding"), f"{label}.relation_binding"),
        preconditions,
        member_evidence,
        _member_proof(record.get("member_proof_attestation"), f"{label}.member_proof_attestation"),
    ]


def _domain_criterion(value: object, label: str) -> list[CborValue]:
    record = _object(value, label)
    kind = _text(record.get("kind"), f"{label}.kind")
    if kind in {"channel_implicated", "channel_excluded"}:
        record = _exact(record, {"kind", "channel", "positive_boundary_fact"}, label)
        return [
            kind,
            [
                _text(record.get("channel"), f"{label}.channel"),
                _positive_boundary_fact(
                    record.get("positive_boundary_fact"), f"{label}.positive_boundary_fact"
                ),
            ],
        ]
    if kind == "rule_domain_required":
        record = _exact(record, {"kind", "citation", "covered_boundary_fields"}, label)
        return [
            kind,
            [
                _b1_citation_ref(record.get("citation"), f"{label}.citation"),
                [
                    _text(item, "boundary field")
                    for item in _array(record.get("covered_boundary_fields"), "boundary fields")
                ],
            ],
        ]
    if kind == "rule_domain_excluded":
        record = _exact(record, {"kind", "excluded_domain_id", "positive_boundary_fact"}, label)
        excluded = _object(record.get("excluded_domain_id"), f"{label}.excluded_domain_id")
        excluded_kind = _text(excluded.get("kind"), f"{label}.excluded_domain_id.kind")
        if excluded_kind == "b1_final_citation":
            excluded = _exact(excluded, {"kind", "citation"}, f"{label}.excluded_domain_id")
            excluded_value: list[CborValue] = [
                excluded_kind,
                _b1_citation_ref(excluded.get("citation"), "excluded citation"),
            ]
        else:
            excluded = _exact(excluded, {"kind", "value"}, f"{label}.excluded_domain_id")
            excluded_value = [excluded_kind, _text(excluded.get("value"), "excluded domain value")]
        return [
            kind,
            [
                excluded_value,
                _positive_boundary_fact(
                    record.get("positive_boundary_fact"), f"{label}.positive_boundary_fact"
                ),
            ],
        ]
    _fail("AUTHORITY_VALUE_INVALID", f"{label}.kind is not a closed domain criterion")


def _domain_member(value: object, label: str) -> tuple[JsonObject, list[CborValue]]:
    record = _exact(
        value,
        {
            "candidate_id",
            "candidate_identity",
            "source_instance_id",
            "candidate_universe_binding",
            "domain_binding",
            "precondition_attestations",
            "member_evidence_refs",
            "domain_member_attestation",
        },
        label,
    )
    _, candidate_identity = _candidate_identity(
        record.get("candidate_identity"), f"{label}.candidate_identity"
    )
    _, preconditions = _precondition_attestations(
        record.get("precondition_attestations"), f"{label}.precondition_attestations"
    )
    _, member_evidence = _evidence_refs(
        record.get("member_evidence_refs"), f"{label}.member_evidence_refs", nonempty=True
    )
    attestation = _exact(
        record.get("domain_member_attestation"),
        {"criterion_attestations"},
        f"{label}.domain_member_attestation",
    )
    criteria = []
    for index, item in enumerate(
        _array(attestation.get("criterion_attestations"), "criterion attestations")
    ):
        criterion = _exact(
            item,
            {"criterion_index", "observed_criterion", "evidence_refs", "equivalence_rationale"},
            f"{label}.criterion_attestations[{index}]",
        )
        _, evidence = _evidence_refs(
            criterion.get("evidence_refs"), "criterion evidence", nonempty=True
        )
        criteria.append(
            [
                _uint32(criterion.get("criterion_index"), "criterion index"),
                _domain_criterion(criterion.get("observed_criterion"), "observed criterion"),
                evidence,
                _text(criterion.get("equivalence_rationale"), "criterion rationale"),
            ]
        )
    return record, [
        _text(record.get("candidate_id"), f"{label}.candidate_id"),
        candidate_identity,
        _text(record.get("source_instance_id"), f"{label}.source_instance_id"),
        _candidate_universe_binding(
            record.get("candidate_universe_binding"), f"{label}.candidate_universe_binding"
        ),
        _domain_binding(record.get("domain_binding"), f"{label}.domain_binding"),
        preconditions,
        member_evidence,
        [criteria],
    ]


def _context_slot(value: object, label: str) -> list[CborValue]:
    record = _exact(
        value,
        {"slot_kind", "slot_name", "observed_value", "evidence_refs", "equivalence_rationale"},
        label,
    )
    _, evidence = _evidence_refs(
        record.get("evidence_refs"), f"{label}.evidence_refs", nonempty=True
    )
    return [
        _text(record.get("slot_kind"), f"{label}.slot_kind"),
        _text(record.get("slot_name"), f"{label}.slot_name"),
        _cbor_value(record.get("observed_value"), f"{label}.observed_value"),
        evidence,
        _text(record.get("equivalence_rationale"), f"{label}.equivalence_rationale"),
    ]


def _context_member(value: object, label: str) -> tuple[JsonObject, list[CborValue]]:
    record = _exact(
        value,
        {
            "candidate_id",
            "candidate_identity",
            "source_instance_id",
            "candidate_universe_binding",
            "context_binding",
            "precondition_attestations",
            "member_evidence_refs",
            "context_member_attestation",
        },
        label,
    )
    _, candidate_identity = _candidate_identity(
        record.get("candidate_identity"), f"{label}.candidate_identity"
    )
    _, preconditions = _precondition_attestations(
        record.get("precondition_attestations"), f"{label}.precondition_attestations"
    )
    _, member_evidence = _evidence_refs(
        record.get("member_evidence_refs"), f"{label}.member_evidence_refs", nonempty=True
    )
    attestation = _exact(
        record.get("context_member_attestation"),
        {"slot_attestations"},
        f"{label}.context_member_attestation",
    )
    slots = [
        _context_slot(item, f"{label}.slot_attestations[{index}]")
        for index, item in enumerate(
            _array(attestation.get("slot_attestations"), "slot attestations")
        )
    ]
    return record, [
        _text(record.get("candidate_id"), f"{label}.candidate_id"),
        candidate_identity,
        _text(record.get("source_instance_id"), f"{label}.source_instance_id"),
        _candidate_universe_binding(
            record.get("candidate_universe_binding"), f"{label}.candidate_universe_binding"
        ),
        _context_binding(record.get("context_binding"), f"{label}.context_binding"),
        preconditions,
        member_evidence,
        [slots],
    ]


def _event_ref(value: object, label: str) -> ReviewEventRefV1:
    record = _exact(value, {"path", "raw_sha256", "locator"}, label)
    locator = _wire_locator(record.get("locator"), acceptance=False, label=f"{label}.locator")
    if locator[0] != "event_id" or locator[1] is None or _EVENT_ID_RE.fullmatch(locator[1]) is None:
        _fail("ACCEPTANCE_BINDING_INVALID", f"{label}.locator must be an event_id")
    return ReviewEventRefV1(
        _text(record.get("path"), f"{label}.path"),
        _digest(record.get("raw_sha256"), f"{label}.raw_sha256"),
        locator[1],
    )


def _acceptance(value: object, label: str) -> tuple[ReviewEventRefV1, JsonObject]:
    record = _exact(value, {"decision", "review_event_ref"}, label)
    if record.get("decision") != "human_accepted":
        _fail("ACCEPTANCE_BINDING_INVALID", f"{label}.decision is not human_accepted")
    reference = _event_ref(record.get("review_event_ref"), f"{label}.review_event_ref")
    return reference, record


class AuthorityValidator:
    """Validate one persisted V1 authority graph without deriving C semantics."""

    def __init__(self, resolver: AuthoritySourceResolver) -> None:
        self._resolver = resolver
        self._root_bindings: _SourceRegistry | None = None
        self._model_binding: SourceBindingDigestV1 | None = None
        self._model: Mapping[str, object] | None = None
        self._records: dict[str, _ValidatedRecord] = {}
        self._supersession_sources: dict[str, str | None] = {}
        self._superseded_record_ids: set[str] = set()
        self._supersession_ids: set[str] = set()
        self._used_bindings: set[bytes] = set()

    def validate(self, value: object) -> AuthorityValidationResult:
        try:
            return self._validate_document(value)
        except ResolutionError:
            raise
        except (AuthorityContractError, TypeError, ValueError) as exc:
            _fail("AUTHORITY_CONTRACT_INVALID", str(exc))

    def _validate_document(self, value: object) -> AuthorityValidationResult:
        self._root_bindings = None
        self._model_binding = None
        self._model = None
        document = _exact(
            value,
            {
                "schema",
                "model_binding",
                "source_bindings",
                "relation_proofs",
                "relation_applications",
                "domain_proofs",
                "domain_applications",
                "context_proofs",
                "context_applications",
                "supersession_records",
            },
            "authority document",
        )
        if document.get("schema") != AUTHORITY_SCHEMA_V1:
            _fail("AUTHORITY_SCHEMA_MISMATCH", "authority document schema is not V1")
        self._records = {}
        self._supersession_sources = {}
        self._superseded_record_ids = set()
        self._supersession_ids = set()
        self._used_bindings = set()
        registry = self._parse_root_bindings(document.get("source_bindings"))
        self._root_bindings = registry
        self._model_binding, self._model = self._resolve_model(
            document.get("model_binding"), registry
        )
        self._used_bindings = {encode_canonical(self._model_binding.to_cbor())}
        self._resolve_root_bindings(registry)

        relation_proofs = self._records_from_array(
            document.get("relation_proofs"),
            RecordKind.RELATION_THEOREM_RECORD,
            self._relation_theorem,
        )
        domain_proofs = self._records_from_array(
            document.get("domain_proofs"), RecordKind.DOMAIN_THEOREM_RECORD, self._domain_theorem
        )
        context_proofs = self._records_from_array(
            document.get("context_proofs"), RecordKind.CONTEXT_THEOREM_RECORD, self._context_theorem
        )
        relation_apps = self._records_from_array(
            document.get("relation_applications"),
            RecordKind.RELATION_APPLICATION_RECORD,
            self._relation_application,
        )
        domain_apps = self._records_from_array(
            document.get("domain_applications"),
            RecordKind.DOMAIN_APPLICATION_RECORD,
            self._domain_application,
        )
        context_apps = self._records_from_array(
            document.get("context_applications"),
            RecordKind.CONTEXT_APPLICATION_RECORD,
            self._context_application,
        )
        del relation_proofs, domain_proofs, context_proofs, relation_apps, domain_apps, context_apps
        self._validate_supersessions(document.get("supersession_records"))
        self._validate_current_application_references()
        self._validate_root_closure()
        counts = {
            "relation_proofs": len(_array(document.get("relation_proofs"), "relation proofs")),
            "relation_applications": len(
                _array(document.get("relation_applications"), "relation applications")
            ),
            "domain_proofs": len(_array(document.get("domain_proofs"), "domain proofs")),
            "domain_applications": len(
                _array(document.get("domain_applications"), "domain applications")
            ),
            "context_proofs": len(_array(document.get("context_proofs"), "context proofs")),
            "context_applications": len(
                _array(document.get("context_applications"), "context applications")
            ),
            "supersession_records": len(
                _array(document.get("supersession_records"), "supersession records")
            ),
        }
        return AuthorityValidationResult(True, MappingProxyType(counts))

    def _parse_root_bindings(self, value: object) -> _SourceRegistry:
        raw = _array(value, "source_bindings")
        bindings: list[SourceBindingDigestV1] = []
        for index, item in enumerate(raw):
            record = _exact(
                item,
                {"authority_kind", "artifact_role", "path", "schema_or_null", "raw_sha256"},
                f"source_bindings[{index}]",
            )
            role = _text(record.get("artifact_role"), "source binding role")
            authority_kind = _text(record.get("authority_kind"), "source binding authority kind")
            if _SOURCE_BINDING_KIND_BY_ROLE.get(role) != authority_kind:
                _fail(
                    "SOURCE_BINDING_INVALID",
                    f"source binding {role!r} has the wrong authority kind",
                )
            schema = record.get("schema_or_null")
            if not isinstance(schema, str | None):
                _fail(
                    "SOURCE_BINDING_INVALID", "source binding schema_or_null must be text or null"
                )
            binding = SourceBindingDigestV1(
                role,
                _text(record.get("path"), "source binding path"),
                schema,
                _digest(record.get("raw_sha256"), "source binding digest"),
            )
            if role not in {"acceptance_event_leaf", "reviewer_roster_leaf"} and any(
                existing.artifact_role == role for existing in bindings
            ):
                _fail(
                    "SOURCE_BINDING_AMBIGUOUS",
                    f"source binding role {role!r} appears more than once",
                )
            bindings.append(binding)
        encoded = [encode_canonical(item.to_cbor()) for item in bindings]
        if encoded != sorted(encoded) or len(set(encoded)) != len(encoded):
            _fail(
                "NONCANONICAL_SOURCE_BINDINGS", "source_bindings must be sorted and duplicate-free"
            )
        return _SourceRegistry(MappingProxyType(dict(zip(encoded, bindings, strict=True))))

    def _resolve_model(
        self, value: object, registry: _SourceRegistry
    ) -> tuple[SourceBindingDigestV1, Mapping[str, object]]:
        record = _exact(value, {"path", "raw_sha256", "model_id", "model_version"}, "model_binding")
        binding = SourceBindingDigestV1(
            "declared_model",
            _text(record.get("path"), "model_binding.path"),
            DECLARED_MODEL_SCHEMA,
            _digest(record.get("raw_sha256"), "model_binding.raw_sha256"),
        )
        registry.require(binding, "model_binding")
        artifact = self._resolver.resolve_source_binding(binding)
        try:
            model = json.loads(artifact.raw_bytes.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            _fail("MODEL_SOURCE_INVALID", str(exc))
        model_record = _object(model, "declared interaction model")
        if model_record.get("schema") != DECLARED_MODEL_SCHEMA:
            _fail("MODEL_SOURCE_INVALID", "declared model schema is not V2")
        if model_record.get("model_id") != record.get("model_id"):
            _fail("MODEL_BINDING_MISMATCH", "declared model ID differs from model_binding")
        if model_record.get("model_version") != record.get("model_version"):
            _fail("MODEL_BINDING_MISMATCH", "declared model version differs from model_binding")
        return binding, MappingProxyType(model_record)

    def _resolve_root_bindings(self, registry: _SourceRegistry) -> None:
        for binding in registry.by_key.values():
            self._resolver.resolve_source_binding(binding)

    def _records_from_array(
        self,
        value: object,
        kind: RecordKind,
        validator: object,
    ) -> tuple[_ValidatedRecord, ...]:
        records = _array(value, kind.value + "s")
        result: list[_ValidatedRecord] = []
        for index, item in enumerate(records):
            parsed = cast(object, validator)(item, f"{kind.value}[{index}]")
            self._register_record(parsed)
            result.append(parsed)
        return tuple(result)

    def _register_record(self, record: _ValidatedRecord) -> None:
        key = record.record_id.as_text()
        if key in self._records:
            _fail("DUPLICATE_RECORD_ID", f"accepted record {key!r} appears more than once")
        self._records[key] = record

    def _require_model(self) -> tuple[SourceBindingDigestV1, Mapping[str, object]]:
        if self._model_binding is None or self._model is None:
            _fail("MODEL_BINDING_MISSING", "authority model binding was not initialized")
        return self._model_binding, self._model

    def _require_root(self) -> _SourceRegistry:
        if self._root_bindings is None:
            _fail("SOURCE_BINDING_MISSING", "authority source bindings were not initialized")
        return self._root_bindings

    def _binding_for_evidence(self, reference: EvidenceRefV1) -> SourceBindingDigestV1:
        role: str | None = None
        schema: str | None = None
        if reference.authority_kind == "model":
            role = "declared_model"
            schema = DECLARED_MODEL_SCHEMA
        elif reference.authority_kind == "rev3":
            role = "rev3_source"
            schema = None if reference.path in _RAW_REV3_PATHS else "interaction-model.v1"
        elif reference.authority_kind == "b2":
            role = _STATIC_ROLE_BY_PATH.get(reference.path)
            if role not in {"b2_catalog", "b2_classifications", "b2_closure"}:
                role = None
            schema = {
                "b2_catalog": B2_CATALOG_SCHEMA,
                "b2_classifications": B2_CLASSIFICATION_SCHEMA,
                "b2_closure": B2_CLOSURE_SCHEMA,
            }.get(role)
        elif reference.authority_kind == "b1_final":
            role = _STATIC_ROLE_BY_PATH.get(reference.path)
            if role not in {"b1_final_citations", "b1_final_closure"}:
                role = None
            schema = {
                "b1_final_citations": B1_FINAL_CITATIONS_SCHEMA,
                "b1_final_closure": B1_FINAL_CLOSURE_SCHEMA,
            }.get(role)
        elif reference.authority_kind == "c_candidate":
            role = "candidate_universe"
            schema = CANDIDATE_UNIVERSE_SCHEMA
        elif reference.authority_kind == "reviewer_roster":
            role = "reviewer_roster_leaf"
            schema = REVIEWER_ROSTER_SCHEMA_V1
        elif reference.authority_kind == "acceptance_event":
            role = "acceptance_event_leaf"
            schema = ACCEPTANCE_EVENT_SCHEMA_V1
        if role is None:
            _fail(
                "EVIDENCE_BINDING_INVALID",
                f"unsupported evidence authority/path {reference.authority_kind}/{reference.path}",
            )
        return SourceBindingDigestV1(role, reference.path, schema, reference.raw_sha256)

    def _resolve_evidence(self, reference: EvidenceRefV1, label: str) -> None:
        binding = self._binding_for_evidence(reference)
        self._require_root().require(binding, label)
        self._used_bindings.add(encode_canonical(binding.to_cbor()))
        artifact = self._resolver.resolve_source_binding(binding)
        kind, payload = reference.locator
        if kind == "archive_member":
            if reference.authority_kind != "rev3" or payload != reference.path:
                _fail(
                    "EVIDENCE_LOCATOR_INVALID",
                    f"{label} archive member is not the bound REV3 artifact",
                )
            self._resolver.resolve_rev3_locator(
                reference.locator, reference.raw_sha256, binding.schema_or_null
            )
            return
        if kind == "whole_artifact":
            return
        if kind == "event_id":
            self._resolver.resolve_locator(artifact, reference.locator)
            return
        try:
            parsed = json.loads(artifact.raw_bytes.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            _fail("EVIDENCE_LOCATOR_INVALID", f"{label} JSON pointer source is not JSON: {exc}")
        _strict_json_pointer(parsed, cast(str, payload), f"{label}.locator")

    def _resolve_evidence_refs(self, references: Sequence[EvidenceRefV1], label: str) -> None:
        for index, reference in enumerate(references):
            self._resolve_evidence(reference, f"{label}[{index}]")

    def _b2_bindings(self) -> B2ArtifactBindingsV1:
        registry = self._require_root()

        def find(role: str) -> SourceBindingDigestV1:
            for binding in registry.by_key.values():
                if binding.artifact_role == role:
                    return binding
            _fail("SOURCE_BINDING_MISSING", f"missing {role} source binding")

        return B2ArtifactBindingsV1(
            find("b2_catalog"),
            find("b2_classifications"),
            find("b2_closure"),
        )

    def _b1_bindings(self) -> B1FinalArtifactBindingsV1:
        registry = self._require_root()
        found: dict[str, SourceBindingDigestV1] = {}
        for binding in registry.by_key.values():
            if binding.artifact_role in {"b1_final_citations", "b1_final_closure"}:
                found[binding.artifact_role] = binding
        if set(found) != {"b1_final_citations", "b1_final_closure"}:
            _fail("SOURCE_BINDING_MISSING", "B1.Final source bindings are incomplete")
        return B1FinalArtifactBindingsV1(found["b1_final_citations"], found["b1_final_closure"])

    def _resolve_b2_refs(self, values: object, label: str) -> None:
        raw_values = _array(values, label)
        if not raw_values:
            return
        bindings = self._b2_bindings()
        for index, value in enumerate(raw_values):
            record = _object(value, f"{label}[{index}]")
            family_id = _text(record.get("family_id"), "B2 family ID")
            definition = _text(record.get("precise_semantic_definition"), "B2 definition")
            family = self._resolver.resolve_b2_requirement_family(family_id, bindings)
            self._resolver.resolve_b2_boundary(family, B2BoundaryReferenceV1(family_id, definition))
            for binding in (bindings.catalog, bindings.classifications, bindings.closure):
                self._used_bindings.add(encode_canonical(binding.to_cbor()))

    def _resolve_b2_family_refs(self, values: object, label: str) -> None:
        raw_values = _array(values, label)
        if not raw_values:
            return
        bindings = self._b2_bindings()
        for index, value in enumerate(raw_values):
            record = _exact(
                value,
                {"family_id", "lifecycle", "assignment_role"},
                f"{label}[{index}]",
            )
            family_id = _text(record.get("family_id"), "B2 family ID")
            family = self._resolver.resolve_b2_requirement_family(family_id, bindings)
            source_lifecycle = _text(family.record.get("status"), "B2 family status").lower()
            if source_lifecycle != record.get("lifecycle"):
                _fail(
                    "B2_FAMILY_MISMATCH",
                    f"{label}[{index}] lifecycle differs from the catalog record",
                )
            for binding in (bindings.catalog, bindings.classifications, bindings.closure):
                self._used_bindings.add(encode_canonical(binding.to_cbor()))

    def _resolve_b1_refs(self, values: object, label: str) -> None:
        raw_values = _array(values, label)
        if not raw_values:
            return
        bindings = self._b1_bindings()
        for index, value in enumerate(raw_values):
            record = _object(value, f"{label}[{index}]")
            self._resolver.resolve_b1_final_authority_citation(
                _text(record.get("authority_id"), "B1 authority ID"),
                _text(record.get("citation_id"), "B1 citation ID"),
                bindings,
            )
            self._used_bindings.add(encode_canonical(bindings.citations.to_cbor()))
            self._used_bindings.add(encode_canonical(bindings.closure.to_cbor()))

    def _resolve_model_boundary(self, value: object, label: str) -> None:
        record = _exact(value, {"path", "schema", "raw_sha256", "locator"}, label)
        model_binding, _ = self._require_model()
        if (
            record.get("path") != model_binding.path
            or record.get("schema") != model_binding.schema_or_null
            or _digest(record.get("raw_sha256"), f"{label}.raw_sha256") != model_binding.raw_sha256
        ):
            _fail("MODEL_BOUNDARY_BINDING_MISMATCH", f"{label} does not bind the declared model")
        artifact = self._resolver.resolve_source_binding(model_binding)
        self._used_bindings.add(encode_canonical(model_binding.to_cbor()))
        locator = _object(record.get("locator"), f"{label}.locator")
        kind = _text(locator.get("kind"), f"{label}.locator.kind")
        if kind == "coverage_scope":
            if set(locator) != {"kind"}:
                _fail("MODEL_BOUNDARY_LOCATOR_INVALID", f"{label}.locator is not closed")
            try:
                model_value = json.loads(artifact.raw_bytes.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                _fail("MODEL_BOUNDARY_SOURCE_INVALID", str(exc))
            _strict_json_pointer(model_value, "/coverage_scope", label)
        elif kind == "excluded_claim":
            if set(locator) != {"kind", "index"}:
                _fail("MODEL_BOUNDARY_LOCATOR_INVALID", f"{label}.locator is not closed")
            pointer = f"/excluded_claims/{_uint32(locator.get('index'), f'{label}.locator.index')}"
            try:
                model_value = json.loads(artifact.raw_bytes.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                _fail("MODEL_BOUNDARY_SOURCE_INVALID", str(exc))
            _strict_json_pointer(model_value, pointer, label)
        else:
            _fail("MODEL_BOUNDARY_LOCATOR_INVALID", f"{label}.locator.kind is not closed")

    def _resolve_positive_fact(self, value: object, label: str) -> None:
        record = _object(value, label)
        kind = _text(record.get("kind"), f"{label}.kind")
        if kind == "b2_boundary":
            boundary = _exact(
                record,
                {
                    "kind",
                    "family_id",
                    "lifecycle",
                    "assignment_role",
                    "precise_semantic_definition",
                },
                label,
            )
            family_id = _text(boundary.get("family_id"), "B2 family ID")
            family = self._resolver.resolve_b2_requirement_family(family_id, self._b2_bindings())
            source_status = _text(family.record.get("status"), "B2 family status").lower()
            if source_status != boundary.get("lifecycle"):
                _fail("B2_FAMILY_MISMATCH", f"{label} lifecycle differs from source")
            self._resolve_b2_refs(
                [
                    {
                        "family_id": record.get("family_id"),
                        "precise_semantic_definition": record.get("precise_semantic_definition"),
                    }
                ],
                label,
            )
        elif kind == "b1_citation":
            self._resolve_b1_refs([record.get("citation")], f"{label}.citation")
        elif kind == "model_boundary":
            model_binding, model = self._require_model()
            if record.get("model_id") != model.get("model_id") or record.get(
                "model_version"
            ) != model.get("model_version"):
                _fail(
                    "MODEL_BOUNDARY_BINDING_MISMATCH", f"{label} does not name the declared model"
                )
            self._resolve_model_boundary(
                {
                    "path": model_binding.path,
                    "schema": model_binding.schema_or_null,
                    "raw_sha256": model_binding.raw_sha256.hex(),
                    "locator": record.get("model_boundary_locator"),
                },
                label,
            )
        elif kind in {"rev3_locator", "b2_locator"}:
            path = _text(record.get("path"), f"{label}.path")
            reference = EvidenceRefV1(
                "rev3" if kind == "rev3_locator" else "b2",
                path,
                _wire_locator(record.get("locator"), acceptance=False, label=f"{label}.locator"),
                _digest(record.get("raw_sha256"), f"{label}.raw_sha256"),
            )
            self._resolve_evidence(reference, label)
        elif kind == "context_slot":
            return
        else:
            _fail("AUTHORITY_VALUE_INVALID", f"{label}.kind is not a closed positive fact")

    def _resolve_model_boundary_from_scope(self, value: object, label: str) -> None:
        record = _object(value, label)
        _, model = self._require_model()
        if record.get("model_id") != model.get("model_id") or record.get(
            "model_version"
        ) != model.get("model_version"):
            _fail("MODEL_BOUNDARY_BINDING_MISMATCH", f"{label} does not name the declared model")
        self._resolve_model_boundary(
            record.get("model_boundary_ref"), f"{label}.model_boundary_ref"
        )
        self._resolve_evidence_wire_list(
            record.get("positive_boundary_evidence_refs"),
            f"{label}.positive_boundary_evidence_refs",
        )

    def _resolve_scope_payload_sources(self, value: object, label: str) -> None:
        record = _object(value, label)
        self._resolve_model_boundary(
            record.get("model_boundary_ref"), f"{label}.model_boundary_ref"
        )
        self._resolve_evidence_wire_list(
            record.get("positive_boundary_evidence_refs"),
            f"{label}.positive_boundary_evidence_refs",
        )

    def _resolve_evidence_wire_list(self, value: object, label: str) -> None:
        references = []
        for index, item in enumerate(_array(value, label)):
            reference, _ = _evidence_ref(item, f"{label}[{index}]")
            references.append(reference)
        self._resolve_evidence_refs(references, label)

    def _resolve_class_equivalence(self, value: object, label: str) -> None:
        record = _object(value, label)
        self._resolve_projection_sources(
            record.get("theorem_projection"), f"{label}.theorem_projection"
        )
        self._resolve_projection_sources(
            record.get("member_projection"), f"{label}.member_projection"
        )
        self._resolve_evidence_wire_list(record.get("evidence_refs"), f"{label}.evidence_refs")

    def _resolve_member_proof(self, value: object, label: str) -> None:
        record = _object(value, label)
        kind = _text(record.get("kind"), f"{label}.kind")
        if kind == "positive_interaction":
            projection = record.get("class_projection_equivalence")
            if projection is not None:
                self._resolve_class_equivalence(projection, f"{label}.class_projection_equivalence")
        elif kind == "positive_separation":
            for index, coverage in enumerate(
                _array(record.get("channel_coverages"), "channel coverages")
            ):
                coverage_record = _object(coverage, f"{label}.channel_coverages[{index}]")
                self._resolve_evidence_wire_list(
                    coverage_record.get("source_evidence_refs"), "channel source evidence"
                )
                self._resolve_b1_refs(
                    coverage_record.get("b1_final_citation_refs"), "channel B1 refs"
                )
                for fact_index, fact in enumerate(
                    _array(coverage_record.get("positive_boundary_facts"), "boundary facts")
                ):
                    self._resolve_positive_fact(fact, f"{label}.boundary_fact[{fact_index}]")
        elif kind == "model_bound_scope":
            self._resolve_model_boundary_from_scope(
                record.get("scope_boundary_attestation"), f"{label}.scope_boundary_attestation"
            )
        else:
            _fail("AUTHORITY_VALUE_INVALID", f"{label}.kind is not closed")

    def _resolve_member_evidence(self, record: Mapping[str, object], label: str) -> None:
        self._resolve_evidence_wire_list(
            record.get("member_evidence_refs"), f"{label}.member_evidence_refs"
        )
        for index, attestation in enumerate(
            _array(record.get("precondition_attestations"), "precondition attestations")
        ):
            attestation_record = _object(attestation, f"{label}.precondition_attestations[{index}]")
            self._resolve_evidence_wire_list(
                attestation_record.get("evidence_refs"), "precondition evidence"
            )

    def _event_role_bindings(
        self, event: Mapping[str, object], roster: ReviewerRosterV1
    ) -> tuple[ReviewerRoleBindingV1, ...]:
        bindings: list[ReviewerRoleBindingV1] = []
        roster_by_id = {reviewer.reviewer_id: reviewer for reviewer in roster.reviewers}
        for index, item in enumerate(
            _array(event.get("reviewer_role_bindings"), "reviewer role bindings")
        ):
            record = _exact(item, {"reviewer_id", "roles"}, f"reviewer_role_bindings[{index}]")
            reviewer_id = _text(record.get("reviewer_id"), "reviewer ID")
            roles = tuple(
                _text(role, "reviewer role")
                for role in _array(record.get("roles"), "reviewer roles")
            )
            try:
                binding = ReviewerRoleBindingV1(reviewer_id, roles)
            except (TypeError, ValueError) as exc:
                _fail("REVIEWER_ROLE_BINDING_INVALID", str(exc))
            reviewer = roster_by_id.get(reviewer_id)
            if reviewer is None or tuple(reviewer.roles) != binding.roles:
                _fail(
                    "REVIEWER_ROLE_BINDING_MISMATCH",
                    f"reviewer {reviewer_id!r} roles differ from roster",
                )
            bindings.append(binding)
        if not bindings:
            _fail("REVIEWER_ROLE_BINDING_INVALID", "acceptance event has no reviewer bindings")
        ids = [binding.reviewer_id for binding in bindings]
        if ids != sorted(ids) or len(set(ids)) != len(ids):
            _fail("NONCANONICAL_REVIEWERS", "reviewer role bindings must be sorted and unique")
        return tuple(bindings)

    def _parse_roster(self, reference: ReviewerRosterRefV1) -> ReviewerRosterV1:
        artifact = self._resolver.resolve_reviewer_roster_leaf(reference)
        try:
            value = json.loads(artifact.raw_bytes.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            _fail("REVIEWER_ROSTER_INVALID", str(exc))
        record = _exact(value, {"schema", "reviewers"}, "reviewer roster leaf")
        reviewers: list[ReviewerV1] = []
        for index, item in enumerate(_array(record.get("reviewers"), "roster reviewers")):
            reviewer = _exact(item, {"reviewer_id", "roles"}, f"roster reviewers[{index}]")
            try:
                reviewers.append(
                    ReviewerV1(
                        _text(reviewer.get("reviewer_id"), "roster reviewer ID"),
                        tuple(
                            _text(role, "roster role")
                            for role in _array(reviewer.get("roles"), "roster roles")
                        ),
                    )
                )
            except (TypeError, ValueError) as exc:
                _fail("REVIEWER_ROSTER_INVALID", str(exc))
        try:
            return ReviewerRosterV1(tuple(reviewers))
        except (TypeError, ValueError) as exc:
            _fail("REVIEWER_ROSTER_INVALID", str(exc))

    def _required_roles(self, record: Mapping[str, object]) -> set[str]:
        required = set(_REQUIRED_ROLE_BY_RECORD_KIND)
        if self._contains_information_dependency(record):
            required.add("information_safety_reviewer")
        return required

    def _contains_information_dependency(self, value: object, key: str | None = None) -> bool:
        if key == "precondition_kind" and value in {"source_context", "temporal_semantic"}:
            return False
        if key == "slot_name" and value in {"visibility", "information_relation"}:
            return True
        if key == "context_dimensions" and isinstance(value, list):
            return len(value) >= 9 and (
                value[1] != "not_applicable" or value[8] != "not_applicable"
            )
        if isinstance(value, Mapping):
            return any(
                self._contains_information_dependency(child, child_key)
                for child_key, child in value.items()
                if child_key not in {"semantic_rationale", "rationale", "why_required"}
            )
        if isinstance(value, list):
            return any(self._contains_information_dependency(child, key) for child in value)
        return False

    def _subject_bindings(
        self,
        record: Mapping[str, object],
        model_binding: SourceBindingDigestV1,
        roster: SourceBindingDigestV1,
    ) -> set[bytes]:
        result: set[bytes] = {
            encode_canonical(model_binding.to_cbor()),
            encode_canonical(roster.to_cbor()),
        }

        def walk(value: object, key: str | None = None) -> None:
            if key == "acceptance":
                return
            if key in {
                "source_evidence_refs",
                "member_evidence_refs",
                "evidence_refs",
                "positive_boundary_evidence_refs",
            }:
                for item in _array(value, key):
                    evidence, _ = _evidence_ref(item, key)
                    binding = self._binding_for_evidence(evidence)
                    self._require_root().require(binding, key)
                    result.add(encode_canonical(binding.to_cbor()))
                return
            if key == "preconditions":
                for index, item in enumerate(_array(value, key)):
                    precondition = _object(item, f"{key}[{index}]")
                    kind = _text(precondition.get("precondition_kind"), "precondition kind")
                    payload = _cbor_value(precondition.get("payload"), "precondition payload")
                    if kind == "b2_boundary":
                        self._add_b2_boundary_bindings(result)
                    elif kind == "class_projection":
                        _array(payload, "class projection precondition payload", 9)
                        self._add_b2_boundary_bindings(result)
                        self._add_all_b1_bindings(result)
                return
            if key == "candidate_universe_binding":
                source = _exact(value, {"path", "schema", "raw_sha256"}, key)
                binding = SourceBindingDigestV1(
                    "candidate_universe",
                    _text(source.get("path"), "candidate universe path"),
                    _text(source.get("schema"), "candidate universe schema"),
                    _digest(source.get("raw_sha256"), "candidate universe digest"),
                )
                self._require_root().require(binding, key)
                result.add(encode_canonical(binding.to_cbor()))
                return
            if key == "b2_boundary_refs":
                for item in _array(value, key):
                    ref = _object(item, key)
                    definition = _text(ref.get("precise_semantic_definition"), "B2 definition")
                    del definition
                    self._add_b2_boundary_bindings(result)
                return
            if key == "b2_family_refs":
                for item in _array(value, key):
                    _b2_family_ref(item, key)
                    self._add_b2_boundary_bindings(result)
                return
            if key == "through_boundary_refs":
                for item in _array(value, key):
                    _b2_boundary_ref(item, key)
                    self._add_b2_boundary_bindings(result)
                return
            if key == "b1_final_citation_refs":
                for _ in _array(value, key):
                    self._add_all_b1_bindings(result)
                return
            if (
                key == "citation"
                and isinstance(value, Mapping)
                and set(value)
                == {
                    "authority_id",
                    "citation_id",
                }
            ):
                self._add_all_b1_bindings(result)
                return
            if key in {"positive_boundary_fact", "positive_boundary_facts"}:
                facts = [value] if key == "positive_boundary_fact" else _array(value, key)
                for index, fact in enumerate(facts):
                    fact_record = _object(fact, f"{key}[{index}]")
                    fact_kind = _text(fact_record.get("kind"), "positive fact kind")
                    if fact_kind == "b2_boundary":
                        self._add_b2_boundary_bindings(result)
                    elif fact_kind == "b1_citation":
                        self._add_all_b1_bindings(result)
                    elif fact_kind in {"rev3_locator", "b2_locator"}:
                        reference, _ = _evidence_ref(
                            {
                                "authority_kind": "rev3" if fact_kind == "rev3_locator" else "b2",
                                "path": fact_record.get("path"),
                                "locator": fact_record.get("locator"),
                                "raw_sha256": fact_record.get("raw_sha256"),
                            },
                            f"{key}[{index}]",
                        )
                        binding = self._binding_for_evidence(reference)
                        self._require_root().require(binding, key)
                        result.add(encode_canonical(binding.to_cbor()))
                return
            if isinstance(value, dict):
                for child_key, child in value.items():
                    walk(child, child_key)
            elif isinstance(value, list):
                for child in value:
                    walk(child, key)

        walk(record)
        return result

    def _add_b2_boundary_bindings(self, target: set[bytes]) -> None:
        for binding in self._require_root().by_key.values():
            if binding.artifact_role in {"b2_catalog", "b2_closure"}:
                target.add(encode_canonical(binding.to_cbor()))

    def _add_all_b1_bindings(self, target: set[bytes]) -> None:
        for binding in self._require_root().by_key.values():
            if binding.artifact_role in {"b1_final_citations", "b1_final_closure"}:
                target.add(encode_canonical(binding.to_cbor()))

    def _validate_acceptance(
        self,
        record: Mapping[str, object],
        subject_kind: AcceptanceSubjectKind,
        subject_payload: list[CborValue],
        label: str,
    ) -> ReviewEventRefV1:
        acceptance_ref, _ = _acceptance(record.get("acceptance"), f"{label}.acceptance")
        event_artifact = self._resolver.resolve_acceptance_event_leaf(acceptance_ref)
        try:
            event_value = json.loads(event_artifact.raw_bytes.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            _fail("ACCEPTANCE_EVENT_INVALID", str(exc))
        event = _exact(
            event_value,
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
        if (
            event.get("schema") != ACCEPTANCE_EVENT_SCHEMA_V1
            or event.get("event_id") != acceptance_ref.event_id
        ):
            _fail("ACCEPTANCE_EVENT_INVALID", "acceptance event schema or ID mismatch")
        if event.get("subject_kind") != subject_kind.value:
            _fail(
                "ACCEPTANCE_SUBJECT_KIND_MISMATCH",
                "acceptance event subject kind differs from record kind",
            )
        roster_record = _exact(
            event.get("reviewer_roster_ref"), {"path", "schema", "raw_sha256"}, "event roster ref"
        )
        roster_ref = ReviewerRosterRefV1(
            _text(roster_record.get("path"), "event roster path"),
            _text(roster_record.get("schema"), "event roster schema"),
            _digest(roster_record.get("raw_sha256"), "event roster digest"),
        )
        roster = self._parse_roster(roster_ref)
        bindings = self._event_role_bindings(event, roster)
        roles = {role for binding in bindings for role in binding.roles}
        if not self._required_roles(record).issubset(roles):
            _fail("REVIEWER_ROLE_UNAUTHORIZED", "acceptance event lacks a required roster role")

        model_binding, _ = self._require_model()
        roster_binding = SourceBindingDigestV1(
            "reviewer_roster_leaf", roster_ref.path, roster_ref.schema, roster_ref.raw_sha256
        )
        expected_bindings = self._subject_bindings(record, model_binding, roster_binding)
        self._used_bindings.update(expected_bindings)
        event_binding = SourceBindingDigestV1(
            "acceptance_event_leaf",
            acceptance_ref.path,
            ACCEPTANCE_EVENT_SCHEMA_V1,
            acceptance_ref.raw_sha256,
        )
        self._require_root().require(event_binding, "acceptance event leaf")
        self._used_bindings.add(encode_canonical(event_binding.to_cbor()))
        raw_event_bindings = _array(event.get("source_binding_digests"), "event source bindings")
        actual_bindings: set[bytes] = set()
        for index, item in enumerate(raw_event_bindings):
            binding_record = _exact(
                item,
                {"artifact_role", "path", "schema_or_null", "raw_sha256"},
                f"event source binding[{index}]",
            )
            schema = binding_record.get("schema_or_null")
            if not isinstance(schema, str | None):
                _fail("SOURCE_BINDING_INVALID", "event source binding schema is invalid")
            binding = SourceBindingDigestV1(
                _text(binding_record.get("artifact_role"), "event artifact role"),
                _text(binding_record.get("path"), "event source path"),
                schema,
                _digest(binding_record.get("raw_sha256"), "event source digest"),
            )
            self._require_root().require(binding, "event source binding")
            actual_bindings.add(encode_canonical(binding.to_cbor()))
        if actual_bindings != expected_bindings:
            _fail(
                "ACCEPTANCE_SOURCE_CLOSURE_MISMATCH",
                "acceptance event source bindings are not the recomputed subject set",
            )

        event_evidence: list[AcceptanceEvidenceRefV1] = []
        for index, item in enumerate(
            _array(event.get("review_evidence_refs"), "event review evidence")
        ):
            evidence_record = _exact(
                item, {"path", "raw_sha256", "locator"}, f"event review evidence[{index}]"
            )
            locator = _wire_locator(
                evidence_record.get("locator"),
                acceptance=True,
                label="event review evidence locator",
            )
            evidence = AcceptanceEvidenceRefV1(
                _text(evidence_record.get("path"), "event review evidence path"),
                _digest(evidence_record.get("raw_sha256"), "event review evidence digest"),
                locator,
            )
            self._resolve_acceptance_evidence(evidence, f"event review evidence[{index}]")
            event_evidence.append(evidence)
        if not event_evidence:
            _fail("ACCEPTANCE_EVIDENCE_MISSING", "acceptance event has no review evidence")

        subject = AcceptanceSubjectPayloadV1(subject_kind, subject_payload)
        if (
            _digest(event.get("subject_payload_digest"), "event subject digest")
            != subject.identity().digest_bytes
        ):
            _fail(
                "ACCEPTANCE_SUBJECT_MISMATCH", "acceptance event subject digest differs from record"
            )
        return acceptance_ref

    def _resolve_acceptance_evidence(self, evidence: AcceptanceEvidenceRefV1, label: str) -> None:
        artifact = self._resolver.resolve_repository_artifact(
            evidence.path, evidence.raw_sha256, None
        )
        kind, payload = evidence.locator
        if kind == "whole_artifact":
            return
        if kind == "json_pointer":
            try:
                value = json.loads(artifact.raw_bytes.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                _fail("ACCEPTANCE_EVIDENCE_INVALID", f"{label} is not JSON: {exc}")
            _strict_json_pointer(value, cast(str, payload), label)
            return
        if kind == "archive_member":
            try:
                with zipfile.ZipFile(io.BytesIO(artifact.raw_bytes)) as archive:
                    if cast(str, payload) not in archive.namelist():
                        _fail(
                            "ACCEPTANCE_EVIDENCE_UNRESOLVED", f"{label} archive member is missing"
                        )
                    archive.read(cast(str, payload))
            except (OSError, zipfile.BadZipFile, KeyError) as exc:
                _fail("ACCEPTANCE_EVIDENCE_INVALID", f"{label} archive member is unreadable: {exc}")
            return
        _fail("ACCEPTANCE_EVIDENCE_INVALID", f"{label} locator is not supported")

    def _relation_theorem(self, value: object, label: str) -> _ValidatedRecord:
        record = _exact(
            value,
            {
                "theorem_id",
                "record_id",
                "proof_kind",
                "subject",
                "preconditions",
                "proof_payload",
                "b2_boundary_refs",
                "b1_final_citation_refs",
                "source_evidence_refs",
                "semantic_rationale",
                "acceptance",
            },
            label,
        )
        theorem_id = _identity_ref(
            record.get("theorem_id"), AuthorityIdentityKind.RELATION_THEOREM, f"{label}.theorem_id"
        )
        record_id = _identity_ref(
            record.get("record_id"),
            AuthorityIdentityKind.RELATION_THEOREM_RECORD,
            f"{label}.record_id",
        )
        subject = _subject(record.get("subject"), f"{label}.subject")
        _, preconditions = _preconditions(record.get("preconditions"), f"{label}.preconditions")
        proof_payload = _proof_payload(record.get("proof_payload"), f"{label}.proof_payload")
        if proof_payload[0] != record.get("proof_kind"):
            _fail(
                "THEOREM_PROOF_KIND_MISMATCH", f"{label}.proof_kind differs from proof_payload.kind"
            )
        if proof_payload[0] == "positive_interaction":
            proof_record = _object(record.get("proof_payload"), f"{label}.proof_payload")
            template = proof_record.get("class_projection_template")
            if template is not None:
                template_values = _class_projection(template, f"{label}.class_projection_template")
                if template_values[:4] != [
                    subject[0],
                    subject[2],
                    subject[3],
                    subject[4],
                ]:
                    _fail(
                        "CLASS_PROJECTION_BINDING_MISMATCH",
                        f"{label} class projection shape differs from theorem subject",
                    )
        _, b2_refs = self._b2_refs_for_identity(
            record.get("b2_boundary_refs"), f"{label}.b2_boundary_refs"
        )
        _, b1_refs = self._b1_refs_for_identity(
            record.get("b1_final_citation_refs"), f"{label}.b1_final_citation_refs"
        )
        source_evidence, source_arrays = _evidence_refs(
            record.get("source_evidence_refs"), f"{label}.source_evidence_refs", nonempty=True
        )
        self._resolve_evidence_refs(source_evidence, f"{label}.source_evidence_refs")
        self._resolve_b2_refs(record.get("b2_boundary_refs"), f"{label}.b2_boundary_refs")
        self._resolve_b1_refs(
            record.get("b1_final_citation_refs"), f"{label}.b1_final_citation_refs"
        )
        self._resolve_precondition_sources(record.get("preconditions"), f"{label}.preconditions")
        self._resolve_proof_payload_sources(record.get("proof_payload"), f"{label}.proof_payload")
        _, model = self._require_model()
        semantic_expected = self._compute_relation_theorem_id(
            record, model, subject, preconditions, proof_payload, b2_refs, b1_refs
        )
        if semantic_expected != theorem_id:
            _fail(
                "THEOREM_IDENTITY_MISMATCH",
                f"{label}.theorem_id does not match its semantic preimage",
            )
        rationale = _text(record.get("semantic_rationale"), f"{label}.semantic_rationale")
        event_ref = self._validate_acceptance(
            record,
            AcceptanceSubjectKind.RELATION_THEOREM_RECORD,
            [theorem_id.digest_bytes, source_arrays, rationale],
            label,
        )
        expected_record = compute_authority_identity(
            AuthorityIdentityKind.RELATION_THEOREM_RECORD,
            [
                "manafold.m2.5.c.relation-proof-record-input.v1",
                theorem_id.digest_bytes,
                source_arrays,
                event_ref.to_cbor(),
                rationale,
            ],
        )
        if expected_record != record_id:
            _fail(
                "RECORD_IDENTITY_MISMATCH",
                f"{label}.record_id does not match its accepted record preimage",
            )
        return _ValidatedRecord(
            RecordKind.RELATION_THEOREM_RECORD,
            MappingProxyType(record),
            theorem_id,
            record_id,
            event_ref,
        )

    def _compute_relation_theorem_id(
        self,
        record: Mapping[str, object],
        model: Mapping[str, object],
        subject: list[CborValue],
        preconditions: list[CborValue],
        proof: list[CborValue],
        b2: list[CborValue],
        b1: list[CborValue],
    ) -> AuthorityIdentityV1:
        return compute_authority_identity(
            AuthorityIdentityKind.RELATION_THEOREM,
            [
                "manafold.m2.5.c.relation-proof-input.v1",
                _text(model.get("model_id"), "model ID"),
                _text(record.get("proof_kind"), "proof kind"),
                subject[0],
                subject[1],
                subject[2],
                subject[4],
                subject[3],
                preconditions,
                proof,
                b2,
                b1,
            ],
        )

    def _domain_theorem(self, value: object, label: str) -> _ValidatedRecord:
        record = _exact(
            value,
            {
                "theorem_id",
                "record_id",
                "review_domain",
                "applicability",
                "criterion",
                "preconditions",
                "b2_boundary_refs",
                "b1_final_citation_refs",
                "source_evidence_refs",
                "semantic_rationale",
                "acceptance",
            },
            label,
        )
        theorem_id = _identity_ref(
            record.get("theorem_id"), AuthorityIdentityKind.DOMAIN_THEOREM, f"{label}.theorem_id"
        )
        record_id = _identity_ref(
            record.get("record_id"),
            AuthorityIdentityKind.DOMAIN_THEOREM_RECORD,
            f"{label}.record_id",
        )
        criteria = [
            _domain_criterion(item, f"{label}.criterion[{index}]")
            for index, item in enumerate(_array(record.get("criterion"), "domain criterion"))
        ]
        _, preconditions = _preconditions(record.get("preconditions"), f"{label}.preconditions")
        _, b2 = self._b2_refs_for_identity(
            record.get("b2_boundary_refs"), f"{label}.b2_boundary_refs"
        )
        _, b1 = self._b1_refs_for_identity(
            record.get("b1_final_citation_refs"), f"{label}.b1_final_citation_refs"
        )
        source_evidence, source_arrays = _evidence_refs(
            record.get("source_evidence_refs"), f"{label}.source_evidence_refs", nonempty=True
        )
        self._resolve_evidence_refs(source_evidence, f"{label}.source_evidence_refs")
        self._resolve_b2_refs(record.get("b2_boundary_refs"), f"{label}.b2_boundary_refs")
        self._resolve_b1_refs(
            record.get("b1_final_citation_refs"), f"{label}.b1_final_citation_refs"
        )
        self._resolve_precondition_sources(record.get("preconditions"), f"{label}.preconditions")
        for index, item in enumerate(_array(record.get("criterion"), "domain criterion")):
            self._resolve_criterion_sources(item, f"{label}.criterion[{index}]")
        _, model = self._require_model()
        expected = compute_authority_identity(
            AuthorityIdentityKind.DOMAIN_THEOREM,
            [
                "manafold.m2.5.c.domain-proof-input.v1",
                _text(model.get("model_id"), "model ID"),
                _text(record.get("review_domain"), "review domain"),
                _text(record.get("applicability"), "applicability"),
                criteria,
                preconditions,
                b2,
                b1,
            ],
        )
        if expected != theorem_id:
            _fail(
                "THEOREM_IDENTITY_MISMATCH",
                f"{label}.theorem_id does not match its semantic preimage",
            )
        rationale = _text(record.get("semantic_rationale"), f"{label}.semantic_rationale")
        event_ref = self._validate_acceptance(
            record,
            AcceptanceSubjectKind.DOMAIN_THEOREM_RECORD,
            [theorem_id.digest_bytes, source_arrays, rationale],
            label,
        )
        expected_record = compute_authority_identity(
            AuthorityIdentityKind.DOMAIN_THEOREM_RECORD,
            [
                "manafold.m2.5.c.domain-proof-record-input.v1",
                theorem_id.digest_bytes,
                source_arrays,
                event_ref.to_cbor(),
                rationale,
            ],
        )
        if expected_record != record_id:
            _fail(
                "RECORD_IDENTITY_MISMATCH",
                f"{label}.record_id does not match its accepted record preimage",
            )
        return _ValidatedRecord(
            RecordKind.DOMAIN_THEOREM_RECORD,
            MappingProxyType(record),
            theorem_id,
            record_id,
            event_ref,
        )

    def _context_theorem(self, value: object, label: str) -> _ValidatedRecord:
        record = _exact(
            value,
            {
                "theorem_id",
                "record_id",
                "subject_shape",
                "context_dimensions",
                "temporal_semantics",
                "preconditions",
                "b2_boundary_refs",
                "b1_final_citation_refs",
                "source_evidence_refs",
                "semantic_rationale",
                "acceptance",
            },
            label,
        )
        theorem_id = _identity_ref(
            record.get("theorem_id"), AuthorityIdentityKind.CONTEXT_THEOREM, f"{label}.theorem_id"
        )
        record_id = _identity_ref(
            record.get("record_id"),
            AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
            f"{label}.record_id",
        )
        subject = _context_binding(record.get("subject_shape"), f"{label}.subject_shape")
        dimensions = [
            _text(item, "context dimension")
            for item in _array(record.get("context_dimensions"), "context dimensions")
        ]
        temporal = [
            _text(item, "temporal semantic")
            for item in _array(record.get("temporal_semantics"), "temporal semantics")
        ]
        _, preconditions = _preconditions(record.get("preconditions"), f"{label}.preconditions")
        _, b2 = self._b2_refs_for_identity(
            record.get("b2_boundary_refs"), f"{label}.b2_boundary_refs"
        )
        _, b1 = self._b1_refs_for_identity(
            record.get("b1_final_citation_refs"), f"{label}.b1_final_citation_refs"
        )
        source_evidence, source_arrays = _evidence_refs(
            record.get("source_evidence_refs"), f"{label}.source_evidence_refs", nonempty=True
        )
        self._resolve_evidence_refs(source_evidence, f"{label}.source_evidence_refs")
        self._resolve_b2_refs(record.get("b2_boundary_refs"), f"{label}.b2_boundary_refs")
        self._resolve_b1_refs(
            record.get("b1_final_citation_refs"), f"{label}.b1_final_citation_refs"
        )
        self._resolve_precondition_sources(record.get("preconditions"), f"{label}.preconditions")
        _, model = self._require_model()
        expected = compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_THEOREM,
            [
                "manafold.m2.5.c.context-proof-input.v1",
                _text(model.get("model_id"), "model ID"),
                subject,
                dimensions,
                temporal,
                preconditions,
                b2,
                b1,
            ],
        )
        if expected != theorem_id:
            _fail(
                "THEOREM_IDENTITY_MISMATCH",
                f"{label}.theorem_id does not match its semantic preimage",
            )
        rationale = _text(record.get("semantic_rationale"), f"{label}.semantic_rationale")
        event_ref = self._validate_acceptance(
            record,
            AcceptanceSubjectKind.CONTEXT_THEOREM_RECORD,
            [theorem_id.digest_bytes, source_arrays, rationale],
            label,
        )
        expected_record = compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
            [
                "manafold.m2.5.c.context-proof-record-input.v1",
                theorem_id.digest_bytes,
                source_arrays,
                event_ref.to_cbor(),
                rationale,
            ],
        )
        if expected_record != record_id:
            _fail(
                "RECORD_IDENTITY_MISMATCH",
                f"{label}.record_id does not match its accepted record preimage",
            )
        return _ValidatedRecord(
            RecordKind.CONTEXT_THEOREM_RECORD,
            MappingProxyType(record),
            theorem_id,
            record_id,
            event_ref,
        )

    def _relation_application(self, value: object, label: str) -> _ValidatedRecord:
        record = _exact(
            value,
            {
                "application_id",
                "record_id",
                "theorem_record_id",
                "terminal_disposition",
                "members",
                "acceptance",
            },
            label,
        )
        application_id = _identity_ref(
            record.get("application_id"),
            AuthorityIdentityKind.RELATION_APPLICATION,
            f"{label}.application_id",
        )
        record_id = _identity_ref(
            record.get("record_id"),
            AuthorityIdentityKind.RELATION_APPLICATION_RECORD,
            f"{label}.record_id",
        )
        theorem_record_id = _identity_ref(
            record.get("theorem_record_id"),
            AuthorityIdentityKind.RELATION_THEOREM_RECORD,
            f"{label}.theorem_record_id",
        )
        theorem = self._require_record(theorem_record_id, RecordKind.RELATION_THEOREM_RECORD, label)
        member_records, members = self._application_members(
            record.get("members"), _relation_member, label
        )
        proof_kind = _text(
            cast(Mapping[str, object], theorem.record).get("proof_kind"), "relation proof kind"
        )
        disposition = _text(record.get("terminal_disposition"), "terminal disposition")
        expected_disposition = {
            "positive_interaction": "required_interaction",
            "positive_separation": "not_an_interaction_with_proof",
            "model_bound_scope": "out_of_declared_scope_with_reason",
        }.get(proof_kind)
        if disposition != expected_disposition:
            _fail(
                "APPLICATION_THEOREM_MISMATCH",
                f"{label}.terminal_disposition does not match theorem proof kind",
            )
        if (
            disposition == "required_interaction"
            and _object(theorem.record.get("proof_payload"), "relation proof payload").get(
                "class_projection_template"
            )
            is None
        ):
            _fail(
                "CLASS_PROJECTION_REQUIRED",
                f"{label} required_interaction has no class projection template",
            )
        self._validate_relation_members(
            member_records, cast(Mapping[str, object], theorem.record), label
        )
        expected = compute_authority_identity(
            AuthorityIdentityKind.RELATION_APPLICATION,
            [
                "manafold.m2.5.c.relation-application-input.v1",
                theorem_record_id.digest_bytes,
                disposition,
                members,
            ],
        )
        if expected != application_id:
            _fail(
                "APPLICATION_IDENTITY_MISMATCH",
                f"{label}.application_id does not match its member set",
            )
        event_ref = self._validate_acceptance(
            record,
            AcceptanceSubjectKind.RELATION_APPLICATION_RECORD,
            [application_id.digest_bytes],
            label,
        )
        expected_record = compute_authority_identity(
            AuthorityIdentityKind.RELATION_APPLICATION_RECORD,
            [
                "manafold.m2.5.c.relation-application-record-input.v1",
                application_id.digest_bytes,
                event_ref.to_cbor(),
            ],
        )
        if expected_record != record_id:
            _fail(
                "RECORD_IDENTITY_MISMATCH",
                f"{label}.record_id does not match its accepted record preimage",
            )
        return _ValidatedRecord(
            RecordKind.RELATION_APPLICATION_RECORD,
            MappingProxyType(record),
            application_id,
            record_id,
            event_ref,
        )

    def _domain_application(self, value: object, label: str) -> _ValidatedRecord:
        record = _exact(
            value,
            {
                "application_id",
                "record_id",
                "theorem_record_id",
                "review_domain",
                "applicability",
                "members",
                "acceptance",
            },
            label,
        )
        application_id = _identity_ref(
            record.get("application_id"),
            AuthorityIdentityKind.DOMAIN_APPLICATION,
            f"{label}.application_id",
        )
        record_id = _identity_ref(
            record.get("record_id"),
            AuthorityIdentityKind.DOMAIN_APPLICATION_RECORD,
            f"{label}.record_id",
        )
        theorem_record_id = _identity_ref(
            record.get("theorem_record_id"),
            AuthorityIdentityKind.DOMAIN_THEOREM_RECORD,
            f"{label}.theorem_record_id",
        )
        theorem = self._require_record(theorem_record_id, RecordKind.DOMAIN_THEOREM_RECORD, label)
        if (
            record.get("review_domain") != theorem.record["review_domain"]
            or record.get("applicability") != theorem.record["applicability"]
        ):
            _fail("APPLICATION_THEOREM_MISMATCH", f"{label} domain binding differs from theorem")
        member_records, members = self._application_members(
            record.get("members"), _domain_member, label
        )
        self._validate_domain_members(
            member_records, cast(Mapping[str, object], theorem.record), label
        )
        expected = compute_authority_identity(
            AuthorityIdentityKind.DOMAIN_APPLICATION,
            [
                "manafold.m2.5.c.domain-application-input.v1",
                theorem_record_id.digest_bytes,
                _text(record.get("review_domain"), "review domain"),
                _text(record.get("applicability"), "applicability"),
                members,
            ],
        )
        if expected != application_id:
            _fail(
                "APPLICATION_IDENTITY_MISMATCH",
                f"{label}.application_id does not match its member set",
            )
        event_ref = self._validate_acceptance(
            record,
            AcceptanceSubjectKind.DOMAIN_APPLICATION_RECORD,
            [application_id.digest_bytes],
            label,
        )
        expected_record = compute_authority_identity(
            AuthorityIdentityKind.DOMAIN_APPLICATION_RECORD,
            [
                "manafold.m2.5.c.domain-application-record-input.v1",
                application_id.digest_bytes,
                event_ref.to_cbor(),
            ],
        )
        if expected_record != record_id:
            _fail(
                "RECORD_IDENTITY_MISMATCH",
                f"{label}.record_id does not match its accepted record preimage",
            )
        return _ValidatedRecord(
            RecordKind.DOMAIN_APPLICATION_RECORD,
            MappingProxyType(record),
            application_id,
            record_id,
            event_ref,
        )

    def _context_application(self, value: object, label: str) -> _ValidatedRecord:
        record = _exact(
            value,
            {"application_id", "record_id", "theorem_record_id", "members", "acceptance"},
            label,
        )
        application_id = _identity_ref(
            record.get("application_id"),
            AuthorityIdentityKind.CONTEXT_APPLICATION,
            f"{label}.application_id",
        )
        record_id = _identity_ref(
            record.get("record_id"),
            AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD,
            f"{label}.record_id",
        )
        theorem_record_id = _identity_ref(
            record.get("theorem_record_id"),
            AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
            f"{label}.theorem_record_id",
        )
        theorem = self._require_record(theorem_record_id, RecordKind.CONTEXT_THEOREM_RECORD, label)
        member_records, members = self._application_members(
            record.get("members"), _context_member, label
        )
        self._validate_context_members(
            member_records, cast(Mapping[str, object], theorem.record), label
        )
        expected = compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_APPLICATION,
            [
                "manafold.m2.5.c.context-application-input.v1",
                theorem_record_id.digest_bytes,
                members,
            ],
        )
        if expected != application_id:
            _fail(
                "APPLICATION_IDENTITY_MISMATCH",
                f"{label}.application_id does not match its member set",
            )
        event_ref = self._validate_acceptance(
            record,
            AcceptanceSubjectKind.CONTEXT_APPLICATION_RECORD,
            [application_id.digest_bytes],
            label,
        )
        expected_record = compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD,
            [
                "manafold.m2.5.c.context-application-record-input.v1",
                application_id.digest_bytes,
                event_ref.to_cbor(),
            ],
        )
        if expected_record != record_id:
            _fail(
                "RECORD_IDENTITY_MISMATCH",
                f"{label}.record_id does not match its accepted record preimage",
            )
        return _ValidatedRecord(
            RecordKind.CONTEXT_APPLICATION_RECORD,
            MappingProxyType(record),
            application_id,
            record_id,
            event_ref,
        )

    def _application_members(
        self, value: object, parser: object, label: str
    ) -> tuple[tuple[JsonObject, ...], list[CborValue]]:
        raw = _array(value, f"{label}.members")
        records: list[JsonObject] = []
        arrays: list[list[CborValue]] = []
        keys: list[bytes] = []
        for index, item in enumerate(raw):
            parsed_record, parsed_array = cast(object, parser)(item, f"{label}.members[{index}]")
            records.append(parsed_record)
            arrays.append(parsed_array)
            identity = _candidate_identity(
                parsed_record.get("candidate_identity"), "candidate identity"
            )[0]
            keys.append(
                encode_canonical(
                    [
                        bytes.fromhex(cast(str, identity["digest_hex"])),
                        _text(parsed_record.get("source_instance_id"), "source instance ID"),
                    ]
                )
            )
        if keys != sorted(keys) or len(set(keys)) != len(keys):
            _fail("NONCANONICAL_MEMBERS", f"{label}.members must be sorted and duplicate-free")
        return tuple(records), arrays

    def _require_record(
        self, identity: AuthorityIdentityV1, kind: RecordKind, label: str
    ) -> _ValidatedRecord:
        record = self._records.get(identity.as_text())
        if record is None or record.kind is not kind:
            _fail(
                "RECORD_REFERENCE_INVALID",
                f"{label} references an unknown or wrong-kind theorem record",
            )
        return record

    def _source_instance_shape(
        self, resolved: ResolvedSourceInstance, label: str
    ) -> tuple[str, str, list[list[CborValue]]]:
        candidate = resolved.candidate.candidate_record
        relation = _text(candidate.get("relation"), f"{label}.candidate.relation")
        shape = _SOURCE_RELATION_SHAPES.get(relation)
        if shape is None:
            _fail(
                "SOURCE_RELATION_SHAPE_UNSUPPORTED",
                f"{label} candidate relation has no admitted C shape",
            )
        arity, directionality = shape
        participants: list[list[CborValue]] = []
        raw_participants = resolved.source_instance_record.get("participant_bindings")
        source_participants = (
            list(raw_participants)
            if isinstance(raw_participants, tuple)
            else _array(
                raw_participants,
                f"{label}.source_instance.participant_bindings",
            )
        )
        for index, item in enumerate(source_participants):
            binding = _exact(
                item,
                {"role", "participant_ref"},
                f"{label}.source_instance.participant_bindings[{index}]",
            )
            participant_ref = _exact(
                binding.get("participant_ref"),
                {"participant_kind", "semantic_ref"},
                f"{label}.source_instance.participant_bindings[{index}].participant_ref",
            )
            participants.append(
                [
                    index,
                    _text(binding.get("role"), "source participant role"),
                    _text(participant_ref.get("participant_kind"), "source participant kind"),
                    _text(participant_ref.get("semantic_ref"), "source participant reference"),
                ]
            )
        expected_count = 1 if arity == "unary" else 2
        if len(participants) != expected_count:
            _fail(
                "SOURCE_RELATION_SHAPE_MISMATCH",
                f"{label} source instance has the wrong participant count for its C shape",
            )
        return arity, directionality, participants

    def _validate_relation_member_binding(
        self,
        member: Mapping[str, object],
        theorem: Mapping[str, object],
        resolved: ResolvedSourceInstance,
        label: str,
    ) -> None:
        relation_record = _object(member.get("relation_binding"), f"{label}.relation_binding")
        subject_record = _object(theorem.get("subject"), "relation theorem subject")
        relation_values = _relation_binding(relation_record, f"{label}.relation_binding")
        subject_values = _subject(subject_record, "relation theorem subject")
        if relation_values[1:] != [
            subject_values[1],
            subject_values[2],
            subject_values[4],
            subject_values[3],
        ]:
            _fail(
                "MEMBER_SUBJECT_MISMATCH", f"{label} relation binding differs from theorem subject"
            )
        candidate = resolved.candidate.candidate_record
        source_arity, source_directionality, source_participants = self._source_instance_shape(
            resolved, label
        )
        if subject_values[0] != source_arity or subject_values[2] != source_directionality:
            _fail(
                "MEMBER_SOURCE_SHAPE_MISMATCH",
                f"{label} theorem direction/arity differs from the source-instance C shape",
            )
        if relation_values[0] != candidate["scope"] or relation_values[1] != candidate["relation"]:
            _fail(
                "MEMBER_SOURCE_BINDING_MISMATCH",
                f"{label} relation binding differs from candidate source",
            )
        candidate_refs = [
            [
                ref["participant_kind"],
                ref["semantic_ref"],
            ]
            for ref in cast(Sequence[Mapping[str, str]], candidate["participant_refs"])
        ]
        member_refs = [
            [participant[2], participant[3]]
            for participant in cast(list[list[CborValue]], relation_values[4])
        ]
        if member_refs != candidate_refs:
            _fail(
                "MEMBER_SOURCE_BINDING_MISMATCH",
                f"{label} participants differ from candidate source",
            )
        if relation_values[4] != source_participants:
            _fail(
                "MEMBER_SOURCE_PARTICIPANT_BINDING_MISMATCH",
                f"{label} participant roles or positions differ from the source instance",
            )
        member_proof = _object(member.get("member_proof_attestation"), f"{label}.member proof")
        equivalence = member_proof.get("class_projection_equivalence")
        if member_proof.get("kind") == "positive_interaction" and equivalence is not None:
            equivalence_record = _object(equivalence, f"{label}.class projection equivalence")
            member_projection = _class_projection(
                equivalence_record.get("member_projection"), f"{label}.member projection"
            )
            if member_projection[:4] != [
                subject_values[0],
                subject_values[2],
                subject_values[3],
                subject_values[4],
            ]:
                _fail(
                    "CLASS_PROJECTION_BINDING_MISMATCH",
                    f"{label} member projection shape differs from relation binding",
                )

    def _validate_domain_member_binding(
        self, member: Mapping[str, object], theorem: Mapping[str, object], label: str
    ) -> None:
        binding = _domain_binding(member.get("domain_binding"), f"{label}.domain_binding")
        expected = [
            _text(theorem.get("review_domain"), "theorem review domain"),
            _text(theorem.get("applicability"), "theorem applicability"),
        ]
        if binding != expected:
            _fail("MEMBER_SUBJECT_MISMATCH", f"{label} domain binding differs from theorem")

    def _validate_context_member_binding(
        self,
        member: Mapping[str, object],
        theorem: Mapping[str, object],
        resolved: ResolvedSourceInstance,
        label: str,
    ) -> None:
        binding = _context_binding(member.get("context_binding"), f"{label}.context_binding")
        expected = _context_binding(theorem.get("subject_shape"), "context theorem subject shape")
        if binding != expected:
            _fail("MEMBER_SUBJECT_MISMATCH", f"{label} context binding differs from theorem")
        source_arity, source_directionality, source_participants = self._source_instance_shape(
            resolved, label
        )
        if binding[0] != source_arity or binding[1] != source_directionality:
            _fail(
                "MEMBER_SOURCE_SHAPE_MISMATCH",
                f"{label} context binding differs from the source-instance C shape",
            )
        if binding[2] != source_participants:
            _fail(
                "MEMBER_SOURCE_PARTICIPANT_BINDING_MISMATCH",
                f"{label} context participant roles or positions differ from the source instance",
            )

    def _validate_context_values_against_source(
        self,
        context_values: Sequence[object],
        resolved: ResolvedSourceInstance,
        label: str,
    ) -> None:
        source_context = _object(
            resolved.source_instance_record.get("source_context"),
            f"{label}.source_instance.source_context",
        )
        for dimension, expected_value in zip(_SOURCE_CONTEXT_KEYS, context_values, strict=True):
            if source_context.get(dimension) != expected_value:
                _fail(
                    "MEMBER_SOURCE_CONTEXT_MISMATCH",
                    f"{label} context dimension {dimension!r} differs from source instance",
                )

    def _validate_member_proof_against_theorem(
        self, member: Mapping[str, object], theorem: Mapping[str, object], label: str
    ) -> None:
        proof = _object(theorem.get("proof_payload"), "relation proof payload")
        proof_kind = _text(proof.get("kind"), "proof kind")
        member_proof = _object(member.get("member_proof_attestation"), f"{label}.member proof")
        if member_proof.get("kind") != proof_kind:
            _fail("MEMBER_PROOF_KIND_MISMATCH", f"{label} member proof kind differs from theorem")
        if proof_kind == "positive_interaction":
            ordinals = [
                _uint32(item, "causal chain ordinal")
                for item in _array(
                    member_proof.get("causal_chain_ordinals"), "causal chain ordinals"
                )
            ]
            chain = _array(proof.get("causal_chain"), "theorem causal chain")
            if ordinals != list(range(len(chain))):
                _fail(
                    "CAUSAL_CHAIN_BINDING_MISMATCH",
                    f"{label} does not bind the complete causal chain",
                )
            template = proof.get("class_projection_template")
            equivalence = member_proof.get("class_projection_equivalence")
            if (template is None) != (equivalence is None):
                _fail(
                    "CLASS_PROJECTION_BINDING_MISMATCH",
                    f"{label} class projection nullability differs",
                )
            if equivalence is not None:
                equivalence_record = _object(equivalence, "class projection equivalence")
                theorem_projection = _class_projection(
                    equivalence_record.get("theorem_projection"), "theorem projection"
                )
                member_projection = _class_projection(
                    equivalence_record.get("member_projection"), "member projection"
                )
                template_projection = _class_projection(template, "class projection template")
                if (
                    theorem_projection != template_projection
                    or member_projection != template_projection
                ):
                    _fail(
                        "CLASS_PROJECTION_BINDING_MISMATCH",
                        f"{label} projection differs from theorem template",
                    )
                claim = _object(
                    equivalence_record.get("semantic_claim_relation"), "semantic claim relation"
                )
                theorem_id = _identity_ref(
                    theorem.get("theorem_id"),
                    AuthorityIdentityKind.RELATION_THEOREM,
                    "theorem identity",
                )
                if (
                    _digest(claim.get("theorem_semantic_digest"), "theorem semantic digest")
                    != theorem_id.digest_bytes
                ):
                    _fail(
                        "CLASS_PROJECTION_BINDING_MISMATCH",
                        f"{label} class claim names another theorem",
                    )
        elif proof_kind == "positive_separation":
            obligations = _array(proof.get("separation_obligations"), "separation obligations")
            coverages = _array(member_proof.get("channel_coverages"), "channel coverages")
            if len(obligations) != len(coverages):
                _fail(
                    "SEPARATION_COVERAGE_INCOMPLETE", f"{label} separation channels are incomplete"
                )
            for obligation, coverage in zip(obligations, coverages, strict=True):
                obligation_record = _object(obligation, "separation obligation")
                coverage_record = _object(coverage, "channel coverage")
                if coverage_record.get("channel") != obligation_record.get(
                    "channel"
                ) or coverage_record.get("coverage") != obligation_record.get(
                    "required_conclusion"
                ):
                    _fail(
                        "SEPARATION_COVERAGE_MISMATCH",
                        f"{label} channel coverage differs from theorem",
                    )
        elif proof_kind == "model_bound_scope":
            expected_scope = _object(proof, "scope proof payload")
            actual_scope = _object(
                member_proof.get("scope_boundary_attestation"), "scope attestation"
            )
            _, model = self._require_model()
            if (
                actual_scope.get("model_id") != model.get("model_id")
                or actual_scope.get("model_version") != model.get("model_version")
                or actual_scope.get("reason_code") != expected_scope.get("reason_code")
                or _candidate_shape(
                    actual_scope.get("observed_candidate_shape"), "member scope shape"
                )
                != _candidate_shape(
                    expected_scope.get("observed_candidate_shape"), "theorem scope shape"
                )
                or _model_boundary_ref(
                    actual_scope.get("model_boundary_ref"), "member scope boundary"
                )
                != _model_boundary_ref(
                    expected_scope.get("model_boundary_ref"), "theorem scope boundary"
                )
            ):
                _fail("SCOPE_BINDING_MISMATCH", f"{label} scope attestation differs from theorem")

    def _validate_relation_members(
        self, members: Sequence[JsonObject], theorem: Mapping[str, object], label: str
    ) -> None:
        proof_kind = _text(theorem.get("proof_kind"), "proof kind")
        for index, member in enumerate(members):
            resolved = self._resolve_application_member(member, f"{label}.members[{index}]")
            self._validate_relation_member_binding(
                member, theorem, resolved, f"{label}.members[{index}]"
            )
            self._resolve_member_evidence(member, f"{label}.members[{index}]")
            self._resolve_member_proof(
                member.get("member_proof_attestation"),
                f"{label}.members[{index}].member_proof_attestation",
            )
            proof = _object(theorem.get("proof_payload"), "relation proof payload")
            if (
                proof.get("kind") != proof_kind
                or _object(member.get("member_proof_attestation"), "member proof").get("kind")
                != proof_kind
            ):
                _fail(
                    "MEMBER_PROOF_KIND_MISMATCH", "relation member proof kind differs from theorem"
                )
            self._validate_member_proof_against_theorem(
                member, theorem, f"{label}.members[{index}]"
            )
            self._validate_precondition_match(
                member, theorem, f"{label}.members[{index}]", resolved
            )

    def _validate_domain_members(
        self, members: Sequence[JsonObject], theorem: Mapping[str, object], label: str
    ) -> None:
        for index, member in enumerate(members):
            resolved = self._resolve_application_member(member, f"{label}.members[{index}]")
            self._validate_domain_member_binding(member, theorem, f"{label}.members[{index}]")
            self._resolve_member_evidence(member, f"{label}.members[{index}]")
            attestation = _object(member.get("domain_member_attestation"), "domain attestation")
            criteria = _array(attestation.get("criterion_attestations"), "criterion attestations")
            theorem_criteria = _array(theorem.get("criterion"), "theorem criteria")
            if len(criteria) != len(theorem_criteria):
                _fail("DOMAIN_CRITERION_COVERAGE", "domain member criterion coverage is incomplete")
            for criterion_index, criterion in enumerate(criteria):
                criterion_record = _object(criterion, "criterion attestation")
                if criterion_record.get("criterion_index") != criterion_index or _domain_criterion(
                    criterion_record.get("observed_criterion"), "observed criterion"
                ) != _domain_criterion(theorem_criteria[criterion_index], "theorem criterion"):
                    _fail(
                        "DOMAIN_CRITERION_MISMATCH",
                        "domain member criterion does not match theorem clause",
                    )
                self._resolve_evidence_wire_list(
                    criterion_record.get("evidence_refs"),
                    f"{label}.members[{index}].criterion_attestations[{criterion_index}]"
                    ".evidence_refs",
                )
                self._resolve_criterion_sources(
                    criterion_record.get("observed_criterion"), "criterion"
                )
            self._validate_precondition_match(
                member, theorem, f"{label}.members[{index}]", resolved
            )

    def _validate_context_members(
        self, members: Sequence[JsonObject], theorem: Mapping[str, object], label: str
    ) -> None:
        expected_slots = list(
            zip(
                ["context_dimension"] * 10 + ["temporal_semantic"] * 4,
                [
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
                    "trigger_order",
                    "dependency_order",
                    "duration",
                    "replacement_order",
                ],
                strict=True,
            )
        )
        context_values = _array(theorem.get("context_dimensions"), "context theorem dimensions")
        temporal_values = _array(
            theorem.get("temporal_semantics"), "context theorem temporal values"
        )
        expected_values = context_values + temporal_values
        for index, member in enumerate(members):
            resolved = self._resolve_application_member(member, f"{label}.members[{index}]")
            member_label = f"{label}.members[{index}]"
            self._validate_context_member_binding(member, theorem, resolved, member_label)
            self._validate_context_values_against_source(context_values, resolved, member_label)
            self._resolve_member_evidence(member, f"{label}.members[{index}]")
            attestation = _object(member.get("context_member_attestation"), "context attestation")
            slots = _array(attestation.get("slot_attestations"), "slot attestations")
            if len(slots) != len(expected_slots):
                _fail("CONTEXT_SLOT_COVERAGE", "context member must cover all fourteen slots")
            for slot_index, slot in enumerate(slots):
                slot_record = _object(slot, "context slot")
                if (slot_record.get("slot_kind"), slot_record.get("slot_name")) != expected_slots[
                    slot_index
                ] or slot_record.get("observed_value") != expected_values[slot_index]:
                    _fail("CONTEXT_SLOT_MISMATCH", "context member slot differs from theorem")
                self._resolve_evidence_wire_list(
                    slot_record.get("evidence_refs"), "context slot evidence"
                )
            self._validate_precondition_match(
                member, theorem, f"{label}.members[{index}]", resolved
            )

    def _resolve_application_member(
        self, member: Mapping[str, object], label: str
    ) -> ResolvedSourceInstance:
        binding_record = _exact(
            member.get("candidate_universe_binding"),
            {"path", "schema", "raw_sha256"},
            f"{label}.candidate_universe_binding",
        )
        binding = SourceBindingDigestV1(
            "candidate_universe",
            _text(binding_record.get("path"), "candidate universe path"),
            _text(binding_record.get("schema"), "candidate universe schema"),
            _digest(binding_record.get("raw_sha256"), "candidate universe digest"),
        )
        self._require_root().require(binding, label)
        candidate_identity = _exact(
            member.get("candidate_identity"),
            _CANDIDATE_IDENTITY_KEYS,
            f"{label}.candidate_identity",
        )
        return self._resolver.resolve_candidate_source_instance(
            _text(member.get("candidate_id"), f"{label}.candidate_id"),
            candidate_identity,
            _text(member.get("source_instance_id"), f"{label}.source_instance_id"),
            binding,
        )

    def _validate_precondition_match(
        self,
        member: Mapping[str, object],
        theorem: Mapping[str, object],
        label: str,
        resolved: ResolvedSourceInstance,
    ) -> None:
        theorem_preconditions = _array(theorem.get("preconditions"), "theorem preconditions")
        attestations = _array(
            member.get("precondition_attestations"), "member precondition attestations"
        )
        if len(theorem_preconditions) != len(attestations):
            _fail("PRECONDITION_COVERAGE", f"{label} does not attest every theorem precondition")
        for index, (theorem_precondition, attestation) in enumerate(
            zip(theorem_preconditions, attestations, strict=True)
        ):
            theorem_id, _, theorem_array = _precondition(
                theorem_precondition, "theorem precondition"
            )
            attestation_record = _object(attestation, "precondition attestation")
            if attestation_record.get("precondition_id") != theorem_id:
                _fail(
                    "PRECONDITION_MISMATCH", f"{label} precondition {index} is out of theorem order"
                )
            kind = cast(str, theorem_array[1][0])
            expected = cast(list[CborValue], theorem_array[1][1])
            if kind == "candidate_relation_shape" and "relation_binding" in member:
                relation = _relation_binding(member.get("relation_binding"), "relation binding")
                expected = relation[:4]
            elif kind == "participant_binding" and "relation_binding" in member:
                payload = cast(list[CborValue], expected)
                position = cast(int, payload[0])
                relation_record = _object(member.get("relation_binding"), "relation binding")
                participants = _array(
                    relation_record.get("participant_bindings"), "participant bindings"
                )
                matches = [
                    _participant(item, "participant binding")
                    for item in participants
                    if _object(item, "participant binding").get("position") == position
                ]
                if len(matches) != 1:
                    _fail(
                        "PRECONDITION_MISMATCH",
                        f"{label} participant precondition has no unique member binding",
                    )
                expected = matches[0]
            elif kind == "source_context":
                dimension = cast(str, expected[0])
                source_context = _object(
                    resolved.source_instance_record.get("source_context"),
                    "source instance context",
                )
                if dimension not in source_context:
                    _fail("PRECONDITION_MISMATCH", f"{label} source context dimension is absent")
                expected = [dimension, source_context[dimension]]
            elif kind == "class_projection" and "member_proof_attestation" in member:
                member_proof = _object(member.get("member_proof_attestation"), "member proof")
                equivalence = member_proof.get("class_projection_equivalence")
                if equivalence is not None:
                    equivalence_record = _object(equivalence, "class projection equivalence")
                    expected = _class_projection(
                        equivalence_record.get("member_projection"),
                        "member class projection",
                    )
            if (
                _cbor_value(attestation_record.get("observed_value"), "observed precondition")
                != expected
            ):
                _fail(
                    "PRECONDITION_MISMATCH",
                    f"{label} observed precondition differs from theorem expectation",
                )

    def _resolve_precondition_sources(self, value: object, label: str) -> None:
        for index, item in enumerate(_array(value, label)):
            precondition = _object(item, f"{label}[{index}]")
            kind = _text(precondition.get("precondition_kind"), "precondition kind")
            payload = _cbor_value(precondition.get("payload"), "precondition payload")
            if kind == "b2_boundary":
                fields = _array(payload, "B2 boundary precondition", 4)
                self._resolve_b2_refs(
                    [
                        {
                            "family_id": fields[0],
                            "precise_semantic_definition": fields[3],
                        }
                    ],
                    f"{label}[{index}].B2 boundary",
                )
            elif kind == "class_projection":
                projection = _array(payload, "class projection precondition", 9)
                self._resolve_b2_family_arrays(
                    projection[6], f"{label}[{index}].class projection B2 families"
                )
                self._resolve_b2_arrays(
                    projection[7], f"{label}[{index}].class projection B2 boundaries"
                )
                self._resolve_b1_arrays(
                    projection[8], f"{label}[{index}].class projection B1 citations"
                )

    def _resolve_b2_family_arrays(self, value: object, label: str) -> None:
        raw = _array(value, label)
        if not raw:
            return
        bindings = self._b2_bindings()
        for index, item in enumerate(raw):
            fields = _array(item, f"{label}[{index}]", 3)
            family_id = _text(fields[0], "B2 family ID")
            family = self._resolver.resolve_b2_requirement_family(family_id, bindings)
            status = _text(family.record.get("status"), "B2 family status").lower()
            if status != fields[1]:
                _fail("B2_FAMILY_MISMATCH", f"{label}[{index}] lifecycle differs from source")
            for binding in (bindings.catalog, bindings.classifications, bindings.closure):
                self._used_bindings.add(encode_canonical(binding.to_cbor()))

    def _resolve_b2_arrays(self, value: object, label: str) -> None:
        raw = _array(value, label)
        if not raw:
            return
        refs = []
        for index, item in enumerate(raw):
            fields = _array(item, f"{label}[{index}]", 2)
            refs.append(
                {
                    "family_id": fields[0],
                    "precise_semantic_definition": fields[1],
                }
            )
        self._resolve_b2_refs(refs, label)

    def _resolve_b1_arrays(self, value: object, label: str) -> None:
        raw = _array(value, label)
        if not raw:
            return
        refs = []
        for index, item in enumerate(raw):
            fields = _array(item, f"{label}[{index}]", 2)
            refs.append({"authority_id": fields[0], "citation_id": fields[1]})
        self._resolve_b1_refs(refs, label)

    def _resolve_proof_payload_sources(self, value: object, label: str) -> None:
        record = _object(value, label)
        kind = _text(record.get("kind"), f"{label}.kind")
        if kind == "positive_interaction":
            for edge_index, edge in enumerate(_array(record.get("causal_chain"), "causal chain")):
                edge_record = _object(edge, f"{label}.causal_chain[{edge_index}]")
                self._resolve_b2_refs(edge_record.get("through_boundary_refs"), "causal B2 refs")
                self._resolve_b1_refs(edge_record.get("b1_final_citation_refs"), "causal B1 refs")
            projection = record.get("class_projection_template")
            if projection is not None:
                self._resolve_projection_sources(projection, f"{label}.class_projection_template")
        elif kind == "model_bound_scope":
            self._resolve_scope_payload_sources(record, label)

    def _resolve_projection_sources(self, value: object, label: str) -> None:
        record = _object(value, label)
        self._resolve_b2_family_refs(record.get("b2_family_refs"), f"{label}.b2_family_refs")
        self._resolve_b2_refs(record.get("b2_boundary_refs"), f"{label}.b2_boundary_refs")
        self._resolve_b1_refs(
            record.get("b1_final_citation_refs"), f"{label}.b1_final_citation_refs"
        )

    def _resolve_criterion_sources(self, value: object, label: str) -> None:
        record = _object(value, label)
        kind = _text(record.get("kind"), f"{label}.kind")
        if kind in {"channel_implicated", "channel_excluded", "rule_domain_excluded"}:
            if kind == "rule_domain_excluded":
                excluded = _object(record.get("excluded_domain_id"), f"{label}.excluded_domain_id")
                if excluded.get("kind") == "b1_final_citation":
                    self._resolve_b1_refs(
                        [excluded.get("citation")], f"{label}.excluded_domain_id.citation"
                    )
            fact_key = "positive_boundary_fact"
            self._resolve_positive_fact(record.get(fact_key), f"{label}.{fact_key}")
        elif kind == "rule_domain_required":
            self._resolve_b1_refs([record.get("citation")], f"{label}.citation")

    def _b2_refs_for_identity(
        self, value: object, label: str
    ) -> tuple[tuple[JsonObject, ...], list[CborValue]]:
        raw = _array(value, label)
        records = tuple(
            _exact(item, {"family_id", "precise_semantic_definition"}, f"{label}[{index}]")
            for index, item in enumerate(raw)
        )
        arrays = [_b2_boundary_ref(item, f"{label}[{index}]") for index, item in enumerate(records)]
        return records, arrays

    def _b1_refs_for_identity(
        self, value: object, label: str
    ) -> tuple[tuple[JsonObject, ...], list[CborValue]]:
        raw = _array(value, label)
        records = tuple(
            _exact(item, {"authority_id", "citation_id"}, f"{label}[{index}]")
            for index, item in enumerate(raw)
        )
        arrays = [_b1_citation_ref(item, f"{label}[{index}]") for index, item in enumerate(records)]
        return records, arrays

    def _validate_supersessions(self, value: object) -> None:
        for index, item in enumerate(_array(value, "supersession records")):
            label = f"supersession_record[{index}]"
            record = _exact(
                item,
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
                label,
            )
            try:
                superseded_kind = RecordKind(
                    _text(record.get("superseded_record_kind"), "superseded record kind")
                )
            except ValueError:
                _fail("SUPERSESSION_INVALID", "superseded record kind is not closed")
            replacement_kind_value = record.get("replacement_record_kind")
            try:
                replacement_kind = (
                    None
                    if replacement_kind_value is None
                    else RecordKind(_text(replacement_kind_value, "replacement record kind"))
                )
            except ValueError:
                _fail("SUPERSESSION_INVALID", "replacement record kind is not closed")
            superseded_id = _identity_ref(
                record.get("superseded_record_id"),
                _RECORD_KIND_TO_RECORD_ID_KIND[superseded_kind],
                "superseded record ID",
            )
            replacement_value = record.get("replacement_record_id")
            replacement_id = (
                None
                if replacement_value is None
                else _identity_ref(
                    replacement_value,
                    _RECORD_KIND_TO_RECORD_ID_KIND[superseded_kind],
                    "replacement record ID",
                )
            )
            if replacement_id is None:
                if (
                    replacement_kind is not None
                    or record.get("reason_code") != "authority_revocation"
                ):
                    _fail("SUPERSESSION_INVALID", "revocation replacement fields are inconsistent")
            elif replacement_kind is not superseded_kind:
                _fail("SUPERSESSION_KIND_MISMATCH", "supersession replacement kind differs")
            self._require_record(superseded_id, superseded_kind, label)
            if replacement_id is not None:
                self._require_record(replacement_id, replacement_kind, label)
                if replacement_id == superseded_id:
                    _fail("SUPERSESSION_CYCLE", "a record cannot supersede itself")
            if superseded_id.as_text() in self._supersession_sources:
                _fail("SUPERSESSION_AMBIGUOUS", "a record has multiple supersession successors")
            source_evidence, source_arrays = _evidence_refs(
                record.get("source_evidence_refs"),
                f"{label}.source_evidence_refs",
                nonempty=True,
            )
            self._resolve_evidence_refs(source_evidence, f"{label}.source_evidence_refs")
            try:
                reason = SupersessionReason(_text(record.get("reason_code"), "supersession reason"))
            except ValueError:
                _fail("SUPERSESSION_INVALID", "supersession reason is not closed")
            event_ref = self._validate_acceptance(
                record,
                AcceptanceSubjectKind.SUPERSESSION_RECORD,
                [
                    superseded_id.digest_bytes,
                    None if replacement_id is None else replacement_id.digest_bytes,
                    superseded_kind.value,
                    None if replacement_kind is None else replacement_kind.value,
                    reason.value,
                    source_arrays,
                ],
                label,
            )
            supersession = SupersessionRecordV1(
                superseded_id,
                replacement_id,
                superseded_kind,
                replacement_kind,
                reason,
                source_evidence,
                event_ref,
            )
            supersession_id = _identity_ref(
                record.get("supersession_id"),
                _RECORD_KIND_TO_SUPERSESSION_KIND[superseded_kind],
                "supersession ID",
            )
            if supersession_id.as_text() in self._supersession_ids:
                _fail("DUPLICATE_SUPERSESSION_ID", "supersession ID appears more than once")
            if supersession.identity() != supersession_id:
                _fail(
                    "SUPERSESSION_IDENTITY_MISMATCH", "supersession ID does not match its preimage"
                )
            self._supersession_ids.add(supersession_id.as_text())
            self._superseded_record_ids.add(superseded_id.as_text())
            self._supersession_sources[superseded_id.as_text()] = (
                None if replacement_id is None else replacement_id.as_text()
            )

        for start in self._supersession_sources:
            seen: set[str] = set()
            current = start
            while current in self._supersession_sources:
                if current in seen:
                    _fail("SUPERSESSION_CYCLE", "supersession graph contains a cycle")
                seen.add(current)
                successor = self._supersession_sources[current]
                if successor is None:
                    break
                if successor not in self._records:
                    _fail("SUPERSESSION_INVALID", "supersession replacement is not a known record")
                current = successor

    def _validate_current_application_references(self) -> None:
        theorem_kind_by_application = {
            RecordKind.RELATION_APPLICATION_RECORD: AuthorityIdentityKind.RELATION_THEOREM_RECORD,
            RecordKind.DOMAIN_APPLICATION_RECORD: AuthorityIdentityKind.DOMAIN_THEOREM_RECORD,
            RecordKind.CONTEXT_APPLICATION_RECORD: AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
        }
        for record in self._records.values():
            theorem_kind = theorem_kind_by_application.get(record.kind)
            if theorem_kind is None or record.record_id.as_text() in self._superseded_record_ids:
                continue
            theorem_record_id = _identity_ref(
                record.record.get("theorem_record_id"),
                theorem_kind,
                "current application theorem record ID",
            )
            if theorem_record_id.as_text() in self._superseded_record_ids:
                _fail(
                    "SUPERSEDED_AUTHORITY_USED",
                    f"current application {record.record_id.as_text()!r} uses a superseded theorem",
                )

    def _validate_root_closure(self) -> None:
        # Root closure is checked against the records and their acceptance
        # leaves.  The model binding is always required, even for an empty graph.
        actual = set(self._require_root().by_key)
        if actual != self._used_bindings:
            _fail(
                "SOURCE_BINDING_CLOSURE_MISMATCH",
                "root source_bindings are not the exact used binding set",
            )


__all__ = ["AuthorityValidationResult", "AuthorityValidator"]
