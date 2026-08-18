# Capability Model

**Status:** accepted model  
**Stability:** normative registry and closure contract

## Purpose

Capabilities are the smallest versioned units of semantic support that cards, formats, and bundles can declare and verify. They prevent “card supported” from meaning “the source file exists.”

## Registry entry

A capability entry contains:

- stable key and semantic version;
- category and lifecycle state;
- concise scope statement;
- transitive dependency keys;
- authority references;
- specification path;
- implementation paths;
- conformance case IDs;
- information-risk classification;
- benchmark scenario IDs where relevant;
- owner roles and notes.

The normative shape is [`../../schemas/capability-registry.v1.schema.json`](../../schemas/capability-registry.v1.schema.json). Cross-entry uniqueness, dependency existence, cycle checks, safe repository-relative paths, specification existence, and lifecycle-specific evidence are semantic validation beyond JSON Schema. In particular, `implemented` requires an existing implementation path, `covered` requires conformance evidence, and `specified` or later cannot retain an unreviewed information-risk classification.

## Closure sources

A bundle’s required closure is the union of:

1. bundle-level requirements;
2. format requirements;
3. every card-definition requirement;
4. generated/reference-object requirements;
5. all transitive capability dependencies;
6. approved native-executor capabilities, if any.

## Closure result

A census reports:

```text
required                       # direct bundle/card requirements
resolved                       # complete transitive registered closure
missing                        # unregistered capability keys
cycles                         # complete dependency paths
below_required_lifecycle       # { key, lifecycle }
missing_definitions             # absent reachable card/generated definitions
card_lifecycle_blockers         # { definition_id, lifecycle }
native_executors                # quarantined executor IDs
```

The static certification report preserves these blockers as typed fields rather than flattening them into diagnostic strings. `required` and `resolved` are intentionally distinct: dependencies appear in `resolved`, while `required` identifies the direct closure roots.

No missing or cyclic closure can load as certified. The reference environment may load experimental content only under an explicit non-certified configuration that cannot produce support claims.

## Versioning

Capability key meaning is stable. Compatible evidence additions may retain the version. A semantic change that can alter legal actions, state, visibility, events, outcomes, replay, or dataset meaning requires a new capability version and recertification of all dependent bundles.

## Native implementation closure

Capability closure and implementation closure are related but distinct. Capabilities come from bundle and definition requirements; native executors come only from reachable definition manifests and are cross-checked against bundle declarations. The census never trusts a bundle-authored executor list as proof that the list is complete.
