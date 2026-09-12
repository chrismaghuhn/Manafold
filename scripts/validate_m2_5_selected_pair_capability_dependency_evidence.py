#!/usr/bin/env python3
"""Build and validate selected-pair capability dependency evidence."""

from __future__ import annotations

import argparse
import csv
import json
import os
from pathlib import Path
from typing import Any

import jsonschema
from validate_m2_5_exact_two_deck_scope_lock import (
    load_lock,
    validate_lock_document,
    verify_pinned_archive,
)
from validate_m2_5_selected_pair_cdi_census import (
    ARTIFACTS as CDI_ARTIFACTS,
)
from validate_m2_5_selected_pair_cdi_census import (
    B2_FILES,
    EXPECTED_SOURCE_PACKAGE_SHA256,
    ROOT,
    SCOPE_RELATIVE_PATH,
    _canonical,
    _dependency_source_bindings,
    _family_owner_roles,
    _json,
    _lock_context,
    _preserved_artifacts,
    _root_dependency_evidence,
    _selected_capability_binding,
    _sha256,
    compute_artifact_content_sha256,
    load_census_artifacts,
    validate_b2_projection_rows,
    validate_dependency_edges,
)

TASK_ID = "M2_5_SELECTED_PAIR_CAPABILITY_DEPENDENCY_EVIDENCE_01"
SCHEMA_ID = "manafold.m2.5.selected-pair-capability-dependency-evidence.v1"
EVIDENCE_SCHEMA_ID = SCHEMA_ID
SCHEMA_RELATIVE_PATH = Path(
    "schemas/m2-5-selected-pair-capability-dependency-evidence.v1.schema.json"
)
ARTIFACT_RELATIVE_PATH = (
    SCOPE_RELATIVE_PATH / "selected_pair_capability_dependency_evidence.v1.json"
)
ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"
DISPOSITIONS = frozenset(
    {
        "DEPENDENCY_SET_EVIDENCED",
        "TERMINAL_LEAF_EVIDENCED",
        "UNRESOLVED_DEPENDENCY_EVIDENCE",
    }
)


class ScopeDependencyEvidenceError(ValueError):
    """The selected-pair dependency evidence contract is malformed."""


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ScopeDependencyEvidenceError(message)


def _file_binding(root: Path, path: str, role: str, locator: str) -> dict[str, Any]:
    file_path = root / path
    _require(file_path.is_file(), f"evidence input is missing: {path}")
    return {
        "path": path,
        "raw_sha256": _sha256(file_path.read_bytes()),
        "evidence_role": role,
        "locator": locator,
    }


def _input_bindings(
    root: Path,
    census: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    lock_path = root / "sources/m2_5/scope/exact_two_deck_scope_lock.v1.json"
    closure_path = root / SCOPE_RELATIVE_PATH / CDI_ARTIFACTS["recursive_capability_closure"]
    registry_path = root / "cards/capabilities/registry.json"
    return {
        "scope_lock": {
            "path": lock_path.relative_to(root).as_posix(),
            "raw_sha256": _sha256(lock_path.read_bytes()),
            "schema": "manafold.m2.5.exact-two-deck-scope-lock.v1",
        },
        "capability_census": {
            **_selected_capability_binding(root, census["capability"]),
            "schema": "manafold.m2.5.selected-pair-cdi-census.v1",
        },
        "recursive_capability_closure": {
            "path": closure_path.relative_to(root).as_posix(),
            "raw_sha256": _sha256(closure_path.read_bytes()),
            "content_sha256": census["recursive_capability_closure"]["content_sha256"],
            "schema": "manafold.m2.5.selected-pair-recursive-capability-closure.v2",
        },
        "b2_current_root": _file_binding(
            root,
            B2_FILES["current_root"][0],
            "ACCEPTED_B2_CURRENT_ROOT",
            "artifact_role",
        ),
        "b2_closure": _file_binding(
            root,
            B2_FILES["closure_v2"][0],
            "ACCEPTED_B2_CLOSURE",
            "snapshot_constants",
        ),
        "b2_classifications": _file_binding(
            root,
            B2_FILES["classifications"][0],
            "ACCEPTED_B2_CLASSIFICATIONS",
            "requirement_assignments",
        ),
        "b2_catalog": _file_binding(
            root,
            B2_FILES["family_catalog"][0],
            "ACCEPTED_B2_FAMILY_CATALOG",
            "families",
        ),
        "b2_projection": _file_binding(
            root,
            B2_FILES["projection"][0],
            "ACCEPTED_B2_PROJECTION",
            "deck_row_classification_refs",
        ),
        "dependency_sources": _dependency_source_bindings(root),
        "production_capability_registry": {
            **_file_binding(
                root,
                "cards/capabilities/registry.json",
                "PRODUCTION_CAPABILITY_REGISTRY_SNAPSHOT",
                "entries",
            ),
            "entry_count": len(_json(registry_path)["entries"]),
        },
    }


def _owner_map(capability: dict[str, Any]) -> dict[str, list[str]]:
    result = {
        family["family_id"]: sorted(set(family["semantic_owner_roles"]))
        for family in capability["families"]
    }
    _require(all(result.values()), "selected root is missing semantic owner")
    return dict(sorted(result.items()))


def _source_bound_root_refs(
    root: Path,
    family_id: str,
    capability: dict[str, Any],
    closure: dict[str, Any],
) -> list[dict[str, Any]]:
    capability_binding = _selected_capability_binding(root, capability)
    dependency_bindings = _dependency_source_bindings(root)
    expected = _root_dependency_evidence(root, family_id, capability_binding, dependency_bindings)
    persisted = {item["family_id"]: item for item in closure["root_classifications"]}.get(family_id)
    _require(persisted is not None, f"persisted closure root evidence is missing: {family_id}")
    _require(
        persisted["evidence_refs"] == expected,
        f"persisted closure root evidence is not B2-bound: {family_id}",
    )
    return persisted["evidence_refs"]


def _validate_capability_census_against_b2(root: Path, capability: dict[str, Any]) -> None:
    catalog = {
        family["family_id"]: family
        for family in _json(root / B2_FILES["family_catalog"][0])["families"]
    }
    classifications = {
        item["oracle_semantic_identity"]: item
        for item in _json(root / B2_FILES["classifications"][0])["classifications"]
    }
    with (root / B2_FILES["projection"][0]).open(encoding="utf-8", newline="") as handle:
        projection = list(csv.DictReader(handle))
    selected_projection = [
        row for row in projection if row["deck_id"] in {"Token Triumph", "Grave Danger"}
    ]
    validate_b2_projection_rows(selected_projection, classifications, capability)
    _lock, _lock_sha, selected_osis, row_ids = _lock_context(root)
    _require(
        {record["oracle_semantic_identity"] for record in capability["records"]} == selected_osis,
        "capability census Oracle identities are not exact",
    )
    seen_rows: set[str] = set()
    assignment_counts: dict[str, dict[str, int]] = {}
    seen_edges: set[tuple[str, str]] = set()
    for record in capability["records"]:
        osi = record["oracle_semantic_identity"]
        classification = classifications.get(osi)
        _require(classification is not None, f"capability census classification missing: {osi}")
        _require(
            record["classification_identity_sha256"]
            == classification["classification_identity"]["digest_hex"],
            f"capability census classification identity mismatch: {osi}",
        )
        _require(
            record["review_status"] == classification["review_status"],
            f"capability census review status mismatch: {osi}",
        )
        for row_id in record["deck_row_ids"]:
            _require(
                row_id in row_ids and row_id not in seen_rows,
                f"capability census row binding mismatch: {row_id}",
            )
            seen_rows.add(row_id)
        source_assignments = {
            item["requirement_family_id"]: item
            for item in classification["requirement_assignments"]
        }
        for assignment in record["capability_assignments"]:
            family_id = assignment["family_id"]
            source_assignment = source_assignments.get(family_id)
            family = catalog.get(family_id)
            _require(
                source_assignment is not None, f"capability assignment is not B2-bound: {family_id}"
            )
            _require(family is not None, f"capability family is not in B2 catalog: {family_id}")
            _require(
                assignment["classification_identity_sha256"]
                == classification["classification_identity"]["digest_hex"],
                f"capability assignment classification digest mismatch: {family_id}",
            )
            _require(
                assignment["review_status"] == classification["review_status"],
                f"capability assignment review status mismatch: {family_id}",
            )
            _require(
                assignment["evidence_basis"] == source_assignment["evidence_basis"],
                f"capability assignment evidence basis mismatch: {family_id}",
            )
            _require(
                assignment["assignment_evidence_sha256"] == _sha256(_canonical(source_assignment)),
                f"capability assignment evidence digest mismatch: {family_id}",
            )
            _require(
                assignment["source_evidence_digest"]
                == classification["source_evidence_digest"]["digest_hex"],
                f"capability assignment source digest mismatch: {family_id}",
            )
            _require(
                assignment["family_boundary_sha256"]
                == _sha256(family["precise_semantic_definition"].encode("utf-8")),
                f"capability assignment family boundary mismatch: {family_id}",
            )
            key = (osi, family_id)
            _require(key not in seen_edges, f"duplicate capability assignment: {key}")
            seen_edges.add(key)
            counts = assignment_counts.setdefault(
                family_id,
                {"edge_count": 0, "oracle_identity_count": 0, "deck_row_count": 0},
            )
            counts["edge_count"] += len(record["deck_row_ids"])
            counts["oracle_identity_count"] += 1
            counts["deck_row_count"] += len(record["deck_row_ids"])
    _require(seen_rows == row_ids, "capability census row coverage mismatch")
    for family_record in capability["families"]:
        family_id = family_record["family_id"]
        family = catalog.get(family_id)
        _require(
            family is not None, f"capability family metadata is not in B2 catalog: {family_id}"
        )
        counts = assignment_counts.get(
            family_id, {"edge_count": 0, "oracle_identity_count": 0, "deck_row_count": 0}
        )
        expected = {
            "family_id": family_id,
            "canonical_name": family["canonical_name"],
            "status": family["status"],
            "terminal_assignable": family["terminal_assignable"],
            "precise_semantic_definition_sha256": _sha256(
                family["precise_semantic_definition"].encode("utf-8")
            ),
            "semantic_owner_roles": _family_owner_roles(family),
            "semantic_owner_status": "SCOPE_ROLE_REQUIRED",
            **counts,
        }
        _require(
            all(family_record.get(key) == value for key, value in expected.items()),
            f"capability family metadata is not independently B2-bound: {family_id}",
        )
    _require(
        capability["record_counts"]["selected_deck_rows"] == 144,
        "capability census row count drift",
    )
    _require(
        capability["record_counts"]["selected_oracle_identities"] == 140,
        "capability census OSI count drift",
    )
    _require(
        capability["record_counts"]["capability_families"] == 125,
        "capability census family count drift",
    )
    _require(
        capability["record_counts"]["capability_assignment_edges"] == 628,
        "capability census edge count drift",
    )


def _validate_persisted_closure_root_evidence(
    root: Path, capability: dict[str, Any], closure: dict[str, Any]
) -> None:
    expected_by_family = {
        family_id: _source_bound_root_refs(root, family_id, capability, closure)
        for family_id in closure["direct_roots"]
    }
    _require(
        set(expected_by_family) == set(closure["direct_roots"]),
        "persisted closure root evidence set is incomplete",
    )


def _validate_selected_scope_inputs(
    root: Path,
    archive_root: Path,
    census: dict[str, dict[str, Any]],
) -> None:
    lock_path = root / "sources/m2_5/scope/exact_two_deck_scope_lock.v1.json"
    lock = load_lock(lock_path)
    lock_sha = _sha256(lock_path.read_bytes())
    validate_lock_document(lock)
    verify_pinned_archive(lock, archive_root)
    selected_osis = {
        row["oracle_semantic_identity"] for deck in lock["decks"] for row in deck["cards"]
    }
    expected_pair = {
        "lock_path": "sources/m2_5/scope/exact_two_deck_scope_lock.v1.json",
        "lock_sha256": lock_sha,
        "deck_ids": [deck["deck_id"] for deck in lock["decks"]],
        "deck_names": [deck["deck_name"] for deck in lock["decks"]],
    }
    capability = census["capability"]
    _require(capability["selected_pair"] == expected_pair, "selected-pair lock binding mismatch")
    _require(
        {deck["deck_name"] for deck in capability["selected_decks"]}
        == {"Token Triumph", "Grave Danger"},
        "selected capability census pair mismatch",
    )
    _require(
        {record["oracle_semantic_identity"] for record in capability["records"]} == selected_osis,
        "selected capability census Oracle identity set mismatch",
    )
    for artifact in census.values():
        _require(
            artifact["source_package_sha256"] == EXPECTED_SOURCE_PACKAGE_SHA256,
            f"source package drift in {artifact['artifact_kind']}",
        )
        _require(
            artifact["content_sha256"] == compute_artifact_content_sha256(artifact),
            f"content digest drift in {artifact['artifact_kind']}",
        )
    closure = census["recursive_capability_closure"]
    _require(
        closure["schema"] == "manafold.m2.5.selected-pair-recursive-capability-closure.v2",
        "closure schema drift",
    )
    _require(closure["status"] == "BLOCKED", "closure status is not the accepted blocked value")
    roots = sorted(family["family_id"] for family in capability["families"])
    _require(closure["direct_roots"] == roots, "closure direct roots drift")
    _require(
        closure["dependency_edges"] == [], "accepted closure unexpectedly contains dependency edges"
    )
    _require(
        closure["terminal_leaves"] == [], "accepted closure unexpectedly contains terminal leaves"
    )
    _require(
        len(closure["unresolved_scope_obligations"]) == len(roots),
        "accepted closure unresolved obligation count drift",
    )
    _require(
        closure["dependency_evidence_bindings"] == _dependency_source_bindings(root),
        "accepted closure dependency source bindings drift",
    )
    _require(
        closure["preserved_artifact_digests"] == _preserved_artifacts(root),
        "accepted closure preserved artifact bindings drift",
    )
    _validate_capability_census_against_b2(root, capability)
    _validate_persisted_closure_root_evidence(root, capability, closure)


def build_dependency_evidence_artifact(
    root: Path = ROOT, archive_root: Path | None = None
) -> dict[str, Any]:
    configured = archive_root or Path(os.environ[ARCHIVE_ENV_VAR])
    census = load_census_artifacts(root)
    _validate_selected_scope_inputs(root, configured, census)
    capability = census["capability"]
    closure = census["recursive_capability_closure"]
    owners = _owner_map(capability)
    roots = sorted(family["family_id"] for family in capability["families"])
    records = []
    for family_id in roots:
        refs = _source_bound_root_refs(root, family_id, capability, closure)
        records.append(
            {
                "capability_family_id": family_id,
                "semantic_owner_roles": owners[family_id],
                "disposition": "UNRESOLVED_DEPENDENCY_EVIDENCE",
                "dependencies": [],
                "terminal_evidence": [],
                "unresolved": {
                    "reason_code": "NO_ACCEPTED_DEPENDENCY_OR_TERMINAL_EVIDENCE",
                    "future_owner": owners[family_id][0],
                    "rationale": (
                        "Accepted B2 classification/catalog/closure material and the "
                        "production capability registry do not provide an accepted "
                        "reusable dependency edge or terminal-leaf contract."
                    ),
                    "evidence_refs": refs,
                },
            }
        )
    unresolved = [
        {
            "obligation_id": f"dependency-evidence:{record['capability_family_id']}",
            "reason_code": record["unresolved"]["reason_code"],
            "subject": record["capability_family_id"],
            "future_owner": record["unresolved"]["future_owner"],
            "evidence_refs": record["unresolved"]["evidence_refs"],
        }
        for record in records
    ]
    artifact = {
        "schema": SCHEMA_ID,
        "artifact_kind": "capability_dependency_evidence",
        "task_id": TASK_ID,
        "status": "PASS",
        "evidence_contract_status": "PASS",
        "selected_pair": capability["selected_pair"],
        "source_package_sha256": EXPECTED_SOURCE_PACKAGE_SHA256,
        "input_bindings": _input_bindings(root, census),
        "preserved_artifact_digests": _preserved_artifacts(root),
        "direct_roots": roots,
        "capabilities": records,
        "dependency_edges": [],
        "terminal_leaves": [],
        "unresolved_scope_obligations": unresolved,
        "summary": {
            "direct_capability_roots": len(roots),
            "dependency_set_evidenced": 0,
            "terminal_leaf_evidenced": 0,
            "unresolved_dependency_evidence": len(records),
            "total_dependency_edges": 0,
            "unique_dependency_children": 0,
            "transitive_only_capability_references": 0,
            "missing_dependency_capabilities": 0,
        },
        "semantic_owner_map": owners,
        "recursive_capability_closure": "BLOCKED",
        "content_sha256": "",
    }
    artifact["content_sha256"] = compute_artifact_content_sha256(artifact)
    return artifact


def _validate_schema(artifact: dict[str, Any], root: Path) -> None:
    schema = _json(root / SCHEMA_RELATIVE_PATH)
    try:
        jsonschema.Draft202012Validator(schema).validate(artifact)
    except jsonschema.ValidationError as exc:
        raise ScopeDependencyEvidenceError(f"schema: {exc.message}") from exc


def _validate_file_binding(root: Path, binding: dict[str, Any]) -> None:
    path = binding["path"]
    file_path = root / path
    _require(file_path.is_file(), f"input binding file is missing: {path}")
    _require(_sha256(file_path.read_bytes()) == binding["raw_sha256"], f"input digest: {path}")


def validate_dependency_evidence_artifact(
    artifact: dict[str, Any], root: Path = ROOT, archive_root: Path | None = None
) -> None:
    _validate_schema(artifact, root)
    _require(
        artifact["content_sha256"] == compute_artifact_content_sha256(artifact),
        "content digest mismatch",
    )
    _require(
        artifact["source_package_sha256"] == EXPECTED_SOURCE_PACKAGE_SHA256,
        "source package binding mismatch",
    )
    configured = archive_root or Path(os.environ[ARCHIVE_ENV_VAR])
    census = load_census_artifacts(root)
    _validate_selected_scope_inputs(root, configured, census)
    capability = census["capability"]
    closure = census["recursive_capability_closure"]
    _require(
        artifact["selected_pair"] == capability["selected_pair"],
        "selected-pair binding mismatch",
    )
    expected_bindings = _input_bindings(root, census)
    _require(artifact["input_bindings"] == expected_bindings, "input binding mismatch")
    _require(
        artifact["preserved_artifact_digests"] == _preserved_artifacts(root),
        "preserved artifact binding mismatch",
    )
    roots = sorted(family["family_id"] for family in capability["families"])
    _require(artifact["direct_roots"] == roots, "direct capability root set mismatch")
    records = artifact["capabilities"]
    _require(
        [record["capability_family_id"] for record in records] == roots,
        "capability evidence root records are missing, extra, or noncanonical",
    )
    owners = _owner_map(capability)
    accepted_dependency_evidence_available = False
    _require(
        not artifact["dependency_edges"],
        "accepted dependency evidence is unavailable",
    )
    _require(
        not artifact["terminal_leaves"],
        "accepted terminal evidence is unavailable",
    )
    dependency_edges = validate_dependency_edges(
        artifact["dependency_edges"],
        {family["family_id"] for family in _json(root / B2_FILES["family_catalog"][0])["families"]},
        root,
    )
    _require(artifact["dependency_edges"] == dependency_edges, "dependency edge order mismatch")
    for record in records:
        family_id = record["capability_family_id"]
        _require(
            record["semantic_owner_roles"] == owners[family_id],
            f"semantic owner mismatch: {family_id}",
        )
        _require(
            record["disposition"] in DISPOSITIONS,
            f"unknown evidence disposition: {family_id}",
        )
        if record["disposition"] == "UNRESOLVED_DEPENDENCY_EVIDENCE":
            refs = record["unresolved"]["evidence_refs"]
            expected_refs = _source_bound_root_refs(root, family_id, capability, closure)
            _require(refs == expected_refs, f"root evidence is not source-bound: {family_id}")
            _require(not record["dependencies"], f"unresolved root has dependencies: {family_id}")
            _require(
                not record["terminal_evidence"],
                f"unresolved root has terminal evidence: {family_id}",
            )
            _require(
                record["unresolved"]["reason_code"]
                == "NO_ACCEPTED_DEPENDENCY_OR_TERMINAL_EVIDENCE",
                f"unresolved reason mismatch: {family_id}",
            )
        elif record["disposition"] == "TERMINAL_LEAF_EVIDENCED":
            _require(
                accepted_dependency_evidence_available,
                f"accepted terminal evidence is unavailable: {family_id}",
            )
            _require(record["terminal_evidence"], f"terminal evidence missing: {family_id}")
            _require(not record["dependencies"], f"terminal root has dependencies: {family_id}")
        else:
            _require(
                accepted_dependency_evidence_available,
                f"accepted dependency evidence is unavailable: {family_id}",
            )
            _require(record["dependencies"], f"dependency set missing: {family_id}")
            _require(
                not record["terminal_evidence"],
                f"dependency root has terminal evidence: {family_id}",
            )
    expected_terminal = sorted(
        record["capability_family_id"]
        for record in records
        if record["disposition"] == "TERMINAL_LEAF_EVIDENCED"
    )
    expected_unresolved = [
        {
            "obligation_id": f"dependency-evidence:{record['capability_family_id']}",
            "reason_code": record["unresolved"]["reason_code"],
            "subject": record["capability_family_id"],
            "future_owner": record["unresolved"]["future_owner"],
            "evidence_refs": record["unresolved"]["evidence_refs"],
        }
        for record in records
        if record["disposition"] == "UNRESOLVED_DEPENDENCY_EVIDENCE"
    ]
    _require(artifact["terminal_leaves"] == expected_terminal, "terminal leaf set mismatch")
    _require(
        artifact["unresolved_scope_obligations"] == expected_unresolved,
        "unresolved evidence obligations mismatch",
    )
    summary = artifact["summary"]
    _require(
        summary
        == {
            "direct_capability_roots": len(roots),
            "dependency_set_evidenced": sum(
                record["disposition"] == "DEPENDENCY_SET_EVIDENCED" for record in records
            ),
            "terminal_leaf_evidenced": len(expected_terminal),
            "unresolved_dependency_evidence": len(expected_unresolved),
            "total_dependency_edges": len(dependency_edges),
            "unique_dependency_children": len(
                {edge["child_family_id"] for edge in dependency_edges}
            ),
            "transitive_only_capability_references": len(
                {
                    edge["child_family_id"]
                    for edge in dependency_edges
                    if edge["child_family_id"] not in set(roots)
                }
            ),
            "missing_dependency_capabilities": 0,
        },
        "evidence summary mismatch",
    )
    _require(artifact["recursive_capability_closure"] == "BLOCKED", "closure status changed")


def load_dependency_evidence_artifact(root: Path = ROOT) -> dict[str, Any]:
    return _json(root / ARTIFACT_RELATIVE_PATH)


def _write_artifact(artifact: dict[str, Any], root: Path) -> None:
    path = root / ARTIFACT_RELATIVE_PATH
    path.write_text(json.dumps(artifact, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--archive-root", type=Path)
    args = parser.parse_args()
    try:
        archive_root = args.archive_root or Path(os.environ[ARCHIVE_ENV_VAR])
        if args.write:
            _write_artifact(build_dependency_evidence_artifact(ROOT, archive_root), ROOT)
        validate_dependency_evidence_artifact(
            load_dependency_evidence_artifact(ROOT), ROOT, archive_root
        )
    except (OSError, KeyError, ValueError, json.JSONDecodeError) as exc:
        print(f"BLOCKED: {exc}")
        return 2
    print("PASS: selected-pair capability dependency evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
