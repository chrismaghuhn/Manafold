"""Python mechanical verification of the V6 replay identity family."""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.errors import WireError
from mtgml.replay import AuthoritativeReplayV6, ReplayManifestV6
from mtgml.wire import decode_canonical, encode_canonical

GOLDEN = ROOT / "wire" / "golden"
NEGATIVE = ROOT / "wire" / "negative"


def load_manifest() -> dict[str, object]:
    return json.loads((GOLDEN / "replay-manifest.v6.json").read_text(encoding="utf-8"))


class V6ReplayGoldenTests(unittest.TestCase):
    def test_manifest_v6_roundtrips_exactly(self) -> None:
        raw = (GOLDEN / "replay-manifest.v6.json").read_bytes()
        decoded = decode_canonical("replay-manifest.v6", raw)
        self.assertIsInstance(decoded, ReplayManifestV6)
        self.assertEqual(encode_canonical(decoded), raw)

    def test_empty_authoritative_replay_v6_roundtrips_exactly(self) -> None:
        raw = (GOLDEN / "authoritative-replay-empty.v6.json").read_bytes()
        decoded = decode_canonical("authoritative-replay.v6", raw)
        self.assertIsInstance(decoded, AuthoritativeReplayV6)
        self.assertEqual(encode_canonical(decoded), raw)

    def test_manifest_binds_state_digest_and_checkpoint_digest(self) -> None:
        manifest = ReplayManifestV6.from_wire(load_manifest())
        identity = manifest.initial_identity
        self.assertEqual(identity.full_state_digest, "00" * 32)
        self.assertEqual(identity.checkpoint_codec_identity.semantic_version, "6")
        self.assertEqual(
            identity.checkpoint_digest,
            identity.recompute_checkpoint_digest(),
        )
        self.assertEqual(manifest.execution_identity, identity.execution_identity)


class V6ReplayNegativeTests(unittest.TestCase):
    def _reject_manifest(self, name: str, expected: str) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "replay-manifest.v6",
                (NEGATIVE / name).read_bytes(),
            )
        self.assertEqual(caught.exception.code, expected)

    def test_wrong_schema_is_not_migrated_from_v5(self) -> None:
        self._reject_manifest("replay-manifest-v6-wrong-schema.json", "semantic.replay_manifest")

    def test_unknown_fields_reject(self) -> None:
        self._reject_manifest("replay-manifest-v6-unknown-field.json", "decode.invalid_json")

    def test_execution_program_is_closed(self) -> None:
        self._reject_manifest(
            "replay-manifest-v6-unknown-program.json",
            "decode.invalid_json",
        )

    def test_malformed_digest_rejects(self) -> None:
        self._reject_manifest("replay-manifest-v6-wrong-digest.json", "decode.invalid_json")

    def test_semantic_contract_mismatch_rejects(self) -> None:
        self._reject_manifest(
            "replay-manifest-v6-semantic-mismatch.json",
            "semantic.replay_manifest",
        )

    def test_rules_contract_mismatch_rejects(self) -> None:
        self._reject_manifest("replay-manifest-v6-rules-mismatch.json", "semantic.replay_manifest")

    def test_comprehensive_rules_snapshot_mismatch_rejects(self) -> None:
        self._reject_manifest("replay-manifest-v6-cr-snapshot.json", "semantic.replay_manifest")

    def test_initial_execution_identity_three_way_mismatch_rejects(self) -> None:
        self._reject_manifest("replay-manifest-v6-three-way.json", "semantic.replay_manifest")

    def test_authoritative_final_execution_identity_mismatch_rejects(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "authoritative-replay.v6",
                (NEGATIVE / "authoritative-replay-v6-three-way.json").read_bytes(),
            )
        self.assertEqual(caught.exception.code, "semantic.replay")

    def test_v5_replay_is_not_reinterpreted_as_v6(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "authoritative-replay.v6",
                (NEGATIVE / "authoritative-replay-v5-presented-as-v6.json").read_bytes(),
            )
        self.assertEqual(caught.exception.code, "semantic.replay")


if __name__ == "__main__":
    unittest.main()
