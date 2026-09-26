from __future__ import annotations

import base64
import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).parent))

from m4_phase2_reference import (
    decode,
    digest_envelope,
    encode,
    validate_authoritative_state,
    validate_basic_land_content_manifest,
    verify_content_child,
)


def read_json(path: str) -> Any:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def v6_input(family: list[Any]) -> list[Any]:
    base_path = ROOT / "crates/mtgml-state/tests/fixtures/magic-sba-graveyard-order-v5-input.hex"
    baseline_bytes = bytes.fromhex(base_path.read_text(encoding="ascii").strip())
    baseline = decode(baseline_bytes)
    if len(baseline) != 13 or encode(baseline) != baseline_bytes:
        raise AssertionError("frozen V5 baseline input is not canonical")
    return [
        "full-state-digest-input.v6",
        "mtgml.full-state-digest.v6",
        *baseline[2:],
        family,
    ]


def digest_v6(family: list[Any]) -> tuple[bytes, str]:
    return digest_envelope(
        "mtgml.full-state-digest.v6",
        "full-state-digest-input.v6",
        encode(v6_input(family)),
    )


def checkpoint_value(full_state_digest: str, semantic_contract_id: str) -> list[Any]:
    full_state_ref = [
        "mtgml.digest-envelope.v1",
        "sha-256",
        "mtgml.full-state-digest.v6",
        "mtgml.canonical-cbor.v1",
        "full-state-digest-input.v6",
        bytes.fromhex(full_state_digest),
    ]
    return [
        "environment-checkpoint-digest-input.v7",
        "mtgml.checkpoint-digest.v7",
        full_state_ref,
        ["running", None],
        [0, 0, 0, 0, 0],
        ["in-memory-reference", "7"],
        [["magic_rules", None], bytes.fromhex(semantic_contract_id)],
    ]


def _mutated_families(base: list[Any]) -> dict[str, list[Any]]:
    operations = {"mana.player": lambda s: s[1][0].__setitem__(0, 3)}
    colors = ("white", "blue", "black", "red", "green", "colorless")
    for index, color in enumerate(colors):
        operations[f"mana.unrestricted.{color}"] = lambda s, i=index: s[1][0][1].__setitem__(
            i, s[1][0][1][i] + 1
        )
        operations[f"mana.creature_spell_only.{color}"] = lambda s, i=index: s[1][0][2].__setitem__(
            i, s[1][0][2][i] + 1
        )
    operations.update(
        {
            "mana.restriction": lambda s: (
                s[1][0][1].__setitem__(0, 2),
                s[1][0][2].__setitem__(0, 0),
            ),
            "mana.u32_boundary": lambda s: s[1][1][2].__setitem__(5, 0xFFFFFFFE),
            "mana.explicit_zero": lambda s: s[1][1][1].__setitem__(0, 1),
            "history.turn_number": lambda s: s[2].__setitem__(0, 2),
            "history.land_plays_used": lambda s: s[2][1][0].__setitem__(1, 0),
            "history.spells_cast_total": lambda s: s[2][1][0].__setitem__(2, 3),
            "history.noncreature_spells_cast": lambda s: s[2][1][0].__setitem__(3, 2),
            "history.lost_life": lambda s: s[2][1][0].__setitem__(4, False),
            "history.red_noncombat_damage": lambda s: s[2][1][0].__setitem__(5, 4),
            "history.permanent_to_graveyard": lambda s: s[2][1][0].__setitem__(6, False),
            "history.target_object": lambda s: s[2][2][0].__setitem__(0, 3),
            "history.target_controller": lambda s: s[2][2][0].__setitem__(1, 1),
            "history.once_ability_object": lambda s: s[2][3][0].__setitem__(0, 2),
            "history.once_ability_key": lambda s: s[2][3][0].__setitem__(1, 1),
            "counter.object": lambda s: s[3][0].__setitem__(0, 2),
            "counter.kind": lambda s: s[3][0][1][0].__setitem__(0, 1),
            "counter.count": lambda s: s[3][0][1][0].__setitem__(1, 3),
            "attachment.source": lambda s: s[4][0].__setitem__(0, 4),
            "attachment.target": lambda s: s[4][0].__setitem__(1, 2),
            "attachment.timestamp_revision": lambda s: s[4][0].__setitem__(2, 3),
            "attachment.timestamp_operation": lambda s: s[4][0].__setitem__(3, 1),
            "face.object": lambda s: s[5][0].__setitem__(0, 2),
            "face.key": lambda s: s[5][0].__setitem__(1, 1),
            "ability.instance": lambda s: s[6][0].__setitem__(0, 2),
            "ability.source": lambda s: s[6][0].__setitem__(1, 2),
            "ability.key": lambda s: s[6][0].__setitem__(2, 1),
        }
    )
    results = {}
    for name, operation in operations.items():
        changed = copy.deepcopy(base)
        operation(changed)
        results[name] = changed
    return results


class FullStateV6VectorTests(unittest.TestCase):
    def test_v6_canonical_preimage_and_digest_match_independent_kat(self) -> None:
        vector = read_json("persistence/golden/full-state-digest-v6-kat.v1.json")
        family = vector["card_rules_authoritative_state"]
        validate_authoritative_state(family)
        payload = encode(v6_input(family))
        self.assertEqual(payload.hex(), vector["canonical_payload_hex"])
        _, digest = digest_envelope(vector["semantic_domain"], vector["input_schema"], payload)
        self.assertEqual(digest, vector["expected_digest"])
        self.assertEqual(
            vector["mana_color_order"], ["white", "blue", "black", "red", "green", "colorless"]
        )
        self.assertEqual(vector["mana_restriction_order"], ["unrestricted", "creature_spell_only"])

    def test_every_authoritative_field_has_a_frozen_mutation_digest(self) -> None:
        vector = read_json("persistence/golden/full-state-digest-v6-kat.v1.json")
        family = vector["card_rules_authoritative_state"]
        mutations = _mutated_families(family)
        self.assertEqual(set(mutations), set(vector["mutation_digests"]))
        for name, changed in mutations.items():
            with self.subTest(field=name):
                _, digest = digest_v6(changed)
                self.assertEqual(digest, vector["mutation_digests"][name])
                self.assertNotEqual(digest, vector["expected_digest"])

    def test_successor_shape_negative_vectors_reject(self) -> None:
        data = read_json("schemas/negative/m4-phase2-v6-state-shapes.json")
        self.assertGreaterEqual(len(data["cases"]), 14)
        for case in data["cases"]:
            with self.subTest(case=case["case"]), self.assertRaises(ValueError):
                validate_authoritative_state(case["family"])

    def test_fixed_positional_orders_and_legacy_vectors_are_frozen(self) -> None:
        vector = read_json("persistence/golden/full-state-digest-v6-kat.v1.json")
        family = vector["card_rules_authoritative_state"]
        self.assertEqual(family[1][0][1], [1, 1, 1, 1, 1, 1])
        self.assertEqual(family[1][0][2], [1, 1, 1, 1, 1, 1])
        self.assertEqual(family[1][1][2][5], 0xFFFFFFFF)
        self.assertEqual([entry[0] for entry in family[3][0][1]], [0, 2])
        self.assertEqual([entry[0] for entry in family[4]], sorted(entry[0] for entry in family[4]))
        hashes = read_json("persistence/golden/m4-phase2-historical-byte-invariance.v1.json")
        for entry in hashes["files"]:
            with self.subTest(path=entry["path"]):
                raw = (ROOT / entry["path"]).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(), entry["sha256"])


class CheckpointV7VectorTests(unittest.TestCase):
    def test_checkpoint_v7_seven_element_preimage_kat(self) -> None:
        vector = read_json("persistence/golden/checkpoint-digest-v7-kat.v1.json")
        value = checkpoint_value(vector["full_state_digest"], vector["semantic_contract_id"])
        payload = encode(value)
        self.assertEqual(payload.hex(), vector["canonical_payload_hex"])
        _, digest = digest_envelope(
            "mtgml.checkpoint-digest.v7",
            "environment-checkpoint-digest-input.v7",
            payload,
        )
        self.assertEqual(digest, vector["expected_digest"])
        self.assertEqual(len(value), 7)

    def test_checkpoint_identity_mutations_change_kat(self) -> None:
        vector = read_json("persistence/golden/checkpoint-digest-v7-kat.v1.json")
        base = checkpoint_value(vector["full_state_digest"], vector["semantic_contract_id"])
        changes = {
            "full_state_digest": lambda x: x[2].__setitem__(
                5, bytes([x[2][5][0] ^ 1]) + x[2][5][1:]
            ),
            "status": lambda x: x.__setitem__(3, ["truncated", ["external_stop", []]]),
            "counter": lambda x: x[4].__setitem__(0, 1),
            "semantic_contract_id": lambda x: x.__setitem__(
                6, [x[6][0], bytes([x[6][1][0] ^ 1]) + x[6][1][1:]]
            ),
            "codec_version": lambda x: x[5].__setitem__(1, "6"),
        }
        self.assertEqual(set(changes), set(vector["mutation_digests"]))
        for name, operation in changes.items():
            changed = copy.deepcopy(base)
            operation(changed)
            with self.subTest(field=name):
                _, digest = digest_envelope(
                    "mtgml.checkpoint-digest.v7",
                    "environment-checkpoint-digest-input.v7",
                    encode(changed),
                )
                self.assertEqual(digest, vector["mutation_digests"][name])
                self.assertNotEqual(digest, vector["expected_digest"])


class ContentAndReplayV7VectorTests(unittest.TestCase):
    def test_profiled_content_identity_and_semantic_contract_chain(self) -> None:
        vector = read_json("persistence/golden/content-contract-basic-land-v1-kat.v1.json")
        content_value = vector["content_manifest_value"]
        content_payload = validate_basic_land_content_manifest(content_value)
        self.assertEqual(content_payload.hex(), vector["canonical_payload_hex"])
        _, content_id = digest_envelope(
            "mtgml.content-contract.v1", "content-contract-manifest.v1", content_payload
        )
        self.assertEqual(content_id, vector["content_contract_id"])

        rules_value = [
            "rules-contract-manifest.v1",
            "mtgml.rules-contract.v1",
            vector["rules_manifest_value"][2],
            vector["rules_manifest_value"][3],
        ]
        rules_payload = encode(rules_value)
        self.assertEqual(rules_payload.hex(), vector["rules_manifest_canonical_payload_hex"])
        _, rules_id = digest_envelope(
            "mtgml.rules-contract.v1", "rules-contract-manifest.v1", rules_payload
        )
        self.assertEqual(rules_id, vector["rules_contract_id"])

        semantic_value = [
            "semantic-contract-manifest.v1",
            "mtgml.semantic-contract.v1",
            bytes.fromhex(rules_id),
            None,
            bytes.fromhex(content_id),
        ]
        semantic_payload = encode(semantic_value)
        self.assertEqual(semantic_payload.hex(), vector["semantic_manifest_canonical_payload_hex"])
        _, semantic_id = digest_envelope(
            "mtgml.semantic-contract.v1", "semantic-contract-manifest.v1", semantic_payload
        )
        self.assertEqual(semantic_id, vector["semantic_contract_id"])

    def test_replay_v7_content_child_transport_is_exact_and_bound(self) -> None:
        vector = read_json("schemas/examples/replay-manifest.v7.json")
        child = vector["semantic_contract"]["content_contract"]
        content = read_json("persistence/golden/content-contract-basic-land-v1-kat.v1.json")
        self.assertEqual(
            verify_content_child(child, content["content_contract_id"]),
            content["content_contract_id"],
        )
        self.assertEqual(
            base64.b64decode(child["manifest_canonical_cbor_base64"], validate=True).hex(),
            content["canonical_payload_hex"],
        )

    def test_content_child_semantic_negative_vectors_reject(self) -> None:
        data = read_json("schemas/negative/m4-phase2-replay-child-semantic-negatives.json")
        for case in data["cases"]:
            with self.subTest(case=case["case"]), self.assertRaises(ValueError):
                verify_content_child(case["content_contract"], case["parent_content_contract_id"])

    def test_duplicate_content_child_json_key_is_rejected_before_mapping(self) -> None:
        raw = (
            ROOT / "schemas/negative/replay-v7-content-child-duplicate-field.jsonraw"
        ).read_text()

        def no_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
            result = {}
            for key, value in pairs:
                if key in result:
                    raise ValueError(f"duplicate object key: {key}")
                result[key] = value
            return result

        with self.assertRaisesRegex(ValueError, "duplicate object key"):
            json.loads(raw, object_pairs_hook=no_duplicate_pairs)

    def test_profile_body_or_rule_relevant_characteristics_change_content_id(self) -> None:
        vector = read_json("persistence/golden/content-contract-basic-land-v1-kat.v1.json")
        baseline = vector["content_manifest_value"]
        _, base_id = digest_envelope(
            "mtgml.content-contract.v1", "content-contract-manifest.v1", encode(baseline)
        )
        for mutation in ("profile", "subtype", "type_line"):
            changed = copy.deepcopy(baseline)
            definition = changed[2][0]
            if mutation == "profile":
                definition[4][1][0] = "basic-land@1.0.1"
            elif mutation == "subtype":
                definition[4][1][1][1] = "plains"
            else:
                definition[2][0][1][3][2] = ["Plains"]
            _, actual = digest_envelope(
                "mtgml.content-contract.v1", "content-contract-manifest.v1", encode(changed)
            )
            with self.subTest(mutation=mutation):
                self.assertNotEqual(actual, base_id)


class DecisionAndPublicFixtureTests(unittest.TestCase):
    def test_future_consumer_wire_vectors_match_frozen_exact_bytes(self) -> None:
        index = read_json("persistence/golden/m4-phase2-wire-vector-index.v1.json")
        self.assertEqual(index["schema_version"], "m4-phase2-wire-vector-index.v1")
        self.assertEqual(len(index["fixtures"]), 16)
        for fixture in index["fixtures"]:
            raw = (ROOT / fixture["path"]).read_bytes()
            with self.subTest(path=fixture["path"]):
                self.assertEqual(fixture["expected_validity"], "valid-schema-example")
                self.assertEqual(hashlib.sha256(raw).hexdigest(), fixture["sha256"])

    def test_play_land_order_is_fixed_and_candidate_ids_are_dense(self) -> None:
        request = read_json("schemas/examples/player-decision-request-v3-ordering.json")
        intents = [candidate["intent"]["kind"] for candidate in request["candidates"]]
        self.assertEqual(intents, ["pass_priority", "play_land", "cast_spell", "activate_ability"])
        self.assertEqual([c["candidate_id"] for c in request["candidates"]], [0, 1, 2, 3])
        negatives = read_json("schemas/negative/m4-phase2-decision-v3-semantic-negatives.json")
        self.assertEqual(len(negatives["cases"]), 3)
        for case in negatives["cases"]:
            ids = [candidate["candidate_id"] for candidate in case["request"]["candidates"]]
            intents = [candidate["intent"]["kind"] for candidate in case["request"]["candidates"]]
            with self.subTest(case=case["case"]):
                self.assertTrue(
                    len(ids) != len(set(ids))
                    or ids != list(range(len(ids)))
                    or intents != ["pass_priority", "play_land", "cast_spell", "activate_ability"]
                )

    def test_observed_event_and_step_face_at_entry_vectors(self) -> None:
        event = read_json("schemas/examples/observed-event-v3-entry-back-tapped.json")
        self.assertEqual(event["event"]["kind"], "object_moved")
        self.assertEqual(event["event"]["entering_face"], "back")
        self.assertIs(event["event"]["tapped"], True)
        self.assertNotIn("transform", [event["event"]["kind"]])
        step = read_json("schemas/examples/player-step-v3-event-next-decision.json")
        self.assertTrue(step["observed_events"])
        self.assertIsNotNone(step["next_decision"])
        rejected = read_json("schemas/examples/player-step-v3-rejected-no-events.json")
        self.assertEqual(rejected["submission"]["kind"], "rejected")
        self.assertEqual(rejected["observed_events"], [])

    def test_observation_contains_only_ordered_public_identity_forms(self) -> None:
        value = read_json("schemas/examples/magic-basic-land-observation-v1-ordered.json")
        self.assertEqual(
            [int(x["player"]) for x in value["mana_pools"]],
            sorted(int(x["player"]) for x in value["mana_pools"]),
        )
        self.assertEqual(
            [int(x["object"]) for x in value["counters"]],
            sorted(int(x["object"]) for x in value["counters"]),
        )
        self.assertEqual(
            [int(x["source"]) for x in value["attachments"]],
            sorted(int(x["source"]) for x in value["attachments"]),
        )
        self.assertEqual(
            [int(x["object"]) for x in value["faces"]],
            sorted(int(x["object"]) for x in value["faces"]),
        )
        forbidden = {
            "game_object_id",
            "physical_card_id",
            "ability_instance_id",
            "card_definition_id",
            "face_key",
            "ability_key",
            "content_contract_id",
        }

        def keys(item: Any) -> set[str]:
            if isinstance(item, dict):
                return set(item) | set().union(*(keys(v) for v in item.values()))
            if isinstance(item, list):
                return set().union(*(keys(v) for v in item)) if item else set()
            return set()

        self.assertFalse(keys(value) & forbidden)


if __name__ == "__main__":
    unittest.main()
