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


def main() -> None:
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
