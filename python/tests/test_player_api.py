from __future__ import annotations

import inspect
import sys
import unittest
from pathlib import Path
from typing import get_type_hints

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.observation import PlayerStepV2
from mtgml.player_client import PlayerClient


class PlayerApiTests(unittest.TestCase):
    def test_python_protocol_contains_the_full_rust_player_surface(self) -> None:
        self.assertEqual(
            {
                name
                for name, value in inspect.getmembers(PlayerClient, inspect.isfunction)
                if not name.startswith("_")
            },
            {"observation", "information_state", "visible_decision", "submit"},
        )
        hints = get_type_hints(PlayerClient.submit)
        self.assertIs(hints["return"], PlayerStepV2)

    def test_v2_public_boundary_excludes_privileged_fields(self) -> None:
        fields = set(PlayerStepV2.__dataclass_fields__)
        self.assertEqual(
            fields,
            {
                "schema_version",
                "information_state",
                "observed_events",
                "next_decision",
                "status",
            },
        )
        self.assertTrue(
            fields.isdisjoint({"root_seed", "checkpoint", "fork", "authoritative_events", "replay"})
        )


def test_v2_public_boundary_excludes_privileged_fields() -> None:
    test = PlayerApiTests("test_v2_public_boundary_excludes_privileged_fields")
    test.test_v2_public_boundary_excludes_privileged_fields()


if __name__ == "__main__":
    unittest.main()
