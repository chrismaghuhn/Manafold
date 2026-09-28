from __future__ import annotations

import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class SharedExecutionObservationSchemaTests(unittest.TestCase):
    def test_named_payload_schema_is_closed_and_has_stack_and_effect_views(self) -> None:
        path = ROOT / "schemas/magic-shared-execution-observation.v1.schema.json"
        schema = json.loads(path.read_text(encoding="utf-8"))
        self.assertEqual(schema["$id"], "magic-shared-execution-observation.v1.schema.json")
        self.assertTrue(schema["additionalProperties"] is False)
        self.assertEqual(
            schema["properties"]["schema_version"]["const"],
            "magic-shared-execution-observation.v1",
        )
        self.assertIn("stack", schema["properties"])
        self.assertIn("temporary_effects", schema["properties"])
        self.assertNotIn("candidates", schema["properties"])

    def test_positive_fixture_contains_all_bounded_stack_and_effect_variants(self) -> None:
        fixture = json.loads(
            (ROOT / "schemas/examples/magic-shared-execution-observation-v1.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(
            {item["kind"] for item in fixture["stack"]},
            {"spell", "activated_ability", "triggered_ability"},
        )
        activated = next(item for item in fixture["stack"] if item["kind"] == "activated_ability")
        self.assertIn("modes", activated)
        self.assertIn(
            "modes",
            json.loads(
                (ROOT / "schemas/magic-shared-execution-observation.v1.schema.json").read_text(
                    encoding="utf-8"
                )
            )["$defs"]["public_stack_item"]["oneOf"][1]["required"],
        )
        self.assertEqual(
            {item["operation"]["kind"] for item in fixture["temporary_effects"]},
            {"power_toughness_delta", "grant_keyword"},
        )
        self.assertNotIn("state_revision", fixture)
        self.assertNotIn("stack_object_id", json.dumps(fixture))


if __name__ == "__main__":
    unittest.main()
