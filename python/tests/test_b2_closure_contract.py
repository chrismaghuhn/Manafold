from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

try:
    import jsonschema
except ImportError:  # pragma: no cover - locked dev environments install it
    jsonschema = None

from mtgml.b2_closure_contract import (
    B2_CLOSURE_ARTIFACT_ROLES,
    B2_CLOSURE_SOURCE_PACKAGE_SHA256,
    B2_CLOSURE_V1_SCHEMA,
    B2ClosureArtifactBindingV1,
    B2ClosureContractError,
    B2ClosureContractErrorCode,
    B2ClosureCurrentnessState,
    B2ClosureCurrentRootV1,
    validate_artifact_bindings,
    validate_current_root_set,
)

FIXTURE = ROOT / "conformance" / "fixtures" / "authority" / "b2_closure_contract_slice1.v1.json"
NEGATIVE_MATRIX = (
    ROOT
    / "conformance"
    / "fixtures"
    / "authority"
    / "b2_closure_contract_slice1_negative_matrix.v1.json"
)


class B2ClosureContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        cls.negative_matrix = json.loads(NEGATIVE_MATRIX.read_text(encoding="utf-8"))

    def test_golden_v1_legacy_root_is_valid(self) -> None:
        root = B2ClosureCurrentRootV1.from_wire(self.fixture["goldens"]["legacy_v1"])
        self.assertEqual(root.artifact_role, "b2_closure")
        self.assertEqual(root.closure_version, "v1")
        self.assertEqual(root.validate(), root)

    def test_golden_v2_shape_is_valid_but_not_materialized(self) -> None:
        root = B2ClosureCurrentRootV1.from_wire(self.fixture["goldens"]["future_v2"])
        self.assertEqual(root.artifact_role, "b2_closure_v2")
        self.assertEqual(root.closure_version, "v2")
        self.assertEqual(root.validate(), root)
        self.assertFalse((ROOT / root.repository_relative_path).exists())

    @unittest.skipIf(jsonschema is None, "jsonschema is not installed")
    def test_golden_fixtures_match_closed_schemas(self) -> None:
        root_schema = json.loads(
            (ROOT / "schemas" / "b2-closure-current-root.v1.schema.json").read_text(
                encoding="utf-8"
            )
        )
        binding_schema = json.loads(
            (ROOT / "schemas" / "b2-closure-artifact-binding.v1.schema.json").read_text(
                encoding="utf-8"
            )
        )
        for value in self.fixture["goldens"].values():
            jsonschema.Draft202012Validator(root_schema).validate(value)
        for value in self.fixture["artifact_bindings"]:
            jsonschema.Draft202012Validator(binding_schema).validate(value)

    def test_currentness_states_are_closed(self) -> None:
        self.assertEqual(
            {state.value for state in B2ClosureCurrentnessState},
            {"v1_current", "v2_ready_not_adopted", "v2_adopted"},
        )

    def test_snapshot_constants_are_frozen(self) -> None:
        self.assertEqual(self.fixture["snapshot_constants"]["classification_count"], 402)
        self.assertEqual(self.fixture["snapshot_constants"]["terminal_assignment_edge_count"], 1883)
        self.assertEqual(self.fixture["snapshot_constants"]["deck_row_count"], 441)
        self.assertEqual(self.fixture["snapshot_constants"]["projection_row_count"], 441)
        self.assertEqual(self.fixture["snapshot_constants"]["historical_family_count"], 216)
        self.assertEqual(self.fixture["snapshot_constants"]["catalog_family_count"], 216)
        self.assertEqual(B2_CLOSURE_SOURCE_PACKAGE_SHA256, self.fixture["source_package_sha256"])

    def test_role_version_matrix_is_closed(self) -> None:
        valid = self.fixture["role_version_matrix"]
        self.assertEqual(valid["v1"], ["b2_closure"])
        self.assertEqual(valid["v2"], ["b2_closure_v2"])
        self.assertNotIn("b2_closure_v1", B2_CLOSURE_ARTIFACT_ROLES)

    def test_current_root_cardinality_fails_closed(self) -> None:
        with self.assertRaises(B2ClosureContractError) as missing:
            validate_current_root_set([])
        self.assertEqual(missing.exception.code, B2ClosureContractErrorCode.CURRENT_ROOT_MISSING)

        root = B2ClosureCurrentRootV1.from_wire(self.fixture["goldens"]["legacy_v1"])
        with self.assertRaises(B2ClosureContractError) as multiple:
            validate_current_root_set([root, root])
        self.assertEqual(multiple.exception.code, B2ClosureContractErrorCode.MULTIPLE_CURRENT_ROOTS)

    def test_negative_matrix_uses_distinct_fail_closed_codes(self) -> None:
        for case in self.negative_matrix["cases"]:
            with self.subTest(case=case["case_id"]):
                with self.assertRaises(B2ClosureContractError) as failure:
                    if case["kind"] == "root":
                        B2ClosureCurrentRootV1.from_wire(case["value"]).validate()
                    elif case["kind"] == "root_set":
                        roots = [B2ClosureCurrentRootV1.from_wire(item) for item in case["value"]]
                        validate_current_root_set(roots)
                    else:
                        bindings = [
                            B2ClosureArtifactBindingV1.from_wire(item) for item in case["value"]
                        ]
                        validate_artifact_bindings(bindings)
                self.assertEqual(failure.exception.code.value, case["expected_error"])

    @unittest.skipIf(jsonschema is None, "jsonschema is not installed")
    def test_negative_extra_fields_are_rejected_by_json_schema(self) -> None:
        root_schema = json.loads(
            (ROOT / "schemas" / "b2-closure-current-root.v1.schema.json").read_text(
                encoding="utf-8"
            )
        )
        binding_schema = json.loads(
            (ROOT / "schemas" / "b2-closure-artifact-binding.v1.schema.json").read_text(
                encoding="utf-8"
            )
        )
        for case in self.negative_matrix["cases"]:
            if case["case_id"] == "root_with_extra_field":
                with self.assertRaises(jsonschema.ValidationError):
                    jsonschema.Draft202012Validator(root_schema).validate(case["value"])
            if case["case_id"] == "artifact_binding_with_extra_field":
                with self.assertRaises(jsonschema.ValidationError):
                    jsonschema.Draft202012Validator(binding_schema).validate(case["value"][0])

    def test_v2_failure_does_not_fallback_to_v1(self) -> None:
        invalid_v2 = dict(self.fixture["goldens"]["future_v2"])
        invalid_v2["closure_schema_id"] = B2_CLOSURE_V1_SCHEMA
        with self.assertRaises(B2ClosureContractError) as failure:
            validate_current_root_set([B2ClosureCurrentRootV1.from_wire(invalid_v2)])
        self.assertEqual(
            failure.exception.code,
            B2ClosureContractErrorCode.CURRENT_ROOT_SCHEMA_MISMATCH,
        )

    def test_artifact_bindings_are_canonical_and_exact(self) -> None:
        bindings = [
            B2ClosureArtifactBindingV1.from_wire(item) for item in self.fixture["artifact_bindings"]
        ]
        self.assertEqual(validate_artifact_bindings(bindings), tuple(bindings))

        reversed_bindings = list(reversed(bindings))
        with self.assertRaises(B2ClosureContractError) as failure:
            validate_artifact_bindings(reversed_bindings)
        self.assertEqual(
            failure.exception.code,
            B2ClosureContractErrorCode.NONCANONICAL_SOURCE_ROLE_ORDER,
        )


if __name__ == "__main__":
    unittest.main()
