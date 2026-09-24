# Replay and Determinism

**Status:** Replay V6 current resumable identity; V1–V5 meanings retained historically
**Stability:** provisional-public replay identity; historical versions never reinterpreted

## Replay identity

### V1 replay

V1 is historical migration/reference evidence from the placeholder RNG world. It is not produced by the current engine.

### V2 replay

M1 Replay V2 is the executable replay contract proven by M1 closure. It binds typed `mtgml.rng.v1` root-seed identity, V2 full-state digests, current player schema identities, exact deck identities, accepted/rejected diagnostic step rules, and final revision/digest identity.

M2 changes authoritative state and player decision response meaning. Replay V2 therefore does not become an M2 semantic replay by reading it with the new runtime.

### V3 replay

M2 introduces:

```text
ReplayManifestV3
ReplayStepV3
AuthoritativeReplayV3
```

A V3 manifest identifies:

- engine build and kernel/backend identity;
- rules, format-policy, Oracle/source, bundle and deck identities already required by replay provenance;
- RNG contract ID and trusted root seed;
- player decision request/response schema versions;
- observation payload/envelope identity;
- information-state, observed-event and PlayerStep schema identities;
- full-state and checkpoint digest envelope/algorithm/domain/codec/input-schema identities;
- replay-step/file schema identities;
- one complete `InitialEnvironmentIdentityV3`.

`InitialEnvironmentIdentityV3` contains exactly:

```text
state_revision
FullStateDigestV3
EpisodeStatus
EnvironmentLimitCounters
CheckpointCodecIdentity
CheckpointDigestV3
```

The checkpoint digest must recompute from the other fields using the V3 checkpoint-digest contract. A replay cannot identify its initial environment by `FullStateDigestV3` alone.

M2 does not resolve stable semantic action keys or trajectory encoding.

### V5 replay (historical)

ADR 0055 introduced Replay V5 and it retains that exact historical meaning.
`ReplayManifestV5` carries the full `ExecutionIdentityV1`, semantic contract
material, and kernel/rules provenance. Its initial/step identities are typed
against `FullStateDigestV4` and `CheckpointDigestV5`.

Detached `AuthoritativeReplayV5::validate()` verifies the three-way identity
binding and the semantic/rules contract hashes. V5 is no longer a current
writer or current-runtime execution path after S3.P0.

### V6 replay (current)

S3.P0 introduces `ReplayManifestV6`, `ReplayStepV6`,
`AuthoritativeReplayV6`, `ReplayRecorderV6`, `ReplaySchemaVersionsV6`, and
`InitialEnvironmentIdentityV6`. V6 binds the complete `ExecutionIdentityV1`
and uses `FullStateDigestV5` / `CheckpointDigestV6` in initial, per-step and
final identities.

Each `ReplayStepV6` contains exactly one real `DecisionResponseV2`. Forced
progress has no replay input; deterministic consequences belong to the real
response that caused them. The shared response transaction may commit the
response product and at most one forced-progress product together. Under the
exact S3.B pass-only contract, one accepted replay step may therefore advance
one or two Rules revisions; other current profiles retain their existing
revision progression. The S3.B replay revision bound is `+2`; it does not
authorize a forced-progress loop. No event-as-input, implicit pass, or
synthetic response is introduced.

The V6 manifest binds the observation payload codec. Existing producers retain
`synthetic-m3-observation.v1`; `magic-m3-observation.v1` is admitted only for a
semantic rules closure containing the selected SBA capability. The observation
envelope and information-state digest identities remain unchanged.

`FullStateDigestV4`, Checkpoint V5 and Replay V5 retain their exact historical
meaning and bytes. V4 digest and V5 checkpoint/replay artifacts are never
reinterpreted as V5 state or V6 replay. V5 artifacts remain detached
`READABLE_VERIFIABLE_ONLY` evidence in the current runtime; executing V5
semantics requires its archived matching runtime. No automatic V5→V6
migration exists.

## ReplayStepV3

A step contains:

```text
step_index
actor
CheckpointDigestV3 before
state_revision_before
DecisionResponseV2
accepted
state_revision_after
FullStateDigestV3 after
EpisodeStatus after
EnvironmentLimitCounters after
CheckpointDigestV3 after
```

The checkpoint codec identity is fixed by the manifest/initial environment identity for the replay segment. The actor is trusted replay input; it is not taken from the player response.

Rules:

- `checkpoint_digest_before` equals the initial checkpoint digest for step 0 and the previous step's `checkpoint_digest_after` thereafter;
- accepted steps advance revision strictly and produce a V3 checkpoint digest that recomputes from after-state digest, after status, after counters and checkpoint codec identity;
- rejected diagnostic steps preserve the **complete checkpoint identity**, including status and every environment counter, not merely state revision/full-state digest;
- adjacent accepted semantic revisions are contiguous under the execution contract;
- response expected revision equals the step before revision;
- response player-decision identity must match the authoritative request reconstructed at that point;
- final replay identity includes final revision, final `FullStateDigestV3`, final `EpisodeStatus`, final `EnvironmentLimitCounters`, and final `CheckpointDigestV3`; for an empty replay these equal the initial environment identity.

Counters whose values are deterministic consequences of submitted decisions/events/resources are recomputed and compared. A replay executor MUST NOT sample the host wall clock to reconstruct `wall_clock_elapsed_millis`; recorded wall-clock/external-limit progression is trusted environment-control trace and is replayed/validated explicitly. The same rule applies to any future environment value that is not a deterministic function of authoritative game state plus submitted responses: it must become explicit versioned replay input rather than hidden controller state.

Wire-decode failures are not semantic replay steps because no typed `DecisionResponseV2` exists.

## Detached and backend-verified replay

`AuthoritativeReplayV3::validate()` is detached structural validation. It
proves the V3 schema and local DTO shape, canonical keyed-array order,
manifest/deck consistency, replay identity-chain shape, and the revision and
counter rules encoded by the replay DTO. It does not execute a response,
reconstruct an authoritative pending request, or prove that a backend reaches
the recorded state, digest, status, or player projection.

`TrustedEnvironmentController::execute_replay_from_checkpoint()` first
validates the detached artifact, then executes it on an internal backend fork
from a validated checkpoint. Its `ReplayExecutionReport` is the
backend/checkpoint-verified evidence: it proves actor and player-decision
binding, authoritative transition parity, deterministic counter parity,
recorded external-counter application, complete after identity, and the
available production projection trace. The report and its privileged fields
remain outside player endpoints.

## Deterministic sources

Authoritative behavior cannot depend on wall clock, thread scheduling, randomized container iteration, locale, filesystem order, network responses, or process-global RNG.

Every random use consumes a typed checkpointable `mtgml.rng.v1` stream with explicit cursor progression. Rejected semantic responses and wire failures consume no randomness.

## Checkpoint and fork

A checkpoint contains every semantic state component and declared complete environment identity. Restore validates before backend mutation. Forks begin with identical semantic state/digests and diverge only through explicit later inputs/random stream use.

M2 newly authoritative continuation, knowledge, perspective identity/allocators, retirement sets and visible-sequence state are part of checkpoint/fork parity.

## Player projection parity

Authoritative replay does not persist player observation/information/event batches as a second authority.

M2 replay parity re-executes the authoritative input segment and deterministically reprojects:

- observation;
- retained information state;
- visible decision/candidates;
- observed events;
- PlayerStep;
- closed semantic rejection/error behavior.

Exact player bytes must match the live run.

## Canonical writing

Replay writers are fallible and emit the declared canonical replay wire. Readers validate schema, canonical form, semantic invariants, supported versions, and content identity before constructing trusted replay values.

The authoritative replay container remains canonical JSON unless/until a separate replay-container ADR changes it. Embedded state digest identities use their own ADR-0038 envelope/CBOR contract; replay JSON does not redefine their preimage.

## Historical support policy

When M2 changes `EngineState`:

- do not reinterpret `FullStateDigestV2`;
- do not keep `EnvironmentCheckpointV2` executable by introducing a legacy duplicate `EngineState`;
- preserve V2 replay schemas/fixtures exactly;
- current-engine support is fixed: `FullStateDigestV2` evidence and Replay V2 are `READABLE_VERIFIABLE_ONLY`; `EnvironmentCheckpointV2` is `UNSUPPORTED` by the current engine after the state cut because it embeds unversioned runtime `EngineState`;
- no V2→V3 migration is defined in M2.A;
- structural readability does not imply current-engine semantic execution;
- an archived matching engine build may be required to execute historical V2 semantics.

Any migration verifies the source under its original contract, converts through Rust-authoritative versioned migration, validates the target, writes a new artifact with new identity/provenance, and never overwrites the source.

## Dataset relationship

Published player trajectories derive from player-safe endpoints, not authoritative replays. Trusted replay may reproduce/verify them but never crosses into the model process.

Request-local player decision/candidate IDs are not dataset labels. OD-011 remains open.

## Source-artifact reproducibility

Generated verification logs and reports are not replay/source inputs. They live outside the deterministic source archive. The final source/archive gate runs after source-changing operations.

## Evidence boundary

M1 Replay V2 evidence remains historical evidence for M1. No V3 replay/checkpoint/information gate becomes `PASS` because these contracts are written. M2 implementation and M2.Final must execute their exact evidence on one final source identity.
