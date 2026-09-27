"""Cross-language mechanical verification for detached Replay V7."""

from __future__ import annotations

import base64
import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.content_contract_v1 import decode_content_contract_manifest_v1
from mtgml.episode import EpisodeStatus
from mtgml.errors import WireError
from mtgml.persistence import (
    calculate_checkpoint_digest_v7,
    calculate_content_contract_id_v1,
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
    def test_rust_python_card_ir_decoder_shared_acceptance_vectors(self) -> None:
        vectors = _read(
            "crates/mtgml-card-ir/tests/fixtures/content_contract_manifest_parity.v1.json"
        )
        self.assertEqual(vectors["schema_version"], "content-contract-manifest-parity-v1")
        for case in vectors["cases"]:
            payload = bytes.fromhex(case["canonical_cbor_hex"])
            try:
                canonical = decode_content_contract_manifest_v1(payload)
            except WireError:
                accepted = False
            else:
                accepted = True
                self.assertEqual(canonical, payload, case["case"])
                child = ContentContractMaterialV1.from_wire(
                    {
                        "content_contract_id": calculate_content_contract_id_v1(canonical),
                        "manifest_canonical_cbor_base64": base64.b64encode(canonical).decode(
                            "ascii"
                        ),
                    }
                )
                self.assertEqual(
                    child.manifest_canonical_cbor_base64,
                    base64.b64encode(payload).decode("ascii"),
                )
            self.assertEqual(accepted, case["expected_valid"], case["case"])

    def test_phase2_v7_bytes_are_preserved_and_phase9_examples_have_distinct_identity(self) -> None:
        historical = _read("persistence/golden/m4-phase2-wire-vector-index.v1.json")
        frozen = {
            "schemas/examples/replay-manifest.v7.json": (
                "3a954d6571ed08a8967d302034c43482a3a651baa659cda35a940cbefdf4e09c"
            ),
            "schemas/examples/authoritative-replay-v7-rejected-step.json": (
                "f0f52b46732233cc368dac6ce3b365081096973f4a39ba30fd306889a01f562a"
            ),
        }
        indexed = {entry["path"]: entry["sha256"] for entry in historical["fixtures"]}
        for path, expected in frozen.items():
            raw = (ROOT / path).read_bytes()
            self.assertEqual(indexed[path], expected)
            self.assertEqual(hashlib.sha256(raw).hexdigest(), expected)
            with self.subTest(path=path), self.assertRaises((WireError, ValueError)):
                if "manifest" in path:
                    ReplayManifestV7.from_wire(json.loads(raw))
                else:
                    AuthoritativeReplayV7.from_wire(json.loads(raw))

        final_index = _read("persistence/golden/m4-phase9-replay-v7-admission-fixtures.v1.json")
        self.assertEqual(final_index["schema_version"], "m4-phase9-replay-v7-admission-fixtures.v1")
        self.assertEqual(
            final_index["historical_fixture_disposition"]["index"],
            "m4-phase2-wire-vector-index.v1.json",
        )
        self.assertTrue(final_index["historical_fixture_disposition"]["bytes_preserved"])
        self.assertFalse(final_index["historical_fixture_disposition"]["final_admission_authority"])
        final_manifest = ReplayManifestV7.from_wire(
            _read("schemas/examples/replay-manifest-v7-phase9-admitted-basic-land.json")
        )
        closure = final_manifest.semantic_contract.rules_manifest["capability_closure"]
        self.assertEqual(
            [f"{entry['key']}@{entry['version']}" for entry in closure],
            final_index["final_basic_land_capability_closure"],
        )
        for entry in final_index["fixtures"]:
            raw = (ROOT / entry["path"]).read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(), entry["sha256"])
            self.assertEqual(entry["expected_validity"], "valid-final-admission-example")

    def test_v7_examples_are_typed_and_round_trip_exact_canonical_json(self) -> None:
        manifest_raw = _read("schemas/examples/replay-manifest-v7-phase9-admitted-basic-land.json")
        replay_raw = _read(
            "schemas/examples/authoritative-replay-v7-phase9-admitted-rejected-step.json"
        )
        for contract, raw in (
            ("replay-manifest.v7", manifest_raw),
            ("authoritative-replay.v7", replay_raw),
        ):
            with self.subTest(contract=contract):
                canonical = _canonical_json(raw)
                parsed = decode_canonical(contract, canonical)
                self.assertEqual(encode_canonical(parsed), canonical)

    def test_v7_manifest_accepts_both_valid_adr_0055_pairs(self) -> None:
        magic = ReplayManifestV7.from_wire(
            _read("schemas/examples/replay-manifest-v7-phase9-admitted-basic-land.json")
        )
        self.assertEqual(magic.execution_identity.program_kind, "magic_rules")

        synthetic = copy.deepcopy(
            _read("schemas/examples/replay-manifest-v7-phase9-admitted-basic-land.json")
        )
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

    def test_phase9_complete_closure_is_required_for_basic_land_v7_codec(self) -> None:
        candidate = copy.deepcopy(
            _read("schemas/examples/replay-manifest-v7-phase9-admitted-basic-land.json")
        )
        semantic = candidate["semantic_contract"]
        closure = semantic["rules_manifest"]["capability_closure"]
        semantic["rules_manifest"]["capability_closure"] = [
            entry for entry in closure if entry["key"] != "rules/state-based-actions-combat"
        ]
        semantic["manifest"]["rules_contract_id"] = calculate_rules_contract_id_v1(
            semantic["rules_manifest"]
        )
        semantic_id = calculate_semantic_contract_id_v1(semantic["manifest"])
        semantic["semantic_contract_id"] = semantic_id
        candidate["execution_identity"]["semantic_contract_id"] = semantic_id
        candidate["initial_identity"]["execution_identity"]["semantic_contract_id"] = semantic_id
        _recompute_checkpoint(candidate)

        with self.assertRaises(WireError):
            ReplayManifestV7.from_wire(candidate)

    def test_v7_manifest_rejects_both_adr_0055_cross_pairs(self) -> None:
        base = _read("schemas/examples/replay-manifest-v7-phase9-admitted-basic-land.json")
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
        base = _read("schemas/examples/replay-manifest-v7-phase9-admitted-basic-land.json")
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
            _read("schemas/examples/authoritative-replay-v7-phase9-admitted-rejected-step.json")
        )
        self.assertEqual(replay.schema_version, "authoritative-replay.v7")


if __name__ == "__main__":
    unittest.main()
