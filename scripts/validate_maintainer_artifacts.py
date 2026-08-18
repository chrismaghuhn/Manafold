#!/usr/bin/env python3
from __future__ import annotations

import sys

sys.dont_write_bytecode = True
import jsonschema
from maintainer_common import (
    ROOT,
    MaintainerArtifactError,
    capability_census,
    certification_closure,
    load_json,
    validate_bundle_manifest,
    validate_capability_registry,
    validate_card_manifest,
)

CASES = [
    ("schemas/capability-registry.v1.schema.json", "cards/capabilities/registry.json"),
    (
        "schemas/capability-registry.v1.schema.json",
        "cards/capabilities/registry.example.json",
    ),
    (
        "schemas/card-definition-manifest.v1.schema.json",
        "cards/definitions/example/card/example-card/manifest.example.json",
    ),
    (
        "schemas/bundle-manifest.v1.schema.json",
        "cards/bundles/example-v1/manifest.example.json",
    ),
    (
        "schemas/bundle-certification.v1.schema.json",
        "cards/bundles/example-v1/certification.example.json",
    ),
    (
        "schemas/scope-impact-report.v1.schema.json",
        "cards/bundles/example-v1/scope-impact.example.json",
    ),
    (
        "schemas/normative-document-register.v1.schema.json",
        "docs/normative-document-register.v1.json",
    ),
]


def main() -> None:
    for schema_rel, value_rel in CASES:
        schema = load_json(ROOT / schema_rel)
        value = load_json(ROOT / value_rel)
        jsonschema.Draft202012Validator(schema).validate(value)

    actual_registry = load_json(ROOT / "cards/capabilities/registry.json")
    example_registry = load_json(ROOT / "cards/capabilities/registry.example.json")
    validate_capability_registry(actual_registry, root=ROOT)
    _, cycles = validate_capability_registry(example_registry, root=ROOT)
    if cycles:
        raise MaintainerArtifactError(f"example registry has dependency cycles: {cycles}")

    card = load_json(ROOT / "cards/definitions/example/card/example-card/manifest.example.json")
    validate_card_manifest(card, root=ROOT)
    bundle = load_json(ROOT / "cards/bundles/example-v1/manifest.example.json")
    validate_bundle_manifest(bundle)
    census = capability_census(
        bundle, example_registry, root=ROOT, required_capability_lifecycle="proposed"
    )
    if census.missing or census.cycles or census.missing_definitions:
        raise MaintainerArtifactError(f"example census is structurally incomplete: {census}")

    certification = load_json(ROOT / "cards/bundles/example-v1/certification.example.json")
    certification_census = capability_census(
        bundle,
        example_registry,
        root=ROOT,
        required_capability_lifecycle="certified",
        required_card_lifecycle="covered",
    )
    expected_closure = certification_closure(certification_census)
    if certification["capability_closure"] != expected_closure:
        raise MaintainerArtifactError(
            "example certification closure does not match bundle/registry census"
        )
    if certification["status"] == "certified" and any(
        gate["status"] != "PASS" for gate in certification["gates"]
    ):
        raise MaintainerArtifactError("certified example contains a non-PASS gate")

    print(f"PASS: {len(CASES)} maintainer artifacts validated with semantic closure")


if __name__ == "__main__":
    main()
