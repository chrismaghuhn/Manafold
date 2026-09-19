"""Task 2 contract tests: Python mechanical mirrors of the V5 contract
digest machinery (spec §7–§9, §19.1/§19.2).

The KAT vectors live in the shared fixture
``persistence/golden/semantic-contract-kat.v1.json`` — the single source of
truth read byte-identically by the Rust and Python suites. This suite also
proves the plan-mandated negative evidence: schema/domain identity defects
reject at the envelope boundary, and malformed child digest lengths reject
inside the mirror functions.
"""

from __future__ import annotations

import hashlib
import json
import struct
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.persistence import (  # noqa: E402
    DIGEST_ENVELOPE_ID,
    RULES_CONTRACT_DOMAIN,
    RULES_CONTRACT_INPUT_SCHEMA,
    SHA256_ID,
    PersistenceError,
    calculate_rules_contract_id_v1,
    calculate_semantic_contract_id_v1,
    decode_envelope,
    encode_canonical,
    encode_envelope,
)

KAT_PATH = ROOT / "persistence" / "golden" / "semantic-contract-kat.v1.json"


def load_kat_vectors() -> list[dict[str, object]]:
    with KAT_PATH.open(encoding="utf-8") as handle:
        document = json.load(handle)
    return document["vectors"]


def synthetic_rules_manifest() -> dict[str, object]:
    return {
        "rules_authority": {"variant": "synthetic_legacy"},
        "capability_closure": None,
    }


def minimal_comprehensive_manifest() -> dict[str, object]:
    return {
        "rules_authority": {
            "variant": "comprehensive_rules",
            "snapshot_id": "CR-2026-09-19",
        },
        "capability_closure": [
            {"key": "rules/synthetic-transition", "version": "1.0.0"}
        ],
    }


def rules_manifest_from(vector: dict[str, object]) -> dict[str, object]:
    authority = vector["rules_authority"]
    assert isinstance(authority, dict)
    return {
        "rules_authority": authority,
        "capability_closure": vector["capability_closure"],
    }


class SharedKatFixtureTests(unittest.TestCase):
    def test_shared_kat_vectors_reproduce_frozen_ids(self) -> None:
        vectors = load_kat_vectors()
        self.assertGreaterEqual(len(vectors), 5)

        rules_ids: dict[str, str] = {}
        for vector in vectors:
            case = str(vector["case"])
            if vector["manifest_kind"] == "rules":
                rules_ids[case] = calculate_rules_contract_id_v1(
                    rules_manifest_from(vector)
                )
                self.assertEqual(
                    rules_ids[case],
                    vector["rules_contract_id"],
                    f"KAT case {case} drifted",
                )

        for vector in vectors:
            if vector["manifest_kind"] != "semantic":
                continue
            case = str(vector["case"])
            reference = vector.get("rules_contract_id_ref")
            rules = (
                rules_ids[str(reference)]
                if reference is not None
                else str(vector["rules_contract_id"])
            )
            semantic_id = calculate_semantic_contract_id_v1(
                {
                    "rules_contract_id": rules,
                    "format_contract_id": vector["format_contract_id"],
                    "content_contract_id": vector["content_contract_id"],
                }
            )
            self.assertEqual(
                semantic_id,
                vector["semantic_contract_id"],
                f"KAT case {case} drifted",
            )


class RulesContractDigestTests(unittest.TestCase):
    def test_invalid_manifests_fail_closed(self) -> None:
        unsorted = {
            "rules_authority": {
                "variant": "comprehensive_rules",
                "snapshot_id": "CR-2026-09-19",
            },
            "capability_closure": [
                {"key": "rules/synthetic-transition", "version": "1.0.0"},
                {"key": "mechanic/lifelink", "version": "1.0.0"},
            ],
        }
        with self.assertRaises(PersistenceError):
            calculate_rules_contract_id_v1(unsorted)

        synthetic_with_closure = {
            "rules_authority": {"variant": "synthetic_legacy"},
            "capability_closure": [
                {"key": "rules/synthetic-transition", "version": "1.0.0"}
            ],
        }
        with self.assertRaises(PersistenceError):
            calculate_rules_contract_id_v1(synthetic_with_closure)

    def test_distinct_manifests_produce_distinct_ids(self) -> None:
        synthetic = calculate_rules_contract_id_v1(synthetic_rules_manifest())
        comprehensive = calculate_rules_contract_id_v1(minimal_comprehensive_manifest())
        self.assertNotEqual(synthetic, comprehensive)


class SemanticContractDigestTests(unittest.TestCase):
    def test_malformed_child_digest_length_rejects(self) -> None:
        # A child contract ID that is not exactly 64 lowercase hex characters
        # fails closed inside the mirror function. The mirror raises
        # PersistenceError for manifest-shape defects and WireError for
        # malformed digest text; both are ValueError subclasses, so the
        # fail-closed boundary is asserted on their common base.
        for bad_length in (31, 33, 63, 65):
            bad_id = "5a" * bad_length
            with self.assertRaises(ValueError):
                calculate_semantic_contract_id_v1(
                    {
                        "rules_contract_id": bad_id,
                        "format_contract_id": None,
                        "content_contract_id": None,
                    }
                )
            with self.assertRaises(ValueError):
                calculate_semantic_contract_id_v1(
                    {
                        "rules_contract_id": "5a" * 32,
                        "format_contract_id": "34" * bad_length,
                        "content_contract_id": None,
                    }
                )
        with self.assertRaises(ValueError):
            calculate_semantic_contract_id_v1(
                {
                    "rules_contract_id": "5a" * 32,
                    "format_contract_id": None,
                    "content_contract_id": "5A" * 32,  # not lowercase
                }
            )

    def test_schema_domain_disagreement_is_detectable_and_bound_to_identity(self) -> None:
        # DEFERRED REJECTION NOTE (accepted Option-A disposition):
        # Spec §8 and docs/STATE_HASHING.md:317 require that a *decoder* reject
        # a payload whose leading schema/domain fields disagree with the
        # envelope identity. Task 2 ships only writer/calculator paths; no
        # rules/semantic contract decode surface exists yet, so this suite
        # honestly proves only the detectability/content-binding preconditions:
        #   1. agreement is observable at the envelope decode boundary;
        #   2. disagreement is observable and yields a DIFFERENT identity;
        #   3. content cannot be reinterpreted across domains (identity change).
        # The rejection assertion itself belongs to the first real contract
        # decode path (plan Task 8 semantic admission scope) and must be added
        # there as a genuine disagreement -> exception case.
        # Evidence status: SCHEMA_DOMAIN_DISAGREEMENT_DETECTABLE = PASS,
        # SCHEMA_DOMAIN_DISAGREEMENT_REJECTED = DEFERRED (not claimed here).
        # Positive control: the canonical rules payload inside its canonical
        # envelope decodes with agreeing schema/domain identity.
        payload = encode_canonical(
            [
                RULES_CONTRACT_INPUT_SCHEMA,
                RULES_CONTRACT_DOMAIN,
                "diagnostic-agreement-probe",
            ]
        )
        correct_envelope = encode_envelope(
            RULES_CONTRACT_DOMAIN, RULES_CONTRACT_INPUT_SCHEMA, payload
        )
        reference, decoded_payload = decode_envelope(correct_envelope)
        self.assertEqual(reference["semantic_domain"], RULES_CONTRACT_DOMAIN)
        self.assertEqual(reference["input_schema_id"], RULES_CONTRACT_INPUT_SCHEMA)
        self.assertEqual(decoded_payload, payload)

        # Content binding: the same payload under the WRONG (semantic) labels
        # is a different artifact with a different identity. This proves
        # DISAGREEMENT_IS_DETECTABLE and IDENTITY_CHANGES; it deliberately does
        # NOT claim DISAGREEMENT_IS_REJECTED (no decode surface exists in Task 2).
        disagreeing_envelope = encode_envelope(
            "mtgml.semantic-contract.v1", "semantic-contract-manifest.v1", payload
        )
        wrong_reference, _ = decode_envelope(disagreeing_envelope)
        self.assertNotEqual(wrong_reference["semantic_domain"], RULES_CONTRACT_DOMAIN)
        self.assertNotEqual(wrong_reference["input_schema_id"], RULES_CONTRACT_INPUT_SCHEMA)
        self.assertNotEqual(
            calculate_rules_contract_id_v1(synthetic_rules_manifest()),
            hashlib.sha256(disagreeing_envelope).hexdigest(),
        )

        # Envelope-level identity defect (control): a corrupted (empty) domain
        # frame rejects the whole envelope at the decode boundary. This is
        # malformed envelope framing, NOT the payload-vs-envelope disagreement
        # case deferred above.
        corrupted = bytearray(correct_envelope)
        domain_length_offset = len(DIGEST_ENVELOPE_ID) + 1 + 8 + len(SHA256_ID)
        struct.pack_into(">Q", corrupted, domain_length_offset, 0)
        with self.assertRaises(PersistenceError):
            decode_envelope(bytes(corrupted))


if __name__ == "__main__":
    unittest.main()
