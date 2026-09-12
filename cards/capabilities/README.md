# Capability Registry

`registry.json` contains the specified durable V1 capability definitions required by the locked Token Triumph versus Grave Danger scope. M3 consumes the registry and the specifications under `docs/rules/capabilities/` directly; the entries do not claim implementation, coverage, or certification. `registry.example.json` demonstrates the schema without claiming support.

`sources/m2_5/scope/selected_pair_durable_capability_mapping.v1.json` is a minimal migration/validation map from selected B2 family identities to durable keys. It is not a registry, dependency authority, support claim, or certification evidence and may be removed after M2.5 Final when the final scope remains independently resolvable.

Use `python scripts/scaffold_capability.py <key> <title>` to add a proposal. Lifecycle and certification rules are defined in `docs/cards/CAPABILITY_MODEL.md`.
