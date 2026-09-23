# M3.S3.P0 Authoritative State Identity Cut Design

**Status:** implementation design; no implementation authorized by this document
**Date:** 2026-09-23
**Task:** M3.S3 TASK 2 FIX-02
**Required base master:** `b67cfdcc0a8e623da52a889ef2ae138a3e4256ac`
**Required parent head:** `ff33168dec92227355a199c1fb970778f9c8777d`

This design allocates new authoritative state, checkpoint and replay
identities for the typed Magic SBA ordering continuation required by S3.A. It
changes no Magic rules behavior or capability lifecycle. Production work
remains unauthorized until a separate implementation task.

## 1. Identity problem and binding authority

The current `ContinuationPayloadV2` admits only `SyntheticM2Assembly`.
`FullStateDigestV4` has a single closed canonical continuation encoding for
that payload. S3.A requires a Magic continuation containing the complete
selected SBA round and APNAP-collected Graveyard permutations. That data is
authoritative `EngineState`: it must survive staged responses, checkpoint and
restore, forks, digest validation, and replay.

ADR 0054 is the binding precedent. When M3 introduced the initial authoritative
state meaning, it cut FullStateDigest V3→V4, checkpoint V3→V4,
CheckpointDigest V3→V4, and Replay V3→V4 together. ADR 0055 subsequently made
Checkpoint V5 bind `ExecutionIdentityV1`, while explicitly leaving
`FullStateDigestV4` and its input schema unchanged. API Lifecycle forbids
reinterpreting a versioned value when its meaning changes. The new Magic
continuation therefore cannot be encoded as an in-place extension of
FullStateDigestV4 or represented by V5 checkpoint/replay identities.

Binding authorities are:

- [ADR 0054](../../adr/0054-m3-pre-t0-hardening.md) for coordinated state,
  checkpoint and replay identity cuts;
- [ADR 0055](../../adr/0055-v5-execution-identity.md) for execution identity
  and the FullStateDigestV4/V5 checkpoint boundary;
- [State and Artifact Hashing](../../STATE_HASHING.md) for detached canonical
  digest encoding;
- [Engine State Closure](../../contracts/ENGINE_STATE_CLOSURE.md) for
  complete authoritative state;
- [Replay and Determinism](../../REPLAY_AND_DETERMINISM.md),
  [API Lifecycle](../../maintenance/API_LIFECYCLE.md), and
  [Schema Evolution](../../maintenance/SCHEMA_EVOLUTION.md) for historical
  support and compatibility.

## 2. Required current identity family

S3.P0 makes V5 state identity and V6 checkpoint/replay identity current:

| Surface | S3.P0 identity |
| --- | --- |
| Full state | `FullStateDigestV5`, `full-state-digest-input.v5`, `mtgml.full-state-digest.v5` |
| Environment checkpoint | `EnvironmentCheckpointV6`, `environment-checkpoint.v6` |
| Checkpoint digest | `CheckpointDigestV6`, `environment-checkpoint-digest-input.v6`, `mtgml.checkpoint-digest.v6` |
| Replay manifest | `ReplayManifestV6`, `replay-manifest.v6` |
| Replay step | `ReplayStepV6`, `replay-step.v6` |
| Authoritative replay | `AuthoritativeReplayV6`, `authoritative-replay.v6` |
| Recorder/schema inventory | `ReplayRecorderV6`, `ReplaySchemaVersionsV6` |
| Initial identity | `InitialEnvironmentIdentityV6` |

`EnvironmentCheckpointV6` embeds the complete `EngineState`,
`FullStateDigestV5`, status, environment counters, codec identity
`in-memory-reference / 6`, full `ExecutionIdentityV1`, and
`CheckpointDigestV6`. Its checkpoint digest has a distinct V6 schema/domain
and binds the complete V5 state digest plus the full execution identity.

`ReplayStepV6` preserves one explicit typed `DecisionResponseV2` per replay
step. Each APNAP Graveyard `Order` response is a real player step; deterministic
SBA and zone consequences remain consequences of that response. V6 adds no
forced-progress input, fake response, event-as-input, implicit pass, or new
Decision response family.

`ExecutionProgramV1`, `ExecutionIdentityV1`, `DecisionResponseV2`,
`ObservationEnvelopeV1`, and `InformationStateDigestV2` retain their exact
current meanings. The S3 semantic-contract ID changes through the existing
content-addressed contract mechanism when the S3 capability closure is
implemented.

## 3. FullStateDigestV5 canonical continuation mapping

`FullStateDigestV5` is a new detached canonical-CBOR state identity. Its
`full-state-digest-input.v5` mapping covers every current `EngineState`
component and the typed Magic continuation. The continuation payload is a
closed, versioned Magic variant with a unique canonical tag, conceptually:

```text
magic_sba_graveyard_order_v1
```

The canonical state carries:

```text
round_start_revision
complete selected SBA action set and causes
owners requiring an Order choice in APNAP order
next APNAP owner index
completed owner -> exact top-to-bottom GameObjectId permutation
```

The final Rust field layout may change only if the complete resume meaning is
preserved. Encoding uses unique discriminants, fixed field order, canonical
keyed-collection ordering, checked integer representations, and strict
rejection of unknown or malformed payloads. The continuation stores no
controller-local history, cached P/T, or unverified before-state copy.

FullStateDigestV4 remains immutable historical meaning and retains exact
known-answer bytes. FullStateDigestV5 gets its own input schema/domain and
KATs. Rust and Python must produce identical canonical bytes and digest values
for V5; every old V4 vector continues to verify with the detached V4 codec.

## 4. EnvironmentCheckpointV6 and Replay V6

The current V5 types concretely contain `FullStateDigestV4` and
`CheckpointDigestV5`. They cannot represent the new state identity. Add new
V6 types; do not alter the V5 DTOs:

```text
EnvironmentCheckpointV6.state_digest = FullStateDigestV5
EnvironmentCheckpointV6.checkpoint_digest = CheckpointDigestV6

InitialEnvironmentIdentityV6.full_state_digest = FullStateDigestV5
InitialEnvironmentIdentityV6.checkpoint_digest = CheckpointDigestV6

ReplayStepV6.before/after checkpoint digest = CheckpointDigestV6
ReplayStepV6.full_state_digest_after = FullStateDigestV5
```

ReplayManifestV6 binds the V6 identities, the full `ExecutionIdentityV1`,
the S3 semantic contract, and the Magic observation codec. It can retain the
previous synthetic observation codec for contracts that use it. The Magic
APNAP observation progress uses `magic-m3-observation.v1` under the existing
Observation Envelope V1 codec slot. `ReplayStepV6` input remains one real
`DecisionResponseV2` per step.

The S3 response path remains:

```text
real response
-> one rules-owned forced consequence if no Decision exists
-> any real APNAP Order responses as their own replay steps
-> SBA batch and selected S2 transitions
-> next real Decision/outcome/error
```

No replay step is added for forced progress. Replay V6 is required solely
because its state/checkpoint identity fields otherwise remain V4/V5-typed;
it adds no unrelated replay behavior.

## 5. Historical support and migrations

After the current runtime cut:

| Artifact | V6 runtime disposition |
| --- | --- |
| `FullStateDigestV4` / V4 canonical inputs | No current writer; detached exact verifier `READABLE_VERIFIABLE_ONLY` |
| `EnvironmentCheckpointV5` | `UNSUPPORTED` by the V6 runtime; archived matching V5 runtime required for execution |
| `CheckpointDigestV5` | Detached exact verifier `READABLE_VERIFIABLE_ONLY` |
| `ReplayManifestV5` / `ReplayStepV5` / `AuthoritativeReplayV5` | Detached exact validation; semantic execution only under an archived matching V5 runtime; `READABLE_VERIFIABLE_ONLY` in V6 runtime |
| Automatic V5→V6 migration | `NONE` |

Never deserialize an old checkpoint into a changed runtime state and call it
migrated. No migration invents continuation state or execution identity, and
no source artifact is overwritten. If a future migration is required, design
it as separate versioned, Rust-authoritative code with preserved source
provenance and exact target validation.

## 6. Observation and player contracts

Do not version `ObservationEnvelopeV1`, `ObservationDigestV1`, or
`InformationStateDigestV2`: their current meaning already binds payload bytes
through the independently named payload codec and observation digest. Add the
Magic order-progress projection as `magic-m3-observation.v1`, leaving
`synthetic-m3-observation.v1` bytes unchanged.

The V6 replay manifest schema binds the payload codec to the supported
semantic contract. No arbitrary payload-codec strings are admitted. Public
Decision, response, event-envelope, information-state-envelope and PlayerStep
DTO families remain their current V2 identities; Graveyard ordering uses
existing `DecisionDomainV2::Order` and `DecisionAnswerV2::Order`.

## 7. S3.P0 non-goals

S3.P0 adds identity/schema/persistence/restore support only. It does not add a
kernel producer for the Magic continuation, create an SBA/Order Decision,
apply Graveyard moves, add a capability dependency, update any lifecycle, or
change player actions/products for already supported S1/S2 states. The
continuation variant remains unreachable through production Magic rules until
the separately reviewed S3.A implementation.

Required parity:

```text
MAGIC_RULE_BEHAVIOR_CHANGED = NO
LEGAL_ACTIONS_CHANGED = NO
CURRENT_SUPPORTED_S1_S2_TRANSITIONS_CHANGED = NO
PLAYER_PRODUCTS_CHANGED = NO for existing contracts/codecs
RNG_CHANGED = NO
S2_LIFECYCLE_CHANGED = NO
S3_CAPABILITY_LIFECYCLE_CHANGED = NO
```

V5/V6 identities intentionally differ because they are new versioned
identities. V4/V5 bytes and validation meanings remain exact.

## 8. Required S3.P0 evidence

- V5 full-state digest KAT for a Magic continuation fixture and Rust/Python
  canonical-byte parity.
- Every V4 KAT and detached historical verifier remains exact.
- CheckpointDigestV6 and EnvironmentCheckpointV6 KATs, validation matrix,
  tamper/unknown-variant negatives, status/counter/identity binding.
- V6 restore and fork at APNAP stage zero and after one accepted owner order.
- Replay V6 structural and backend parity: each APNAP Order response is one
  real step; no forced progress input exists.
- Replay V5 remains detached-readable/verifiable and unsupported for execution
  under the V6 runtime; no V5 artifact is reinterpreted.
- Rust/Python/schema/golden/negative parity, generator/drift checks, repo and
  archive reproducibility.
- Restore admission rejects incompatible semantic contract identity before
  state mutation or projection.

These prove identity/persistence, not Magic correctness, support, coverage or
certification.

## Required conclusions

```text
NEW_AUTHORITATIVE_MAGIC_CONTINUATION_STATE = YES
FULL_STATE_DIGEST_V4_CAN_BE_EXTENDED_IN_PLACE = NO
FULL_STATE_DIGEST_V5_REQUIRED = YES
ENVIRONMENT_CHECKPOINT_V6_REQUIRED = YES
CHECKPOINT_DIGEST_V6_REQUIRED = YES
REPLAY_V6_REQUIRED = YES
OBSERVATION_ENVELOPE_V2_REQUIRED = NO
INFORMATION_STATE_DIGEST_V3_REQUIRED = NO, unless separate retained-information review finds a meaning change
MAGIC_OBSERVATION_PAYLOAD_CODEC = magic-m3-observation.v1
V5_ARTIFACTS_REINTERPRETED = NO
AUTOMATIC_V5_TO_V6_MIGRATION = NONE
S3_P0_REQUIRED = YES
S3_IMPLEMENTATION_AUTHORIZED = NO
```
