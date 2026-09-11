from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from shutil import copytree
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "src"))

from authority_source_resolver import ResolutionError, ResolutionStatus
from b2_closure_source_resolver import (
    B2ClosureResolutionMode,
    B2ClosureSourceBinding,
    B2ClosureV2VerificationError,
    binding_for_mode,
    resolve_b2_closure,
    resolve_b2_closure_binding,
)
from mtgml.authority import ReviewAuthorityArtifactRoleV4
from mtgml.b2_closure_contract import (
    B2_CLOSURE_SOURCE_PACKAGE_SHA256,
    B2_CLOSURE_V1_PATH,
    B2_CLOSURE_V1_RAW_SHA256,
    B2_CLOSURE_V1_SCHEMA,
    B2_CLOSURE_V2_PATH,
    B2_CLOSURE_V2_SCHEMA,
)


class B2ClosureSourceResolverTests(unittest.TestCase):
    def test_slice3_source_resolver_module_exists(self) -> None:
        self.assertIsNotNone(importlib.util.find_spec("b2_closure_source_resolver"))

    def test_historical_v1_resolves_exact_immutable_binding(self) -> None:
        result = resolve_b2_closure(ROOT, B2ClosureResolutionMode.HISTORICAL_V1)

        self.assertEqual(result.mode, B2ClosureResolutionMode.HISTORICAL_V1)
        self.assertEqual(result.binding.artifact_role, "b2_closure")
        self.assertEqual(result.binding.closure_version, "v1")
        self.assertEqual(result.binding.repository_relative_path, B2_CLOSURE_V1_PATH)
        self.assertEqual(result.binding.schema_identifier, B2_CLOSURE_V1_SCHEMA)
        self.assertEqual(result.binding.raw_sha256, B2_CLOSURE_V1_RAW_SHA256)
        self.assertEqual(
            result.raw_bytes,
            (ROOT / Path(*B2_CLOSURE_V1_PATH.split("/"))).read_bytes(),
        )

    def test_candidate_v2_resolves_exact_verified_binding_without_currentness(self) -> None:
        result = resolve_b2_closure(ROOT, B2ClosureResolutionMode.CANDIDATE_V2)

        self.assertEqual(result.mode, B2ClosureResolutionMode.CANDIDATE_V2)
        self.assertEqual(result.binding.artifact_role, "b2_closure_v2")
        self.assertEqual(result.binding.closure_version, "v2")
        self.assertEqual(result.binding.repository_relative_path, B2_CLOSURE_V2_PATH)
        self.assertEqual(result.binding.schema_identifier, B2_CLOSURE_V2_SCHEMA)
        self.assertEqual(
            result.raw_sha256,
            "43a0613be367159ffd224d6282deb4e263b2147b861acb90a26388d56729f748",
        )
        self.assertEqual(result.binding.source_package_sha256, B2_CLOSURE_SOURCE_PACKAGE_SHA256)
        self.assertTrue(result.v2_verified)
        self.assertFalse(result.v2_current)

    def test_coexisting_files_are_selected_only_by_explicit_request_and_read_only(self) -> None:
        v1_path = ROOT / Path(*B2_CLOSURE_V1_PATH.split("/"))
        v2_path = ROOT / Path(*B2_CLOSURE_V2_PATH.split("/"))
        before = (v1_path.read_bytes(), v2_path.read_bytes())

        first_v1 = resolve_b2_closure(ROOT, B2ClosureResolutionMode.HISTORICAL_V1)
        second_v1 = resolve_b2_closure(ROOT, B2ClosureResolutionMode.HISTORICAL_V1)
        first_v2 = resolve_b2_closure(ROOT, B2ClosureResolutionMode.CANDIDATE_V2)
        second_v2 = resolve_b2_closure(ROOT, B2ClosureResolutionMode.CANDIDATE_V2)

        self.assertEqual(first_v1, second_v1)
        self.assertEqual(first_v2, second_v2)
        self.assertEqual(first_v1.binding.artifact_role, "b2_closure")
        self.assertEqual(first_v2.binding.artifact_role, "b2_closure_v2")
        self.assertEqual(before, (v1_path.read_bytes(), v2_path.read_bytes()))
        self.assertNotIn("b2_closure_v2", {role.value for role in ReviewAuthorityArtifactRoleV4})

    def test_role_and_version_matrix_is_closed(self) -> None:
        v1 = binding_for_mode(B2ClosureResolutionMode.HISTORICAL_V1)
        v2 = binding_for_mode(B2ClosureResolutionMode.CANDIDATE_V2)

        self._assert_failure(
            replace(v1, artifact_role="b2_closure_v2"), "CURRENT_ROOT_ROLE_MISMATCH"
        )
        self._assert_failure(replace(v2, artifact_role="b2_closure"), "CURRENT_ROOT_ROLE_MISMATCH")
        self._assert_failure(replace(v1, artifact_role="b2_closure_v1"), "UNKNOWN_SOURCE_ROLE")
        self._assert_failure(replace(v2, artifact_role="b2_closure_v3"), "UNKNOWN_SOURCE_ROLE")
        self._assert_failure(replace(v1, closure_version="v3"), "UNSUPPORTED_CLOSURE_VERSION")
        self._assert_failure(replace(v1, closure_version="latest"), "UNKNOWN_CLOSURE_VERSION")

    def test_path_schema_digest_and_source_package_bindings_are_exact(self) -> None:
        v1 = binding_for_mode(B2ClosureResolutionMode.HISTORICAL_V1)
        self._assert_failure(
            replace(v1, repository_relative_path="sources/../sources/x"),
            "CURRENT_ROOT_PATH_MISMATCH",
        )
        self._assert_failure(
            replace(v1, repository_relative_path=B2_CLOSURE_V2_PATH),
            "CURRENT_ROOT_PATH_MISMATCH",
        )
        self._assert_failure(
            replace(v1, schema_identifier=B2_CLOSURE_V2_SCHEMA),
            "CURRENT_ROOT_SCHEMA_MISMATCH",
        )
        self._assert_failure(
            replace(v1, raw_sha256="g" * 64), "CURRENT_ROOT_DIGEST_MISMATCH"
        )
        self._assert_failure(
            replace(v1, raw_sha256=v1.raw_sha256.upper()), "CURRENT_ROOT_DIGEST_MISMATCH"
        )
        self._assert_failure(replace(v1, raw_sha256="0" * 64), "CURRENT_ROOT_DIGEST_MISMATCH")
        self._assert_failure(replace(v1, source_package_sha256="0" * 64), "SOURCE_PACKAGE_MISMATCH")

    def test_explicit_binding_resolution_requires_the_same_closed_contract(self) -> None:
        v1 = binding_for_mode(B2ClosureResolutionMode.HISTORICAL_V1)
        result = resolve_b2_closure_binding(ROOT, v1)
        self.assertEqual(result.binding, v1)

    def test_tampered_v1_and_v2_bytes_fail_without_fallback(self) -> None:
        for mode, relative_path in (
            (B2ClosureResolutionMode.HISTORICAL_V1, B2_CLOSURE_V1_PATH),
            (B2ClosureResolutionMode.CANDIDATE_V2, B2_CLOSURE_V2_PATH),
        ):
            with self.subTest(mode=mode), self._copied_repo() as repo:
                path = repo / Path(*relative_path.split("/"))
                path.write_bytes(path.read_bytes() + b"\n")
                with self.assertRaises(ResolutionError) as failure:
                    resolve_b2_closure(repo, mode)
                self.assertEqual(failure.exception.status, ResolutionStatus.FAIL)
                self.assertEqual(failure.exception.code, "SOURCE_DIGEST_MISMATCH")

    def test_missing_artifact_never_retries_the_other_version(self) -> None:
        for mode, relative_path, other_mode in (
            (
                B2ClosureResolutionMode.CANDIDATE_V2,
                B2_CLOSURE_V2_PATH,
                B2ClosureResolutionMode.HISTORICAL_V1,
            ),
            (
                B2ClosureResolutionMode.HISTORICAL_V1,
                B2_CLOSURE_V1_PATH,
                B2ClosureResolutionMode.CANDIDATE_V2,
            ),
        ):
            with self.subTest(mode=mode), self._copied_repo() as repo:
                (repo / Path(*relative_path.split("/"))).unlink()
                with self.assertRaises(ResolutionError) as failure:
                    resolve_b2_closure(repo, mode)
                self.assertEqual(failure.exception.code, "REPOSITORY_SOURCE_MISSING")
                expected_role = (
                    "b2_closure"
                    if other_mode is B2ClosureResolutionMode.HISTORICAL_V1
                    else "b2_closure_v2"
                )
                self.assertEqual(
                    resolve_b2_closure(repo, other_mode).binding.artifact_role,
                    expected_role,
                )

    def test_invalid_v2_internal_closure_fails_without_retry(self) -> None:
        with patch(
            "b2_closure_source_resolver.verify_closure_v2",
            side_effect=B2ClosureV2VerificationError("invalid internal closure"),
        ), self.assertRaises(ResolutionError) as failure:
            resolve_b2_closure(ROOT, B2ClosureResolutionMode.CANDIDATE_V2)
        self.assertEqual(failure.exception.code, "B2_CLOSURE_INVALID")

    def test_v1_historical_contract_and_current_consumers_are_unchanged(self) -> None:
        self.assertEqual(
            B2_CLOSURE_V1_PATH,
            "sources/m2_5/closures/B2/classification_closure.v1.json",
        )
        self.assertEqual(B2_CLOSURE_V1_SCHEMA, "manafold.m2.5.b2.classification-closure.v1")
        self.assertEqual(
            B2_CLOSURE_V1_RAW_SHA256,
            "ed6a0bf4b0eb83c85027fdcc61eaf32bfa7bb06d4de78c77d0946d87212e7d43",
        )

    def _assert_failure(self, binding: B2ClosureSourceBinding, expected_code: str) -> None:
        with self.assertRaises(ResolutionError) as failure:
            resolve_b2_closure_binding(ROOT, binding)
        self.assertEqual(failure.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(failure.exception.code, expected_code)

    def _copied_repo(self) -> _TemporaryRepo:
        directory = tempfile.TemporaryDirectory()
        repo = Path(directory.name)
        copytree(ROOT / "sources", repo / "sources")
        copytree(ROOT / "schemas", repo / "schemas")
        return _TemporaryRepo(directory)


class _TemporaryRepo:
    def __init__(self, directory: tempfile.TemporaryDirectory[str]) -> None:
        self._directory = directory
        self.repo = Path(directory.name)

    def __enter__(self) -> Path:
        return self.repo

    def __exit__(self, *args: object) -> None:
        self._directory.cleanup()


if __name__ == "__main__":
    unittest.main()
