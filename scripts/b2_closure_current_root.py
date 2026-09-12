"""Sole current B2 closure-root loading and current-construction resolution."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from b2_closure_source_resolver import (
    B2_CLOSURE_V2_RAW_SHA256,
    B2ClosureResolutionMode,
    B2ClosureSourceBinding,
    B2ClosureSourceResolution,
    resolve_b2_closure_binding,
)
from mtgml.b2_closure_contract import (
    B2_CLOSURE_CURRENT_ROOT_SCHEMA,
    B2_CLOSURE_SOURCE_PACKAGE_SHA256,
    B2_CLOSURE_V2_PATH,
    B2_CLOSURE_V2_SCHEMA,
    B2ClosureContractError,
    B2ClosureCurrentRootV1,
)

B2_CURRENT_ROOT_PATH = "sources/m2_5/closures/B2/current_root.json"


class B2CurrentRootError(ValueError):
    """A fail-closed current-root admission failure."""

    def __init__(self, code: str, message: str) -> None:
        self.code = code
        self.message = message
        super().__init__(f"[{code}] {message}")


@dataclass(frozen=True)
class B2CurrentClosureResolution:
    """The sole admitted current root and its freshly verified v2 closure."""

    root: B2ClosureCurrentRootV1
    resolution: B2ClosureSourceResolution

    @property
    def current(self) -> bool:
        return True


def _root_path(repo_root: Path) -> Path:
    return repo_root / Path(*B2_CURRENT_ROOT_PATH.split("/"))


def _candidate_root_paths(repo_root: Path) -> list[Path]:
    directory = _root_path(repo_root).parent
    return sorted(directory.glob("current_root*.json"))


def load_current_b2_closure_root(repo_root: Path) -> B2ClosureCurrentRootV1:
    """Load exactly one persisted current-root admission and validate it as v2."""

    root_path = _root_path(repo_root)
    if not root_path.is_file():
        raise B2CurrentRootError("CURRENT_ROOT_MISSING", B2_CURRENT_ROOT_PATH)
    if len(_candidate_root_paths(repo_root)) != 1:
        raise B2CurrentRootError(
            "MULTIPLE_CURRENT_ROOTS", "more than one current-root admission file exists"
        )
    try:
        value = json.loads(root_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise B2CurrentRootError("INVALID_ROOT_SHAPE", "current-root JSON is unreadable") from exc
    try:
        root = B2ClosureCurrentRootV1.from_wire(value)
        root.validate(expected_raw_sha256=B2_CLOSURE_V2_RAW_SHA256)
    except B2ClosureContractError as exc:
        raise B2CurrentRootError(exc.code.value, exc.detail or str(exc)) from exc
    if root.schema != B2_CLOSURE_CURRENT_ROOT_SCHEMA:
        raise B2CurrentRootError("CURRENT_ROOT_SCHEMA_MISMATCH", "root schema is not v1")
    if root.artifact_role != "b2_closure_v2" or root.closure_version != "v2":
        raise B2CurrentRootError("CURRENT_ROOT_ROLE_MISMATCH", "adopted root must be v2")
    if root.repository_relative_path != B2_CLOSURE_V2_PATH:
        raise B2CurrentRootError("CURRENT_ROOT_PATH_MISMATCH", "root path is not the v2 path")
    if root.closure_schema_id != B2_CLOSURE_V2_SCHEMA:
        raise B2CurrentRootError("CURRENT_ROOT_SCHEMA_MISMATCH", "closure schema is not v2")
    if root.source_package_sha256 != B2_CLOSURE_SOURCE_PACKAGE_SHA256:
        raise B2CurrentRootError("SOURCE_PACKAGE_MISMATCH", "source package is not admitted")
    return root


def resolve_current_b2_closure(repo_root: Path) -> B2CurrentClosureResolution:
    """Resolve currentness only through the sole persisted v2 root."""

    root = load_current_b2_closure_root(repo_root)
    binding = B2ClosureSourceBinding(
        artifact_role=root.artifact_role,
        closure_version=root.closure_version,
        repository_relative_path=root.repository_relative_path,
        schema_identifier=root.closure_schema_id,
        raw_sha256=root.closure_raw_sha256,
        source_package_sha256=root.source_package_sha256,
    )
    resolution = resolve_b2_closure_binding(repo_root, binding)
    if resolution.mode is not B2ClosureResolutionMode.CANDIDATE_V2 or not resolution.v2_verified:
        raise B2CurrentRootError(
            "CURRENT_CONSUMER_VERSION_UNSUPPORTED",
            "current construction requires verified closure v2",
        )
    return B2CurrentClosureResolution(root=root, resolution=resolution)


__all__ = [
    "B2_CURRENT_ROOT_PATH",
    "B2CurrentClosureResolution",
    "B2CurrentRootError",
    "load_current_b2_closure_root",
    "resolve_current_b2_closure",
]
