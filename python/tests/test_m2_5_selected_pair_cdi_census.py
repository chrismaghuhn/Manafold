from __future__ import annotations

import copy
import csv
import json
import os
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from validate_m2_5_selected_pair_cdi_census import (
    EXPECTED_SOURCE_PACKAGE_SHA256,
    ScopeCensusValidationError,
    build_census_artifacts,
    compute_artifact_content_sha256,
    load_census_artifacts,
    validate_b2_projection_rows,
    validate_census_set,
    validate_generated_object_records,
)


class SelectedPairCdiCensusTests(unittest.TestCase):
    def setUp(self) -> None:
        self.artifacts = load_census_artifacts(ROOT)

    def test_selected_pair_counts_and_bindings_are_exact(self) -> None:
        validate_census_set(self.artifacts, ROOT)

        capability = self.artifacts["capability"]
        self.assertEqual(capability["source_package_sha256"], EXPECTED_SOURCE_PACKAGE_SHA256)
        self.assertEqual(capability["record_counts"]["selected_deck_rows"], 144)
        self.assertEqual(capability["record_counts"]["selected_oracle_identities"], 140)
        self.assertEqual(capability["record_counts"]["capability_families"], 125)
        self.assertEqual(capability["record_counts"]["capability_assignment_edges"], 628)
        self.assertEqual(
            {item["deck_name"] for item in capability["selected_decks"]},
            {"Token Triumph", "Grave Danger"},
        )
        zone_identity = next(
            item
            for item in self.artifacts["generated_object"]["records"]
            if item["object_class_id"] == "new_zone_incarnation"
        )
        self.assertEqual(zone_identity["source_card_osis"], [])
        self.assertEqual(zone_identity["owning_capability_families"], [])
        self.assertEqual(zone_identity["source_binding_mode"], "SCOPE_INVARIANT")
        self.assertEqual(self.artifacts["recursive_capability_closure"]["status"], "BLOCKED")

    def test_all_artifact_content_digests_recompute(self) -> None:
        for artifact in self.artifacts.values():
            self.assertEqual(artifact["content_sha256"], compute_artifact_content_sha256(artifact))

    def test_third_deck_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.artifacts)
        mutated["capability"]["selected_decks"].append(
            {"deck_id": "m2-5/third-deck", "deck_name": "Third Deck", "player": 3}
        )

        with self.assertRaisesRegex(ScopeCensusValidationError, "selected pair"):
            validate_census_set(mutated, ROOT)

    def test_recursive_closure_pass_promotion_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.artifacts)
        mutated["recursive_capability_closure"]["status"] = "PASS"
        mutated["recursive_capability_closure"]["content_sha256"] = compute_artifact_content_sha256(
            mutated["recursive_capability_closure"]
        )

        with self.assertRaisesRegex(
            ScopeCensusValidationError, "recursive closure must remain BLOCKED"
        ):
            validate_census_set(mutated, ROOT)

    def test_b2_and_source_identity_bindings_are_required(self) -> None:
        configured = os.environ.get("MANAFOLD_SOURCE_ARCHIVE")
        if not configured:
            self.skipTest("MANAFOLD_SOURCE_ARCHIVE is not configured")
        mutated = copy.deepcopy(self.artifacts)
        mutated["capability"]["records"][0]["source_record_raw_sha256"] = "0" * 64
        mutated["capability"]["content_sha256"] = compute_artifact_content_sha256(
            mutated["capability"]
        )

        with self.assertRaisesRegex(ScopeCensusValidationError, "source raw digest"):
            validate_census_set(mutated, ROOT, Path(configured))

    def test_private_source_binding_when_configured(self) -> None:
        configured = os.environ.get("MANAFOLD_SOURCE_ARCHIVE")
        if not configured:
            self.skipTest("MANAFOLD_SOURCE_ARCHIVE is not configured")
        validate_census_set(self.artifacts, ROOT, Path(configured))

    def test_b2_projection_join_rejects_digest_mutation(self) -> None:
        projection_path = ROOT / "sources/m2_5/closures/B2/deck_row_classification_refs.v1.csv"
        with projection_path.open(encoding="utf-8", newline="") as handle:
            projection_rows = list(csv.DictReader(handle))
        classifications = {
            item["oracle_semantic_identity"]: item
            for item in json.loads(
                (ROOT / "sources/m2_5/closures/B2/card_semantic_classifications.v1.json").read_text(
                    encoding="utf-8"
                )
            )["classifications"]
        }
        selected = [
            row for row in projection_rows if row["deck_id"] in {"Token Triumph", "Grave Danger"}
        ]
        selected[0]["terminal_classification_identity"] = "0" * 64

        with self.assertRaisesRegex(ScopeCensusValidationError, "projection"):
            validate_b2_projection_rows(selected, classifications, self.artifacts["capability"])

    def test_b2_projection_join_rejects_census_assignment_mutation(self) -> None:
        mutated = copy.deepcopy(self.artifacts)
        assignment = mutated["capability"]["records"][0]["capability_assignments"][0]
        assignment["family_id"] = (
            "cap.draw" if assignment["family_id"] != "cap.draw" else "cap.aura"
        )
        mutated["capability"]["content_sha256"] = compute_artifact_content_sha256(
            mutated["capability"]
        )

        with self.assertRaisesRegex(ScopeCensusValidationError, "projection/census assignment"):
            validate_census_set(mutated, ROOT)

    def test_generated_object_source_join_rejects_token_mutation(self) -> None:
        configured = os.environ.get("MANAFOLD_SOURCE_ARCHIVE")
        if not configured:
            self.skipTest("MANAFOLD_SOURCE_ARCHIVE is not configured")
        mutated = copy.deepcopy(self.artifacts["generated_object"])
        token = next(item for item in mutated["records"] if item["object_kind"] == "TOKEN_CLASS")
        token["name"] = "forged-token-name"
        token["content_sha256"] = compute_artifact_content_sha256(mutated)

        with self.assertRaisesRegex(ScopeCensusValidationError, "all_parts"):
            validate_generated_object_records(mutated, ROOT, Path(configured))

    def test_generated_object_source_join_rejects_missing_binding(self) -> None:
        configured = os.environ.get("MANAFOLD_SOURCE_ARCHIVE")
        if not configured:
            self.skipTest("MANAFOLD_SOURCE_ARCHIVE is not configured")
        mutated = copy.deepcopy(self.artifacts["generated_object"])
        token = next(item for item in mutated["records"] if item["object_kind"] == "TOKEN_CLASS")
        token["source_part_bindings"].pop()
        token["content_sha256"] = compute_artifact_content_sha256(mutated)

        with self.assertRaisesRegex(ScopeCensusValidationError, "bindings are missing"):
            validate_generated_object_records(mutated, ROOT, Path(configured))

    def test_semantic_generated_object_evidence_mutation_is_rejected(self) -> None:
        configured = os.environ.get("MANAFOLD_SOURCE_ARCHIVE")
        if not configured:
            self.skipTest("MANAFOLD_SOURCE_ARCHIVE is not configured")
        mutated = copy.deepcopy(self.artifacts["generated_object"])
        record = next(item for item in mutated["records"] if item["object_kind"] == "COPY_INSTANCE")
        evidence = record.setdefault("capability_evidence", [{"family_id": "cap.aura"}])[0]
        evidence["family_id"] = "cap.aura"
        mutated["content_sha256"] = compute_artifact_content_sha256(mutated)

        with self.assertRaisesRegex(ScopeCensusValidationError, "capability evidence"):
            validate_generated_object_records(mutated, ROOT, Path(configured))

    def test_builder_is_deterministic_when_configured(self) -> None:
        configured = os.environ.get("MANAFOLD_SOURCE_ARCHIVE")
        if not configured:
            self.skipTest("MANAFOLD_SOURCE_ARCHIVE is not configured")
        rebuilt = build_census_artifacts(ROOT, Path(configured))
        self.assertEqual(rebuilt, self.artifacts)


if __name__ == "__main__":
    unittest.main()
