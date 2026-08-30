from __future__ import annotations

import hashlib
import io
import json
import sys
import tempfile
import unittest
import warnings
import zipfile
from collections.abc import Callable
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import (
    ACCEPTANCE_EVENT_SCHEMA_V1,
    REV3_CENSUS_MEMBER,
    AuthoritySourceResolver,
    ResolutionError,
    ResolutionStatus,
    Rev3ArchiveStore,
)
from mtgml.authority import (
    ReviewerRosterRefV1,
    ReviewEventRefV1,
    SourceBindingDigestV1,
)


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def json_bytes(value: object) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def archive_bytes(
    entries: dict[str, bytes],
    *,
    manifest_entries: list[dict[str, object]] | None = None,
    duplicate_member: str | None = None,
) -> bytes:
    manifest = {
        "schema": "manafold.m2.5.rev3.package-manifest.v1",
        "entries": manifest_entries
        if manifest_entries is not None
        else [
            {"path": path, "bytes": len(raw), "sha256": digest(raw)}
            for path, raw in entries.items()
        ],
        "manifest_excluded_paths": [],
        "manifest_excludes_self": True,
    }
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_STORED) as archive:
        archive.writestr("Manafold_M2_5_Package_Manifest_REV3.json", json_bytes(manifest))
        for path, raw in entries.items():
            archive.writestr(path, raw)
            if path == duplicate_member:
                with warnings.catch_warnings():
                    warnings.simplefilter("ignore", UserWarning)
                    archive.writestr(path, raw)
    return buffer.getvalue()


class AuthoritySourceResolverTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.repo = Path(self.tempdir.name) / "repo"
        self.repo.mkdir()

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def write_repo(self, relative: str, raw: bytes) -> Path:
        path = self.repo / Path(*relative.split("/"))
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        return path

    def assert_resolution_error(
        self,
        operation: Callable[[], object],
        status: ResolutionStatus,
        code: str,
    ) -> None:
        with self.assertRaises(ResolutionError) as context:
            operation()
        self.assertEqual(context.exception.status, status)
        self.assertEqual(context.exception.code, code)

    def test_repository_verifies_digest_before_json_parsing(self) -> None:
        resolver = AuthoritySourceResolver(self.repo)
        raw = b"{not valid JSON"
        self.write_repo("artifact.json", raw)

        self.assert_resolution_error(
            lambda: resolver.resolve_repository_artifact(
                "artifact.json", "00" * 32, "example.schema.v1"
            ),
            ResolutionStatus.FAIL,
            "SOURCE_DIGEST_MISMATCH",
        )

        valid = json_bytes({"schema": "example.schema.v1", "value": 7})
        self.write_repo("artifact.json", valid)
        resolved = resolver.resolve_repository_artifact(
            "artifact.json", digest(valid), "example.schema.v1"
        )
        self.assertEqual(resolved.raw_bytes, valid)
        self.assertEqual(resolved.raw_sha256, digest(valid))
        self.assertEqual(resolved.json_value, {"schema": "example.schema.v1", "value": 7})

    def test_repository_paths_are_confined_and_normalized(self) -> None:
        resolver = AuthoritySourceResolver(self.repo)
        for relative in ("../outside.json", "/absolute.json", "C:/outside.json", "a\\b.json"):
            with self.subTest(relative=relative):

                def resolve_invalid_path(relative: str = relative) -> object:
                    return resolver.resolve_repository_artifact(relative, "00" * 32, None)

                self.assert_resolution_error(
                    resolve_invalid_path,
                    ResolutionStatus.FAIL,
                    "PATH_INVALID",
                )

    def test_json_pointer_uses_strict_rfc6901_evaluation(self) -> None:
        value = {
            "schema": "example.schema.v1",
            "a/b": {"m~n": ["selected"]},
            "array": ["first", "second"],
        }
        raw = json_bytes(value)
        self.write_repo("artifact.json", raw)
        resolver = AuthoritySourceResolver(self.repo)
        artifact = resolver.resolve_repository_artifact(
            "artifact.json", digest(raw), "example.schema.v1"
        )

        self.assertEqual(
            resolver.resolve_locator(artifact, ("json_pointer", "")).value,
            value,
        )
        self.assertEqual(
            resolver.resolve_locator(artifact, ("json_pointer", "/a~1b/m~0n/0")).value,
            "selected",
        )
        self.assertEqual(
            resolver.resolve_locator(artifact, ("json_pointer", "/array/1")).value,
            "second",
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_locator(artifact, ("json_pointer", "/a~2b")),
            ResolutionStatus.FAIL,
            "LOCATOR_INVALID",
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_locator(artifact, ("json_pointer", "/array/01")),
            ResolutionStatus.FAIL,
            "LOCATOR_UNRESOLVED",
        )

    def test_archive_reads_only_manifest_verified_members(self) -> None:
        member = b"candidate,relation\nrow-0,review\n"
        raw = archive_bytes({"derived/example.csv": member})
        archive = Rev3ArchiveStore.from_bytes(raw, digest(raw))
        resolver = AuthoritySourceResolver(self.repo, rev3_archive=archive)

        resolved = resolver.resolve_rev3_locator(
            ("archive_member", "derived/example.csv"), digest(member)
        )
        self.assertEqual(resolved.value, member)
        self.assertEqual(resolved.artifact.path, "derived/example.csv")

    def test_archive_failures_are_fail_not_blocked(self) -> None:
        member = b"member"
        valid = archive_bytes({"derived/example.csv": member})
        self.assert_resolution_error(
            lambda: Rev3ArchiveStore.from_bytes(valid, "00" * 32),
            ResolutionStatus.FAIL,
            "REV3_ARCHIVE_DIGEST_MISMATCH",
        )
        self.assert_resolution_error(
            lambda: Rev3ArchiveStore.from_bytes(b"not a zip", digest(b"not a zip")),
            ResolutionStatus.FAIL,
            "REV3_ARCHIVE_INVALID",
        )

        missing = archive_bytes(
            {"derived/example.csv": member},
            manifest_entries=[{"path": "derived/missing.csv", "bytes": 1, "sha256": digest(b"x")}],
        )
        self.assert_resolution_error(
            lambda: Rev3ArchiveStore.from_bytes(missing, digest(missing)),
            ResolutionStatus.FAIL,
            "REV3_MEMBER_MISSING",
        )

        wrong_digest = archive_bytes(
            {"derived/example.csv": member},
            manifest_entries=[
                {"path": "derived/example.csv", "bytes": len(member), "sha256": "00" * 32}
            ],
        )
        self.assert_resolution_error(
            lambda: Rev3ArchiveStore.from_bytes(wrong_digest, digest(wrong_digest)),
            ResolutionStatus.FAIL,
            "REV3_MEMBER_DIGEST_MISMATCH",
        )

        duplicate = archive_bytes(
            {"derived/example.csv": member}, duplicate_member="derived/example.csv"
        )
        self.assert_resolution_error(
            lambda: Rev3ArchiveStore.from_bytes(duplicate, digest(duplicate)),
            ResolutionStatus.FAIL,
            "REV3_MEMBER_DUPLICATE",
        )

    def test_missing_configured_rev3_archive_is_blocked(self) -> None:
        resolver = AuthoritySourceResolver(self.repo, rev3_archive_root=self.repo / "missing")
        binding = SourceBindingDigestV1(
            artifact_role="rev3_source",
            path=REV3_CENSUS_MEMBER,
            schema_or_null=None,
            raw_sha256=bytes.fromhex(
                "82f9312113bb1007ad6562d454c515f85dbc1e0d7a471f7b1c6793725aea45d4"
            ),
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_source_binding(binding),
            ResolutionStatus.BLOCKED,
            "REV3_ARCHIVE_SOURCE_UNAVAILABLE",
        )

    def test_real_static_model_probe_resolves_after_bytes_and_schema_check(self) -> None:
        source_root = ROOT / "sources/m2_5/closures/C"
        raw = (source_root / "declared_interaction_model.v2.json").read_bytes()
        resolver = AuthoritySourceResolver(ROOT)
        binding = SourceBindingDigestV1(
            artifact_role="declared_model",
            path="sources/m2_5/closures/C/declared_interaction_model.v2.json",
            schema_or_null="manafold.m2.5.c.declared-interaction-model.v2",
            raw_sha256=bytes.fromhex(digest(raw)),
        )
        resolved = resolver.resolve_source_binding(binding)
        self.assertEqual(resolved.raw_bytes, raw)
        resolved_json = cast(dict[str, object], resolved.json_value)
        self.assertEqual(resolved_json["schema"], "manafold.m2.5.c.declared-interaction-model.v2")

    def test_content_addressed_acceptance_event_and_roster_leaves_are_verified(self) -> None:
        resolver = AuthoritySourceResolver(self.repo)
        event_path = ROOT / "conformance/fixtures/authority/review_acceptance_event.v1.json"
        event_raw = event_path.read_bytes()
        event = json.loads(event_raw)
        event_id = event["event_id"]
        event_relative = (
            "sources/m2_5/authorities/review_acceptance_events/v1/"
            + event_id.split("/", maxsplit=1)[1]
            + ".json"
        )
        self.write_repo(event_relative, event_raw)
        event_ref = ReviewEventRefV1(
            path=event_relative,
            raw_sha256=bytes.fromhex(digest(event_raw)),
            event_id=event_id,
        )
        resolved_event = resolver.resolve_acceptance_event_leaf(event_ref)
        resolved_event_json = cast(dict[str, object], resolved_event.json_value)
        self.assertEqual(resolved_event_json["event_id"], event_id)
        self.assertEqual(resolved_event.schema_or_null, ACCEPTANCE_EVENT_SCHEMA_V1)
        self.assertEqual(
            resolver.resolve_locator(resolved_event, ("event_id", event_id)).value,
            event,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_locator(resolved_event, ("event_id", "ae.v1/" + "11" * 32)),
            ResolutionStatus.FAIL,
            "ACCEPTANCE_EVENT_ID_MISMATCH",
        )
        for field, value in (("decision", "not_accepted"), ("checklist_id", "other.v1")):
            with self.subTest(field=field):
                changed = dict(event)
                changed[field] = value
                changed_raw = json_bytes(changed)
                self.write_repo(event_relative, changed_raw)
                changed_ref = ReviewEventRefV1(
                    path=event_relative,
                    raw_sha256=bytes.fromhex(digest(changed_raw)),
                    event_id=event_id,
                )

                def resolve_changed_event(reference: ReviewEventRefV1 = changed_ref) -> object:
                    return resolver.resolve_acceptance_event_leaf(reference)

                self.assert_resolution_error(
                    resolve_changed_event,
                    ResolutionStatus.FAIL,
                    "ACCEPTANCE_EVENT_INVALID",
                )

        roster_path = ROOT / "conformance/fixtures/authority/reviewer_roster.v1.json"
        roster_raw = roster_path.read_bytes()
        roster_digest = bytes.fromhex(digest(roster_raw))
        roster_relative = (
            "sources/m2_5/authorities/reviewer_rosters/v1/" + roster_digest.hex() + ".json"
        )
        self.write_repo(roster_relative, roster_raw)
        roster_ref = ReviewerRosterRefV1(
            path=roster_relative,
            schema="manafold.m2.5.c.reviewer-roster.v1",
            raw_sha256=roster_digest,
        )
        resolved_roster = resolver.resolve_reviewer_roster_leaf(roster_ref)
        self.assertEqual(resolved_roster.json_value["schema"], roster_ref.schema)

        tampered = dict(event)
        tampered["event_id"] = "ae.v1/" + "11" * 32
        tampered_raw = json_bytes(tampered)
        self.write_repo(event_relative, tampered_raw)
        tampered_ref = ReviewEventRefV1(
            path=event_relative,
            raw_sha256=bytes.fromhex(digest(tampered_raw)),
            event_id=event_id,
        )
        self.assert_resolution_error(
            lambda: resolver.resolve_acceptance_event_leaf(tampered_ref),
            ResolutionStatus.FAIL,
            "ACCEPTANCE_EVENT_ID_MISMATCH",
        )


if __name__ == "__main__":
    unittest.main()
