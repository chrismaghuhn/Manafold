#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
import jsonschema

ROOT = Path(__file__).resolve().parents[1]
WIRE_MAPPING = {
    "player-decision-request.v1": "player-decision-request.v1.schema.json",
    "decision-response.v1": "decision-response.v1.schema.json",
    "player-decision-request.v2": "player-decision-request.v2.schema.json",
    "decision-response.v2": "decision-response.v2.schema.json",
    "episode-status.v1": "episode-status.v1.schema.json",
    "observed-event-envelope.v1": "observed-event-envelope.v1.schema.json",
    "observation-envelope.v1": "observation-envelope.v1.schema.json",
    "information-state-envelope.v1": "information-state-envelope.v1.schema.json",
    "player-step.v1": "player-step.v1.schema.json",
    "replay-manifest.v1": "replay-manifest.v1.schema.json",
    "authoritative-replay.v1": "authoritative-replay.v1.schema.json",
    "replay-manifest.v2": "replay-manifest.v2.schema.json",
    "authoritative-replay.v2": "authoritative-replay.v2.schema.json",
    "information-state-envelope.v2": "information-state-envelope.v2.schema.json",
    "observed-event-envelope.v2": "observed-event-envelope.v2.schema.json",
    "player-step.v2": "player-step.v2.schema.json",
    "replay-manifest.v3": "replay-manifest.v3.schema.json",
    "authoritative-replay.v3": "authoritative-replay.v3.schema.json",
    "synthetic-m3-observation.v1": "synthetic-m3-observation.v1.schema.json",
    "replay-manifest.v4": "replay-manifest.v4.schema.json",
    "authoritative-replay.v4": "authoritative-replay.v4.schema.json",
    "replay-manifest.v5": "replay-manifest.v5.schema.json",
    "authoritative-replay.v5": "authoritative-replay.v5.schema.json",
}
ARTIFACT_CASES = [
    ("capability-registry.v1.schema.json", "cards/capabilities/registry.json"),
    ("capability-registry.v1.schema.json", "cards/capabilities/registry.example.json"),
    (
        "card-definition-manifest.v1.schema.json",
        "cards/definitions/example/card/example-card/manifest.example.json",
    ),
    (
        "bundle-manifest.v1.schema.json",
        "cards/bundles/example-v1/manifest.example.json",
    ),
    (
        "bundle-certification.v1.schema.json",
        "cards/bundles/example-v1/certification.example.json",
    ),
    (
        "scope-impact-report.v1.schema.json",
        "cards/bundles/example-v1/scope-impact.example.json",
    ),
    (
        "normative-document-register.v1.schema.json",
        "docs/normative-document-register.v1.json",
    ),
    (
        "contract-vocabulary-catalog.v1.schema.json",
        "contracts/catalog/contract-vocabulary.v1.json",
    ),
    ("golden-path-index.v1.schema.json", "examples/golden-path/index.json"),
]


def load(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_wire_schema_inventory(
    inventory: object,
    *,
    schema_root: Path = ROOT / "schemas",
) -> None:
    """Validate the declared wire-schema inventory against the validator map."""

    if not isinstance(inventory, dict):
        raise ValueError("schema inventory must be a JSON object")
    declared = inventory.get("wire_contracts")
    if not isinstance(declared, list) or not all(isinstance(name, str) for name in declared):
        raise ValueError("schema inventory wire_contracts must be a list of strings")

    duplicate_entries = sorted({name for name in declared if declared.count(name) > 1})
    if duplicate_entries:
        raise ValueError(f"duplicate schema inventory entry: {duplicate_entries}")

    mapped = list(WIRE_MAPPING.values())
    duplicate_mapping = sorted({name for name in mapped if mapped.count(name) > 1})
    if duplicate_mapping:
        raise ValueError(f"duplicate schema mapping value: {duplicate_mapping}")

    expected = sorted(mapped)
    declared_set = set(declared)
    expected_set = set(expected)
    missing = sorted(expected_set - declared_set)
    stale = sorted(declared_set - expected_set)
    if missing:
        raise ValueError(f"missing schema inventory entries: {missing}")
    if stale:
        raise ValueError(f"unexpected schema inventory entries: {stale}")
    if declared != expected:
        raise ValueError(f"schema inventory is not in canonical order: {declared} != {expected}")

    missing_files = sorted(name for name in expected if not (schema_root / name).is_file())
    if missing_files:
        raise ValueError(f"missing schema files: {missing_files}")


def main() -> None:
    validate_wire_schema_inventory(load(ROOT / "schemas" / "README.json"))
    manifest = load(ROOT / "wire/golden/manifest.json")
    assert isinstance(manifest, dict)
    fixtures = manifest["fixtures"]
    for case in fixtures:
        schema = load(ROOT / "schemas" / WIRE_MAPPING[case["contract"]])
        instance = load(ROOT / "wire/golden" / case["path"])
        jsonschema.Draft202012Validator(schema).validate(instance)
    for schema_rel, value_rel in ARTIFACT_CASES:
        jsonschema.Draft202012Validator(load(ROOT / "schemas" / schema_rel)).validate(
            load(ROOT / value_rel)
        )
    print(
        f"PASS: {len(fixtures)} wire fixtures and"
        f" {len(ARTIFACT_CASES)} maintainer artifacts validated against schemas"
    )


if __name__ == "__main__":
    main()
