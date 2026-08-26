#!/usr/bin/env python3
"""Deterministic B1 official-authority-citation-closure verifier.

Validates sources/m2_5/closures/B1 against the preflight-verified REV3 private
archive contract:

  - exact seven-authority input universe (no missing/duplicate/unknown);
  - every B1 record cross-bound EXACTLY to its REV3 register entry (URL,
    artifact path, artifact digest, retrieval time, availability, status);
  - every authority terminal (CITED or NOT_REQUIRED_WITH_PROOF);
  - CITED: official role/host, pinned artifact bytes resolved out of the
    verified ZIP, and every citation locator mechanically resolved inside
    those bytes; CR identifiers are derived into their own locators, so a
    valid identifier paired with a foreign heading fails;
  - NOT_REQUIRED_WITH_PROOF: primary evidence is the explicit
    authority->semantic dependency model - every catalog requirement family
    and every detected oracle-text mechanic must carry a covering citation
    edge into a CITED authority other than magic_2013_release_notes, whose
    edge count is therefore mechanically zero; additionally the supplementary
    string scan is re-executed over the CHECKER-pinned canonical surface
    catalog, so manifest omissions cannot stay self-consistent;
  - closure agreement (gate statuses, counts, remaining gates BLOCKED,
    DECK_PAIR_LOCKED/AUTHORITATIVE_RANKING_AVAILABLE/M3_STARTED false);
  - all B1 evidence digest-bound through the closure record.

--negative-self-test executes the adversarial fixture matrix against the real
checker logic using a synthetic archive/payload/closures fixture and REQUIRES
an unmutated positive control to pass first.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import os
import re
import sys
import tempfile
import zipfile
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CLOSURES_DIR = ROOT / "sources" / "m2_5" / "closures" / "B1"
PROVENANCE_PATH = ROOT / "sources" / "m2_5" / "pre_research" / "REV3" / "IMPORT_PROVENANCE.json"
ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"
CITATIONS_NAME = "official_authority_citations.v1.json"
CLOSURE_NAME = "official_authority_citation_closure.v1.json"
REPORT_NAME = "AUTHORITY_CITATION_REPORT.md"

EXPECTED_UNIVERSE = [
    "banned_restricted",
    "commander_1v1",
    "commander_general",
    "commander_legends_release_notes",
    "comprehensive_rules",
    "kaldheim_release_notes",
    "magic_2013_release_notes",
]
NOT_REQUIRED_AUTHORITY = "magic_2013_release_notes"

# Canonical dependency-surface catalog, pinned HERE (checker-side), so a proof
# record cannot omit files from its own scan manifest.
SURFACE_FILES = (
    "derived/Card_Requirement_Map_REV3.csv",
    "derived/Interaction_Model_Coverage_REV3.json",
    "derived/Pair_Interaction_Census_REV3.csv",
    "derived/Pair_Requirement_Aggregates_REV3.json",
    "derived/Ranking_Factors_REV3.json",
    "derived/Ranking_Sensitivity_REV3.json",
    "derived/Ranking_Uncertainty_Scenarios_REV3.json",
    "inputs/card_semantic_classification_REV3.json",
    "inputs/deck_row_source_resolution_REV3.csv",
    "inputs/oracle_semantic_evidence_REV3.json",
    "inputs/ranking_formula_REV3.json",
    "inputs/requirement_family_catalog_REV3.json",
)
SCAN_PATTERNS = ("magic_2013", "magic-2013", "magic 2013", "update-bulletin")
DECK_ROW_SURFACE = "inputs/deck_row_source_resolution_REV3.csv"
FAMILY_CATALOG_SURFACE = "inputs/requirement_family_catalog_REV3.json"
MECHANIC_CONCEPTS = {
    "foretell": "foretell",
    "partner": "partner",
    "crew": "crew",
    "equip": "equip",
    "living weapon": "living weapon",
    "reconfigure": "reconfigure",
    "convoke": "convoke",
    "amass": "amass",
    "populate": "populate",
    "saga": "saga",
}

ALLOWED_ROLES_PREFIX = "OFFICIAL_"
ALLOWED_URL_HOSTS = {"magic.wizards.com", "media.wizards.com"}
CR_RULE_RE = re.compile(r"^CR (\d{3})(\.\d+[a-z]?)?$")
REQUIRED_GATES_BLOCKED = [
    "CLASSIFICATION_REFERENCE_CLOSURE",
    "DECLARED_INTERACTION_MODEL_CLOSURE",
    "REV2_REUSE_RATIO_REPRODUCIBLE",
    "RANKING_UNCERTAINTY_PROPAGATION",
]

EXIT_PASS = 0
EXIT_FAIL = 1
EXIT_BLOCKED = 2


class B1Error(Exception):
    def __init__(self, status: str, message: str) -> None:
        super().__init__(message)
        self.status = status
        self.message = message


def fail(message: str) -> None:
    raise B1Error("FAIL", message)


def blocked(message: str) -> None:
    raise B1Error("BLOCKED", message)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_json_bytes(data: bytes, label: str) -> dict:
    try:
        value = json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        blocked(f"unreadable {label}: {exc}")
    if not isinstance(value, dict):
        blocked(f"{label} is not a JSON object")
    return value


def resolve_archive_path(provenance_path: Path = PROVENANCE_PATH) -> tuple[Path, str]:
    if not provenance_path.is_file():
        blocked(f"M2.5.A import provenance not found: {provenance_path}")
    prov = read_json_bytes(provenance_path.read_bytes(), "IMPORT_PROVENANCE.json")
    package = prov.get("source_package")
    if not isinstance(package, dict):
        blocked("IMPORT_PROVENANCE.source_package missing")
    if package.get("storage_class") != "MAINTAINER_PRIVATE_ARCHIVE":
        blocked("unexpected archive storage class")
    locator = package.get("logical_locator")
    base = os.environ.get(ARCHIVE_ENV_VAR)
    if not isinstance(locator, str) or ARCHIVE_ENV_VAR not in locator or not base:
        blocked(
            f"{ARCHIVE_ENV_VAR} is unset or locator malformed; the REV3 archive "
            "cannot be located (fail-closed)"
        )
    relative = (
        locator.replace("${" + ARCHIVE_ENV_VAR + "}", "")
        .replace("$" + ARCHIVE_ENV_VAR, "")
        .lstrip("/")
    )
    return Path(base) / relative, str(package.get("sha256"))


class ArchiveReader:
    """Reads members out of the SHA-256-verified REV3 private archive ZIP."""

    def __init__(self, archive_path: Path, expected_zip_sha256: str) -> None:
        if not archive_path.is_file():
            blocked(f"REV3 archive not found: {archive_path}")
        actual = sha256_bytes(archive_path.read_bytes())
        if actual != expected_zip_sha256:
            fail(f"REV3 archive digest mismatch ({actual} != {expected_zip_sha256})")
        self._zip_path = archive_path

    def read(self, member: str) -> bytes:
        with zipfile.ZipFile(self._zip_path) as zf:
            try:
                return zf.read(member)
            except KeyError:
                blocked(f"archive member missing: {member}")


def parse_register(raw: bytes) -> dict[str, dict]:
    entries = json.loads(raw.decode("utf-8"))
    result: dict[str, dict] = {}
    for entry in entries:
        result[entry["authority_id"]] = entry
    return result


def resolve_rule_line(artifact: bytes, identifier: str, locator: dict) -> None:
    """Resolve the exact line implied by the CR identifier itself."""
    match = CR_RULE_RE.fullmatch(identifier)
    if not match:
        fail(f"non-canonical CR rule identifier: {identifier!r}")
    number = match.group(1) + (match.group(2) or "")
    kind = locator.get("locator_kind")
    if kind != "RULE_HEADING_LINE":
        fail(f"CR citation locator kind must be RULE_HEADING_LINE, found {kind!r}")
    if "expected_heading_prefix" in locator:
        fail(
            "locator must not carry an independently supplied heading prefix; "
            "it is derived from rule_identifier"
        )
    line_number = locator.get("line_number_1based")
    digest = locator.get("heading_line_sha256")
    if not isinstance(line_number, int) or line_number < 1:
        fail(f"invalid rule heading line number: {line_number!r}")
    lines = artifact.decode("utf-8-sig", errors="strict").split("\n")
    if line_number > len(lines):
        fail(f"rule line {line_number} beyond end of artifact ({len(lines)} lines)")
    stripped = lines[line_number - 1].strip()
    if number[-1:].isalpha():
        # subrule citation: the line must BE that subrule, e.g. '702.143a Foretell ...'
        token = stripped.split(" ")[0]
        if token != number:
            fail(
                f"identifier/locator binding failure: {identifier} resolves to "
                f"{token!r} at line {line_number}"
            )
    else:
        # section citation: heading must start '<num>.' exactly
        if not stripped.startswith(number + "."):
            fail(
                f"identifier/locator binding failure: {identifier} resolves to "
                f"{stripped[:20]!r} at line {line_number}"
            )
    if sha256_bytes(stripped.encode()) != digest:
        fail(f"line {line_number} heading digest mismatch for {identifier}")


def resolve_byte_fragment(artifact: bytes, locator: dict) -> None:
    if locator.get("locator_kind") != "UNIQUE_BYTE_FRAGMENT":
        fail(f"unknown locator kind: {locator.get('locator_kind')!r}")
    offset = locator.get("byte_offset")
    length = locator.get("byte_length")
    digest = locator.get("fragment_sha256")
    if not isinstance(offset, int) or not isinstance(length, int) or offset < 0 or length <= 0:
        fail(f"invalid byte fragment geometry: offset={offset!r} len={length!r}")
    if offset + length > len(artifact):
        fail(f"byte fragment [{offset}:{offset + length}] beyond artifact size {len(artifact)}")
    window = artifact[offset : offset + length]
    if sha256_bytes(window) != digest:
        fail(f"byte fragment digest mismatch at offset {offset}")
    if artifact.count(window) != 1:
        fail(f"byte fragment at offset {offset} occurs {artifact.count(window)} times")


def detect_mechanics(reader) -> set[str]:
    data = reader.read(DECK_ROW_SURFACE)
    decoded = data.decode("utf-8", errors="strict")
    rows = list(csv.DictReader(io.StringIO(decoded)))
    corpus = "\n".join(
        ((r.get("oracle_top_level_text") or "") + "\n" + (r.get("oracle_faces") or "")).lower()
        for r in rows
    )
    return {c for c, needle in MECHANIC_CONCEPTS.items() if needle in corpus}


def validate_citation(citation: dict, artifacts: dict[str, bytes], artifact_path: str) -> None:
    for key in ("citation_id", "citation_kind", "why_required"):
        value = citation.get(key)
        if not isinstance(value, str) or not value.strip():
            fail(f"citation {citation.get('citation_id')!r} has empty {key}")
    kind = citation["citation_kind"]
    identifier = citation.get("rule_identifier")
    if kind == "CR_RULE_IDENTIFIER":
        if not isinstance(identifier, str) or not CR_RULE_RE.fullmatch(identifier):
            fail(f"non-canonical CR rule identifier: {identifier!r}")
        resolve_rule_line(artifacts[artifact_path], identifier, citation["artifact_local_locator"])
    elif kind in {"POLICY_SECTION_LOCATOR", "RELEASE_NOTE_LOCATOR"}:
        if identifier is not None:
            fail(f"{kind} must not carry a fabricated rule identifier")
        resolve_byte_fragment(artifacts[artifact_path], citation["artifact_local_locator"])
    else:
        fail(f"unknown citation kind: {kind!r}")


def cross_bind_register(record: dict, register_entry: dict) -> None:
    identity = record["artifact_identity"]
    pairs = [
        ("source_url", record.get("original_official_url"), register_entry.get("source_url")),
        ("artifact_path", identity.get("artifact_path"), register_entry.get("artifact_path")),
        (
            "artifact_sha256",
            identity.get("artifact_sha256"),
            register_entry.get("artifact_sha256"),
        ),
        ("retrieval_time", identity.get("retrieval_time"), register_entry.get("retrieved_at")),
        (
            "raw_artifact_available",
            identity.get("raw_artifact_available"),
            register_entry.get("raw_artifact_available"),
        ),
        (
            "acquisition_http_status",
            identity.get("acquisition_http_status"),
            register_entry.get("http_status"),
        ),
        ("acquisition_error", identity.get("acquisition_error"), register_entry.get("error")),
    ]
    for field, recorded, pinned in pairs:
        if recorded != pinned:
            fail(
                f"authority {record['authority_id']} field {field!r} contradicts the "
                f"pinned REV3 register ({recorded!r} != {pinned!r})"
            )


def validate_authority_record(
    record: dict,
    seen_ids: set[str],
    universe: list[str],
    reader,
    artifacts_cache: dict[str, bytes],
    register: dict[str, dict],
) -> None:
    authority_id = record.get("authority_id")
    if authority_id not in universe:
        fail(f"unknown authority id: {authority_id!r}")
    if authority_id in seen_ids:
        fail(f"duplicate authority id: {authority_id!r}")
    seen_ids.add(authority_id)
    role = record.get("authority_role")
    if not isinstance(role, str) or not role.startswith(ALLOWED_ROLES_PREFIX):
        fail(f"authority {authority_id} role is not official: {role!r}")
    url = record.get("original_official_url")
    host_ok = isinstance(url, str) and url.split("/")[2] in ALLOWED_URL_HOSTS
    if not host_ok:
        fail(f"authority {authority_id} URL is not an official Wizards origin: {url!r}")

    status = record.get("citation_status")
    identity = record.get("artifact_identity")
    if not isinstance(identity, dict):
        fail(f"authority {authority_id} lacks artifact_identity")

    if status == "CITED":
        path = identity.get("artifact_path")
        digest = identity.get("artifact_sha256")
        if not isinstance(path, str) or not isinstance(digest, str):
            fail(f"CITED authority {authority_id} lacks pinned artifact identity")
        if path not in artifacts_cache:
            artifacts_cache[path] = reader.read(path)
        if sha256_bytes(artifacts_cache[path]) != digest:
            fail(f"artifact digest mismatch for {authority_id}: {path}")
        retrieval = identity.get("retrieval_time")
        if not isinstance(retrieval, str) or not retrieval.strip():
            fail(f"CITED authority {authority_id} lacks retrieval_time")
        citations = record.get("citations")
        if not isinstance(citations, list) or not citations:
            fail(f"CITED authority {authority_id} has empty citations[]")
        for citation in citations:
            validate_citation(citation, artifacts_cache, path)
    elif status == "NOT_REQUIRED_WITH_PROOF":
        proof = record.get("not_required_proof")
        if not isinstance(proof, dict):
            fail(f"NOT_REQUIRED authority {authority_id} lacks proof object")
    else:
        fail(
            f"authority {authority_id} has non-terminal status {status!r}; "
            "only CITED / NOT_REQUIRED_WITH_PROOF close"
        )
    cross_bind_register(record, register[authority_id])


def reexecute_dependency_scan(record: dict, reader) -> None:
    proof = record["not_required_proof"]
    scan = proof.get("supplementary_string_scan", proof.get("dependency_scan"))
    if not isinstance(scan, dict):
        fail("supplementary string scan missing")
    patterns = [p.lower() for p in scan.get("patterns", [])]
    if sorted(patterns) != sorted(SCAN_PATTERNS):
        fail("scan patterns differ from the canonical pattern set")
    manifest = scan.get("scan_manifest")
    if not isinstance(manifest, list):
        fail("dependency scan manifest malformed")

    manifest_paths = [entry.get("path") for entry in manifest]
    if sorted(manifest_paths) != sorted(SURFACE_FILES):
        missing = sorted(set(SURFACE_FILES) - set(manifest_paths))
        extra = sorted(set(manifest_paths) - set(SURFACE_FILES))
        fail(f"surface catalog deviation; missing={missing} extra={extra}")

    total = 0
    for entry in manifest:
        rel = entry.get("path")
        data = reader.read(rel)
        if sha256_bytes(data) != entry.get("sha256"):
            fail(f"payload surface changed since scan: {rel}")
        low = data.lower()
        hits = sum(low.count(p.encode()) for p in SCAN_PATTERNS)
        if hits != entry.get("hits_total"):
            fail(f"re-scanned hit count differs for {rel}: {hits}")
        total += hits
    if len(manifest) != scan.get("files_scanned"):
        fail("dependency scan summary disagrees with executed re-scan")
    if total != 0 or scan.get("total_relevant_dependency_hits") != 0:
        fail(
            f"FALSE_NOT_REQUIRED: {total} relevant dependency hits exist while the "
            "authority is marked NOT_REQUIRED_WITH_PROOF"
        )


def validate_dependency_model(citations: dict, authorities_by_id: dict, reader) -> None:
    model = citations.get("semantic_dependency_model")
    if not isinstance(model, dict):
        fail("semantic_dependency_model missing; string absence alone proves nothing")
    catalog_raw = reader.read(FAMILY_CATALOG_SURFACE)
    catalog_ids = {f["id"] for f in json.loads(catalog_raw.decode("utf-8"))}
    coverage = model.get("family_coverage")
    if not isinstance(coverage, dict):
        fail("family_coverage missing")
    if set(coverage.keys()) != catalog_ids:
        missing = sorted(catalog_ids - set(coverage.keys()))[:10]
        extra = sorted(set(coverage.keys()) - catalog_ids)[:10]
        fail(f"coverage is not the complete family partition; missing={missing} extra={extra}")

    cited_citation_ids = {
        c["citation_id"]: aid
        for aid, a in authorities_by_id.items()
        if a.get("citation_status") == "CITED"
        for c in a.get("citations", [])
    }
    for fid, target in coverage.items():
        owner = cited_citation_ids.get(target)
        if owner is None:
            fail(f"family {fid} covers into non-existent citation {target!r}")
        if owner == NOT_REQUIRED_AUTHORITY:
            fail(f"family {fid} depends on the NOT_REQUIRED authority")
    detected = detect_mechanics(reader)
    mechanic_edges = model.get("mechanic_edges")
    if not isinstance(mechanic_edges, dict):
        fail("mechanic_edges missing")
    for concept in sorted(detected):
        edges = mechanic_edges.get(concept)
        if not edges:
            fail(f"detected oracle-text mechanic {concept!r} has no covering edge")
        for edge in edges:
            owner = cited_citation_ids.get(edge)
            if owner is None or owner == NOT_REQUIRED_AUTHORITY:
                fail(f"mechanic {concept!r} edge {edge!r} is not a CITED-authority citation")
    declared_zero = model.get("magic_2013_dependency_edge_count")
    all_targets = list(coverage.values()) + [e for v in mechanic_edges.values() for e in v]
    structural = sum(1 for t in all_targets if t == NOT_REQUIRED_AUTHORITY)
    if declared_zero != 0 or structural != 0:
        fail(
            f"magic_2013 still carries dependency edges (declared={declared_zero}, "
            f"structural={structural})"
        )


def validate_closure_agreement(closure: dict, authorities: list[dict]) -> None:
    gates = closure.get("gate_statuses")
    if not isinstance(gates, dict):
        fail("closure.gate_statuses missing")
    if gates.get("OFFICIAL_RULE_CITATION_CLOSURE") != "PASS":
        fail("closure does not grant OFFICIAL_RULE_CITATION_CLOSURE = PASS")
    for gate in REQUIRED_GATES_BLOCKED:
        if gates.get(gate) != "BLOCKED":
            fail(f"B1 may not promote {gate}: found {gates.get(gate)!r}")
    terminals = {
        a["authority_id"]
        for a in authorities
        if a.get("citation_status") in ("CITED", "NOT_REQUIRED_WITH_PROOF")
    }
    if closure.get("AUTHORITY_TERMINAL_COUNT") != len(terminals):
        fail("AUTHORITY_TERMINAL_COUNT disagrees with records")
    if len(terminals) != len(authorities):
        fail("cannot grant PASS while non-terminal authority records exist")
    if closure.get("unresolved_authority_ids") != []:
        fail("closure still lists unresolved authorities")
    if closure.get("third_party_authority_count") != 0:
        fail("third-party authority promotion recorded")
    for key in ("DECK_PAIR_LOCKED", "AUTHORITATIVE_RANKING_AVAILABLE", "M3_STARTED"):
        if closure.get(key) is not False:
            fail(f"{key} must remain false after B1")
    if closure.get("M2_5_A_COMPLETE") is not True:
        fail("M2.5.A completion flag missing")


def run_check(closures_dir: Path, reader) -> int:
    citations_path = closures_dir / CITATIONS_NAME
    closure_path = closures_dir / CLOSURE_NAME
    report_path = closures_dir / REPORT_NAME
    if not citations_path.is_file() or not closure_path.is_file():
        blocked(f"B1 evidence missing under {closures_dir}")
    citations_raw = citations_path.read_bytes()
    citations = read_json_bytes(citations_raw, CITATIONS_NAME)
    closure = read_json_bytes(closure_path.read_bytes(), CLOSURE_NAME)

    universe_obj = citations.get("input_universe")
    if not isinstance(universe_obj, dict):
        fail("citations.input_universe missing")
    if universe_obj.get("authority_ids_in_order") != EXPECTED_UNIVERSE:
        fail("citations input universe is not the exact REV3 seven-authority set")
    register_digest = universe_obj.get("source_register_sha256")
    register_raw = reader.read("source/official_authority_register_REV3.json")
    if sha256_bytes(register_raw) != register_digest:
        fail("REV3 authority register digest mismatch; input universe is not pinned")
    register = parse_register(register_raw)

    authorities = citations.get("authorities")
    if not isinstance(authorities, list):
        fail("citations.authorities missing")
    if closure.get("AUTHORITY_INPUT_COUNT") != len(EXPECTED_UNIVERSE):
        fail("AUTHORITY_INPUT_COUNT must be 7")
    seen: set[str] = set()
    artifacts_cache: dict[str, bytes] = {}
    for record in authorities:
        validate_authority_record(
            record, seen, EXPECTED_UNIVERSE, reader, artifacts_cache, register
        )
    if seen != set(EXPECTED_UNIVERSE):
        missing = sorted(set(EXPECTED_UNIVERSE) - seen)
        fail(f"incomplete authority universe; missing: {missing}")
    authorities_by_id = {a["authority_id"]: a for a in authorities}
    for record in authorities:
        if record["citation_status"] == "NOT_REQUIRED_WITH_PROOF":
            reexecute_dependency_scan(record, reader)
    validate_dependency_model(citations, authorities_by_id, reader)

    binding = closure.get("bound_evidence")
    if not isinstance(binding, dict):
        fail("closure.bound_evidence missing; B1 evidence must be digest-bound")
    expected_bound = {CITATIONS_NAME: citations_raw, REPORT_NAME: report_path.read_bytes()}
    for name, raw in expected_bound.items():
        if sha256_bytes(raw) != binding.get(name):
            fail(f"bound evidence digest mismatch: {name}")

    validate_closure_agreement(closure, authorities)
    print(
        f"B1_AUTHORITY_CITATION_CLOSURE_CHECK = PASS "
        f"(authorities=7 terminal={len(seen)} unresolved=0)"
    )
    return EXIT_PASS


# --------------------------------------------------------------------- self-test

SYN_CR_LINES = [
    "Magic: The Gathering Comprehensive Rules",
    "These rules are effective as of August 7, 2026.",
    "",
    "111. Tokens",
    "111.1. A token is a marker used to represent any permanent that isn't a card.",
]


def build_synthetic_fixture(base: Path) -> tuple[Path, ArchiveReader]:
    """Create a consistent synthetic archive/payload/closures fixture."""
    archive_root = base / "archive" / "m2_5"
    archive_root.mkdir(parents=True)
    cr_text = "\n".join(SYN_CR_LINES) + "\n"
    html_page = b"<html><h2>Play Rules</h2>policy body</html>"
    payload_files = {
        DECK_ROW_SURFACE: b"card,oracle_top_level_text,oracle_faces\nSafe Card,foretell text,\n",
        "derived/Pair_Interaction_Census_REV3.csv": b"pair\n1\n",
        "inputs/card_semantic_classification_REV3.json": b'[{"id": "a"}]',
        FAMILY_CATALOG_SURFACE: json.dumps([{"id": f"cap.f{i}"} for i in range(4)]).encode(),
        "derived/census.csv": b"pair\n1\n",
        "inputs/ranking_formula_REV3.json": b"{}",
        "derived/Card_Requirement_Map_REV3.csv": b"deck_row_id,requirement_id\nr1,cap.f0\n",
        "derived/Pair_Requirement_Aggregates_REV3.json": b"{}",
        "derived/Interaction_Model_Coverage_REV3.json": b"{}",
        "derived/Ranking_Factors_REV3.json": b"{}",
        "derived/Ranking_Sensitivity_REV3.json": b"{}",
        "derived/Ranking_Uncertainty_Scenarios_REV3.json": b"{}",
        "inputs/oracle_semantic_evidence_REV3.json": b"[]",
    }
    members: dict[str, bytes] = {
        "source/official_authority_register_REV3.json": json.dumps(
            [
                {
                    "authority_id": "comprehensive_rules",
                    "source_url": "https://magic.wizards.com/rules",
                    "artifact_path": "source/authorities/comprehensive_rules.txt",
                    "artifact_sha256": sha256_bytes(cr_text.encode()),
                    "retrieved_at": "t",
                    "raw_artifact_available": True,
                    "http_status": 200,
                    "error": None,
                },
                {
                    "authority_id": "commander_general",
                    "source_url": "https://magic.wizards.com/formats/commander",
                    "artifact_path": "source/authorities/policy.html",
                    "artifact_sha256": sha256_bytes(html_page),
                    "retrieved_at": "t",
                    "raw_artifact_available": True,
                    "http_status": 200,
                    "error": None,
                },
                {
                    "authority_id": "banned_restricted",
                    "source_url": "https://magic.wizards.com/news/banned_restricted",
                    "artifact_path": None,
                    "artifact_sha256": None,
                    "retrieved_at": "t",
                    "raw_artifact_available": False,
                    "http_status": None,
                    "error": "HTTP Error 404: Not Found",
                },
                {
                    "authority_id": "commander_1v1",
                    "source_url": "https://magic.wizards.com/news/commander_1v1",
                    "artifact_path": None,
                    "artifact_sha256": None,
                    "retrieved_at": "t",
                    "raw_artifact_available": False,
                    "http_status": None,
                    "error": "HTTP Error 404: Not Found",
                },
                {
                    "authority_id": "commander_legends_release_notes",
                    "source_url": "https://magic.wizards.com/news/commander_legends_release_notes",
                    "artifact_path": None,
                    "artifact_sha256": None,
                    "retrieved_at": "t",
                    "raw_artifact_available": False,
                    "http_status": None,
                    "error": "HTTP Error 404: Not Found",
                },
                {
                    "authority_id": "kaldheim_release_notes",
                    "source_url": "https://magic.wizards.com/news/kaldheim_release_notes",
                    "artifact_path": None,
                    "artifact_sha256": None,
                    "retrieved_at": "t",
                    "raw_artifact_available": False,
                    "http_status": None,
                    "error": "HTTP Error 404: Not Found",
                },
                {
                    "authority_id": "magic_2013_release_notes",
                    "source_url": "https://magic.wizards.com/news/magic_2013_release_notes",
                    "artifact_path": None,
                    "artifact_sha256": None,
                    "retrieved_at": "t",
                    "raw_artifact_available": False,
                    "http_status": None,
                    "error": "HTTP Error 404: Not Found",
                },
            ]
        ).encode(),
        "source/authorities/comprehensive_rules.txt": cr_text.encode(),
        "source/authorities/policy.html": html_page,
        **payload_files,
    }
    zip_buffer = io.BytesIO()
    with zipfile.ZipFile(zip_buffer, "w") as zf:
        for name in sorted(members):
            zf.writestr(name, members[name])
    zip_path = archive_root / "synthetic.zip"
    zip_path.write_bytes(zip_buffer.getvalue())
    reader = ArchiveReader(zip_path, sha256_bytes(zip_path.read_bytes()))

    frag_offset = html_page.find(b"<h2>")
    frag_len = len(b"<h2>Play Rules</h2>")

    def syn_scan() -> dict:
        entries = []
        for rel in SURFACE_FILES:
            blob = payload_files[rel]
            low = blob.lower()
            entries.append(
                {
                    "path": rel,
                    "sha256": sha256_bytes(blob),
                    "hits_total": sum(low.count(p.encode()) for p in SCAN_PATTERNS),
                }
            )
        total = sum(e["hits_total"] for e in entries)
        return {
            "patterns": list(SCAN_PATTERNS),
            "files_scanned": len(entries),
            "total_relevant_dependency_hits": total,
            "scan_manifest": entries,
        }

    def covered_families(count: int) -> dict[str, str]:
        return {f"cap.f{i}": "SYN-CR-111" for i in range(count)}

    citations = {
        "schema": "manafold.m2.5.b1.official-authority-citations.v2",
        "input_universe": {
            "authority_ids_in_order": EXPECTED_UNIVERSE,
            "source_register_sha256": sha256_bytes(
                members["source/official_authority_register_REV3.json"]
            ),
        },
        "semantic_dependency_model": {
            "family_coverage": covered_families(4),
            "mechanic_edges": {"foretell": ["SYN-CR-111"]},
            "magic_2013_dependency_edge_count": 0,
        },
        "authorities": [
            {
                "authority_id": "comprehensive_rules",
                "authority_role": "OFFICIAL_RULES_TEXT",
                "original_official_url": "https://magic.wizards.com/rules",
                "artifact_identity": {
                    "artifact_path": "source/authorities/comprehensive_rules.txt",
                    "artifact_sha256": sha256_bytes(cr_text.encode()),
                    "retrieval_time": "t",
                    "raw_artifact_available": True,
                    "acquisition_http_status": 200,
                    "acquisition_error": None,
                },
                "citation_status": "CITED",
                "citations": [
                    {
                        "citation_id": "SYN-CR-111",
                        "citation_kind": "CR_RULE_IDENTIFIER",
                        "rule_identifier": "CR 111",
                        "artifact_local_locator": {
                            "locator_kind": "RULE_HEADING_LINE",
                            "line_number_1based": 4,
                            "heading_line_sha256": sha256_bytes(SYN_CR_LINES[3].strip().encode()),
                        },
                        "why_required": "tokens",
                        "dependent_requirement_families": [],
                    }
                ],
                "dependent_research_requirements": [],
            },
            {
                "authority_id": "commander_general",
                "authority_role": "OFFICIAL_FORMAT_POLICY",
                "original_official_url": "https://magic.wizards.com/formats/commander",
                "artifact_identity": {
                    "artifact_path": "source/authorities/policy.html",
                    "artifact_sha256": sha256_bytes(html_page),
                    "retrieval_time": "t",
                    "raw_artifact_available": True,
                    "acquisition_http_status": 200,
                    "acquisition_error": None,
                },
                "citation_status": "CITED",
                "citations": [
                    {
                        "citation_id": "SYN-POLICY-1",
                        "citation_kind": "POLICY_SECTION_LOCATOR",
                        "rule_identifier": None,
                        "artifact_local_locator": {
                            "locator_kind": "UNIQUE_BYTE_FRAGMENT",
                            "byte_offset": frag_offset,
                            "byte_length": frag_len,
                            "fragment_sha256": sha256_bytes(
                                html_page[frag_offset : frag_offset + frag_len]
                            ),
                        },
                        "why_required": "format policy",
                        "dependent_requirement_families": [],
                    }
                ],
                "dependent_research_requirements": [],
            },
        ]
        + [
            {
                "authority_id": aid,
                "authority_role": "OFFICIAL_UPDATE_NOTES",
                "original_official_url": f"https://magic.wizards.com/news/{aid}",
                "artifact_identity": {
                    "artifact_path": None,
                    "artifact_sha256": None,
                    "retrieval_time": "t",
                    "raw_artifact_available": False,
                    "acquisition_http_status": None,
                    "acquisition_error": "HTTP Error 404: Not Found",
                },
                "citation_status": "NOT_REQUIRED_WITH_PROOF",
                "not_required_proof": {
                    "resolution": "B_NOT_REQUIRED_FOR_REV3_CLOSURE",
                    "supplementary_string_scan": syn_scan(),
                },
                "citations": [],
                "dependent_research_requirements": [],
            }
            for aid in EXPECTED_UNIVERSE
            if aid not in {"comprehensive_rules", "commander_general"}
        ],
    }
    closures = base / "closures"
    closures.mkdir()
    cit_raw = json.dumps(citations, indent=2).encode() + b"\n"
    (closures / CITATIONS_NAME).write_bytes(cit_raw)
    (closures / REPORT_NAME).write_bytes(b"# B1 report\n")
    closure = {
        "schema": "manafold.m2.5.b1.official-authority-citation-closure.v1",
        "gate_statuses": {
            "OFFICIAL_RULE_CITATION_CLOSURE": "PASS",
            **{g: "BLOCKED" for g in REQUIRED_GATES_BLOCKED},
        },
        "AUTHORITY_INPUT_COUNT": 7,
        "AUTHORITY_TERMINAL_COUNT": 7,
        "unresolved_authority_ids": [],
        "third_party_authority_count": 0,
        "DECK_PAIR_LOCKED": False,
        "AUTHORITATIVE_RANKING_AVAILABLE": False,
        "M3_STARTED": False,
        "M2_5_A_COMPLETE": True,
        "bound_evidence": {
            CITATIONS_NAME: sha256_bytes(cit_raw),
            REPORT_NAME: sha256_bytes(b"# B1 report\n"),
        },
    }
    (closures / CLOSURE_NAME).write_bytes(json.dumps(closure, indent=2).encode() + b"\n")
    return closures, reader


class MergedReader:
    """Tampered payload surfaces win over the pristine archive members."""

    def __init__(self, primary: ArchiveReader, fallback: Path) -> None:
        self._primary = primary
        self._fallback = fallback

    def read(self, member: str) -> bytes:
        preferred = self._fallback / member
        if preferred.is_file():
            return preferred.read_bytes()
        return self._primary.read(member)


def negative_self_test() -> int:
    failures: list[str] = []
    cases_run = 0
    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)
        closures, reader = build_synthetic_fixture(base)

        cases_run += 1
        try:
            run_check_quiet(closures, reader)
            print("POSITIVE synthetic baseline fixture: PASS")
        except B1Error as exc:
            failures.append(
                f"BASELINE_FIXTURE_NOT_PASSING ({exc.status}: {exc.message}); "
                "rejections would be vacuous"
            )

        def fresh() -> tuple[dict, dict]:
            cits = read_json_bytes((closures / CITATIONS_NAME).read_bytes(), "")
            clos = read_json_bytes((closures / CLOSURE_NAME).read_bytes(), "")
            return cits, clos

        def write_case(name: str, cits: dict, clos: dict) -> Path:
            case = base / "cases" / name
            case.mkdir(parents=True)
            (case / CITATIONS_NAME).write_bytes(json.dumps(cits, indent=2).encode() + b"\n")
            (case / CLOSURE_NAME).write_bytes(json.dumps(clos, indent=2).encode() + b"\n")
            (case / REPORT_NAME).write_bytes(b"# B1 report\n")
            return case

        def expect_reject(case_id: str, mutate) -> None:
            nonlocal cases_run
            cases_run += 1
            cits, clos = fresh()
            mutate(cits, clos)
            case = write_case(case_id.lower(), cits, clos)
            try:
                run_check_quiet(case, reader)
            except B1Error as exc:
                if exc.status not in {"FAIL", "BLOCKED"}:
                    failures.append(f"{case_id}: unexpected status {exc.status}")
                print(f"NEGATIVE {case_id}: rejected ({exc.status})")
            except Exception as exc:
                failures.append(f"{case_id}: crashed instead of rejecting ({exc})")
            else:
                failures.append(f"{case_id}: check unexpectedly PASSED")

        def drop_authority(c, _cl):
            c["authorities"] = [
                a for a in c["authorities"] if a["authority_id"] != "kaldheim_release_notes"
            ]

        def dup_authority(c, _cl):
            c["authorities"].append(dict(c["authorities"][0]))

        def unknown_authority(c, _cl):
            c["authorities"].append(dict(c["authorities"][2], authority_id="nec_release_notes"))

        def empty_citation(c, _cl):
            c["authorities"][0]["citations"] = []

        def wrong_digest(c, _cl):
            c["authorities"][0]["artifact_identity"]["artifact_sha256"] = "0" * 64

        def invalid_locator(c, _cl):
            c["authorities"][0]["citations"][0]["artifact_local_locator"]["line_number_1based"] = (
                9999
            )

        def swapped_provenance(c, _cl):
            # Swap full provenance of two real authorities; both keep valid
            # Wizards hosts and internally-valid locators for their documents.
            a = next(x for x in c["authorities"] if x["authority_id"] == "comprehensive_rules")
            b = next(x for x in c["authorities"] if x["authority_id"] == "commander_general")
            a["original_official_url"], b["original_official_url"] = (
                b["original_official_url"],
                a["original_official_url"],
            )
            a["artifact_identity"], b["artifact_identity"] = (
                b["artifact_identity"],
                a["artifact_identity"],
            )

        def third_party(c, _cl):
            c["authorities"][1]["original_official_url"] = "https://scryfall.com/search?q=x"

        def unresolved_required(c, _cl):
            target = next(
                a for a in c["authorities"] if a["authority_id"] == "magic_2013_release_notes"
            )
            target["citation_status"] = "CITED"

        def identifier_mismatch(c, _cl):
            # Valid CR identifier bound to a valid but FOREIGN heading locator.
            target = next(a for a in c["authorities"] if a["authority_id"] == "comprehensive_rules")
            cite = target["citations"][0]
            cite["rule_identifier"] = "CR 111.1"
            loc = cite["artifact_local_locator"]
            loc["line_number_1based"] = 5
            loc["heading_line_sha256"] = sha256_bytes(SYN_CR_LINES[4].strip().encode())

        def scan_omission(c, _cl):
            # Remove one surface entry AND fix counters: self-consistent proof,
            # caught only by the checker-side canonical catalog.
            target = next(
                a for a in c["authorities"] if a["authority_id"] == "magic_2013_release_notes"
            )
            scan = target["not_required_proof"]["supplementary_string_scan"]
            removed = scan["scan_manifest"].pop()
            scan["files_scanned"] = len(scan["scan_manifest"])

            def hits(blob: bytes) -> int:
                low = blob.lower()
                return sum(low.count(p.encode()) for p in SCAN_PATTERNS)

            total = 0
            for entry in scan["scan_manifest"]:
                entry["hits_total"] = hits(reader.read(entry["path"]))
                total += entry["hits_total"]
            scan["total_relevant_dependency_hits"] = total
            del removed

        def promote_nonterminal(c, cl):
            c["authorities"][3]["citation_status"] = "PENDING_HUMAN_REVIEW"

        def promote_other_gate(_c, cl):
            cl["gate_statuses"]["CLASSIFICATION_REFERENCE_CLOSURE"] = "PASS"

        expect_reject("MISSING_AUTHORITY_REJECTED", drop_authority)
        expect_reject("DUPLICATE_AUTHORITY_REJECTED", dup_authority)
        expect_reject("UNKNOWN_AUTHORITY_REJECTED", unknown_authority)
        expect_reject("EMPTY_CITATION_REJECTED", empty_citation)
        expect_reject("WRONG_ARTIFACT_DIGEST_REJECTED", wrong_digest)
        expect_reject("INVALID_ARTIFACT_LOCATOR_REJECTED", invalid_locator)
        expect_reject("SWAPPED_REGISTER_PROVENANCE_REJECTED", swapped_provenance)
        expect_reject("THIRD_PARTY_AS_AUTHORITY_REJECTED", third_party)
        expect_reject("UNRESOLVED_REQUIRED_AUTHORITY_REJECTED", unresolved_required)
        expect_reject("IDENTIFIER_LOCATOR_MISMATCH_REJECTED", identifier_mismatch)
        expect_reject("SURFACE_CATALOG_OMISSION_REJECTED", scan_omission)
        expect_reject("GATE_PROMOTION_WITH_NONTERMINAL_RECORD_REJECTED", promote_nonterminal)
        expect_reject("OTHER_REV3_GATE_PROMOTION_REJECTED", promote_other_gate)

        # FALSE_NOT_REQUIRED uses tampered payload surfaces through MergedReader.
        cases_run += 1
        tampered_payload = base / "tampered_payload"
        tampered = {
            "inputs/card_semantic_classification_REV3.json": (
                b'{"note": "magic_2013 rules context"}'
            ),
            DECK_ROW_SURFACE: (
                b"card,oracle_top_level_text,oracle_faces\nSafe Card,foretell text,\n"
            ),
            FAMILY_CATALOG_SURFACE: json.dumps([{"id": f"cap.f{i}"} for i in range(4)]).encode(),
            "derived/census.csv": b"pair\n1\n",
            "inputs/ranking_formula_REV3.json": b"{}",
            "derived/Card_Requirement_Map_REV3.csv": b"deck_row_id,requirement_id\nr1,cap.f0\n",
            "derived/Pair_Requirement_Aggregates_REV3.json": b"{}",
            "derived/Interaction_Model_Coverage_REV3.json": b"{}",
            "derived/Ranking_Factors_REV3.json": b"{}",
            "derived/Ranking_Sensitivity_REV3.json": b"{}",
            "derived/Ranking_Uncertainty_Scenarios_REV3.json": b"{}",
            "inputs/oracle_semantic_evidence_REV3.json": b"[]",
        }
        for name, blob in tampered.items():
            path = tampered_payload / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(blob)
        cits, clos = fresh()
        target = next(
            a for a in cits["authorities"] if a["authority_id"] == "magic_2013_release_notes"
        )
        scan = target["not_required_proof"]["supplementary_string_scan"]
        for entry in scan["scan_manifest"]:
            if entry["path"] in tampered:
                entry["sha256"] = sha256_bytes(tampered[entry["path"]])
                low = tampered[entry["path"]].lower()
                entry["hits_total"] = sum(low.count(p.encode()) for p in SCAN_PATTERNS)
        scan["total_relevant_dependency_hits"] = sum(e["hits_total"] for e in scan["scan_manifest"])
        case = write_case("false-not-required", cits, clos)
        try:
            run_check_quiet(case, MergedReader(reader, tampered_payload))
        except B1Error as exc:
            print(f"NEGATIVE FALSE_NOT_REQUIRED_REJECTED: rejected ({exc.status})")
        else:
            failures.append("FALSE_NOT_REQUIRED_REJECTED: check unexpectedly PASSED")

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return EXIT_FAIL
    rejected = cases_run - 1
    print(
        f"B1_NEGATIVE_SELF_TEST = PASS "
        f"(positive control PASS; {rejected}/{rejected} adversarial fixtures rejected)"
    )
    return EXIT_PASS


def run_check_quiet(closures_dir: Path, reader) -> None:
    """run_check without printing (self-test helper)."""
    import contextlib

    with contextlib.redirect_stdout(io.StringIO()):
        run_check(closures_dir, reader)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--closures-dir", type=Path, default=DEFAULT_CLOSURES_DIR)
    parser.add_argument("--negative-self-test", action="store_true")
    args = parser.parse_args()
    try:
        if args.negative_self_test:
            return negative_self_test()
        archive, expected = resolve_archive_path()
        reader = ArchiveReader(archive, expected)
        return run_check(args.closures_dir.resolve(), reader)
    except B1Error as exc:
        print(f"{exc.status}: {exc.message}")
        return EXIT_FAIL if exc.status == "FAIL" else EXIT_BLOCKED


if __name__ == "__main__":
    raise SystemExit(main())
