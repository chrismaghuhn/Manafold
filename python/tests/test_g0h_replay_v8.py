from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from mtgml.episode import EpisodeStatus
from mtgml.errors import WireError
from mtgml.replay import (
    AuthoritativeReplayV8,
    ContentContractMaterialV1,
    ExecutionIdentityV1,
    ReplayManifestV8,
    SemanticContractMaterialV7,
    calculate_checkpoint_digest_v8,
)
from mtgml.wire import decode_canonical, encode_canonical

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

    def test_execution_identity_validate_rejects_unknown_program_kind(self) -> None:
        fixture = json.loads(
            (ROOT / "schemas/examples/authoritative-replay-v8.json").read_text(encoding="utf-8")
        )
        identity = ExecutionIdentityV1(
            program_kind="synthetic_rules_compat",
            semantic_contract_id=fixture["manifest"]["execution_identity"]["semantic_contract_id"],
        )
        with self.assertRaises(WireError):
            identity.validate()

    def test_replay_v8_rejects_steps_after_terminal_and_truncated_but_keeps_closed_files(
        self,
    ) -> None:
        for kind in ("terminal", "truncated"):
            path = ROOT / "wire" / "negative" / f"authoritative-replay-v8-step-after-{kind}.json"
            fixture = json.loads(path.read_text(encoding="utf-8"))
            with (
                self.subTest(kind=kind, case="transition-after-closed"),
                self.assertRaises(WireError),
            ):
                AuthoritativeReplayV8.from_wire(fixture)

            terminal_final = copy.deepcopy(fixture)
            terminal_final["steps"] = terminal_final["steps"][:1]
            with self.subTest(kind=kind, case="closed-final-step"):
                replay = AuthoritativeReplayV8.from_wire(terminal_final)
                self.assertEqual(replay.final_identity.episode_status.kind, kind)


def _read(relative: str) -> object:
    return json.loads((ROOT / relative).read_text(encoding="utf-8"))


class ContentContractMaterialTests(unittest.TestCase):
    """The content child that Replay V8 embeds in its semantic contract."""

    def test_content_child_semantic_negative_vectors_reject(self) -> None:
        vectors = _read("schemas/negative/m4-phase2-replay-child-semantic-negatives.json")
        base = _read("schemas/examples/replay-manifest-v8.json")
        assert isinstance(vectors, dict) and isinstance(base, dict)
        for case in vectors["cases"]:
            candidate = copy.deepcopy(base)
            candidate["semantic_contract"]["content_contract"] = case["content_contract"]
            candidate["semantic_contract"]["manifest"]["content_contract_id"] = case[
                "parent_content_contract_id"
            ]
            with self.subTest(case=case["case"]), self.assertRaises((WireError, ValueError)):
                ReplayManifestV8.from_wire(candidate)

    def test_content_child_wire_negatives_reject(self) -> None:
        for name in (
            "invalid-base64",
            "missing-padding",
            "extra-padding",
            "whitespace",
            "uppercase-id",
            "unknown-field",
        ):
            with self.subTest(name=name), self.assertRaises(WireError):
                ContentContractMaterialV1.from_wire(
                    _read(f"schemas/negative/content-contract-child-{name}.json")
                )
        presence = _read("schemas/negative/content-contract-presence-mismatch.json")
        with self.assertRaises(WireError):
            SemanticContractMaterialV7.from_wire(presence)

    def test_duplicate_content_child_fields_reject_at_raw_json_boundary(self) -> None:
        manifest = ReplayManifestV8.from_wire(_read("schemas/examples/replay-manifest-v8.json"))
        canonical = encode_canonical(manifest)
        self.assertEqual(decode_canonical("replay-manifest.v8", canonical), manifest)
        child_id = (
            b'"content_contract_id":'
            b'"80d26c187739664e880948e767e44ed9791aa7c25e7ef703e6d63385311fb346",'
        )
        self.assertIn(child_id, canonical)
        duplicated = canonical.replace(child_id, child_id + child_id, 1)
        with self.assertRaises(WireError) as caught:
            decode_canonical("replay-manifest.v8", duplicated)
        self.assertEqual(caught.exception.code, "decode.invalid_json")
        self.assertIn("duplicate object key", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
