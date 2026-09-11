#!/usr/bin/env python3
"""Independently verify the persisted, non-current B2 closure-v2 artifact."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from b2_closure_v2_support import (
    B2_CLOSURE_V2_PATH,
    B2_CLOSURE_V2_SCHEMA,
    FROZEN_SNAPSHOT_CONSTANTS,
    SOURCE_PACKAGE_SHA256,
    recompute_snapshot_constants,
    verify_v1_artifact_bindings,
)
from mtgml.b2_closure_contract import B2ClosureArtifactBindingV1


class B2ClosureV2VerificationError(ValueError):
    pass


def _load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise B2ClosureV2VerificationError(f"cannot read JSON artifact: {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise B2ClosureV2VerificationError("closure v2 must be a JSON object")
    return value


def _schema_validate(repo_root: Path, value: dict[str, Any]) -> None:
    try:
        import jsonschema  # type: ignore[import-untyped]
    except ImportError as exc:  # pragma: no cover - locked environments install it
        raise B2ClosureV2VerificationError("jsonschema is unavailable") from exc
    schema = _load_json(repo_root / "schemas/b2-classification-closure.v2.schema.json")
    try:
        jsonschema.Draft202012Validator(schema).validate(value)
    except jsonschema.ValidationError as exc:
        raise B2ClosureV2VerificationError(f"v2 schema validation failed: {exc.message}") from exc


def verify_closure_v2(repo_root: Path, closure_path: Path) -> dict[str, Any]:
    expected_path = repo_root / B2_CLOSURE_V2_PATH
    if closure_path.resolve() != expected_path.resolve():
        raise B2ClosureV2VerificationError("closure path is not the admitted v2 path")
    value = _load_json(closure_path)
    _schema_validate(repo_root, value)
    if value.get("schema") != B2_CLOSURE_V2_SCHEMA:
        raise B2ClosureV2VerificationError("closure schema identity mismatch")
    if value.get("source_package_sha256") != SOURCE_PACKAGE_SHA256:
        raise B2ClosureV2VerificationError("source package mismatch")
    bindings = value.get("artifact_bindings")
    if not isinstance(bindings, list):
        raise B2ClosureV2VerificationError("artifact_bindings is not an array")
    try:
        typed_bindings = [B2ClosureArtifactBindingV1.from_wire(item) for item in bindings]
        actual_bindings = verify_v1_artifact_bindings(repo_root)
        expected_bindings = [B2ClosureArtifactBindingV1.from_wire(item) for item in actual_bindings]
        if tuple(typed_bindings) != tuple(expected_bindings):
            raise B2ClosureV2VerificationError("artifact bindings do not match actual v1 bytes")
    except Exception as exc:
        if isinstance(exc, B2ClosureV2VerificationError):
            raise
        raise B2ClosureV2VerificationError(str(exc)) from exc
    constants = value.get("snapshot_constants")
    actual_constants = recompute_snapshot_constants(repo_root)
    if constants != FROZEN_SNAPSHOT_CONSTANTS or constants != actual_constants:
        raise B2ClosureV2VerificationError("snapshot constants do not match actual v1 artifacts")
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    verify_closure_v2(args.repo_root, args.repo_root / B2_CLOSURE_V2_PATH)
    print("B2_CLOSURE_V2_VERIFIER = PASS (v2 remains NON_CURRENT)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
