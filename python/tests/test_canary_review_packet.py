from __future__ import annotations

import json
import sys
import tempfile
import unittest
from collections.abc import Mapping
from copy import deepcopy
from dataclasses import replace
from pathlib import Path
from typing import ClassVar, cast
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import (
    SOURCE_CONTEXT_KEYS,
    AuthoritySourceResolver,
    ResolutionError,
    ResolutionStatus,
)
from authority_validator import AuthorityValidator
from build_m2_5_c_authority_review_worklist import (
    LoadedReviewInputs,
    ReviewWorklistError,
    load_review_inputs,
)
from build_m2_5_c_canary_review_packet import (
    BLOCKED,
    CANARY_ORDINAL,
    CanaryPacketResult,
    build_canary_packet,
    qualify_packet,
)


class CanaryReviewPacketTests(unittest.TestCase):
    inputs: ClassVar[LoadedReviewInputs]

    @classmethod
    def setUpClass(cls) -> None:
        cls.inputs = load_review_inputs(ROOT)

    def _read_json(self, packet_dir: Path, name: str) -> dict[str, object]:
        return cast(dict[str, object], json.loads((packet_dir / name).read_text(encoding="utf-8")))

    def test_real_canary_is_ordinal_zero_and_exactly_bound(self) -> None:
        candidate_id = cast(str, self.inputs.classification_records[CANARY_ORDINAL]["candidate_id"])
        candidate = next(
            item for item in self.inputs.candidate_records if item["candidate_id"] == candidate_id
        )
        instance = next(
            item
            for item in self.inputs.source_instance_records
            if item["candidate_id"] == candidate_id
        )
        with tempfile.TemporaryDirectory() as temporary:
            result = build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
            manifest = self._read_json(result.packet_dir, "manifest.v1.json")
            inventory = self._read_json(result.packet_dir, "source_inventory.v1.json")
        canary = cast(dict[str, object], manifest["canary"])
        self.assertEqual(result.status, BLOCKED)
        self.assertEqual(canary["ordinal"], 0)
        self.assertEqual(canary["candidate_id"], candidate_id)
        self.assertEqual(canary["candidate_identity"], candidate["candidate_identity"])
        self.assertEqual(canary["source_instance_id"], instance["source_instance_id"])
        facts = cast(dict[str, object], inventory["facts"])
        candidate_fact = cast(dict[str, object], facts["candidate"])
        instance_fact = cast(dict[str, object], facts["source_instance"])
        self.assertEqual(candidate_fact["record"], candidate)
        self.assertEqual(instance_fact["record"], instance)
        self.assertEqual(cast(dict[str, object], facts["rev3"])["status"], BLOCKED)
        b1 = cast(dict[str, object], facts["b1_final"])
        self.assertEqual(
            cast(dict[str, object], b1["official_source_resolution"])["status"],
            BLOCKED,
        )

    def test_source_inventory_contains_exact_b2_and_b1_references(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
            inventory = self._read_json(result.packet_dir, "source_inventory.v1.json")
        facts = cast(dict[str, object], inventory["facts"])
        b2 = cast(dict[str, object], facts["b2"])
        families = cast(list[dict[str, object]], b2["requirement_families"])
        self.assertEqual([family["family_id"] for family in families], ["cap.aura", "cap.draw"])
        for family in families:
            self.assertEqual(family["kind"], "SOURCE_FACT")
            source_binding = cast(Mapping[str, object], family["source_binding"])
            self.assertEqual(
                source_binding["path"],
                "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
            )
        b1 = cast(dict[str, object], facts["b1_final"])
        citations_binding = cast(dict[str, object], b1["citations_binding"])
        input_bindings = cast(
            Mapping[str, object], self.inputs.candidate_universe["input_bindings"]
        )
        expected = cast(list[Mapping[str, object]], input_bindings["b1_final_artifacts"])[0]
        self.assertEqual(citations_binding["path"], expected["path"])
        self.assertEqual(citations_binding["raw_sha256"], expected["raw_sha256"])
        self.assertEqual(b1["authority_count"], 7)
        self.assertEqual(b1["selected_citations"], [])

    def test_review_worksheet_has_model_domains_and_unresolved_slots(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
            worksheet = self._read_json(result.packet_dir, "review_worksheet.v1.json")
        decisions = cast(dict[str, object], worksheet["reviewer_decisions"])
        domains = cast(list[dict[str, object]], decisions["domain_reviews"])
        self.assertEqual(
            [item["review_domain"] for item in domains], list(self.inputs.review_domains)
        )
        self.assertTrue(all(item["applicability"] == "AWAITING_HUMAN_REVIEW" for item in domains))
        relation = cast(dict[str, object], decisions["relation_review"])
        self.assertTrue(
            all(
                value == "AWAITING_HUMAN_REVIEW" for key, value in relation.items() if key != "kind"
            )
        )
        context = cast(dict[str, object], decisions["conditional_context_review"])
        self.assertEqual(context["status"], "AWAITING_HUMAN_REVIEW")
        dimensions = cast(list[dict[str, object]], context["dimensions"])
        self.assertEqual(
            [item["dimension"] for item in dimensions],
            list(SOURCE_CONTEXT_KEYS),
        )
        self.assertTrue(
            all(
                item["value"] == "AWAITING_HUMAN_REVIEW"
                and item["evidence_selection"] == "AWAITING_HUMAN_REVIEW"
                for item in dimensions
            )
        )
        temporal = cast(list[dict[str, object]], context["temporal_semantics"])
        self.assertEqual(len(temporal), 4)
        self.assertEqual(
            [item["slot"] for item in temporal],
            list(cast(dict[str, object], self.inputs.model["temporal_value_vocabulary"])),
        )
        self.assertTrue(
            all(
                item["value"] == "AWAITING_HUMAN_REVIEW"
                and item["evidence_selection"] == "AWAITING_HUMAN_REVIEW"
                for item in temporal
            )
        )
        scope = cast(dict[str, object], decisions["conditional_scope_review"])
        self.assertEqual(scope["status"], "AWAITING_HUMAN_REVIEW")

    def test_blocked_packet_is_not_authority_and_qualifies_blocked(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
            self.assertEqual(qualify_packet(result.packet_dir), BLOCKED)
            manifest = self._read_json(result.packet_dir, "manifest.v1.json")
        with self.assertRaises(ResolutionError) as context:
            AuthorityValidator(AuthoritySourceResolver(ROOT)).validate(manifest)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)

    def test_machine_packet_is_repeatable_and_has_no_local_path_or_acceptance(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = build_canary_packet(ROOT, root / "first", inputs=self.inputs)
            second = build_canary_packet(ROOT, root / "second", inputs=self.inputs)
            for name in (
                "manifest.v1.json",
                "source_inventory.v1.json",
                "review_worksheet.v1.json",
            ):
                first_raw = (first.packet_dir / name).read_bytes()
                second_raw = (second.packet_dir / name).read_bytes()
                self.assertEqual(first_raw, second_raw)
                self.assertNotIn(str(ROOT).encode(), first_raw)
                self.assertNotIn(b"human_accepted", first_raw)
                self.assertNotIn(b"review_event_ref", first_raw)
                self.assertNotIn(b"terminal_disposition", first_raw)
                self.assertNotIn(b"interaction_class_id", first_raw)
            self.assertEqual(first.packet_sha256, second.packet_sha256)

    def test_tracked_tree_drift_fails_closed(self) -> None:
        path = ROOT / "scripts" / "README.md"
        original = path.read_bytes()
        try:
            path.write_bytes(original + b"\n")
            with (
                tempfile.TemporaryDirectory() as temporary,
                self.assertRaises(ReviewWorklistError) as context,
            ):
                build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
        finally:
            path.write_bytes(original)
        self.assertEqual(context.exception.code, "SOURCE_TREE_DIRTY")

    def test_tampered_source_binding_fails_closed(self) -> None:
        binding = deepcopy(self.inputs.candidate_universe_binding)
        binding["raw_sha256"] = "00" * 32
        mutated = replace(self.inputs, candidate_universe_binding=binding)
        with (
            tempfile.TemporaryDirectory() as temporary,
            self.assertRaises(ValueError) as context,
        ):
            build_canary_packet(ROOT, Path(temporary), inputs=mutated)
        self.assertIn("SOURCE_BINDING_MISMATCH", str(context.exception))

    def test_missing_source_record_fails_closed(self) -> None:
        mutated = replace(
            self.inputs, source_instance_records=self.inputs.source_instance_records[:-1]
        )
        with (
            tempfile.TemporaryDirectory() as temporary,
            self.assertRaises(ValueError) as context,
        ):
            build_canary_packet(ROOT, Path(temporary), inputs=mutated)
        self.assertIn("SOURCE_INSTANCE_CARDINALITY_MISMATCH", str(context.exception))

    def test_cli_does_not_accept_output_directory_override(self) -> None:
        import build_m2_5_c_canary_review_packet as packet_builder

        with tempfile.TemporaryDirectory() as temporary:
            with (
                patch.object(
                    sys,
                    "argv",
                    [packet_builder.__file__, "--output-dir", temporary],
                ),
                self.assertRaises(SystemExit) as context,
            ):
                packet_builder.main()
            self.assertNotEqual(context.exception.code, 0)

    def test_cli_reports_blocked_result_with_blocked_exit_code(self) -> None:
        import build_m2_5_c_canary_review_packet as packet_builder

        result = CanaryPacketResult(
            status=BLOCKED,
            packet_dir=ROOT / "dist" / "m2-5-c-authority-review" / "canary",
            worklist_path=ROOT / "dist" / "m2-5-c-authority-review" / "review_worklist.v1.jsonl",
            worklist_sha256="00" * 32,
            packet_sha256="11" * 32,
        )
        with (
            patch.object(sys, "argv", [packet_builder.__file__]),
            patch.object(packet_builder, "build_canary_packet", return_value=result),
        ):
            self.assertEqual(packet_builder.main(), 2)


if __name__ == "__main__":
    unittest.main()
