"""Task 3 RED tests: semantic-contract catalog generator (spec §10 chain).

Covers: source-of-truth shape (facts only, no hand-authored identity),
deterministic emit, ``--check`` drift semantics, zero-diff rerun, the
plan-mandated end-to-end negative evidence, production catalog policy, and
the independent Python §19.4 recompute KAT.
"""

from __future__ import annotations

import importlib.util
import json
import re
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


def scratch_module_copy(module, catalog: dict[str, object]):
    """Return a source-shaped catalog document for renderer-level tests."""
    return catalog


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

    def test_source_contains_no_hand_authored_identity(self) -> None:
        # BLOCKER regression: derived IDs are GENERATED, never hand-authored.
        # The machine-readable source carries manifest FACTS only; any digest
        # literal there would be a second authority for derived identity.
        text = SOURCE_PATH.read_text(encoding="utf-8")
        document = json.loads(text)
        entry = document["entries"][0]
        self.assertNotIn("rules_contract_id", entry)
        self.assertNotIn("semantic_contract_id", entry)
        self.assertNotIn("rules_contract_id", document)
        self.assertNotIn("semantic_contract_id", document)
        self.assertNotIn("19bac684", text, "no hand-maintained digest literal in the source")
        self.assertNotIn("66ccac95", text, "no hand-maintained digest literal in the source")


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
        rules_id, semantic_id = module.derive_ids(document["entries"][0])
        self.assertIn(rules_id, text)
        self.assertIn(semantic_id, text)

    def test_generated_symbols_are_source_driven(self) -> None:
        # MAJOR-1 regression: every emitted symbol derives from entry_id.
        # A valid source with a DIFFERENT entry_id must produce functions and
        # call sites under the new name — never a reference to a hardcoded
        # other entry's symbol (which would emit invalid Rust).
        module = load_generator_module()
        alternate = {
            "schema_version": "semantic-contracts-catalog.v1",
            "entries": [
                {
                    "entry_id": "scratch_other_name",
                    "rules_authority": {"variant": "synthetic_legacy"},
                    "capability_closure": None,
                    "format_contract_id": None,
                    "content_contract_id": None,
                }
            ],
        }
        text = module.render_generated(alternate)
        self.assertIn("pub fn scratch_other_name_rules_contract_id()", text)
        self.assertIn("pub fn scratch_other_name_semantic_contract_id()", text)
        self.assertIn("pub fn scratch_other_name_rules_manifest()", text)
        self.assertIn("pub fn scratch_other_name_semantic_manifest()", text)
        self.assertIn("rules_contract_id: scratch_other_name_rules_contract_id(),", text)
        self.assertNotIn(
            "synthetic_legacy_default_rules_contract_id()",
            text,
            "an alternate entry must not reference another entry's symbol",
        )


class IndependentPythonKatTests(unittest.TestCase):
    """Spec §10/§19.4: the Python KAT recomputes IDs INDEPENDENTLY — it reads
    the source facts, calls the Task-2 mirror functions DIRECTLY, and compares
    against the checked-in generated output. It must not route through the
    generator's own derivation helper, which would make the check
    self-referential."""

    def test_python_kat_recomputes_independently_from_generated_output(self) -> None:
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
                "format_contract_id": entry["format_contract_id"],
                "content_contract_id": entry["content_contract_id"],
            }
        )
        generated = GENERATED_PATH.read_text(encoding="utf-8")
        hex_literals = set(re.findall(r'"([0-9a-f]{64})"', generated))
        self.assertIn(rules_id, hex_literals, "rules ID missing from generated output")
        self.assertIn(semantic_id, hex_literals, "semantic ID missing from generated output")

    def test_derived_ids_match_checked_in_generated_values(self) -> None:
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
                "format_contract_id": entry["format_contract_id"],
                "content_contract_id": entry["content_contract_id"],
            }
        )
        self.assertIn(rules_id, GENERATED_PATH.read_text(encoding="utf-8"))
        self.assertIn(semantic_id, GENERATED_PATH.read_text(encoding="utf-8"))


class NegativeEvidenceTests(unittest.TestCase):
    """Plan-mandated adversarial evidence: mutate one manifest fact →
    regenerated ID constants change AND --check fails against stale generated
    output; no Magic production contract may be generated."""

    def test_single_fact_mutation_changes_ids_and_fails_check(self) -> None:
        # Plan-exact evidence: mutate EXACTLY ONE manifest fact (the
        # comprehensive snapshot_id) between two otherwise-identical VALID
        # comprehensive entries. The single change must alter both derived ID
        # constants and make --check fail against the baseline (stale)
        # generated output.
        module = load_generator_module()
        baseline_entry = {
            "entry_id": "scratch_mutation_probe",
            "rules_authority": {
                "variant": "comprehensive_rules",
                "snapshot_id": "CR-BASELINE",
            },
            "capability_closure": [
                {"key": "rules/synthetic-transition", "version": "1.0.0"}
            ],
            "format_contract_id": None,
            "content_contract_id": None,
        }
        mutated_entry = dict(baseline_entry)
        mutated_entry["rules_authority"] = {
            "variant": "comprehensive_rules",
            "snapshot_id": "CR-MUTATED",
        }
        baseline_catalog = {
            "schema_version": "semantic-contracts-catalog.v1",
            "entries": [baseline_entry],
        }
        mutated_catalog = {
            "schema_version": "semantic-contracts-catalog.v1",
            "entries": [mutated_entry],
        }
        baseline_rules, baseline_semantic = module.derive_ids(baseline_entry)
        mutated_rules, mutated_semantic = module.derive_ids(mutated_entry)
        self.assertNotEqual(
            baseline_rules,
            mutated_rules,
            "a single fact change must alter the rules ID",
        )
        self.assertNotEqual(
            baseline_semantic,
            mutated_semantic,
            "a single fact change must alter the semantic ID",
        )
        baseline_render = module.render_generated(baseline_catalog)
        mutated_render = module.render_generated(mutated_catalog)
        self.assertIn(baseline_rules, baseline_render)
        self.assertIn(mutated_rules, mutated_render)
        self.assertNotIn(mutated_rules, baseline_render)
        self.assertNotIn(mutated_semantic, baseline_render)
        with tempfile.TemporaryDirectory() as scratch:
            target = Path(scratch) / "generated.rs"
            module.write_generated(target, baseline_render)  # stale after the mutation
            self.assertEqual(module.check_paths([target], catalog=baseline_catalog), 0)
            self.assertEqual(
                module.check_paths([target], catalog=mutated_catalog),
                1,
                "--check must fail against stale generated output after the mutation",
            )

    def test_valid_comprehensive_entry_renders_but_is_refused_as_production(self) -> None:
        # The renderer is policy-free and accepts any VALID manifest (the S1
        # slice will use this); the production policy validator refuses a
        # valid ComprehensiveRules production entry BY POLICY, not by
        # invalidity.
        module = load_generator_module()
        comprehensive = {
            "schema_version": "semantic-contracts-catalog.v1",
            "entries": [
                {
                    "entry_id": "hypothetical_magic",
                    "rules_authority": {
                        "variant": "comprehensive_rules",
                        "snapshot_id": "CR-HYPOTHETICAL",
                    },
                    "capability_closure": [
                        {"key": "rules/synthetic-transition", "version": "1.0.0"}
                    ],
                    "format_contract_id": None,
                    "content_contract_id": None,
                }
            ],
        }
        text = module.render_generated(comprehensive)
        self.assertIn("RulesAuthorityV1::ComprehensiveRules", text)
        self.assertIn("capability_closure: Some(vec![", text)
        self.assertIn("hypothetical_magic_rules_manifest", text)
        with self.assertRaises(SystemExit, msg="production policy must refuse a valid ComprehensiveRules entry"):
            module.assert_production_policy(comprehensive)

    def test_production_policy_accepts_current_source(self) -> None:
        module = load_generator_module()
        document = json.loads(SOURCE_PATH.read_text(encoding="utf-8"))
        module.assert_production_policy(document)

    def test_unknown_authority_variant_fails_closed_at_rendering(self) -> None:
        # Not a valid RulesAuthorityV1 variant at all: rejected by the Task-2
        # digest boundary (and by the renderer's variant dispatch).
        module = load_generator_module()
        document = json.loads(SOURCE_PATH.read_text(encoding="utf-8"))
        document["entries"][0]["rules_authority"] = {"variant": "magic_rules"}
        with self.assertRaises(PersistenceError):
            module.derive_ids(document["entries"][0])
        # The renderer fails closed at the digest boundary before its own
        # variant dispatch can fire; both boundaries reject the same defect.
        with self.assertRaises((SystemExit, PersistenceError)):
            module.render_generated(document)

    def test_source_shape_tampering_fails_closed(self) -> None:
        module = load_generator_module()
        document = json.loads(SOURCE_PATH.read_text(encoding="utf-8"))
        document["entries"][0]["unexpected_key"] = "x"
        with self.assertRaises(SystemExit):
            module.render_generated(document)


class GeneratedModuleTests(unittest.TestCase):
    def test_generated_file_exists_at_expected_path(self) -> None:
        self.assertTrue(
            GENERATED_PATH.is_file(),
            f"generated catalog module is absent: {GENERATED_PATH}",
        )


if __name__ == "__main__":
    unittest.main()
