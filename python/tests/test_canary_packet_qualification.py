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
            self.assertNotEqual(qualify_packet(result.packet_dir), "READY_FOR_HUMAN_REVIEW")

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


if __name__ == "__main__":
    unittest.main()
