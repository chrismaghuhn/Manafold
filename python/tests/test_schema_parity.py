from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

try:
    import jsonschema
except ImportError:  # pragma: no cover - the locked dev environment installs it
    jsonschema = None


class SchemaParityTests(unittest.TestCase):
    @unittest.skipIf(jsonschema is None, "jsonschema is not installed")
    def test_all_golden_fixtures_match_their_normative_schema(self) -> None:
        mapping = {
            "player-decision-request.v1": "player-decision-request.v1.schema.json",
            "decision-response.v1": "decision-response.v1.schema.json",
            "player-decision-request.v2": "player-decision-request.v2.schema.json",
            "decision-response.v2": "decision-response.v2.schema.json",
            "episode-status.v1": "episode-status.v1.schema.json",
            "observed-event-envelope.v1": "observed-event-envelope.v1.schema.json",
            "observation-envelope.v1": "observation-envelope.v1.schema.json",
            "information-state-envelope.v1": "information-state-envelope.v1.schema.json",
            "player-step.v1": "player-step.v1.schema.json",
            "replay-manifest.v1": "replay-manifest.v1.schema.json",
            "authoritative-replay.v1": "authoritative-replay.v1.schema.json",
            "replay-manifest.v2": "replay-manifest.v2.schema.json",
            "authoritative-replay.v2": "authoritative-replay.v2.schema.json",
            "information-state-envelope.v2": "information-state-envelope.v2.schema.json",
            "observed-event-envelope.v2": "observed-event-envelope.v2.schema.json",
            "player-step.v2": "player-step.v2.schema.json",
            "replay-manifest.v3": "replay-manifest.v3.schema.json",
            "authoritative-replay.v3": "authoritative-replay.v3.schema.json",
        }
        directory = ROOT / "wire" / "golden"
        manifest = json.loads((directory / "manifest.json").read_text(encoding="utf-8"))
        for case in manifest["fixtures"]:
            schema = json.loads(
                (ROOT / "schemas" / mapping[case["contract"]]).read_text(encoding="utf-8")
            )
            instance = json.loads((directory / case["path"]).read_text(encoding="utf-8"))
            with self.subTest(case=case["path"]):
                jsonschema.Draft202012Validator(schema).validate(instance)

    @unittest.skipIf(jsonschema is None, "jsonschema is not installed")
    def test_m2_b_detached_schema_fixtures(self) -> None:
        mapping = {
            "player-decision-request.v2": "player-decision-request.v2.schema.json",
            "decision-response.v2": "decision-response.v2.schema.json",
            "information-state-envelope.v2": "information-state-envelope.v2.schema.json",
            "observed-event-envelope.v2": "observed-event-envelope.v2.schema.json",
            "player-step.v2": "player-step.v2.schema.json",
            "replay-manifest.v3": "replay-manifest.v3.schema.json",
            "authoritative-replay.v3": "authoritative-replay.v3.schema.json",
        }
        directory = ROOT / "wire" / "golden"
        manifest = json.loads((directory / "manifest.json").read_text(encoding="utf-8"))
        cases = [case for case in manifest["fixtures"] if case["contract"] in mapping]
        self.assertEqual(
            {case["contract"] for case in cases},
            set(mapping),
        )
        for case in cases:
            schema = json.loads(
                (ROOT / "schemas" / mapping[case["contract"]]).read_text(encoding="utf-8")
            )
            instance = json.loads((directory / case["path"]).read_text(encoding="utf-8"))
            with self.subTest(case=case["path"]):
                jsonschema.Draft202012Validator(schema).validate(instance)

    def test_replay_manifest_schema_has_exact_required_identity_fields(self) -> None:
        schema = json.loads(
            (ROOT / "schemas" / "replay-manifest.v1.schema.json").read_text(encoding="utf-8")
        )
        self.assertEqual(
            set(schema["required"]),
            {
                "schema_version",
                "engine_build",
                "kernel",
                "rules_snapshot",
                "format_policy_snapshot",
                "oracle_snapshot",
                "card_bundle",
                "schemas",
                "randomness",
                "decks",
                "initial_state_revision",
                "initial_state_digest",
            },
        )

    def test_observed_event_schema_contains_all_seven_closed_variants(self) -> None:
        schema = json.loads(
            (ROOT / "schemas" / "observed-event-envelope.v1.schema.json").read_text(
                encoding="utf-8"
            )
        )
        variants = {
            entry["properties"]["kind"]["const"] for entry in schema["properties"]["event"]["oneOf"]
        }
        self.assertEqual(
            variants,
            {
                "object_moved",
                "object_ceased_to_exist",
                "life_changed",
                "object_tapped",
                "decision_available",
                "random_outcome_visible",
                "public_outcome",
            },
        )

    def test_episode_reasons_are_schema_enums_not_open_strings(self) -> None:
        schema = json.loads(
            (ROOT / "schemas" / "episode-status.v1.schema.json").read_text(encoding="utf-8")
        )
        terminal = next(
            item
            for item in schema["oneOf"]
            if item["properties"]["kind"].get("const") == "terminal"
        )
        truncated = next(
            item
            for item in schema["oneOf"]
            if item["properties"]["kind"].get("const") == "truncated"
        )
        self.assertEqual(len(terminal["properties"]["reason"]["enum"]), 5)
        self.assertEqual(len(truncated["properties"]["reason"]["enum"]), 5)


def test_m2_b_detached_schema_fixtures() -> None:
    test = SchemaParityTests("test_m2_b_detached_schema_fixtures")
    test.test_m2_b_detached_schema_fixtures()


if __name__ == "__main__":
    unittest.main()
