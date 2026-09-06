from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from context_application_v2_validator import (
    ContextApplicationV2SemanticInput,
    ContextApplicationV2SemanticValidationError,
    ContextPreconditionValueV1,
    validate_context_application_v2_semantics,
)
from mtgml.authority import ContextBridgeRelationV2
from mtgml.persistence import PersistenceValue


class ContextApplicationV2SemanticCoreTests(unittest.TestCase):
    def test_exact_match_control_is_accepted(self) -> None:
        result = validate_context_application_v2_semantics(self._case("exact_match"))
        self.assertTrue(result.valid)
        self.assertIsNone(result.error_code)

    def test_reviewed_divergence_control_is_accepted_without_v1_source_equality(self) -> None:
        result = validate_context_application_v2_semantics(
            self._case("reviewed_divergence")
        )
        self.assertTrue(result.valid)
        self.assertIsNone(result.error_code)

    def test_every_negative_matrix_case_returns_its_declared_error_code(self) -> None:
        for raw in cast(list[dict[str, object]], self._matrix()["cases"]):
            expected = cast(dict[str, object], raw["expected"])
            if expected["valid"]:
                continue
            with self.subTest(case_id=raw["case_id"]):
                with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
                    validate_context_application_v2_semantics(
                        self._case(cast(str, raw["case_id"]))
                    )
                self.assertEqual(error.exception.code, expected["error_code"])

    def _matrix(self) -> dict[str, object]:
        return cast(
            dict[str, object],
            json.loads(
                (
                    ROOT
                    / "conformance/fixtures/authority/"
                    / "context_application_v2_semantic_golden_matrix.v1.json"
                ).read_text(encoding="utf-8")
            ),
        )

    def _case(self, case_id: str) -> ContextApplicationV2SemanticInput:
        cases = cast(list[dict[str, object]], self._matrix()["cases"])
        raw = next(case for case in cases if case["case_id"] == case_id)
        return ContextApplicationV2SemanticInput(
            theorem_subject_shape=cast(PersistenceValue, raw["theorem_subject_shape"]),
            member_context_binding=cast(PersistenceValue, raw["member_context_binding"]),
            historical_source_values=tuple(cast(list[str], raw["historical_source_values"])),
            bridge_source_values=tuple(cast(list[str], raw["bridge_source_values"])),
            theorem_context_values=tuple(cast(list[str], raw["theorem_context_values"])),
            bridge_reviewed_values=tuple(cast(list[str], raw["bridge_reviewed_values"])),
            bridge_relations=tuple(
                ContextBridgeRelationV2(value)
                for value in cast(list[str], raw["bridge_relations"])
            ),
            theorem_temporal_values=tuple(cast(list[str], raw["theorem_temporal_values"])),
            bridge_temporal_values=tuple(cast(list[str], raw["bridge_temporal_values"])),
            theorem_preconditions=tuple(
                ContextPreconditionValueV1(
                    value["precondition_id"],
                    cast(PersistenceValue, value["payload"]),
                )
                for value in cast(list[dict[str, object]], raw["theorem_preconditions"])
            ),
            member_preconditions=tuple(
                ContextPreconditionValueV1(
                    value["precondition_id"],
                    cast(PersistenceValue, value["observed_value"]),
                )
                for value in cast(list[dict[str, object]], raw["member_preconditions"])
            ),
        )


if __name__ == "__main__":
    unittest.main()
