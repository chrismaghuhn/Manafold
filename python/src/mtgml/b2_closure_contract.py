"""ADR 0046 Slice-1 B2 closure/current-root contract vocabulary.

This module defines shapes and fail-closed validation only. It does not read
the repository, materialize closure v2, select a current root, or migrate any
authority consumer.
"""

from __future__ import annotations

import re
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from enum import StrEnum
from typing import Final

B2_CLOSURE_CURRENT_ROOT_SCHEMA: Final = "manafold.m2.5.b2.closure-current-root.v1"
B2_CLOSURE_ARTIFACT_BINDING_SCHEMA: Final = "manafold.m2.5.b2.closure-artifact-binding.v1"
B2_CLOSURE_V1_PATH: Final = "sources/m2_5/closures/B2/classification_closure.v1.json"
B2_CLOSURE_V1_SCHEMA: Final = "manafold.m2.5.b2.classification-closure.v1"
B2_CLOSURE_V2_PATH: Final = "sources/m2_5/closures/B2/classification_closure.v2.json"
B2_CLOSURE_V2_SCHEMA: Final = "manafold.m2.5.b2.classification-closure.v2"
B2_CLOSURE_SOURCE_PACKAGE_SHA256: Final = (
    "99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90"
)
B2_CLOSURE_V1_RAW_SHA256: Final = "ed6a0bf4b0eb83c85027fdcc61eaf32bfa7bb06d4de78c77d0946d87212e7d43"

B2_CLOSURE_ARTIFACT_ROLES: Final = frozenset(
    {
        "b2_classifications_v1",
        "b2_family_catalog_v1",
        "b2_projection_v1",
    }
)
B2_CLOSURE_ROOT_ROLES: Final = frozenset({"b2_closure", "b2_closure_v2"})

B2_CLOSURE_SNAPSHOT_CONSTANTS: Final = {
    "oracle_semantic_identity_count": 402,
    "classification_count": 402,
    "terminal_assignment_edge_count": 1883,
    "deck_row_count": 441,
    "projection_row_count": 441,
    "historical_family_count": 216,
    "catalog_family_count": 216,
}

B2_CLOSURE_ARTIFACT_BINDINGS: Final = {
    "b2_classifications_v1": (
        "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
        "manafold.m2.5.b2.card-semantic-classifications.v1",
        "40cd5b9c37e26157a6df0449a75040f8a5879d825e3946dd500d666a502201d5",
    ),
    "b2_family_catalog_v1": (
        "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
        "manafold.m2.5.b2.requirement-family-catalog.v1",
        "a9dc94b86a2efdb6885081191e53380cf5b3723a58487600b6372bcb789abb92",
    ),
    "b2_projection_v1": (
        "sources/m2_5/closures/B2/deck_row_classification_refs.v1.csv",
        "manafold.m2.5.b2.deck-row-classification-refs.v1",
        "59a0f6ca00af6376fcce1d6c33c06ada3655c2c7fb3bf07354ab3643a96dba5a",
    ),
}

_HEX64 = re.compile(r"^[0-9a-f]{64}$")
_VERSION = re.compile(r"^v[0-9]+$")


class B2ClosureContractErrorCode(StrEnum):
    CURRENT_ROOT_MISSING = "CURRENT_ROOT_MISSING"
    MULTIPLE_CURRENT_ROOTS = "MULTIPLE_CURRENT_ROOTS"
    UNKNOWN_CLOSURE_VERSION = "UNKNOWN_CLOSURE_VERSION"
    UNSUPPORTED_CLOSURE_VERSION = "UNSUPPORTED_CLOSURE_VERSION"
    CURRENT_ROOT_ROLE_MISMATCH = "CURRENT_ROOT_ROLE_MISMATCH"
    CURRENT_ROOT_PATH_MISMATCH = "CURRENT_ROOT_PATH_MISMATCH"
    CURRENT_ROOT_SCHEMA_MISMATCH = "CURRENT_ROOT_SCHEMA_MISMATCH"
    CURRENT_ROOT_DIGEST_MISMATCH = "CURRENT_ROOT_DIGEST_MISMATCH"
    SOURCE_PACKAGE_MISMATCH = "SOURCE_PACKAGE_MISMATCH"
    REFERENCED_SEMANTIC_ARTIFACT_MISMATCH = "REFERENCED_SEMANTIC_ARTIFACT_MISMATCH"
    UNKNOWN_SOURCE_ROLE = "UNKNOWN_SOURCE_ROLE"
    DUPLICATE_SOURCE_ROLE = "DUPLICATE_SOURCE_ROLE"
    CURRENT_CONSUMER_VERSION_UNSUPPORTED = "CURRENT_CONSUMER_VERSION_UNSUPPORTED"
    INVALID_ROOT_SHAPE = "INVALID_ROOT_SHAPE"
    INVALID_ARTIFACT_BINDING_SHAPE = "INVALID_ARTIFACT_BINDING_SHAPE"
    NONCANONICAL_SOURCE_ROLE_ORDER = "NONCANONICAL_SOURCE_ROLE_ORDER"


class B2ClosureContractError(ValueError):
    """A closed Slice-1 contract rejection category."""

    def __init__(self, code: B2ClosureContractErrorCode, detail: str = "") -> None:
        self.code = code
        self.detail = detail
        message = code.value if not detail else f"{code.value}: {detail}"
        super().__init__(message)


class B2ClosureCurrentnessState(StrEnum):
    V1_CURRENT = "v1_current"
    V2_READY_NOT_ADOPTED = "v2_ready_not_adopted"
    V2_ADOPTED = "v2_adopted"


def _require_wire_fields(
    value: Mapping[str, object], expected: set[str], code: B2ClosureContractErrorCode
) -> None:
    if set(value) != expected:
        raise B2ClosureContractError(code, "closed field set mismatch")


def _require_text(value: object, field: str, code: B2ClosureContractErrorCode) -> str:
    if not isinstance(value, str) or not value:
        raise B2ClosureContractError(code, f"{field} must be non-empty text")
    return value


def _validate_sha(value: str, code: B2ClosureContractErrorCode) -> None:
    if _HEX64.fullmatch(value) is None:
        raise B2ClosureContractError(code, "expected lowercase SHA-256 hex")


def _version_error(value: str) -> B2ClosureContractError:
    if _VERSION.fullmatch(value):
        return B2ClosureContractError(B2ClosureContractErrorCode.UNSUPPORTED_CLOSURE_VERSION, value)
    return B2ClosureContractError(B2ClosureContractErrorCode.UNKNOWN_CLOSURE_VERSION, value)


@dataclass(frozen=True)
class B2ClosureCurrentRootV1:
    schema: str
    artifact_role: str
    closure_version: str
    repository_relative_path: str
    closure_schema_id: str
    closure_raw_sha256: str
    source_package_sha256: str

    @classmethod
    def from_wire(cls, value: object) -> B2ClosureCurrentRootV1:
        if not isinstance(value, Mapping):
            raise B2ClosureContractError(B2ClosureContractErrorCode.INVALID_ROOT_SHAPE)
        expected = {
            "schema",
            "artifact_role",
            "closure_version",
            "repository_relative_path",
            "closure_schema_id",
            "closure_raw_sha256",
            "source_package_sha256",
        }
        _require_wire_fields(value, expected, B2ClosureContractErrorCode.INVALID_ROOT_SHAPE)
        return cls(
            schema=_require_text(
                value["schema"], "schema", B2ClosureContractErrorCode.INVALID_ROOT_SHAPE
            ),
            artifact_role=_require_text(
                value["artifact_role"],
                "artifact_role",
                B2ClosureContractErrorCode.INVALID_ROOT_SHAPE,
            ),
            closure_version=_require_text(
                value["closure_version"],
                "closure_version",
                B2ClosureContractErrorCode.INVALID_ROOT_SHAPE,
            ),
            repository_relative_path=_require_text(
                value["repository_relative_path"],
                "repository_relative_path",
                B2ClosureContractErrorCode.INVALID_ROOT_SHAPE,
            ),
            closure_schema_id=_require_text(
                value["closure_schema_id"],
                "closure_schema_id",
                B2ClosureContractErrorCode.INVALID_ROOT_SHAPE,
            ),
            closure_raw_sha256=_require_text(
                value["closure_raw_sha256"],
                "closure_raw_sha256",
                B2ClosureContractErrorCode.INVALID_ROOT_SHAPE,
            ),
            source_package_sha256=_require_text(
                value["source_package_sha256"],
                "source_package_sha256",
                B2ClosureContractErrorCode.INVALID_ROOT_SHAPE,
            ),
        )

    def validate(self, *, expected_raw_sha256: str | None = None) -> B2ClosureCurrentRootV1:
        if self.schema != B2_CLOSURE_CURRENT_ROOT_SCHEMA:
            raise B2ClosureContractError(B2ClosureContractErrorCode.CURRENT_ROOT_SCHEMA_MISMATCH)
        if self.closure_version not in {"v1", "v2"}:
            raise _version_error(self.closure_version)
        if self.artifact_role not in B2_CLOSURE_ROOT_ROLES:
            raise B2ClosureContractError(
                B2ClosureContractErrorCode.UNKNOWN_SOURCE_ROLE, self.artifact_role
            )
        expected_role = "b2_closure" if self.closure_version == "v1" else "b2_closure_v2"
        if self.artifact_role != expected_role:
            raise B2ClosureContractError(B2ClosureContractErrorCode.CURRENT_ROOT_ROLE_MISMATCH)
        expected_path = B2_CLOSURE_V1_PATH if self.closure_version == "v1" else B2_CLOSURE_V2_PATH
        expected_schema = (
            B2_CLOSURE_V1_SCHEMA if self.closure_version == "v1" else B2_CLOSURE_V2_SCHEMA
        )
        if self.repository_relative_path != expected_path:
            raise B2ClosureContractError(B2ClosureContractErrorCode.CURRENT_ROOT_PATH_MISMATCH)
        if self.closure_schema_id != expected_schema:
            raise B2ClosureContractError(B2ClosureContractErrorCode.CURRENT_ROOT_SCHEMA_MISMATCH)
        _validate_sha(
            self.closure_raw_sha256, B2ClosureContractErrorCode.CURRENT_ROOT_DIGEST_MISMATCH
        )
        if expected_raw_sha256 is not None and self.closure_raw_sha256 != expected_raw_sha256:
            raise B2ClosureContractError(B2ClosureContractErrorCode.CURRENT_ROOT_DIGEST_MISMATCH)
        if self.closure_version == "v1" and self.closure_raw_sha256 != B2_CLOSURE_V1_RAW_SHA256:
            raise B2ClosureContractError(B2ClosureContractErrorCode.CURRENT_ROOT_DIGEST_MISMATCH)
        _validate_sha(
            self.source_package_sha256, B2ClosureContractErrorCode.SOURCE_PACKAGE_MISMATCH
        )
        if self.source_package_sha256 != B2_CLOSURE_SOURCE_PACKAGE_SHA256:
            raise B2ClosureContractError(B2ClosureContractErrorCode.SOURCE_PACKAGE_MISMATCH)
        return self


def validate_current_root_set(roots: Sequence[B2ClosureCurrentRootV1]) -> B2ClosureCurrentRootV1:
    if not roots:
        raise B2ClosureContractError(B2ClosureContractErrorCode.CURRENT_ROOT_MISSING)
    if len(roots) != 1:
        raise B2ClosureContractError(B2ClosureContractErrorCode.MULTIPLE_CURRENT_ROOTS)
    return roots[0].validate()


_ARTIFACT_REGISTRY = B2_CLOSURE_ARTIFACT_BINDINGS


@dataclass(frozen=True)
class B2ClosureArtifactBindingV1:
    artifact_role: str
    repository_relative_path: str
    schema_identifier: str
    raw_sha256: str

    @classmethod
    def from_wire(cls, value: object) -> B2ClosureArtifactBindingV1:
        if not isinstance(value, Mapping):
            raise B2ClosureContractError(B2ClosureContractErrorCode.INVALID_ARTIFACT_BINDING_SHAPE)
        expected = {
            "artifact_role",
            "repository_relative_path",
            "schema_identifier",
            "raw_sha256",
        }
        _require_wire_fields(
            value, expected, B2ClosureContractErrorCode.INVALID_ARTIFACT_BINDING_SHAPE
        )
        return cls(
            artifact_role=_require_text(
                value["artifact_role"],
                "artifact_role",
                B2ClosureContractErrorCode.INVALID_ARTIFACT_BINDING_SHAPE,
            ),
            repository_relative_path=_require_text(
                value["repository_relative_path"],
                "repository_relative_path",
                B2ClosureContractErrorCode.INVALID_ARTIFACT_BINDING_SHAPE,
            ),
            schema_identifier=_require_text(
                value["schema_identifier"],
                "schema_identifier",
                B2ClosureContractErrorCode.INVALID_ARTIFACT_BINDING_SHAPE,
            ),
            raw_sha256=_require_text(
                value["raw_sha256"],
                "raw_sha256",
                B2ClosureContractErrorCode.INVALID_ARTIFACT_BINDING_SHAPE,
            ),
        )

    def validate(self) -> B2ClosureArtifactBindingV1:
        expected = _ARTIFACT_REGISTRY.get(self.artifact_role)
        if expected is None:
            raise B2ClosureContractError(B2ClosureContractErrorCode.UNKNOWN_SOURCE_ROLE)
        _validate_sha(
            self.raw_sha256, B2ClosureContractErrorCode.REFERENCED_SEMANTIC_ARTIFACT_MISMATCH
        )
        if (self.repository_relative_path, self.schema_identifier, self.raw_sha256) != expected:
            raise B2ClosureContractError(
                B2ClosureContractErrorCode.REFERENCED_SEMANTIC_ARTIFACT_MISMATCH
            )
        return self


def validate_artifact_bindings(
    bindings: Sequence[B2ClosureArtifactBindingV1],
) -> tuple[B2ClosureArtifactBindingV1, ...]:
    roles = [binding.artifact_role for binding in bindings]
    if len(set(roles)) != len(roles):
        raise B2ClosureContractError(B2ClosureContractErrorCode.DUPLICATE_SOURCE_ROLE)
    unknown_roles = set(roles) - B2_CLOSURE_ARTIFACT_ROLES
    if unknown_roles:
        raise B2ClosureContractError(B2ClosureContractErrorCode.UNKNOWN_SOURCE_ROLE)
    if len(bindings) != len(_ARTIFACT_REGISTRY):
        raise B2ClosureContractError(
            B2ClosureContractErrorCode.REFERENCED_SEMANTIC_ARTIFACT_MISMATCH
        )
    if roles != sorted(roles):
        raise B2ClosureContractError(B2ClosureContractErrorCode.NONCANONICAL_SOURCE_ROLE_ORDER)
    if set(roles) != B2_CLOSURE_ARTIFACT_ROLES:
        raise B2ClosureContractError(
            B2ClosureContractErrorCode.REFERENCED_SEMANTIC_ARTIFACT_MISMATCH
        )
    return tuple(binding.validate() for binding in bindings)
