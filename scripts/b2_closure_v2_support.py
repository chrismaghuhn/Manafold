"""Shared Slice-2 input reading and deterministic count recomputation."""

from __future__ import annotations

import csv
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from mtgml.b2_closure_contract import (
    B2_CLOSURE_ARTIFACT_BINDINGS,
    B2_CLOSURE_SNAPSHOT_CONSTANTS,
    B2_CLOSURE_SOURCE_PACKAGE_SHA256,
)

SOURCE_PACKAGE_SHA256 = B2_CLOSURE_SOURCE_PACKAGE_SHA256
B2_CLOSURE_V2_PATH = "sources/m2_5/closures/B2/classification_closure.v2.json"
B2_CLOSURE_V2_SCHEMA = "manafold.m2.5.b2.classification-closure.v2"

FROZEN_SNAPSHOT_CONSTANTS = B2_CLOSURE_SNAPSHOT_CONSTANTS

B2_V1_ARTIFACT_BINDINGS = tuple(B2_CLOSURE_ARTIFACT_BINDINGS)

_ARTIFACTS = B2_CLOSURE_ARTIFACT_BINDINGS


class B2ClosureV2InputError(ValueError):
    pass


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise B2ClosureV2InputError(f"expected object: {path}")
    return value


def verify_v1_artifact_bindings(repo_root: Path) -> list[dict[str, str]]:
    result: list[dict[str, str]] = []
    for role in B2_V1_ARTIFACT_BINDINGS:
        relative_path, schema, expected_sha = _ARTIFACTS[role]
        path = repo_root / relative_path
        if not path.is_file():
            raise B2ClosureV2InputError(f"missing v1 semantic artifact: {relative_path}")
        actual_sha = sha256_file(path)
        if actual_sha != expected_sha:
            raise B2ClosureV2InputError(
                f"raw SHA mismatch for {role}: {actual_sha} != {expected_sha}"
            )
        if role == "b2_classifications_v1" or role == "b2_family_catalog_v1":
            value = _read_json(path)
            if value.get("schema") != schema:
                raise B2ClosureV2InputError(f"schema mismatch for {role}")
            if value.get("source_package_sha256") != SOURCE_PACKAGE_SHA256:
                raise B2ClosureV2InputError(f"source package mismatch for {role}")
        result.append(
            {
                "artifact_role": role,
                "repository_relative_path": relative_path,
                "schema_identifier": schema,
                "raw_sha256": actual_sha,
            }
        )
    return result


def recompute_snapshot_constants(repo_root: Path) -> dict[str, int]:
    classifications = _read_json(
        repo_root / "sources/m2_5/closures/B2/card_semantic_classifications.v1.json"
    )
    catalog = _read_json(repo_root / "sources/m2_5/closures/B2/requirement_family_catalog.v1.json")
    projection_path = repo_root / "sources/m2_5/closures/B2/deck_row_classification_refs.v1.csv"
    records = classifications.get("classifications")
    families = catalog.get("families")
    if not isinstance(records, list) or not isinstance(families, list):
        raise B2ClosureV2InputError("B2 semantic artifacts have invalid array shapes")
    identities = [
        record.get("oracle_semantic_identity") for record in records if isinstance(record, dict)
    ]
    if any(not isinstance(identity, str) for identity in identities):
        raise B2ClosureV2InputError("classification identity field is invalid")
    terminal_edges = 0
    for record in records:
        if not isinstance(record, dict) or not isinstance(
            record.get("requirement_assignments"), list
        ):
            raise B2ClosureV2InputError("classification assignment shape is invalid")
        terminal_edges += len(record["requirement_assignments"])
    with projection_path.open(newline="", encoding="utf-8") as handle:
        projection_rows = list(csv.DictReader(handle))
    return {
        "oracle_semantic_identity_count": len(set(identities)),
        "classification_count": len(records),
        "terminal_assignment_edge_count": terminal_edges,
        "deck_row_count": len(projection_rows),
        "projection_row_count": len(projection_rows),
        "historical_family_count": int(catalog.get("legacy_family_count", -1)),
        "catalog_family_count": len(families),
    }


def build_closure_payload(repo_root: Path) -> dict[str, object]:
    bindings = verify_v1_artifact_bindings(repo_root)
    counts = recompute_snapshot_constants(repo_root)
    if counts != FROZEN_SNAPSHOT_CONSTANTS:
        raise B2ClosureV2InputError(f"frozen snapshot mismatch: {counts!r}")
    return {
        "schema": B2_CLOSURE_V2_SCHEMA,
        "source_package_sha256": SOURCE_PACKAGE_SHA256,
        "artifact_bindings": bindings,
        "snapshot_constants": counts,
    }
