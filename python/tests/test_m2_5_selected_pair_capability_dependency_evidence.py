from __future__ import annotations

import copy
import os
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
import sys

sys.path.insert(0, str(ROOT / "scripts"))

from validate_m2_5_selected_pair_capability_dependency_evidence import (
    EVIDENCE_SCHEMA_ID,
    ScopeDependencyEvidenceError,
    build_dependency_evidence_artifact,
    compute_artifact_content_sha256,
    load_dependency_evidence_artifact,
    validate_dependency_evidence_artifact,
)


class SelectedPairCapabilityDependencyEvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.configured = os.environ.get("MANAFOLD_SOURCE_ARCHIVE")
        if not self.configured:
            self.skipTest("MANAFOLD_SOURCE_ARCHIVE is not configured")
        self.artifact = load_dependency_evidence_artifact(ROOT)
        self.archive_root = Path(self.configured)

    def test_exact_root_coverage_and_unresolved_dispositions(self) -> None:
        validate_dependency_evidence_artifact(self.artifact, ROOT, self.archive_root)
        self.assertEqual(self.artifact["schema"], EVIDENCE_SCHEMA_ID)
        self.assertEqual(self.artifact["status"], "PASS")
        self.assertEqual(len(self.artifact["direct_roots"]), 125)
        self.assertEqual(len(self.artifact["capabilities"]), 125)
        self.assertEqual(
            {record["disposition"] for record in self.artifact["capabilities"]},
            {"UNRESOLVED_DEPENDENCY_EVIDENCE"},
        )
        self.assertEqual(self.artifact["dependency_edges"], [])
        self.assertEqual(self.artifact["terminal_leaves"], [])
        self.assertEqual(len(self.artifact["unresolved_scope_obligations"]), 125)

    def test_builder_is_deterministic(self) -> None:
        first = build_dependency_evidence_artifact(ROOT, self.archive_root)
        second = build_dependency_evidence_artifact(ROOT, self.archive_root)
        self.assertEqual(first, second)

    def test_terminal_promotion_without_terminal_evidence_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.artifact)
        record = mutated["capabilities"][0]
        record["disposition"] = "TERMINAL_LEAF_EVIDENCED"
        record["unresolved"] = None
        mutated["terminal_leaves"] = [record["capability_family_id"]]
        mutated["summary"]["terminal_leaf_evidenced"] = 1
        mutated["summary"]["unresolved_dependency_evidence"] = 124
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeDependencyEvidenceError, "terminal evidence"):
            validate_dependency_evidence_artifact(mutated, ROOT, self.archive_root)

    def test_dependency_edge_without_evidence_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.artifact)
        root = mutated["direct_roots"][0]
        child = mutated["direct_roots"][1]
        mutated["dependency_edges"] = [
            {
                "parent_family_id": root,
                "child_family_id": child,
                "dependency_kind": "SEMANTIC_PREREQUISITE",
                "evidence_refs": [],
                "rationale": "forged",
            }
        ]
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeDependencyEvidenceError, "schema|evidence"):
            validate_dependency_evidence_artifact(mutated, ROOT, self.archive_root)

    def test_root_evidence_locator_mutation_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.artifact)
        mutated["capabilities"][0]["unresolved"]["evidence_refs"][0]["locator"] = "forged-locator"
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeDependencyEvidenceError, "root evidence"):
            validate_dependency_evidence_artifact(mutated, ROOT, self.archive_root)

    def test_input_binding_mutation_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.artifact)
        mutated["input_bindings"]["capability_census"]["content_sha256"] = "0" * 64
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(
            ScopeDependencyEvidenceError, "input binding|capability census"
        ):
            validate_dependency_evidence_artifact(mutated, ROOT, self.archive_root)

    def test_third_root_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.artifact)
        mutated["direct_roots"].append("cap.third-deck-forged")
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeDependencyEvidenceError, "root set"):
            validate_dependency_evidence_artifact(mutated, ROOT, self.archive_root)


if __name__ == "__main__":
    unittest.main()
