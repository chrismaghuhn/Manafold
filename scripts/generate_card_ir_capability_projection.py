#!/usr/bin/env python3
"""Generate the validated Rust projection of the canonical capability registry."""

from __future__ import annotations

import argparse
import copy
import json
import sys
from typing import Any

sys.dont_write_bytecode = True

from maintainer_common import (
    LIFECYCLE_ORDER,
    ROOT,
    capability_census,
    load_json,
    validate_capability_registry,
)

PROJECTION = ROOT / "crates/mtgml-card-ir/src/generated_capability_registry.json"
PARITY = ROOT / "crates/mtgml-card-ir/tests/fixtures/capability_registry_parity.v1.json"


def render() -> tuple[bytes, bytes]:
    registry = load_json(ROOT / "cards/capabilities/registry.json")
    by_key, cycle_paths = validate_capability_registry(registry, root=ROOT)
    if cycle_paths:
        raise ValueError(f"canonical registry contains dependency cycles: {cycle_paths}")

    projection = {
        "schema_version": "card-ir-capability-projection.v1",
        "registry_id": registry["registry_id"],
        "entries": [
            {
                "key": key,
                "version": entry["version"],
                "lifecycle": entry["lifecycle"],
                "lifecycle_rank": LIFECYCLE_ORDER[entry["lifecycle"]],
                "dependencies": sorted(entry.get("dependencies", [])),
            }
            for key, entry in sorted(by_key.items())
        ],
    }

    cases: list[dict[str, Any]] = []
    lifecycle_cases = [
        ("specified", "proposed", "specified"),
        ("implemented", "implemented", "covered"),
        ("certified", "certified", "certified"),
    ]
    for root_lifecycle, dependency_lifecycle, minimum in lifecycle_cases:
        root_entry = copy.deepcopy(by_key["rules/basic-priority"])
        dependency_entry = copy.deepcopy(by_key["rules/turn-structure"])
        root_entry.update(
            key="rules/parity-root",
            lifecycle=root_lifecycle,
            dependencies=["rules/parity-dependency"],
        )
        dependency_entry.update(
            key="rules/parity-dependency",
            lifecycle=dependency_lifecycle,
            dependencies=[],
        )
        synthetic_registry = {
            "schema_version": "capability-registry.v1",
            "registry_id": registry["registry_id"],
            "entries": [root_entry, dependency_entry],
        }
        validate_capability_registry(synthetic_registry, root=ROOT)
        roots = [{"key": root_entry["key"], "version": root_entry["version"]}]
        bundle = {
            "schema_version": "bundle-manifest.v1",
            "bundle_id": "test/capability-parity",
            "required_capabilities": [root["key"] for root in roots],
        }
        census = capability_census(
            bundle,
            synthetic_registry,
            root=ROOT,
            required_capability_lifecycle=minimum,
        )
        cases.append(
            {
                "minimum_lifecycle": minimum,
                "entries": [
                    {
                        "key": entry["key"],
                        "version": entry["version"],
                        "lifecycle": entry["lifecycle"],
                        "lifecycle_rank": LIFECYCLE_ORDER[entry["lifecycle"]],
                        "dependencies": sorted(entry.get("dependencies", [])),
                    }
                    for entry in (root_entry, dependency_entry)
                ],
                "roots": roots,
                "resolved": list(census.resolved),
                "below_required_lifecycle": [list(row) for row in census.below_required_lifecycle],
                "missing": list(census.missing),
                "cycles": [list(cycle) for cycle in census.cycles],
            }
        )
    parity = {
        "schema_version": "capability-closure-parity.v1",
        "registry_id": registry["registry_id"],
        "cases": cases,
    }

    def encode(value: Any) -> bytes:
        return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()

    return encode(projection), encode(parity)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    try:
        projection, parity = render()
    except (KeyError, TypeError, ValueError) as error:
        parser.error(str(error))
    expected = {PROJECTION: projection, PARITY: parity}
    drift = []
    for path, content in expected.items():
        if args.check:
            if not path.is_file() or path.read_bytes() != content:
                drift.append(path.relative_to(ROOT).as_posix())
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
    if drift:
        print("generated Card-IR capability artifact drift:")
        print("\n".join(f"- {path}" for path in drift))
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
