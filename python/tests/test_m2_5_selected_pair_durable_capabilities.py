from __future__ import annotations

import copy
import hashlib
import json
import subprocess
import sys
import unittest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BASE_SHA = "c578eb78cd5f0ba7ca6db7267f7a05211a92ec06"
MAPPING = ROOT / "sources/m2_5/scope/selected_pair_durable_capability_mapping.v1.json"
CENSUS = ROOT / "sources/m2_5/scope/selected_pair_capability_census.v1.json"
LOCK = ROOT / "sources/m2_5/scope/exact_two_deck_scope_lock.v1.json"
REGISTRY = ROOT / "cards/capabilities/registry.json"
sys.path.insert(0, str(ROOT / "scripts"))

from maintainer_common import find_dependency_cycles, validate_capability_registry


def _canonical_json(value: object) -> str:
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _load_required(path: Path) -> dict[str, Any]:
    if not path.is_file():
        raise AssertionError(f"required durable capability artifact is missing: {path}")
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise AssertionError(f"expected JSON object: {path}")
    return value


def _selected_roots(census: dict[str, Any]) -> set[str]:
    families = census.get("families")
    if not isinstance(families, list):
        raise AssertionError("selected census families must be a list")
    roots = {family.get("family_id") for family in families}
    if not all(isinstance(root, str) for root in roots):
        raise AssertionError("selected census contains a family without a string identity")
    return roots


def _mapping_by_family(mapping: dict[str, Any]) -> dict[str, dict[str, Any]]:
    rows = mapping.get("mappings")
    if not isinstance(rows, list):
        raise AssertionError("mapping.mappings must be a list")
    result: dict[str, dict[str, Any]] = {}
    for row in rows:
        if not isinstance(row, dict):
            raise AssertionError("mapping row must be an object")
        if set(row) != {"b2_family_id", "durable_keys"}:
            raise AssertionError("mapping rows may contain only B2 identity and durable targets")
        family_id = row.get("b2_family_id")
        durable_keys = row.get("durable_keys")
        if not isinstance(family_id, str) or not isinstance(durable_keys, list):
            raise AssertionError("mapping row has an invalid identity or target list")
        if not durable_keys or not all(isinstance(key, str) for key in durable_keys):
            raise AssertionError(f"mapping row has no valid durable target: {family_id}")
        if durable_keys != sorted(set(durable_keys)):
            raise AssertionError(f"mapping targets are not unique and sorted: {family_id}")
        if family_id in result:
            raise AssertionError(f"duplicate B2 mapping row: {family_id}")
        result[family_id] = row
    return result


def _validate_selected_pair_mapping(
    mapping: dict[str, Any],
    census: dict[str, Any],
    lock: dict[str, Any],
    registry: dict[str, Any],
) -> tuple[set[str], set[str]]:
    if set(mapping) != {"mappings", "schema", "source"}:
        raise AssertionError("migration map contains registry or dependency semantics")
    if mapping.get("schema") != ("manafold.m2.5.selected-pair-durable-capability-mapping.v1"):
        raise AssertionError("unexpected migration map schema")

    source = mapping.get("source")
    if not isinstance(source, dict) or set(source) != {
        "selected_pair_census_path",
        "selected_pair_census_sha256",
        "scope_lock_path",
        "scope_lock_sha256",
    }:
        raise AssertionError("migration map source binding is not minimal and exact")
    if source["selected_pair_census_path"] != CENSUS.relative_to(ROOT).as_posix():
        raise AssertionError("migration map is bound to the wrong selected-pair census")
    if source["scope_lock_path"] != LOCK.relative_to(ROOT).as_posix():
        raise AssertionError("migration map is bound to the wrong scope lock")
    if source["selected_pair_census_sha256"] != _sha256(CENSUS):
        raise AssertionError("selected-pair census hash does not match the migration map")
    if source["scope_lock_sha256"] != _sha256(LOCK):
        raise AssertionError("scope lock hash does not match the migration map")

    if lock.get("lock_status") != "LOCKED" or lock.get("deck_pair_locked") is not True:
        raise AssertionError("selected deck pair is not locked")
    deck_names = [deck.get("deck_name") for deck in lock.get("decks", [])]
    if deck_names != ["Token Triumph", "Grave Danger"]:
        raise AssertionError(
            "mapping validation requires exactly the locked Token Triumph/Grave Danger pair"
        )
    selected_pair = census.get("selected_pair")
    if not isinstance(selected_pair, dict) or selected_pair.get("deck_ids") != [
        "m2-5/token-triumph",
        "m2-5/grave-danger",
    ]:
        raise AssertionError("selected census is not bound to the locked pair")

    selected_roots = _selected_roots(census)
    if (
        len(selected_roots) != 125
        or census.get("record_counts", {}).get("capability_families") != 125
    ):
        raise AssertionError("selected B2 root count is not exactly 125")
    if census.get("record_counts", {}).get("selected_oracle_identities") != 140:
        raise AssertionError("selected Oracle identity count drifted from the locked census")

    by_family = _mapping_by_family(mapping)
    if list(by_family) != sorted(selected_roots):
        raise AssertionError("mapping rows are not a complete deterministic selected-root sequence")
    if set(by_family) != selected_roots:
        raise AssertionError("selected B2 roots are missing or extra in the migration map")
    if MAPPING.read_text(encoding="utf-8") != _canonical_json(mapping):
        raise AssertionError("migration map is not canonically serialized")

    registry_entries = registry.get("entries")
    if not isinstance(registry_entries, list):
        raise AssertionError("capability registry entries must be a list")
    registry_by_key = {entry.get("key"): entry for entry in registry_entries}
    if len(registry_by_key) != len(registry_entries):
        raise AssertionError("capability registry contains duplicate keys")
    mapped_keys = {durable_key for row in by_family.values() for durable_key in row["durable_keys"]}
    for durable_key in sorted(mapped_keys):
        entry = registry_by_key.get(durable_key)
        if entry is None:
            raise AssertionError(f"mapping target is not registered: {durable_key}")
        if entry.get("lifecycle") != "specified":
            raise AssertionError(f"mapped target is not specified: {durable_key}")
        if not entry.get("authority_refs"):
            raise AssertionError(f"mapped target has no authority references: {durable_key}")
        if entry.get("information_risk") == "unreviewed":
            raise AssertionError(f"mapped target has unreviewed information risk: {durable_key}")
        if not entry.get("owners") or any("TBD" in owner for owner in entry["owners"]):
            raise AssertionError(f"mapped target has an invalid owner: {durable_key}")
        spec_path = ROOT / entry.get("spec_path", "")
        if not spec_path.is_file():
            raise AssertionError(f"mapped target specification is missing: {durable_key}")
    return selected_roots, mapped_keys


class SelectedPairDurableCapabilityTests(unittest.TestCase):
    def _artifacts(self) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any]]:
        return (
            _load_required(MAPPING),
            _load_required(CENSUS),
            _load_required(LOCK),
            _load_required(REGISTRY),
        )

    def test_selected_pair_mapping_is_complete_and_deterministic(self) -> None:
        mapping, census, lock, registry = self._artifacts()
        selected_roots, mapped_keys = _validate_selected_pair_mapping(
            mapping, census, lock, registry
        )
        self.assertEqual(len(selected_roots), 125)
        self.assertGreaterEqual(len(mapped_keys), 1)

    def test_every_mapping_target_is_registered_and_specified(self) -> None:
        mapping, census, lock, registry = self._artifacts()
        _, mapped_keys = _validate_selected_pair_mapping(mapping, census, lock, registry)
        by_key, cycles = validate_capability_registry(registry, root=ROOT)
        self.assertEqual(cycles, [])
        self.assertEqual(
            sum(by_key[key]["lifecycle"] == "proposed" for key in mapped_keys),
            0,
        )
        self.assertTrue(
            all(
                by_key[key]["implementation_paths"] == []
                and by_key[key]["conformance_cases"] == []
                and by_key[key]["benchmark_scenarios"] == []
                for key in mapped_keys
            )
        )

    def test_unrelated_registry_proposal_is_outside_selected_pair_lifecycle_gate(self) -> None:
        mapping, census, lock, registry = self._artifacts()
        unrelated = copy.deepcopy(registry)
        unrelated["entries"].append(
            {
                "authority_refs": [],
                "benchmark_scenarios": [],
                "category": "mechanic",
                "conformance_cases": [],
                "dependencies": [],
                "implementation_paths": [],
                "information_risk": "unreviewed",
                "key": "mechanic/future-proposal",
                "lifecycle": "proposed",
                "owners": ["future-maintainer"],
                "spec_path": "docs/rules/capabilities/mechanic/example-draw.md",
                "summary": "Unrelated future proposal outside this selected-pair gate.",
                "version": "0.1.0",
            }
        )
        _, mapped_keys = _validate_selected_pair_mapping(mapping, census, lock, unrelated)
        self.assertEqual(
            sum(
                entry["lifecycle"] == "proposed"
                for entry in unrelated["entries"]
                if entry["key"] in mapped_keys
            ),
            0,
        )

    def test_selected_pair_mapping_rejects_third_deck_contamination(self) -> None:
        mapping, census, lock, registry = self._artifacts()
        contaminated_lock = copy.deepcopy(lock)
        contaminated_lock["decks"].append({"deck_name": "Third deck"})
        with self.assertRaisesRegex(AssertionError, "exactly the locked"):
            _validate_selected_pair_mapping(mapping, census, contaminated_lock, registry)

    def test_selected_pair_registry_has_no_unknown_duplicate_self_or_cyclic_dependencies(
        self,
    ) -> None:
        mapping, census, lock, registry = self._artifacts()
        _, mapped_keys = _validate_selected_pair_mapping(mapping, census, lock, registry)
        by_key, cycles = validate_capability_registry(registry, root=ROOT)
        self.assertEqual(cycles, [])
        self.assertEqual(find_dependency_cycles(by_key), [])
        for key in mapped_keys:
            dependencies = by_key[key]["dependencies"]
            self.assertEqual(dependencies, sorted(set(dependencies)))
            self.assertNotIn(key, dependencies)
            self.assertTrue(all(dependency in by_key for dependency in dependencies))

    def test_selected_pair_registry_is_sorted_and_specs_have_no_scaffold_placeholders(self) -> None:
        mapping, census, lock, registry = self._artifacts()
        _, mapped_keys = _validate_selected_pair_mapping(mapping, census, lock, registry)
        entries = registry["entries"]
        self.assertEqual(
            [entry["key"] for entry in entries],
            sorted(entry["key"] for entry in entries),
        )
        self.assertEqual(REGISTRY.read_text(encoding="utf-8"), _canonical_json(registry))
        for key in sorted(mapped_keys):
            entry = next(entry for entry in entries if entry["key"] == key)
            text = (ROOT / entry["spec_path"]).read_text(encoding="utf-8")
            self.assertIn(f"`{key}@{entry['version']}`", text)
            self.assertIn("## Supported scope", text)
            self.assertIn("## Explicit exclusions", text)
            self.assertIn("## Authority", text)
            self.assertIn("## Dependencies", text)
            for forbidden in ("TBD", "TODO", "Generated proposal", "TBD-owner-role"):
                self.assertNotIn(forbidden, text)

    def test_capability_spec_directory_has_no_orphaned_durable_specs(self) -> None:
        registry = _load_required(REGISTRY)
        registered = {entry["spec_path"] for entry in registry["entries"]}
        allowed_examples = {"docs/rules/capabilities/mechanic/example-draw.md"}
        actual = {
            path.relative_to(ROOT).as_posix()
            for path in (ROOT / "docs/rules/capabilities").rglob("*.md")
        }
        self.assertEqual(actual - registered - allowed_examples, set())

    def test_high_risk_selected_families_have_individual_review_sections(self) -> None:
        mapping, census, lock, registry = self._artifacts()
        _, mapped_keys = _validate_selected_pair_mapping(mapping, census, lock, registry)
        mapping_by_family = _mapping_by_family(mapping)
        outlier_osi = {item["oracle_semantic_identity"] for item in census["high_risk_outliers"]}
        high_risk_families: dict[str, set[str]] = {}
        for record in census["records"]:
            if record["oracle_semantic_identity"] in outlier_osi:
                outlier = next(
                    item
                    for item in census["high_risk_outliers"]
                    if item["oracle_semantic_identity"] == record["oracle_semantic_identity"]
                )
                details = {
                    f"{card_name} [{outlier['risk_tags']}]" for card_name in record["card_names"]
                }
                for assignment in record["capability_assignments"]:
                    high_risk_families.setdefault(assignment["family_id"], set()).update(details)
        registry_by_key = {entry["key"]: entry for entry in registry["entries"]}
        self.assertGreater(len(high_risk_families), 0)
        for family_id in sorted(high_risk_families):
            for durable_key in mapping_by_family[family_id]["durable_keys"]:
                self.assertIn(durable_key, mapped_keys)
                self.assertIn(
                    registry_by_key[durable_key]["information_risk"],
                    {"high", "critical"},
                )
                text = (ROOT / registry_by_key[durable_key]["spec_path"]).read_text(
                    encoding="utf-8"
                )
                self.assertIn("## High-risk review", text)
                self.assertIn(f"Selected B2 family: `{family_id}`", text)
                for detail in sorted(high_risk_families[family_id]):
                    self.assertIn(detail, text)

    def test_b1_b2_and_c_sources_are_unchanged_from_task_base(self) -> None:
        changed: set[str] = set()
        for command in (
            ["git", "diff", "--name-only", BASE_SHA],
            ["git", "diff", "--cached", "--name-only", BASE_SHA],
            ["git", "ls-files", "--others", "--exclude-standard"],
        ):
            completed = subprocess.run(
                command,
                cwd=ROOT,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
            )
            changed.update(line for line in completed.stdout.splitlines() if line)
        forbidden_prefixes = (
            "sources/m2_5/closures/B1/",
            "sources/m2_5/closures/B2/",
            "sources/m2_5/closures/C/",
        )
        self.assertFalse(
            any(path.startswith(forbidden) for path in changed for forbidden in forbidden_prefixes),
            sorted(changed),
        )


if __name__ == "__main__":
    unittest.main()
