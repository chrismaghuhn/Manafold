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
- Replay V2: provisional-public M1 replay identity; after the M2 state cut it is `READABLE_VERIFIABLE_ONLY` in the current engine and is not semantically executed against M2 `EngineState`.
- M2 Decision V2, Information/Event/PlayerStep V2, synthetic observation payload V1: `experimental` during M2.A–M2.H; promotion to `provisional-public` requires M2 executable closure.
- `FullStateDigestV3`, Checkpoint V3 and Replay V3: experimental/freeze-candidate until their ADR-0038 codec/schema fixtures and executable parity gates pass.
- `FullStateDigestV5`, Checkpoint V6 and Replay V6: exact historical identity family after PR #248 activated the M4 V6/V7 cut on `master`; digest/replay readers and verifiers retain their original meanings, V6 checkpoint restore is unsupported by the current V7 runtime.
- `FullStateDigestV6`, `EnvironmentCheckpointV7` / `CheckpointDigestV7`, and Replay V7: current resumable identity family on `master` for the bounded M4.2 Mountain/Plains slice after PR #248; this identity status does not imply broader card/deck support.
- M4 player products on `master` after PR #248: PlayerDecisionRequestV3, DecisionResponseV2, ObservationEnvelopeV1, PlayerInformationStateV2 / InformationStateDigestV2, ObservedEventEnvelopeV3, and PlayerStepV3 are the current bounded M4.2 endpoint family; each retains its own exact versioned meaning.
- FullStateDigest V4, Checkpoint V5, and Replay V5 retain their exact original meanings as historical evidence; they are not interpreted as V5-state/V6-checkpoint artifacts.
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
## M2-cut historical V2 support matrix

Once the M2 V3 runtime cut lands, current-engine support is frozen as follows:

| V2 surface | Current writer | Current reader | Current verifier | Current semantic execution | Migration | Classification |
|---|---:|---:|---:|---:|---:|---|
| `FullStateDigestV2` / detached V2 digest input evidence | no | digest/reference parsing only | yes, against immutable V2 known-answer/domain fixtures | n/a | n/a | `READABLE_VERIFIABLE_ONLY` |
| `EnvironmentCheckpointV2` | no | no current-runtime checkpoint reader | detached V2 digest/contract evidence only | no; requires the archived matching M1 engine build | none defined | `UNSUPPORTED` by the current engine |
| `ReplayManifestV2` / `AuthoritativeReplayV2` | no | yes only as detached/version-specific V2 DTO where retained | yes, structural/identity validation under the V2 contract | no current-engine replay execution after the state cut | none defined | `READABLE_VERIFIABLE_ONLY` |

`EnvironmentCheckpointV2` never had a durable detached historical state codec; therefore current M2 code must not pretend to read it by deserializing into the changed `EngineState`. Historical M1 execution remains reproducible only with the archived matching engine/source identity.

A future explicit V2→V3 migration ADR may change only the `Migration` column by adding a provenance-preserving Rust-authoritative migration. It cannot relabel or reinterpret the source artifact.

## V4–V6 historical and V7 current support matrix

ADR 0055 §2.12 freezes the historical V4→V5 cut. S3.P0 added the V5-state/V6-checkpoint/replay cut, which PR #248 later superseded on `master` with the bounded M4 V6-state/V7-checkpoint/replay cut. No cut defines an automatic migration.

| Surface | Writer | Reader | Verifier | Semantic execution | Migration | Classification |
|---|---|---|---|---|---|---|
| `FullStateDigestV4` | no current writer | detached exact reader | yes, exact V4 bytes | n/a | none | `READABLE_VERIFIABLE_ONLY` |
| `EnvironmentCheckpointV4` | no | no durable reader | digest evidence only | no under V6 runtime | none | `UNSUPPORTED` |
| `CheckpointDigestV4` | no | yes | yes | n/a | none | `READABLE_VERIFIABLE_ONLY` |
| `ReplayManifestV4` | no | yes | yes, detached | no | none | `READABLE_VERIFIABLE_ONLY` |
| `ReplayStepV4` | no | yes | yes, detached | no | none | `READABLE_VERIFIABLE_ONLY` |
| `AuthoritativeReplayV4` | no | yes | yes, detached | archived matching V4 runtime ONLY | none | `READABLE_VERIFIABLE_ONLY` |
| `FullStateDigestV5` | no current writer | detached exact reader | yes, exact V5 canonical input | historical only | none | `READABLE_VERIFIABLE_ONLY` |
| `EnvironmentCheckpointV5` | no current writer | historical in-memory value only | exact historical validator | no under V7 runtime | none | `UNSUPPORTED` for current restore |
| `CheckpointDigestV5` | no current writer | detached input/value | yes, exact V5 preimage | n/a | none | `READABLE_VERIFIABLE_ONLY` |
| `ReplayManifestV5` / `ReplayStepV5` / `AuthoritativeReplayV5` | no current writer | yes, version-specific DTO | yes, exact V5 identity chain | archived matching V5 runtime only | none | `READABLE_VERIFIABLE_ONLY` |
| `EnvironmentCheckpointV6` | no current writer | historical in-memory value only | exact historical validator | no under V7 runtime | none | `UNSUPPORTED` for current restore |
| `CheckpointDigestV6` | no current writer | detached input/value | yes, exact V6 preimage | n/a | none | `READABLE_VERIFIABLE_ONLY` |
| Replay V6 family | no current writer | yes, version-specific DTO | yes, exact V6 identity chain | archived matching V6 runtime only | none | `READABLE_VERIFIABLE_ONLY` |
| `FullStateDigestV6` | yes | yes | yes, V6 canonical input | current bounded M4.2 state identity | n/a | `EXECUTABLE` within locked slice |
| `EnvironmentCheckpointV7` / `CheckpointDigestV7` | yes | yes | yes, V6 state digest + V7 checkpoint identity | current bounded M4.2 checkpoint identity | n/a | `EXECUTABLE` within locked slice |
| Replay V7 family | yes | yes | yes, exact V7 identity chain | current bounded M4.2 replay identity | n/a | `EXECUTABLE` within locked slice |
| V4 → V5 automatic migration | — | — | — | — | NONE | — |
| V5 → V6 automatic migration | — | — | — | — | NONE | — |
| V6 → V7 state/checkpoint/replay automatic migration | — | — | — | — | NONE | — |

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
