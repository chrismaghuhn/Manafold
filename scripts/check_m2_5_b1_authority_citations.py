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
CITATIONS_NAME = "official_authority_citations.v2.json"
EXPECTED_CITATIONS_SCHEMA = "manafold.m2.5.b1.official-authority-citations.v2"
EXPECTED_CLOSURE_SCHEMA = "manafold.m2.5.b1.official-authority-citation-closure.v1"
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
    def __init__(self, status: str, code: str, message: str) -> None:
        super().__init__(f"[{code}] {message}")
        self.code = code
        self.status = status
        self.message = message


def fail(code: str, message: str) -> None:
    raise B1Error("FAIL", code, message)


def blocked(code: str, message: str) -> None:
    raise B1Error("BLOCKED", code, message)


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
        fail("IDENTIFIER_FORMAT", f"non-canonical CR rule identifier: {identifier!r}")
    number = match.group(1) + (match.group(2) or "")
    kind = locator.get("locator_kind")
    if kind != "RULE_HEADING_LINE":
        fail(
            "IDENTIFIER_LOCATOR_BINDING",
            f"CR citation locator kind must be RULE_HEADING_LINE, found {kind!r}",
        )
    if "expected_heading_prefix" in locator:
        fail(
            "IDENTIFIER_LOCATOR_BINDING",
            "locator must not carry an independently supplied heading prefix; "
            "it is derived from rule_identifier",
        )
    line_number = locator.get("line_number_1based")
    digest = locator.get("heading_line_sha256")
    if not isinstance(line_number, int) or line_number < 1:
        fail("IDENTIFIER_LOCATOR_BINDING", f"invalid rule heading line number: {line_number!r}")
    lines = artifact.decode("utf-8-sig", errors="strict").split("\n")
    if line_number > len(lines):
        fail(
            "IDENTIFIER_LOCATOR_BINDING",
            f"rule line {line_number} beyond end of artifact ({len(lines)} lines)",
        )
    stripped = lines[line_number - 1].strip()
    if number[-1:].isalpha():
        # subrule citation: the line must BE that subrule, e.g. '702.143a Foretell ...'
        token = stripped.split(" ")[0]
        if token != number:
            fail(
                "IDENTIFIER_LOCATOR_BINDING",
                (
                    f"identifier/locator binding failure: {identifier} resolves to "
                    f"{token!r} at line {line_number}"
                ),
            )
    else:
        # section citation: heading must start '<num>.' exactly
        if not stripped.startswith(number + "."):
            fail(
                "IDENTIFIER_LOCATOR_BINDING",
                (
                    f"identifier/locator binding failure: {identifier} resolves to "
                    f"{stripped[:20]!r} at line {line_number}"
                ),
            )
    if sha256_bytes(stripped.encode()) != digest:
        fail(
            "IDENTIFIER_LOCATOR_BINDING",
            f"line {line_number} heading digest mismatch for {identifier}",
        )


def resolve_byte_fragment(artifact: bytes, locator: dict) -> None:
    if locator.get("locator_kind") != "UNIQUE_BYTE_FRAGMENT":
        fail("ARTIFACT_LOCATOR_INVALID", f"unknown locator kind: {locator.get('locator_kind')!r}")
    offset = locator.get("byte_offset")
    length = locator.get("byte_length")
    digest = locator.get("fragment_sha256")
    if not isinstance(offset, int) or not isinstance(length, int) or offset < 0 or length <= 0:
        fail(
            "ARTIFACT_LOCATOR_INVALID",
            f"invalid byte fragment geometry: offset={offset!r} len={length!r}",
        )
    if offset + length > len(artifact):
        fail(
            "ARTIFACT_LOCATOR_INVALID",
            f"byte fragment [{offset}:{offset + length}] beyond artifact size {len(artifact)}",
        )
    window = artifact[offset : offset + length]
    if sha256_bytes(window) != digest:
        fail("ARTIFACT_LOCATOR_INVALID", f"byte fragment digest mismatch at offset {offset}")
    if artifact.count(window) != 1:
        fail(
            "ARTIFACT_LOCATOR_INVALID",
            f"byte fragment at offset {offset} occurs {artifact.count(window)} times",
        )


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
            fail(
                "CITATION_FIELD_EMPTY", f"citation {citation.get('citation_id')!r} has empty {key}"
            )
    kind = citation["citation_kind"]
    identifier = citation.get("rule_identifier")
    if kind == "CR_RULE_IDENTIFIER":
        if not isinstance(identifier, str) or not CR_RULE_RE.fullmatch(identifier):
            fail("IDENTIFIER_FORMAT", f"non-canonical CR rule identifier: {identifier!r}")
        resolve_rule_line(artifacts[artifact_path], identifier, citation["artifact_local_locator"])
    elif kind in {"POLICY_SECTION_LOCATOR", "RELEASE_NOTE_LOCATOR"}:
        if identifier is not None:
            fail("CITATION_KIND", f"{kind} must not carry a fabricated rule identifier")
        resolve_byte_fragment(artifacts[artifact_path], citation["artifact_local_locator"])
    else:
        fail("CITATION_KIND", f"unknown citation kind: {kind!r}")


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
                "REGISTER_CROSS_BINDING",
                (
                    f"authority {record['authority_id']} field {field!r} contradicts the "
                    f"pinned REV3 register ({recorded!r} != {pinned!r})"
                ),
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
        fail("AUTHORITY_UNKNOWN", f"unknown authority id: {authority_id!r}")
    if authority_id in seen_ids:
        fail("AUTHORITY_DUPLICATE", f"duplicate authority id: {authority_id!r}")
    seen_ids.add(authority_id)
    role = record.get("authority_role")
    if not isinstance(role, str) or not role.startswith(ALLOWED_ROLES_PREFIX):
        fail("AUTHORITY_NOT_OFFICIAL", f"authority {authority_id} role is not official: {role!r}")
    url = record.get("original_official_url")
    host_ok = isinstance(url, str) and url.split("/")[2] in ALLOWED_URL_HOSTS
    if not host_ok:
        fail(
            "AUTHORITY_NOT_OFFICIAL",
            f"authority {authority_id} URL is not an official Wizards origin: {url!r}",
        )

    status = record.get("citation_status")
    identity = record.get("artifact_identity")
    if not isinstance(identity, dict):
        fail(f"authority {authority_id} lacks artifact_identity")

    cross_bind_register(record, register[authority_id])

    if status == "CITED":
        path = identity.get("artifact_path")
        digest = identity.get("artifact_sha256")
        if not isinstance(path, str) or not isinstance(digest, str):
            fail(
                "CITED_RECORD_INCOMPLETE",
                f"CITED authority {authority_id} lacks pinned artifact identity",
            )
        if path not in artifacts_cache:
            artifacts_cache[path] = reader.read(path)
        if sha256_bytes(artifacts_cache[path]) != digest:
            fail("ARTIFACT_DIGEST_MISMATCH", f"artifact digest mismatch for {authority_id}: {path}")
        retrieval = identity.get("retrieval_time")
        if not isinstance(retrieval, str) or not retrieval.strip():
            fail("CITED_RECORD_INCOMPLETE", f"CITED authority {authority_id} lacks retrieval_time")
        citations = record.get("citations")
        if not isinstance(citations, list) or not citations:
            fail("EMPTY_CITATIONS", f"CITED authority {authority_id} has empty citations[]")
        for citation in citations:
            validate_citation(citation, artifacts_cache, path)
    elif status == "NOT_REQUIRED_WITH_PROOF":
        proof = record.get("not_required_proof")
        if not isinstance(proof, dict):
            fail(
                "PROOF_OBJECT_MISSING", f"NOT_REQUIRED authority {authority_id} lacks proof object"
            )
    else:
        fail(
            "NON_TERMINAL_STATUS",
            (
                f"authority {authority_id} has non-terminal status {status!r}; "
                "only CITED / NOT_REQUIRED_WITH_PROOF close"
            ),
        )
    cross_bind_register(record, register[authority_id])


def reexecute_dependency_scan(record: dict, reader) -> None:
    proof = record["not_required_proof"]
    scan = proof.get("supplementary_string_scan", proof.get("dependency_scan"))
    if not isinstance(scan, dict):
        fail("SCAN_MANIFEST_MISSING", "supplementary string scan missing")
    patterns = [p.lower() for p in scan.get("patterns", [])]
    if sorted(patterns) != sorted(SCAN_PATTERNS):
        fail("SCAN_PATTERNS_DEVIATION", "scan patterns differ from the canonical pattern set")
    manifest = scan.get("scan_manifest")
    if not isinstance(manifest, list):
        fail("SCAN_MANIFEST_MALFORMED", "dependency scan manifest malformed")

    manifest_paths = [entry.get("path") for entry in manifest]
    if sorted(manifest_paths) != sorted(SURFACE_FILES):
        missing = sorted(set(SURFACE_FILES) - set(manifest_paths))
        extra = sorted(set(manifest_paths) - set(SURFACE_FILES))
        fail(
            "SURFACE_CATALOG_DEVIATION",
            f"surface catalog deviation; missing={missing} extra={extra}",
        )

    total = 0
    for entry in manifest:
        rel = entry.get("path")
        data = reader.read(rel)
        if sha256_bytes(data) != entry.get("sha256"):
            fail("SURFACE_DIGEST_MISMATCH", f"payload surface changed since scan: {rel}")
        low = data.lower()
        hits = sum(low.count(p.encode()) for p in SCAN_PATTERNS)
        if hits != entry.get("hits_total"):
            fail("SCAN_HIT_MISMATCH", f"re-scanned hit count differs for {rel}: {hits}")
        total += hits
    if len(manifest) != scan.get("files_scanned"):
        fail("SCAN_SUMMARY_MISMATCH", "dependency scan summary disagrees with executed re-scan")
    if total != 0 or scan.get("total_relevant_dependency_hits") != 0:
        fail(
            "DEPENDENCY_HIT_FOUND",
            (
                f"FALSE_NOT_REQUIRED: {total} relevant dependency hits exist while the "
                "authority is marked NOT_REQUIRED_WITH_PROOF"
            ),
        )


def family_corpora(reader) -> tuple[dict[str, str], dict[str, int]]:
    """Recompute per-family oracle-text corpora from the pinned payload."""
    map_raw = reader.read("derived/Card_Requirement_Map_REV3.csv").decode("utf-8", errors="strict")
    res_raw = reader.read(DECK_ROW_SURFACE).decode("utf-8", errors="strict")
    res_rows = {r["deck_row_id"]: r for r in csv.DictReader(io.StringIO(res_raw))}
    corpus: dict[str, str] = {}
    members: dict[str, int] = {}
    for m in csv.DictReader(io.StringIO(map_raw)):
        fid = m["requirement_id"]
        r = res_rows[m["deck_row_id"]]
        corpus[fid] = (
            corpus.get(fid, "")
            + (
                ((r.get("oracle_top_level_text") or "") + "\n" + (r.get("oracle_faces") or ""))
                + "\n"
            ).lower()
        )
        members[fid] = members.get(fid, 0) + 1
    return corpus, members


def validate_dependency_model(citations: dict, authorities_by_id: dict, reader) -> None:
    model = citations.get("semantic_dependency_model")
    if not isinstance(model, dict):
        fail("MODEL_MISSING", "semantic_dependency_model missing; string absence proves nothing")
    catalog_raw = reader.read(FAMILY_CATALOG_SURFACE)
    catalog_ids = {f["id"] for f in json.loads(catalog_raw.decode("utf-8"))}
    coverage = model.get("family_coverage")
    if not isinstance(coverage, dict):
        fail("MODEL_PARTITION", "family_coverage missing")
    if set(coverage.keys()) != catalog_ids:
        missing = sorted(catalog_ids - set(coverage.keys()))[:10]
        extra = sorted(set(coverage.keys()) - catalog_ids)[:10]
        fail(
            "MODEL_PARTITION",
            f"coverage is not the complete partition; missing={missing} extra={extra}",
        )

    cited_citation_ids = {
        c["citation_id"]: aid
        for aid, a in authorities_by_id.items()
        if a.get("citation_status") == "CITED"
        for c in a.get("citations", [])
    }
    corpus, _members = family_corpora(reader)
    structural_seen: set[str] = set()
    for fid, entry in sorted(coverage.items()):
        if not isinstance(entry, dict) or "citation_id" not in entry:
            fail("MODEL_EDGE_MALFORMED", f"malformed coverage edge for {fid}")
        target = entry["citation_id"]
        owner = cited_citation_ids.get(target)
        if owner is None:
            fail("MODEL_EDGE_TARGET", f"family {fid} covers into non-existent citation {target!r}")
        if owner == NOT_REQUIRED_AUTHORITY:
            fail("MODEL_MAGIC2013_EDGE", f"family {fid} depends on the NOT_REQUIRED authority")
        basis = entry.get("coverage_basis")
        if basis == "lexical_markers":
            cite = cited_citation_ids and _citation_by_id(authorities_by_id, target)
            markers = (cite or {}).get("semantic_markers") or []
            matched = entry.get("matched_marker")
            if not isinstance(matched, str) or matched not in markers:
                fail(
                    "MODEL_MARKER_UNDECLARED",
                    f"family {fid}: marker {matched!r} not declared on {target}",
                )
            if matched.lower() not in corpus.get(fid, ""):
                fail(
                    "MODEL_MARKER_ABSENT",
                    f"family {fid}: marker {matched!r} does not occur in its "
                    "member-card oracle corpus",
                )
        elif basis in {"structural_grounding", "policy_structural"}:
            structural_seen.add(fid)
            rationale = entry.get("rationale")
            if basis == "structural_grounding" and (
                not isinstance(rationale, str) or not rationale.strip()
            ):
                fail("MODEL_STRUCTUREL_RATIONALE", f"structural edge {fid} lacks rationale")
        else:
            fail("MODEL_BASIS_UNKNOWN", f"unknown coverage basis {basis!r} for {fid}")

    whitelist = model.get("structural_edge_whitelist")
    if not isinstance(whitelist, list) or set(whitelist) != structural_seen:
        fail(
            "MODEL_WHITELIST_MISMATCH",
            "structural_edge_whitelist does not match the actual structural edges",
        )

    mechanic_edges = model.get("mechanic_edges")
    if not isinstance(mechanic_edges, dict):
        fail("MODEL_MECHANIC_EDGES_MISSING", "mechanic_edges missing")
    detected = detect_mechanics(reader)
    for concept in sorted(detected):
        edges = mechanic_edges.get(concept)
        if not edges:
            fail(
                "MODEL_MECHANIC_UNCOVERED",
                f"detected oracle-text mechanic {concept!r} has no covering edge",
            )
        for edge in edges:
            owner = cited_citation_ids.get(edge)
            if owner is None or owner == NOT_REQUIRED_AUTHORITY:
                fail("MODEL_MECHANIC_BAD_EDGE", f"mechanic {concept!r} edge {edge!r} is invalid")

    declared = model.get("magic_2013_dependency_edge_count")
    all_targets = [v["citation_id"] for v in coverage.values()] + [
        e for v in mechanic_edges.values() for e in v
    ]
    structural = sum(1 for x in all_targets if x == NOT_REQUIRED_AUTHORITY)
    if declared != 0 or structural != 0:
        fail(
            "MODEL_MAGIC2013_EDGES",
            f"magic_2013 carries dependency edges (declared={declared}, structural={structural})",
        )


def _citation_by_id(authorities_by_id: dict, citation_id: str):
    for a in authorities_by_id.values():
        if a.get("citation_status") != "CITED":
            continue
        for c in a.get("citations", []):
            if c["citation_id"] == citation_id:
                return c
    return None


def validate_closure_agreement(closure: dict, authorities: list[dict]) -> None:
    gates = closure.get("gate_statuses")
    if not isinstance(gates, dict):
        fail("CLOSURE_MALFORMED", "closure.gate_statuses missing")
    if gates.get("OFFICIAL_RULE_CITATION_CLOSURE") != "PASS":
        fail("GRANT_MISSING", "closure does not grant OFFICIAL_RULE_CITATION_CLOSURE = PASS")
    for gate in REQUIRED_GATES_BLOCKED:
        if gates.get(gate) != "BLOCKED":
            fail(
                "GATE_PROMOTION_FORBIDDEN", f"B1 may not promote {gate}: found {gates.get(gate)!r}"
            )
    terminals = {
        a["authority_id"]
        for a in authorities
        if a.get("citation_status") in ("CITED", "NOT_REQUIRED_WITH_PROOF")
    }
    if closure.get("AUTHORITY_TERMINAL_COUNT") != len(terminals):
        fail("TERMINAL_COUNT_DISAGREEMENT", "AUTHORITY_TERMINAL_COUNT disagrees with records")
    if len(terminals) != len(authorities):
        fail("NON_TERMINAL_PRESENT", "cannot grant PASS while non-terminal authority records exist")
    if closure.get("unresolved_authority_ids") != []:
        fail("UNRESOLVED_PRESENT", "closure still lists unresolved authorities")
    if closure.get("third_party_authority_count") != 0:
        fail("THIRD_PARTY_PROMOTION", "third-party authority promotion recorded")
    for key in ("DECK_PAIR_LOCKED", "AUTHORITATIVE_RANKING_AVAILABLE", "M3_STARTED"):
        if closure.get(key) is not False:
            fail("STATE_FLAG_VIOLATION", f"{key} must remain false after B1")
    if closure.get("M2_5_A_COMPLETE") is not True:
        fail("STATE_FLAG_VIOLATION", "M2.5.A completion flag missing")


def run_check(closures_dir: Path, reader) -> int:
    citations_path = closures_dir / CITATIONS_NAME
    closure_path = closures_dir / CLOSURE_NAME
    report_path = closures_dir / REPORT_NAME
    if not citations_path.is_file() or not closure_path.is_file():
        blocked(f"B1 evidence missing under {closures_dir}")
    citations_raw = citations_path.read_bytes()
    citations = read_json_bytes(citations_raw, CITATIONS_NAME)
    closure = read_json_bytes(closure_path.read_bytes(), CLOSURE_NAME)
    if citations.get("schema") != EXPECTED_CITATIONS_SCHEMA:
        fail("SCHEMA_MISMATCH", f"unexpected citations schema: {citations.get('schema')!r}")
    if closure.get("schema") != EXPECTED_CLOSURE_SCHEMA:
        fail("SCHEMA_MISMATCH", f"unexpected closure schema: {closure.get('schema')!r}")

    universe_obj = citations.get("input_universe")
    if not isinstance(universe_obj, dict):
        fail("UNIVERSE_DECLARATION", "citations.input_universe missing")
    if universe_obj.get("authority_ids_in_order") != EXPECTED_UNIVERSE:
        fail(
            "UNIVERSE_DECLARATION",
            "citations input universe is not the exact REV3 seven-authority set",
        )
    register_digest = universe_obj.get("source_register_sha256")
    register_raw = reader.read("source/official_authority_register_REV3.json")
    if sha256_bytes(register_raw) != register_digest:
        fail(
            "REGISTER_DIGEST_MISMATCH",
            "REV3 authority register digest mismatch; input universe is not pinned",
        )
    register = parse_register(register_raw)

    authorities = citations.get("authorities")
    if not isinstance(authorities, list):
        fail("CLOSURE_DISAGREEMENT", "citations.authorities missing")
    if closure.get("AUTHORITY_INPUT_COUNT") != len(EXPECTED_UNIVERSE):
        fail("CLOSURE_DISAGREEMENT", "AUTHORITY_INPUT_COUNT must be 7")
    seen: set[str] = set()
    artifacts_cache: dict[str, bytes] = {}
    for record in authorities:
        validate_authority_record(
            record, seen, EXPECTED_UNIVERSE, reader, artifacts_cache, register
        )
    if seen != set(EXPECTED_UNIVERSE):
        missing = sorted(set(EXPECTED_UNIVERSE) - seen)
        fail("AUTHORITY_UNIVERSE_INCOMPLETE", f"incomplete authority universe; missing: {missing}")
    authorities_by_id = {a["authority_id"]: a for a in authorities}
    for record in authorities:
        if record["citation_status"] == "NOT_REQUIRED_WITH_PROOF":
            reexecute_dependency_scan(record, reader)
    validate_dependency_model(citations, authorities_by_id, reader)

    binding = closure.get("bound_evidence")
    if not isinstance(binding, dict):
        fail(
            "EVIDENCE_BINDING_MISSING",
            "closure.bound_evidence missing; B1 evidence must be digest-bound",
        )
    expected_bound = {CITATIONS_NAME: citations_raw, REPORT_NAME: report_path.read_bytes()}
    for name, raw in expected_bound.items():
        if sha256_bytes(raw) != binding.get(name):
            fail("EVIDENCE_DIGEST_MISMATCH", f"bound evidence digest mismatch: {name}")

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


def build_synthetic_fixture(
    base: Path, *, tamper_policy_byte: bool = False
) -> tuple[Path, ArchiveReader]:
    """Create a consistent synthetic archive/payload/closures fixture."""
    archive_root = base / "archive" / "m2_5"
    archive_root.mkdir(parents=True)
    cr_text = "\n".join(SYN_CR_LINES) + "\n"
    html_page = b"<html><h2>Play Rules</h2>policy body</html>"
    payload_files = {
        DECK_ROW_SURFACE: (
            b"deck_row_id,card,oracle_top_level_text,oracle_faces\nr1,Safe Card,foretell text,\n"
        ),
        "derived/Pair_Interaction_Census_REV3.csv": b"pair\n1\n",
        "inputs/card_semantic_classification_REV3.json": b'[{"id": "a"}]',
        FAMILY_CATALOG_SURFACE: json.dumps([{"id": f"cap.f{i}"} for i in range(4)]).encode(),
        "derived/census.csv": b"pair\n1\n",
        "inputs/ranking_formula_REV3.json": b"{}",
        "derived/Card_Requirement_Map_REV3.csv": (
            b"deck_row_id,requirement_id\nr1,cap.f0\nr1,cap.f1\nr1,cap.f2\nr1,cap.f3\n"
        ),
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
    if tamper_policy_byte:
        # Flip archive bytes AFTER every recorded digest was derived, so the
        # records still claim the pristine artifact while the archive lies.
        members["source/authorities/policy.html"] += b"<!-- x -->"

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

    def covered_families(count: int) -> dict[str, dict]:
        return {
            f"cap.f{i}": {
                "citation_id": "SYN-CR-111",
                "coverage_basis": "lexical_markers",
                "matched_marker": "foretell",
                "member_deck_rows": 1,
            }
            for i in range(count)
        }

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
            "structural_edge_whitelist": [],
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
                        "coverage_basis": "lexical_markers",
                        "semantic_markers": ["foretell"],
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
    """Adversarial fixtures against the real checker logic (positive control first)."""
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
                f"BASELINE_FIXTURE_NOT_PASSING ({exc.status}/{exc.code}: {exc.message}); "
                "rejections would be vacuous"
            )

        def fresh() -> tuple[dict, dict]:
            cits = read_json_bytes((closures / CITATIONS_NAME).read_bytes(), "")
            clos = read_json_bytes((closures / CLOSURE_NAME).read_bytes(), "")
            return cits, clos

        def write_case(name: str, cits: dict, clos: dict, *, rebind: bool) -> Path:
            case = base / "cases" / name
            case.mkdir(parents=True)
            cit_raw = json.dumps(cits, indent=2).encode() + b"\n"
            clos_raw = json.dumps(clos, indent=2).encode() + b"\n"
            if rebind:
                clos["bound_evidence"][CITATIONS_NAME] = sha256_bytes(cit_raw)
                clos_raw = json.dumps(clos, indent=2).encode() + b"\n"
            (case / CITATIONS_NAME).write_bytes(cit_raw)
            (case / CLOSURE_NAME).write_bytes(clos_raw)
            (case / REPORT_NAME).write_bytes(b"# B1 report\n")
            return case

        def expect_reject(
            case_id: str,
            expected_code: str,
            mutate,
            *,
            rebind: bool = True,
        ) -> None:
            nonlocal cases_run
            cases_run += 1
            cits, clos = fresh()
            mutate(cits, clos)
            case = write_case(case_id.lower(), cits, clos, rebind=rebind)
            try:
                run_check_quiet(case, reader)
            except B1Error as exc:
                if exc.status not in {"FAIL", "BLOCKED"}:
                    failures.append(f"{case_id}: unexpected status {exc.status}")
                elif exc.code != expected_code:
                    failures.append(f"{case_id}: expected code {expected_code}, got {exc.code}")
                else:
                    print(f"NEGATIVE {case_id}: rejected ({exc.status}/{exc.code})")
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

        def foreign_heading_locator() -> dict:
            return {
                "locator_kind": "RULE_HEADING_LINE",
                "line_number_1based": 2,
                "heading_line_sha256": sha256_bytes(SYN_CR_LINES[1].strip().encode()),
            }

        def identifier_mismatch(c, _cl):
            # Valid section identifier bound to a valid but FOREIGN heading line.
            target = next(a for a in c["authorities"] if a["authority_id"] == "comprehensive_rules")
            cite = target["citations"][0]
            cite["rule_identifier"] = "CR 111"
            cite["artifact_local_locator"] = foreign_heading_locator()

        def scan_omission(c, _cl):
            # Remove one surface entry and fix counters: self-consistent proof,
            # caught only by the checker-side canonical catalog.
            target = next(
                a for a in c["authorities"] if a["authority_id"] == "magic_2013_release_notes"
            )
            scan = target["not_required_proof"]["supplementary_string_scan"]
            scan["scan_manifest"].pop()
            scan["files_scanned"] = len(scan["scan_manifest"])

            def hits(blob: bytes) -> int:
                low = blob.lower()
                return sum(low.count(pat.encode()) for pat in SCAN_PATTERNS)

            total = 0
            for entry in scan["scan_manifest"]:
                entry["hits_total"] = hits(reader.read(entry["path"]))
                total += entry["hits_total"]
            scan["total_relevant_dependency_hits"] = total

        def promote_nonterminal(c, cl):
            c["authorities"][3]["citation_status"] = "PENDING_HUMAN_REVIEW"

        def promote_other_gate(_c, cl):
            cl["gate_statuses"]["CLASSIFICATION_REFERENCE_CLOSURE"] = "PASS"

        def wrong_citations_schema(c, _cl):
            c["schema"] = "manafold.m2.5.b1.official-authority-citations.v1"

        def wrong_closure_schema(_c, cl):
            cl["schema"] = "manafold.m2.5.b1.official-authority-citation-closure.v9"

        def tamper_without_rebind(c, _cl):
            c["tamper_marker"] = True

        expect_reject("MISSING_AUTHORITY_REJECTED", "AUTHORITY_UNIVERSE_INCOMPLETE", drop_authority)
        expect_reject("DUPLICATE_AUTHORITY_REJECTED", "AUTHORITY_DUPLICATE", dup_authority)
        expect_reject("UNKNOWN_AUTHORITY_REJECTED", "AUTHORITY_UNKNOWN", unknown_authority)
        expect_reject("EMPTY_CITATION_REJECTED", "EMPTY_CITATIONS", empty_citation)
        expect_reject("WRONG_ARTIFACT_DIGEST_REJECTED", "REGISTER_CROSS_BINDING", wrong_digest)
        expect_reject(
            "INVALID_ARTIFACT_LOCATOR_REJECTED", "IDENTIFIER_LOCATOR_BINDING", invalid_locator
        )
        expect_reject(
            "SWAPPED_REGISTER_PROVENANCE_REJECTED", "REGISTER_CROSS_BINDING", swapped_provenance
        )
        expect_reject("THIRD_PARTY_AS_AUTHORITY_REJECTED", "AUTHORITY_NOT_OFFICIAL", third_party)
        expect_reject(
            "UNRESOLVED_REQUIRED_AUTHORITY_REJECTED", "CITED_RECORD_INCOMPLETE", unresolved_required
        )
        expect_reject(
            "IDENTIFIER_LOCATOR_MISMATCH_REJECTED",
            "IDENTIFIER_LOCATOR_BINDING",
            identifier_mismatch,
        )
        expect_reject(
            "SURFACE_CATALOG_OMISSION_REJECTED", "SURFACE_CATALOG_DEVIATION", scan_omission
        )
        expect_reject(
            "GATE_PROMOTION_WITH_NONTERMINAL_RECORD_REJECTED",
            "NON_TERMINAL_STATUS",
            promote_nonterminal,
        )
        expect_reject(
            "OTHER_REV3_GATE_PROMOTION_REJECTED", "GATE_PROMOTION_FORBIDDEN", promote_other_gate
        )
        expect_reject("WRONG_CITATIONS_SCHEMA_REJECTED", "SCHEMA_MISMATCH", wrong_citations_schema)
        expect_reject("WRONG_CLOSURE_SCHEMA_REJECTED", "SCHEMA_MISMATCH", wrong_closure_schema)
        expect_reject(
            "DIGEST_BINDING_TAMPER_REJECTED",
            "EVIDENCE_DIGEST_MISMATCH",
            tamper_without_rebind,
            rebind=False,
        )

        # Byte-level artifact tamper: archive member differs while every
        # recorded digest still claims the pristine artifact.
        cases_run += 1
        with tempfile.TemporaryDirectory() as tmp2:
            base2 = Path(tmp2)
            closures2, reader2 = build_synthetic_fixture(base2, tamper_policy_byte=True)
            try:
                run_check_quiet(closures2, reader2)
            except B1Error as exc:
                if exc.code != "ARTIFACT_DIGEST_MISMATCH":
                    failures.append(f"TAMPERED_ARCHIVE_MEMBER_REJECTED: wrong code {exc.code}")
                else:
                    print(
                        "NEGATIVE TAMPERED_ARCHIVE_MEMBER_REJECTED: rejected "
                        f"({exc.status}/{exc.code})"
                    )
            except Exception as exc:
                failures.append(f"TAMPERED_ARCHIVE_MEMBER_REJECTED: crashed ({exc})")
            else:
                failures.append("TAMPERED_ARCHIVE_MEMBER_REJECTED: check unexpectedly PASSED")

        # FALSE_NOT_REQUIRED: consistent manifest over tampered surfaces that now
        # genuinely contains a dependency hit.
        cases_run += 1
        tampered_payload = base / "tampered_payload"
        tampered = {
            "inputs/card_semantic_classification_REV3.json": (
                b'{"note": "magic_2013 rules context"}'
            ),
            DECK_ROW_SURFACE: (
                b"deck_row_id,card,oracle_top_level_text,oracle_faces\n"
                b"r1,Safe Card,foretell text,\n"
            ),
            FAMILY_CATALOG_SURFACE: json.dumps([{"id": f"cap.f{i}"} for i in range(4)]).encode(),
            "derived/Pair_Interaction_Census_REV3.csv": b"pair\n1\n",
            "derived/Card_Requirement_Map_REV3.csv": (
                b"deck_row_id,requirement_id\nr1,cap.f0\nr1,cap.f1\nr1,cap.f2\nr1,cap.f3\n"
            ),
            "derived/Pair_Requirement_Aggregates_REV3.json": b"{}",
            "derived/Interaction_Model_Coverage_REV3.json": b"{}",
            "derived/Ranking_Factors_REV3.json": b"{}",
            "derived/Ranking_Sensitivity_REV3.json": b"{}",
            "derived/Ranking_Uncertainty_Scenarios_REV3.json": b"{}",
            "inputs/ranking_formula_REV3.json": b"{}",
            "inputs/oracle_semantic_evidence_REV3.json": b"[]",
        }
        for name, blob in tampered.items():
            path = tampered_payload / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(blob)
        cits, clos = fresh()
        not_required_records = [
            a for a in cits["authorities"] if a.get("citation_status") == "NOT_REQUIRED_WITH_PROOF"
        ]
        assert len(not_required_records) >= 1
        for target in not_required_records:
            scan = target["not_required_proof"]["supplementary_string_scan"]
            for entry in scan["scan_manifest"]:
                if entry["path"] in tampered:
                    entry["sha256"] = sha256_bytes(tampered[entry["path"]])
                    low = tampered[entry["path"]].lower()
                    entry["hits_total"] = sum(low.count(pat.encode()) for pat in SCAN_PATTERNS)
            scan["total_relevant_dependency_hits"] = sum(
                e["hits_total"] for e in scan["scan_manifest"]
            )
        case = write_case("false-not-required", cits, clos, rebind=False)
        try:
            run_check_quiet(case, MergedReader(reader, tampered_payload))
        except B1Error as exc:
            if exc.code != "DEPENDENCY_HIT_FOUND":
                failures.append(f"FALSE_NOT_REQUIRED_REJECTED: wrong code {exc.code}")
            else:
                print(f"NEGATIVE FALSE_NOT_REQUIRED_REJECTED: rejected ({exc.status}/{exc.code})")
        except Exception as exc:
            failures.append(f"FALSE_NOT_REQUIRED_REJECTED: crashed ({exc})")
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
