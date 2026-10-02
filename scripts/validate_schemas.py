#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
import jsonschema

ROOT = Path(__file__).resolve().parents[1]
WIRE_MAPPING = {
    "player-decision-request.v4": "player-decision-request.v4.schema.json",
    "decision-response.v3": "decision-response.v3.schema.json",
    "episode-status.v1": "episode-status.v1.schema.json",
    "observation-envelope.v2": "observation-envelope.v2.schema.json",
    "information-state-envelope.v3": "information-state-envelope.v3.schema.json",
    "observed-event-envelope.v4": "observed-event-envelope.v4.schema.json",
    "player-step.v4": "player-step.v4.schema.json",
    "magic-basic-land-observation.v1": "magic-basic-land-observation.v1.schema.json",
    "magic-shared-execution-observation.v1": "magic-shared-execution-observation.v1.schema.json",
    "replay-manifest.v8": "replay-manifest.v8.schema.json",
    "authoritative-replay.v8": "authoritative-replay.v8.schema.json",
    "replay-step.v8": "replay-step.v8.schema.json",
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
SCHEMA_NEGATIVE_CASES = [
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-hand-size-discard-public.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-mulligan-bottom-public.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-blocker-declaration-public.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-blocker-declaration-attacker-missing.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-blocker-declaration-wrong-candidate.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-cost-route-normal-with-ordinal.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-cost-route-alternative-without-ordinal.json",
    ),
    (
        "replay-manifest.v8.schema.json",
        "schemas/negative/replay-manifest-v8-program-authority-mismatch.json",
    ),
    (
        "authoritative-replay.v8.schema.json",
        "schemas/negative/authoritative-replay-v8-program-authority-mismatch.json",
    ),
    (
        "replay-manifest.v8.schema.json",
        "schemas/negative/replay-manifest-v8-old-response-identity.json",
    ),
    (
        "authoritative-replay.v8.schema.json",
        "schemas/negative/authoritative-replay-v8-wrong-version.json",
    ),
    (
        "observed-event-envelope.v4.schema.json",
        "schemas/negative/observed-event-v4-global-revision.json",
    ),
    (
        "observed-event-envelope.v4.schema.json",
        "schemas/negative/observed-event-v4-trusted-stack-id.json",
    ),
    (
        "observed-event-envelope.v4.schema.json",
        "schemas/negative/observed-event-v4-mulligan-not-boolean.json",
    ),
    (
        "player-step.v4.schema.json",
        "schemas/negative/player-step-v4-global-revision.json",
    ),
    (
        "observation-envelope.v2.schema.json",
        "schemas/negative/observation-envelope-v2-global-state-revision.json",
    ),
    (
        "information-state-envelope.v3.schema.json",
        "schemas/negative/information-state-envelope-v3-global-state-revision.json",
    ),
    (
        "magic-basic-land-observation.v1.schema.json",
        "schemas/negative/magic-basic-land-observation-v1-candidates.json",
    ),
    (
        "magic-basic-land-observation.v1.schema.json",
        "schemas/negative/magic-basic-land-observation-v1-life-as-string.json",
    ),
    (
        "magic-basic-land-observation.v1.schema.json",
        "schemas/negative/magic-basic-land-observation-v1-players-missing.json",
    ),
    (
        "magic-basic-land-observation.v1.schema.json",
        "schemas/negative/magic-basic-land-observation-v1-power-as-string.json",
    ),
    (
        "magic-basic-land-observation.v1.schema.json",
        "schemas/negative/magic-basic-land-observation-v1-arrival-turn-as-number.json",
    ),
    (
        "magic-basic-land-observation.v1.schema.json",
        "schemas/negative/magic-basic-land-observation-v1-power-without-toughness.json",
    ),
    (
        "magic-basic-land-observation.v1.schema.json",
        "schemas/negative/magic-basic-land-observation-v1-permanents-missing.json",
    ),
    (
        "magic-shared-execution-observation.v1.schema.json",
        "schemas/negative/magic-shared-execution-observation-v1-tapped-repeated.json",
    ),
    (
        "magic-shared-execution-observation.v1.schema.json",
        "schemas/negative/magic-shared-execution-observation-v1-attacking-repeated.json",
    ),
    (
        "magic-shared-execution-observation.v1.schema.json",
        "schemas/negative/magic-shared-execution-observation-v1-global-state-revision.json",
    ),
    (
        "magic-shared-execution-observation.v1.schema.json",
        "schemas/negative/magic-shared-execution-observation-v1-trusted-stack-id.json",
    ),
    (
        "magic-shared-execution-observation.v1.schema.json",
        "schemas/negative/magic-shared-execution-observation-v1-unknown-effect-keyword.json",
    ),
    (
        "magic-shared-execution-observation.v1.schema.json",
        "schemas/negative/magic-shared-execution-observation-v1-unbound-ability.json",
    ),
    (
        "magic-shared-execution-observation.v1.schema.json",
        "schemas/negative/magic-shared-execution-observation-v1-activated-modes-missing.json",
    ),
    (
        "decision-response.v3.schema.json",
        "schemas/negative/decision-response-v3-global-state-revision.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-global-state-revision.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-trusted-trigger-id.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-event-subject-mismatch.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-trusted-trigger-instance-id.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-purpose-domain-mismatch.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-trigger-order-public.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-cost-route-public.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-sba-graveyard-order-public.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-sba-graveyard-order-wrong-candidate.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-unbound-source-ability.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-synthetic-stage-domain-mismatch.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-attacker-wrong-candidate-intent.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/negative/player-decision-request-v4-synthetic-wrong-candidate-intent.json",
    ),
    (
        "replay-manifest.v8.schema.json#content-contract-child",
        "schemas/negative/content-contract-child-unknown-field.json",
    ),
    (
        "replay-manifest.v8.schema.json#content-contract-child",
        "schemas/negative/content-contract-child-invalid-base64.json",
    ),
    (
        "replay-manifest.v8.schema.json#content-presence-rule",
        "schemas/negative/content-contract-presence-mismatch.json",
    ),
    (
        "replay-manifest.v8.schema.json#content-presence-rule",
        "schemas/negative/content-contract-id-without-child.json",
    ),
    (
        "replay-manifest.v8.schema.json#content-contract-child",
        "schemas/negative/content-contract-child-uppercase-id.json",
    ),
    (
        "replay-manifest.v8.schema.json#content-contract-child",
        "schemas/negative/content-contract-child-extra-padding.json",
    ),
    (
        "replay-manifest.v8.schema.json#content-contract-child",
        "schemas/negative/content-contract-child-whitespace.json",
    ),
    (
        "replay-manifest.v8.schema.json#content-contract-child",
        "schemas/negative/content-contract-child-missing-padding.json",
    ),
]
SCHEMA_POSITIVE_CASES = [
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-cost-route.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-sba-graveyard-order.json",
    ),
    ("replay-manifest.v8.schema.json", "schemas/examples/replay-manifest-v8.json"),
    ("replay-step.v8.schema.json", "schemas/examples/replay-step-v8.json"),
    ("authoritative-replay.v8.schema.json", "schemas/examples/authoritative-replay-v8.json"),
    (
        "magic-shared-execution-observation.v1.schema.json",
        "schemas/examples/magic-shared-execution-observation-v1.json",
    ),
    ("observation-envelope.v2.schema.json", "schemas/examples/observation-envelope-v2.json"),
    (
        "information-state-envelope.v3.schema.json",
        "schemas/examples/information-state-envelope-v3.json",
    ),
    ("decision-response.v3.schema.json", "schemas/examples/decision-response-v3.json"),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-trigger-order.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-attacker-declaration.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-hand-size-discard.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-starting-player.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-mulligan-declaration.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-mulligan-bottom.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-blocker-declaration.json",
    ),
    (
        "player-step.v4.schema.json",
        "schemas/examples/player-step-v4-hand-size-discard.json",
    ),
    (
        "player-step.v4.schema.json",
        "schemas/examples/player-step-v4-blocker-declaration.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-synthetic-entry.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-synthetic-count.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-synthetic-members.json",
    ),
    (
        "player-decision-request.v4.schema.json",
        "schemas/examples/player-decision-request-v4-synthetic-order.json",
    ),
    (
        "magic-basic-land-observation.v1.schema.json",
        "schemas/examples/magic-basic-land-observation-v1.json",
    ),
    (
        "magic-basic-land-observation.v1.schema.json",
        "schemas/examples/magic-basic-land-observation-v1-ordered.json",
    ),
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

    identities: dict[str, str] = {}
    for name in expected:
        schema = load(schema_root / name)
        if not isinstance(schema, dict):
            raise ValueError(f"wire schema must be an object: {name}")
        jsonschema.Draft202012Validator.check_schema(schema)
        schema_id = schema.get("$id")
        if schema_id != name:
            raise ValueError(f"wire schema $id must match filename: {name} has {schema_id!r}")
        previous = identities.get(schema_id)
        if previous is not None:
            raise ValueError(f"duplicate wire schema $id: {schema_id} in {previous} and {name}")
        identities[schema_id] = name


def main() -> None:
    validate_wire_schema_inventory(load(ROOT / "schemas" / "README.json"))
    for schema_name, fixture_rel in SCHEMA_POSITIVE_CASES:
        jsonschema.Draft202012Validator(load(ROOT / "schemas" / schema_name)).validate(
            load(ROOT / fixture_rel)
        )
    for schema_ref, fixture_rel in SCHEMA_NEGATIVE_CASES:
        schema_name, _, fragment = schema_ref.partition("#")
        schema = load(ROOT / "schemas" / schema_name)
        if fragment == "content-contract-child":
            schema = schema["$defs"]["semantic_contract_material"]["properties"][
                "content_contract"
            ]["oneOf"][1]
        elif fragment == "content-presence-rule":
            schema = schema["$defs"]["semantic_contract_material"]["allOf"][0]
        fixture = load(ROOT / fixture_rel)
        if jsonschema.Draft202012Validator(schema).is_valid(fixture):
            raise ValueError(f"schema accepted negative fixture: {fixture_rel}")
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
        f" {len(ARTIFACT_CASES)} maintainer artifacts validated against schemas; "
        f"{len(SCHEMA_POSITIVE_CASES)} successor-schema examples accepted and "
        f"{len(SCHEMA_NEGATIVE_CASES)} negative cases rejected"
    )


if __name__ == "__main__":
    main()
