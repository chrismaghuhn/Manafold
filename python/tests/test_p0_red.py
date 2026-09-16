from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

import mtgml.observation as observation
import mtgml.persistence as persistence
import mtgml.replay as replay


class P0RedTests(unittest.TestCase):
    def test_python_replay_v4_identity_family_is_not_current_yet(self) -> None:
        self.assertTrue(
            hasattr(replay, "ReplayManifestV4"),
            "P0 requires the Python ReplayManifestV4 DTO",
        )
        self.assertTrue(
            hasattr(replay, "AuthoritativeReplayV4"),
            "P0 requires the Python AuthoritativeReplayV4 DTO",
        )

    def test_python_v4_checkpoint_digest_is_not_current_yet(self) -> None:
        self.assertTrue(
            hasattr(persistence, "calculate_checkpoint_digest_v4"),
            "P0 requires the Python V4 checkpoint digest primitive",
        )

    def test_python_m3_observation_payload_is_not_current_yet(self) -> None:
        self.assertTrue(
            hasattr(observation, "SyntheticM3Observation"),
            "P0 requires the synthetic-m3-observation.v1 DTO",
        )


if __name__ == "__main__":
    unittest.main()
