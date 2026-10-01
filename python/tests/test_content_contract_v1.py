from __future__ import annotations

import json
import unittest
from pathlib import Path

from mtgml.content_contract_v1 import decode_content_contract_manifest_v1
from mtgml.errors import WireError
from mtgml.persistence import (
    PersistenceValue,
    calculate_content_contract_id_v1,
    encode_canonical,
)

ROOT = Path(__file__).resolve().parents[2]
PARITY_FIXTURE = (
    ROOT / "crates/mtgml-card-ir/tests/fixtures/content_contract_manifest_parity.v1.json"
)
BASIC_LAND_CONTENT = ROOT / "cards/definitions/basic-land-v1/content-contract.v1.cbor"
COMBINED_CONTENT = (
    ROOT / "cards/definitions/basic-land-and-vanilla-creature-v1/content-contract.v1.cbor"
)
COMBINED_KAT = (
    ROOT / "persistence/golden/content-contract-basic-land-and-vanilla-creature-v1-kat.v1.json"
)


def savannah_lions(**overrides: PersistenceValue) -> bytes:
    """Canonical one-definition manifest for Savannah Lions, with fields overridden."""
    fields: dict[str, PersistenceValue] = {
        "cost": [["white", None]],
        "supertypes": [],
        "card_types": ["Creature"],
        "subtypes": ["Cat"],
        "power_toughness": [2, 1],
        "loyalty": None,
        "abilities": [],
        "body": ["vanilla-creature-profile.v1", None],
        "second_face": False,
    }
    fields.update(overrides)
    characteristics: PersistenceValue = [
        "Savannah Lions",
        fields["cost"],
        [],
        [fields["supertypes"], fields["card_types"], fields["subtypes"]],
        fields["power_toughness"],
        fields["loyalty"],
        None,
    ]
    faces: list[PersistenceValue] = [[0, characteristics]]
    if fields["second_face"]:
        faces.append([1, characteristics])
    definition: PersistenceValue = [
        "card-definition-envelope.v1",
        1,
        faces,
        fields["abilities"],
        ["profiled", ["vanilla-creature@1.0.0", fields["body"]]],
        [],
        [],
    ]
    return encode_canonical(
        ["content-contract-manifest.v1", "mtgml.content-contract.v1", [definition]]
    )


class VanillaCreatureProfileTests(unittest.TestCase):
    def test_a_vanilla_creature_manifest_is_accepted(self) -> None:
        payload = savannah_lions()
        self.assertEqual(decode_content_contract_manifest_v1(payload), payload)

    def test_the_vanilla_body_label_must_match_exactly(self) -> None:
        with self.assertRaises(WireError) as caught:
            decode_content_contract_manifest_v1(
                savannah_lions(body=["vanilla-creature-profile.v2", None])
            )
        self.assertEqual(caught.exception.code, "semantic.replay_manifest")

    def test_a_vanilla_creature_with_rules_or_bad_shape_is_rejected(self) -> None:
        cases: dict[str, dict[str, PersistenceValue]] = {
            "toughness 0": {"power_toughness": [2, 0]},
            "negative power": {"power_toughness": [-1, 1]},
            "no power and toughness": {"power_toughness": None},
            "an ability identity": {"abilities": [[0, 0]]},
            "creature and artifact": {"card_types": ["Creature", "Artifact"]},
            "a supertype": {"supertypes": ["Legendary"]},
            "no subtype": {"subtypes": []},
            "no mana cost": {"cost": None},
            "an empty mana cost": {"cost": []},
            "a hybrid mana symbol": {"cost": [["hybrid", ["white", "red"]]]},
            "two faces": {"second_face": True},
            "loyalty": {"loyalty": 3},
            "the basic-land body label": {"body": ["basic-land-profile.v1", None]},
            "a text body payload": {"body": ["vanilla-creature-profile.v1", "mountain"]},
        }
        for name, overrides in cases.items():
            with self.subTest(name):
                with self.assertRaises(WireError) as caught:
                    decode_content_contract_manifest_v1(savannah_lions(**overrides))
                self.assertEqual(caught.exception.code, "semantic.replay_manifest")

    def test_a_vanilla_creature_with_zero_power_is_accepted(self) -> None:
        payload = savannah_lions(power_toughness=[0, 1])
        self.assertEqual(decode_content_contract_manifest_v1(payload), payload)

    def test_the_basic_land_content_is_still_accepted_unchanged(self) -> None:
        payload = BASIC_LAND_CONTENT.read_bytes()
        self.assertEqual(decode_content_contract_manifest_v1(payload), payload)

    def test_the_combined_catalog_matches_its_known_answer(self) -> None:
        payload = COMBINED_CONTENT.read_bytes()
        self.assertEqual(decode_content_contract_manifest_v1(payload), payload)
        kat = json.loads(COMBINED_KAT.read_text(encoding="utf-8"))
        self.assertEqual(kat["definition_ids"], [1, 2, 3, 4, 5])
        self.assertEqual(bytes.fromhex(kat["canonical_payload_hex"]), payload)
        self.assertEqual(
            encode_canonical(kat["content_manifest_value"]),
            payload,
            "the typed value in the known answer encodes to the committed bytes",
        )
        self.assertEqual(calculate_content_contract_id_v1(payload), kat["content_contract_id"])

    def test_acceptance_equals_the_rust_parity_fixture(self) -> None:
        fixture = json.loads(PARITY_FIXTURE.read_text(encoding="utf-8"))
        names = [case["case"] for case in fixture["cases"]]
        self.assertIn("valid_vanilla_creature", names)
        for case in fixture["cases"]:
            with self.subTest(case["case"]):
                payload = bytes.fromhex(case["canonical_cbor_hex"])
                if case["expected_valid"]:
                    self.assertEqual(decode_content_contract_manifest_v1(payload), payload)
                else:
                    with self.assertRaises(WireError):
                        decode_content_contract_manifest_v1(payload)


if __name__ == "__main__":
    unittest.main()
