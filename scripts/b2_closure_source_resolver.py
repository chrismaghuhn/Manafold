"""Explicit historical-v1 and candidate-v2 B2 closure source resolution.

This module is deliberately separate from the existing Authority source-role
registries.  It resolves an explicitly requested B2 closure binding, verifies
the source bytes first, and never selects a current root or changes an
existing Authority consumer's role vocabulary.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import cast

from authority_source_resolver import (
    AuthoritySourceResolver,
    ResolutionError,
    ResolutionStatus,
    ResolvedArtifact,
)
from check_m2_5_b2_closure_v2 import (
    B2ClosureV2VerificationError,
    verify_closure_v2,
)
from mtgml.b2_closure_contract import (
    B2_CLOSURE_ROOT_ROLES,
    B2_CLOSURE_SOURCE_PACKAGE_SHA256,
    B2_CLOSURE_V1_PATH,
    B2_CLOSURE_V1_RAW_SHA256,
    B2_CLOSURE_V1_SCHEMA,
    B2_CLOSURE_V2_PATH,
    B2_CLOSURE_V2_SCHEMA,
)

B2_CLOSURE_V2_RAW_SHA256 = "43a0613be367159ffd224d6282deb4e263b2147b861acb90a26388d56729f748"

_LOWERCASE_SHA256 = re.compile(r"^[0-9a-f]{64}$")


class B2ClosureResolutionMode(StrEnum):
    """The only source-resolution modes admitted by Slice 3."""

    HISTORICAL_V1 = "HISTORICAL_V1"
    CANDIDATE_V2 = "CANDIDATE_V2"


@dataclass(frozen=True)
class B2ClosureSourceBinding:
    """An explicit, non-persisted B2 closure source binding."""

    artifact_role: str
    closure_version: str
    repository_relative_path: str
    schema_identifier: str
    raw_sha256: str
    source_package_sha256: str


@dataclass(frozen=True)
class B2ClosureSourceResolution:
    """Bytes resolved and verified for one explicit B2 closure request."""

    mode: B2ClosureResolutionMode
    binding: B2ClosureSourceBinding
    artifact: ResolvedArtifact
    v2_verified: bool
    v2_current: bool = False

    @property
    def raw_bytes(self) -> bytes:
        return cast(bytes, self.artifact.raw_bytes)

    @property
    def raw_sha256(self) -> str:
        return cast(str, self.artifact.raw_sha256)


_CANONICAL_BINDINGS = {
    B2ClosureResolutionMode.HISTORICAL_V1: B2ClosureSourceBinding(
        artifact_role="b2_closure",
        closure_version="v1",
        repository_relative_path=B2_CLOSURE_V1_PATH,
        schema_identifier=B2_CLOSURE_V1_SCHEMA,
        raw_sha256=B2_CLOSURE_V1_RAW_SHA256,
        source_package_sha256=B2_CLOSURE_SOURCE_PACKAGE_SHA256,
    ),
    B2ClosureResolutionMode.CANDIDATE_V2: B2ClosureSourceBinding(
        artifact_role="b2_closure_v2",
        closure_version="v2",
        repository_relative_path=B2_CLOSURE_V2_PATH,
        schema_identifier=B2_CLOSURE_V2_SCHEMA,
        raw_sha256=B2_CLOSURE_V2_RAW_SHA256,
        source_package_sha256=B2_CLOSURE_SOURCE_PACKAGE_SHA256,
    ),
}


def _fail(code: str, message: str) -> None:
    raise ResolutionError(ResolutionStatus.FAIL, code, message)


def _coerce_mode(mode: B2ClosureResolutionMode | str) -> B2ClosureResolutionMode:
    if isinstance(mode, B2ClosureResolutionMode):
        return mode
    try:
        return B2ClosureResolutionMode(mode)
    except (TypeError, ValueError) as exc:
        _fail("UNKNOWN_RESOLUTION_MODE", f"unsupported B2 closure resolution mode: {mode!r}")
        raise AssertionError("unreachable") from exc


def binding_for_mode(mode: B2ClosureResolutionMode | str) -> B2ClosureSourceBinding:
    """Return the immutable canonical binding for one explicit mode."""

    normalized = _coerce_mode(mode)
    return _CANONICAL_BINDINGS[normalized]


def _mode_for_binding(binding: B2ClosureSourceBinding) -> B2ClosureResolutionMode:
    if not isinstance(binding, B2ClosureSourceBinding):
        _fail("B2_CLOSURE_BINDING_INVALID", "B2 closure binding has the wrong type")

    if not isinstance(binding.artifact_role, str) or (
        binding.artifact_role not in B2_CLOSURE_ROOT_ROLES
    ):
        _fail("UNKNOWN_SOURCE_ROLE", f"unknown B2 closure role: {binding.artifact_role!r}")

    if not isinstance(binding.closure_version, str):
        _fail("UNKNOWN_CLOSURE_VERSION", "B2 closure version must be text")
    if binding.closure_version not in {"v1", "v2"}:
        code = (
            "UNSUPPORTED_CLOSURE_VERSION"
            if re.fullmatch(r"v[0-9]+", binding.closure_version)
            else "UNKNOWN_CLOSURE_VERSION"
        )
        _fail(code, f"unsupported B2 closure version: {binding.closure_version!r}")

    expected_role = "b2_closure" if binding.closure_version == "v1" else "b2_closure_v2"
    if binding.artifact_role != expected_role:
        _fail(
            "CURRENT_ROOT_ROLE_MISMATCH",
            f"role {binding.artifact_role!r} is not valid for {binding.closure_version}",
        )

    mode = (
        B2ClosureResolutionMode.HISTORICAL_V1
        if binding.closure_version == "v1"
        else B2ClosureResolutionMode.CANDIDATE_V2
    )
    expected = binding_for_mode(mode)

    if not isinstance(binding.repository_relative_path, str) or (
        binding.repository_relative_path != expected.repository_relative_path
    ):
        _fail(
            "CURRENT_ROOT_PATH_MISMATCH",
            f"B2 closure path is not the admitted exact path for {binding.artifact_role!r}",
        )
    if not isinstance(binding.schema_identifier, str) or (
        binding.schema_identifier != expected.schema_identifier
    ):
        _fail(
            "CURRENT_ROOT_SCHEMA_MISMATCH",
            f"B2 closure schema is not the admitted exact schema for {binding.artifact_role!r}",
        )
    if not isinstance(binding.raw_sha256, str) or (
        _LOWERCASE_SHA256.fullmatch(binding.raw_sha256) is None
    ):
        _fail("CURRENT_ROOT_DIGEST_MISMATCH", "B2 closure digest must be lowercase SHA-256 hex")
    if binding.raw_sha256 != expected.raw_sha256:
        _fail("CURRENT_ROOT_DIGEST_MISMATCH", "B2 closure digest is not the admitted exact digest")
    if not isinstance(binding.source_package_sha256, str) or (
        binding.source_package_sha256 != expected.source_package_sha256
    ):
        _fail(
            "SOURCE_PACKAGE_MISMATCH",
            "B2 closure source package is not the admitted exact digest",
        )
    if _LOWERCASE_SHA256.fullmatch(binding.source_package_sha256) is None:
        _fail("SOURCE_PACKAGE_MISMATCH", "B2 closure source package must be lowercase SHA-256 hex")

    return mode


def _verify_declared_source_package(
    artifact: ResolvedArtifact, binding: B2ClosureSourceBinding
) -> None:
    value = artifact.json_value
    if not isinstance(value, dict) or (
        value.get("source_package_sha256") != binding.source_package_sha256
    ):
        _fail("SOURCE_PACKAGE_MISMATCH", "resolved B2 closure declares the wrong source package")


def resolve_b2_closure_binding(
    repo_root: Path, binding: B2ClosureSourceBinding
) -> B2ClosureSourceResolution:
    """Resolve one explicitly supplied historical or candidate binding."""

    mode = _mode_for_binding(binding)
    resolver = AuthoritySourceResolver(repo_root)
    artifact = resolver.resolve_repository_artifact(
        binding.repository_relative_path,
        binding.raw_sha256,
        binding.schema_identifier,
    )
    _verify_declared_source_package(artifact, binding)

    v2_verified = False
    if mode is B2ClosureResolutionMode.CANDIDATE_V2:
        try:
            verify_closure_v2(
                repo_root,
                repo_root / Path(*binding.repository_relative_path.split("/")),
            )
        except B2ClosureV2VerificationError as exc:
            _fail("B2_CLOSURE_INVALID", str(exc))
        v2_verified = True

    return B2ClosureSourceResolution(
        mode=mode,
        binding=binding,
        artifact=artifact,
        v2_verified=v2_verified,
    )


def resolve_b2_closure(
    repo_root: Path, mode: B2ClosureResolutionMode | str
) -> B2ClosureSourceResolution:
    """Resolve only the canonical binding for an explicit mode.

    This function has no current-root or fallback behavior.  A candidate-v2
    failure is returned as a failure; historical-v1 is never tried implicitly.
    """

    return resolve_b2_closure_binding(repo_root, binding_for_mode(mode))


__all__ = [
    "B2_CLOSURE_V2_RAW_SHA256",
    "B2ClosureResolutionMode",
    "B2ClosureSourceBinding",
    "B2ClosureSourceResolution",
    "B2ClosureV2VerificationError",
    "binding_for_mode",
    "resolve_b2_closure",
    "resolve_b2_closure_binding",
]
