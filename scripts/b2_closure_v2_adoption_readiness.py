#!/usr/bin/env python3
"""Build and verify deterministic, non-current ADR 0046 Slice-5 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import replace
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from authority_source_resolver import ResolutionError
from b2_closure_downstream_readiness import (
    B2_DOWNSTREAM_OWNERSHIP,
    B2DownstreamReadinessStatus,
    prepare_b2_downstream_readiness,
)
from b2_closure_source_resolver import (
    B2ClosureResolutionMode,
    B2ClosureSourceBinding,
    binding_for_mode,
    resolve_b2_closure,
    resolve_b2_closure_binding,
)
from b2_closure_v2_support import (
    B2_CLOSURE_V2_PATH,
    FROZEN_SNAPSHOT_CONSTANTS,
    SOURCE_PACKAGE_SHA256,
    recompute_snapshot_constants,
    verify_v1_artifact_bindings,
)
from check_m2_5_b2_closure_v2 import (
    verify_closure_v2,
)
from mtgml.b2_closure_contract import (
    B2_CLOSURE_V1_PATH,
    B2_CLOSURE_V1_RAW_SHA256,
)

B2_ADOPTION_READINESS_SCHEMA = "manafold.m2.5.b2.closure-v2-adoption-readiness.v1"
B2_ADOPTION_READINESS_PATH = (
    "sources/m2_5/closures/B2/verification/b2_closure_v2_adoption_readiness.v1.json"
)
B2_CLOSURE_V2_RAW_SHA256 = "43a0613be367159ffd224d6282deb4e263b2147b861acb90a26388d56729f748"
BASELINE_COMMIT = "85c470a869e12e84edbf13328b052bda631d35db"
SOURCE_ARCHIVE_ENV = "MANAFOLD_SOURCE_ARCHIVE"
ARCHIVE_RELATIVE_PATH = "m2_5/Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip"

HISTORICAL_B2_PATHS = (
    "sources/m2_5/closures/B2/B2_DESIGN_SPEC.md",
    "sources/m2_5/closures/B2/classification_closure.v1.json",
    "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
    "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
    "sources/m2_5/closures/B2/deck_row_classification_refs.v1.csv",
    "sources/m2_5/closures/B2/CLASSIFICATION_REPORT.md",
    "sources/m2_5/closures/B2/verification/b2_negative_test_matrix.v1.json",
    "sources/m2_5/closures/B2/verification/b2_verification_summary.v1.json",
)
HISTORICAL_MUTATION_PREFIXES = (
    "sources/m2_5/closures/B1/",
    "sources/m2_5/closures/C/",
    "sources/m2_5/authorities/",
)
CURRENT_ROOT_PATH = "sources/m2_5/closures/B2/current_root.json"
SLICE2_NEGATIVE_MATRIX_PATH = "conformance/fixtures/authority/b2_closure_v2_negative_matrix.v1.json"


class B2AdoptionReadinessError(ValueError):
    """A fail-closed Slice-5 evidence rejection."""


class B2AdoptionReadinessBlocked(B2AdoptionReadinessError):
    """A mandatory external evidence source is unavailable."""


def sha256_bytes(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def sha256_file(path: Path) -> str:
    try:
        return sha256_bytes(path.read_bytes())
    except OSError as exc:
        raise B2AdoptionReadinessError(f"cannot read {path}: {exc}") from exc


def _path(repo_root: Path, relative: str) -> Path:
    return repo_root / Path(*relative.split("/"))


def _run_git(repo_root: Path, *args: str) -> str:
    try:
        result = subprocess.run(
            ["git", *args],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        raise B2AdoptionReadinessError(f"git evidence command failed: {args!r}") from exc
    return result.stdout.strip()


def _baseline_bytes(repo_root: Path, relative: str) -> bytes:
    try:
        result = subprocess.run(
            ["git", "show", f"{BASELINE_COMMIT}:{relative}"],
            cwd=repo_root,
            check=True,
            capture_output=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        raise B2AdoptionReadinessError(
            f"baseline bytes unavailable for {relative}: {BASELINE_COMMIT}"
        ) from exc
    return result.stdout


def run_historical_v1_verification(repo_root: Path) -> str:
    """Run the existing source-grounded B2 v1 checker without persisting local paths."""

    archive_root = os.environ.get(SOURCE_ARCHIVE_ENV)
    if not archive_root:
        raise B2AdoptionReadinessBlocked(
            f"{SOURCE_ARCHIVE_ENV} is required for historical B2 verification"
        )
    archive = Path(archive_root) / Path(*ARCHIVE_RELATIVE_PATH.split("/"))
    expected_archive_sha = "99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90"
    if not archive.is_file() or sha256_file(archive) != expected_archive_sha:
        raise B2AdoptionReadinessBlocked(
            "the pinned REV3 source archive is unavailable or mismatched"
        )
    environment = os.environ.copy()
    environment[SOURCE_ARCHIVE_ENV] = archive_root
    result = subprocess.run(
        [sys.executable, str(repo_root / "scripts/check_m2_5_b2_classifications.py")],
        cwd=repo_root,
        env=environment,
        capture_output=True,
        text=True,
    )
    if result.returncode == 2:
        raise B2AdoptionReadinessBlocked(result.stdout.strip() or result.stderr.strip())
    if result.returncode != 0:
        raise B2AdoptionReadinessError(result.stdout.strip() or result.stderr.strip())
    return "PASS"


def _binding_dict(binding: B2ClosureSourceBinding) -> dict[str, str]:
    return {
        "artifact_role": binding.artifact_role,
        "closure_version": binding.closure_version,
        "repository_relative_path": binding.repository_relative_path,
        "schema_identifier": binding.schema_identifier,
        "raw_sha256": binding.raw_sha256,
        "source_package_sha256": binding.source_package_sha256,
    }


def _historical_parity(repo_root: Path) -> dict[str, object]:
    records: list[dict[str, str]] = []
    for relative in HISTORICAL_B2_PATHS:
        current = sha256_file(_path(repo_root, relative))
        baseline = sha256_bytes(_baseline_bytes(repo_root, relative))
        if current != baseline:
            raise B2AdoptionReadinessError(f"historical B2 byte drift: {relative}")
        records.append(
            {
                "path": relative,
                "baseline_raw_sha256": baseline,
                "current_raw_sha256": current,
            }
        )
    v2_current = sha256_file(_path(repo_root, B2_CLOSURE_V2_PATH))
    v2_baseline = sha256_bytes(_baseline_bytes(repo_root, B2_CLOSURE_V2_PATH))
    if v2_current != B2_CLOSURE_V2_RAW_SHA256 or v2_current != v2_baseline:
        raise B2AdoptionReadinessError("closure-v2 bytes changed from the Slice-2 baseline")
    return {
        "status": "PASS",
        "b2_v1_artifacts": records,
        "b2_v2_baseline_raw_sha256": v2_baseline,
        "b2_v2_current_raw_sha256": v2_current,
    }


def _historical_nonmutation(repo_root: Path) -> dict[str, object]:
    changed = _run_git(repo_root, "diff", "--name-only", BASELINE_COMMIT, "HEAD").splitlines()
    forbidden = [
        path
        for path in changed
        if any(path.startswith(prefix) for prefix in HISTORICAL_MUTATION_PREFIXES)
    ]
    if forbidden:
        raise B2AdoptionReadinessError(f"historical authority paths changed: {forbidden}")
    return {
        "status": "PASS",
        "b1": "NO",
        "c": "NO",
        "authority": "NO",
        "identity_rewrite": "NO",
    }


def _role_version_matrix(repo_root: Path) -> str:
    v1 = binding_for_mode(B2ClosureResolutionMode.HISTORICAL_V1)
    v2 = binding_for_mode(B2ClosureResolutionMode.CANDIDATE_V2)
    invalid = (
        replace(v1, artifact_role="b2_closure_v2"),
        replace(v2, artifact_role="b2_closure"),
        replace(v1, artifact_role="b2_closure_v1"),
        replace(v2, artifact_role="b2_closure_v3"),
    )
    for binding in invalid:
        try:
            resolve_b2_closure_binding(repo_root, binding)
        except ResolutionError:
            continue
        raise B2AdoptionReadinessError("an invalid B2 closure role/version pair was accepted")
    return "PASS"


def _copy_resolution_repo(repo_root: Path) -> tuple[tempfile.TemporaryDirectory[str], Path]:
    directory = tempfile.TemporaryDirectory()
    target = Path(directory.name)
    shutil.copytree(repo_root / "sources", target / "sources")
    shutil.copytree(repo_root / "schemas", target / "schemas")
    return directory, target


def _no_fallback(repo_root: Path) -> str:
    directory, copied = _copy_resolution_repo(repo_root)
    try:
        v2_path = _path(copied, B2_CLOSURE_V2_PATH)
        v2_path.write_bytes(v2_path.read_bytes() + b"\n")
        try:
            resolve_b2_closure(copied, B2ClosureResolutionMode.CANDIDATE_V2)
        except ResolutionError:
            pass
        else:
            raise B2AdoptionReadinessError("invalid v2 resolution was accepted")
        resolve_b2_closure(copied, B2ClosureResolutionMode.HISTORICAL_V1)
    finally:
        directory.cleanup()

    directory, copied = _copy_resolution_repo(repo_root)
    try:
        v1_path = _path(copied, B2_CLOSURE_V1_PATH)
        v1_path.write_bytes(v1_path.read_bytes() + b"\n")
        try:
            resolve_b2_closure(copied, B2ClosureResolutionMode.HISTORICAL_V1)
        except ResolutionError:
            pass
        else:
            raise B2AdoptionReadinessError("invalid v1 historical resolution was accepted")
        resolve_b2_closure(copied, B2ClosureResolutionMode.CANDIDATE_V2)
    finally:
        directory.cleanup()
    return "PASS"


def _downstream_readiness(repo_root: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for consumer, ownership in B2_DOWNSTREAM_OWNERSHIP.items():
        key = consumer.value
        if ownership.status is B2DownstreamReadinessStatus.NOT_APPLICABLE:
            result[key] = "NOT_APPLICABLE"
            continue
        readiness = prepare_b2_downstream_readiness(repo_root, consumer)
        if not readiness.v2_ready or readiness.v2_current or readiness.production_record_created:
            raise B2AdoptionReadinessError(f"downstream readiness is not non-current: {key}")
        result[key] = "PASS"
    return result


def _slice2_negative_matrix(repo_root: Path) -> str:
    path = _path(repo_root, SLICE2_NEGATIVE_MATRIX_PATH)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise B2AdoptionReadinessError(f"Slice-2 negative matrix is unreadable: {path}") from exc
    cases = value.get("cases") if isinstance(value, dict) else None
    if (
        not isinstance(cases, list)
        or not cases
        or any(not isinstance(case, dict) or case.get("expected") != "reject" for case in cases)
    ):
        raise B2AdoptionReadinessError("Slice-2 negative matrix is not closed and rejecting")
    return "PASS"


def build_adoption_readiness(repo_root: Path) -> dict[str, object]:
    historical_status = run_historical_v1_verification(repo_root)
    v2_path = _path(repo_root, B2_CLOSURE_V2_PATH)
    verify_closure_v2(repo_root, v2_path)
    v1_resolution = resolve_b2_closure(repo_root, B2ClosureResolutionMode.HISTORICAL_V1)
    v2_resolution = resolve_b2_closure(repo_root, B2ClosureResolutionMode.CANDIDATE_V2)
    semantic_bindings = verify_v1_artifact_bindings(repo_root)
    counts = recompute_snapshot_constants(repo_root)
    if counts != FROZEN_SNAPSHOT_CONSTANTS:
        raise B2AdoptionReadinessError("snapshot constants do not match the frozen B2 values")
    if v1_resolution.binding.raw_sha256 != B2_CLOSURE_V1_RAW_SHA256:
        raise B2AdoptionReadinessError("historical v1 closure digest changed")
    if v2_resolution.raw_sha256 != B2_CLOSURE_V2_RAW_SHA256:
        raise B2AdoptionReadinessError("candidate v2 closure digest changed")
    if _path(repo_root, CURRENT_ROOT_PATH).exists():
        raise B2AdoptionReadinessError("a current-root admission artifact already exists")

    role_matrix = _role_version_matrix(repo_root)
    fallback = _no_fallback(repo_root)
    downstream = _downstream_readiness(repo_root)
    historical_nonmutation = _historical_nonmutation(repo_root)
    historical_parity = _historical_parity(repo_root)
    return {
        "schema": B2_ADOPTION_READINESS_SCHEMA,
        "currentness_state": "v2_ready_not_adopted",
        "source_package_sha256": SOURCE_PACKAGE_SHA256,
        "historical_v1_verification": historical_status,
        "v2_verification": "PASS",
        "historical_v1_binding": _binding_dict(v1_resolution.binding),
        "candidate_v2_binding": _binding_dict(v2_resolution.binding),
        "semantic_artifact_bindings": semantic_bindings,
        "snapshot_constants": counts,
        "historical_parity": historical_parity,
        "historical_nonmutation": historical_nonmutation,
        "source_role_parity": {
            "historical_role": "b2_closure",
            "candidate_role": "b2_closure_v2",
            "b2_closure_v1_alias_introduced": False,
            "role_version_matrix": role_matrix,
        },
        "no_fallback": fallback,
        "downstream_readiness": downstream,
        "adoption_gate_matrix": {
            "v2_schema_dto_closure": "PASS",
            "v2_positive_verifier": "PASS",
            "v2_negative_matrix": _slice2_negative_matrix(repo_root),
            "b2_v1_semantic_byte_parity": historical_parity["status"],
            "exact_artifact_role_bindings": "PASS",
            "exact_artifact_path_bindings": "PASS",
            "exact_artifact_schema_bindings": "PASS",
            "exact_artifact_raw_sha_bindings": "PASS",
            "source_package_binding": "PASS",
            "snapshot_count_consistency": "PASS",
            "historical_v1_verification": historical_status,
            "versioned_source_role_admission": role_matrix,
            "authority_source_resolver_v2_readiness": downstream["authority_source_resolution"],
            "authority_validator_v2_readiness": downstream["authority_validator"],
            "b1_current_evidence_root_readiness": downstream["b1_current_evidence_root"],
            "c_current_source_root_readiness": downstream["c_current_source_root"],
            "relation_application_readiness": downstream["relation_application"],
            "context_application_readiness": downstream["context_application"],
            "host_binding_readiness": downstream["host_binding"],
            "review_acceptance_readiness": downstream["review_acceptance"],
            "no_aliasing": role_matrix,
            "no_fallback": fallback,
            "no_historical_mutation": historical_nonmutation["status"],
            "no_identity_rewrite": historical_nonmutation["identity_rewrite"],
            "local_integration": "EXTERNAL_EVIDENCE_REQUIRED",
            "rust_gates": "EXTERNAL_EVIDENCE_REQUIRED",
            "python_gates": "EXTERNAL_EVIDENCE_REQUIRED",
            "schema_gates": "EXTERNAL_EVIDENCE_REQUIRED",
            "conformance_gates": "EXTERNAL_EVIDENCE_REQUIRED",
            "maintainer_gates": "EXTERNAL_EVIDENCE_REQUIRED",
            "hosted_exact_head_ci": "EXTERNAL_EVIDENCE_REQUIRED",
            "independent_review": "EXTERNAL_EVIDENCE_REQUIRED",
            "explicit_maintainer_adoption": "NOT_AUTHORIZED",
        },
        "current_root": {
            "admission_path": CURRENT_ROOT_PATH,
            "created": False,
            "adopted_v2": False,
        },
        "evidence_dag": {
            "status": "PASS",
            "edges": [
                "immutable_b2_v1_semantic_artifacts->classification_closure_v2",
                "classification_closure_v2->slice5_readiness_evidence",
            ],
        },
    }


def render_adoption_readiness(value: dict[str, object]) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def _schema_validate(repo_root: Path, value: dict[str, object]) -> None:
    try:
        import jsonschema  # type: ignore[import-untyped]

        schema = json.loads(
            (_path(repo_root, "schemas/b2-closure-v2-adoption-readiness.v1.schema.json")).read_text(
                encoding="utf-8"
            )
        )
        jsonschema.Draft202012Validator(schema).validate(value)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        raise B2AdoptionReadinessError(f"readiness schema validation failed: {exc}") from exc
    except Exception as exc:
        raise B2AdoptionReadinessError(f"readiness schema validation failed: {exc}") from exc


def verify_adoption_readiness(repo_root: Path, evidence_path: Path) -> dict[str, object]:
    expected_path = _path(repo_root, B2_ADOPTION_READINESS_PATH)
    if evidence_path.resolve() != expected_path.resolve():
        raise B2AdoptionReadinessError("readiness evidence path is not the admitted path")
    try:
        value = json.loads(evidence_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise B2AdoptionReadinessError(f"cannot read readiness evidence: {exc}") from exc
    if not isinstance(value, dict):
        raise B2AdoptionReadinessError("readiness evidence must be an object")
    return verify_adoption_readiness_value(repo_root, value)


def verify_adoption_readiness_value(repo_root: Path, value: dict[str, object]) -> dict[str, object]:
    """Verify one decoded evidence value against freshly recomputed repository facts."""

    _schema_validate(repo_root, value)
    recomputed = build_adoption_readiness(repo_root)
    if value != recomputed:
        raise B2AdoptionReadinessError(
            "readiness evidence differs from recomputed repository facts"
        )
    return value


def write_adoption_readiness(repo_root: Path, value: dict[str, object]) -> Path:
    path = _path(repo_root, B2_ADOPTION_READINESS_PATH)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(render_adoption_readiness(value))
    return path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=ROOT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    try:
        value = build_adoption_readiness(args.repo_root)
        path = _path(args.repo_root, B2_ADOPTION_READINESS_PATH)
        if args.check:
            verify_adoption_readiness(args.repo_root, path)
            print("B2_CLOSURE_V2_ADOPTION_READINESS = PASS (v2 remains NON_CURRENT)")
        else:
            write_adoption_readiness(args.repo_root, value)
            print(f"B2_CLOSURE_V2_ADOPTION_READINESS_WRITTEN = {path}")
        return 0
    except B2AdoptionReadinessBlocked as exc:
        print(f"BLOCKED: {exc}")
        return 2
    except B2AdoptionReadinessError as exc:
        print(f"FAIL: {exc}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
