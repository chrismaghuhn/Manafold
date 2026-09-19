"""Task 3 RED tests: semantic-contract catalog generator (spec §10 chain).

The RED phase expects failure caused ONLY by the absent generator behavior
(missing script entry point / missing generated output). The suite imports
the generator as a module (mirroring how the repository's own tests invoke
checked-in scripts) and exercises: deterministic emit, ``--check`` drift
semantics, zero-diff rerun, mutation sensitivity, and the single-entry
production rule.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE_PATH = ROOT / "contracts" / "catalog" / "semantic-contracts.v1.json"
GENERATED_PATH = ROOT / "crates" / "mtgml-environment" / "src" / "semantic_catalog_generated.rs"
GENERATOR_PATH = ROOT / "scripts" / "generate_semantic_contract_catalog.py"

sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.persistence import (  # noqa: E402
    PersistenceError,
    calculate_rules_contract_id_v1,
    calculate_semantic_contract_id_v1,
)


def load_generator_module():
    if not GENERATOR_PATH.is_file():
        raise FileNotFoundError(f"generator script is absent: {GENERATOR_PATH}")
    spec = importlib.util.spec_from_file_location("generate_semantic_contract_catalog", GENERATOR_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class SourceOfTruthTests(unittest.TestCase):
    def test_source_json_exists_with_expected_shape(self) -> None:
        self.assertTrue(SOURCE_PATH.is_file(), f"missing source of truth: {SOURCE_PATH}")
        document = json.loads(SOURCE_PATH.read_text(encoding="utf-8"))
        self.assertEqual(document["schema_version"], "semantic-contracts-catalog.v1")
        entries = document["entries"]
        self.assertIsInstance(entries, list)
        self.assertEqual(len(entries), 1, "production catalog must contain exactly one entry")
        entry = entries[0]
        self.assertEqual(entry["rules_authority"], {"variant": "synthetic_legacy"})
        self.assertIsNone(entry["capability_closure"])
        self.assertIsNone(entry["format_contract_id"])
        self.assertIsNone(entry["content_contract_id"])

    def test_source_ids_recompute_via_task2_mirrors(self) -> None:
        document = json.loads(SOURCE_PATH.read_text(encoding="utf-8"))
        entry = document["entries"][0]
        rules_id = calculate_rules_contract_id_v1(
            {
                "rules_authority": entry["rules_authority"],
                "capability_closure": entry["capability_closure"],
            }
        )
        semantic_id = calculate_semantic_contract_id_v1(
            {
                "rules_contract_id": rules_id,
                "format_contract_id": None,
                "content_contract_id": None,
            }
        )
        self.assertEqual(entry["rules_contract_id"], rules_id)
        self.assertEqual(entry["semantic_contract_id"], semantic_id)


class GeneratorEmitTests(unittest.TestCase):
    def test_generate_is_deterministic_and_stable(self) -> None:
        module = load_generator_module()
        with tempfile.TemporaryDirectory() as scratch:
            out_a = Path(scratch) / "a.rs"
            out_b = Path(scratch) / "b.rs"
            module.write_generated(out_a, module.render_generated())
            module.write_generated(out_b, module.render_generated())
            self.assertEqual(out_a.read_bytes(), out_b.read_bytes())
            self.assertIn("@generated", out_a.read_text(encoding="utf-8"))
            self.assertIn("DO NOT EDIT", out_a.read_text(encoding="utf-8"))

    def test_check_mode_passes_on_fresh_output_and_fails_on_stale(self) -> None:
        module = load_generator_module()
        with tempfile.TemporaryDirectory() as scratch:
            target = Path(scratch) / "generated.rs"
            module.write_generated(target, module.render_generated())
            self.assertEqual(module.check_paths([target]), 0)
            target.write_text(
                target.read_text(encoding="utf-8").replace("synthetic_legacy", "magic_legacy"),
                encoding="utf-8",
            )
            self.assertEqual(module.check_paths([target]), 1)

    def test_check_mode_reports_stale_generated_output(self) -> None:
        module = load_generator_module()
        with tempfile.TemporaryDirectory() as scratch:
            target = Path(scratch) / "generated.rs"
            module.write_generated(target, module.render_generated())
            stale = target.read_text(encoding="utf-8").replace("semantic_contract", "semantic_contract_stale", 1)
            target.write_text(stale, encoding="utf-8")
            self.assertNotEqual(module.check_paths([target]), 0)

    def test_cli_check_exit_codes(self) -> None:
        if not GENERATOR_PATH.is_file():
            self.fail(f"generator script is absent: {GENERATOR_PATH}")
        result = subprocess.run(
            [sys.executable, str(GENERATOR_PATH), "--check"],
            capture_output=True,
            text=True,
        )
        self.assertIn(result.returncode, (0, 1), "CLI must fail closed with 0/1 only")

    def test_generator_does_not_duplicate_digest_logic(self) -> None:
        if not GENERATOR_PATH.is_file():
            self.fail(f"generator script is absent: {GENERATOR_PATH}")
        text = GENERATOR_PATH.read_text(encoding="utf-8")
        self.assertNotIn("sha256", text.lower())
        self.assertNotIn("hashlib", text.lower())
        self.assertNotIn("encode_envelope", text.lower())
        self.assertNotIn("encode_canonical", text.lower())

    def test_generated_output_uses_task2_mirrors_for_ids(self) -> None:
        module = load_generator_module()
        with tempfile.TemporaryDirectory() as scratch:
            target = Path(scratch) / "generated.rs"
            module.write_generated(target, module.render_generated())
            text = target.read_text(encoding="utf-8")
        document = json.loads(SOURCE_PATH.read_text(encoding="utf-8"))
        entry = document["entries"][0]
        self.assertIn(entry["rules_contract_id"], text)
        self.assertIn(entry["semantic_contract_id"], text)


class NegativeEvidenceTests(unittest.TestCase):
    """Plan-mandated adversarial evidence, realized honestly for the single
    synthetic entry surface: the only ID-relevant manifest facts of a valid
    synthetic entry (authority, closure) are pinned by validity, so any
    mutation either (a) changes derived IDs through the generator's own
    derivation path, or (b) fails closed at the emission boundary. Tampered
    recorded IDs always fail closed."""

    def test_mutated_manifest_fact_changes_derived_ids(self) -> None:
        import unittest.mock as mock

        module = load_generator_module()
        mutated = {
            "schema_version": "semantic-contracts-catalog.v1",
            "entries": [
                {
                    "entry_id": "hypothetical_comprehensive",
                    "rules_authority": {
                        "variant": "comprehensive_rules",
                        "snapshot_id": "CR-HYPOTHETICAL",
                    },
                    "capability_closure": [
                        {"key": "rules/synthetic-transition", "version": "1.0.0"}
                    ],
                    "format_contract_id": None,
                    "content_contract_id": None,
                    "rules_contract_id": "0" * 64,
                    "semantic_contract_id": "0" * 64,
                }
            ],
        }
        with tempfile.TemporaryDirectory() as scratch:
            scratch_path = Path(scratch) / "mutated.json"
            scratch_path.write_text(json.dumps(mutated), encoding="utf-8")
            with mock.patch.object(module, "SOURCE_PATH", scratch_path):
                # The generator's derivation path reflects manifest facts:
                entry = mutated["entries"][0]
                rules_id, semantic_id = module.derive_ids(entry)
            self.assertNotEqual(
                rules_id,
                "19bac684b74115b4fab823dfae2d3e75c23ee24f78effe51e9f72a33d4a58521",
                "a mutated manifest fact must change the derived rules ID",
            )
            self.assertNotEqual(
                semantic_id,
                "66ccac959475370e641e853473cbdd7f88489399587794b43f66cfa0342b1be4",
                "a mutated manifest fact must change the derived semantic ID",
            )
            # ...while the emission boundary stays fail-closed for this
            # generator version (single synthetic production entry only):
            with mock.patch.object(module, "SOURCE_PATH", scratch_path):
                with self.assertRaises(SystemExit):
                    module.render_generated()

    def test_recorded_id_tampering_fails_closed(self) -> None:
        import unittest.mock as mock

        module = load_generator_module()
        document = json.loads(module.SOURCE_PATH.read_text(encoding="utf-8"))
        document["entries"][0]["rules_contract_id"] = (
            "0" + document["entries"][0]["rules_contract_id"][1:]
        )
        with tempfile.TemporaryDirectory() as scratch:
            scratch_path = Path(scratch) / "tampered.json"
            scratch_path.write_text(json.dumps(document), encoding="utf-8")
            with mock.patch.object(module, "SOURCE_PATH", scratch_path):
                with self.assertRaises(SystemExit):
                    module.render_generated()

    def test_unsupported_variant_fails_closed(self) -> None:
        import unittest.mock as mock

        module = load_generator_module()
        document = json.loads(module.SOURCE_PATH.read_text(encoding="utf-8"))
        document["entries"][0]["rules_authority"] = {"variant": "magic_rules"}
        with tempfile.TemporaryDirectory() as scratch:
            scratch_path = Path(scratch) / "unsupported.json"
            scratch_path.write_text(json.dumps(document), encoding="utf-8")
            with mock.patch.object(module, "SOURCE_PATH", scratch_path):
                # The unsupported variant is rejected fail-closed at the
                # derivation boundary (Task-2 mirror raises PersistenceError)
                # before the generator's own emission boundary could raise
                # SystemExit; both boundaries reject the same defect.
                with self.assertRaises((SystemExit, PersistenceError)):
                    module.render_generated()


class GeneratedModuleTests(unittest.TestCase):
    def test_generated_file_exists_at_expected_path(self) -> None:
        self.assertTrue(
            GENERATED_PATH.is_file(),
            f"generated catalog module is absent: {GENERATED_PATH}",
        )


if __name__ == "__main__":
    unittest.main()
