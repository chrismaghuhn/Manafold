# Mechanic and Capability Lifecycle

**Status:** accepted  
**Stability:** normative certification vocabulary

## Lifecycle states

| State | Meaning |
|---|---|
| `proposed` | key and owner exist; semantics may be incomplete |
| `specified` | authority, scope, state, decisions, visibility, ordering, and exclusions are reviewed |
| `implemented` | reference backend executes the declared scope and fails closed outside it |
| `covered` | required conformance, property, replay, and information tests exist and pass |
| `certified` | the capability version is included in a locked bundle whose complete closure passes all gates |
| `deprecated` | no new bundle may depend on it; migration/supersession documented |

Lifecycle is monotonic except that a discovered defect may revoke `covered` or `certified` status for affected snapshots/bundles.

## Capability key grammar

```text
rules/<name>
mechanic/<name>
format/<format>/<name>
decision/<name>
visibility/<name>
tooling/<name>
```

Keys use lowercase ASCII letters, digits, and hyphens. Meaning never changes in place. Semantic breaks create a new version.

## Dependency closure

A capability may require other capabilities. The registry must be acyclic and all dependencies must exist. Bundle closure includes direct requirements, transitive dependencies, card requirements, generated objects, format requirements, and any approved native-executor capability.

## Evidence

Registry entries point to implementation paths, mechanic specs, conformance case IDs, information review, and benchmark scenarios. Paths are evidence references, not proof by existence; certification tooling verifies the referenced gate results.
