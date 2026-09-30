from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

import validate_schemas

try:
    import jsonschema
except ImportError:  # pragma: no cover - the locked dev environment installs it
    jsonschema = None


class SchemaParityTests(unittest.TestCase):
    def _schema_inventory(self) -> dict[str, object]:
        return json.loads((ROOT / "schemas" / "README.json").read_text(encoding="utf-8"))

    def test_schema_readme_matches_wire_mapping(self) -> None:
        inventory = self._schema_inventory()
        validate_schemas.validate_wire_schema_inventory(inventory)
        self.assertEqual(
            inventory["wire_contracts"],
            sorted(validate_schemas.WIRE_MAPPING.values()),
        )

    def test_wire_schema_ids_match_their_filenames(self) -> None:
        validate_schemas.validate_wire_schema_inventory(self._schema_inventory())

    def test_wire_schema_identity_collision_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = "first.v1.schema.json"
            second = "second.v1.schema.json"
            for name in (first, second):
                (root / name).write_text(
                    json.dumps({"$id": first}),
                    encoding="utf-8",
                )
            inventory = {"wire_contracts": [first, second]}
            with (
                patch.object(validate_schemas, "WIRE_MAPPING", {"first": first, "second": second}),
                self.assertRaisesRegex(ValueError, r"\$id must match filename"),
            ):
                validate_schemas.validate_wire_schema_inventory(inventory, schema_root=root)

    def test_schema_inventory_duplicate_rejected(self) -> None:
        inventory = self._schema_inventory()
        inventory["wire_contracts"] = [
            *inventory["wire_contracts"],
            "replay-manifest.v8.schema.json",
        ]
        with self.assertRaisesRegex(ValueError, "duplicate schema inventory entry"):
            validate_schemas.validate_wire_schema_inventory(inventory)

    def test_schema_inventory_missing_rejected(self) -> None:
        inventory = self._schema_inventory()
        inventory["wire_contracts"] = [
            name for name in inventory["wire_contracts"] if name != "replay-manifest.v8.schema.json"
        ]
        with self.assertRaisesRegex(ValueError, "missing schema inventory entries"):
            validate_schemas.validate_wire_schema_inventory(inventory)

    def test_schema_inventory_stale_rejected(self) -> None:
        inventory = self._schema_inventory()
        inventory["wire_contracts"] = [
            *inventory["wire_contracts"],
            "stale-wire-contract.v1.schema.json",
        ]
        with self.assertRaisesRegex(ValueError, "unexpected schema inventory entries"):
            validate_schemas.validate_wire_schema_inventory(inventory)

    def test_schema_inventory_missing_schema_file_rejected(self) -> None:
        inventory = {"wire_contracts": sorted(validate_schemas.WIRE_MAPPING.values())}
        with (
            tempfile.TemporaryDirectory() as directory,
            self.assertRaisesRegex(ValueError, "missing schema files"),
        ):
            validate_schemas.validate_wire_schema_inventory(
                inventory,
                schema_root=Path(directory),
            )

    @unittest.skipIf(jsonschema is None, "jsonschema is not installed")
    def test_all_golden_fixtures_match_their_normative_schema(self) -> None:
        mapping = validate_schemas.WIRE_MAPPING
        directory = ROOT / "wire" / "golden"
        manifest = json.loads((directory / "manifest.json").read_text(encoding="utf-8"))
        for case in manifest["fixtures"]:
            schema = json.loads(
                (ROOT / "schemas" / mapping[case["contract"]]).read_text(encoding="utf-8")
            )
            instance = json.loads((directory / case["path"]).read_text(encoding="utf-8"))
            with self.subTest(case=case["path"]):
                jsonschema.Draft202012Validator(schema).validate(instance)

    def test_episode_reasons_are_schema_enums_not_open_strings(self) -> None:
        schema = json.loads(
            (ROOT / "schemas" / "episode-status.v1.schema.json").read_text(encoding="utf-8")
        )
        terminal = next(
            item
            for item in schema["oneOf"]
            if item["properties"]["kind"].get("const") == "terminal"
        )
        truncated = next(
            item
            for item in schema["oneOf"]
            if item["properties"]["kind"].get("const") == "truncated"
        )
        self.assertEqual(len(terminal["properties"]["reason"]["enum"]), 5)
        self.assertEqual(len(truncated["properties"]["reason"]["enum"]), 5)


if __name__ == "__main__":
    unittest.main()
