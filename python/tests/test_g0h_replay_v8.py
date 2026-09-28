from __future__ import annotations

import json
import unittest
from pathlib import Path

from mtgml.episode import EpisodeStatus
from mtgml.errors import WireError
from mtgml.replay import AuthoritativeReplayV8, calculate_checkpoint_digest_v8

ROOT = Path(__file__).resolve().parents[2]


class G0HReplayV8Tests(unittest.TestCase):
    def test_checkpoint_digest_v8_matches_rust_kat(self) -> None:
        fixture = json.loads(
            (ROOT / "persistence/golden/checkpoint-digest-v8-kat.v1.json").read_text(
                encoding="utf-8"
            )
        )
        digest = calculate_checkpoint_digest_v8(
            fixture["full_state_digest_v7"],
            EpisodeStatus.running(),
            {
                "decisions_submitted": 0,
                "accepted_transitions": 0,
                "rule_events_emitted": 0,
                "resource_units_consumed": 0,
                "wall_clock_elapsed_millis": 0,
            },
            "in-memory-reference",
            "8",
            "magic_rules",
            fixture["semantic_contract_id"],
        )
        self.assertEqual(digest, fixture["expected_digest"])

    def test_authoritative_replay_v8_fixture_validates_digest_identity(self) -> None:
        fixture = json.loads(
            (ROOT / "schemas/examples/authoritative-replay-v8.json").read_text(encoding="utf-8")
        )
        replay = AuthoritativeReplayV8.from_wire(fixture)
        self.assertEqual(replay.to_wire(), fixture)
        fixture["manifest"]["initial_identity"]["checkpoint_digest"] = "0" * 64
        with self.assertRaises(WireError):
            AuthoritativeReplayV8.from_wire(fixture)


if __name__ == "__main__":
    unittest.main()
