from __future__ import annotations

import json
import sys
import unittest
from copy import deepcopy
from pathlib import Path
from typing import cast
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "src"))

from b2_closure_v2_adoption_readiness import (
    B2_ADOPTION_READINESS_PATH,
    B2_ADOPTION_READINESS_SCHEMA,
    B2AdoptionReadinessError,
    build_adoption_readiness,
    render_adoption_readiness,
    run_negative_evidence_matrix,
    verify_adoption_readiness_value,
)

NEGATIVE_MATRIX = (
    ROOT
    / "conformance"
    / "fixtures"
    / "authority"
    / "b2_closure_v2_adoption_readiness_negative_matrix.v1.json"
)


class B2ClosureAdoptionReadinessTests(unittest.TestCase):
    @patch("b2_closure_v2_adoption_readiness.run_historical_v1_verification")
    def test_builder_emits_verified_non_current_readiness(
        self, historical_verification: object
    ) -> None:
        historical_verification.return_value = "PASS"  # type: ignore[attr-defined]
        evidence = build_adoption_readiness(ROOT, allow_current_root_for_historical_test=True)

        self.assertEqual(evidence["schema"], B2_ADOPTION_READINESS_SCHEMA)
        self.assertEqual(evidence["currentness_state"], "v2_ready_not_adopted")
        current_root = cast(dict[str, object], evidence["current_root"])
        candidate = cast(dict[str, object], evidence["candidate_v2_binding"])
        downstream = cast(dict[str, object], evidence["downstream_readiness"])
        self.assertFalse(current_root["created"])
        self.assertFalse(current_root["adopted_v2"])
        self.assertEqual(candidate["artifact_role"], "b2_closure_v2")
        self.assertEqual(downstream["authority_validator"], "PASS")

    @patch("b2_closure_v2_adoption_readiness.run_historical_v1_verification")
    def test_builder_is_deterministic(self, historical_verification: object) -> None:
        historical_verification.return_value = "PASS"  # type: ignore[attr-defined]
        first = render_adoption_readiness(
            build_adoption_readiness(ROOT, allow_current_root_for_historical_test=True)
        )
        second = render_adoption_readiness(
            build_adoption_readiness(ROOT, allow_current_root_for_historical_test=True)
        )

        self.assertEqual(first, second)

    def test_materialized_path_is_the_known_verification_path(self) -> None:
        self.assertEqual(
            B2_ADOPTION_READINESS_PATH,
            "sources/m2_5/closures/B2/verification/b2_closure_v2_adoption_readiness.v1.json",
        )

    def test_rendered_evidence_is_closed_json(self) -> None:
        with patch(
            "b2_closure_v2_adoption_readiness.run_historical_v1_verification",
            return_value="PASS",
        ):
            rendered = render_adoption_readiness(
                build_adoption_readiness(ROOT, allow_current_root_for_historical_test=True)
            )

        self.assertIsInstance(json.loads(rendered), dict)

    def test_negative_matrix_is_closed_and_declares_rejection(self) -> None:
        matrix = json.loads(NEGATIVE_MATRIX.read_text(encoding="utf-8"))
        cases = matrix["cases"]
        self.assertEqual(
            matrix["schema"], "manafold.m2.5.b2.closure-v2-adoption-readiness-negative-matrix.v1"
        )
        self.assertEqual(len(cases), 19)
        self.assertEqual({case["expected"] for case in cases}, {"reject"})
        self.assertEqual(len({case["case_id"] for case in cases}), len(cases))

    @patch("b2_closure_v2_adoption_readiness.run_historical_v1_verification")
    def test_verifier_rejects_currentness_and_extra_field_mutations(
        self, historical_verification: object
    ) -> None:
        historical_verification.return_value = "PASS"  # type: ignore[attr-defined]
        evidence = build_adoption_readiness(ROOT, allow_current_root_for_historical_test=True)
        for field, value in (
            (("current_root", "created"), True),
            (("current_root", "adopted_v2"), True),
        ):
            mutated = deepcopy(evidence)
            cast(dict[str, object], mutated[field[0]])[field[1]] = value
            with self.subTest(field=field), self.assertRaises(B2AdoptionReadinessError):
                verify_adoption_readiness_value(ROOT, mutated, recomputed=evidence)

        extra = deepcopy(evidence)
        extra["unexpected_future_field"] = True
        with self.assertRaises(B2AdoptionReadinessError):
            verify_adoption_readiness_value(ROOT, extra, recomputed=evidence)

    @patch("b2_closure_v2_adoption_readiness.run_historical_v1_verification")
    def test_negative_matrix_executes_every_declared_case(
        self, historical_verification: object
    ) -> None:
        historical_verification.return_value = "PASS"  # type: ignore[attr-defined]
        evidence = build_adoption_readiness(ROOT, allow_current_root_for_historical_test=True)
        executed = run_negative_evidence_matrix(ROOT, evidence)

        self.assertEqual(len(executed), 19)
        self.assertEqual(len(set(executed)), 19)


if __name__ == "__main__":
    unittest.main()
