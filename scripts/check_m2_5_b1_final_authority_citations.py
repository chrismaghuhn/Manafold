#!/usr/bin/env python3
"""Fail-closed verifier for the terminal M2.5.B1 authority closure.

The historical B1 V2 checker remains a separate regression.  This checker
consumes the immutable B2 terminal snapshot, binds every input byte, and
validates the reviewed boundary-to-authority dependency graph without using
lexical markers as semantic authority.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
B1_DIR = ROOT / "sources" / "m2_5" / "closures" / "B1"
B2_DIR = ROOT / "sources" / "m2_5" / "closures" / "B2"
PROVENANCE_PATH = ROOT / "sources" / "m2_5" / "pre_research" / "REV3" / "IMPORT_PROVENANCE.json"
ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"
ARCHIVE_RELATIVE_PATH = Path("m2_5/Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip")
EXPECTED_ARCHIVE_SHA256 = "99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90"

EXPECTED_CITATIONS_SCHEMA = "manafold.m2.5.b1.official-authority-citations.v3"
EXPECTED_CLOSURE_SCHEMA = "manafold.m2.5.b1.official-authority-citation-closure.v2"
EXPECTED_NEGATIVE_SCHEMA = "manafold.m2.5.b1.final-negative-test-matrix.v1"
EXPECTED_SUMMARY_SCHEMA = "manafold.m2.5.b1.final-verification-summary.v1"
CITATIONS_NAME = "official_authority_citations.v3.json"
CLOSURE_NAME = "official_authority_citation_closure.v2.json"
REPORT_NAME = "AUTHORITY_CITATION_FINAL_REPORT.md"
MATRIX_NAME = "verification/b1_final_negative_test_matrix.v1.json"
SUMMARY_NAME = "verification/b1_final_verification_summary.v1.json"

EXPECTED_UNIVERSE = [
    "banned_restricted",
    "commander_1v1",
    "commander_general",
    "commander_legends_release_notes",
    "comprehensive_rules",
    "kaldheim_release_notes",
    "magic_2013_release_notes",
]
MAGIC_2013 = "magic_2013_release_notes"
EXPECTED_ACTIVE_UNASSIGNED = {
    "cap.commander_zone_choice",
    "cap.copy_token_batch",
    "cap.delayed_sacrifice",
    "cap.sacrifice_additional_cost",
    "cap.state_based_actions",
    "cap.token_replacement",
}
EXPECTED_METRICS = {
    "catalog_family_count": 216,
    "classification_count": 402,
    "terminal_assignment_edge_count": 1883,
    "projection_row_count": 441,
    "active_family_count": 210,
    "active_unassigned_family_count": 6,
}
REQUIRED_B2_FILES = {
    "B2_DESIGN_SPEC.md",
    "card_semantic_classifications.v1.json",
    "deck_row_classification_refs.v1.csv",
    "requirement_family_catalog.v1.json",
    "classification_closure.v1.json",
    "CLASSIFICATION_REPORT.md",
    "verification/b2_negative_test_matrix.v1.json",
    "verification/b2_verification_summary.v1.json",
}
REQUIRED_GATE_STATUSES = {
    "CLASSIFICATION_REFERENCE_CLOSURE": "PASS",
    "OFFICIAL_RULE_CITATION_CLOSURE": "PASS",
    "DECLARED_INTERACTION_MODEL_CLOSURE": "BLOCKED",
    "REV2_REUSE_RATIO_REPRODUCIBLE": "BLOCKED",
    "RANKING_UNCERTAINTY_PROPAGATION": "BLOCKED",
}
REQUIRED_FALSE_FLAGS = {
    "DECK_PAIR_LOCKED": False,
    "AUTHORITATIVE_RANKING_AVAILABLE": False,
    "M3_STARTED": False,
}
FORBIDDEN_AUTHORITY_PROOF_TERMS = ("lexical", "regex", "marker scan", "keyword scan")


class FinalError(Exception):
    def __init__(self, status: str, code: str, message: str) -> None:
        super().__init__(f"[{code}] {message}")
        self.status = status
        self.code = code
        self.message = message


def fail(code: str, message: str) -> None:
    raise FinalError("FAIL", code, message)


def blocked(code: str, message: str) -> None:
    raise FinalError("BLOCKED", code, message)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_json(path: Path, code: str = "ARTIFACT_UNREADABLE") -> tuple[dict, bytes]:
    try:
        raw = path.read_bytes()
        value = json.loads(raw.decode("utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        blocked(code, f"cannot read {path}: {exc}")
    if not isinstance(value, dict):
        fail("SCHEMA_MISMATCH", f"{path} must contain a JSON object")
    return value, raw


def load_legacy_helpers():
    path = ROOT / "scripts" / "check_m2_5_b1_authority_citations.py"
    spec = importlib.util.spec_from_file_location("manafold_historical_b1", path)
    if spec is None or spec.loader is None:
        blocked("HELPER_UNAVAILABLE", f"cannot load historical B1 helper: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def resolve_archive() -> tuple[Path, str]:
    if not PROVENANCE_PATH.is_file():
        blocked("ARCHIVE_PROVENANCE_UNAVAILABLE", f"missing {PROVENANCE_PATH}")
    provenance, _ = load_json(PROVENANCE_PATH, "ARCHIVE_PROVENANCE_UNAVAILABLE")
    package = provenance.get("source_package")
    if (
        not isinstance(package, dict)
        or package.get("storage_class") != "MAINTAINER_PRIVATE_ARCHIVE"
    ):
        blocked("ARCHIVE_PROVENANCE_INVALID", "REV3 source package is not a private archive")
    base = os.environ.get(ARCHIVE_ENV_VAR)
    if not base:
        blocked("ARCHIVE_SOURCE_UNAVAILABLE", f"environment variable {ARCHIVE_ENV_VAR} is unset")
    path = Path(base) / ARCHIVE_RELATIVE_PATH
    expected = package.get("sha256")
    if expected != EXPECTED_ARCHIVE_SHA256:
        fail(
            "ARCHIVE_PROVENANCE_DIGEST_MISMATCH",
            "IMPORT_PROVENANCE does not carry the pinned archive SHA",
        )
    if not path.is_file():
        blocked("ARCHIVE_SOURCE_UNAVAILABLE", f"pinned archive is missing: {path}")
    actual = sha256_bytes(path.read_bytes())
    if actual != EXPECTED_ARCHIVE_SHA256:
        fail(
            "ARCHIVE_DIGEST_MISMATCH",
            f"pinned archive has {actual}, expected {EXPECTED_ARCHIVE_SHA256}",
        )
    return path, expected


def b2_input_records(b2_dir: Path) -> tuple[list[dict], dict[str, bytes]]:
    records: list[dict] = []
    raw_by_name: dict[str, bytes] = {}
    for relative in sorted(REQUIRED_B2_FILES):
        path = b2_dir / relative
        if not path.is_file():
            blocked("B2_INPUT_MISSING", f"missing B2 input: {path}")
        raw = path.read_bytes()
        raw_by_name[relative] = raw
        records.append({"path": relative, "raw_sha256": sha256_bytes(raw)})
    return records, raw_by_name


def parse_boundary(value: object, family_id: str) -> dict[str, str]:
    if not isinstance(value, str) or not value.startswith("B2_SEMANTIC_BOUNDARY_V1|"):
        fail("B2_BOUNDARY_INVALID", f"family {family_id} lacks a B2 semantic boundary")
    fields: dict[str, str] = {}
    for part in value.split("|")[1:]:
        if "=" not in part:
            fail("B2_BOUNDARY_INVALID", f"family {family_id} has malformed boundary field")
        key, field_value = part.split("=", 1)
        if not key or not field_value or key in fields:
            fail("B2_BOUNDARY_INVALID", f"family {family_id} has invalid boundary field {key!r}")
        fields[key] = field_value
    if fields.get("family_id") != family_id:
        fail("B2_BOUNDARY_INVALID", f"boundary family identity mismatch for {family_id}")
    required = {
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
    }
    if set(fields) != required:
        fail("B2_BOUNDARY_INVALID", f"boundary field set mismatch for {family_id}")
    return fields


def resolve_citation(citation: dict, authority: dict, artifacts: dict[str, bytes], legacy) -> None:
    for key in ("citation_id", "citation_kind", "why_required"):
        if not isinstance(citation.get(key), str) or not citation[key].strip():
            fail(
                "CITATION_FIELD_EMPTY", f"citation {citation.get('citation_id')!r} has empty {key}"
            )
    path = authority["artifact_identity"].get("artifact_path")
    if not isinstance(path, str) or path not in artifacts:
        fail("CITATION_ARTIFACT_UNKNOWN", f"citation artifact is not available: {path!r}")
    try:
        legacy.validate_citation(citation, artifacts, path)
    except Exception as exc:
        if hasattr(exc, "code"):
            fail(str(exc.code), str(exc.message))
        raise


def validate_authorities(citations: dict, reader, legacy) -> dict[str, tuple[dict, dict]]:
    if citations.get("schema") != EXPECTED_CITATIONS_SCHEMA:
        fail(
            "SCHEMA_MISMATCH", f"unexpected B1.Final citations schema: {citations.get('schema')!r}"
        )
    universe = citations.get("input_universe")
    if (
        not isinstance(universe, dict)
        or universe.get("authority_ids_in_order") != EXPECTED_UNIVERSE
    ):
        fail(
            "AUTHORITY_UNIVERSE_MISMATCH",
            "B1.Final does not declare the exact seven-authority universe",
        )
    if universe.get("archive_sha256") != EXPECTED_ARCHIVE_SHA256:
        fail("ARCHIVE_DIGEST_MISMATCH", "B1.Final archive binding is not the pinned SHA")
    register_raw = reader.read("source/official_authority_register_REV3.json")
    if universe.get("source_register_sha256") != sha256_bytes(register_raw):
        fail("REGISTER_DIGEST_MISMATCH", "B1.Final source-register digest is stale")
    register = {entry["authority_id"]: entry for entry in json.loads(register_raw.decode("utf-8"))}
    authorities = citations.get("authorities")
    if not isinstance(authorities, list) or len(authorities) != len(EXPECTED_UNIVERSE):
        fail(
            "AUTHORITY_UNIVERSE_INCOMPLETE",
            "B1.Final authorities must contain exactly seven records",
        )
    seen: set[str] = set()
    citation_index: dict[str, tuple[dict, dict]] = {}
    artifacts: dict[str, bytes] = {}
    for record in authorities:
        aid = record.get("authority_id")
        if aid not in EXPECTED_UNIVERSE:
            fail("AUTHORITY_UNKNOWN", f"unknown authority {aid!r}")
        if aid in seen:
            fail("AUTHORITY_DUPLICATE", f"duplicate authority {aid!r}")
        seen.add(aid)
        pinned = register.get(aid)
        if pinned is None:
            fail("AUTHORITY_REGISTER_MISSING", f"authority {aid!r} absent from REV3 register")
        identity = record.get("artifact_identity")
        if not isinstance(identity, dict):
            fail("AUTHORITY_RECORD_INCOMPLETE", f"authority {aid} lacks artifact_identity")
        field_pairs = {
            "source_url": (record.get("original_official_url"), pinned.get("source_url")),
            "artifact_path": (identity.get("artifact_path"), pinned.get("artifact_path")),
            "artifact_sha256": (identity.get("artifact_sha256"), pinned.get("artifact_sha256")),
            "retrieval_time": (identity.get("retrieval_time"), pinned.get("retrieved_at")),
            "raw_artifact_available": (
                identity.get("raw_artifact_available"),
                pinned.get("raw_artifact_available"),
            ),
            "acquisition_http_status": (
                identity.get("acquisition_http_status"),
                pinned.get("http_status"),
            ),
            "acquisition_error": (identity.get("acquisition_error"), pinned.get("error")),
        }
        for field, (actual, expected) in field_pairs.items():
            if actual != expected:
                fail(
                    "REGISTER_CROSS_BINDING",
                    f"authority {aid} field {field} disagrees with REV3 register",
                )
        role = record.get("authority_role")
        url = record.get("original_official_url")
        if not isinstance(role, str) or not role.startswith("OFFICIAL_"):
            fail("AUTHORITY_NOT_OFFICIAL", f"authority {aid} role is not official")
        if not isinstance(url, str) or url.split("/")[2] not in {
            "magic.wizards.com",
            "media.wizards.com",
        }:
            fail("AUTHORITY_NOT_OFFICIAL", f"authority {aid} URL is not an official Wizards origin")
        status = record.get("citation_status")
        if status not in {"CITED", "NOT_REQUIRED_WITH_PROOF"}:
            fail("NON_TERMINAL_STATUS", f"authority {aid} is not terminal")
        path = identity.get("artifact_path")
        if status == "CITED":
            if not isinstance(path, str) or not isinstance(identity.get("artifact_sha256"), str):
                fail("CITED_RECORD_INCOMPLETE", f"CITED authority {aid} lacks artifact identity")
            if path not in artifacts:
                artifacts[path] = reader.read(path)
            if sha256_bytes(artifacts[path]) != identity["artifact_sha256"]:
                fail("ARTIFACT_DIGEST_MISMATCH", f"authority artifact digest mismatch for {aid}")
            cited = record.get("citations")
            if not isinstance(cited, list) or not cited:
                fail("EMPTY_CITATIONS", f"CITED authority {aid} has no citations")
            for citation in cited:
                if not isinstance(citation, dict):
                    fail("CITATION_RECORD_INVALID", f"authority {aid} has a non-object citation")
                cid = citation.get("citation_id")
                if cid in citation_index:
                    fail("CITATION_DUPLICATE", f"duplicate citation id {cid!r}")
                resolve_citation(citation, record, artifacts, legacy)
                citation_index[cid] = (record, citation)
        else:
            if aid != MAGIC_2013:
                fail(
                    "UNEXPECTED_NOT_REQUIRED_AUTHORITY",
                    "only Magic 2013 may be NOT_REQUIRED_WITH_PROOF",
                )
            proof = record.get("not_required_proof")
            if not isinstance(proof, dict):
                fail("PROOF_OBJECT_MISSING", "Magic 2013 proof is missing")
            primary = proof.get("primary_evidence")
            if (
                not isinstance(primary, dict)
                or primary.get("model") != "b1_final_semantic_dependency_graph"
            ):
                fail("MAGIC_2013_PROOF_SCOPE", "Magic 2013 proof is not bound to B1.Final graph")
            if primary.get("required_dependency_edge_count") != 0:
                fail("MAGIC_2013_REQUIRED", "Magic 2013 has a required dependency edge")
    if seen != set(EXPECTED_UNIVERSE):
        fail("AUTHORITY_UNIVERSE_INCOMPLETE", "B1.Final authority set is incomplete")
    return citation_index


def copy_json(value: dict) -> dict:
    return copy.deepcopy(value)


def validate_b2_binding(
    model: dict, b2_dir: Path, citation_index: dict, reader
) -> tuple[dict, dict, dict, dict, dict[str, int]]:
    binding = model.get("b2_inputs")
    if (
        not isinstance(binding, list)
        or {x.get("path") for x in binding if isinstance(x, dict)} != REQUIRED_B2_FILES
    ):
        fail("B2_INPUT_SET_MISMATCH", "B1.Final does not bind the exact B2 input set")
    bound_paths: set[str] = set()
    for entry in binding:
        if not isinstance(entry, dict) or entry.get("path") in bound_paths:
            fail("B2_INPUT_SET_MISMATCH", "B1.Final B2 input binding is duplicate or malformed")
        rel = entry.get("path")
        bound_paths.add(rel)
        actual_path = b2_dir / rel
        if not actual_path.is_file():
            fail("B2_INPUT_MISSING", f"missing bound B2 input {actual_path}")
        if entry.get("raw_sha256") != sha256_bytes(actual_path.read_bytes()):
            fail("B2_INPUT_DIGEST_MISMATCH", f"B2 input digest mismatch for {rel}")
    cat, _ = load_json(b2_dir / "requirement_family_catalog.v1.json")
    classifications, _ = load_json(b2_dir / "card_semantic_classifications.v1.json")
    closure, _ = load_json(b2_dir / "classification_closure.v1.json")
    if closure.get("schema") != "manafold.m2.5.b2.classification-closure.v1":
        fail("B2_CLOSURE_SCHEMA", "B2 classification closure schema is not accepted")
    if closure.get("CLASSIFICATION_REFERENCE_CLOSURE") != "PASS":
        fail("B2_CLOSURE_STATUS", "B2 classification reference closure is not PASS")
    if (
        closure.get("OFFICIAL_RULE_CITATION_CLOSURE") != "BLOCKED"
        or closure.get("block_reason") != "PENDING_B1_FINAL"
    ):
        fail("B2_HANDOFF_STATUS", "B2 historical handoff is not BLOCKED/PENDING_B1_FINAL")
    if (
        cat.get("schema") != "manafold.m2.5.b2.requirement-family-catalog.v1"
        or cat.get("catalog_family_count") != 216
    ):
        fail("B2_CATALOG_UNIVERSE", "B2 catalog is not the expected 216-family snapshot")
    if classifications.get("schema") != "manafold.m2.5.b2.card-semantic-classifications.v1":
        fail("B2_CLASSIFICATION_SCHEMA", "B2 classifications schema is not accepted")
    records = classifications.get("classifications")
    families = cat.get("families")
    if (
        not isinstance(records, list)
        or len(records) != 402
        or not isinstance(families, list)
        or len(families) != 216
    ):
        fail("B2_UNIVERSE_COUNT", "B2 classification or catalog count changed")
    family_by_id = {f.get("family_id"): f for f in families if isinstance(f, dict)}
    if len(family_by_id) != 216:
        fail("B2_CATALOG_DUPLICATE", "B2 catalog family IDs are not unique")
    status_counts = {
        status: sum(f.get("status") == status for f in families)
        for status in {"ACTIVE", "ACTIVE_UNASSIGNED", "SUPERSEDED", "RETIRED"}
    }
    if status_counts != {"ACTIVE": 210, "ACTIVE_UNASSIGNED": 6, "SUPERSEDED": 0, "RETIRED": 0}:
        fail("B2_LIFECYCLE_COUNTS", f"B2 lifecycle counts changed: {status_counts}")
    active_unassigned = {f["family_id"] for f in families if f.get("status") == "ACTIVE_UNASSIGNED"}
    if active_unassigned != EXPECTED_ACTIVE_UNASSIGNED:
        fail("B2_ACTIVE_UNASSIGNED_SET", "B2 ACTIVE_UNASSIGNED vocabulary changed")
    used: set[str] = set()
    assignment_counts: dict[str, int] = {}
    osi_by_family: dict[str, set[str]] = {}
    seen_osi: set[str] = set()
    for record in records:
        osi = record.get("oracle_semantic_identity")
        if not isinstance(osi, str) or osi in seen_osi:
            fail("B2_CLASSIFICATION_UNIVERSE", "B2 OracleSemanticIdentity is missing or duplicated")
        seen_osi.add(osi)
        assignments = record.get("requirement_assignments")
        if not isinstance(assignments, list):
            fail("B2_ASSIGNMENTS_INVALID", f"B2 assignments missing for {osi}")
        local: set[str] = set()
        for assignment in assignments:
            fid = assignment.get("requirement_family_id") if isinstance(assignment, dict) else None
            if fid in local:
                fail("B2_ASSIGNMENT_DUPLICATE", f"duplicate B2 assignment {fid} for {osi}")
            local.add(fid)
            family = family_by_id.get(fid)
            if family is None:
                fail("B2_UNKNOWN_FAMILY", f"B2 assignment references unknown family {fid}")
            if family.get("status") != "ACTIVE" or family.get("terminal_assignable") is not True:
                fail("B2_NONACTIVE_ASSIGNMENT", f"B2 assignment references non-ACTIVE family {fid}")
            used.add(fid)
            assignment_counts[fid] = assignment_counts.get(fid, 0) + 1
            osi_by_family.setdefault(fid, set()).add(osi)
    if len(seen_osi) != 402 or len(used) != 210 or sum(assignment_counts.values()) != 1883:
        fail("B2_ASSIGNMENT_COUNTS", "B2 terminal assignment counts changed")
    metrics = model.get("b2_metrics")
    if metrics != EXPECTED_METRICS:
        fail("B2_METRICS_MISMATCH", f"B1.Final expected metrics differ: {metrics!r}")
    return (
        cat,
        classifications,
        closure,
        family_by_id,
        assignment_counts | {f"__osi__:{k}": len(v) for k, v in osi_by_family.items()},
    )


def citation_lookup(
    citation_index: dict[str, tuple[dict, dict]], authority_id: str, citation_id: str
) -> tuple[dict, dict]:
    if authority_id == MAGIC_2013:
        fail("NOT_REQUIRED_AUTHORITY_EDGE", "dependency graph points into Magic 2013")
    pair = citation_index.get(citation_id)
    if pair is None:
        fail("CITATION_UNKNOWN", f"unknown citation {citation_id}")
    record, citation = pair
    if record.get("authority_id") != authority_id:
        fail(
            "CITATION_AUTHORITY_MISMATCH",
            f"citation {citation_id} belongs to {record.get('authority_id')}, not {authority_id}",
        )
    return record, citation


def validate_dependency_model(
    model: dict,
    cat: dict,
    family_by_id: dict,
    assignment_counts: dict[str, int],
    citation_index: dict,
) -> dict:
    deps = model.get("family_dependencies")
    if not isinstance(deps, list):
        fail("DEPENDENCY_MODEL_MISSING", "family_dependencies is missing")
    used = {key for key in assignment_counts if not key.startswith("__osi__:")}
    dep_by_family: dict[str, dict] = {}
    for dep in deps:
        if not isinstance(dep, dict):
            fail("DEPENDENCY_RECORD_INVALID", "family dependency is not an object")
        fid = dep.get("family_id")
        if fid not in family_by_id:
            fail("DEPENDENCY_FAMILY_UNKNOWN", f"unknown dependency family {fid}")
        if fid in dep_by_family:
            fail("DEPENDENCY_FAMILY_DUPLICATE", f"duplicate dependency family {fid}")
        dep_by_family[fid] = dep
        family = family_by_id[fid]
        if family.get("status") != "ACTIVE" or dep.get("lifecycle") != "ACTIVE":
            fail("DEPENDENCY_LIFECYCLE_MISMATCH", f"dependency family {fid} is not ACTIVE")
        usage = dep.get("terminal_usage")
        if (
            not isinstance(usage, dict)
            or usage.get("terminal_assignment_count") != assignment_counts[fid]
            or usage.get("terminal_osi_count") != assignment_counts.get(f"__osi__:{fid}")
        ):
            fail("TERMINAL_USAGE_MISMATCH", f"terminal usage mismatch for {fid}")
        binding = dep.get("b2_boundary_binding")
        if not isinstance(binding, dict) or binding.get("exact_boundary") != family.get(
            "precise_semantic_definition"
        ):
            fail("B2_BOUNDARY_BINDING", f"dependency boundary mismatch for {fid}")
        if binding.get("boundary_sha256") != sha256_bytes(
            family["precise_semantic_definition"].encode("utf-8")
        ):
            fail("B2_BOUNDARY_DIGEST", f"dependency boundary digest mismatch for {fid}")
        edges = dep.get("authority_dependencies")
        if not isinstance(edges, list) or not edges:
            fail("MISSING_AUTHORITY_DEPENDENCY", f"family {fid} has no authority dependency")
        edge_ids: set[str] = set()
        for edge in edges:
            if not isinstance(edge, dict):
                fail("DEPENDENCY_EDGE_INVALID", f"family {fid} has a malformed edge")
            cid = edge.get("citation_id")
            if cid in edge_ids:
                fail("DEPENDENCY_EDGE_DUPLICATE", f"family {fid} repeats citation {cid}")
            edge_ids.add(cid)
            authority_id = edge.get("authority_id")
            if (
                not isinstance(authority_id, str)
                or not isinstance(edge.get("semantic_role"), str)
                or not isinstance(edge.get("rationale"), str)
                or not edge["rationale"].strip()
            ):
                fail("DEPENDENCY_EDGE_INVALID", f"family {fid} has incomplete authority edge")
            if any(term in edge["rationale"].lower() for term in FORBIDDEN_AUTHORITY_PROOF_TERMS):
                fail(
                    "LEXICAL_AUTHORITY_PROOF",
                    f"family {fid} uses lexical tooling as authority proof",
                )
            citation_lookup(citation_index, authority_id, cid)
        required = set(dep.get("required_citation_ids", []))
        if required != edge_ids:
            fail(
                "COMPOSITE_DEPENDENCY_INCOMPLETE",
                f"family {fid} required citation set disagrees with edges",
            )
    if set(dep_by_family) != used:
        missing = sorted(used - set(dep_by_family))
        extra = sorted(set(dep_by_family) - used)
        fail(
            "DEPENDENCY_FAMILY_SET",
            f"family dependency set mismatch; missing={missing} extra={extra}",
        )
    unassigned = model.get("active_unassigned_handling")
    if (
        not isinstance(unassigned, list)
        or {x.get("family_id") for x in unassigned if isinstance(x, dict)}
        != EXPECTED_ACTIVE_UNASSIGNED
    ):
        fail("ACTIVE_UNASSIGNED_HANDLING", "ACTIVE_UNASSIGNED families are not explicitly handled")
    for record in unassigned:
        if (
            record.get("terminal_assignment_count") != 0
            or record.get("card_semantic_dependency_required") is not False
        ):
            fail(
                "ACTIVE_UNASSIGNED_REQUIRED",
                f"ACTIVE_UNASSIGNED family {record.get('family_id')} was treated as card-required",
            )
    policy = model.get("format_policy_dependencies")
    if not isinstance(policy, list) or not policy:
        fail("FORMAT_POLICY_DEPENDENCIES_MISSING", "format/policy roots are missing")
    policy_ids = set()
    for edge in policy:
        if not isinstance(edge, dict):
            fail("FORMAT_POLICY_EDGE_INVALID", "format/policy edge is malformed")
        if edge.get("dependency_id") in policy_ids:
            fail("FORMAT_POLICY_EDGE_DUPLICATE", "duplicate format/policy dependency")
        policy_ids.add(edge.get("dependency_id"))
        citation_lookup(citation_index, edge.get("authority_id"), edge.get("citation_id"))
        if edge.get("scope_root") not in {"FORMAT", "LEGALITY", "RELEASE_NOTE"}:
            fail("FORMAT_POLICY_EDGE_INVALID", "format/policy edge has invalid scope root")
        if not isinstance(edge.get("rationale"), str) or not edge["rationale"].strip():
            fail("FORMAT_POLICY_EDGE_INVALID", "format/policy edge lacks rationale")
    return dep_by_family


EXPECTED_REVIEW_FIELDS = {
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
}
EXPECTED_SEMANTIC_ANCHORS = {
    "cap.token_or_counters": {"CR-111-tokens", "CR-122-counters", "CR-700-2-modes"},
    "cap.tribal_permission": {"CR-205-type-line", "CR-601-casting-spells"},
    "cap.modified_predicate": {"CR-700-9-modified"},
    "cap.continuous_ability": {"CR-604-static-abilities", "CR-611-continuous-effects", "CR-613-continuous-effects"},
    "cap.alternate_cast_zone": {"CR-601-casting-spells"},
    "cap.artifact_animation": {"CR-301-artifacts", "CR-613-continuous-effects"},
    "cap.flash": {"CR-702-8-flash"},
    "cap.flashback": {"CR-404-graveyard", "CR-601-casting-spells", "CR-702-34-flashback"},
    "cap.improvise": {"CR-702-126-improvise"},
    "cap.mass_untap": {"CR-701-26-tap-untap"},
    "cap.attack_mana": {"CR-508-declare-attackers", "CR-605-mana-abilities"},
    "cap.copy": {"CR-111-tokens", "CR-707-copying-objects"},
}


def validate_semantic_review(model: dict, dep_by_family: dict) -> None:
    review = model.get("semantic_dependency_review")
    if not isinstance(review, dict) or review.get("schema") != "manafold.m2.5.b1.semantic-dependency-review.v1":
        fail("SEMANTIC_REVIEW_MISSING", "B1.Final lacks the versioned semantic dependency review")
    if review.get("review_status") != "ALL_REQUIRED_FAMILIES_REVIEWED" or review.get("lexical_scans_are_authority") is not False:
        fail("SEMANTIC_REVIEW_STATUS", "semantic review is not closed without lexical authority")
    records = review.get("records")
    if not isinstance(records, list) or len(records) != len(dep_by_family):
        fail("SEMANTIC_REVIEW_FAMILY_COUNT", "semantic review does not contain exactly one record per required family")
    record_by_family = {}
    for record in records:
        if not isinstance(record, dict) or record.get("family_id") in record_by_family:
            fail("SEMANTIC_REVIEW_RECORD_INVALID", "semantic review records are malformed or duplicated")
        fid = record.get("family_id")
        if fid not in dep_by_family:
            fail("SEMANTIC_REVIEW_UNKNOWN_FAMILY", f"semantic review contains non-required family {fid}")
        record_by_family[fid] = record
        dep = dep_by_family[fid]
        if record.get("review_status") != "REVIEWED":
            fail("SEMANTIC_REVIEW_UNREVIEWED", f"family {fid} is not marked REVIEWED")
        if record.get("boundary_sha256") != dep["b2_boundary_binding"]["boundary_sha256"]:
            fail("SEMANTIC_REVIEW_BOUNDARY_MISMATCH", f"semantic review boundary binding differs for {fid}")
        if set(record.get("reviewed_boundary_fields", [])) != EXPECTED_REVIEW_FIELDS:
            fail("SEMANTIC_REVIEW_FIELDS_INCOMPLETE", f"semantic review fields are incomplete for {fid}")
        if record.get("required_citation_ids") != dep.get("required_citation_ids"):
            fail("SEMANTIC_REVIEW_DEPENDENCY_MISMATCH", f"semantic review citation set differs for {fid}")
        removed = set(record.get("removed_or_replaced_v2_citation_ids", []))
        if removed & set(dep.get("required_citation_ids", [])):
            fail("INHERITED_UNRELATED_EDGE_RETAINED", f"family {fid} retains a removed V2 citation")
        domains = record.get("material_rule_domains")
        domain_ids = {d.get("domain_id") for d in domains} if isinstance(domains, list) else set()
        required = set(dep.get("required_citation_ids", []))
        if domain_ids != required:
            fail("SEMANTIC_DOMAIN_COVERAGE", f"material semantic domains do not match required edges for {fid}")
        edge_domains = set()
        for edge in dep.get("authority_dependencies", []):
            covered = edge.get("covered_domain_ids")
            if not isinstance(covered, list) or not covered or not set(covered) <= required:
                fail("SEMANTIC_EDGE_COVERAGE", f"citation edge lacks valid boundary-domain coverage for {fid}")
            edge_domains.update(covered)
        if edge_domains != required:
            fail("SEMANTIC_DOMAIN_UNCOVERED", f"a material boundary domain is uncovered for {fid}")
    if set(record_by_family) != set(dep_by_family):
        fail("SEMANTIC_REVIEW_FAMILY_COUNT", "semantic review family set differs from dependency graph")
    anchors = review.get("regression_anchors")
    anchor_map = {x.get("family_id"): set(x.get("expected_required_citation_ids", [])) for x in anchors} if isinstance(anchors, list) else {}
    if set(anchor_map) != set(EXPECTED_SEMANTIC_ANCHORS):
        fail("SEMANTIC_ANCHOR_SET", "semantic regression anchor set is incomplete")
    for fid, expected in EXPECTED_SEMANTIC_ANCHORS.items():
        if anchor_map[fid] != expected or set(dep_by_family[fid]["required_citation_ids"]) != expected:
            fail("SEMANTIC_ANCHOR_REGRESSION", f"fixed semantic anchor failed for {fid}")


def validate_gate_state(closure: dict, citations: dict, model: dict) -> None:
    if closure.get("schema") != EXPECTED_CLOSURE_SCHEMA:
        fail("SCHEMA_MISMATCH", f"unexpected B1.Final closure schema: {closure.get('schema')!r}")
    if closure.get("AUTHORITY_INPUT_COUNT") != 7 or closure.get("AUTHORITY_TERMINAL_COUNT") != 7:
        fail("CLOSURE_COUNT_MISMATCH", "B1.Final authority closure counts are not 7/7")
    if closure.get("REQUIRED_B2_FAMILY_COUNT") != 210:
        fail("CLOSURE_COUNT_MISMATCH", "B1.Final required family count is not 210")
    if closure.get("unresolved_authority_dependencies") != []:
        fail("UNRESOLVED_DEPENDENCIES", "B1.Final still records unresolved authority dependencies")
    statuses = closure.get("gate_status")
    if statuses != REQUIRED_GATE_STATUSES:
        fail(
            "GATE_PROMOTION_FORBIDDEN",
            "B1.Final gate state differs from the exact terminal transition",
        )
    if closure.get("flags") != REQUIRED_FALSE_FLAGS:
        fail("DOWNSTREAM_PROMOTION_FORBIDDEN", "B1.Final promoted a downstream flag")
    if model.get("downstream_gate_status") != REQUIRED_GATE_STATUSES:
        fail("GATE_PROMOTION_FORBIDDEN", "B1.Final semantic model promoted a later gate")
    if closure.get("bound_evidence", {}).get(CITATIONS_NAME) != sha256_bytes(
        json.dumps(citations, indent=2, ensure_ascii=False).encode("utf-8") + b"\n"
    ):
        fail("EVIDENCE_DIGEST_MISMATCH", "closure does not bind the exact B1.Final citations bytes")


def validate_banned_proof(model: dict, citation_index: dict, reader) -> None:
    proof = model.get("banned_list_proof")
    if not isinstance(proof, dict):
        fail("BANNED_LIST_PROOF_MISSING", "banned-list proof is missing")
    citation_lookup(citation_index, proof.get("authority_id"), proof.get("citation_id"))
    if (
        proof.get("authority_id") != "banned_restricted"
        or proof.get("citation_id") != "BRL-COMMANDER"
    ):
        fail("BANNED_LIST_PROOF_INVALID", "banned-list proof is not tied to BRL-COMMANDER")
    names = proof.get("scoped_card_names")
    if not isinstance(names, list) or len(names) != 402 or len(set(names)) != 402:
        fail("BANNED_LIST_UNIVERSE", "banned-list proof does not contain 402 distinct scoped names")
    if proof.get("name_matches") != [] or proof.get("distinct_card_name_count") != 402:
        fail("BANNED_LIST_MATCH_FOUND", "banned-list proof reports a current scoped name match")
    if proof.get("scoped_card_names_sha256") != sha256_bytes(
        json.dumps(sorted(names), ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    ):
        fail("BANNED_LIST_DIGEST", "banned-list scoped-name digest is invalid")
    banned_record, banned_citation = citation_index["BRL-COMMANDER"]
    path = banned_record["artifact_identity"]["artifact_path"]
    artifact = reader.read(path)
    loc = banned_citation["artifact_local_locator"]
    fragment = artifact[loc["byte_offset"] : loc["byte_offset"] + loc["byte_length"]]
    for name in names:
        if name.casefold().encode("utf-8") in fragment.lower():
            fail("BANNED_LIST_MATCH_FOUND", f"scoped card name appears in banned section: {name}")


def run_check(closures_dir: Path = B1_DIR, b2_dir: Path = B2_DIR) -> int:
    archive, _ = resolve_archive()
    legacy = load_legacy_helpers()
    reader = legacy.ArchiveReader(archive, EXPECTED_ARCHIVE_SHA256)
    citations, citations_raw = load_json(closures_dir / CITATIONS_NAME)
    closure, _closure_raw = load_json(closures_dir / CLOSURE_NAME)
    report_path = closures_dir / REPORT_NAME
    if not report_path.is_file():
        blocked("REPORT_MISSING", f"missing {report_path}")
    report_raw = report_path.read_bytes()
    citation_index = validate_authorities(citations, reader, legacy)
    model = citations.get("semantic_dependency_model")
    if (
        not isinstance(model, dict)
        or model.get("schema") != "manafold.m2.5.b1.final-semantic-dependency-model.v1"
    ):
        fail("MODEL_SCHEMA_MISMATCH", "B1.Final semantic dependency model schema is invalid")
    b2_records, _ = b2_input_records(b2_dir)
    expected_binding = model.get("b2_inputs")
    if sorted(b2_records, key=lambda x: x["path"]) != sorted(
        expected_binding or [], key=lambda x: x.get("path", "")
    ):
        fail("B2_INPUT_DIGEST_MISMATCH", "B1.Final B2 binding does not match current B2 bytes")
    cat, classifications, b2_closure, family_by_id, assignment_counts = validate_b2_binding(
        model, b2_dir, citation_index, reader
    )
    if (
        model.get("b2_source_package_sha256") != EXPECTED_ARCHIVE_SHA256
        or classifications.get("source_package_sha256") != EXPECTED_ARCHIVE_SHA256
        or cat.get("source_package_sha256") != EXPECTED_ARCHIVE_SHA256
        or b2_closure.get("source_package_sha256") != EXPECTED_ARCHIVE_SHA256
    ):
        fail(
            "B2_SOURCE_PACKAGE_MISMATCH", "B1.Final is not bound to the pinned REV3 source package"
        )
    dep_by_family = validate_dependency_model(
        model, cat, family_by_id, assignment_counts, citation_index
    )
    validate_semantic_review(model, dep_by_family)
    validate_banned_proof(model, citation_index, reader)
    validate_gate_state(closure, citations, model)
    expected_bound = {
        CITATIONS_NAME: sha256_bytes(citations_raw),
        REPORT_NAME: sha256_bytes(report_raw),
    }
    if closure.get("bound_evidence") != expected_bound:
        fail(
            "EVIDENCE_DIGEST_MISMATCH", "B1.Final bound evidence digests do not match source bytes"
        )
    matrix_path = closures_dir / MATRIX_NAME
    matrix, _ = load_json(matrix_path)
    if (
        matrix.get("schema") != EXPECTED_NEGATIVE_SCHEMA
        or matrix.get("case_count") != 26
        or len(matrix.get("cases", [])) != 26
    ):
        fail("NEGATIVE_MATRIX_SCHEMA", "B1.Final negative matrix does not contain exactly 26 cases")
    print(
        "B1_FINAL_AUTHORITY_CITATION_CLOSURE_CHECK = PASS "
        f"(authorities=7 required_families={len(dep_by_family)} terminal_assignments=1883)"
    )
    return 0


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n"
    )


def negative_self_test() -> int:
    """Run the 26-case matrix against copied source artifacts."""
    failures: list[str] = []
    with tempfile.TemporaryDirectory() as temp:
        base = Path(temp)
        final_dir = base / "B1"
        final_dir.mkdir()
        for name in (CITATIONS_NAME, CLOSURE_NAME, REPORT_NAME, MATRIX_NAME):
            source = B1_DIR / name
            if not source.is_file():
                print(f"BLOCKED: missing B1.Final artifact {source}")
                return 2
            target = final_dir / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
        b2_copy = base / "B2"
        shutil.copytree(B2_DIR, b2_copy)
        _archive, _ = resolve_archive()
        os.environ[ARCHIVE_ENV_VAR] = str(Path(os.environ[ARCHIVE_ENV_VAR]))
        matrix, _ = load_json(final_dir / MATRIX_NAME)
        cases = {case["case_id"]: case for case in matrix["cases"]}

        def load_case() -> tuple[dict, dict]:
            citations, _ = load_json(final_dir / CITATIONS_NAME)
            closure, _ = load_json(final_dir / CLOSURE_NAME)
            return citations, closure

        def save_case(citations: dict, closure: dict, *, rebind: bool = True) -> None:
            if rebind:
                citations_raw = (
                    json.dumps(citations, indent=2, ensure_ascii=False).encode("utf-8") + b"\n"
                )
                closure["bound_evidence"][CITATIONS_NAME] = sha256_bytes(citations_raw)
            write_json(final_dir / CITATIONS_NAME, citations)
            write_json(final_dir / CLOSURE_NAME, closure)

        mutations = {
            "B2_CLOSURE_NOT_PASS": lambda c, cl: load_json(
                b2_copy / "classification_closure.v1.json"
            )[0].__setitem__("CLASSIFICATION_REFERENCE_CLOSURE", "BLOCKED"),
            "B2_CATALOG_DIGEST_CHANGED": lambda c, cl: c["semantic_dependency_model"]["b2_inputs"][
                3
            ].__setitem__("raw_sha256", "0" * 64),
            "B2_CLASSIFICATION_DIGEST_CHANGED": lambda c, cl: c["semantic_dependency_model"][
                "b2_inputs"
            ][1].__setitem__("raw_sha256", "0" * 64),
            "B2_FAMILY_BOUNDARY_CHANGED": lambda c, cl: c["semantic_dependency_model"][
                "family_dependencies"
            ][0]["b2_boundary_binding"].__setitem__("exact_boundary", "tampered"),
            "OMITTED_TERMINAL_FAMILY": lambda c, cl: c["semantic_dependency_model"][
                "family_dependencies"
            ].pop(),
            "UNKNOWN_FAMILY_INJECTED": lambda c, cl: c["semantic_dependency_model"][
                "family_dependencies"
            ].append({"family_id": "cap.unknown"}),
            "ACTIVE_UNASSIGNED_FALSE_REQUIRED": lambda c, cl: c["semantic_dependency_model"][
                "active_unassigned_handling"
            ][0].__setitem__("card_semantic_dependency_required", True),
            "ASSIGNED_FAMILY_MARKED_UNREQUIRED": lambda c, cl: c["semantic_dependency_model"][
                "family_dependencies"
            ].pop(),
            "MISSING_REQUIRED_CITATION_EDGE": lambda c, cl: next(
                dep
                for dep in c["semantic_dependency_model"]["family_dependencies"]
                if len(dep["authority_dependencies"]) == 1
            )["authority_dependencies"].pop(),
            "COMPOSITE_EDGE_REMOVED": lambda c, cl: c["semantic_dependency_model"][
                "family_dependencies"
            ][
                next(
                    i
                    for i, d in enumerate(c["semantic_dependency_model"]["family_dependencies"])
                    if d["family_id"] == "cap.target_destroy"
                )
            ]["authority_dependencies"].pop(),
            "UNKNOWN_CITATION": lambda c, cl: c["semantic_dependency_model"]["family_dependencies"][
                0
            ]["authority_dependencies"][0].__setitem__("citation_id", "CR-NOT-REAL"),
            "MAGIC_2013_EDGE": lambda c, cl: c["semantic_dependency_model"]["family_dependencies"][
                0
            ]["authority_dependencies"][0].update(
                {"authority_id": MAGIC_2013, "citation_id": "BRL-ROOT"}
            ),
            "CITATION_DIFFERENT_RULE": lambda c, cl: next(
                a for a in c["authorities"] if a["authority_id"] == "comprehensive_rules"
            )["citations"][0]["artifact_local_locator"].__setitem__("line_number_1based", 2),
            "CITATION_WRONG_LINE": lambda c, cl: next(
                a for a in c["authorities"] if a["authority_id"] == "comprehensive_rules"
            )["citations"][0]["artifact_local_locator"].__setitem__("line_number_1based", 3),
            "THIRD_PARTY_AUTHORITY": lambda c, cl: next(
                a for a in c["authorities"] if a["authority_id"] == "comprehensive_rules"
            ).__setitem__("authority_role", "THIRD_PARTY_REFERENCE"),
            "PROVENANCE_SWAP": lambda c, cl: c["authorities"][0].__setitem__(
                "original_official_url", c["authorities"][1]["original_official_url"]
            ),
            "AUTHORITY_ARTIFACT_DIGEST": lambda c, cl: c["authorities"][0][
                "artifact_identity"
            ].__setitem__("artifact_sha256", "0" * 64),
            "WRONG_CITATIONS_SCHEMA": lambda c, cl: c.__setitem__("schema", "wrong"),
            "WRONG_CLOSURE_SCHEMA": lambda c, cl: cl.__setitem__("schema", "wrong"),
            "CLOSURE_PROMOTES_WITH_UNRESOLVED": lambda c, cl: c["semantic_dependency_model"][
                "family_dependencies"
            ].pop(),
            "LATER_GATE_PROMOTION": lambda c, cl: cl["gate_status"].__setitem__(
                "DECLARED_INTERACTION_MODEL_CLOSURE", "PASS"
            ),
            "DECK_LOCK_PROMOTION": lambda c, cl: cl["flags"].__setitem__("DECK_PAIR_LOCKED", True),
            "RANKING_PROMOTION": lambda c, cl: cl["flags"].__setitem__(
                "AUTHORITATIVE_RANKING_AVAILABLE", True
            ),
            "M3_PROMOTION": lambda c, cl: cl["flags"].__setitem__("M3_STARTED", True),
            "EVIDENCE_DIGEST_TAMPER": lambda c, cl: c.__setitem__("tampered", True),
            "STALE_B2_IDENTITY": lambda c, cl: c["semantic_dependency_model"].__setitem__(
                "b2_source_package_sha256", "0" * 64
            ),
        }
        # The first control must pass before mutation results are meaningful.
        try:
            run_check(final_dir, b2_copy)
        except FinalError as exc:
            print(f"FAIL: positive control did not pass ({exc.code})")
            return 1
        for case_id, case in cases.items():
            citations, closure = load_case()
            if case_id == "B2_CLOSURE_NOT_PASS":
                b2closure, _ = load_json(b2_copy / "classification_closure.v1.json")
                b2closure["CLASSIFICATION_REFERENCE_CLOSURE"] = "BLOCKED"
                write_json(b2_copy / "classification_closure.v1.json", b2closure)
                changed_closure_sha = sha256_bytes(
                    (b2_copy / "classification_closure.v1.json").read_bytes()
                )
                for binding in citations["semantic_dependency_model"]["b2_inputs"]:
                    if binding["path"] == "classification_closure.v1.json":
                        binding["raw_sha256"] = changed_closure_sha
            else:
                mutations[case_id](citations, closure)
            save_case(citations, closure, rebind=case_id not in {"EVIDENCE_DIGEST_TAMPER"})
            try:
                run_check(final_dir, b2_copy)
            except FinalError as exc:
                if exc.code != case["expected_error_code"]:
                    failures.append(
                        f"{case_id}: expected {case['expected_error_code']}, got {exc.code}"
                    )
                else:
                    print(f"NEGATIVE {case_id}: rejected ({exc.status}/{exc.code})")
            else:
                failures.append(f"{case_id}: unexpectedly passed")
            # Restore exact copied inputs and source artifacts for the next case.
            shutil.rmtree(b2_copy)
            shutil.copytree(B2_DIR, b2_copy)
            for name in (CITATIONS_NAME, CLOSURE_NAME, REPORT_NAME, MATRIX_NAME):
                source = B1_DIR / name
                target = final_dir / name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, target)
        if failures:
            for failure in failures:
                print(f"FAIL: {failure}")
            return 1
        print("B1_FINAL_NEGATIVE_SELF_TEST = PASS (26/26 exact mutation codes)")
        return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--closures-dir", type=Path, default=B1_DIR)
    parser.add_argument("--b2-dir", type=Path, default=B2_DIR)
    parser.add_argument("--negative-self-test", action="store_true")
    args = parser.parse_args()
    try:
        if args.negative_self_test:
            return negative_self_test()
        return run_check(args.closures_dir.resolve(), args.b2_dir.resolve())
    except FinalError as exc:
        print(f"{exc.status}: {exc.message}")
        return 2 if exc.status == "BLOCKED" else 1


if __name__ == "__main__":
    raise SystemExit(main())
