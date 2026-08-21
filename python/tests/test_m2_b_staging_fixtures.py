from __future__ import annotations

import hashlib
import json
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
STARTING_SHA = "a4e769eb940611d34df05fc79effd9430891d897"
STAGING = ROOT / "wire" / "staging" / "m2-b"
HISTORICAL = ROOT / "wire" / "historical" / "v1-v2-fixtures.json"

EXPECTED_PROMOTED = {
    ("information-state-envelope.v2", "information-state-envelope.v2.json"): "information-state-envelope.v2.schema.json",
    ("observed-event-envelope.v2", "observed-event-envelope.v2.json"): "observed-event-envelope.v2.schema.json",
    ("player-step.v2", "player-step.v2.json"): "player-step.v2.schema.json",
    ("replay-manifest.v3", "replay-manifest.v3.json"): "replay-manifest.v3.schema.json",
    ("authoritative-replay.v3", "authoritative-replay-empty.v3.json"): "authoritative-replay.v3.schema.json",
    ("authoritative-replay.v3", "replay-v3-checkpoint-digest-mismatch.json"): "authoritative-replay.v3.schema.json",
}


def _git_bytes(relative_path: str) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{STARTING_SHA}:{relative_path}"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return completed.stdout


def _baseline_manifest(kind: str) -> list[dict[str, object]]:
    relative_path = f"wire/{kind}/manifest.json"
    return json.loads(_git_bytes(relative_path).decode("utf-8"))["fixtures"]


class M2BStagingFixtureTests(unittest.TestCase):
    def test_promoted_fixtures_have_no_staging_copy(self) -> None:
        self.assertFalse(STAGING.exists())
        records = {}
        for kind in ("golden", "negative"):
            manifest = json.loads(
                (ROOT / "wire" / kind / "manifest.json").read_text(encoding="utf-8")
            )
            for entry in manifest["fixtures"]:
                if entry["path"] in {path for _, path in EXPECTED_PROMOTED}:
                    records[(entry["contract"], entry["path"])] = kind
        self.assertEqual(set(records), set(EXPECTED_PROMOTED))
        for contract, path in EXPECTED_PROMOTED:
            self.assertTrue((ROOT / "wire" / records[(contract, path)] / path).is_file())

    def test_promoted_public_shapes_are_closed_and_perspective_safe(self) -> None:
        decision = json.loads(
            (ROOT / "wire" / "golden" / "player-decision-request.v2.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(decision["schema_version"], "player-decision-request.v2")
        self.assertNotIn("decision_id", decision)
        self.assertNotIn("continuation_id", decision)
        self.assertNotIn("trusted_binding", decision)
        self.assertNotIn("semantic_key", decision["candidates"][0])

        replay = json.loads(
            (ROOT / "wire" / "golden" / "authoritative-replay-empty.v3.json").read_text(encoding="utf-8")
        )
        self.assertEqual(replay["schema_version"], "authoritative-replay.v3")
        initial_identity = replay["manifest"]["initial_identity"]
        for key in (
            "state_revision",
            "full_state_digest",
            "episode_status",
            "environment_limit_counters",
            "checkpoint_codec_identity",
            "checkpoint_digest",
        ):
            self.assertIn(key, initial_identity)
            self.assertIn(key, replay["final_identity"])

    def test_baseline_fixture_bytes_and_historical_inventory_remain_covered(self) -> None:
        inventory = json.loads(HISTORICAL.read_text(encoding="utf-8"))
        self.assertEqual(inventory["schema_version"], "wire-historical-fixture-inventory.v1")
        self.assertEqual(inventory["source_commit"], STARTING_SHA)

        actual_inventory: dict[tuple[str, str, str], dict[str, object]] = {}
        for entry in inventory["fixtures"]:
            key = (entry["manifest"], entry["contract"], entry["path"])
            self.assertNotIn(key, actual_inventory)
            actual_inventory[key] = entry

        baseline: dict[tuple[str, str, str], bytes] = {}
        for kind in ("golden", "negative"):
            for entry in _baseline_manifest(kind):
                key = (kind, entry["contract"], entry["path"])
                relative_path = f"wire/{kind}/{entry['path']}"
                baseline[key] = _git_bytes(relative_path)

        self.assertEqual(set(actual_inventory), set(baseline))
        for key, baseline_bytes in baseline.items():
            inventory_entry = actual_inventory[key]
            current_path = ROOT / "wire" / key[0] / key[2]
            self.assertEqual(current_path.read_bytes(), baseline_bytes, key[2])
            self.assertEqual(
                inventory_entry["sha256"],
                hashlib.sha256(baseline_bytes).hexdigest(),
                key[2],
            )


if __name__ == "__main__":
    unittest.main()
