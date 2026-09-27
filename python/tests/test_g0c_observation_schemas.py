from __future__ import annotations

import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class SuccessorObservationSchemaTests(unittest.TestCase):
    def test_observation_envelope_v2_uses_view_sequence_without_global_revision(self) -> None:
        schema = json.loads(
            (ROOT / "schemas/observation-envelope.v2.schema.json").read_text(encoding="utf-8")
        )
        self.assertEqual(schema["$id"], "observation-envelope.v2.schema.json")
        self.assertTrue(schema["additionalProperties"] is False)
        self.assertEqual(schema["properties"]["schema_version"]["const"], "observation-envelope.v2")
        self.assertIn("view_sequence", schema["properties"])
        self.assertNotIn("state_revision", schema["properties"])

    def test_information_state_v3_uses_perspective_cursor_without_global_revision(self) -> None:
        schema = json.loads(
            (ROOT / "schemas/information-state-envelope.v3.schema.json").read_text(encoding="utf-8")
        )
        self.assertEqual(schema["$id"], "information-state-envelope.v3.schema.json")
        self.assertTrue(schema["additionalProperties"] is False)
        self.assertEqual(
            schema["properties"]["schema_version"]["const"],
            "information-state-envelope.v3",
        )
        self.assertIn("next_visible_sequence", schema["properties"])
        self.assertNotIn("state_revision", schema["properties"])


if __name__ == "__main__":
    unittest.main()
