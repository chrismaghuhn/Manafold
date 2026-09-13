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
COMPATIBILITY = ROOT / "wire" / "historical" / "v1-v2-compatibility.v1.json"
COMPATIBILITY_SCHEMA = "wire-historical-v1-v2-compatibility.v1"
COMPATIBILITY_CLASSIFICATION = "historical_evidence_only_current_semantic_regeneration"

EXPECTED_COMPATIBILITY = {
    ("golden", "observation-envelope.v1", "observation-envelope.v1.json"),
    ("golden", "information-state-envelope.v1", "information-state-envelope.v1.json"),
    ("golden", "player-step.v1", "player-step.v1.json"),
    ("negative", "player-step.v1", "player-step-privileged-extra-field.json"),
}
COMPATIBILITY_TOP_LEVEL_KEYS = {
    "schema_version",
    "source_commit",
    "classification",
    "fixtures",
}
COMPATIBILITY_ENTRY_KEYS = {"manifest", "contract", "path", "classification"}

EXPECTED_PROMOTED = {
    (
        "information-state-envelope.v2",
        "information-state-envelope.v2.json",
    ): "information-state-envelope.v2.schema.json",
    (
        "observed-event-envelope.v2",
        "observed-event-envelope.v2.json",
    ): "observed-event-envelope.v2.schema.json",
    ("player-step.v2", "player-step.v2.json"): "player-step.v2.schema.json",
    ("replay-manifest.v3", "replay-manifest.v3.json"): "replay-manifest.v3.schema.json",
    (
        "authoritative-replay.v3",
        "authoritative-replay-empty.v3.json",
    ): "authoritative-replay.v3.schema.json",
    (
        "authoritative-replay.v3",
        "replay-v3-checkpoint-digest-mismatch.json",
    ): "authoritative-replay.v3.schema.json",
}


def _git_bytes(relative_path: str) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{STARTING_SHA}:{relative_path}"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    return completed.stdout


def _baseline_manifest(kind: str) -> list[dict[str, object]]:
    relative_path = f"wire/{kind}/manifest.json"
    return json.loads(_git_bytes(relative_path).decode("utf-8"))["fixtures"]


def _compatibility_paths() -> set[tuple[str, str, str]]:
    compatibility = json.loads(COMPATIBILITY.read_text(encoding="utf-8"))
    if set(compatibility) != COMPATIBILITY_TOP_LEVEL_KEYS:
        raise AssertionError("historical compatibility top-level shape changed")
    if compatibility["schema_version"] != COMPATIBILITY_SCHEMA:
        raise AssertionError("historical compatibility schema changed")
    if compatibility["source_commit"] != STARTING_SHA:
        raise AssertionError("historical compatibility source SHA changed")
    if compatibility["classification"] != COMPATIBILITY_CLASSIFICATION:
        raise AssertionError("historical compatibility classification changed")
    entries = compatibility["fixtures"]
    if not isinstance(entries, list) or len(entries) != len(EXPECTED_COMPATIBILITY):
        raise AssertionError("historical compatibility fixture count changed")
    actual: set[tuple[str, str, str]] = set()
    for entry in entries:
        if set(entry) != COMPATIBILITY_ENTRY_KEYS:
            raise AssertionError("historical compatibility entry shape changed")
        if entry["classification"] != COMPATIBILITY_CLASSIFICATION:
            raise AssertionError("historical compatibility entry classification changed")
        key = (entry["manifest"], entry["contract"], entry["path"])
        if key in actual:
            raise AssertionError(f"duplicate historical compatibility entry: {key}")
        actual.add(key)
    if actual != EXPECTED_COMPATIBILITY:
        raise AssertionError(
            "historical compatibility paths changed: "
            f"expected={EXPECTED_COMPATIBILITY}, actual={actual}"
        )
    return actual


class M2BStagingFixtureTests(unittest.TestCase):
    def test_historical_compatibility_classification_is_explicit(self) -> None:
        actual = _compatibility_paths()
        inventory = json.loads(HISTORICAL.read_text(encoding="utf-8"))
        inventory_keys = {
            (entry["manifest"], entry["contract"], entry["path"]) for entry in inventory["fixtures"]
        }
        self.assertTrue(actual <= inventory_keys)

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
            (ROOT / "wire" / "golden" / "authoritative-replay-empty.v3.json").read_text(
                encoding="utf-8"
            )
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
        classified_paths = _compatibility_paths()
        active_paths = set()
        for kind in ("golden", "negative"):
            manifest = json.loads(
                (ROOT / "wire" / kind / "manifest.json").read_text(encoding="utf-8")
            )
            active_paths.update(
                (kind, entry["contract"], entry["path"]) for entry in manifest["fixtures"]
            )
        self.assertTrue(classified_paths <= active_paths)

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
            if key not in classified_paths:
                self.assertEqual(current_path.read_bytes(), baseline_bytes, key[2])
            else:
                self.assertTrue(current_path.is_file(), key[2])
            self.assertEqual(
                inventory_entry["sha256"],
                hashlib.sha256(baseline_bytes).hexdigest(),
                key[2],
            )


if __name__ == "__main__":
    unittest.main()
