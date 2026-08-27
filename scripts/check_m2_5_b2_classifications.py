#!/usr/bin/env python3
"""Fail-closed verifier for the M2.5.B2 terminal classification snapshot.

The verifier is deliberately source-grounded and offline.  It consumes the
maintainer-pinned REV3 archive, validates the additive B2 artifact set, and
recomputes every persisted identity from the existing ADR-0038 codec.  It does
not implement Magic rules and it never treats a family record as evidence for
a card assignment.
"""

from __future__ import annotations

import argparse
import copy
import csv
import hashlib
import io
import json
import os
import re
import struct
import subprocess
import sys
import tempfile
import zipfile
from collections import Counter
from collections.abc import Callable
from pathlib import Path
from typing import Any, NoReturn

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from mtgml.persistence import encode_canonical, encode_envelope

EXIT_PASS = 0
EXIT_FAIL = 1
EXIT_BLOCKED = 2

B2_DIR = ROOT / "sources" / "m2_5" / "closures" / "B2"
ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"
ARCHIVE_RELATIVE_PATH = Path("m2_5/Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip")
EXPECTED_ARCHIVE_SHA256 = "99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90"

CLASSIFICATION_SCHEMA = "manafold.m2.5.b2.card-semantic-classifications.v1"
PROJECTION_SCHEMA = "manafold.m2.5.b2.deck-row-classification-refs.v1"
CATALOG_SCHEMA = "manafold.m2.5.b2.requirement-family-catalog.v1"
CLOSURE_SCHEMA = "manafold.m2.5.b2.classification-closure.v1"
NEGATIVE_SCHEMA = "manafold.m2.5.b2.negative-test-matrix.v1"
SUMMARY_SCHEMA = "manafold.m2.5.b2.verification-summary.v1"

SOURCE_DOMAIN = "manafold.m2.5.b2.source-identity.v1"
SOURCE_INPUT_SCHEMA = "manafold.m2.5.b2.source-identity-input.v1"
REV3_DOMAIN = "manafold.m2.5.b2.rev3-classification-record-identity.v1"
REV3_INPUT_SCHEMA = "manafold.m2.5.b2.rev3-classification-record-identity-input.v1"
CLASSIFICATION_DOMAIN = "manafold.m2.5.b2.classification-record-identity.v1"
CLASSIFICATION_INPUT_SCHEMA = "manafold.m2.5.b2.classification-record-identity-input.v1"
REVIEW_BASIS_SCHEMA = "manafold.m2.5.b2.review-basis.v1"
PROVENANCE_SCHEMA = "manafold.m2.5.b2.provenance.v1"
ORACLE_LOCATOR_SCHEMA = "manafold.m2.5.b2.oracle-field-locator.v1"
AUTHORITY_LOCATOR_SCHEMA = "manafold.m2.5.b2.authority-byte-fragment-locator.v1"
RULE_LOCATOR_SCHEMA = "manafold.m2.5.b2.comprehensive-rule-locator.v1"
SEMANTIC_BOUNDARY_PREFIX = "B2_SEMANTIC_BOUNDARY_V1"
SEMANTIC_BOUNDARY_FIELDS = (
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

RAW_ORACLE_ARTIFACT = "source/raw/oracle_cards_selected_REV3.jsonl"
REQUIRED_ARCHIVE_MEMBERS = (
    "inputs/deck_row_source_resolution_REV3.csv",
    "inputs/deck_row_classification_refs_REV3.csv",
    "inputs/oracle_semantic_evidence_REV3.json",
    "inputs/card_semantic_classification_REV3.json",
    "inputs/requirement_family_catalog_REV3.json",
    RAW_ORACLE_ARTIFACT,
    "derived/Card_Requirement_Map_REV3.csv",
    "derived/Pair_Requirement_Aggregates_REV3.json",
    "Manafold_M2_5_Package_Manifest_REV3.json",
)
AUTHORITY_MEMBERS = (
    "source/authorities/comprehensive_rules.txt",
    "source/authorities/commander_general.html",
    "source/authorities/commander_1v1.html",
    "source/authorities/banned_restricted.html",
    "source/authorities/commander_legends_release_notes.html",
    "source/authorities/kaldheim_release_notes.html",
)

EXACT_B2_FILES = (
    "B2_DESIGN_SPEC.md",
    "card_semantic_classifications.v1.json",
    "deck_row_classification_refs.v1.csv",
    "requirement_family_catalog.v1.json",
    "classification_closure.v1.json",
    "CLASSIFICATION_REPORT.md",
    "verification/b2_negative_test_matrix.v1.json",
    "verification/b2_verification_summary.v1.json",
)
B2_GIT_RELATIVE_ROOT = Path("sources/m2_5/closures/B2")
B2_SUMMARY_GIT_PATH = B2_GIT_RELATIVE_ROOT / "verification/b2_verification_summary.v1.json"
B2_GIT_FILES = tuple(B2_GIT_RELATIVE_ROOT / relative for relative in EXACT_B2_FILES)
CLOSURE_BOUND_FILES = (
    "B2_DESIGN_SPEC.md",
    "card_semantic_classifications.v1.json",
    "deck_row_classification_refs.v1.csv",
    "requirement_family_catalog.v1.json",
    "CLASSIFICATION_REPORT.md",
    "verification/b2_negative_test_matrix.v1.json",
)

REQUIRED_COMMANDS = (
    "python scripts/check_m2_5_master_drift.py",
    "python scripts/check_m2_5_master_drift.py --negative-self-test",
    "python scripts/check_m2_5_master_drift.py --verify-archive",
    "python scripts/check_m2_5_b2_classifications.py",
    "python scripts/check_m2_5_b2_classifications.py --negative-self-test",
    "python scripts/run_checks.py integration",
    "cargo +1.85.1 fmt --all -- --check",
    "cargo +1.85.1 check --workspace --all-targets --all-features --locked",
)

NEGATIVE_CASES = (
    ("MISSING_CLASSIFICATION_REJECTED", "Remove one of the 402 OSI classification records."),
    ("DUPLICATE_ORACLE_IDENTITY_REJECTED", "Duplicate one OSI classification identity."),
    ("UNKNOWN_ORACLE_IDENTITY_REJECTED", "Replace an OSI with an ID absent from pinned evidence."),
    (
        "NONTERMINAL_CLASSIFICATION_REJECTED",
        "Change a terminal review status to a working/private status.",
    ),
    ("MISSING_DECK_ROW_REFERENCE_REJECTED", "Remove one of the 441 projection rows."),
    ("UNKNOWN_DECK_ROW_REFERENCE_REJECTED", "Add a row ID absent from pinned deck resolution."),
    ("DECK_ROW_OSI_REBIND_REJECTED", "Rebind an existing row to another valid OSI."),
    ("REUSED_ORACLE_IDENTITY_FORK_REJECTED", "Give one repeated OSI a different assignment set."),
    (
        "SOURCE_DIGEST_MISMATCH_REJECTED",
        "Change a source binding without changing the pinned source.",
    ),
    ("SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", "Point a locator to a wrong record or field."),
    (
        "DISALLOWED_EVIDENCE_BASIS_REJECTED",
        "Use a basis absent from the assigned family allowlist.",
    ),
    (
        "EVIDENCE_BASIS_LOCATOR_KIND_MISMATCH_REJECTED",
        "Use an incompatible locator kind for the declared basis.",
    ),
    (
        "CARD_SIDE_EVIDENCE_MISSING_REJECTED",
        "Remove all card-side OracleFieldLocatorV1 values from an assignment.",
    ),
    ("UNKNOWN_REQUIREMENT_FAMILY_REJECTED", "Add an ID absent from the catalog."),
    ("SUPERSEDED_FAMILY_ASSIGNED_REJECTED", "Assign a family with status SUPERSEDED."),
    (
        "ACTIVE_UNASSIGNED_FAMILY_ASSIGNED_REJECTED",
        "Assign a family with status ACTIVE_UNASSIGNED.",
    ),
    ("RETIRED_FAMILY_ASSIGNED_REJECTED", "Assign a family with status RETIRED."),
    (
        "SUPERSEDED_WITHOUT_SUCCESSOR_REJECTED",
        "Remove every catalog-level target from a SUPERSEDED family.",
    ),
    ("RETIRED_WITH_SUCCESSOR_REJECTED", "Add a target to a RETIRED family."),
    ("SUPERSESSION_UNKNOWN_TARGET_REJECTED", "Point superseded_by at an absent family ID."),
    ("SUPERSESSION_SELF_TARGET_REJECTED", "Point superseded_by at the same family ID."),
    ("SUPERSESSION_NONASSIGNABLE_TARGET_REJECTED", "Point superseded_by at SUPERSEDED or RETIRED."),
    ("HISTORICAL_FAMILY_MISSING_REJECTED", "Remove one of the 216 legacy records."),
    (
        "HISTORICAL_REV3_BLOCK_TAMPER_REJECTED",
        "Change a preserved historical REV3 field or typed checksum.",
    ),
    (
        "HISTORICAL_DEFINITION_PROJECTION_MISMATCH_REJECTED",
        "Change a recomputed historical definition projection.",
    ),
    ("ACTIVE_WITH_ZERO_ASSIGNMENTS_REJECTED", "Leave a zero-usage family ACTIVE."),
    (
        "SPECULATIVE_NEW_FAMILY_REJECTED",
        "Add a new family with no terminal use and no supersession-target need.",
    ),
    (
        "SILENT_CLASSIFICATION_CHANGE_REJECTED",
        "Change an assignment while leaving changes[] unchanged.",
    ),
    (
        "CORRECTION_WITHOUT_RATIONALE_REJECTED",
        "Remove rationale from an added, removed, or superseded change.",
    ),
    ("CORRECTION_WITHOUT_EVIDENCE_REJECTED", "Remove evidence from a changed assignment/change."),
    (
        "NEW_FAMILY_HISTORICAL_BLOCK_PRESENT_REJECTED",
        "Add historical_rev3 or historical_definition to a B2_NEW family.",
    ),
    ("WRONG_CLASSIFICATION_SCHEMA_REJECTED", "Replace the classification schema ID."),
    ("WRONG_CLOSURE_SCHEMA_REJECTED", "Replace the closure schema ID."),
    (
        "EVIDENCE_DIGEST_TAMPER_REJECTED",
        "Change a raw or normalized source digest without rebinding.",
    ),
    ("B2_FILE_INVENTORY_REJECTED", "Add an unrecognized file under closures/B2/."),
    ("OTHER_GATE_PROMOTION_REJECTED", "Promote interaction or citation status to PASS."),
    ("DECK_LOCK_PROMOTION_REJECTED", "Set DECK_PAIR_LOCKED to YES."),
    ("M3_PROMOTION_REJECTED", "Set M3_STARTED to YES."),
)

HEX64_RE = re.compile(r"^[0-9a-f]{64}$")
UUID_RE = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")


class B2CheckError(Exception):
    def __init__(self, status: str, code: str, message: str) -> None:
        super().__init__(f"{status} {code}: {message}")
        self.status = status
        self.code = code
        self.message = message


def fail(code: str, message: str) -> NoReturn:
    raise B2CheckError("FAIL", code, message)


def blocked(code: str, message: str) -> NoReturn:
    raise B2CheckError("BLOCKED", code, message)


def require(condition: bool, code: str, message: str) -> None:
    if not condition:
        fail(code, message)


def require_mapping(value: object, code: str, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        fail(code, f"{label} must be a JSON object")
    return value


def require_list(value: object, code: str, label: str) -> list[Any]:
    if not isinstance(value, list):
        fail(code, f"{label} must be a JSON array")
    return value


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json_bytes(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def canonical_json_digest(value: object) -> str:
    return sha256_bytes(canonical_json_bytes(value))


def write_json_bytes(value: object) -> bytes:
    return canonical_json_bytes(value) + b"\n"


def require_hex(value: object, label: str) -> str:
    if not isinstance(value, str) or HEX64_RE.fullmatch(value) is None:
        fail("INVALID_DIGEST_REFERENCE", f"{label} is not lowercase 64-hex")
    return value


def require_uuid(value: object, label: str) -> str:
    if not isinstance(value, str) or UUID_RE.fullmatch(value) is None:
        fail("INVALID_ORACLE_IDENTITY", f"{label} is not a lowercase UUID")
    return value


def require_text(value: object, code: str, label: str) -> str:
    if not isinstance(value, str) or not value:
        fail(code, f"{label} must be nonempty text")
    return value


def sort_by_cbor(values: list[Any]) -> list[Any]:
    return sorted(values, key=lambda value: encode_canonical(value))


def assert_canonical_set(values: list[Any], label: str, code: str) -> None:
    keys = [encode_canonical(value) for value in values]
    if len(set(keys)) != len(keys):
        fail(code, f"{label} contains duplicate semantic keys")
    if keys != sorted(keys):
        fail(code, f"{label} is not in canonical semantic-key order")


def digest_reference_json(
    semantic_domain: str,
    input_schema_id: str,
    digest_bytes: bytes,
) -> dict[str, str]:
    if len(digest_bytes) != 32:
        fail("INVALID_DIGEST_REFERENCE", "digest reference must contain exactly 32 bytes")
    return {
        "envelope_id": "mtgml.digest-envelope.v1",
        "algorithm_id": "sha-256",
        "semantic_domain": semantic_domain,
        "payload_codec_id": "mtgml.canonical-cbor.v1",
        "input_schema_id": input_schema_id,
        "digest_hex": digest_bytes.hex(),
    }


def digest_reference_cbor(reference: object, label: str) -> list[Any]:
    value = require_mapping(reference, "INVALID_DIGEST_REFERENCE", label)
    expected_keys = {
        "envelope_id",
        "algorithm_id",
        "semantic_domain",
        "payload_codec_id",
        "input_schema_id",
        "digest_hex",
    }
    require(set(value) == expected_keys, "INVALID_DIGEST_REFERENCE", f"{label} fields are closed")
    require(
        value.get("envelope_id") == "mtgml.digest-envelope.v1",
        "INVALID_DIGEST_REFERENCE",
        f"{label}.envelope_id is not the accepted envelope",
    )
    require(
        value.get("algorithm_id") == "sha-256",
        "INVALID_DIGEST_REFERENCE",
        f"{label}.algorithm_id is not sha-256",
    )
    require(
        value.get("payload_codec_id") == "mtgml.canonical-cbor.v1",
        "INVALID_DIGEST_REFERENCE",
        f"{label}.payload_codec_id is not canonical CBOR",
    )
    require_text(
        value.get("semantic_domain"), "INVALID_DIGEST_REFERENCE", f"{label}.semantic_domain"
    )
    require_text(
        value.get("input_schema_id"), "INVALID_DIGEST_REFERENCE", f"{label}.input_schema_id"
    )
    digest = require_hex(value.get("digest_hex"), f"{label}.digest_hex")
    return [
        value["envelope_id"],
        value["algorithm_id"],
        value["semantic_domain"],
        value["payload_codec_id"],
        value["input_schema_id"],
        bytes.fromhex(digest),
    ]


def digest_bytes(reference: object, label: str) -> bytes:
    value = digest_reference_cbor(reference, label)
    return value[-1]


def digest_for_input(
    semantic_domain: str,
    input_schema_id: str,
    payload: list[Any],
) -> dict[str, str]:
    canonical_payload = encode_canonical(payload)
    envelope = encode_envelope(semantic_domain, input_schema_id, canonical_payload)
    return digest_reference_json(
        semantic_domain, input_schema_id, hashlib.sha256(envelope).digest()
    )


def field_value_digest(value: object) -> str:
    if isinstance(value, str):
        return sha256_bytes(value.encode("utf-8"))
    return canonical_json_digest(value)


def variant(value: str) -> list[Any]:
    return [value.lower(), None]


def review_status_variant(value: str) -> list[Any]:
    mapping = {
        "REVIEWED_CONFIRMED": "reviewed_confirmed",
        "REVIEWED_CORRECTED": "reviewed_corrected",
    }
    if value not in mapping:
        fail("NONTERMINAL_CLASSIFICATION_REJECTED", f"unknown terminal review status {value!r}")
    return variant(mapping[value])


def evidence_basis_variant(value: str) -> list[Any]:
    allowed = {
        "ORACLE_TEXT",
        "TYPE_LINE",
        "CARD_FACE",
        "STRUCTURAL_CARD_PROPERTY",
        "FORMAT_POLICY",
        "RULE_DERIVED",
    }
    if value not in allowed:
        fail("EVIDENCE_BASIS_LOCATOR_KIND_MISMATCH_REJECTED", f"unknown evidence basis {value!r}")
    return variant(value)


def change_kind_variant(value: str) -> list[Any]:
    allowed = {"RETAINED", "ADDED", "REMOVED", "SUPERSEDED"}
    if value not in allowed:
        fail("SILENT_CLASSIFICATION_CHANGE_REJECTED", f"unknown change kind {value!r}")
    return variant(value)


def locator_variant_id(locator: object, label: str) -> str:
    value = require_mapping(locator, "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", label)
    locator_version = value.get("locator_version")
    mapping = {
        ORACLE_LOCATOR_SCHEMA: "oracle_field",
        AUTHORITY_LOCATOR_SCHEMA: "authority_byte_fragment",
        RULE_LOCATOR_SCHEMA: "comprehensive_rule",
    }
    if locator_version not in mapping:
        fail("SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", f"{label} has unknown locator_version")
    return mapping[locator_version]


def locator_to_cbor(locator: object, label: str) -> list[Any]:
    value = require_mapping(locator, "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", label)
    kind = locator_variant_id(value, label)
    if kind == "oracle_field":
        expected = {
            "locator_version",
            "archive_artifact",
            "oracle_source_record_id",
            "raw_line_sha256",
            "json_pointer",
            "field_value_sha256",
        }
        require(
            set(value) == expected,
            "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
            f"{label} fields are closed",
        )
        payload = [
            ORACLE_LOCATOR_SCHEMA,
            value["archive_artifact"],
            value["oracle_source_record_id"],
            bytes.fromhex(require_hex(value["raw_line_sha256"], f"{label}.raw_line_sha256")),
            value["json_pointer"],
            bytes.fromhex(require_hex(value["field_value_sha256"], f"{label}.field_value_sha256")),
        ]
    elif kind == "authority_byte_fragment":
        expected = {
            "locator_version",
            "archive_artifact",
            "artifact_sha256",
            "byte_offset",
            "byte_length",
            "fragment_sha256",
        }
        require(
            set(value) == expected,
            "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
            f"{label} fields are closed",
        )
        payload = [
            AUTHORITY_LOCATOR_SCHEMA,
            value["archive_artifact"],
            bytes.fromhex(require_hex(value["artifact_sha256"], f"{label}.artifact_sha256")),
            value["byte_offset"],
            value["byte_length"],
            bytes.fromhex(require_hex(value["fragment_sha256"], f"{label}.fragment_sha256")),
        ]
    else:
        expected = {
            "locator_version",
            "archive_artifact",
            "artifact_sha256",
            "rule_identifier",
            "line_number",
            "line_sha256",
        }
        require(
            set(value) == expected,
            "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
            f"{label} fields are closed",
        )
        payload = [
            RULE_LOCATOR_SCHEMA,
            value["archive_artifact"],
            bytes.fromhex(require_hex(value["artifact_sha256"], f"{label}.artifact_sha256")),
            value["rule_identifier"],
            value["line_number"],
            bytes.fromhex(require_hex(value["line_sha256"], f"{label}.line_sha256")),
        ]
    return [kind, payload]


def locator_sort_key(locator: object, label: str) -> bytes:
    return encode_canonical(locator_to_cbor(locator, label))


def normalize_locators(locators: object, label: str) -> list[dict[str, Any]]:
    values = require_list(locators, "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", label)
    keys = [locator_sort_key(value, f"{label}[{index}]") for index, value in enumerate(values)]
    if len(set(keys)) != len(keys):
        fail("SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", f"{label} contains duplicate locator keys")
    if keys != sorted(keys):
        fail("SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", f"{label} is not canonically ordered")
    return [
        require_mapping(value, "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", label)
        for value in values
    ]


def source_identity_input(source: dict[str, Any]) -> list[Any]:
    return [
        SOURCE_INPUT_SCHEMA,
        source["archive_artifact"],
        source["oracle_semantic_identity"],
        source["oracle_source_record_id"],
        source["oracle_layout"],
        bytes.fromhex(require_hex(source["source_record_raw_sha256"], "source raw digest")),
        bytes.fromhex(require_hex(source["normalized_record_sha256"], "source normalized digest")),
    ]


def rev3_identity_input(record: dict[str, Any]) -> list[Any]:
    return [
        REV3_INPUT_SCHEMA,
        record["card_name"],
        record["card_specific_interaction_trigger"],
        record["classification_drift"],
        record["classification_provenance"],
        record["classification_tier"],
        record["decision_surface"],
        record["higher_order_interaction_trigger"],
        record["identity_surface"],
        record["information_surface"],
        record["oracle_semantic_identity"],
        record["provenance_complete"],
        record["provisional_role"],
        record["ranking_eligible"],
        sort_by_cbor(record["requirement_ids"]),
        record["risk_score_0_10"],
        record["risk_tags"],
        sort_by_cbor(record["source_deck_row_ids"]),
        record["terminal_review_status"],
    ]


def assignment_input(assignment: dict[str, Any]) -> list[Any]:
    return [
        assignment["requirement_family_id"],
        evidence_basis_variant(assignment["evidence_basis"]),
        [
            locator_to_cbor(x, "assignment.evidence_locators")
            for x in assignment["evidence_locators"]
        ],
        assignment["review_rationale"],
    ]


def change_input(change: dict[str, Any]) -> list[Any]:
    return [
        change["family_id"],
        change_kind_variant(change["change_kind"]),
        sort_by_cbor(change["replacement_family_ids"]),
        change["rationale"],
        [locator_to_cbor(x, "change.evidence_locators") for x in change["evidence_locators"]],
    ]


def classification_identity_input(record: dict[str, Any]) -> list[Any]:
    review_basis = require_mapping(record["review_basis"], "INVALID_REVIEW_BASIS", "review_basis")
    provenance = require_mapping(record["provenance"], "INVALID_PROVENANCE", "provenance")
    review_locators = normalize_locators(
        review_basis.get("evidence_locators"), "review_basis.evidence_locators"
    )
    provenance_digest = bytes.fromhex(
        require_hex(provenance.get("source_package_sha256"), "provenance source package")
    )
    return [
        CLASSIFICATION_INPUT_SCHEMA,
        record["oracle_semantic_identity"],
        digest_bytes(record["source_evidence_digest"], "source_evidence_digest"),
        review_status_variant(record["review_status"]),
        digest_bytes(
            record["previous_rev3_classification_identity"],
            "previous_rev3_classification_identity",
        ),
        [assignment_input(x) for x in record["requirement_assignments"]],
        [change_input(x) for x in record["classification_delta"]["changes"]],
        [
            REVIEW_BASIS_SCHEMA,
            variant(str(review_basis["review_method"]).lower()),
            [locator_to_cbor(x, "review_basis.evidence_locators") for x in review_locators],
        ],
        [
            PROVENANCE_SCHEMA,
            provenance_digest,
            variant(str(provenance["provenance_method"]).lower()),
        ],
    ]


def expected_identity(record: dict[str, Any]) -> dict[str, str]:
    return digest_for_input(
        CLASSIFICATION_DOMAIN,
        CLASSIFICATION_INPUT_SCHEMA,
        classification_identity_input(record),
    )


def expected_source_digest(source: dict[str, Any]) -> dict[str, str]:
    return digest_for_input(SOURCE_DOMAIN, SOURCE_INPUT_SCHEMA, source_identity_input(source))


def expected_rev3_digest(record: dict[str, Any]) -> dict[str, str]:
    return digest_for_input(REV3_DOMAIN, REV3_INPUT_SCHEMA, rev3_identity_input(record))


def json_pointer_get(record: object, pointer: str) -> object:
    if not isinstance(pointer, str) or not pointer.startswith("/"):
        fail("SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", f"invalid JSON pointer {pointer!r}")
    value = record
    for raw_part in pointer[1:].split("/"):
        part = raw_part.replace("~1", "/").replace("~0", "~")
        if isinstance(value, dict) and part in value:
            value = value[part]
        elif isinstance(value, list) and part.isdigit() and int(part) < len(value):
            value = value[int(part)]
        else:
            fail(
                "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
                f"JSON pointer does not resolve: {pointer}",
            )
    return value


class ArchiveData:
    def __init__(self) -> None:
        self.archive_path: Path
        self.zip_bytes: bytes
        self.manifest: dict[str, Any]
        self.member_bytes: dict[str, bytes] = {}
        self.evidence: list[dict[str, Any]] = []
        self.evidence_by_osi: dict[str, dict[str, Any]] = {}
        self.raw_by_record_id: dict[str, tuple[dict[str, Any], bytes, str]] = {}
        self.rev3_classifications: list[dict[str, Any]] = []
        self.rev3_by_osi: dict[str, dict[str, Any]] = {}
        self.rev3_families: list[dict[str, Any]] = []
        self.rev3_family_by_id: dict[str, dict[str, Any]] = {}
        self.deck_rows: list[dict[str, Any]] = []
        self.deck_rows_by_id: dict[str, dict[str, Any]] = {}
        self.deck_refs: list[dict[str, Any]] = []
        self.map_rows: list[dict[str, Any]] = []


def archive_path() -> Path:
    base = os.environ.get(ARCHIVE_ENV_VAR)
    if not base:
        blocked(
            "ARCHIVE_SOURCE_UNAVAILABLE",
            f"environment variable {ARCHIVE_ENV_VAR} is unset",
        )
    path = Path(base) / ARCHIVE_RELATIVE_PATH
    if not path.is_file():
        blocked("ARCHIVE_SOURCE_UNAVAILABLE", f"pinned archive is missing: {path}")
    actual = sha256_file(path)
    if actual != EXPECTED_ARCHIVE_SHA256:
        fail("ARCHIVE_DIGEST_MISMATCH", f"{path} has {actual}, expected {EXPECTED_ARCHIVE_SHA256}")
    return path


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as exc:
        blocked("ARCHIVE_SOURCE_UNAVAILABLE", f"cannot read {path}: {exc}")
    return digest.hexdigest()


def load_json_member(data: ArchiveData, name: str) -> Any:
    try:
        return json.loads(data.member_bytes[name].decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        fail("ARCHIVE_MEMBER_INVALID", f"{name} is not valid UTF-8 JSON: {exc}")


def load_csv_member(data: ArchiveData, name: str) -> list[dict[str, str]]:
    try:
        return list(csv.DictReader(io.StringIO(data.member_bytes[name].decode("utf-8"))))
    except UnicodeDecodeError as exc:
        fail("ARCHIVE_MEMBER_INVALID", f"{name} is not UTF-8 CSV: {exc}")


def load_archive() -> ArchiveData:
    path = archive_path()
    data = ArchiveData()
    data.archive_path = path
    try:
        data.zip_bytes = path.read_bytes()
        with zipfile.ZipFile(io.BytesIO(data.zip_bytes)) as archive:
            names = [info.filename for info in archive.infolist()]
            if len(names) != len(set(names)):
                fail("ARCHIVE_MEMBER_INVALID", "ZIP contains duplicate member names")
            if any(name.replace("\\", "/") != name for name in names):
                fail("ARCHIVE_MEMBER_INVALID", "ZIP contains non-normalized member paths")
            manifest_name = "Manafold_M2_5_Package_Manifest_REV3.json"
            if manifest_name not in names:
                fail("ARCHIVE_MEMBER_INVALID", "package manifest is absent")
            manifest = json.loads(archive.read(manifest_name).decode("utf-8"))
            data.manifest = require_mapping(manifest, "ARCHIVE_MEMBER_INVALID", "package manifest")
            entries = require_list(
                data.manifest.get("entries"), "ARCHIVE_MEMBER_INVALID", "manifest.entries"
            )
            require(
                len(entries) == 72, "ARCHIVE_MEMBER_INVALID", "manifest must declare 72 entries"
            )
            declared: dict[str, dict[str, Any]] = {}
            for entry in entries:
                record = require_mapping(entry, "ARCHIVE_MEMBER_INVALID", "manifest entry")
                member = require_text(
                    record.get("path"), "ARCHIVE_MEMBER_INVALID", "manifest entry path"
                )
                if member in declared:
                    fail("ARCHIVE_MEMBER_INVALID", f"duplicate manifest path {member}")
                declared[member] = record
                if member not in names:
                    fail("ARCHIVE_MEMBER_INVALID", f"manifest member missing from ZIP: {member}")
                payload = archive.read(member)
                expected_bytes = record.get("bytes")
                expected_sha = record.get("sha256")
                require(
                    isinstance(expected_bytes, int),
                    "ARCHIVE_MEMBER_INVALID",
                    f"invalid bytes for {member}",
                )
                require_file_digest(expected_sha, f"manifest digest {member}")
                if len(payload) != expected_bytes or sha256_bytes(payload) != expected_sha:
                    fail("ARCHIVE_MEMBER_INVALID", f"manifest digest mismatch for {member}")
            excluded = require_list(
                data.manifest.get("manifest_excluded_paths"),
                "ARCHIVE_MEMBER_INVALID",
                "manifest_excluded_paths",
            )
            expected_extras = {manifest_name, *[str(x) for x in excluded]}
            if set(names) - set(declared) != expected_extras:
                fail(
                    "ARCHIVE_MEMBER_INVALID",
                    "ZIP members outside the manifest are not exactly the declared exclusions",
                )
            for member in REQUIRED_ARCHIVE_MEMBERS:
                if member not in names:
                    fail("ARCHIVE_MEMBER_INVALID", f"required archive member missing: {member}")
                data.member_bytes[member] = archive.read(member)
            for member in AUTHORITY_MEMBERS:
                if member in names:
                    data.member_bytes[member] = archive.read(member)
    except zipfile.BadZipFile as exc:
        fail("ARCHIVE_MEMBER_INVALID", f"invalid ZIP: {exc}")
    except OSError as exc:
        blocked("ARCHIVE_SOURCE_UNAVAILABLE", f"cannot open pinned archive: {exc}")

    data.evidence = require_list(
        load_json_member(data, "inputs/oracle_semantic_evidence_REV3.json"),
        "ARCHIVE_MEMBER_INVALID",
        "oracle evidence",
    )
    data.evidence_by_osi = {}
    for record in data.evidence:
        record = require_mapping(record, "ARCHIVE_MEMBER_INVALID", "oracle evidence record")
        osi = require_uuid(record.get("oracle_semantic_identity"), "oracle evidence OSI")
        if osi in data.evidence_by_osi:
            fail("ARCHIVE_MEMBER_INVALID", f"duplicate evidence OSI {osi}")
        data.evidence_by_osi[osi] = record
    require(
        len(data.evidence) == 402,
        "ARCHIVE_UNIVERSE_MISMATCH",
        "pinned evidence must contain 402 OSIs",
    )

    raw_bytes = data.member_bytes[RAW_ORACLE_ARTIFACT]
    lines = raw_bytes.splitlines(keepends=True)
    require(
        len(lines) == 402 and raw_bytes.endswith(b"\n"),
        "ARCHIVE_MEMBER_INVALID",
        "raw Oracle JSONL must contain 402 newline-terminated records",
    )
    for line in lines:
        if not line.endswith(b"\n"):
            fail("ARCHIVE_MEMBER_INVALID", "raw Oracle JSONL record is missing its final newline")
        try:
            record = json.loads(line.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            fail("ARCHIVE_MEMBER_INVALID", f"raw Oracle JSONL record is invalid: {exc}")
        record = require_mapping(record, "ARCHIVE_MEMBER_INVALID", "raw Oracle record")
        source_id = require_uuid(record.get("id"), "raw Oracle record id")
        if source_id in data.raw_by_record_id:
            fail("ARCHIVE_MEMBER_INVALID", f"duplicate raw Oracle record id {source_id}")
        data.raw_by_record_id[source_id] = (record, line, sha256_bytes(line))
    for evidence in data.evidence:
        source_id = require_uuid(
            evidence.get("oracle_source_record_id"), "evidence source record id"
        )
        if source_id not in data.raw_by_record_id:
            fail(
                "ARCHIVE_MEMBER_INVALID",
                f"evidence source record missing from raw JSONL: {source_id}",
            )
        _, _, raw_digest = data.raw_by_record_id[source_id]
        if raw_digest != evidence.get("source_record_raw_sha256"):
            fail("ARCHIVE_MEMBER_INVALID", f"raw digest mismatch for source record {source_id}")

    data.rev3_classifications = require_list(
        load_json_member(data, "inputs/card_semantic_classification_REV3.json"),
        "ARCHIVE_MEMBER_INVALID",
        "REV3 classifications",
    )
    for record in data.rev3_classifications:
        record = require_mapping(record, "ARCHIVE_MEMBER_INVALID", "REV3 classification")
        osi = require_uuid(record.get("oracle_semantic_identity"), "REV3 classification OSI")
        if osi in data.rev3_by_osi:
            fail("ARCHIVE_MEMBER_INVALID", f"duplicate REV3 classification OSI {osi}")
        data.rev3_by_osi[osi] = record
    require(
        len(data.rev3_classifications) == 402,
        "ARCHIVE_UNIVERSE_MISMATCH",
        "REV3 classifications must contain 402 OSIs",
    )
    require(
        set(data.rev3_by_osi) == set(data.evidence_by_osi),
        "ARCHIVE_UNIVERSE_MISMATCH",
        "evidence and REV3 classification OSI sets differ",
    )

    data.rev3_families = require_list(
        load_json_member(data, "inputs/requirement_family_catalog_REV3.json"),
        "ARCHIVE_MEMBER_INVALID",
        "REV3 family catalog",
    )
    for family in data.rev3_families:
        family = require_mapping(family, "ARCHIVE_MEMBER_INVALID", "REV3 family")
        family_id = require_text(family.get("id"), "ARCHIVE_MEMBER_INVALID", "REV3 family id")
        if family_id in data.rev3_family_by_id:
            fail("ARCHIVE_MEMBER_INVALID", f"duplicate REV3 family id {family_id}")
        data.rev3_family_by_id[family_id] = family
    require(
        len(data.rev3_families) == 216,
        "ARCHIVE_UNIVERSE_MISMATCH",
        "REV3 catalog must contain 216 families",
    )

    data.deck_rows = load_csv_member(data, "inputs/deck_row_source_resolution_REV3.csv")
    data.deck_rows_by_id = {row["deck_row_id"]: row for row in data.deck_rows}
    require(
        len(data.deck_rows) == 441 and len(data.deck_rows_by_id) == 441,
        "ARCHIVE_UNIVERSE_MISMATCH",
        "pinned deck resolution must contain 441 unique rows",
    )
    data.deck_refs = load_csv_member(data, "inputs/deck_row_classification_refs_REV3.csv")
    require(
        len(data.deck_refs) == 441,
        "ARCHIVE_UNIVERSE_MISMATCH",
        "pinned deck classification refs must contain 441 rows",
    )
    data.map_rows = load_csv_member(data, "derived/Card_Requirement_Map_REV3.csv")
    require(
        len(data.map_rows) == 470,
        "ARCHIVE_UNIVERSE_MISMATCH",
        "pinned requirement map must contain 470 rows",
    )

    repeated = Counter(row["oracle_semantic_identity"] for row in data.deck_rows)
    require(
        sum(count > 1 for count in repeated.values()) == 23,
        "ARCHIVE_UNIVERSE_MISMATCH",
        "pinned deck rows must contain 23 reused OSIs",
    )
    require(
        sum(count - 1 for count in repeated.values() if count > 1) == 39,
        "ARCHIVE_UNIVERSE_MISMATCH",
        "pinned deck rows must contain 39 additional reused references",
    )
    total_quantity = sum(int(row["quantity"]) for row in data.deck_rows)
    require(total_quantity == 600, "ARCHIVE_UNIVERSE_MISMATCH", "pinned deck quantity must be 600")
    return data


def require_file_digest(value: object, label: str) -> str:
    if not isinstance(value, str) or HEX64_RE.fullmatch(value) is None:
        fail("ARCHIVE_MEMBER_INVALID", f"{label} is not a lowercase 64-hex digest")
    return value


def source_for_osi(data: ArchiveData, osi: str) -> dict[str, Any]:
    if osi not in data.evidence_by_osi:
        fail("UNKNOWN_ORACLE_IDENTITY_REJECTED", f"unknown OracleSemanticIdentity {osi}")
    evidence = data.evidence_by_osi[osi]
    source_id = require_uuid(evidence.get("oracle_source_record_id"), "source record id")
    raw_record, raw_line, raw_digest = data.raw_by_record_id[source_id]
    if raw_digest != evidence.get("source_record_raw_sha256"):
        fail("SOURCE_DIGEST_MISMATCH_REJECTED", f"pinned raw digest differs for {osi}")
    return {
        "archive_artifact": RAW_ORACLE_ARTIFACT,
        "oracle_semantic_identity": osi,
        "oracle_source_record_id": source_id,
        "oracle_layout": evidence.get("oracle_layout"),
        "source_record_raw_sha256": evidence.get("source_record_raw_sha256"),
        "normalized_record_sha256": evidence.get("normalized_record_sha256"),
        "_raw_record": raw_record,
        "_raw_line": raw_line,
    }


def expected_member_osis(data: ArchiveData, family_id: str) -> list[str]:
    return sorted(
        [
            record["oracle_semantic_identity"]
            for record in data.rev3_classifications
            if family_id in record.get("requirement_ids", [])
        ]
    )


def typed_checksum(value: object, schema: str) -> dict[str, str]:
    return {
        "checksum_kind": "EVIDENCE_CHECKSUM",
        "algorithm_id": "sha-256",
        "input_schema_id": schema,
        "digest_hex": canonical_json_digest(value),
    }


def expected_historical_definition(
    data: ArchiveData,
    family: dict[str, Any],
    member_osis: list[str],
) -> dict[str, Any]:
    assignment_context = {
        "member_osi": member_osis,
        "member_card_names": [data.rev3_by_osi[osi]["card_name"] for osi in member_osis],
        "assignment_record_digests": [
            {
                "oracle_semantic_identity": osi,
                "digest_hex": expected_rev3_digest(data.rev3_by_osi[osi])["digest_hex"],
            }
            for osi in member_osis
        ],
    }
    projection = {
        "rev3_name": family["name"],
        "rev3_description": family["description"],
        "rev3_criteria": family["classification_criteria"],
        "assignment_context": assignment_context,
    }
    return {
        **projection,
        "projection_sha256": typed_checksum(
            projection,
            "manafold.m2.5.b2.historical-definition-projection-evidence.v1",
        ),
    }


def expected_historical_block(
    data: ArchiveData,
    family: dict[str, Any],
    member_osis: list[str],
) -> dict[str, Any]:
    record = copy.deepcopy(family)
    return {
        "record": record,
        "record_sha256": typed_checksum(
            record,
            "manafold.m2.5.b2.rev3-record-evidence.v1",
        ),
        "member_osi": member_osis,
        "assignment_record_digests": [
            {
                "oracle_semantic_identity": osi,
                "digest_hex": expected_rev3_digest(data.rev3_by_osi[osi])["digest_hex"],
            }
            for osi in member_osis
        ],
    }


def validate_typed_checksum(value: object, expected_value: object, schema: str, label: str) -> None:
    checksum = require_mapping(value, "HISTORICAL_REV3_BLOCK_TAMPER_REJECTED", label)
    expected_keys = {"checksum_kind", "algorithm_id", "input_schema_id", "digest_hex"}
    if (
        set(checksum) != expected_keys
        or checksum.get("checksum_kind") != "EVIDENCE_CHECKSUM"
        or checksum.get("algorithm_id") != "sha-256"
        or checksum.get("input_schema_id") != schema
        or checksum.get("digest_hex") != canonical_json_digest(expected_value)
    ):
        fail("HISTORICAL_REV3_BLOCK_TAMPER_REJECTED", f"{label} is not the expected typed checksum")


def validate_spec(data: ArchiveData, artifacts: dict[str, Any]) -> None:
    path = B2_DIR / "B2_DESIGN_SPEC.md"
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        fail("B2_FILE_INVENTORY_REJECTED", f"cannot read B2 design spec: {exc}")
    require(
        "DigestReferenceJsonV1" in text,
        "B2_FILE_INVENTORY_REJECTED",
        "B2 spec omits DigestReferenceJsonV1",
    )
    require(
        "NEW_FAMILY_HISTORICAL_BLOCK_PRESENT_REJECTED" in text,
        "B2_FILE_INVENTORY_REJECTED",
        "B2 spec omits the executable new-family historical-block case",
    )
    require(
        "LEGACY_FAMILY_REINTERPRETATION_REJECTED" not in text,
        "B2_FILE_INVENTORY_REJECTED",
        "B2 spec retains the non-executable legacy reinterpretation case",
    )
    del artifacts


def validate_locator(
    data: ArchiveData,
    locator: object,
    source: dict[str, Any],
    basis: str | None,
    label: str,
) -> str:
    value = require_mapping(locator, "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", label)
    kind = locator_variant_id(value, label)
    if kind == "oracle_field":
        expected_keys = {
            "locator_version",
            "archive_artifact",
            "oracle_source_record_id",
            "raw_line_sha256",
            "json_pointer",
            "field_value_sha256",
        }
        require(
            set(value) == expected_keys,
            "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
            f"{label} fields are not closed",
        )
        if (
            value.get("archive_artifact") != RAW_ORACLE_ARTIFACT
            or value.get("oracle_source_record_id") != source["oracle_source_record_id"]
        ):
            fail(
                "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
                f"{label} points to a different Oracle record",
            )
        raw_digest = require_hex(value.get("raw_line_sha256"), f"{label}.raw_line_sha256")
        if raw_digest != source["source_record_raw_sha256"]:
            fail(
                "EVIDENCE_DIGEST_TAMPER_REJECTED",
                f"{label} raw-line digest does not match the pinned source",
            )
        pointer = value.get("json_pointer")
        if basis == "ORACLE_TEXT" and pointer != "/oracle_text":
            fail(
                "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", f"{label} is not an oracle_text locator"
            )
        if basis == "TYPE_LINE" and pointer != "/type_line":
            fail("SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", f"{label} is not a type_line locator")
        if basis == "CARD_FACE" and (
            not isinstance(pointer, str) or not pointer.startswith("/oracle_faces/")
        ):
            fail("SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", f"{label} is not a card-face locator")
        selected = json_pointer_get(source["_raw_record"], pointer)
        selected_digest = require_hex(
            value.get("field_value_sha256"), f"{label}.field_value_sha256"
        )
        if selected_digest != field_value_digest(selected):
            fail(
                "EVIDENCE_DIGEST_TAMPER_REJECTED",
                f"{label} field-value digest does not match the pinned source",
            )
        return kind
    artifact = value.get("archive_artifact")
    if not isinstance(artifact, str) or artifact not in data.member_bytes:
        fail(
            "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
            f"{label} references an unknown authority artifact",
        )
    actual_artifact_digest = sha256_bytes(data.member_bytes[artifact])
    if value.get("artifact_sha256") != actual_artifact_digest:
        fail("EVIDENCE_DIGEST_TAMPER_REJECTED", f"{label} authority artifact digest is not pinned")
    if kind == "authority_byte_fragment":
        offset = value.get("byte_offset")
        length = value.get("byte_length")
        if (
            isinstance(offset, bool)
            or not isinstance(offset, int)
            or isinstance(length, bool)
            or not isinstance(length, int)
            or offset < 0
            or length < 0
        ):
            fail("SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", f"{label} fragment bounds are invalid")
        fragment = data.member_bytes[artifact][offset : offset + length]
        if len(fragment) != length:
            fail(
                "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
                f"{label} fragment is outside the authority artifact",
            )
        if value.get("fragment_sha256") != sha256_bytes(fragment):
            fail("EVIDENCE_DIGEST_TAMPER_REJECTED", f"{label} fragment digest does not match")
        return kind
    line_number = value.get("line_number")
    if isinstance(line_number, bool) or not isinstance(line_number, int) or line_number < 1:
        fail("SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", f"{label} rule line number is invalid")
    lines = data.member_bytes[artifact].splitlines()
    if line_number > len(lines):
        fail(
            "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
            f"{label} rule line is outside the authority artifact",
        )
    line = lines[line_number - 1]
    if value.get("line_sha256") != sha256_bytes(line):
        fail("EVIDENCE_DIGEST_TAMPER_REJECTED", f"{label} rule line digest does not match")
    identifier = value.get("rule_identifier")
    if not isinstance(identifier, str) or identifier not in line.decode("utf-8", errors="replace"):
        fail(
            "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
            f"{label} rule identifier does not resolve to the pinned line",
        )
    return kind


def validate_locator_list(
    data: ArchiveData,
    value: object,
    source: dict[str, Any],
    basis: str | None,
    label: str,
    require_nonempty: bool = True,
) -> list[dict[str, Any]]:
    locators = require_list(value, "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", label)
    if require_nonempty and not locators:
        fail("CORRECTION_WITHOUT_EVIDENCE_REJECTED", f"{label} is empty")
    kinds = [
        validate_locator(data, locator, source, basis, f"{label}[{index}]")
        for index, locator in enumerate(locators)
    ]
    normalized = normalize_locators(locators, label)
    if "oracle_field" not in kinds:
        return normalized
    return normalized


def validate_review_basis(
    data: ArchiveData, record: dict[str, Any], source: dict[str, Any]
) -> None:
    value = require_mapping(record.get("review_basis"), "INVALID_REVIEW_BASIS", "review_basis")
    require(
        set(value) == {"review_method", "evidence_locators"},
        "INVALID_REVIEW_BASIS",
        "review_basis fields are not closed",
    )
    require(
        value.get("review_method") == "SOURCE_GROUNDED_CARD_REVIEW",
        "INVALID_REVIEW_BASIS",
        "review method is not source-grounded",
    )
    locators = validate_locator_list(
        data, value.get("evidence_locators"), source, None, "review_basis.evidence_locators"
    )
    if not any(
        locator_variant_id(locator, "review basis locator") == "oracle_field"
        for locator in locators
    ):
        fail("CARD_SIDE_EVIDENCE_MISSING_REJECTED", "review_basis has no card-side locator")


def validate_provenance(record: dict[str, Any]) -> None:
    value = require_mapping(record.get("provenance"), "INVALID_PROVENANCE", "provenance")
    require(
        set(value) == {"source_package_sha256", "provenance_method"},
        "INVALID_PROVENANCE",
        "provenance fields are not closed",
    )
    require(
        value.get("source_package_sha256") == EXPECTED_ARCHIVE_SHA256,
        "INVALID_PROVENANCE",
        "provenance package digest is not pinned",
    )
    require(
        value.get("provenance_method") == "SOURCE_GROUNDED_REVIEW_V1",
        "INVALID_PROVENANCE",
        "provenance method is not source-grounded",
    )


def validate_semantic_boundary(value: object, family_id: str) -> None:
    definition = require_text(
        value,
        "B2_FILE_INVENTORY_REJECTED",
        f"{family_id}.precise_semantic_definition",
    )
    parts = definition.split("|")
    require(
        len(parts) == len(SEMANTIC_BOUNDARY_FIELDS) + 1 and parts[0] == SEMANTIC_BOUNDARY_PREFIX,
        "B2_FILE_INVENTORY_REJECTED",
        f"{family_id}.precise_semantic_definition is not B2_SEMANTIC_BOUNDARY_V1",
    )
    parsed: dict[str, str] = {}
    for expected_field, part in zip(SEMANTIC_BOUNDARY_FIELDS, parts[1:], strict=True):
        key, separator, field_value = part.partition("=")
        require(
            separator == "=" and key == expected_field and field_value,
            "B2_FILE_INVENTORY_REJECTED",
            f"{family_id}.precise_semantic_definition field order or value is invalid",
        )
        require(
            key not in parsed,
            "B2_FILE_INVENTORY_REJECTED",
            f"{family_id}.precise_semantic_definition repeats {key}",
        )
        parsed[key] = field_value
    require(
        parsed["family_id"] == family_id,
        "B2_FILE_INVENTORY_REJECTED",
        f"{family_id}.precise_semantic_definition binds another family",
    )
    require(
        "covers the explicitly evidenced" not in definition
        and "no unstated object, timing, zone" not in definition,
        "B2_FILE_INVENTORY_REJECTED",
        f"{family_id}.precise_semantic_definition is tautological",
    )


def validate_catalog(data: ArchiveData, catalog: object) -> dict[str, dict[str, Any]]:
    value = require_mapping(catalog, "B2_FILE_INVENTORY_REJECTED", "requirement family catalog")
    require(
        value.get("schema") == CATALOG_SCHEMA,
        "B2_FILE_INVENTORY_REJECTED",
        "catalog schema is wrong",
    )
    require(
        value.get("source_package_sha256") == EXPECTED_ARCHIVE_SHA256,
        "B2_FILE_INVENTORY_REJECTED",
        "catalog package digest is wrong",
    )
    require(
        value.get("rev3_catalog_sha256") == canonical_json_digest(data.rev3_families),
        "HISTORICAL_REV3_BLOCK_TAMPER_REJECTED",
        "REV3 catalog digest is wrong",
    )
    families = require_list(value.get("families"), "B2_FILE_INVENTORY_REJECTED", "catalog.families")
    new_count = sum(
        1 for item in families if isinstance(item, dict) and item.get("family_origin") == "B2_NEW"
    )
    require(
        value.get("legacy_family_count") == 216,
        "HISTORICAL_FAMILY_MISSING_REJECTED",
        "legacy family count is not 216",
    )
    require(
        value.get("new_family_count") == new_count,
        "B2_FILE_INVENTORY_REJECTED",
        "new family count is inconsistent",
    )
    require(
        value.get("catalog_family_count") == 216 + new_count,
        "B2_FILE_INVENTORY_REJECTED",
        "catalog family count is inconsistent",
    )
    ids = [
        require_text(item.get("family_id"), "B2_FILE_INVENTORY_REJECTED", "catalog family id")
        for item in families
        if isinstance(item, dict)
    ]
    require(
        ids == sorted(ids, key=lambda item: encode_canonical(item)),
        "B2_FILE_INVENTORY_REJECTED",
        "catalog families are not canonically ordered",
    )
    catalog_by_id: dict[str, dict[str, Any]] = {}
    for family in families:
        family = require_mapping(family, "B2_FILE_INVENTORY_REJECTED", "catalog family")
        family_id = require_text(
            family.get("family_id"), "B2_FILE_INVENTORY_REJECTED", "catalog family id"
        )
        if family_id in catalog_by_id:
            fail("B2_FILE_INVENTORY_REJECTED", f"duplicate catalog family {family_id}")
        catalog_by_id[family_id] = family
        origin = family.get("family_origin")
        status = family.get("status")
        require(
            origin in {"REV3_LEGACY", "B2_NEW"},
            "B2_FILE_INVENTORY_REJECTED",
            f"unknown family origin for {family_id}",
        )
        require(
            status in {"ACTIVE", "ACTIVE_UNASSIGNED", "SUPERSEDED", "RETIRED"},
            "B2_FILE_INVENTORY_REJECTED",
            f"unknown lifecycle status for {family_id}",
        )
        require(
            isinstance(family.get("terminal_assignable"), bool),
            "B2_FILE_INVENTORY_REJECTED",
            f"terminal_assignable missing for {family_id}",
        )
        expected_assignable = status == "ACTIVE"
        require(
            family["terminal_assignable"] == expected_assignable,
            "B2_FILE_INVENTORY_REJECTED",
            f"terminal_assignable disagrees with status for {family_id}",
        )
        targets = require_list(
            family.get("superseded_by"), "B2_FILE_INVENTORY_REJECTED", f"{family_id}.superseded_by"
        )
        assert_canonical_set(targets, f"{family_id}.superseded_by", "B2_FILE_INVENTORY_REJECTED")
        if status == "SUPERSEDED":
            if not targets:
                fail(
                    "SUPERSEDED_WITHOUT_SUCCESSOR_REJECTED",
                    f"{family_id} has no supersession target",
                )
            require_text(
                family.get("supersession_reason"),
                "B2_FILE_INVENTORY_REJECTED",
                f"{family_id}.supersession_reason",
            )
        elif status == "RETIRED" and targets:
            fail("RETIRED_WITH_SUCCESSOR_REJECTED", f"{family_id} is RETIRED but has a target")
        else:
            require(
                family.get("supersession_reason") in {None, ""},
                "B2_FILE_INVENTORY_REJECTED",
                f"{family_id} has an unexpected supersession reason",
            )
        if origin == "REV3_LEGACY":
            require(
                family_id in data.rev3_family_by_id,
                "HISTORICAL_FAMILY_MISSING_REJECTED",
                f"unknown legacy family {family_id}",
            )
            historical = require_mapping(
                family.get("historical_rev3"),
                "HISTORICAL_REV3_BLOCK_TAMPER_REJECTED",
                f"{family_id}.historical_rev3",
            )
            historical_record = historical.get("record")
            require(
                historical_record == data.rev3_family_by_id[family_id],
                "HISTORICAL_REV3_BLOCK_TAMPER_REJECTED",
                f"historical record changed for {family_id}",
            )
            validate_typed_checksum(
                historical.get("record_sha256"),
                historical_record,
                "manafold.m2.5.b2.rev3-record-evidence.v1",
                f"{family_id}.historical_rev3.record_sha256",
            )
            member_osis = expected_member_osis(data, family_id)
            require(
                historical.get("member_osi") == member_osis,
                "HISTORICAL_REV3_BLOCK_TAMPER_REJECTED",
                f"historical member OSI set changed for {family_id}",
            )
            expected_digests = expected_historical_block(
                data, data.rev3_family_by_id[family_id], member_osis
            )["assignment_record_digests"]
            require(
                historical.get("assignment_record_digests") == expected_digests,
                "HISTORICAL_REV3_BLOCK_TAMPER_REJECTED",
                f"historical assignment digests changed for {family_id}",
            )
            definition = require_mapping(
                family.get("historical_definition"),
                "HISTORICAL_DEFINITION_PROJECTION_MISMATCH_REJECTED",
                f"{family_id}.historical_definition",
            )
            expected_definition = expected_historical_definition(
                data, data.rev3_family_by_id[family_id], member_osis
            )
            require(
                definition == expected_definition,
                "HISTORICAL_DEFINITION_PROJECTION_MISMATCH_REJECTED",
                f"historical definition projection changed for {family_id}",
            )
            relation = family.get("lifecycle_relation")
            expected_relation = {
                "ACTIVE": "ACTIVE_EQUIVALENT",
                "ACTIVE_UNASSIGNED": "ACTIVE_EQUIVALENT",
                "SUPERSEDED": "SUPERSEDED_BY_REPLACEMENT",
                "RETIRED": "RETIRED_NO_SUCCESSOR",
            }[status]
            require(
                relation == expected_relation,
                "B2_FILE_INVENTORY_REJECTED",
                f"lifecycle relation disagrees for {family_id}",
            )
        else:
            require(
                re.fullmatch(r"req\.b2\.[a-z0-9]+(?:_[a-z0-9]+)*", family_id) is not None,
                "B2_FILE_INVENTORY_REJECTED",
                f"new family id is not in req.b2 namespace: {family_id}",
            )
            if "historical_rev3" in family or "historical_definition" in family:
                fail(
                    "NEW_FAMILY_HISTORICAL_BLOCK_PRESENT_REJECTED",
                    f"B2_NEW family {family_id} contains a historical block",
                )
            require(
                family.get("lifecycle_relation") == "NEW_TERMINAL_CONCEPT",
                "B2_FILE_INVENTORY_REJECTED",
                f"new family {family_id} has wrong lifecycle relation",
            )
        allowed_basis = require_list(
            family.get("evidence_basis_allowed"),
            "B2_FILE_INVENTORY_REJECTED",
            f"{family_id}.evidence_basis_allowed",
        )
        assert_canonical_set(
            allowed_basis, f"{family_id}.evidence_basis_allowed", "B2_FILE_INVENTORY_REJECTED"
        )
        require(
            all(
                item
                in {
                    "ORACLE_TEXT",
                    "TYPE_LINE",
                    "CARD_FACE",
                    "STRUCTURAL_CARD_PROPERTY",
                    "FORMAT_POLICY",
                    "RULE_DERIVED",
                }
                for item in allowed_basis
            ),
            "B2_FILE_INVENTORY_REJECTED",
            f"unknown evidence basis for {family_id}",
        )
        require_text(
            family.get("canonical_name"),
            "B2_FILE_INVENTORY_REJECTED",
            f"{family_id}.canonical_name",
        )
        validate_semantic_boundary(family.get("precise_semantic_definition"), family_id)
        review = require_mapping(
            family.get("review_provenance"),
            "B2_FILE_INVENTORY_REJECTED",
            f"{family_id}.review_provenance",
        )
        require(
            review.get("review_status") in {"REVIEWED_CONFIRMED", "REVIEWED_CORRECTED"},
            "B2_FILE_INVENTORY_REJECTED",
            f"{family_id} review status is not terminal",
        )
        require(
            review.get("review_basis") == "SOURCE_GROUNDED_CARD_REVIEW",
            "B2_FILE_INVENTORY_REJECTED",
            f"{family_id} review basis is not source-grounded",
        )
        review_locators = require_list(
            review.get("evidence_locators"),
            "B2_FILE_INVENTORY_REJECTED",
            f"{family_id}.review_provenance.evidence_locators",
        )
        if origin == "REV3_LEGACY":
            review_source = source_for_osi(data, expected_member_osis(data, family_id)[0])
            validated_review_locators = validate_locator_list(
                data,
                review_locators,
                review_source,
                None,
                f"{family_id}.review_provenance.evidence_locators",
            )
            require(
                any(
                    locator_variant_id(locator, "family review locator") == "oracle_field"
                    for locator in validated_review_locators
                ),
                "CARD_SIDE_EVIDENCE_MISSING_REJECTED",
                f"{family_id} review provenance has no card-side locator",
            )
    missing = sorted(set(data.rev3_family_by_id) - set(catalog_by_id))
    if missing:
        fail("HISTORICAL_FAMILY_MISSING_REJECTED", f"catalog omits historical family {missing[0]}")
    for family_id, family in catalog_by_id.items():
        for target in family["superseded_by"]:
            if target not in catalog_by_id:
                fail(
                    "SUPERSESSION_UNKNOWN_TARGET_REJECTED",
                    f"{family_id} targets unknown family {target}",
                )
            if target == family_id:
                fail("SUPERSESSION_SELF_TARGET_REJECTED", f"{family_id} targets itself")
            if catalog_by_id[target]["status"] not in {"ACTIVE", "ACTIVE_UNASSIGNED"}:
                fail(
                    "SUPERSESSION_NONASSIGNABLE_TARGET_REJECTED",
                    f"{family_id} targets nonassignable family {target}",
                )

    def visit(node: str, active: set[str], done: set[str]) -> None:
        if node in active:
            fail("B2_FILE_INVENTORY_REJECTED", f"supersession cycle at {node}")
        if node in done:
            return
        active.add(node)
        for target in catalog_by_id[node]["superseded_by"]:
            visit(target, active, done)
        active.remove(node)
        done.add(node)

    done: set[str] = set()
    for family_id in catalog_by_id:
        visit(family_id, set(), done)
    return catalog_by_id


def validate_assignment(
    data: ArchiveData,
    assignment: object,
    catalog: dict[str, dict[str, Any]],
    source: dict[str, Any],
    label: str,
) -> dict[str, Any]:
    value = require_mapping(assignment, "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", label)
    expected_keys = {
        "requirement_family_id",
        "evidence_basis",
        "evidence_locators",
        "review_rationale",
    }
    require(
        set(value) == expected_keys,
        "SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED",
        f"{label} fields are not closed",
    )
    family_id = require_text(
        value.get("requirement_family_id"),
        "UNKNOWN_REQUIREMENT_FAMILY_REJECTED",
        f"{label}.requirement_family_id",
    )
    if family_id not in catalog:
        fail(
            "UNKNOWN_REQUIREMENT_FAMILY_REJECTED", f"{label} references unknown family {family_id}"
        )
    family = catalog[family_id]
    status = family.get("status")
    if status == "SUPERSEDED":
        fail(
            "SUPERSEDED_FAMILY_ASSIGNED_REJECTED", f"{label} assigns SUPERSEDED family {family_id}"
        )
    if status == "ACTIVE_UNASSIGNED":
        fail(
            "ACTIVE_UNASSIGNED_FAMILY_ASSIGNED_REJECTED",
            f"{label} assigns ACTIVE_UNASSIGNED family {family_id}",
        )
    if status == "RETIRED":
        fail("RETIRED_FAMILY_ASSIGNED_REJECTED", f"{label} assigns RETIRED family {family_id}")
    basis = value.get("evidence_basis")
    require(
        basis
        in {
            "ORACLE_TEXT",
            "TYPE_LINE",
            "CARD_FACE",
            "STRUCTURAL_CARD_PROPERTY",
            "FORMAT_POLICY",
            "RULE_DERIVED",
        },
        "EVIDENCE_BASIS_LOCATOR_KIND_MISMATCH_REJECTED",
        f"{label} has unknown evidence basis",
    )
    allowed = family.get("evidence_basis_allowed", [])
    if basis not in allowed:
        fail(
            "DISALLOWED_EVIDENCE_BASIS_REJECTED",
            f"{label} basis {basis} is not allowed for {family_id}",
        )
    rationale = value.get("review_rationale")
    if not isinstance(rationale, str) or not rationale.strip():
        fail("CORRECTION_WITHOUT_RATIONALE_REJECTED", f"{label} has no review rationale")
    locators = require_list(
        value.get("evidence_locators"),
        "CORRECTION_WITHOUT_EVIDENCE_REJECTED",
        f"{label}.evidence_locators",
    )
    if not locators:
        fail("CORRECTION_WITHOUT_EVIDENCE_REJECTED", f"{label} has no evidence locators")
    kinds = [
        validate_locator(data, locator, source, basis, f"{label}.evidence_locators[{index}]")
        for index, locator in enumerate(locators)
    ]
    normalize_locators(locators, f"{label}.evidence_locators")
    if "oracle_field" not in kinds:
        fail(
            "CARD_SIDE_EVIDENCE_MISSING_REJECTED", f"{label} has no card-side OracleFieldLocatorV1"
        )
    if basis in {"ORACLE_TEXT", "TYPE_LINE", "CARD_FACE", "STRUCTURAL_CARD_PROPERTY"}:
        compatible = False
        for locator, kind in zip(locators, kinds, strict=True):
            if kind != "oracle_field":
                continue
            pointer = locator.get("json_pointer")
            compatible = compatible or (
                (basis == "ORACLE_TEXT" and pointer == "/oracle_text")
                or (basis == "TYPE_LINE" and pointer == "/type_line")
                or (
                    basis == "CARD_FACE"
                    and isinstance(pointer, str)
                    and pointer.startswith("/oracle_faces/")
                )
                or (
                    basis == "STRUCTURAL_CARD_PROPERTY"
                    and pointer
                    in {
                        "/layout",
                        "/keywords",
                        "/mana_cost",
                        "/power",
                        "/toughness",
                        "/colors",
                        "/type_line",
                    }
                )
            )
        if not compatible:
            fail(
                "EVIDENCE_BASIS_LOCATOR_KIND_MISMATCH_REJECTED",
                f"{label} has no compatible card-side locator",
            )
    if basis == "FORMAT_POLICY" and "authority_byte_fragment" not in kinds:
        # A card-side locator remains mandatory, but a format basis must also
        # carry the typed format authority it claims to use.
        fail(
            "EVIDENCE_BASIS_LOCATOR_KIND_MISMATCH_REJECTED",
            f"{label} has no format-policy authority locator",
        )
    if basis == "RULE_DERIVED" and not (
        {"comprehensive_rule", "authority_byte_fragment"} & set(kinds)
    ):
        fail(
            "EVIDENCE_BASIS_LOCATOR_KIND_MISMATCH_REJECTED",
            f"{label} has no rule authority locator",
        )
    return value


def expected_summary_arrays(
    rev3_ids: set[str],
    terminal_ids: set[str],
    catalog: dict[str, dict[str, Any]],
) -> dict[str, list[str]]:
    retained = rev3_ids & terminal_ids
    added = terminal_ids - rev3_ids
    removed = {
        family_id
        for family_id in rev3_ids - terminal_ids
        if catalog[family_id]["status"] != "SUPERSEDED"
    }
    superseded = {
        family_id
        for family_id in rev3_ids - terminal_ids
        if catalog[family_id]["status"] == "SUPERSEDED"
    }
    return {
        "retained_family_ids": sorted(retained, key=lambda item: encode_canonical(item)),
        "added_family_ids": sorted(added, key=lambda item: encode_canonical(item)),
        "removed_family_ids": sorted(removed, key=lambda item: encode_canonical(item)),
        "superseded_family_ids": sorted(superseded, key=lambda item: encode_canonical(item)),
    }


def validate_change(
    data: ArchiveData,
    change: object,
    expected_kind: str,
    catalog: dict[str, dict[str, Any]],
    source: dict[str, Any],
    terminal_ids: set[str],
    label: str,
) -> dict[str, Any]:
    value = require_mapping(change, "SILENT_CLASSIFICATION_CHANGE_REJECTED", label)
    expected_keys = {
        "family_id",
        "change_kind",
        "replacement_family_ids",
        "rationale",
        "evidence_locators",
    }
    require(
        set(value) == expected_keys,
        "SILENT_CLASSIFICATION_CHANGE_REJECTED",
        f"{label} fields are not closed",
    )
    family_id = require_text(
        value.get("family_id"), "SILENT_CLASSIFICATION_CHANGE_REJECTED", f"{label}.family_id"
    )
    if family_id not in catalog:
        fail(
            "UNKNOWN_REQUIREMENT_FAMILY_REJECTED", f"{label} references unknown family {family_id}"
        )
    if value.get("change_kind") != expected_kind:
        fail(
            "SILENT_CLASSIFICATION_CHANGE_REJECTED",
            f"{label} has {value.get('change_kind')!r}, expected {expected_kind!r}",
        )
    rationale = value.get("rationale")
    if not isinstance(rationale, str) or not rationale.strip():
        fail("CORRECTION_WITHOUT_RATIONALE_REJECTED", f"{label} has no rationale")
    locators = require_list(
        value.get("evidence_locators"),
        "CORRECTION_WITHOUT_EVIDENCE_REJECTED",
        f"{label}.evidence_locators",
    )
    if not locators:
        fail("CORRECTION_WITHOUT_EVIDENCE_REJECTED", f"{label} has no evidence")
    validate_locator_list(data, locators, source, None, f"{label}.evidence_locators")
    replacements = require_list(
        value.get("replacement_family_ids"),
        "SILENT_CLASSIFICATION_CHANGE_REJECTED",
        f"{label}.replacement_family_ids",
    )
    assert_canonical_set(
        replacements, f"{label}.replacement_family_ids", "SILENT_CLASSIFICATION_CHANGE_REJECTED"
    )
    if expected_kind != "SUPERSEDED":
        if replacements:
            fail(
                "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                f"{label} has replacements for {expected_kind}",
            )
    else:
        family = catalog[family_id]
        if family.get("status") != "SUPERSEDED":
            fail(
                "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                f"{label} supersedes a non-SUPERSEDED family",
            )
        allowed = set(family.get("superseded_by", []))
        if not set(replacements).issubset(allowed):
            fail(
                "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                f"{label} replacement is outside catalog superseded_by",
            )
        if not replacements and not rationale.strip():
            fail(
                "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                f"{label} empty replacement lacks explicit rationale",
            )
        for replacement in replacements:
            if replacement not in terminal_ids or catalog[replacement].get("status") != "ACTIVE":
                fail(
                    "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                    f"{label} replacement is not terminally assigned and ACTIVE",
                )
    return value


def validate_classifications(
    data: ArchiveData,
    classifications: object,
    catalog: dict[str, dict[str, Any]],
) -> tuple[dict[str, dict[str, Any]], Counter[str]]:
    records = require_list(classifications, "MISSING_CLASSIFICATION_REJECTED", "classifications")
    if len(records) != 402:
        if len(records) < 402:
            fail(
                "MISSING_CLASSIFICATION_REJECTED",
                f"expected 402 classifications, found {len(records)}",
            )
        fail(
            "DUPLICATE_ORACLE_IDENTITY_REJECTED",
            f"expected 402 classifications, found {len(records)}",
        )
    by_osi: dict[str, dict[str, Any]] = {}
    for record in records:
        record = require_mapping(record, "SILENT_CLASSIFICATION_CHANGE_REJECTED", "classification")
        osi = require_uuid(record.get("oracle_semantic_identity"), "classification OSI")
        if osi in by_osi:
            fail("DUPLICATE_ORACLE_IDENTITY_REJECTED", f"duplicate classification OSI {osi}")
        by_osi[osi] = record
    require(
        set(by_osi) == set(data.evidence_by_osi),
        "UNKNOWN_ORACLE_IDENTITY_REJECTED",
        "classification OSI set differs from pinned evidence",
    )
    ordered = [record["oracle_semantic_identity"] for record in records]
    require(
        ordered == sorted(ordered),
        "DUPLICATE_ORACLE_IDENTITY_REJECTED",
        "classifications are not sorted by OSI",
    )
    usage: Counter[str] = Counter()
    for osi, record in by_osi.items():
        source = source_for_osi(data, osi)
        source_value = require_mapping(
            record.get("source_identity"),
            "SOURCE_DIGEST_MISMATCH_REJECTED",
            f"{osi}.source_identity",
        )
        expected_source = {
            key: source[key]
            for key in (
                "archive_artifact",
                "oracle_semantic_identity",
                "oracle_source_record_id",
                "oracle_layout",
                "source_record_raw_sha256",
                "normalized_record_sha256",
            )
        }
        if source_value != expected_source:
            fail("SOURCE_DIGEST_MISMATCH_REJECTED", f"source identity differs for {osi}")
        source_ref = record.get("source_evidence_digest")
        digest_reference_cbor(source_ref, f"{osi}.source_evidence_digest")
        expected_source_ref = expected_source_digest(expected_source)
        if source_ref != expected_source_ref:
            fail("EVIDENCE_DIGEST_TAMPER_REJECTED", f"source evidence digest differs for {osi}")
        previous = record.get("previous_rev3_classification_identity")
        previous_value = digest_reference_cbor(
            previous, f"{osi}.previous_rev3_classification_identity"
        )
        require(
            previous.get("semantic_domain") == REV3_DOMAIN
            and previous.get("input_schema_id") == REV3_INPUT_SCHEMA,
            "SILENT_CLASSIFICATION_CHANGE_REJECTED",
            f"previous REV3 identity metadata is wrong for {osi}",
        )
        if previous != expected_rev3_digest(data.rev3_by_osi[osi]):
            fail(
                "SILENT_CLASSIFICATION_CHANGE_REJECTED", f"previous REV3 identity differs for {osi}"
            )
        del previous_value
        review_status = record.get("review_status")
        if review_status not in {"REVIEWED_CONFIRMED", "REVIEWED_CORRECTED"}:
            fail("NONTERMINAL_CLASSIFICATION_REJECTED", f"classification {osi} is not terminal")
        validate_review_basis(data, record, source)
        validate_provenance(record)
        assignments = require_list(
            record.get("requirement_assignments"),
            "SILENT_CLASSIFICATION_CHANGE_REJECTED",
            f"{osi}.requirement_assignments",
        )
        assignment_ids: list[str] = []
        for index, assignment in enumerate(assignments):
            value = validate_assignment(
                data, assignment, catalog, source, f"{osi}.requirement_assignments[{index}]"
            )
            assignment_ids.append(value["requirement_family_id"])
            usage[value["requirement_family_id"]] += 1
        assert_canonical_set(
            assignment_ids,
            f"{osi}.requirement_assignments",
            "SILENT_CLASSIFICATION_CHANGE_REJECTED",
        )
        if len(set(assignment_ids)) != len(assignment_ids):
            fail("SILENT_CLASSIFICATION_CHANGE_REJECTED", f"duplicate assignment for {osi}")
        rev3_ids = set(data.rev3_by_osi[osi].get("requirement_ids", []))
        terminal_ids = set(assignment_ids)
        summary = expected_summary_arrays(rev3_ids, terminal_ids, catalog)
        delta = require_mapping(
            record.get("classification_delta"),
            "SILENT_CLASSIFICATION_CHANGE_REJECTED",
            f"{osi}.classification_delta",
        )
        for key in summary:
            values = require_list(
                delta.get(key),
                "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                f"{osi}.classification_delta.{key}",
            )
            assert_canonical_set(
                values, f"{osi}.classification_delta.{key}", "SILENT_CLASSIFICATION_CHANGE_REJECTED"
            )
            if values != summary[key]:
                fail(
                    "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                    f"derived delta summary differs for {osi}: {key}",
                )
        union = rev3_ids | terminal_ids
        changes = require_list(
            delta.get("changes"),
            "SILENT_CLASSIFICATION_CHANGE_REJECTED",
            f"{osi}.classification_delta.changes",
        )
        if len(changes) != len(union):
            fail(
                "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                f"classification {osi} does not have exactly one change per union family",
            )
        change_ids: set[str] = set()
        expected_kinds: dict[str, str] = {}
        for family_id in union:
            if family_id in rev3_ids and family_id in terminal_ids:
                expected_kinds[family_id] = "RETAINED"
            elif family_id in terminal_ids:
                expected_kinds[family_id] = "ADDED"
            elif catalog[family_id].get("status") == "SUPERSEDED":
                expected_kinds[family_id] = "SUPERSEDED"
            else:
                expected_kinds[family_id] = "REMOVED"
        for index, change in enumerate(changes):
            change_value = require_mapping(
                change,
                "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                f"{osi}.classification_delta.changes[{index}]",
            )
            family_id = require_text(
                change_value.get("family_id"),
                "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                f"{osi}.classification_delta.changes[{index}].family_id",
            )
            if family_id in change_ids:
                fail(
                    "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                    f"duplicate change family {family_id} for {osi}",
                )
            change_ids.add(family_id)
            if family_id not in expected_kinds:
                fail(
                    "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                    f"change family {family_id} is outside the REV3/terminal union",
                )
            validate_change(
                data,
                change,
                expected_kinds[family_id],
                catalog,
                source,
                terminal_ids,
                f"{osi}.classification_delta.changes[{index}]",
            )
        change_keys = [[change["family_id"], change["change_kind"]] for change in changes]
        assert_canonical_set(
            change_keys,
            f"{osi}.classification_delta.changes",
            "SILENT_CLASSIFICATION_CHANGE_REJECTED",
        )
        if change_ids != union:
            fail("SILENT_CLASSIFICATION_CHANGE_REJECTED", f"change family set differs for {osi}")
        expected_status = (
            "REVIEWED_CONFIRMED"
            if all(kind == "RETAINED" for kind in expected_kinds.values())
            else "REVIEWED_CORRECTED"
        )
        if review_status != expected_status:
            fail(
                "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                f"review status/delta relation differs for {osi}",
            )
        expected_classification = expected_identity(record)
        if record.get("classification_identity") != expected_classification:
            fail(
                "SILENT_CLASSIFICATION_CHANGE_REJECTED",
                f"classification identity differs for {osi}",
            )
    return by_osi, usage


def validate_classification_artifact(value: object) -> list[Any]:
    artifact = require_mapping(value, "MISSING_CLASSIFICATION_REJECTED", "classification artifact")
    require(
        artifact.get("schema") == CLASSIFICATION_SCHEMA,
        "WRONG_CLASSIFICATION_SCHEMA_REJECTED",
        "classification schema is wrong",
    )
    require(
        artifact.get("source_package_sha256") == EXPECTED_ARCHIVE_SHA256,
        "SOURCE_DIGEST_MISMATCH_REJECTED",
        "classification package digest is wrong",
    )
    require(
        artifact.get("input_oracle_identity_count") == 402,
        "MISSING_CLASSIFICATION_REJECTED",
        "classification input count is not 402",
    )
    return require_list(
        artifact.get("classifications"),
        "MISSING_CLASSIFICATION_REJECTED",
        "classification artifact classifications",
    )


def validate_lifecycle_usage(
    catalog: dict[str, dict[str, Any]], usage: Counter[str]
) -> dict[str, int]:
    target_ids = {
        target for family in catalog.values() for target in family.get("superseded_by", [])
    }
    counts: Counter[str] = Counter()
    for family_id, family in catalog.items():
        status = family["status"]
        count = usage.get(family_id, 0)
        counts[status] += 1
        if status == "ACTIVE" and count == 0:
            fail(
                "ACTIVE_WITH_ZERO_ASSIGNMENTS_REJECTED",
                f"ACTIVE family {family_id} has zero assignments",
            )
        if status == "ACTIVE_UNASSIGNED" and count != 0:
            fail(
                "ACTIVE_UNASSIGNED_FAMILY_ASSIGNED_REJECTED",
                f"ACTIVE_UNASSIGNED family {family_id} is assigned",
            )
        if status == "SUPERSEDED" and count != 0:
            fail(
                "SUPERSEDED_FAMILY_ASSIGNED_REJECTED", f"SUPERSEDED family {family_id} is assigned"
            )
        if status == "RETIRED" and count != 0:
            fail("RETIRED_FAMILY_ASSIGNED_REJECTED", f"RETIRED family {family_id} is assigned")
        if family.get("family_origin") == "B2_NEW" and count == 0 and family_id not in target_ids:
            fail(
                "SPECULATIVE_NEW_FAMILY_REJECTED",
                f"new family {family_id} has no terminal use or supersession-target need",
            )
    counts["ACTIVE_ASSIGNED"] = sum(
        1
        for family_id, family in catalog.items()
        if family["status"] == "ACTIVE" and usage.get(family_id, 0) > 0
    )
    counts["ACTIVE_UNASSIGNED"] = sum(
        1 for family_id, family in catalog.items() if family["status"] == "ACTIVE_UNASSIGNED"
    )
    return dict(counts)


def parse_projection(value: object) -> list[dict[str, str]]:
    if not isinstance(value, list):
        fail("MISSING_DECK_ROW_REFERENCE_REJECTED", "projection is not a CSV row list")
    return [
        require_mapping(row, "MISSING_DECK_ROW_REFERENCE_REJECTED", "projection row")  # type: ignore[return-value]
        for row in value
    ]


def projection_requirement_ids(record: dict[str, Any]) -> list[str]:
    return [
        assignment["requirement_family_id"]
        for assignment in sorted(
            record["requirement_assignments"],
            key=lambda value: encode_canonical(value["requirement_family_id"]),
        )
    ]


def validate_projection(
    data: ArchiveData,
    projection: object,
    classifications: dict[str, dict[str, Any]],
) -> None:
    rows = parse_projection(projection)
    if len(rows) < 441:
        fail(
            "MISSING_DECK_ROW_REFERENCE_REJECTED", f"projection has {len(rows)} rows, expected 441"
        )
    if len(rows) > 441:
        fail(
            "UNKNOWN_DECK_ROW_REFERENCE_REJECTED", f"projection has {len(rows)} rows, expected 441"
        )
    seen: set[str] = set()
    for row in rows:
        row_id = row.get("deck_row_id")
        if not isinstance(row_id, str) or row_id not in data.deck_rows_by_id:
            fail("UNKNOWN_DECK_ROW_REFERENCE_REJECTED", f"projection row {row_id!r} is not pinned")
        if row_id in seen:
            fail("UNKNOWN_DECK_ROW_REFERENCE_REJECTED", f"projection row {row_id} is duplicated")
        seen.add(row_id)
        pinned = data.deck_rows_by_id[row_id]
        if row.get("deck_id") != pinned["deck_id"]:
            fail("REUSED_ORACLE_IDENTITY_FORK_REJECTED", f"deck id changed for row {row_id}")
        expected_osi = pinned["oracle_semantic_identity"]
        if row.get("oracle_semantic_identity") != expected_osi:
            if row.get("oracle_semantic_identity") in classifications:
                fail(
                    "DECK_ROW_OSI_REBIND_REJECTED",
                    f"row {row_id} was rebound to a different valid OSI",
                )
            fail("UNKNOWN_ORACLE_IDENTITY_REJECTED", f"row {row_id} references an unknown OSI")
        if expected_osi not in classifications:
            fail("MISSING_CLASSIFICATION_REJECTED", f"row {row_id} resolves to an unclassified OSI")
        classification = classifications[expected_osi]
        expected_identity = classification["classification_identity"]["digest_hex"]
        if row.get("terminal_classification_identity") != expected_identity:
            fail(
                "REUSED_ORACLE_IDENTITY_FORK_REJECTED",
                f"row {row_id} has a forked classification identity",
            )
        if row.get("classification_status") != classification["review_status"]:
            fail(
                "REUSED_ORACLE_IDENTITY_FORK_REJECTED",
                f"row {row_id} has a forked classification status",
            )
        try:
            requirement_ids = json.loads(row.get("terminal_requirement_ids", ""))
        except json.JSONDecodeError:
            fail(
                "REUSED_ORACLE_IDENTITY_FORK_REJECTED",
                f"row {row_id} has invalid terminal requirement JSON",
            )
        if requirement_ids != projection_requirement_ids(classification):
            fail(
                "REUSED_ORACLE_IDENTITY_FORK_REJECTED", f"row {row_id} has a forked assignment set"
            )
    expected_ids = [row["deck_row_id"] for row in data.deck_rows]
    actual_ids = [row["deck_row_id"] for row in rows]
    if actual_ids != expected_ids:
        missing = [row_id for row_id in expected_ids if row_id not in seen]
        if missing:
            fail("MISSING_DECK_ROW_REFERENCE_REJECTED", f"projection omits pinned row {missing[0]}")
        fail(
            "REUSED_ORACLE_IDENTITY_FORK_REJECTED",
            "projection row order differs from pinned source order",
        )


def expected_gate_statuses() -> dict[str, str]:
    return {
        "CLASSIFICATION_REFERENCE_CLOSURE": "PASS",
        "OFFICIAL_RULE_CITATION_CLOSURE": "BLOCKED",
        "DECLARED_INTERACTION_MODEL_CLOSURE": "BLOCKED",
        "REV2_REUSE_RATIO_REPRODUCIBLE": "BLOCKED",
        "RANKING_UNCERTAINTY_PROPAGATION": "BLOCKED",
        "DECK_PAIR_LOCKED": "NO",
        "AUTHORITATIVE_RANKING_AVAILABLE": "NO",
        "M3_STARTED": "NO",
    }


def actual_inventory(extra_paths: set[str] | None = None) -> set[str]:
    files: set[str] = set()
    if B2_DIR.exists():
        for path in B2_DIR.rglob("*"):
            if path.is_file():
                files.add(path.relative_to(B2_DIR).as_posix())
    if extra_paths:
        files.update(extra_paths)
    if files != set(EXACT_B2_FILES):
        extras = sorted(files - set(EXACT_B2_FILES))
        missing = sorted(set(EXACT_B2_FILES) - files)
        if extras:
            fail("B2_FILE_INVENTORY_REJECTED", f"unrecognized B2 file: {extras[0]}")
        fail("B2_FILE_INVENTORY_REJECTED", f"required B2 file is missing: {missing[0]}")
    return files


def validate_matrix(matrix: object) -> None:
    value = require_mapping(matrix, "B2_FILE_INVENTORY_REJECTED", "negative matrix")
    require(
        value.get("schema") == NEGATIVE_SCHEMA,
        "B2_FILE_INVENTORY_REJECTED",
        "negative matrix schema is wrong",
    )
    cases = require_list(value.get("cases"), "B2_FILE_INVENTORY_REJECTED", "negative matrix cases")
    actual: list[tuple[str, str]] = []
    for case in cases:
        item = require_mapping(case, "B2_FILE_INVENTORY_REJECTED", "negative matrix case")
        actual.append(
            (
                require_text(
                    item.get("error_code"),
                    "B2_FILE_INVENTORY_REJECTED",
                    "negative matrix error code",
                ),
                require_text(
                    item.get("mutation"), "B2_FILE_INVENTORY_REJECTED", "negative matrix mutation"
                ),
            )
        )
    require(
        actual == list(NEGATIVE_CASES),
        "B2_FILE_INVENTORY_REJECTED",
        "negative matrix does not match the closed 38-case contract",
    )
    require(
        len({code for code, _ in actual}) == 38 and len(actual) == 38,
        "B2_FILE_INVENTORY_REJECTED",
        "negative matrix is not exactly 38 unique cases",
    )
    require(
        "ALLOWLIST_NEAR_MISS_PATH_REJECTED" not in {code for code, _ in actual},
        "B2_FILE_INVENTORY_REJECTED",
        "master-drift near-miss case leaked into B2 matrix",
    )


def validate_closure(
    data: ArchiveData,
    closure: object,
    catalog: dict[str, dict[str, Any]],
    classifications: dict[str, dict[str, Any]],
    usage: Counter[str],
    projection: list[dict[str, str]],
    *,
    validate_bound_files: bool,
) -> None:
    value = require_mapping(closure, "WRONG_CLOSURE_SCHEMA_REJECTED", "classification closure")
    if value.get("schema") != CLOSURE_SCHEMA:
        fail("WRONG_CLOSURE_SCHEMA_REJECTED", f"unexpected closure schema {value.get('schema')!r}")
    require(
        value.get("source_package_sha256") == EXPECTED_ARCHIVE_SHA256,
        "WRONG_CLOSURE_SCHEMA_REJECTED",
        "closure package digest is wrong",
    )
    require(
        value.get("CLASSIFICATION_REFERENCE_CLOSURE") == "PASS",
        "OTHER_GATE_PROMOTION_REJECTED",
        "classification closure is not PASS",
    )
    statuses = require_mapping(
        value.get("gate_status"), "OTHER_GATE_PROMOTION_REJECTED", "closure.gate_status"
    )
    expected_statuses = expected_gate_statuses()
    for key, expected in expected_statuses.items():
        actual = statuses.get(key)
        if (
            key in {"DECK_PAIR_LOCKED", "AUTHORITATIVE_RANKING_AVAILABLE", "M3_STARTED"}
            and actual != expected
        ):
            code = (
                "DECK_LOCK_PROMOTION_REJECTED"
                if key == "DECK_PAIR_LOCKED"
                else "M3_PROMOTION_REJECTED"
                if key == "M3_STARTED"
                else "OTHER_GATE_PROMOTION_REJECTED"
            )
            fail(code, f"closure promotes {key} to {actual!r}")
        if (
            key not in {"DECK_PAIR_LOCKED", "AUTHORITATIVE_RANKING_AVAILABLE", "M3_STARTED"}
            and actual != expected
        ):
            fail(
                "OTHER_GATE_PROMOTION_REJECTED",
                f"closure changes protected status {key} to {actual!r}",
            )
    require(
        value.get("OFFICIAL_RULE_CITATION_CLOSURE") == "BLOCKED",
        "OTHER_GATE_PROMOTION_REJECTED",
        "official citation closure is not BLOCKED",
    )
    require(
        value.get("block_reason") == "PENDING_B1_FINAL",
        "OTHER_GATE_PROMOTION_REJECTED",
        "citation block reason is not PENDING_B1_FINAL",
    )
    metrics = require_mapping(
        value.get("metrics"), "WRONG_CLOSURE_SCHEMA_REJECTED", "closure.metrics"
    )
    expected_metrics = {
        "oracle_semantic_identity_count": 402,
        "deck_row_count": 441,
        "reused_osi_count": 23,
        "additional_row_reference_count": 39,
        "total_deck_quantity": 600,
        "historical_family_count": 216,
        "catalog_family_count": len(catalog),
        "classification_count": len(classifications),
        "projection_row_count": len(projection),
        "terminal_assignment_edge_count": sum(usage.values()),
    }
    require(
        metrics == expected_metrics,
        "WRONG_CLOSURE_SCHEMA_REJECTED",
        "closure metrics do not match the validated model",
    )
    bound = require_list(
        value.get("bound_artifacts"), "WRONG_CLOSURE_SCHEMA_REJECTED", "closure.bound_artifacts"
    )
    if len(bound) != len(CLOSURE_BOUND_FILES):
        fail("WRONG_CLOSURE_SCHEMA_REJECTED", "closure does not bind exactly six artifacts")
    seen: set[str] = set()
    for item in bound:
        entry = require_mapping(item, "WRONG_CLOSURE_SCHEMA_REJECTED", "closure bound artifact")
        path = require_text(
            entry.get("path"), "WRONG_CLOSURE_SCHEMA_REJECTED", "closure bound path"
        )
        digest = require_file_digest(entry.get("raw_sha256"), f"closure bound digest {path}")
        if path in seen or path not in CLOSURE_BOUND_FILES:
            fail(
                "WRONG_CLOSURE_SCHEMA_REJECTED",
                f"closure has duplicate or unapproved bound path {path}",
            )
        seen.add(path)
        if validate_bound_files:
            candidate = B2_DIR / Path(path)
            if not candidate.is_file() or sha256_file(candidate) != digest:
                fail(
                    "WRONG_CLOSURE_SCHEMA_REJECTED",
                    f"closure bound artifact digest mismatch for {path}",
                )
    require(
        seen == set(CLOSURE_BOUND_FILES),
        "WRONG_CLOSURE_SCHEMA_REJECTED",
        "closure bound-artifact set is incomplete",
    )
    require(
        "classification_closure.v1.json" not in seen
        and "verification/b2_verification_summary.v1.json" not in seen,
        "WRONG_CLOSURE_SCHEMA_REJECTED",
        "closure binds a root or summary record",
    )


def git_bytes(
    args: list[str], input_bytes: bytes | None = None, *, repo_root: Path = ROOT
) -> bytes:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo_root), *args],
            input=input_bytes,
            capture_output=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        blocked("GIT_EVIDENCE_UNAVAILABLE", f"git {' '.join(args)} failed: {exc}")
    return result.stdout


def git_try_bytes(args: list[str], *, repo_root: Path = ROOT) -> bytes | None:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo_root), *args],
            capture_output=True,
            check=False,
        )
    except OSError as exc:
        blocked("GIT_EVIDENCE_UNAVAILABLE", f"git {' '.join(args)} failed: {exc}")
    if result.returncode != 0:
        return None
    return result.stdout


def git_is_ancestor(ancestor: str, descendant: str, *, repo_root: Path = ROOT) -> bool:
    try:
        result = subprocess.run(
            [
                "git",
                "-C",
                str(repo_root),
                "merge-base",
                "--is-ancestor",
                ancestor,
                descendant,
            ],
            capture_output=True,
            check=False,
        )
    except OSError as exc:
        blocked("GIT_EVIDENCE_UNAVAILABLE", f"git merge-base failed: {exc}")
    return result.returncode == 0


def resolve_historical_evidence(
    execution_commit: str,
    *,
    repo_root: Path = ROOT,
    current_b2_dir: Path = B2_DIR,
) -> str:
    """Resolve the unique final evidence commit for a recorded H_exec."""
    current_head = git_bytes(["rev-parse", "HEAD"], repo_root=repo_root).decode("ascii").strip()
    if not git_is_ancestor(execution_commit, current_head, repo_root=repo_root):
        fail(
            "EXECUTION_COMMIT_NOT_ANCESTOR_REJECTED",
            "execution_commit "
            f"{execution_commit} is not an ancestor of current HEAD {current_head}",
        )

    parent_rows = (
        git_bytes(["rev-list", "--parents", current_head], repo_root=repo_root)
        .decode("ascii")
        .splitlines()
    )
    direct_children = []
    for row in parent_rows:
        parts = row.split()
        if len(parts) == 2 and parts[1] == execution_commit:
            direct_children.append(parts[0])
    direct_children.sort()
    rejected: list[tuple[str, str, str]] = []
    valid: list[str] = []

    for candidate in direct_children:
        diff_raw = git_bytes(
            ["diff", "--name-only", "--no-renames", execution_commit, candidate, "--"],
            repo_root=repo_root,
        )
        diff_files = [line for line in diff_raw.decode("utf-8").splitlines() if line]
        if B2_SUMMARY_GIT_PATH.as_posix() not in diff_files:
            continue
        if diff_files != [B2_SUMMARY_GIT_PATH.as_posix()]:
            rejected.append(
                (
                    candidate,
                    "HISTORICAL_EVIDENCE_DIFF_REJECTED",
                    "candidate changes "
                    f"{diff_files!r}, expected only {B2_SUMMARY_GIT_PATH.as_posix()!r}",
                )
            )
            continue

        historical_summary_oid = git_try_bytes(
            ["rev-parse", f"{candidate}:{B2_SUMMARY_GIT_PATH.as_posix()}"],
            repo_root=repo_root,
        )
        current_summary_oid = git_try_bytes(
            ["rev-parse", f"{current_head}:{B2_SUMMARY_GIT_PATH.as_posix()}"],
            repo_root=repo_root,
        )
        if (
            historical_summary_oid is None
            or current_summary_oid is None
            or historical_summary_oid.strip() != current_summary_oid.strip()
        ):
            rejected.append(
                (
                    candidate,
                    "HISTORICAL_SUMMARY_BLOB_MISMATCH_REJECTED",
                    "historical H_evidence summary blob differs from current HEAD",
                )
            )
            continue

        artifact_drift = False
        for git_path in B2_GIT_FILES:
            historical_oid = git_try_bytes(
                ["rev-parse", f"{candidate}:{git_path.as_posix()}"],
                repo_root=repo_root,
            )
            current_oid = git_try_bytes(
                ["rev-parse", f"{current_head}:{git_path.as_posix()}"],
                repo_root=repo_root,
            )
            if (
                historical_oid is None
                or current_oid is None
                or historical_oid.strip() != current_oid.strip()
            ):
                rejected.append(
                    (
                        candidate,
                        "B2_ARTIFACT_DRIFT_REJECTED",
                        "B2 artifact "
                        f"{git_path.as_posix()} differs between H_evidence and current HEAD",
                    )
                )
                artifact_drift = True
                break
            historical_bytes = git_try_bytes(
                ["show", f"{candidate}:{git_path.as_posix()}"], repo_root=repo_root
            )
            relative = git_path.relative_to(B2_GIT_RELATIVE_ROOT)
            try:
                current_bytes = (current_b2_dir / relative).read_bytes()
            except OSError as exc:
                fail(
                    "B2_FILE_INVENTORY_REJECTED",
                    f"cannot read current B2 artifact {relative.as_posix()}: {exc}",
                )
            if historical_bytes is None or historical_bytes != current_bytes:
                rejected.append(
                    (
                        candidate,
                        "B2_ARTIFACT_DRIFT_REJECTED",
                        f"current B2 artifact {relative.as_posix()} differs from H_evidence",
                    )
                )
                artifact_drift = True
                break
        if artifact_drift:
            continue

        if not git_is_ancestor(execution_commit, candidate, repo_root=repo_root):
            rejected.append(
                (
                    candidate,
                    "HISTORICAL_EVIDENCE_PARENT_REJECTED",
                    "candidate evidence commit is not descended from execution_commit",
                )
            )
            continue
        if not git_is_ancestor(candidate, current_head, repo_root=repo_root):
            rejected.append(
                (
                    candidate,
                    "HISTORICAL_EVIDENCE_NOT_REACHABLE_REJECTED",
                    "candidate evidence commit is not reachable from current HEAD",
                )
            )
            continue
        valid.append(candidate)

    if len(valid) > 1:
        fail(
            "HISTORICAL_EVIDENCE_AMBIGUOUS_REJECTED",
            f"multiple valid historical evidence commits: {valid}",
        )
    if not valid:
        if rejected:
            _, code, message = sorted(rejected)[0]
            fail(code, message)
        fail(
            "HISTORICAL_EVIDENCE_NOT_FOUND_REJECTED",
            "no valid summary-only evidence child of execution_commit "
            f"{execution_commit} is reachable from {current_head}",
        )
    return valid[0]


def tracked_source_fingerprint() -> str:
    paths = git_bytes(["ls-files", "-z"]).split(b"\0")
    digest = hashlib.sha256()
    for path_bytes in paths:
        if not path_bytes:
            continue
        try:
            payload = (ROOT / Path(path_bytes.decode("utf-8"))).read_bytes()
        except (OSError, UnicodeDecodeError) as exc:
            blocked("GIT_EVIDENCE_UNAVAILABLE", f"cannot read tracked source {path_bytes!r}: {exc}")
        digest.update(struct.pack(">Q", len(path_bytes)))
        digest.update(path_bytes)
        digest.update(struct.pack(">Q", len(payload)))
        digest.update(payload)
    return digest.hexdigest()


def tracked_source_fingerprint_of_commit(commit: str) -> str:
    paths = git_bytes(["ls-tree", "-r", "-z", "--name-only", commit]).split(b"\0")
    digest = hashlib.sha256()
    for path_bytes in paths:
        if not path_bytes:
            continue
        try:
            path = path_bytes.decode("utf-8")
        except UnicodeDecodeError as exc:
            fail("SUMMARY_FINGERPRINT_MISMATCH", f"H_exec contains a non-UTF-8 path: {exc}")
        payload = git_bytes(["show", f"{commit}:{path}"])
        digest.update(struct.pack(">Q", len(path_bytes)))
        digest.update(path_bytes)
        digest.update(struct.pack(">Q", len(payload)))
        digest.update(payload)
    return digest.hexdigest()


def read_artifact_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        fail(
            "B2_FILE_INVENTORY_REJECTED",
            f"missing B2 artifact: {path.relative_to(B2_DIR).as_posix()}",
        )
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        fail("B2_FILE_INVENTORY_REJECTED", f"cannot read B2 artifact {path}: {exc}")


def read_artifacts(*, extra_inventory: set[str] | None = None) -> dict[str, Any]:
    actual_inventory(extra_inventory)
    try:
        projection_text = (B2_DIR / "deck_row_classification_refs.v1.csv").read_text(
            encoding="utf-8"
        )
        projection = list(csv.DictReader(io.StringIO(projection_text)))
    except (OSError, UnicodeDecodeError) as exc:
        fail("MISSING_DECK_ROW_REFERENCE_REJECTED", f"cannot read B2 projection: {exc}")
    return {
        "spec": (B2_DIR / "B2_DESIGN_SPEC.md").read_bytes(),
        "classifications": read_artifact_json(B2_DIR / "card_semantic_classifications.v1.json"),
        "projection": projection,
        "catalog": read_artifact_json(B2_DIR / "requirement_family_catalog.v1.json"),
        "closure": read_artifact_json(B2_DIR / "classification_closure.v1.json"),
        "report": (B2_DIR / "CLASSIFICATION_REPORT.md").read_text(encoding="utf-8"),
        "matrix": read_artifact_json(B2_DIR / "verification" / "b2_negative_test_matrix.v1.json"),
        "summary": read_artifact_json(B2_DIR / "verification" / "b2_verification_summary.v1.json"),
    }


def validate_summary(data: ArchiveData, summary: object, *, allow_staging: bool) -> str:
    value = require_mapping(summary, "B2_FILE_INVENTORY_REJECTED", "verification summary")
    require(
        value.get("schema") == SUMMARY_SCHEMA,
        "B2_FILE_INVENTORY_REJECTED",
        "verification summary schema is wrong",
    )
    require(
        value.get("closure_file_sha256") == sha256_file(B2_DIR / "classification_closure.v1.json"),
        "B2_FILE_INVENTORY_REJECTED",
        "verification summary closure checksum is wrong",
    )
    identity = require_mapping(
        value.get("checker_version_and_identity"),
        "B2_FILE_INVENTORY_REJECTED",
        "verification summary checker identity",
    )
    require(
        identity.get("path") == "scripts/check_m2_5_b2_classifications.py",
        "B2_FILE_INVENTORY_REJECTED",
        "verification summary checker path is wrong",
    )
    require_hex(identity.get("raw_sha256"), "verification summary checker raw SHA")
    commands = require_list(
        value.get("actual_commands"), "B2_FILE_INVENTORY_REJECTED", "verification summary commands"
    )
    require(
        len(commands) == len(REQUIRED_COMMANDS),
        "B2_FILE_INVENTORY_REJECTED",
        "verification summary command count is wrong",
    )
    statuses: list[str] = []
    for command, expected in zip(commands, REQUIRED_COMMANDS, strict=True):
        item = require_mapping(
            command, "B2_FILE_INVENTORY_REJECTED", "verification summary command"
        )
        require(
            item.get("command") == expected,
            "B2_FILE_INVENTORY_REJECTED",
            "verification summary command order differs",
        )
        status = item.get("status")
        require(
            status in {"PASS", "FAIL", "NOT_RUN", "BLOCKED", "EXPERIMENTAL"},
            "B2_FILE_INVENTORY_REJECTED",
            f"unknown verification command status {status!r}",
        )
        statuses.append(status)
    execution_commit = value.get("execution_commit")
    before = value.get("source_tree_before_fingerprint")
    after = value.get("source_tree_after_fingerprint")
    if execution_commit is None:
        if (
            not allow_staging
            or any(status != "NOT_RUN" for status in statuses)
            or before is not None
            or after is not None
        ):
            fail(
                "B2_FILE_INVENTORY_REJECTED",
                "verification summary is neither valid final evidence nor valid H_exec staging",
            )
        require(
            identity.get("raw_sha256")
            == sha256_file(ROOT / "scripts" / "check_m2_5_b2_classifications.py"),
            "B2_FILE_INVENTORY_REJECTED",
            "staging checker identity does not match the working tree",
        )
        return "STAGING"
    require(
        isinstance(execution_commit, str)
        and re.fullmatch(r"[0-9a-f]{40}", execution_commit) is not None,
        "B2_FILE_INVENTORY_REJECTED",
        "execution_commit is not a full lowercase Git SHA",
    )
    resolve_historical_evidence(execution_commit)
    require(
        all(status == "PASS" for status in statuses),
        "B2_GATES_NOT_PASS",
        "final verification summary contains a non-PASS command status",
    )
    expected_checker_sha = sha256_bytes(
        git_bytes(["show", f"{execution_commit}:scripts/check_m2_5_b2_classifications.py"])
    )
    require(
        identity.get("raw_sha256") == expected_checker_sha,
        "SUMMARY_CHECKER_IDENTITY_MISMATCH",
        "checker identity is not bound to H_exec",
    )
    before_digest = require_hex(before, "source_tree_before_fingerprint")
    after_digest = require_hex(after, "source_tree_after_fingerprint")
    recomputed = tracked_source_fingerprint_of_commit(execution_commit)
    require(
        before_digest == recomputed,
        "SUMMARY_FINGERPRINT_MISMATCH",
        "before fingerprint is not the tracked-source fingerprint of H_exec",
    )
    require(
        after_digest == recomputed,
        "SUMMARY_FINGERPRINT_MISMATCH",
        "after fingerprint is not the tracked-source fingerprint of H_exec",
    )
    require(
        before_digest == after_digest,
        "SUMMARY_FINGERPRINT_MISMATCH",
        "before/after execution fingerprints differ",
    )
    return "FINAL"


def validate_report(
    report: object,
    data: ArchiveData,
    classifications: dict[str, dict[str, Any]],
    catalog: dict[str, dict[str, Any]],
) -> None:
    if not isinstance(report, str):
        fail("B2_FILE_INVENTORY_REJECTED", "classification report is not text")
    required_fragments = (
        EXPECTED_ARCHIVE_SHA256,
        "216/216",
        "402/402",
        "441/441",
        "23",
        "39",
        "B2_SEMANTIC_BOUNDARY_V1",
        "disposable candidate/review tooling only",
        "OFFICIAL_RULE_CITATION_CLOSURE = BLOCKED",
        "DECK_PAIR_LOCKED = NO",
        "M3_STARTED = NO",
    )
    for fragment in required_fragments:
        if fragment not in report:
            fail(
                "B2_FILE_INVENTORY_REJECTED",
                f"classification report omits required evidence fragment {fragment!r}",
            )
    del data, classifications, catalog


def validate_model(
    data: ArchiveData,
    artifacts: dict[str, Any],
    *,
    extra_inventory: set[str] | None = None,
    validate_bound_files: bool = True,
    allow_staging: bool = True,
) -> tuple[dict[str, dict[str, Any]], Counter[str], str]:
    actual_inventory(extra_inventory)
    validate_spec(data, artifacts)
    validate_matrix(artifacts["matrix"])
    catalog = validate_catalog(data, artifacts["catalog"])
    classification_records = validate_classification_artifact(artifacts["classifications"])
    classifications, usage = validate_classifications(data, classification_records, catalog)
    validate_lifecycle_usage(catalog, usage)
    validate_projection(data, artifacts["projection"], classifications)
    validate_closure(
        data,
        artifacts["closure"],
        catalog,
        classifications,
        usage,
        artifacts["projection"],
        validate_bound_files=validate_bound_files,
    )
    validate_report(artifacts["report"], data, classifications, catalog)
    summary_status = validate_summary(data, artifacts["summary"], allow_staging=allow_staging)
    return classifications, usage, summary_status


def known_answer_test() -> None:
    locator_payload = [
        ORACLE_LOCATOR_SCHEMA,
        RAW_ORACLE_ARTIFACT,
        "00000000-0000-0000-0000-000000000000",
        bytes(32),
        "/oracle_text",
        bytes([0x11]) * 32,
    ]
    payload = [
        CLASSIFICATION_INPUT_SCHEMA,
        "00000000-0000-0000-0000-000000000000",
        bytes(32),
        ["reviewed_confirmed", None],
        bytes([0x11]) * 32,
        [],
        [],
        [
            REVIEW_BASIS_SCHEMA,
            ["source_grounded_card_review", None],
            [["oracle_field", locator_payload]],
        ],
        [
            PROVENANCE_SCHEMA,
            bytes([0x22]) * 32,
            ["source_grounded_review_v1", None],
        ],
    ]
    canonical_payload = encode_canonical(payload)
    envelope = encode_envelope(
        CLASSIFICATION_DOMAIN, CLASSIFICATION_INPUT_SCHEMA, canonical_payload
    )
    digest = hashlib.sha256(envelope).digest()
    require(
        len(canonical_payload) == 572,
        "KNOWN_ANSWER_VECTOR_FAILED",
        f"KAT payload length is {len(canonical_payload)}",
    )
    require(
        len(envelope) == 773,
        "KNOWN_ANSWER_VECTOR_FAILED",
        f"KAT envelope length is {len(envelope)}",
    )
    require(
        digest.hex() == "9312ec114479f1872ce5aa1fc7bcad967a5faa470d0a66adbe9689841dcf6eac",
        "KNOWN_ANSWER_VECTOR_FAILED",
        f"KAT digest is {digest.hex()}",
    )
    reference = digest_reference_json(CLASSIFICATION_DOMAIN, CLASSIFICATION_INPUT_SCHEMA, digest)
    require(
        digest_bytes(reference, "KAT DigestReferenceJsonV1") == digest,
        "KNOWN_ANSWER_VECTOR_FAILED",
        "JSON and CBOR digest reference bytes differ",
    )


def first_classification(artifacts: dict[str, Any], *, assigned: bool = False) -> dict[str, Any]:
    records = artifacts["classifications"]["classifications"]
    for record in records:
        if not assigned or record["requirement_assignments"]:
            return record
    fail("NEGATIVE_FIXTURE_FAILED", "no suitable classification fixture")


def catalog_by_id_from_artifacts(artifacts: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {item["family_id"]: item for item in artifacts["catalog"]["families"]}


def first_assignment_record(artifacts: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    record = first_classification(artifacts, assigned=True)
    return record, record["requirement_assignments"][0]


def first_active_family(artifacts: dict[str, Any]) -> str:
    record, assignment = first_assignment_record(artifacts)
    del record
    return assignment["requirement_family_id"]


def mutate_status(artifacts: dict[str, Any], status: str, relation: str) -> None:
    catalog = catalog_by_id_from_artifacts(artifacts)
    family_id = first_active_family(artifacts)
    family = catalog[family_id]
    family["status"] = status
    family["terminal_assignable"] = status == "ACTIVE"
    family["lifecycle_relation"] = relation
    family["supersession_reason"] = None
    family["superseded_by"] = []


def mutate_superseded(artifacts: dict[str, Any], targets: list[str]) -> None:
    catalog = catalog_by_id_from_artifacts(artifacts)
    family_id = first_active_family(artifacts)
    family = catalog[family_id]
    family["status"] = "SUPERSEDED"
    family["terminal_assignable"] = False
    family["lifecycle_relation"] = "SUPERSEDED_BY_REPLACEMENT"
    family["supersession_reason"] = "synthetic negative fixture"
    family["superseded_by"] = targets


def make_new_family(family_id: str, status: str = "ACTIVE_UNASSIGNED") -> dict[str, Any]:
    return {
        "family_id": family_id,
        "canonical_name": "synthetic B2 family",
        "precise_semantic_definition": (
            "B2_SEMANTIC_BOUNDARY_V1|family_id="
            f"{family_id}|includes=synthetic fixture concept|excludes=unrelated concepts|"
            "objects=synthetic fixture object|action_or_event=synthetic fixture action|"
            "timing=synthetic fixture timing|zone_visibility=synthetic fixture visibility|"
            "eligibility_condition_duration=synthetic fixture condition and duration|"
            "targets_choices=synthetic fixture choices|ownership_control=synthetic fixture control|"
            "numeric_scaling_counters=synthetic fixture numeric boundary|"
            "information_identity_effect=synthetic fixture information effect|"
            "rule_dependency=synthetic fixture rule dependency"
        ),
        "evidence_basis_allowed": sorted(["ORACLE_TEXT"], key=encode_canonical),
        "status": status,
        "terminal_assignable": status == "ACTIVE",
        "superseded_by": [],
        "supersession_reason": None,
        "review_provenance": {
            "review_status": "REVIEWED_CONFIRMED",
            "review_basis": "SOURCE_GROUNDED_CARD_REVIEW",
            "evidence_locators": [],
        },
        "family_origin": "B2_NEW",
        "lifecycle_relation": "NEW_TERMINAL_CONCEPT",
    }


def add_new_family(artifacts: dict[str, Any], family: dict[str, Any]) -> None:
    artifacts["catalog"]["families"].append(family)
    artifacts["catalog"]["families"].sort(key=lambda item: encode_canonical(item["family_id"]))
    artifacts["catalog"]["new_family_count"] += 1
    artifacts["catalog"]["catalog_family_count"] += 1


def authority_locator_for_negative(data: ArchiveData) -> dict[str, Any]:
    artifact = AUTHORITY_MEMBERS[0]
    payload = data.member_bytes[artifact][:1]
    return {
        "locator_version": AUTHORITY_LOCATOR_SCHEMA,
        "archive_artifact": artifact,
        "artifact_sha256": sha256_bytes(data.member_bytes[artifact]),
        "byte_offset": 0,
        "byte_length": 1,
        "fragment_sha256": sha256_bytes(payload),
    }


def mutate_active_to_zero(artifacts: dict[str, Any]) -> None:
    counts: Counter[str] = Counter()
    for record in artifacts["classifications"]["classifications"]:
        for assignment in record["requirement_assignments"]:
            counts[assignment["requirement_family_id"]] += 1
    family_id = next(
        family_id
        for family_id, count in counts.items()
        if count == 1
        and any(
            any(
                change.get("family_id") == family_id and change.get("change_kind") == "RETAINED"
                for change in record.get("classification_delta", {}).get("changes", [])
            )
            for record in artifacts["classifications"]["classifications"]
        )
    )
    for record in artifacts["classifications"]["classifications"]:
        if family_id not in {
            item["requirement_family_id"] for item in record["requirement_assignments"]
        }:
            continue
        removed = next(
            item
            for item in record["requirement_assignments"]
            if item["requirement_family_id"] == family_id
        )
        record["requirement_assignments"] = [
            item
            for item in record["requirement_assignments"]
            if item["requirement_family_id"] != family_id
        ]
        record["review_status"] = "REVIEWED_CORRECTED"
        delta = record["classification_delta"]
        delta["changes"] = [item for item in delta["changes"] if item["family_id"] != family_id]
        delta["changes"].append(
            {
                "family_id": family_id,
                "change_kind": "REMOVED",
                "replacement_family_ids": [],
                "rationale": "synthetic removal fixture",
                "evidence_locators": removed["evidence_locators"],
            }
        )
        delta["changes"].sort(
            key=lambda item: (
                encode_canonical(item["family_id"]),
                encode_canonical(item["change_kind"]),
            )
        )
        delta["retained_family_ids"] = [
            item["family_id"] for item in delta["changes"] if item["change_kind"] == "RETAINED"
        ]
        delta["added_family_ids"] = [
            item["family_id"] for item in delta["changes"] if item["change_kind"] == "ADDED"
        ]
        delta["removed_family_ids"] = [
            item["family_id"] for item in delta["changes"] if item["change_kind"] == "REMOVED"
        ]
        delta["superseded_family_ids"] = [
            item["family_id"] for item in delta["changes"] if item["change_kind"] == "SUPERSEDED"
        ]
        record["classification_identity"] = expected_identity(record)
        return
    fail("NEGATIVE_FIXTURE_FAILED", "could not find a singleton family assignment")


TEST_B2_RELATIVE_ROOT = Path("sources/m2_5/closures/B2")
TEST_B2_SUMMARY_PATH = TEST_B2_RELATIVE_ROOT / "verification" / "b2_verification_summary.v1.json"


def test_git(repo: Path, *args: str) -> str:
    env = os.environ.copy()
    env.update(
        {
            "GIT_AUTHOR_NAME": "Manafold B2 verifier test",
            "GIT_AUTHOR_EMAIL": "b2-verifier@example.invalid",
            "GIT_COMMITTER_NAME": "Manafold B2 verifier test",
            "GIT_COMMITTER_EMAIL": "b2-verifier@example.invalid",
        }
    )
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        check=True,
        capture_output=True,
        text=True,
        env=env,
    )
    if args and args[0] == "commit":
        result = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
            env=env,
        )
    return result.stdout.strip()


def write_test_file(repo: Path, relative: Path, content: bytes) -> None:
    path = repo / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)


def make_git_evidence_fixture(base: Path) -> tuple[Path, str, str]:
    repo = base / "repo"
    repo.mkdir()
    test_git(repo, "init", "-b", "master")
    test_git(repo, "config", "core.autocrlf", "false")
    for relative in EXACT_B2_FILES:
        write_test_file(
            repo,
            TEST_B2_RELATIVE_ROOT / relative,
            f"initial:{relative}\n".encode(),
        )
    write_test_file(repo, TEST_B2_SUMMARY_PATH, b"summary:staging\n")
    test_git(repo, "add", ".")
    h_exec = test_git(repo, "commit", "-m", "H_exec")
    write_test_file(repo, TEST_B2_SUMMARY_PATH, b"summary:final\n")
    test_git(repo, "add", str(TEST_B2_SUMMARY_PATH))
    h_evidence = test_git(repo, "commit", "-m", "H_evidence")
    return repo, h_exec, h_evidence


def resolve_test_fixture(repo: Path, execution_commit: str) -> str:
    return resolve_historical_evidence(
        execution_commit,
        repo_root=repo,
        current_b2_dir=repo / TEST_B2_RELATIVE_ROOT,
    )


def git_evidence_regression_self_test() -> int:
    failures: list[str] = []

    def expect_pass(label: str, setup: Callable[[Path, str, str], None]) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo, h_exec, h_evidence = make_git_evidence_fixture(Path(tmp))
            setup(repo, h_exec, h_evidence)
            try:
                actual = resolve_test_fixture(repo, h_exec)
            except B2CheckError as exc:
                failures.append(f"{label}: unexpected {exc.code}")
            else:
                if actual != h_evidence:
                    failures.append(f"{label}: resolved {actual}, expected {h_evidence}")

    def expect_fail(
        label: str,
        expected_code: str,
        setup: Callable[[Path, str, str], str],
    ) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo, h_exec, h_evidence = make_git_evidence_fixture(Path(tmp))
            execution_commit = setup(repo, h_exec, h_evidence)
            try:
                resolve_test_fixture(repo, execution_commit)
            except B2CheckError as exc:
                if exc.code != expected_code:
                    failures.append(f"{label}: found {exc.code}, expected {expected_code}")
            else:
                failures.append(f"{label}: unexpectedly passed")

    expect_pass("EVIDENCE_HEAD", lambda repo, _exec, _evidence: None)

    def ordinary_descendant(repo: Path, _exec: str, _evidence: str) -> None:
        write_test_file(repo, Path("unrelated.txt"), b"ordinary descendant\n")
        test_git(repo, "add", "unrelated.txt")
        test_git(repo, "commit", "-m", "unrelated descendant")

    expect_pass("ORDINARY_DESCENDANT", ordinary_descendant)

    def merge_descendant(repo: Path, h_exec: str, h_evidence: str) -> None:
        test_git(repo, "switch", "--detach", h_exec)
        write_test_file(repo, Path("side.txt"), b"side branch\n")
        test_git(repo, "add", "side.txt")
        side = test_git(repo, "commit", "-m", "side branch")
        test_git(repo, "switch", "--detach", side)
        test_git(repo, "merge", "--no-ff", h_evidence, "-m", "merge evidence as second parent")

    expect_pass("MERGE_DESCENDANT", merge_descendant)

    def execution_not_ancestor(repo: Path, h_exec: str, _evidence: str) -> str:
        test_git(repo, "switch", "--detach", h_exec)
        write_test_file(repo, Path("outsider.txt"), b"outsider\n")
        test_git(repo, "add", "outsider.txt")
        outsider = test_git(repo, "commit", "-m", "outsider execution")
        test_git(repo, "switch", "--detach", h_exec)
        return outsider

    expect_fail(
        "EXECUTION_NOT_ANCESTOR",
        "EXECUTION_COMMIT_NOT_ANCESTOR_REJECTED",
        execution_not_ancestor,
    )

    def wrong_parent(repo: Path, h_exec: str, _evidence: str) -> str:
        test_git(repo, "switch", "--detach", h_exec)
        write_test_file(repo, Path("intermediate.txt"), b"intermediate\n")
        test_git(repo, "add", "intermediate.txt")
        test_git(repo, "commit", "-m", "intermediate")
        write_test_file(repo, TEST_B2_SUMMARY_PATH, b"summary:final\n")
        test_git(repo, "add", str(TEST_B2_SUMMARY_PATH))
        test_git(repo, "commit", "-m", "wrong parent evidence")
        return h_exec

    expect_fail("WRONG_PARENT", "HISTORICAL_EVIDENCE_NOT_FOUND_REJECTED", wrong_parent)

    def additional_diff(repo: Path, h_exec: str, _evidence: str) -> str:
        test_git(repo, "switch", "--detach", h_exec)
        write_test_file(repo, TEST_B2_SUMMARY_PATH, b"summary:final\n")
        write_test_file(repo, Path("extra.txt"), b"extra evidence change\n")
        test_git(repo, "add", ".")
        test_git(repo, "commit", "-m", "summary plus extra file")
        return h_exec

    expect_fail(
        "ADDITIONAL_DIFF",
        "HISTORICAL_EVIDENCE_DIFF_REJECTED",
        additional_diff,
    )

    def current_summary_drift(repo: Path, _h_exec: str, h_evidence: str) -> str:
        test_git(repo, "switch", "--detach", h_evidence)
        write_test_file(repo, TEST_B2_SUMMARY_PATH, b"summary:changed-after-evidence\n")
        test_git(repo, "add", str(TEST_B2_SUMMARY_PATH))
        test_git(repo, "commit", "-m", "tamper current summary")
        return _h_exec

    expect_fail(
        "CURRENT_SUMMARY_DRIFT",
        "HISTORICAL_SUMMARY_BLOB_MISMATCH_REJECTED",
        current_summary_drift,
    )

    def artifact_drift(relative: str) -> Callable[[Path, str, str], str]:
        def mutate(repo: Path, _h_exec: str, h_evidence: str) -> str:
            test_git(repo, "switch", "--detach", h_evidence)
            write_test_file(repo, TEST_B2_RELATIVE_ROOT / relative, b"artifact drift\n")
            test_git(repo, "add", str(TEST_B2_RELATIVE_ROOT / relative))
            test_git(repo, "commit", "-m", f"tamper {relative}")
            return _h_exec

        return mutate

    for label, relative in (
        ("CLASSIFICATION_DRIFT", "card_semantic_classifications.v1.json"),
        ("CATALOG_DRIFT", "requirement_family_catalog.v1.json"),
        ("BOUNDARY_DRIFT", "B2_DESIGN_SPEC.md"),
        ("PROJECTION_DRIFT", "deck_row_classification_refs.v1.csv"),
    ):
        expect_fail(label, "B2_ARTIFACT_DRIFT_REJECTED", artifact_drift(relative))

    def unreachable_evidence(repo: Path, h_exec: str, _h_evidence: str) -> str:
        test_git(repo, "switch", "--detach", h_exec)
        write_test_file(repo, Path("unrelated.txt"), b"no evidence ancestor\n")
        test_git(repo, "add", "unrelated.txt")
        test_git(repo, "commit", "-m", "unrelated branch")
        return h_exec

    expect_fail(
        "UNREACHABLE_EVIDENCE",
        "HISTORICAL_EVIDENCE_NOT_FOUND_REJECTED",
        unreachable_evidence,
    )

    def ambiguous_evidence(repo: Path, h_exec: str, _h_evidence: str) -> str:
        test_git(repo, "switch", "--detach", h_exec)
        write_test_file(repo, TEST_B2_SUMMARY_PATH, b"summary:final\n")
        test_git(repo, "add", str(TEST_B2_SUMMARY_PATH))
        first = test_git(repo, "commit", "-m", "evidence candidate one")
        test_git(repo, "switch", "--detach", h_exec)
        write_test_file(repo, TEST_B2_SUMMARY_PATH, b"summary:final\n")
        test_git(repo, "add", str(TEST_B2_SUMMARY_PATH))
        second = test_git(repo, "commit", "-m", "evidence candidate two")
        test_git(repo, "switch", "--detach", first)
        test_git(repo, "merge", "--no-ff", second, "-m", "ambiguous evidence candidates")
        return h_exec

    expect_fail(
        "AMBIGUOUS_EVIDENCE",
        "HISTORICAL_EVIDENCE_AMBIGUOUS_REJECTED",
        ambiguous_evidence,
    )

    if failures:
        for failure in failures:
            print(f"GIT_EVIDENCE {failure}")
        return EXIT_FAIL
    print("GIT_EVIDENCE_SELF_TEST = PASS (3 positive; 10 negative history fixtures)")
    return EXIT_PASS


def summary_binding_regression_self_test(data: ArchiveData, summary: object) -> int:
    cases: tuple[tuple[str, Callable[[dict[str, Any]], None], str], ...] = (
        (
            "CHECKER_IDENTITY_TAMPER",
            lambda value: value["checker_version_and_identity"].__setitem__("raw_sha256", "0" * 64),
            "SUMMARY_CHECKER_IDENTITY_MISMATCH",
        ),
        (
            "EXECUTION_FINGERPRINT_TAMPER",
            lambda value: value.__setitem__("source_tree_before_fingerprint", "0" * 64),
            "SUMMARY_FINGERPRINT_MISMATCH",
        ),
    )
    failures: list[str] = []
    for label, mutation, expected_code in cases:
        mutated = copy.deepcopy(summary)
        mutation(mutated)
        try:
            validate_summary(data, mutated, allow_staging=False)
        except B2CheckError as exc:
            if exc.code != expected_code:
                failures.append(f"{label}: found {exc.code}, expected {expected_code}")
        else:
            failures.append(f"{label}: unexpectedly passed")
    if failures:
        for failure in failures:
            print(f"SUMMARY_BINDING {failure}")
        return EXIT_FAIL
    print("SUMMARY_BINDING_SELF_TEST = PASS (2 negative metadata fixtures)")
    return EXIT_PASS


def negative_self_test() -> int:
    if git_evidence_regression_self_test() != EXIT_PASS:
        return EXIT_FAIL
    data = load_archive()
    known_answer_test()
    base = read_artifacts()
    validate_model(data, base)
    if summary_binding_regression_self_test(data, base["summary"]) != EXIT_PASS:
        return EXIT_FAIL
    cases: list[tuple[str, Callable[[dict[str, Any]], None], str | None]] = []

    def case(
        code: str, mutation: Callable[[dict[str, Any]], None], extra: str | None = None
    ) -> None:
        cases.append((code, mutation, extra))

    case("MISSING_CLASSIFICATION_REJECTED", lambda a: a["classifications"]["classifications"].pop())
    case(
        "DUPLICATE_ORACLE_IDENTITY_REJECTED",
        lambda a: a["classifications"]["classifications"].append(
            copy.deepcopy(a["classifications"]["classifications"][0])
        ),
    )
    case(
        "UNKNOWN_ORACLE_IDENTITY_REJECTED",
        lambda a: a["classifications"]["classifications"][0].__setitem__(
            "oracle_semantic_identity", "00000000-0000-0000-0000-000000000000"
        ),
    )
    case(
        "NONTERMINAL_CLASSIFICATION_REJECTED",
        lambda a: first_classification(a).__setitem__("review_status", "IN_REVIEW"),
    )
    case("MISSING_DECK_ROW_REFERENCE_REJECTED", lambda a: a["projection"].pop())
    case(
        "UNKNOWN_DECK_ROW_REFERENCE_REJECTED",
        lambda a: a["projection"].append({**a["projection"][0], "deck_row_id": "unknown:row"}),
    )
    case(
        "DECK_ROW_OSI_REBIND_REJECTED",
        lambda a: a["projection"][0].__setitem__(
            "oracle_semantic_identity",
            next(
                osi
                for osi in [
                    x["oracle_semantic_identity"] for x in a["classifications"]["classifications"]
                ]
                if osi != a["projection"][0]["oracle_semantic_identity"]
            ),
        ),
    )

    def fork_reused(a: dict[str, Any]) -> None:
        row = next(
            row
            for row in a["projection"]
            if sum(
                1
                for candidate in a["projection"]
                if candidate["oracle_semantic_identity"] == row["oracle_semantic_identity"]
            )
            > 1
        )
        row["terminal_requirement_ids"] = '["cap.activated_ability"]'

    case("REUSED_ORACLE_IDENTITY_FORK_REJECTED", fork_reused)
    case(
        "SOURCE_DIGEST_MISMATCH_REJECTED",
        lambda a: first_classification(a, assigned=True)["source_identity"].__setitem__(
            "oracle_source_record_id", "00000000-0000-0000-0000-000000000000"
        ),
    )

    def invalid_locator(a: dict[str, Any]) -> None:
        _, assignment = first_assignment_record(a)
        assignment["evidence_locators"][0]["json_pointer"] = "/missing_field"

    case("SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED", invalid_locator)

    def disallowed_basis(a: dict[str, Any]) -> None:
        _, assignment = first_assignment_record(a)
        assignment["evidence_basis"] = "FORMAT_POLICY"

    case("DISALLOWED_EVIDENCE_BASIS_REJECTED", disallowed_basis)

    def locator_kind_mismatch(a: dict[str, Any]) -> None:
        record, assignment = first_assignment_record(a)
        family = catalog_by_id_from_artifacts(a)[assignment["requirement_family_id"]]
        family["evidence_basis_allowed"] = sorted(
            [*family["evidence_basis_allowed"], "FORMAT_POLICY"], key=encode_canonical
        )
        assignment["evidence_basis"] = "FORMAT_POLICY"
        del record

    case("EVIDENCE_BASIS_LOCATOR_KIND_MISMATCH_REJECTED", locator_kind_mismatch)

    def no_card_evidence(a: dict[str, Any]) -> None:
        _, assignment = first_assignment_record(a)
        assignment["evidence_basis"] = "FORMAT_POLICY"
        family = catalog_by_id_from_artifacts(a)[assignment["requirement_family_id"]]
        family["evidence_basis_allowed"] = sorted(
            [*family["evidence_basis_allowed"], "FORMAT_POLICY"], key=encode_canonical
        )
        assignment["evidence_locators"] = [authority_locator_for_negative(data)]

    case("CARD_SIDE_EVIDENCE_MISSING_REJECTED", no_card_evidence)
    case(
        "UNKNOWN_REQUIREMENT_FAMILY_REJECTED",
        lambda a: first_assignment_record(a)[1].__setitem__("requirement_family_id", "cap.unknown"),
    )

    def superseded_assigned(a: dict[str, Any]) -> None:
        ids = list(catalog_by_id_from_artifacts(a))
        mutate_superseded(a, [ids[1]])

    case("SUPERSEDED_FAMILY_ASSIGNED_REJECTED", superseded_assigned)
    case(
        "ACTIVE_UNASSIGNED_FAMILY_ASSIGNED_REJECTED",
        lambda a: mutate_status(a, "ACTIVE_UNASSIGNED", "ACTIVE_EQUIVALENT"),
    )
    case(
        "RETIRED_FAMILY_ASSIGNED_REJECTED",
        lambda a: mutate_status(a, "RETIRED", "RETIRED_NO_SUCCESSOR"),
    )
    case("SUPERSEDED_WITHOUT_SUCCESSOR_REJECTED", lambda a: mutate_superseded(a, []))

    def retired_with_successor(a: dict[str, Any]) -> None:
        ids = list(catalog_by_id_from_artifacts(a))
        family_id = first_active_family(a)
        family = catalog_by_id_from_artifacts(a)[family_id]
        family["status"] = "RETIRED"
        family["terminal_assignable"] = False
        family["lifecycle_relation"] = "RETIRED_NO_SUCCESSOR"
        family["superseded_by"] = [ids[1] if ids[1] != family_id else ids[2]]

    case("RETIRED_WITH_SUCCESSOR_REJECTED", retired_with_successor)
    case("SUPERSESSION_UNKNOWN_TARGET_REJECTED", lambda a: mutate_superseded(a, ["cap.unknown"]))

    def self_target(a: dict[str, Any]) -> None:
        family_id = first_active_family(a)
        mutate_superseded(a, [family_id])

    case("SUPERSESSION_SELF_TARGET_REJECTED", self_target)

    def nonassignable_target(a: dict[str, Any]) -> None:
        ids = list(catalog_by_id_from_artifacts(a))
        source_id = ids[0]
        target_id = ids[1]
        other_id = ids[2]
        source = catalog_by_id_from_artifacts(a)[source_id]
        target = catalog_by_id_from_artifacts(a)[target_id]
        source["status"] = "SUPERSEDED"
        source["terminal_assignable"] = False
        source["lifecycle_relation"] = "SUPERSEDED_BY_REPLACEMENT"
        source["supersession_reason"] = "synthetic"
        source["superseded_by"] = [target_id]
        target["status"] = "SUPERSEDED"
        target["terminal_assignable"] = False
        target["lifecycle_relation"] = "SUPERSEDED_BY_REPLACEMENT"
        target["supersession_reason"] = "synthetic"
        target["superseded_by"] = [other_id]

    case("SUPERSESSION_NONASSIGNABLE_TARGET_REJECTED", nonassignable_target)
    case("HISTORICAL_FAMILY_MISSING_REJECTED", lambda a: a["catalog"]["families"].pop())

    def historical_tamper(a: dict[str, Any]) -> None:
        family = a["catalog"]["families"][0]
        family["historical_rev3"]["record"]["name"] = "tampered"

    case("HISTORICAL_REV3_BLOCK_TAMPER_REJECTED", historical_tamper)

    def historical_projection(a: dict[str, Any]) -> None:
        a["catalog"]["families"][0]["historical_definition"]["rev3_name"] = "tampered"

    case("HISTORICAL_DEFINITION_PROJECTION_MISMATCH_REJECTED", historical_projection)
    case("ACTIVE_WITH_ZERO_ASSIGNMENTS_REJECTED", mutate_active_to_zero)
    case(
        "SPECULATIVE_NEW_FAMILY_REJECTED",
        lambda a: add_new_family(a, make_new_family("req.b2.speculative")),
    )

    def silent_change(a: dict[str, Any]) -> None:
        _, assignment = first_assignment_record(a)
        current = assignment["requirement_family_id"]
        assignment["requirement_family_id"] = next(
            family_id
            for family_id, family in catalog_by_id_from_artifacts(a).items()
            if family_id != current and family.get("status") == "ACTIVE"
        )

    case("SILENT_CLASSIFICATION_CHANGE_REJECTED", silent_change)
    case(
        "CORRECTION_WITHOUT_RATIONALE_REJECTED",
        lambda a: first_classification(a, assigned=True)["classification_delta"]["changes"][
            0
        ].__setitem__("rationale", ""),
    )
    case(
        "CORRECTION_WITHOUT_EVIDENCE_REJECTED",
        lambda a: first_classification(a, assigned=True)["classification_delta"]["changes"][
            0
        ].__setitem__("evidence_locators", []),
    )

    def new_historical(a: dict[str, Any]) -> None:
        family = make_new_family("req.b2.with_history", "ACTIVE")
        family["historical_rev3"] = {}
        add_new_family(a, family)

    case("NEW_FAMILY_HISTORICAL_BLOCK_PRESENT_REJECTED", new_historical)
    case(
        "WRONG_CLASSIFICATION_SCHEMA_REJECTED",
        lambda a: a["classifications"].__setitem__("schema", "wrong.v0"),
    )
    case("WRONG_CLOSURE_SCHEMA_REJECTED", lambda a: a["closure"].__setitem__("schema", "wrong.v0"))
    case(
        "EVIDENCE_DIGEST_TAMPER_REJECTED",
        lambda a: first_classification(a, assigned=True)["source_evidence_digest"].__setitem__(
            "digest_hex", "f" * 64
        ),
    )
    case("B2_FILE_INVENTORY_REJECTED", lambda a: None, "unexpected-extra.json")
    case(
        "OTHER_GATE_PROMOTION_REJECTED",
        lambda a: a["closure"].__setitem__("OFFICIAL_RULE_CITATION_CLOSURE", "PASS"),
    )
    case(
        "DECK_LOCK_PROMOTION_REJECTED",
        lambda a: a["closure"]["gate_status"].__setitem__("DECK_PAIR_LOCKED", "YES"),
    )
    case(
        "M3_PROMOTION_REJECTED",
        lambda a: a["closure"]["gate_status"].__setitem__("M3_STARTED", "YES"),
    )

    failures: list[str] = []
    for expected_code, mutation, extra in cases:
        mutated = copy.deepcopy(base)
        mutation(mutated)
        try:
            validate_model(
                data,
                mutated,
                extra_inventory={extra} if extra else None,
                validate_bound_files=False,
            )
        except B2CheckError as exc:
            if exc.code != expected_code:
                failures.append(f"{expected_code}: found {exc.code}")
                print(f"NEGATIVE {expected_code}: WRONG_CODE {exc.code}")
            else:
                print(f"NEGATIVE {expected_code}: rejected ({exc.status})")
        else:
            failures.append(f"{expected_code}: mutation unexpectedly passed")
            print(f"NEGATIVE {expected_code}: UNEXPECTED_PASS")
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return EXIT_FAIL
    print("NEGATIVE_SELF_TEST = PASS (38/38 exact mutation codes)")
    return EXIT_PASS


def run_positive() -> int:
    data = load_archive()
    known_answer_test()
    artifacts = read_artifacts()
    _, usage, summary_status = validate_model(data, artifacts)
    evidence_stage = "H_exec staging" if summary_status == "STAGING" else "final evidence"
    print(
        f"B2_VERIFIER = PASS ({evidence_stage}; 402 classifications, "
        f"{sum(usage.values())} assignments)"
    )
    return EXIT_PASS


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--negative-self-test", action="store_true")
    args = parser.parse_args()
    try:
        if args.negative_self_test:
            return negative_self_test()
        return run_positive()
    except B2CheckError as exc:
        print(f"{exc.status} {exc.code}: {exc.message}")
        return EXIT_FAIL if exc.status == "FAIL" else EXIT_BLOCKED


if __name__ == "__main__":
    raise SystemExit(main())
