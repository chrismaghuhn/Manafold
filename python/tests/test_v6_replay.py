"""Python mechanical verification of the V6 replay identity family."""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml._replay_v5 import calculate_rules_contract_id_v1, calculate_semantic_contract_id_v1
from mtgml.episode import EpisodeStatus
from mtgml.errors import WireError
from mtgml.persistence import calculate_checkpoint_digest_v6
from mtgml.replay import AuthoritativeReplayV6, ReplayManifestV6
from mtgml.wire import decode_canonical, encode_canonical

GOLDEN = ROOT / "wire" / "golden"
NEGATIVE = ROOT / "wire" / "negative"


def load_manifest() -> dict[str, object]:
    return json.loads((GOLDEN / "replay-manifest.v6.json").read_text(encoding="utf-8"))


def manifest_for_rules(
    authority: dict[str, object], closure: object, codec: str
) -> dict[str, object]:
    wire = load_manifest()
    rules_manifest = {"rules_authority": authority, "capability_closure": closure}
    rules_id = calculate_rules_contract_id_v1(rules_manifest)
    semantic_manifest = {
        "rules_contract_id": rules_id,
        "format_contract_id": None,
        "content_contract_id": None,
    }
    semantic_id = calculate_semantic_contract_id_v1(semantic_manifest)
    program_kind = (
        "synthetic_rules_compat" if authority["variant"] == "synthetic_legacy" else "magic_rules"
    )
    execution = {"program_kind": program_kind, "semantic_contract_id": semantic_id}
    wire["rules_snapshot"] = authority.get("snapshot_id", "synthetic-rules")
    wire["schemas"]["observation_payload_codec"] = codec
    wire["semantic_contract"] = {
        "semantic_contract_id": semantic_id,
        "manifest": semantic_manifest,
        "rules_manifest": rules_manifest,
    }
    wire["execution_identity"] = execution
    initial = wire["initial_identity"]
    initial["execution_identity"] = execution
    identity = initial
    identity["checkpoint_digest"] = calculate_checkpoint_digest_v6(
        identity["full_state_digest"],
        EpisodeStatus.from_wire(identity["episode_status"]),
        identity["environment_limit_counters"],
        identity["checkpoint_codec_identity"]["codec_id"],
        identity["checkpoint_codec_identity"]["semantic_version"],
        execution["program_kind"],
        semantic_id,
    )
    return wire


def schema_codec_cases() -> list[tuple[str, dict[str, object], object, str, bool]]:
    cr = {"variant": "comprehensive_rules", "snapshot_id": "cr:test"}
    combat_closure = [
        {"key": "rules/basic-priority", "version": "0.1.0"},
        {"key": "rules/combat-phase", "version": "0.1.0"},
        {"key": "rules/declare-attackers", "version": "0.1.0"},
        {"key": "rules/draw-card", "version": "0.1.0"},
        {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
        {"key": "rules/turn-structure", "version": "0.1.0"},
        {"key": "rules/zone-incarnation", "version": "0.1.0"},
    ]
    combat_blockers_closure = [
        {"key": "rules/basic-priority", "version": "0.1.0"},
        {"key": "rules/combat-phase", "version": "0.1.0"},
        {"key": "rules/declare-attackers", "version": "0.1.0"},
        {"key": "rules/declare-blockers", "version": "0.1.0"},
        {"key": "rules/draw-card", "version": "0.1.0"},
        {"key": "rules/state-based-actions-combat", "version": "0.1.0"},
        {"key": "rules/turn-structure", "version": "0.1.0"},
        {"key": "rules/zone-incarnation", "version": "0.1.0"},
    ]
    return [
        (
            "synthetic",
            {"variant": "synthetic_legacy"},
            None,
            "synthetic-m3-observation.v1",
            True,
        ),
        (
            "cr_s1",
            cr,
            [{"key": "rules/turn-structure", "version": "0.1.0"}],
            "synthetic-m3-observation.v1",
            True,
        ),
        (
            "cr_sba",
            cr,
            [{"key": "rules/state-based-actions-combat", "version": "0.1.0"}],
            "magic-m3-observation.v1",
            True,
        ),
        (
            "cr_combat_attackers",
            cr,
            combat_closure,
            "magic-combat-observation.v2",
            True,
        ),
        (
            "cr_combat_blockers",
            cr,
            combat_blockers_closure,
            "magic-combat-observation.v3",
            True,
        ),
        (
            "cr_combat_blockers_wrong_codec",
            cr,
            combat_blockers_closure,
            "magic-combat-observation.v2",
            False,
        ),
        (
            "cr_combat_attackers_wrong_codec",
            cr,
            combat_closure,
            "magic-combat-observation.v3",
            False,
        ),
        (
            "cr_sba_synthetic",
            cr,
            [{"key": "rules/state-based-actions-combat", "version": "0.1.0"}],
            "synthetic-m3-observation.v1",
            False,
        ),
        (
            "cr_wrong_sba_magic",
            cr,
            [{"key": "rules/state-based-actions-combat", "version": "999.0.0"}],
            "magic-m3-observation.v1",
            False,
        ),
    ]


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

    def test_observation_codec_binding_requires_exact_authority_and_capability_version(
        self,
    ) -> None:
        cases = (
            (
                {"variant": "synthetic_legacy"},
                None,
                "synthetic-m3-observation.v1",
                True,
            ),
            (
                {"variant": "comprehensive_rules", "snapshot_id": "cr:test"},
                [{"key": "rules/turn-structure", "version": "0.1.0"}],
                "synthetic-m3-observation.v1",
                True,
            ),
            (
                {"variant": "comprehensive_rules", "snapshot_id": "cr:test"},
                [{"key": "rules/state-based-actions-combat", "version": "0.1.0"}],
                "magic-m3-observation.v1",
                True,
            ),
            (
                {"variant": "comprehensive_rules", "snapshot_id": "cr:test"},
                [{"key": "rules/state-based-actions-combat", "version": "0.1.0"}],
                "synthetic-m3-observation.v1",
                False,
            ),
            (
                {"variant": "comprehensive_rules", "snapshot_id": "cr:test"},
                [{"key": "rules/state-based-actions-combat", "version": "999.0.0"}],
                "magic-m3-observation.v1",
                False,
            ),
        )
        for authority, closure, codec, accepted in cases:
            with self.subTest(authority=authority, closure=closure, codec=codec):
                candidate = manifest_for_rules(authority, closure, codec)
                if accepted:
                    ReplayManifestV6.from_wire(candidate)
                else:
                    with self.assertRaises(WireError):
                        ReplayManifestV6.from_wire(candidate)

    def test_synthetic_authority_fake_sba_closure_cannot_claim_magic_codec(self) -> None:
        candidate = load_manifest()
        candidate["schemas"]["observation_payload_codec"] = "magic-m3-observation.v1"
        candidate["semantic_contract"]["rules_manifest"]["capability_closure"] = [
            {"key": "rules/state-based-actions-combat", "version": "0.1.0"}
        ]
        with self.assertRaises(WireError):
            ReplayManifestV6.from_wire(candidate)

    def test_codec_binding_wire_negative_fixtures_are_registered(self) -> None:
        self._reject_manifest(
            "replay-manifest-v6-sba-synthetic-codec.json",
            "semantic.replay_manifest",
        )
        self._reject_manifest(
            "replay-manifest-v6-wrong-sba-version-magic-codec.json",
            "semantic.replay_manifest",
        )
        self._reject_manifest(
            "replay-manifest-v6-synthetic-authority-magic-codec.json",
            "semantic.replay_manifest",
        )

    def test_unknown_observation_codec_rejects(self) -> None:
        candidate = load_manifest()
        candidate["schemas"]["observation_payload_codec"] = "unknown-observation.v1"
        with self.assertRaises(WireError):
            ReplayManifestV6.from_wire(candidate)

    def test_json_schemas_enforce_the_same_observation_codec_relation(self) -> None:
        try:
            import jsonschema
        except ImportError:  # pragma: no cover - locked dev tools include jsonschema
            self.skipTest("jsonschema is not installed")

        manifest_schema = json.loads(
            (ROOT / "schemas" / "replay-manifest.v6.schema.json").read_text(encoding="utf-8")
        )
        replay_schema = json.loads(
            (ROOT / "schemas" / "authoritative-replay.v6.schema.json").read_text(encoding="utf-8")
        )
        authoritative = json.loads(
            (GOLDEN / "authoritative-replay-empty.v6.json").read_text(encoding="utf-8")
        )
        for name, authority, closure, codec, accepted in schema_codec_cases():
            with self.subTest(case=name):
                candidate = manifest_for_rules(authority, closure, codec)
                manifest_valid = jsonschema.Draft202012Validator(manifest_schema).is_valid(
                    candidate
                )
                replay_candidate = dict(authoritative)
                replay_candidate["manifest"] = candidate
                replay_candidate["final_identity"] = candidate["initial_identity"]
                replay_valid = jsonschema.Draft202012Validator(replay_schema).is_valid(
                    replay_candidate
                )
                self.assertEqual(manifest_valid, accepted)
                self.assertEqual(replay_valid, accepted)

        fake_synthetic = load_manifest()
        fake_synthetic["schemas"]["observation_payload_codec"] = "magic-m3-observation.v1"
        fake_synthetic["semantic_contract"]["rules_manifest"]["capability_closure"] = [
            {"key": "rules/state-based-actions-combat", "version": "0.1.0"}
        ]
        self.assertFalse(jsonschema.Draft202012Validator(manifest_schema).is_valid(fake_synthetic))
        replay_fake_synthetic = dict(authoritative)
        replay_fake_synthetic["manifest"] = fake_synthetic
        replay_fake_synthetic["final_identity"] = fake_synthetic["initial_identity"]
        self.assertFalse(
            jsonschema.Draft202012Validator(replay_schema).is_valid(replay_fake_synthetic)
        )


if __name__ == "__main__":
    unittest.main()
