"""Task 2 contract tests: Python mechanical mirrors of the V5 contract
digest machinery (spec §7–§9, §19.1/§19.2).

The RED phase expects failure caused ONLY by the missing mirror functions.
KAT hex literals are frozen during GREEN and asserted byte-identically by
the Rust and Python suites (shared vectors, independent implementations).
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.persistence import (  # noqa: E402
    calculate_rules_contract_id_v1,
    calculate_semantic_contract_id_v1,
)


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


def semantic_manifest(
    rules_contract_id: str,
    format_contract_id: str | None,
    content_contract_id: str | None,
) -> dict[str, object]:
    return {
        "rules_contract_id": rules_contract_id,
        "format_contract_id": format_contract_id,
        "content_contract_id": content_contract_id,
    }


RULES_SYNTHETIC_LEGACY_KAT = "19bac684b74115b4fab823dfae2d3e75c23ee24f78effe51e9f72a33d4a58521"
RULES_COMPREHENSIVE_MINIMAL_KAT = "6759a1bfa9ab56da74bdddde7c2e56ade99fc3169bffe9e16ce27f3947574722"
SEMANTIC_SYNTHETIC_NULL_NULL_KAT = "66ccac959475370e641e853473cbdd7f88489399587794b43f66cfa0342b1be4"
SEMANTIC_MAGIC_HYPOTHETICAL_NULL_NULL_KAT = (
    "f82a44a672f1095d7db11a505e0d6004e1e0670604f71cea80b3787255f883e9"
)
SEMANTIC_RESERVED_DIMENSIONS_KAT = "5465b1d7799db3ed2175211cb49a342520258ee7e6782a7f8aa45b985ca150fb"


class RulesContractDigestTests(unittest.TestCase):
    def test_kat_synthetic_legacy_is_frozen(self) -> None:
        self.assertEqual(
            calculate_rules_contract_id_v1(synthetic_rules_manifest()),
            RULES_SYNTHETIC_LEGACY_KAT,
        )

    def test_kat_minimal_comprehensive_is_frozen(self) -> None:
        self.assertEqual(
            calculate_rules_contract_id_v1(minimal_comprehensive_manifest()),
            RULES_COMPREHENSIVE_MINIMAL_KAT,
        )

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
        with self.assertRaises(Exception):
            calculate_rules_contract_id_v1(unsorted)  # noqa: B017

        synthetic_with_closure = {
            "rules_authority": {"variant": "synthetic_legacy"},
            "capability_closure": [
                {"key": "rules/synthetic-transition", "version": "1.0.0"}
            ],
        }
        with self.assertRaises(Exception):
            calculate_rules_contract_id_v1(synthetic_with_closure)  # noqa: B017

    def test_distinct_manifests_produce_distinct_ids(self) -> None:
        synthetic = calculate_rules_contract_id_v1(synthetic_rules_manifest())
        comprehensive = calculate_rules_contract_id_v1(minimal_comprehensive_manifest())
        self.assertNotEqual(synthetic, comprehensive)


class SemanticContractDigestTests(unittest.TestCase):
    def test_kat_synthetic_null_null_is_frozen(self) -> None:
        rules = calculate_rules_contract_id_v1(synthetic_rules_manifest())
        self.assertEqual(
            calculate_semantic_contract_id_v1(semantic_manifest(rules, None, None)),
            SEMANTIC_SYNTHETIC_NULL_NULL_KAT,
        )

    def test_kat_magic_hypothetical_null_null_is_frozen(self) -> None:
        # KAT-only value (spec §19.2); not a catalog entry (spec §25).
        rules = "5a" * 32
        self.assertEqual(
            calculate_semantic_contract_id_v1(semantic_manifest(rules, None, None)),
            SEMANTIC_MAGIC_HYPOTHETICAL_NULL_NULL_KAT,
        )

    def test_kat_reserved_dimensions_use_raw_bytes(self) -> None:
        rules = calculate_rules_contract_id_v1(synthetic_rules_manifest())
        self.assertEqual(
            calculate_semantic_contract_id_v1(
                semantic_manifest(rules, "34" * 32, "56" * 32)
            ),
            SEMANTIC_RESERVED_DIMENSIONS_KAT,
        )


if __name__ == "__main__":
    unittest.main()
