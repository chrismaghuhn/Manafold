"""Rules-neutral, read-only source and locator resolution for M2.5.C.

This module verifies bytes and source identity before parsing or interpreting
them. It resolves repository artifacts and the externally configured REV3
package only; it does not classify candidates, derive C semantics, or accept
authority records.
"""

from __future__ import annotations

import hashlib
import io
import json
import os
import re
import sys
import zipfile
from collections.abc import Mapping
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Literal, NoReturn, TypeAlias, cast

ROOT = Path(__file__).resolve().parents[1]
PYTHON_SRC = ROOT / "python" / "src"
if str(PYTHON_SRC) not in sys.path:
    sys.path.insert(0, str(PYTHON_SRC))

from mtgml.authority import (
    ACCEPTANCE_EVENT_SCHEMA_V1,
    REVIEWER_ROSTER_SCHEMA_V1,
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKind,
    ReviewAcceptanceEventInputV1,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewerRosterV1,
    ReviewerV1,
    ReviewEventRefV1,
    ReviewMode,
    SourceBindingDigestV1,
)

REV3_ARCHIVE_ENV_VAR = "MANAFOLD_SOURCE_ARCHIVE"
REV3_ARCHIVE_RELATIVE_PATH = Path("m2_5/Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip")
EXPECTED_REV3_ARCHIVE_SHA256 = "99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90"
REV3_PACKAGE_MANIFEST_MEMBER = "Manafold_M2_5_Package_Manifest_REV3.json"
REV3_PACKAGE_MANIFEST_SCHEMA = "manafold.m2.5.rev3.package-manifest.v1"
REV3_CENSUS_MEMBER = "derived/Pair_Interaction_Census_REV3.csv"

SourceKind: TypeAlias = Literal["repository", "rev3_archive"]
Locator: TypeAlias = tuple[str, str | int | None]
DigestInput: TypeAlias = str | bytes

_HEX64_RE = re.compile(r"^[0-9a-f]{64}$")
_EVENT_ID_RE = re.compile(r"^ae\.v1/[0-9a-f]{64}$")
_EVENT_LEAF_PATH_RE = re.compile(
    r"^sources/m2_5/authorities/review_acceptance_events/v1/[0-9a-f]{64}\.json$"
)


class ResolutionStatus(str, Enum):
    FAIL = "FAIL"
    BLOCKED = "BLOCKED"


class ResolutionError(Exception):
    """A source resolution failure with an explicit fail-closed status."""

    def __init__(self, status: ResolutionStatus, code: str, message: str) -> None:
        self.status = status
        self.code = code
        self.message = message
        super().__init__(f"[{status.value}:{code}] {message}")


@dataclass(frozen=True)
class ResolvedArtifact:
    source_kind: SourceKind
    path: str
    raw_bytes: bytes
    raw_sha256: str
    schema_or_null: str | None
    json_value: object | None


@dataclass(frozen=True)
class ResolvedLocator:
    artifact: ResolvedArtifact
    locator: Locator
    value: object


def _fail(code: str, message: str) -> NoReturn:
    raise ResolutionError(ResolutionStatus.FAIL, code, message)


def _blocked(code: str, message: str) -> NoReturn:
    raise ResolutionError(ResolutionStatus.BLOCKED, code, message)


def _digest_hex(value: DigestInput, label: str) -> str:
    if isinstance(value, bytes):
        if len(value) != 32:
            _fail("DIGEST_INVALID", f"{label} must contain exactly 32 bytes")
        return value.hex()
    if not isinstance(value, str) or _HEX64_RE.fullmatch(value) is None:
        _fail("DIGEST_INVALID", f"{label} must be lowercase SHA-256 hex")
    return value


def _relative_path(value: object, label: str) -> str:
    if not isinstance(value, str) or not value or "\x00" in value:
        _fail("PATH_INVALID", f"{label} must be a non-empty path")
    if (
        value.startswith(("/", "\\"))
        or "\\" in value
        or "://" in value
        or re.match(r"^[A-Za-z]:", value) is not None
    ):
        _fail("PATH_INVALID", f"{label} must be slash-separated and relative")
    parts = value.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        _fail("PATH_INVALID", f"{label} contains an invalid path segment")
    return value


def _json_object(value: object, label: str) -> dict[str, object]:
    if not isinstance(value, dict):
        _fail("JSON_OBJECT_REQUIRED", f"{label} must contain a JSON object")
    return cast(dict[str, object], value)


def _json_text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        _fail("JSON_FIELD_INVALID", f"{label} must be non-empty text")
    return value


def _json_digest(value: object, label: str) -> bytes:
    text = _json_text(value, label)
    if _HEX64_RE.fullmatch(text) is None:
        _fail("JSON_FIELD_INVALID", f"{label} must be SHA-256 hex")
    return bytes.fromhex(text)


def _verify_json_schema(raw: bytes, schema_or_null: str | None, path: str) -> object | None:
    if schema_or_null is None:
        return None
    try:
        value = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        _fail("JSON_INVALID", f"{path} is not valid UTF-8 JSON: {exc}")
    record = _json_object(value, path)
    if record.get("schema") != schema_or_null:
        _fail("SCHEMA_MISMATCH", f"{path} does not declare {schema_or_null!r}")
    return cast(object, value)


def _resolved_artifact(
    source_kind: SourceKind,
    path: str,
    raw: bytes,
    expected_raw_sha256: DigestInput,
    schema_or_null: str | None,
) -> ResolvedArtifact:
    expected = _digest_hex(expected_raw_sha256, f"{path} expected digest")
    actual = hashlib.sha256(raw).hexdigest()
    if actual != expected:
        _fail("SOURCE_DIGEST_MISMATCH", f"{path} has {actual}, expected {expected}")
    json_value = _verify_json_schema(raw, schema_or_null, path)
    return ResolvedArtifact(source_kind, path, raw, actual, schema_or_null, json_value)


def _json_pointer_token(raw_token: str) -> str:
    result: list[str] = []
    index = 0
    while index < len(raw_token):
        character = raw_token[index]
        if character != "~":
            result.append(character)
            index += 1
            continue
        if index + 1 >= len(raw_token) or raw_token[index + 1] not in "01":
            _fail("LOCATOR_INVALID", "JSON Pointer contains an invalid escape")
        result.append("~" if raw_token[index + 1] == "0" else "/")
        index += 2
    return "".join(result)


def _json_pointer(value: object, pointer: object) -> object:
    if not isinstance(pointer, str) or (pointer and not pointer.startswith("/")):
        _fail("LOCATOR_INVALID", "JSON Pointer must be empty or begin with '/'")
    if pointer == "":
        return value
    current = value
    for raw_token in pointer[1:].split("/"):
        token = _json_pointer_token(raw_token)
        if isinstance(current, dict):
            if token not in current:
                _fail("LOCATOR_UNRESOLVED", f"JSON Pointer token {token!r} is absent")
            current = current[token]
        elif isinstance(current, list):
            if token == "0":
                index = 0
            elif token.isdigit() and not token.startswith("0"):
                index = int(token)
            else:
                _fail("LOCATOR_UNRESOLVED", f"JSON Pointer array index {token!r} is invalid")
            if index >= len(current):
                _fail("LOCATOR_UNRESOLVED", f"JSON Pointer index {index} is out of range")
            current = current[index]
        else:
            _fail("LOCATOR_UNRESOLVED", "JSON Pointer traverses a scalar value")
    return current


def _locator(value: object) -> Locator:
    if not isinstance(value, tuple) or len(value) != 2:
        _fail("LOCATOR_INVALID", "locator must be a two-position tuple")
    kind, payload = value
    if kind == "whole_artifact":
        if payload is not None:
            _fail("LOCATOR_INVALID", "whole_artifact payload must be null")
    elif kind == "json_pointer":
        if not isinstance(payload, str):
            _fail("LOCATOR_INVALID", "json_pointer payload must be text")
    elif kind == "archive_member":
        _relative_path(payload, "archive member locator")
    elif kind == "event_id":
        if not isinstance(payload, str) or _EVENT_ID_RE.fullmatch(payload) is None:
            _fail("LOCATOR_INVALID", "event_id locator is not an acceptance-event ID")
    else:
        _fail("LOCATOR_INVALID", f"unknown locator variant {kind!r}")
    return cast(Locator, value)


def _exact_keys(value: Mapping[str, object], expected: set[str], label: str) -> None:
    if set(value) != expected:
        _fail("SCHEMA_MISMATCH", f"{label} fields are not exactly {sorted(expected)!r}")


class Rev3ArchiveStore:
    """Verified access to members of the pinned, non-extracted REV3 ZIP."""

    def __init__(
        self,
        raw: bytes,
        archive: zipfile.ZipFile,
        manifest_entries: dict[str, tuple[int, str]],
    ) -> None:
        self._raw = raw
        self._archive = archive
        self._manifest_entries = manifest_entries

    @classmethod
    def from_root(
        cls,
        root: Path,
        expected_archive_sha256: str = EXPECTED_REV3_ARCHIVE_SHA256,
    ) -> Rev3ArchiveStore:
        configured_root = root.resolve()
        archive_path = (configured_root / REV3_ARCHIVE_RELATIVE_PATH).resolve()
        try:
            archive_path.relative_to(configured_root)
        except ValueError:
            _fail("REV3_ARCHIVE_PATH_INVALID", "REV3 archive path escapes its configured root")
        if not archive_path.is_file():
            _blocked("REV3_ARCHIVE_SOURCE_UNAVAILABLE", f"REV3 archive is missing: {archive_path}")
        try:
            raw = archive_path.read_bytes()
        except OSError as exc:
            _blocked("REV3_ARCHIVE_SOURCE_UNAVAILABLE", f"cannot read {archive_path}: {exc}")
        return cls.from_bytes(raw, expected_archive_sha256)

    @classmethod
    def from_bytes(
        cls,
        raw: bytes,
        expected_archive_sha256: str,
    ) -> Rev3ArchiveStore:
        expected = _digest_hex(expected_archive_sha256, "REV3 archive digest")
        actual = hashlib.sha256(raw).hexdigest()
        if actual != expected:
            _fail("REV3_ARCHIVE_DIGEST_MISMATCH", f"REV3 archive has {actual}, expected {expected}")
        try:
            archive = zipfile.ZipFile(io.BytesIO(raw))
        except (OSError, zipfile.BadZipFile) as exc:
            _fail("REV3_ARCHIVE_INVALID", f"REV3 archive is not a readable ZIP: {exc}")

        infos = archive.infolist()
        names = [info.filename for info in infos]
        if len(names) != len(set(names)):
            _fail("REV3_MEMBER_DUPLICATE", "REV3 archive contains duplicate member names")
        for info in infos:
            if info.is_dir():
                _fail(
                    "REV3_MEMBER_INVALID",
                    f"REV3 archive contains a directory member {info.filename!r}",
                )
            _relative_path(info.filename, "REV3 archive member")
        if REV3_PACKAGE_MANIFEST_MEMBER not in names:
            _fail("REV3_MANIFEST_MISSING", "REV3 package manifest is missing")
        try:
            manifest_raw = archive.read(REV3_PACKAGE_MANIFEST_MEMBER)
            manifest_value = json.loads(manifest_raw.decode("utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError, zipfile.BadZipFile) as exc:
            _fail("REV3_MANIFEST_INVALID", f"REV3 package manifest is unreadable: {exc}")
        manifest = _json_object(manifest_value, "REV3 package manifest")
        if manifest.get("schema") != REV3_PACKAGE_MANIFEST_SCHEMA:
            _fail("REV3_MANIFEST_SCHEMA_MISMATCH", "REV3 package manifest schema is not V1")
        entries = manifest.get("entries")
        excluded = manifest.get("manifest_excluded_paths")
        if not isinstance(entries, list) or not isinstance(excluded, list):
            _fail("REV3_MANIFEST_INVALID", "REV3 package manifest entries are not arrays")
        if manifest.get("manifest_excludes_self") is not True:
            _fail("REV3_MANIFEST_INVALID", "REV3 package manifest must exclude itself")

        manifest_entries: dict[str, tuple[int, str]] = {}
        for raw_entry in entries:
            entry = _json_object(raw_entry, "REV3 package manifest entry")
            _exact_keys(entry, {"bytes", "path", "sha256"}, "REV3 package manifest entry")
            path = _relative_path(entry.get("path"), "REV3 package manifest entry path")
            if path == REV3_PACKAGE_MANIFEST_MEMBER or path in manifest_entries:
                _fail("REV3_MEMBER_DUPLICATE", f"duplicate manifest member {path!r}")
            size = entry.get("bytes")
            if isinstance(size, bool) or not isinstance(size, int) or size < 0:
                _fail("REV3_MANIFEST_INVALID", f"invalid byte count for {path!r}")
            sha = _digest_hex(
                cast(DigestInput, entry.get("sha256", "")), f"manifest member {path} digest"
            )
            manifest_entries[path] = (size, sha)

        excluded_paths: set[str] = set()
        for raw_path in excluded:
            path = _relative_path(raw_path, "REV3 manifest exclusion")
            if path == REV3_PACKAGE_MANIFEST_MEMBER or path in excluded_paths:
                _fail("REV3_MEMBER_DUPLICATE", f"duplicate manifest exclusion {path!r}")
            excluded_paths.add(path)

        actual_members = set(names) - {REV3_PACKAGE_MANIFEST_MEMBER}
        expected_members = set(manifest_entries) | excluded_paths
        missing_declared = sorted(set(manifest_entries) - actual_members)
        if missing_declared:
            _fail(
                "REV3_MEMBER_MISSING", f"REV3 manifest member is missing: {missing_declared[0]!r}"
            )
        if actual_members != expected_members:
            missing = sorted(expected_members - actual_members)
            extra = sorted(actual_members - expected_members)
            _fail(
                "REV3_MEMBER_SET_MISMATCH",
                f"REV3 manifest/member set differs; missing={missing!r}, extra={extra!r}",
            )

        for path, (size, expected_sha) in manifest_entries.items():
            try:
                payload = archive.read(path)
            except (OSError, zipfile.BadZipFile) as exc:
                _fail("REV3_MEMBER_READ_FAILED", f"cannot read REV3 member {path!r}: {exc}")
            if len(payload) != size:
                _fail("REV3_MEMBER_SIZE_MISMATCH", f"REV3 member {path!r} has the wrong size")
            actual_sha = hashlib.sha256(payload).hexdigest()
            if actual_sha != expected_sha:
                _fail(
                    "REV3_MEMBER_DIGEST_MISMATCH",
                    f"REV3 member {path!r} has {actual_sha}, expected {expected_sha}",
                )
        return cls(raw, archive, manifest_entries)

    def resolve_member(
        self,
        member_path: str,
        expected_raw_sha256: DigestInput,
        schema_or_null: str | None = None,
    ) -> ResolvedArtifact:
        path = _relative_path(member_path, "REV3 member path")
        entry = self._manifest_entries.get(path)
        if entry is None:
            _fail("REV3_MEMBER_MISSING", f"REV3 member {path!r} is not in the manifest")
        try:
            raw = self._archive.read(path)
        except (OSError, zipfile.BadZipFile) as exc:
            _fail("REV3_MEMBER_READ_FAILED", f"cannot read REV3 member {path!r}: {exc}")
        if hashlib.sha256(raw).hexdigest() != entry[1]:
            _fail(
                "REV3_MEMBER_DIGEST_MISMATCH",
                f"REV3 member {path!r} changed after package validation",
            )
        return _resolved_artifact("rev3_archive", path, raw, expected_raw_sha256, schema_or_null)

    def resolve_locator(
        self,
        locator: Locator,
        expected_raw_sha256: DigestInput,
        schema_or_null: str | None = None,
    ) -> ResolvedLocator:
        kind, payload = _locator(locator)
        if kind != "archive_member" or not isinstance(payload, str):
            _fail("LOCATOR_INVALID", "REV3 resolution requires an archive_member locator")
        artifact = self.resolve_member(payload, expected_raw_sha256, schema_or_null)
        return ResolvedLocator(artifact, locator, artifact.raw_bytes)


class AuthoritySourceResolver:
    """Resolve source bytes without granting them semantic authority."""

    def __init__(
        self,
        repo_root: Path,
        *,
        rev3_archive_root: Path | None = None,
        rev3_archive: Rev3ArchiveStore | None = None,
        expected_rev3_archive_sha256: str = EXPECTED_REV3_ARCHIVE_SHA256,
    ) -> None:
        self._repo_root = repo_root.resolve()
        self._rev3_archive_root = rev3_archive_root
        self._rev3_archive = rev3_archive
        self._expected_rev3_archive_sha256 = expected_rev3_archive_sha256
        if rev3_archive_root is not None and rev3_archive is not None:
            _fail(
                "CONFIGURATION_INVALID",
                "provide either rev3_archive_root or rev3_archive, not both",
            )

    def resolve_repository_artifact(
        self,
        path: str,
        expected_raw_sha256: DigestInput,
        schema_or_null: str | None,
    ) -> ResolvedArtifact:
        relative = _relative_path(path, "repository source path")
        candidate = (self._repo_root / Path(*relative.split("/"))).resolve()
        try:
            candidate.relative_to(self._repo_root)
        except ValueError:
            _fail(
                "REPOSITORY_PATH_ESCAPES",
                f"repository path escapes the configured root: {relative}",
            )
        if not candidate.is_file():
            _blocked("REPOSITORY_SOURCE_UNAVAILABLE", f"repository source is missing: {relative}")
        try:
            raw = candidate.read_bytes()
        except OSError as exc:
            _blocked(
                "REPOSITORY_SOURCE_UNAVAILABLE", f"cannot read repository source {relative}: {exc}"
            )
        return _resolved_artifact("repository", relative, raw, expected_raw_sha256, schema_or_null)

    def _archive(self) -> Rev3ArchiveStore:
        if self._rev3_archive is None:
            root = self._rev3_archive_root
            if root is None:
                configured = os.environ.get(REV3_ARCHIVE_ENV_VAR)
                if not configured:
                    _blocked(
                        "REV3_ARCHIVE_SOURCE_UNAVAILABLE",
                        f"{REV3_ARCHIVE_ENV_VAR} is not configured",
                    )
                root = Path(configured)
            self._rev3_archive = Rev3ArchiveStore.from_root(
                root, self._expected_rev3_archive_sha256
            )
        return self._rev3_archive

    def resolve_rev3_member(
        self,
        member_path: str,
        expected_raw_sha256: DigestInput,
        schema_or_null: str | None = None,
    ) -> ResolvedArtifact:
        return self._archive().resolve_member(member_path, expected_raw_sha256, schema_or_null)

    def resolve_rev3_locator(
        self,
        locator: Locator,
        expected_raw_sha256: DigestInput,
        schema_or_null: str | None = None,
    ) -> ResolvedLocator:
        return self._archive().resolve_locator(locator, expected_raw_sha256, schema_or_null)

    def resolve_source_binding(self, binding: SourceBindingDigestV1) -> ResolvedArtifact:
        if binding.artifact_role == "rev3_source":
            return self.resolve_rev3_member(
                binding.path, binding.raw_sha256, binding.schema_or_null
            )
        return self.resolve_repository_artifact(
            binding.path, binding.raw_sha256, binding.schema_or_null
        )

    def resolve_locator(self, artifact: ResolvedArtifact, locator: Locator) -> ResolvedLocator:
        kind, payload = _locator(locator)
        if kind == "whole_artifact":
            value = artifact.json_value if artifact.json_value is not None else artifact.raw_bytes
            return ResolvedLocator(artifact, locator, value)
        if kind == "json_pointer":
            if artifact.json_value is None:
                _fail("LOCATOR_INVALID", "json_pointer requires a JSON artifact schema")
            return ResolvedLocator(artifact, locator, _json_pointer(artifact.json_value, payload))
        if kind == "event_id" and isinstance(payload, str):
            if artifact.source_kind != "repository":
                _fail("LOCATOR_INVALID", "event_id locators require a repository event leaf")
            try:
                reference = ReviewEventRefV1(
                    path=artifact.path,
                    raw_sha256=bytes.fromhex(artifact.raw_sha256),
                    event_id=payload,
                )
            except ValueError as exc:
                _fail("ACCEPTANCE_EVENT_ID_MISMATCH", str(exc))
            resolved = self.resolve_acceptance_event_leaf(reference)
            return ResolvedLocator(artifact, locator, cast(dict[str, object], resolved.json_value))
        _fail("LOCATOR_INVALID", "archive_member locators require resolve_rev3_locator")

    def resolve_acceptance_event_leaf(self, reference: ReviewEventRefV1) -> ResolvedArtifact:
        if _EVENT_LEAF_PATH_RE.fullmatch(reference.path) is None:
            _fail("ACCEPTANCE_EVENT_PATH_INVALID", "acceptance event path is not a V1 leaf path")
        artifact = self.resolve_repository_artifact(
            reference.path, reference.raw_sha256, ACCEPTANCE_EVENT_SCHEMA_V1
        )
        event = _json_object(artifact.json_value, "acceptance event leaf")
        if event.get("event_id") != reference.event_id:
            _fail(
                "ACCEPTANCE_EVENT_ID_MISMATCH",
                "acceptance event locator does not match the leaf event_id",
            )
        expected_path = (
            "sources/m2_5/authorities/review_acceptance_events/v1/"
            + reference.event_id.removeprefix("ae.v1/")
            + ".json"
        )
        if reference.path != expected_path:
            _fail("ACCEPTANCE_EVENT_PATH_INVALID", "acceptance event path is not bound to event_id")
        self._verify_acceptance_event_identity(event, reference.event_id)
        return artifact

    def resolve_reviewer_roster_leaf(self, reference: ReviewerRosterRefV1) -> ResolvedArtifact:
        artifact = self.resolve_repository_artifact(
            reference.path, reference.raw_sha256, REVIEWER_ROSTER_SCHEMA_V1
        )
        roster = _json_object(artifact.json_value, "reviewer roster leaf")
        _exact_keys(roster, {"schema", "reviewers"}, "reviewer roster leaf")
        raw_reviewers = roster.get("reviewers")
        if not isinstance(raw_reviewers, list):
            _fail("REVIEWER_ROSTER_INVALID", "reviewer roster reviewers must be an array")
        reviewers: list[ReviewerV1] = []
        for raw_reviewer in raw_reviewers:
            record = _json_object(raw_reviewer, "reviewer roster entry")
            _exact_keys(record, {"reviewer_id", "roles"}, "reviewer roster entry")
            roles = record.get("roles")
            if not isinstance(roles, list) or any(not isinstance(role, str) for role in roles):
                _fail("REVIEWER_ROSTER_INVALID", "reviewer roster roles must be text")
            try:
                reviewers.append(
                    ReviewerV1(
                        _json_text(record.get("reviewer_id"), "reviewer ID"),
                        tuple(cast(list[str], roles)),
                    )
                )
            except (TypeError, ValueError) as exc:
                _fail("REVIEWER_ROSTER_INVALID", str(exc))
        try:
            ReviewerRosterV1(tuple(reviewers))
        except (TypeError, ValueError) as exc:
            _fail("REVIEWER_ROSTER_INVALID", str(exc))
        return artifact

    def _verify_acceptance_event_identity(
        self,
        event: Mapping[str, object],
        expected_event_id: str,
    ) -> None:
        _exact_keys(
            event,
            {
                "event_id",
                "schema",
                "subject_kind",
                "subject_payload_digest",
                "decision",
                "reviewer_roster_ref",
                "reviewer_role_bindings",
                "review_mode",
                "checklist_id",
                "source_binding_digests",
                "review_evidence_refs",
            },
            "acceptance event leaf",
        )
        if event.get("decision") != "human_accepted":
            _fail("ACCEPTANCE_EVENT_INVALID", "acceptance event decision is not human_accepted")
        if event.get("checklist_id") != "interaction-authority-review-checklist.v1":
            _fail("ACCEPTANCE_EVENT_INVALID", "acceptance event checklist is not the V1 contract")
        roster_record = _json_object(event.get("reviewer_roster_ref"), "reviewer roster reference")
        _exact_keys(roster_record, {"path", "schema", "raw_sha256"}, "reviewer roster reference")
        try:
            roster_ref = ReviewerRosterRefV1(
                path=_json_text(roster_record.get("path"), "reviewer roster path"),
                schema=_json_text(roster_record.get("schema"), "reviewer roster schema"),
                raw_sha256=_json_digest(roster_record.get("raw_sha256"), "reviewer roster digest"),
            )
            raw_bindings = event.get("reviewer_role_bindings")
            if not isinstance(raw_bindings, list):
                raise ValueError("reviewer role bindings must be an array")
            role_bindings = tuple(
                ReviewerRoleBindingV1(
                    reviewer_id=_json_text(
                        _json_object(item, "reviewer role binding").get("reviewer_id"),
                        "reviewer ID",
                    ),
                    roles=tuple(
                        cast(list[str], _json_object(item, "reviewer role binding").get("roles"))
                    ),
                )
                for item in raw_bindings
            )
            raw_sources = event.get("source_binding_digests")
            if not isinstance(raw_sources, list):
                raise ValueError("source binding digests must be an array")
            source_bindings = tuple(
                SourceBindingDigestV1(
                    artifact_role=_json_text(
                        _json_object(item, "source binding").get("artifact_role"), "artifact role"
                    ),
                    path=_json_text(
                        _json_object(item, "source binding").get("path"), "source binding path"
                    ),
                    schema_or_null=cast(
                        str | None, _json_object(item, "source binding").get("schema_or_null")
                    ),
                    raw_sha256=_json_digest(
                        _json_object(item, "source binding").get("raw_sha256"),
                        "source binding digest",
                    ),
                )
                for item in raw_sources
            )
            raw_evidence = event.get("review_evidence_refs")
            if not isinstance(raw_evidence, list):
                raise ValueError("review evidence references must be an array")
            evidence = tuple(
                AcceptanceEvidenceRefV1(
                    path=_json_text(
                        _json_object(item, "acceptance evidence").get("path"),
                        "acceptance evidence path",
                    ),
                    raw_sha256=_json_digest(
                        _json_object(item, "acceptance evidence").get("raw_sha256"),
                        "acceptance evidence digest",
                    ),
                    locator=self._wire_locator(
                        _json_object(item, "acceptance evidence").get("locator")
                    ),
                )
                for item in raw_evidence
            )
            candidate = ReviewAcceptanceEventInputV1(
                subject_kind=AcceptanceSubjectKind(
                    _json_text(event.get("subject_kind"), "subject kind")
                ),
                subject_payload_digest=_json_digest(
                    event.get("subject_payload_digest"), "subject payload digest"
                ),
                reviewer_roster_ref=roster_ref,
                reviewer_role_bindings=role_bindings,
                review_mode=ReviewMode(_json_text(event.get("review_mode"), "review mode")),
                source_binding_digests=source_bindings,
                review_evidence_refs=evidence,
            )
            actual_event_id = candidate.identity().as_text()
        except (TypeError, ValueError) as exc:
            _fail("ACCEPTANCE_EVENT_INVALID", str(exc))
        if actual_event_id != expected_event_id:
            _fail(
                "ACCEPTANCE_EVENT_ID_MISMATCH",
                f"acceptance event bytes derive {actual_event_id}, expected {expected_event_id}",
            )

    @staticmethod
    def _wire_locator(value: object) -> Locator:
        record = _json_object(value, "locator")
        _exact_keys(record, {"kind"} | ({"value"} if "value" in record else set()), "locator")
        kind = _json_text(record.get("kind"), "locator kind")
        payload = record.get("value")
        return _locator((kind, cast(str | int | None, payload)))


__all__ = [
    "ACCEPTANCE_EVENT_SCHEMA_V1",
    "EXPECTED_REV3_ARCHIVE_SHA256",
    "REV3_CENSUS_MEMBER",
    "AuthoritySourceResolver",
    "ResolutionError",
    "ResolutionStatus",
    "ResolvedArtifact",
    "ResolvedLocator",
    "Rev3ArchiveStore",
]
