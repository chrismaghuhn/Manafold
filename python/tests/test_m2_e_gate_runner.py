from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from run_m2_e_gates import (
    check_no_runtime_lifecycle_channel,
    validate_m2_fixture_feature_scope,
)

RULES_MANIFEST = """
[features]
m2-conformance-fixtures = []
"""


def conformance_manifest(features: str) -> str:
    return f"""
[dependencies]
mtgml-rules = {{ path = \"../mtgml-rules\", features = {features} }}
"""


class ConformanceFeatureScopeTests(unittest.TestCase):
    def test_source_check_accepts_additional_conformance_only_feature(self) -> None:
        self.assertEqual(
            check_no_runtime_lifecycle_channel(),
            "test module gated, fixture feature scoped, no runtime caller",
        )

    def test_single_m2_fixture_feature_is_accepted(self) -> None:
        validate_m2_fixture_feature_scope(
            RULES_MANIFEST, conformance_manifest('["m2-conformance-fixtures"]')
        )

    def test_additional_conformance_feature_is_accepted(self) -> None:
        validate_m2_fixture_feature_scope(
            RULES_MANIFEST,
            conformance_manifest('["m2-conformance-fixtures", "m3-conformance-testkit"]'),
        )

    def test_missing_m2_fixture_feature_is_rejected(self) -> None:
        with self.assertRaisesRegex(AssertionError, "does not enable the fixture feature"):
            validate_m2_fixture_feature_scope(
                RULES_MANIFEST,
                conformance_manifest('["m3-conformance-testkit"]'),
            )

    def test_rules_manifest_must_declare_fixture_feature(self) -> None:
        with self.assertRaisesRegex(AssertionError, "lost the empty m2-conformance-fixtures"):
            validate_m2_fixture_feature_scope(
                "[features]\nother-feature = []\n",
                conformance_manifest('["m2-conformance-fixtures"]'),
            )

    def test_malformed_toml_is_rejected(self) -> None:
        with self.assertRaisesRegex(AssertionError, "invalid Cargo TOML"):
            validate_m2_fixture_feature_scope(RULES_MANIFEST, "[dependencies\n")

    def test_wrong_feature_structure_is_rejected(self) -> None:
        with self.assertRaisesRegex(AssertionError, "features must be a TOML string array"):
            validate_m2_fixture_feature_scope(
                RULES_MANIFEST,
                conformance_manifest('"m2-conformance-fixtures"'),
            )


if __name__ == "__main__":
    unittest.main()
