from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from maintainer_common import (  # noqa: E402
    MaintainerArtifactError,
    capability_census,
    certification_status,
    load_json,
    validate_capability_registry,
)


class MaintainerArtifactTests(unittest.TestCase):
    def test_example_bundle_has_resolvable_structural_closure(self) -> None:
        bundle = load_json(ROOT / "cards/bundles/example-v1/manifest.example.json")
        registry = load_json(ROOT / "cards/capabilities/registry.example.json")
        census = capability_census(bundle, registry, root=ROOT, required_capability_lifecycle="proposed")
        self.assertEqual(census.missing, ())
        self.assertEqual(census.cycles, ())
        self.assertEqual(census.missing_definitions, ())
        self.assertIn("mechanic/example-draw", census.resolved)
        self.assertIn("rules/zone-change", census.resolved)

    def test_cycle_is_detected(self) -> None:
        registry = {
            "schema_version": "capability-registry.v1",
            "registry_id": "test/capabilities",
            "entries": [
                {
                    "key": "rules/a",
                    "version": "0.1.0",
                    "category": "core_rule",
                    "lifecycle": "proposed",
                    "summary": "a",
                    "dependencies": ["rules/b"],
                    "authority_refs": [],
                    "spec_path": "a.md",
                    "implementation_paths": [],
                    "conformance_cases": [],
                    "information_risk": "unreviewed",
                    "benchmark_scenarios": [],
                    "owners": ["test"],
                },
                {
                    "key": "rules/b",
                    "version": "0.1.0",
                    "category": "core_rule",
                    "lifecycle": "proposed",
                    "summary": "b",
                    "dependencies": ["rules/a"],
                    "authority_refs": [],
                    "spec_path": "b.md",
                    "implementation_paths": [],
                    "conformance_cases": [],
                    "information_risk": "unreviewed",
                    "benchmark_scenarios": [],
                    "owners": ["test"],
                },
            ],
        }
        _, cycles = validate_capability_registry(registry)
        self.assertTrue(cycles)

    def test_unregistered_dependency_is_rejected(self) -> None:
        registry = load_json(ROOT / "cards/capabilities/registry.example.json")
        registry["entries"][0]["dependencies"] = ["rules/missing"]
        with self.assertRaises(MaintainerArtifactError):
            validate_capability_registry(registry)

    def test_unknown_lifecycle_threshold_is_rejected_cleanly(self) -> None:
        bundle = load_json(ROOT / "cards/bundles/example-v1/manifest.example.json")
        registry = load_json(ROOT / "cards/capabilities/registry.example.json")
        with self.assertRaisesRegex(MaintainerArtifactError, "unknown required card lifecycle"):
            capability_census(
                bundle,
                registry,
                root=ROOT,
                required_card_lifecycle="proposed",
            )

    def test_deprecated_lifecycle_cannot_be_used_as_minimum_threshold(self) -> None:
        bundle = load_json(ROOT / "cards/bundles/example-v1/manifest.example.json")
        registry = load_json(ROOT / "cards/capabilities/registry.example.json")
        with self.assertRaisesRegex(MaintainerArtifactError, "unknown required capability lifecycle"):
            capability_census(
                bundle,
                registry,
                root=ROOT,
                required_capability_lifecycle="deprecated",
            )

    def test_missing_capability_spec_is_rejected_when_root_is_known(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            registry = {
                "schema_version": "capability-registry.v1",
                "registry_id": "test/capabilities",
                "entries": [
                    {
                        "key": "rules/example",
                        "version": "0.1.0",
                        "category": "core_rule",
                        "lifecycle": "proposed",
                        "summary": "example",
                        "dependencies": [],
                        "authority_refs": [],
                        "spec_path": "docs/rules/capabilities/rules/example.md",
                        "implementation_paths": [],
                        "conformance_cases": [],
                        "information_risk": "unreviewed",
                        "benchmark_scenarios": [],
                        "owners": ["test"],
                    }
                ],
            }
            with self.assertRaisesRegex(MaintainerArtifactError, "missing capability specification"):
                validate_capability_registry(registry, root=Path(directory))

    def test_implemented_capability_requires_existing_implementation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            spec = root / "docs/rules/capabilities/rules/example.md"
            spec.parent.mkdir(parents=True)
            spec.write_text("# Example\n", encoding="utf-8")
            registry = {
                "schema_version": "capability-registry.v1",
                "registry_id": "test/capabilities",
                "entries": [
                    {
                        "key": "rules/example",
                        "version": "0.1.0",
                        "category": "core_rule",
                        "lifecycle": "implemented",
                        "summary": "example",
                        "dependencies": [],
                        "authority_refs": ["rules@test"],
                        "spec_path": "docs/rules/capabilities/rules/example.md",
                        "implementation_paths": [],
                        "conformance_cases": [],
                        "information_risk": "low",
                        "benchmark_scenarios": [],
                        "owners": ["test"],
                    }
                ],
            }
            with self.assertRaisesRegex(MaintainerArtifactError, "requires at least one implementation path"):
                validate_capability_registry(registry, root=root)

    def test_native_executors_are_discovered_from_definition_closure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shutil.copytree(ROOT / "cards", root / "cards")
            shutil.copytree(
                ROOT / "docs" / "rules" / "capabilities",
                root / "docs" / "rules" / "capabilities",
            )
            manifest_path = (
                root
                / "cards"
                / "definitions"
                / "example"
                / "card"
                / "example-card"
                / "manifest.example.json"
            )
            manifest = load_json(manifest_path)
            manifest["native_executor"] = "native/example-card"
            manifest_path.write_text(
                json.dumps(manifest, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            bundle = load_json(
                root / "cards" / "bundles" / "example-v1" / "manifest.example.json"
            )
            registry = load_json(
                root / "cards" / "capabilities" / "registry.example.json"
            )

            census = capability_census(
                bundle,
                registry,
                root=root,
                required_capability_lifecycle="proposed",
            )
            self.assertEqual(census.native_executors, ("native/example-card",))
            self.assertEqual(
                census.undeclared_native_executors,
                ("native/example-card",),
            )
            self.assertEqual(census.stale_native_executor_declarations, ())
            self.assertEqual(
                certification_status(census, [{"gate": "synthetic", "status": "PASS"}]),
                "blocked",
            )

    def test_stale_bundle_native_executor_declaration_is_a_blocker(self) -> None:
        bundle = load_json(ROOT / "cards/bundles/example-v1/manifest.example.json")
        registry = load_json(ROOT / "cards/capabilities/registry.example.json")
        bundle["native_executors"] = ["native/stale"]
        census = capability_census(
            bundle,
            registry,
            root=ROOT,
            required_capability_lifecycle="proposed",
        )
        self.assertEqual(census.native_executors, ())
        self.assertEqual(
            census.stale_native_executor_declarations,
            ("native/stale",),
        )

    def test_static_preflight_never_certifies_not_run_gates(self) -> None:
        bundle = load_json(ROOT / "cards/bundles/example-v1/manifest.example.json")
        registry = load_json(ROOT / "cards/capabilities/registry.example.json")
        census = capability_census(bundle, registry, root=ROOT, required_capability_lifecycle="proposed")
        status = certification_status(census, [{"gate": "REFERENCE_IMPLEMENTATION", "status": "NOT_RUN"}])
        self.assertEqual(status, "blocked")


if __name__ == "__main__":
    unittest.main()
