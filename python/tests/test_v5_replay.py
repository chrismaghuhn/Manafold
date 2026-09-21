"""Task 11 contract tests: Python mirror of V5 replay DTOs (spec §6, §9).

Round-trips the golden V5 fixtures (`wire/golden/replay-manifest.v5.json` and
`wire/golden/authoritative-replay-empty.v5.json`) and asserts every §19
negative V5 replay fixture is rejected with its expected error code.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.errors import WireError
from mtgml.replay import (
    AuthoritativeReplayV5,
    ExecutionIdentityV1,
    ReplayManifestV5,
)
from mtgml.wire import decode_canonical, encode_canonical

GOLDEN = ROOT / "wire" / "golden"
NEGATIVE = ROOT / "wire" / "negative"


def _v5_golden_manifest() -> dict[str, object]:
    return json.loads((GOLDEN / "replay-manifest.v5.json").read_text(encoding="utf-8"))


def _v5_golden_replay() -> dict[str, object]:
    return json.loads((GOLDEN / "authoritative-replay-empty.v5.json").read_text(encoding="utf-8"))


class V5ExecutionIdentityTests(unittest.TestCase):
    def test_known_program_kinds_roundtrip(self) -> None:
        for program in ("synthetic_rules_compat", "magic_rules"):
            identity = ExecutionIdentityV1(
                program_kind=program,
                semantic_contract_id="11" * 32,
            )
            wire = identity.to_wire()
            self.assertEqual(wire["program_kind"], program)
            roundtrip = ExecutionIdentityV1.from_wire(wire)
            self.assertEqual(roundtrip, identity)

    def test_unknown_program_kind_rejects(self) -> None:
        with self.assertRaises(WireError):
            ExecutionIdentityV1.from_wire(
                {"program_kind": "bogus", "semantic_contract_id": "11" * 32}
            )

    def test_malformed_contract_id_rejects(self) -> None:
        for bad in ("too_short", "5a" * 31, "5A" * 32):
            with self.subTest(bad=bad), self.assertRaises(WireError):
                ExecutionIdentityV1.from_wire(
                    {
                        "program_kind": "synthetic_rules_compat",
                        "semantic_contract_id": bad,
                    }
                )


class V5GoldenFixtureTests(unittest.TestCase):
    def test_replay_manifest_v5_roundtrips(self) -> None:
        manifest_bytes = (GOLDEN / "replay-manifest.v5.json").read_bytes()
        decoded = decode_canonical("replay-manifest.v5", manifest_bytes)
        self.assertIsInstance(decoded, ReplayManifestV5)
        self.assertEqual(encode_canonical(decoded), manifest_bytes)

    def test_authoritative_replay_v5_roundtrips(self) -> None:
        replay_bytes = (GOLDEN / "authoritative-replay-empty.v5.json").read_bytes()
        decoded = decode_canonical("authoritative-replay.v5", replay_bytes)
        self.assertIsInstance(decoded, AuthoritativeReplayV5)
        self.assertEqual(encode_canonical(decoded), replay_bytes)

    def test_manifest_execution_identity_matches_initial_identity(self) -> None:
        manifest = _v5_golden_manifest()
        decoded = ReplayManifestV5.from_wire(manifest)
        self.assertEqual(
            decoded.execution_identity,
            decoded.initial_identity.execution_identity,
        )

    def test_manifest_semantic_contract_is_bound_to_execution_identity(self) -> None:
        manifest = _v5_golden_manifest()
        decoded = ReplayManifestV5.from_wire(manifest)
        self.assertEqual(
            decoded.semantic_contract.semantic_contract_id,
            decoded.execution_identity.semantic_contract_id,
        )


class V5NegativeFixtureTests(unittest.TestCase):
    def test_unknown_program_fails_at_decode_or_semantic(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "replay-manifest.v5",
                (NEGATIVE / "replay-manifest-v5-unknown-program.json").read_bytes(),
            )
        self.assertIn(caught.exception.code, {"decode.invalid_json", "semantic.replay_manifest"})

    def test_unknown_field_fails_at_decode(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "replay-manifest.v5",
                (NEGATIVE / "replay-manifest-v5-unknown-field.json").read_bytes(),
            )
        self.assertEqual(caught.exception.code, "decode.invalid_json")

    def test_wrong_schema_version_fails_at_semantic(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "replay-manifest.v5",
                (NEGATIVE / "replay-manifest-v5-wrong-schema.json").read_bytes(),
            )
        self.assertEqual(caught.exception.code, "semantic.replay_manifest")

    def test_wrong_digest_length_fails_at_decode_or_semantic(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "replay-manifest.v5",
                (NEGATIVE / "replay-manifest-v5-wrong-digest.json").read_bytes(),
            )
        self.assertIn(caught.exception.code, {"decode.invalid_json", "semantic.replay_manifest"})

    def test_semantic_mismatch_fails_at_semantic(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "replay-manifest.v5",
                (NEGATIVE / "replay-manifest-v5-semantic-mismatch.json").read_bytes(),
            )
        self.assertEqual(caught.exception.code, "semantic.replay_manifest")

    def test_rules_mismatch_fails_at_semantic(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "replay-manifest.v5",
                (NEGATIVE / "replay-manifest-v5-rules-mismatch.json").read_bytes(),
            )
        self.assertEqual(caught.exception.code, "semantic.replay_manifest")

    def test_cr_snapshot_mismatch_fails_at_semantic(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "replay-manifest.v5",
                (NEGATIVE / "replay-manifest-v5-cr-snapshot.json").read_bytes(),
            )
        self.assertEqual(caught.exception.code, "semantic.replay_manifest")

    def test_manifest_three_way_identity_mismatch_fails(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "replay-manifest.v5",
                (NEGATIVE / "replay-manifest-v5-three-way.json").read_bytes(),
            )
        self.assertEqual(caught.exception.code, "semantic.replay_manifest")

    def test_authoritative_replay_three_way_fails(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_canonical(
                "authoritative-replay.v5",
                (NEGATIVE / "authoritative-replay-v5-three-way.json").read_bytes(),
            )
        self.assertEqual(caught.exception.code, "semantic.replay")


class V5NestedContractObjectStrictnessTests(unittest.TestCase):
    """Nested contract manifests must be JSON objects, not arrays of pairs.

    Rust serde and the V5 JSON schemas require objects at these positions.
    Python's dict() constructor silently accepts array-of-pairs, which we
    must reject at decode time (decode.invalid_json).
    """

    def test_manifest_as_array_of_pairs_is_rejected(self) -> None:
        manifest = _v5_golden_manifest()
        manifest["semantic_contract"]["manifest"] = [
            ["rules_contract_id", manifest["semantic_contract"]["manifest"]["rules_contract_id"]],
            ["format_contract_id", None],
            ["content_contract_id", None],
        ]
        with self.assertRaises(WireError) as caught:
            ReplayManifestV5.from_wire(manifest)
        self.assertEqual(caught.exception.code, "decode.invalid_json")

    def test_rules_manifest_as_array_of_pairs_is_rejected(self) -> None:
        manifest = _v5_golden_manifest()
        original = manifest["semantic_contract"]["rules_manifest"]
        manifest["semantic_contract"]["rules_manifest"] = [
            ["rules_authority", original["rules_authority"]],
            ["capability_closure", None],
        ]
        with self.assertRaises(WireError) as caught:
            ReplayManifestV5.from_wire(manifest)
        self.assertEqual(caught.exception.code, "decode.invalid_json")

    def test_manifest_as_non_object_primitive_is_rejected(self) -> None:
        manifest = _v5_golden_manifest()
        manifest["semantic_contract"]["manifest"] = "not-an-object"
        with self.assertRaises(WireError) as caught:
            ReplayManifestV5.from_wire(manifest)
        self.assertEqual(caught.exception.code, "decode.invalid_json")


class V5CheckpointIdentityConsistencyTests(unittest.TestCase):
    def test_manifest_initial_identity_checkpoint_digest_is_consistent(self) -> None:
        manifest = _v5_golden_manifest()
        decoded = ReplayManifestV5.from_wire(manifest)
        # The golden fixture was generated by the Rust V5 recorder, so the
        # embedded checkpoint digest must be the detached recomputation of
        # the visible identity fields.
        recomputed = decoded.initial_identity.recompute_checkpoint_digest()
        self.assertEqual(
            recomputed,
            decoded.initial_identity.checkpoint_digest,
        )


if __name__ == "__main__":
    unittest.main()
