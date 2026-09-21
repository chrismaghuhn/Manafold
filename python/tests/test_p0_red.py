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
    def test_python_replay_v5_identity_family_is_present(self) -> None:
        self.assertTrue(
            hasattr(replay, "ReplayManifestV5"),
            "P0 requires the Python ReplayManifestV5 DTO",
        )
        self.assertTrue(
            hasattr(replay, "AuthoritativeReplayV5"),
            "P0 requires the Python AuthoritativeReplayV5 DTO",
        )

    def test_python_v5_checkpoint_digest_is_current(self) -> None:
        self.assertTrue(
            hasattr(persistence, "calculate_checkpoint_digest_v5"),
            "P0 requires the Python V5 checkpoint digest primitive",
        )

    def test_python_m3_observation_payload_is_current(self) -> None:
        self.assertTrue(
            hasattr(observation, "SyntheticM3Observation"),
            "P0 requires the synthetic-m3-observation.v1 DTO",
        )


if __name__ == "__main__":
    unittest.main()
