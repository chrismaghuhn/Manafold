from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import ClassVar, cast
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import build_m2_5_c_authority_review_worklist as worklist_builder
from build_m2_5_c_authority_review_worklist import (
    LoadedReviewInputs,
    _json_bytes,
    load_review_inputs,
)
from build_m2_5_c_canary_review_packet import build_canary_packet, qualify_packet


class CanaryPacketQualificationTests(unittest.TestCase):
    inputs: ClassVar[LoadedReviewInputs]

    @classmethod
    def setUpClass(cls) -> None:
        cls.clean_check_patch = patch.object(
            worklist_builder, "_ensure_worktree_clean", return_value=None
        )
        cls.clean_check_patch.start()
        cls.inputs = load_review_inputs(ROOT)

    @classmethod
    def tearDownClass(cls) -> None:
        cls.clean_check_patch.stop()

    def test_qualifier_rejects_promoted_blocked_packet(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
            manifest_path = result.packet_dir / "manifest.v1.json"
            manifest = cast(
                dict[str, object], json.loads(manifest_path.read_text(encoding="utf-8"))
            )
            manifest["packet_status"] = "READY_FOR_HUMAN_REVIEW"
            manifest_path.write_bytes(_json_bytes(manifest))
            inventory_path = result.packet_dir / "source_inventory.v1.json"
            inventory = cast(
                dict[str, object], json.loads(inventory_path.read_text(encoding="utf-8"))
            )
            inventory["inventory_status"] = "READY_FOR_HUMAN_REVIEW"
            inventory_path.write_bytes(_json_bytes(inventory))
            with self.assertRaises(ValueError):
                qualify_packet(result.packet_dir)

    def test_qualifier_rejects_canary_identity_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
            manifest_path = result.packet_dir / "manifest.v1.json"
            manifest = cast(
                dict[str, object], json.loads(manifest_path.read_text(encoding="utf-8"))
            )
            canary = cast(dict[str, object], manifest["canary"])
            canary["candidate_id"] = "tampered-candidate"
            manifest_path.write_bytes(_json_bytes(manifest))
            worksheet_path = result.packet_dir / "review_worksheet.v1.json"
            worksheet = cast(
                dict[str, object], json.loads(worksheet_path.read_text(encoding="utf-8"))
            )
            worksheet["canary"] = canary
            worksheet_path.write_bytes(_json_bytes(worksheet))
            with self.assertRaises(ValueError):
                qualify_packet(result.packet_dir)

    def test_qualifier_rejects_review_domain_not_from_model(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
            worksheet_path = result.packet_dir / "review_worksheet.v1.json"
            worksheet = cast(
                dict[str, object], json.loads(worksheet_path.read_text(encoding="utf-8"))
            )
            decisions = cast(dict[str, object], worksheet["reviewer_decisions"])
            domains = cast(list[object], decisions["domain_reviews"])
            first_domain = cast(dict[str, object], domains[0])
            first_domain["review_domain"] = "fabricated-review-domain"
            worksheet_path.write_bytes(_json_bytes(worksheet))

            with self.assertRaises(ValueError) as raised:
                qualify_packet(result.packet_dir, inputs=self.inputs)
            self.assertIn("WORKSHEET_SOURCE_BINDING_MISMATCH", str(raised.exception))

    def test_qualifier_rejects_non_reviewer_decision_worksheet_kind(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
            worksheet_path = result.packet_dir / "review_worksheet.v1.json"
            worksheet = cast(
                dict[str, object], json.loads(worksheet_path.read_text(encoding="utf-8"))
            )
            decisions = cast(dict[str, object], worksheet["reviewer_decisions"])
            relation = cast(dict[str, object], decisions["relation_review"])
            relation["kind"] = "SOURCE_FACT"
            worksheet_path.write_bytes(_json_bytes(worksheet))

            with self.assertRaises(ValueError) as raised:
                qualify_packet(result.packet_dir, inputs=self.inputs)
            self.assertIn("WORKSHEET_SOURCE_BINDING_MISMATCH", str(raised.exception))

    def test_qualifier_rejects_source_inventory_kind_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
            inventory_path = result.packet_dir / "source_inventory.v1.json"
            inventory = cast(
                dict[str, object], json.loads(inventory_path.read_text(encoding="utf-8"))
            )
            facts = cast(dict[str, object], inventory["facts"])
            candidate_fact = cast(dict[str, object], facts["candidate"])
            candidate_fact["kind"] = "REVIEWER_DECISION"
            inventory_path.write_bytes(_json_bytes(inventory))

            with self.assertRaises(ValueError) as raised:
                qualify_packet(result.packet_dir, inputs=self.inputs)
            self.assertIn("PACKET_SOURCE_FACT_MISMATCH", str(raised.exception))

    def test_qualifier_rejects_rendered_packet_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result = build_canary_packet(ROOT, Path(temporary), inputs=self.inputs)
            markdown_path = result.packet_dir / "REVIEW_PACKET.md"
            markdown_path.write_bytes(markdown_path.read_bytes() + b"\n\nhuman_accepted\n")

            with self.assertRaises(ValueError) as raised:
                qualify_packet(result.packet_dir, inputs=self.inputs)
            self.assertIn("PACKET_RENDERING_MISMATCH", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
