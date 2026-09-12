from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from shutil import copytree

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "src"))

from authority_source_resolver import ResolutionError
from b2_closure_current_root import (
    B2_CURRENT_ROOT_PATH,
    B2CurrentRootError,
    load_current_b2_closure_root,
    resolve_current_b2_closure,
)
from b2_closure_source_resolver import (
    B2ClosureResolutionMode,
    resolve_b2_closure,
)


class B2ClosureCurrentRootAdoptionTests(unittest.TestCase):
    def test_sole_root_loads_exact_v2_binding(self) -> None:
        root = load_current_b2_closure_root(ROOT)

        self.assertEqual(root.artifact_role, "b2_closure_v2")
        self.assertEqual(root.closure_version, "v2")
        self.assertEqual(
            root.repository_relative_path,
            "sources/m2_5/closures/B2/classification_closure.v2.json",
        )
        self.assertEqual(
            root.closure_raw_sha256,
            "43a0613be367159ffd224d6282deb4e263b2147b861acb90a26388d56729f748",
        )

    def test_current_construction_resolves_verified_v2(self) -> None:
        result = resolve_current_b2_closure(ROOT)

        self.assertEqual(result.root.artifact_role, "b2_closure_v2")
        self.assertEqual(result.resolution.mode, B2ClosureResolutionMode.CANDIDATE_V2)
        self.assertTrue(result.resolution.v2_verified)
        self.assertTrue(result.current)
        self.assertTrue(result.resolution.v2_current)
        self.assertEqual(result.resolution.raw_sha256, result.root.closure_raw_sha256)

        candidate = resolve_b2_closure(ROOT, B2ClosureResolutionMode.CANDIDATE_V2)
        self.assertFalse(candidate.v2_current)

    def test_historical_v1_remains_independent_of_current_root(self) -> None:
        result = resolve_b2_closure(ROOT, B2ClosureResolutionMode.HISTORICAL_V1)

        self.assertEqual(result.binding.artifact_role, "b2_closure")
        self.assertEqual(result.binding.closure_version, "v1")
        self.assertFalse(result.v2_current)

    def test_missing_root_fails_without_v1_fallback(self) -> None:
        with self._copied_repo() as repo:
            (repo / B2_CURRENT_ROOT_PATH).unlink()
            with self.assertRaises(B2CurrentRootError) as failure:
                resolve_current_b2_closure(repo)
            self.assertEqual(failure.exception.code, "CURRENT_ROOT_MISSING")
            self.assertEqual(
                resolve_b2_closure(
                    repo, B2ClosureResolutionMode.HISTORICAL_V1
                ).binding.artifact_role,
                "b2_closure",
            )

    def test_multiple_roots_fail_closed(self) -> None:
        with self._copied_repo() as repo:
            extra = repo / "sources/m2_5/closures/B2/current_root.v1.json"
            extra.write_bytes((repo / B2_CURRENT_ROOT_PATH).read_bytes())
            with self.assertRaises(B2CurrentRootError) as failure:
                load_current_b2_closure_root(repo)
            self.assertEqual(failure.exception.code, "MULTIPLE_CURRENT_ROOTS")

    def test_wrong_root_values_fail_closed(self) -> None:
        mutations = (
            ("artifact_role", "b2_closure"),
            ("artifact_role", "b2_closure_v1"),
            ("artifact_role", "latest"),
            ("closure_version", "v1"),
            ("closure_version", "v3"),
            ("closure_version", "v99"),
            ("repository_relative_path", "wrong/path"),
            ("closure_schema_id", "wrong.schema"),
            ("closure_raw_sha256", "0" * 64),
            (
                "closure_raw_sha256",
                "43A0613BE367159FFD224D6282DEB4E263B2147B861ACB90A26388D56729F748",
            ),
            ("source_package_sha256", "0" * 64),
            ("unexpected", True),
        )
        for field, value in mutations:
            with self.subTest(field=field), self._copied_repo() as repo:
                root_path = repo / B2_CURRENT_ROOT_PATH
                root = json.loads(root_path.read_text(encoding="utf-8"))
                root[field] = value
                root_path.write_text(json.dumps(root), encoding="utf-8")
                with self.assertRaises(B2CurrentRootError):
                    resolve_current_b2_closure(repo)

    def test_tampered_v2_bytes_fail_without_v1_fallback(self) -> None:
        with self._copied_repo() as repo:
            v2_path = repo / "sources/m2_5/closures/B2/classification_closure.v2.json"
            v2_path.write_bytes(v2_path.read_bytes() + b"\n")
            with self.assertRaises(ResolutionError):
                resolve_current_b2_closure(repo)
            self.assertEqual(
                resolve_b2_closure(
                    repo, B2ClosureResolutionMode.HISTORICAL_V1
                ).binding.artifact_role,
                "b2_closure",
            )

    def test_slice5_evidence_bytes_are_unchanged(self) -> None:
        relative = "sources/m2_5/closures/B2/verification/b2_closure_v2_adoption_readiness.v1.json"
        import subprocess

        baseline = subprocess.run(
            ["git", "show", f"869d028d900c0876afc4d6cf0c508e98a37a7d31:{relative}"],
            cwd=ROOT,
            check=True,
            capture_output=True,
        ).stdout
        self.assertEqual((ROOT / relative).read_bytes(), baseline)

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
