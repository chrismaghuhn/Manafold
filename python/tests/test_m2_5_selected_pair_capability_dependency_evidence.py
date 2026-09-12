from __future__ import annotations

import copy
import hashlib
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
        self.evidence_path = "docs/cards/CAPABILITY_MODEL.md"
        self.evidence_sha256 = hashlib.sha256((ROOT / self.evidence_path).read_bytes()).hexdigest()

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

    def test_forged_terminal_and_dependency_claims_are_rejected(self) -> None:
        terminal = copy.deepcopy(self.artifact)
        record = terminal["capabilities"][0]
        record["disposition"] = "TERMINAL_LEAF_EVIDENCED"
        record["unresolved"] = None
        record["terminal_evidence"] = [
            {
                "path": self.evidence_path,
                "raw_sha256": self.evidence_sha256,
                "locator": "forged-terminal",
                "evidence_role": "ACCEPTED_TERMINAL_LEAF",
            }
        ]
        terminal["terminal_leaves"] = [record["capability_family_id"]]
        terminal["summary"]["terminal_leaf_evidenced"] = 1
        terminal["summary"]["unresolved_dependency_evidence"] = 124
        terminal["content_sha256"] = compute_artifact_content_sha256(terminal)
        with self.assertRaisesRegex(ScopeDependencyEvidenceError, "terminal evidence"):
            validate_dependency_evidence_artifact(terminal, ROOT, self.archive_root)

        edge = copy.deepcopy(self.artifact)
        edge["dependency_edges"] = [
            {
                "parent_family_id": edge["direct_roots"][0],
                "child_family_id": edge["direct_roots"][1],
                "dependency_kind": "SEMANTIC_PREREQUISITE",
                "evidence_refs": [
                    {
                        "path": self.evidence_path,
                        "raw_sha256": self.evidence_sha256,
                        "locator": "forged-edge",
                        "evidence_role": "ACCEPTED_DEPENDENCY_EDGE",
                    }
                ],
                "rationale": "forged",
            }
        ]
        edge["content_sha256"] = compute_artifact_content_sha256(edge)
        with self.assertRaisesRegex(ScopeDependencyEvidenceError, "dependency evidence"):
            validate_dependency_evidence_artifact(edge, ROOT, self.archive_root)

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

    def test_selected_pair_binding_mutation_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.artifact)
        mutated["selected_pair"]["deck_names"] = ["Grave Danger", "Token Triumph"]
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeDependencyEvidenceError, "selected-pair"):
            validate_dependency_evidence_artifact(mutated, ROOT, self.archive_root)


if __name__ == "__main__":
    unittest.main()
