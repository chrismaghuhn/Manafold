"""Mechanical Python parity for the V6 checkpoint digest identity."""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.episode import EpisodeStatus
from mtgml.persistence import (
    CHECKPOINT_CODEC_ID_V6,
    CHECKPOINT_CODEC_VERSION_V6,
    CHECKPOINT_DOMAIN_V6,
    CHECKPOINT_INPUT_SCHEMA_V6,
    PersistenceError,
    calculate_checkpoint_digest_v6,
)

KAT_PATH = ROOT / "persistence" / "golden" / "checkpoint-digest-v6-kat.v1.json"


def load_vectors() -> list[dict[str, object]]:
    with KAT_PATH.open(encoding="utf-8") as handle:
        document = json.load(handle)
    return document["vectors"]


class V6CheckpointDigestTests(unittest.TestCase):
    def test_v6_identity_constants_and_shared_known_answer(self) -> None:
        self.assertEqual(CHECKPOINT_DOMAIN_V6, "mtgml.checkpoint-digest.v6")
        self.assertEqual(
            CHECKPOINT_INPUT_SCHEMA_V6,
            "environment-checkpoint-digest-input.v6",
        )
        vectors = load_vectors()
        self.assertGreaterEqual(len(vectors), 1)
        for vector in vectors:
            with self.subTest(case=vector["case"]):
                digest = calculate_checkpoint_digest_v6(
                    full_state_digest=str(vector["full_state_digest"]),
                    status=EpisodeStatus.running(),
                    counters=vector["counters"],
                    codec_id=str(vector["codec_id"]),
                    semantic_version=str(vector["semantic_version"]),
                    program_kind=str(vector["program_kind"]),
                    semantic_contract_id=str(vector["semantic_contract_id"]),
                )
                self.assertEqual(digest, vector["expected_digest"])

    def test_each_checkpoint_identity_dimension_changes_digest(self) -> None:
        base = calculate_checkpoint_digest_v6(
            full_state_digest="01" * 32,
            status=EpisodeStatus.running(),
            counters={
                "decisions_submitted": 0,
                "accepted_transitions": 0,
                "rule_events_emitted": 0,
                "resource_units_consumed": 0,
                "wall_clock_elapsed_millis": 0,
            },
            codec_id=CHECKPOINT_CODEC_ID_V6,
            semantic_version=CHECKPOINT_CODEC_VERSION_V6,
            program_kind="magic_rules",
            semantic_contract_id="11" * 32,
        )
        changed_state = calculate_checkpoint_digest_v6(
            full_state_digest="02" * 32,
            status=EpisodeStatus.running(),
            counters={
                "decisions_submitted": 0,
                "accepted_transitions": 0,
                "rule_events_emitted": 0,
                "resource_units_consumed": 0,
                "wall_clock_elapsed_millis": 0,
            },
            codec_id=CHECKPOINT_CODEC_ID_V6,
            semantic_version=CHECKPOINT_CODEC_VERSION_V6,
            program_kind="magic_rules",
            semantic_contract_id="11" * 32,
        )
        self.assertNotEqual(base, changed_state)

    def test_v5_codec_identity_is_not_accepted_as_v6(self) -> None:
        vector = load_vectors()[0]
        with self.assertRaises(PersistenceError):
            calculate_checkpoint_digest_v6(
                full_state_digest=str(vector["full_state_digest"]),
                status=EpisodeStatus.running(),
                counters=vector["counters"],
                codec_id=CHECKPOINT_CODEC_ID_V6,
                semantic_version="5",
                program_kind=str(vector["program_kind"]),
                semantic_contract_id=str(vector["semantic_contract_id"]),
            )


if __name__ == "__main__":
    unittest.main()
