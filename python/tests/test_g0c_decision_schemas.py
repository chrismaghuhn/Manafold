from __future__ import annotations

import json
import unittest
from pathlib import Path

import jsonschema

ROOT = Path(__file__).resolve().parents[2]


class SuccessorDecisionSchemaTests(unittest.TestCase):
    def test_request_v4_schema_is_closed_and_has_no_global_revision(self) -> None:
        path = ROOT / "schemas" / "player-decision-request.v4.schema.json"
        schema = json.loads(path.read_text(encoding="utf-8"))

        self.assertEqual(schema["$id"], "player-decision-request.v4.schema.json")
        self.assertTrue(schema["additionalProperties"] is False)
        self.assertEqual(
            schema["properties"]["schema_version"]["const"],
            "player-decision-request.v4",
        )
        self.assertNotIn("state_revision", schema["properties"])
        self.assertIn("view_sequence", schema["properties"])
        self.assertIn("purpose", schema["properties"])
        self.assertIn("decision_domain_v2", schema["properties"])

    def test_request_v4_trigger_descriptor_fixture_is_valid_and_closed(self) -> None:
        schema = json.loads(
            (ROOT / "schemas" / "player-decision-request.v4.schema.json").read_text(
                encoding="utf-8"
            )
        )
        fixture = json.loads(
            (
                ROOT / "schemas" / "examples" / "player-decision-request-v4-trigger-order.json"
            ).read_text(encoding="utf-8")
        )
        jsonschema.Draft202012Validator(schema).validate(fixture)
        descriptors = [candidate["intent"]["trigger"] for candidate in fixture["candidates"]]
        self.assertEqual(
            [item["event_kind"] for item in descriptors],
            [
                "spell_cast",
                "ability_activated",
                "target_became",
                "object_entered",
                "object_left_or_died",
                "beginning_of_combat",
                "attack_declared",
                "card_drawn",
                "counter_changed",
                "damage_applied",
                "life_changed",
            ],
        )
        targets = descriptors[1]["subject"]["targets"] + [descriptors[2]["subject"]["target"]]
        self.assertEqual({target["kind"] for target in targets}, {"object", "player", "stack_item"})
        bad = dict(fixture)
        bad["state_revision"] = "18"
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(bad)
        bad = json.loads(json.dumps(fixture))
        bad["candidates"][0]["intent"]["trigger"]["subject"]["kind"] = "card_drawn"
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(bad)

    def test_response_v3_schema_binds_to_player_decision_and_view_only(self) -> None:
        path = ROOT / "schemas" / "decision-response.v3.schema.json"
        schema = json.loads(path.read_text(encoding="utf-8"))

        self.assertEqual(schema["$id"], "decision-response.v3.schema.json")
        self.assertTrue(schema["additionalProperties"] is False)
        self.assertEqual(
            schema["properties"]["schema_version"]["const"],
            "decision-response.v3",
        )
        self.assertNotIn("state_revision", schema["properties"])
        self.assertIn("player_decision_id", schema["properties"])
        self.assertIn("view_sequence", schema["properties"])


if __name__ == "__main__":
    unittest.main()
