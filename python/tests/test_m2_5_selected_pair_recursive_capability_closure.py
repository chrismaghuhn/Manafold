from __future__ import annotations

import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from validate_m2_5_selected_pair_cdi_census import (
    ScopeCensusValidationError,
    compute_artifact_content_sha256,
    compute_dependency_closure,
    load_census_artifacts,
    validate_dependency_edges,
    validate_recursive_capability_closure,
)


class SelectedPairRecursiveCapabilityClosureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.artifacts = load_census_artifacts(ROOT)
        self.capability = self.artifacts["capability"]
        self.closure = self.artifacts["recursive_capability_closure"]
        self.family_ids = {family["family_id"] for family in self.capability["families"]}
        self.evidence_path = "docs/cards/CAPABILITY_MODEL.md"
        self.evidence_sha256 = hashlib.sha256((ROOT / self.evidence_path).read_bytes()).hexdigest()

    def _edge(
        self,
        parent: str = "cap.parent",
        child: str = "cap.child",
        dependency_kind: str = "SEMANTIC_PREREQUISITE",
    ) -> dict[str, object]:
        return {
            "parent_family_id": parent,
            "child_family_id": child,
            "dependency_kind": dependency_kind,
            "evidence_refs": [
                {
                    "path": self.evidence_path,
                    "raw_sha256": self.evidence_sha256,
                    "locator": f"dependency-edge:{parent}->{child}",
                    "evidence_role": "ACCEPTED_DEPENDENCY_EDGE",
                }
            ],
            "rationale": "The accepted evidence explicitly requires the child capability.",
        }

    def test_current_closure_preserves_exact_blocked_root_envelope(self) -> None:
        validate_recursive_capability_closure(self.closure, self.capability, ROOT)
        self.assertEqual(self.closure["status"], "BLOCKED")
        self.assertEqual(len(self.closure["direct_roots"]), 125)
        self.assertEqual(
            {record["family_id"] for record in self.closure["root_classifications"]},
            set(self.closure["direct_roots"]),
        )
        self.assertTrue(
            all(
                record["state"] == "BLOCKED_MISSING_DEPENDENCY_EVIDENCE"
                for record in self.closure["root_classifications"]
            )
        )
        self.assertEqual(self.closure["dependency_edges"], [])
        self.assertEqual(self.closure["terminal_leaves"], [])
        self.assertEqual(self.closure["missing_capabilities"], [])
        self.assertEqual(self.closure["cycles"], [])
        self.assertEqual(
            self.closure["record_counts"]["unresolved_dependency_obligations"],
            125,
        )

    def test_unknown_parent_is_rejected(self) -> None:
        with self.assertRaisesRegex(ScopeCensusValidationError, "unknown parent"):
            validate_dependency_edges(
                [self._edge("cap.unknown", "cap.child")],
                {"cap.child"},
                ROOT,
            )

    def test_unknown_child_is_rejected(self) -> None:
        with self.assertRaisesRegex(ScopeCensusValidationError, "unknown child"):
            validate_dependency_edges(
                [self._edge("cap.parent", "cap.unknown")],
                {"cap.parent"},
                ROOT,
            )

    def test_duplicate_and_conflicting_edges_are_rejected(self) -> None:
        edge = self._edge("cap.parent", "cap.child")
        with self.assertRaisesRegex(ScopeCensusValidationError, "duplicate dependency edge"):
            validate_dependency_edges(
                [edge, copy.deepcopy(edge)], {"cap.parent", "cap.child"}, ROOT
            )

        conflicting = self._edge("cap.parent", "cap.child", "STATE_MODEL_PREREQUISITE")
        with self.assertRaisesRegex(ScopeCensusValidationError, "conflicting dependency edge"):
            validate_dependency_edges([edge, conflicting], {"cap.parent", "cap.child"}, ROOT)

    def test_self_edge_is_rejected(self) -> None:
        with self.assertRaisesRegex(ScopeCensusValidationError, "self-dependency"):
            validate_dependency_edges(
                [self._edge("cap.parent", "cap.parent")], {"cap.parent"}, ROOT
            )

    def test_cycle_reports_full_path(self) -> None:
        edges = [
            self._edge("cap.a", "cap.b"),
            self._edge("cap.b", "cap.c"),
            self._edge("cap.c", "cap.a"),
        ]
        canonical = validate_dependency_edges(edges, {"cap.a", "cap.b", "cap.c"}, ROOT)
        result = compute_dependency_closure(["cap.a"], canonical)
        self.assertEqual(result["cycles"], [["cap.a", "cap.b", "cap.c", "cap.a"]])

    def test_dependency_direction_is_parent_requires_child(self) -> None:
        edge = self._edge("cap.parent", "cap.child")
        canonical = validate_dependency_edges([edge], {"cap.parent", "cap.child"}, ROOT)
        result = compute_dependency_closure(["cap.parent"], canonical)
        self.assertEqual(result["resolved_families"], ["cap.child", "cap.parent"])
        self.assertEqual(result["transitive_only_families"], ["cap.child"])

    def test_b2_catalog_space_allows_transitive_only_family(self) -> None:
        catalog = {
            item["family_id"]
            for item in json.loads(
                (ROOT / "sources/m2_5/closures/B2/requirement_family_catalog.v1.json").read_text(
                    encoding="utf-8"
                )
            )["families"]
        }
        direct = next(iter(self.family_ids))
        transitive_only = next(item for item in sorted(catalog) if item not in self.family_ids)
        edge = self._edge(direct, transitive_only)
        canonical = validate_dependency_edges([edge], catalog, ROOT)
        result = compute_dependency_closure([direct], canonical)
        self.assertIn(transitive_only, result["transitive_only_families"])

    def test_full_artifact_rejects_noncanonical_stored_edge_order(self) -> None:
        mutated = copy.deepcopy(self.closure)
        roots = mutated["direct_roots"]
        parent, child_a, child_b = roots[:3]

        def edge(child: str) -> dict[str, object]:
            return {
                "parent_family_id": parent,
                "child_family_id": child,
                "dependency_kind": "SEMANTIC_PREREQUISITE",
                "evidence_refs": [
                    {
                        "path": self.evidence_path,
                        "raw_sha256": self.evidence_sha256,
                        "locator": f"dependency-edge:{parent}->{child}",
                        "evidence_role": "ACCEPTED_DEPENDENCY_EDGE",
                    }
                ],
                "rationale": "Synthetic validator fixture.",
            }

        mutated["dependency_edges"] = [edge(child_b), edge(child_a)]
        mutated["root_classifications"][0]["state"] = "HAS_ACCEPTED_DEPENDENCIES"
        mutated["records"] = mutated["root_classifications"]
        mutated["blocked_families"] = roots[1:]
        mutated["unresolved_scope_obligations"] = [
            item for item in mutated["unresolved_scope_obligations"] if item["subject"] != parent
        ]
        mutated["record_counts"].update(
            {
                "explicit_dependency_edges": 2,
                "blocked_families": len(roots) - 1,
                "unresolved_dependency_obligations": len(roots) - 1,
            }
        )
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeCensusValidationError, "edge ordering"):
            validate_recursive_capability_closure(mutated, self.capability, ROOT)

    def test_canonical_order_is_independent_of_input_order(self) -> None:
        edges = [
            self._edge("cap.a", "cap.c"),
            self._edge("cap.a", "cap.b"),
        ]
        first = validate_dependency_edges(edges, {"cap.a", "cap.b", "cap.c"}, ROOT)
        second = validate_dependency_edges(list(reversed(edges)), {"cap.a", "cap.b", "cap.c"}, ROOT)
        self.assertEqual(first, second)
        self.assertEqual(
            compute_dependency_closure(["cap.a"], first),
            compute_dependency_closure(["cap.a"], second),
        )

    def test_missing_or_mutated_edge_evidence_is_rejected(self) -> None:
        missing = self._edge()
        missing["evidence_refs"] = []
        with self.assertRaisesRegex(ScopeCensusValidationError, "evidence"):
            validate_dependency_edges([missing], {"cap.parent", "cap.child"}, ROOT)

        mutated = self._edge()
        mutated["evidence_refs"][0]["raw_sha256"] = "0" * 64
        with self.assertRaisesRegex(ScopeCensusValidationError, "evidence"):
            validate_dependency_edges([mutated], {"cap.parent", "cap.child"}, ROOT)

        wrong_role = self._edge()
        wrong_role["evidence_refs"][0]["evidence_role"] = "FORGED_ROLE"
        with self.assertRaisesRegex(ScopeCensusValidationError, "evidence role"):
            validate_dependency_edges([wrong_role], {"cap.parent", "cap.child"}, ROOT)

        wrong_locator = self._edge()
        wrong_locator["evidence_refs"][0]["locator"] = "wrong-locator"
        with self.assertRaisesRegex(ScopeCensusValidationError, "locator"):
            validate_dependency_edges([wrong_locator], {"cap.parent", "cap.child"}, ROOT)

    def test_deleting_unresolved_obligations_does_not_promote_pass(self) -> None:
        mutated = copy.deepcopy(self.artifacts)
        closure = mutated["recursive_capability_closure"]
        closure["status"] = "PASS"
        closure["unresolved_scope_obligations"] = []
        closure["record_counts"]["unresolved_dependency_obligations"] = 0
        closure["content_sha256"] = compute_artifact_content_sha256(closure)
        with self.assertRaisesRegex(ScopeCensusValidationError, "blocked root"):
            validate_recursive_capability_closure(closure, mutated["capability"], ROOT)

    def test_terminal_promotion_without_evidence_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.closure)
        first = mutated["root_classifications"][0]
        first["state"] = "TERMINAL_LEAF"
        mutated["records"] = mutated["root_classifications"]
        mutated["blocked_families"] = mutated["blocked_families"][1:]
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeCensusValidationError, "terminal-leaf evidence"):
            validate_recursive_capability_closure(mutated, self.capability, ROOT)

    def test_forged_terminal_role_cannot_promote_pass(self) -> None:
        mutated = copy.deepcopy(self.artifacts)
        closure = mutated["recursive_capability_closure"]
        roots = closure["direct_roots"]
        for record in closure["root_classifications"]:
            record["state"] = "TERMINAL_LEAF"
        closure["status"] = "PASS"
        closure["accepted_terminal_leaf_evidence"] = [
            {
                "path": self.evidence_path,
                "raw_sha256": self.evidence_sha256,
                "locator": "forged-terminal-locator",
                "evidence_role": "ACCEPTED_TERMINAL_LEAF",
            }
        ]
        closure["terminal_leaves"] = roots
        closure["blocked_families"] = []
        closure["unresolved_scope_obligations"] = []
        closure["record_counts"].update(
            {"terminal_leaves": 125, "blocked_families": 0, "unresolved_dependency_obligations": 0}
        )
        closure["records"] = closure["root_classifications"]
        closure["content_sha256"] = compute_artifact_content_sha256(closure)
        with self.assertRaisesRegex(ScopeCensusValidationError, "terminal-leaf evidence contract"):
            validate_recursive_capability_closure(closure, mutated["capability"], ROOT)

    def test_accepted_dependency_state_requires_an_edge(self) -> None:
        mutated = copy.deepcopy(self.closure)
        mutated["root_classifications"][0]["state"] = "HAS_ACCEPTED_DEPENDENCIES"
        mutated["records"] = mutated["root_classifications"]
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeCensusValidationError, "has no edges"):
            validate_recursive_capability_closure(mutated, self.capability, ROOT)

    def test_terminal_leaf_cannot_also_be_blocked(self) -> None:
        mutated = copy.deepcopy(self.closure)
        first_id = mutated["direct_roots"][0]
        mutated["root_classifications"][0]["state"] = "TERMINAL_LEAF"
        mutated["terminal_leaves"] = [first_id]
        mutated["terminal_leaves"][0] = first_id
        mutated["records"] = mutated["root_classifications"]
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeCensusValidationError, "both terminal and blocked"):
            validate_recursive_capability_closure(mutated, self.capability, ROOT)

    def test_missing_semantic_owner_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.closure)
        mutated["root_classifications"][0]["semantic_owner_roles"] = ["FORGED_OWNER"]
        mutated["records"] = mutated["root_classifications"]
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeCensusValidationError, "semantic owner"):
            validate_recursive_capability_closure(mutated, self.capability, ROOT)

    def test_capability_census_binding_mutation_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.closure)
        mutated["selected_capability_census"]["content_sha256"] = "0" * 64
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)
        with self.assertRaisesRegex(ScopeCensusValidationError, "capability census"):
            validate_recursive_capability_closure(mutated, self.capability, ROOT)

    def test_family_and_record_payload_mutations_are_rejected(self) -> None:
        family_mutation = copy.deepcopy(self.closure)
        family_mutation["families"][0]["canonical_name"] = "forged-family"
        family_mutation["content_sha256"] = compute_artifact_content_sha256(family_mutation)
        with self.assertRaisesRegex(ScopeCensusValidationError, "family records"):
            validate_recursive_capability_closure(family_mutation, self.capability, ROOT)

        record_mutation = copy.deepcopy(self.closure)
        record_mutation["records"][0]["state"] = "TERMINAL_LEAF"
        record_mutation["content_sha256"] = compute_artifact_content_sha256(record_mutation)
        with self.assertRaisesRegex(ScopeCensusValidationError, "root classifications"):
            validate_recursive_capability_closure(record_mutation, self.capability, ROOT)


if __name__ == "__main__":
    unittest.main()
