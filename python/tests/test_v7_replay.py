"""Cross-language mechanical verification for detached Replay V7."""

from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.episode import EpisodeStatus
from mtgml.errors import WireError
from mtgml.persistence import (
    calculate_checkpoint_digest_v7,
    calculate_rules_contract_id_v1,
    calculate_semantic_contract_id_v1,
)
from mtgml.replay import (
    AuthoritativeReplayV7,
    ContentContractMaterialV1,
    ReplayManifestV7,
    SemanticContractMaterialV7,
)
from mtgml.wire import decode_canonical, encode_canonical


def _read(relative: str) -> object:
    return json.loads((ROOT / relative).read_text(encoding="utf-8"))


def _canonical_json(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode()


def _recompute_checkpoint(raw: dict[str, object]) -> None:
    identity = raw["initial_identity"]
    assert isinstance(identity, dict)
    codec = identity["checkpoint_codec_identity"]
    execution = identity["execution_identity"]
    counters = identity["environment_limit_counters"]
    status = EpisodeStatus.from_wire(identity["episode_status"])
    identity["checkpoint_digest"] = calculate_checkpoint_digest_v7(
        identity["full_state_digest"],
        status,
        counters,
        codec["codec_id"],
        codec["semantic_version"],
        execution["program_kind"],
        execution["semantic_contract_id"],
    )


class ReplayV7Tests(unittest.TestCase):
    def test_v7_examples_are_typed_and_round_trip_exact_canonical_json(self) -> None:
        manifest_raw = _read("schemas/examples/replay-manifest.v7.json")
        replay_raw = _read("schemas/examples/authoritative-replay-v7-rejected-step.json")
        for contract, raw in (
            ("replay-manifest.v7", manifest_raw),
            ("authoritative-replay.v7", replay_raw),
        ):
            with self.subTest(contract=contract):
                canonical = _canonical_json(raw)
                parsed = decode_canonical(contract, canonical)
                self.assertEqual(encode_canonical(parsed), canonical)

    def test_v7_manifest_accepts_both_valid_adr_0055_pairs(self) -> None:
        magic = ReplayManifestV7.from_wire(_read("schemas/examples/replay-manifest.v7.json"))
        self.assertEqual(magic.execution_identity.program_kind, "magic_rules")

        synthetic = copy.deepcopy(_read("schemas/examples/replay-manifest.v7.json"))
        rules = {"rules_authority": {"variant": "synthetic_legacy"}, "capability_closure": None}
        rules_id = calculate_rules_contract_id_v1(rules)
        semantic = {
            "rules_contract_id": rules_id,
            "format_contract_id": None,
            "content_contract_id": None,
        }
        semantic_id = calculate_semantic_contract_id_v1(semantic)
        execution = {"program_kind": "synthetic_rules_compat", "semantic_contract_id": semantic_id}
        synthetic["semantic_contract"] = {
            "semantic_contract_id": semantic_id,
            "manifest": semantic,
            "rules_manifest": rules,
            "content_contract": None,
        }
        synthetic["execution_identity"] = execution
        identity = synthetic["initial_identity"]
        identity["execution_identity"] = execution
        synthetic["schemas"]["observation_payload_codec"] = "synthetic-m3-observation.v1"
        _recompute_checkpoint(synthetic)
        self.assertEqual(
            ReplayManifestV7.from_wire(synthetic).execution_identity.program_kind,
            "synthetic_rules_compat",
        )

    def test_v7_manifest_rejects_both_adr_0055_cross_pairs(self) -> None:
        base = _read("schemas/examples/replay-manifest.v7.json")
        mutations = []

        magic_synthetic = copy.deepcopy(base)
        rules = {"rules_authority": {"variant": "synthetic_legacy"}, "capability_closure": None}
        rules_id = calculate_rules_contract_id_v1(rules)
        semantic = {
            "rules_contract_id": rules_id,
            "format_contract_id": None,
            "content_contract_id": None,
        }
        semantic_id = calculate_semantic_contract_id_v1(semantic)
        magic_synthetic["semantic_contract"] = {
            "semantic_contract_id": semantic_id,
            "manifest": semantic,
            "rules_manifest": rules,
            "content_contract": None,
        }
        magic_synthetic["execution_identity"]["semantic_contract_id"] = semantic_id
        magic_synthetic["initial_identity"]["execution_identity"]["semantic_contract_id"] = (
            semantic_id
        )
        magic_synthetic["schemas"]["observation_payload_codec"] = "synthetic-m3-observation.v1"
        _recompute_checkpoint(magic_synthetic)
        mutations.append(magic_synthetic)

        synthetic_magic = copy.deepcopy(base)
        synthetic_magic["execution_identity"]["program_kind"] = "synthetic_rules_compat"
        synthetic_magic["initial_identity"]["execution_identity"]["program_kind"] = (
            "synthetic_rules_compat"
        )
        _recompute_checkpoint(synthetic_magic)
        mutations.append(synthetic_magic)

        for candidate in mutations:
            with (
                self.subTest(candidate=candidate),
                self.assertRaises((WireError, TypeError, ValueError)),
            ):
                ReplayManifestV7.from_wire(candidate)

    def test_phase2_replay_child_negative_vectors_reject(self) -> None:
        vectors = _read("schemas/negative/m4-phase2-replay-child-semantic-negatives.json")
        base = _read("schemas/examples/replay-manifest.v7.json")
        for case in vectors["cases"]:
            candidate = copy.deepcopy(base)
            candidate["semantic_contract"]["content_contract"] = case["content_contract"]
            candidate["semantic_contract"]["manifest"]["content_contract_id"] = case[
                "parent_content_contract_id"
            ]
            with self.subTest(case=case["case"]), self.assertRaises((WireError, ValueError)):
                ReplayManifestV7.from_wire(candidate)

    def test_all_replay_v7_child_wire_negatives_reject_in_python(self) -> None:
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
                    _read(f"schemas/negative/replay-v7-content-child-{name}.json")
                )

        presence = _read("schemas/negative/replay-v7-content-presence-mismatch.json")
        with self.assertRaises(WireError):
            SemanticContractMaterialV7.from_wire(presence)

    def test_duplicate_child_fields_reject_at_raw_json_boundary(self) -> None:
        raw = (
            ROOT / "schemas/negative/replay-v7-content-child-duplicate-field.jsonraw"
        ).read_bytes()
        from mtgml.wire import decode_canonical

        with self.assertRaises(WireError):
            decode_canonical("replay-manifest.v7", raw)

    def test_authoritative_replay_v7_preserves_historical_v6_decoder(self) -> None:
        replay = AuthoritativeReplayV7.from_wire(
            _read("schemas/examples/authoritative-replay-v7-rejected-step.json")
        )
        self.assertEqual(replay.schema_version, "authoritative-replay.v7")


if __name__ == "__main__":
    unittest.main()
