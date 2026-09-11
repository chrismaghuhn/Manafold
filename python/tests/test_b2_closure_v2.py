from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path
from shutil import copytree

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "src"))

from b2_closure_v2_support import (
    B2_CLOSURE_V2_PATH,
    B2_CLOSURE_V2_SCHEMA,
    B2_V1_ARTIFACT_BINDINGS,
    FROZEN_SNAPSHOT_CONSTANTS,
    SOURCE_PACKAGE_SHA256,
)
from build_m2_5_b2_closure_v2 import build_closure_v2, render_closure_v2
from check_m2_5_b2_classifications import B2CheckError, actual_inventory
from check_m2_5_b2_closure_v2 import (
    B2ClosureV2VerificationError,
    verify_closure_v2,
)

SLICE2_NEGATIVE_MATRIX = (
    ROOT / "conformance" / "fixtures" / "authority" / "b2_closure_v2_negative_matrix.v1.json"
)


class B2ClosureV2Tests(unittest.TestCase):
    def test_builder_is_deterministic_and_matches_materialized_bytes(self) -> None:
        first = render_closure_v2(build_closure_v2(ROOT))
        second = render_closure_v2(build_closure_v2(ROOT))
        self.assertEqual(first, second)
        self.assertEqual(hashlib.sha256(first).hexdigest(), hashlib.sha256(second).hexdigest())
        self.assertEqual((ROOT / B2_CLOSURE_V2_PATH).read_bytes(), first)

    def test_contract_shape_and_frozen_values(self) -> None:
        closure = build_closure_v2(ROOT)
        self.assertEqual(closure["schema"], B2_CLOSURE_V2_SCHEMA)
        self.assertEqual(closure["source_package_sha256"], SOURCE_PACKAGE_SHA256)
        self.assertEqual(closure["snapshot_constants"], FROZEN_SNAPSHOT_CONSTANTS)
        self.assertEqual(
            [item["artifact_role"] for item in closure["artifact_bindings"]],
            list(B2_V1_ARTIFACT_BINDINGS),
        )

    def test_persisted_closure_verifies_against_actual_inputs(self) -> None:
        closure = build_closure_v2(ROOT)
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            copytree(ROOT / "sources", repo / "sources")
            copytree(ROOT / "schemas", repo / "schemas")
            output = repo / B2_CLOSURE_V2_PATH
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_bytes(render_closure_v2(closure))
            self.assertEqual(verify_closure_v2(repo, output), closure)

    def test_verifier_rejects_tampered_persisted_counts(self) -> None:
        closure = build_closure_v2(ROOT)
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            copytree(ROOT / "sources", repo / "sources")
            copytree(ROOT / "schemas", repo / "schemas")
            output = repo / B2_CLOSURE_V2_PATH
            output.parent.mkdir(parents=True, exist_ok=True)
            mutated = json.loads(render_closure_v2(closure))
            mutated["snapshot_constants"]["classification_count"] += 1
            output.write_text(
                json.dumps(mutated, indent=2, sort_keys=True) + "\n", encoding="utf-8"
            )
            with self.assertRaises(B2ClosureV2VerificationError):
                verify_closure_v2(repo, output)

    def test_negative_matrix_rejects_closure_mutations(self) -> None:
        closure = build_closure_v2(ROOT)
        declared = json.loads(SLICE2_NEGATIVE_MATRIX.read_text(encoding="utf-8"))
        self.assertEqual({case["expected"] for case in declared["cases"]}, {"reject"})
        for case in declared["cases"]:
            with self.subTest(case=case["case_id"]):
                if case["case_id"] == "unknown_extra_b2_file":
                    with self.assertRaises(B2CheckError):
                        actual_inventory({"unexpected-extra.json"})
                    continue
                with tempfile.TemporaryDirectory() as directory:
                    repo = Path(directory)
                    copytree(ROOT / "sources", repo / "sources")
                    copytree(ROOT / "schemas", repo / "schemas")
                    output = repo / B2_CLOSURE_V2_PATH
                    output.parent.mkdir(parents=True, exist_ok=True)
                    mutated = deepcopy(closure)
                    if case["case_id"] == "wrong_closure_schema":
                        mutated["schema"] = "wrong.schema"
                    elif case["case_id"] == "extra_top_level_field":
                        mutated["unexpected"] = True
                    elif case["case_id"] == "missing_top_level_field":
                        del mutated["snapshot_constants"]
                    elif case["case_id"] == "missing_artifact_binding":
                        mutated["artifact_bindings"] = mutated["artifact_bindings"][:-1]
                    elif case["case_id"] == "duplicate_artifact_role":
                        mutated["artifact_bindings"][1] = deepcopy(mutated["artifact_bindings"][0])
                    elif case["case_id"] == "unknown_artifact_role":
                        mutated["artifact_bindings"][0]["artifact_role"] = "b2_unknown_v1"
                    elif case["case_id"] == "noncanonical_artifact_order":
                        mutated["artifact_bindings"] = list(reversed(mutated["artifact_bindings"]))
                    elif case["case_id"] == "wrong_artifact_path":
                        mutated["artifact_bindings"][0]["repository_relative_path"] = "wrong/path"
                    elif case["case_id"] == "wrong_artifact_schema":
                        mutated["artifact_bindings"][0]["schema_identifier"] = "wrong.schema"
                    elif case["case_id"] == "wrong_artifact_sha":
                        mutated["artifact_bindings"][0]["raw_sha256"] = "a" * 64
                    elif case["case_id"] == "tampered_classification_bytes":
                        source = (
                            repo / "sources/m2_5/closures/B2/card_semantic_classifications.v1.json"
                        )
                        source.write_bytes(source.read_bytes() + b"\n")
                    elif case["case_id"] == "wrong_source_package":
                        mutated["source_package_sha256"] = "b" * 64
                    elif case["case_id"] == "wrong_snapshot_count":
                        mutated["snapshot_constants"]["classification_count"] += 1
                    elif case["case_id"] == "self_binding_attempt":
                        mutated["artifact_bindings"][0]["artifact_role"] = "b2_closure_v2"
                    elif case["case_id"] == "verification_summary_binding_attempt":
                        mutated["artifact_bindings"][0]["repository_relative_path"] = (
                            "sources/m2_5/closures/B2/verification/b2_verification_summary.v1.json"
                        )
                    elif case["case_id"] == "current_root_binding_attempt":
                        mutated["artifact_bindings"][0]["repository_relative_path"] = (
                            "sources/m2_5/closures/B2/current_root.json"
                        )
                    else:
                        raise AssertionError(case["case_id"])
                    output.write_text(
                        json.dumps(mutated, indent=2, sort_keys=True) + "\n", encoding="utf-8"
                    )
                    with self.assertRaises(B2ClosureV2VerificationError):
                        verify_closure_v2(repo, output)


if __name__ == "__main__":
    unittest.main()
