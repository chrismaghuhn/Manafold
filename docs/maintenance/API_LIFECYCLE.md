# API Lifecycle

**Status:** accepted  
**Stability:** normative compatibility process

## Stability classes

| Class | Meaning |
|---|---|
| `internal` | may change freely within one coherent change set; not consumed externally |
| `experimental` | externally visible for prototyping; versioned but may break with compatibility notes |
| `provisional-public` | intended public shape; breaking changes require ADR and migration/retirement analysis |
| `frozen-public` | support commitment for declared versions; compatibility policy applies strictly |

## Current classification

- M1 Decision/Observation/Information/Event/PlayerStep V1 contracts: current M1 executable/provisional-public meanings until the M2 structural cut; once superseded they retain that exact historical meaning and are not reinterpreted as M2.
- Replay V2: provisional-public M1 replay identity; after the M2 state cut its historical read/execution support is explicitly classified rather than assumed from a current runtime type.
- M2 Decision V2, Information/Event/PlayerStep V2, synthetic observation payload V1: `experimental` during M2.A–M2.H; promotion to `provisional-public` requires M2 executable closure.
- `FullStateDigestV3`, Checkpoint V3 and Replay V3: experimental/freeze-candidate until their ADR-0038 codec/schema fixtures and executable parity gates pass.
- the temporary M2 subprocess Python semantic adapter: internal/experimental test infrastructure; never a production transport promise.
- concrete Card IR variants: experimental.
- Rust crate APIs: internal/experimental unless explicitly registered otherwise.
- semantic action keys and ML trajectory schema: experimental/open under OD-011.
- production Python/native transport: open under OD-009.

## Historical runtime types

A versioned name does not guarantee indefinite current-engine executability.

If a historical runtime type embeds the unversioned current `EngineState`, a later state-layout/semantic change may require retiring that runtime producer/type rather than silently changing historical meaning.

Historical support is classified explicitly as:

```text
EXECUTABLE
MIGRATION_REQUIRED
READABLE_VERIFIABLE_ONLY
UNSUPPORTED
```

Do not create a duplicate legacy rules/state engine solely to make an old in-memory type appear executable.

## Deprecation and version changes

A public value is never repurposed.

When meaning changes:

- allocate a new schema/domain/version;
- preserve immutable historical fixtures/documentation;
- classify reader/writer/execution support;
- provide migration only where justified and Rust-authoritative;
- never overwrite historical artifacts with migrated values.

New readers may support old and new versions only when they can preserve each version's exact original contract. “Deserialize into the current runtime type” is not sufficient evidence.

## Freeze rule

A freeze candidate is not frozen public API.

M2 documentation/ADRs can freeze architecture for implementation while player V2/V3 executable surfaces remain experimental until the required Rust/Python/schema/replay/noninterference evidence passes.

## Registration

The normative document register and compatibility policy identify binding public surfaces. Merely making a Rust item `pub` does not freeze it.
