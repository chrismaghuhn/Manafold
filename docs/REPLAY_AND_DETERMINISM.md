# Replay and Determinism

**Status:** provisional-public replay identity; semantic execution pending M1

## Replay identity

### V1 replay (historical)

Historical replay manifests used placeholder RNG fields: RNG algorithm string, derivation version string, named stream names, and u64 counters. V1 manifests exist in fixtures and migration reference material but are not produced by the current engine.

### V2 replay (current)

A V2 replay manifest (`ReplayManifestV2` / `AuthoritativeReplayV2`) identifies:

- engine build and kernel/backend identity;
- rules, format-policy, Oracle/source, and bundle snapshots;
- fixed schema/canonicalization versions;
- RNG contract ID (`mtgml.rng.v1`), root seed material (trusted replay only), and typed stream keys with cursor semantics;
- exact deck identities;
- initial state revision and full digest.

## Step rules

- accepted steps advance revision strictly;
- rejected diagnostic steps preserve revision and full state identity;
- adjacent revisions are contiguous;
- response expected revision equals the step’s before revision;
- final revision/digest equal the final step or, for empty replay, the initial identity.

## Deterministic sources

Authoritative behavior cannot depend on wall clock, thread scheduling, randomized container iteration, locale, filesystem order, network responses, or process-global RNG. Every random use consumes one typed checkpointable stream (`RandomStreamKeyV1`) with explicit cursor progression (`RandomStreamCursorV1::next_raw_u64`).

## Checkpoint and fork

A checkpoint contains every semantic state component and declared codec identity. Restore validates before use. Forks begin with identical state/digests and diverge only through explicit subsequent inputs/random stream usage.

## Canonical writing

Replay writers are fallible and emit canonical bytes. Readers validate schema, canonical form, semantic invariants, supported versions, and content identity before constructing domain values.

## Dataset relationship

Published player trajectories derive from player-safe endpoints, not authoritative replays. The authoritative replay may reproduce/verify them in trusted tooling but is not exposed to the model process.

## Source-artifact reproducibility

The deterministic source-archive timestamp, root prefix, and compression policy have one owner: `config/reproducibility.toml`. Build, verification, CI, and release tooling consume that file; duplicating literal defaults in commands or documentation is prohibited. `SOURCE_DATE_EPOCH` may override the timestamp for an intentional rebuild, but the override becomes part of the recorded build evidence.

## V0.2.1 checkpoint/replay boundary

`FullStateDigest` (V1, historical) identifies authoritative game state only. `EnvironmentCheckpointV1` (V1, historical) additionally captures episode status and environment-limit counters but embeds an unversioned state representation. `FullStateDigestV2` and `EnvironmentCheckpointV2` are the current identity/checkpoint contracts, binding the typed RNG state representation explicitly. Replays record accepted/rejected decision history and state identities; a replay implementation that resumes execution must restore equivalent checkpoint semantics rather than reconstructing hidden controller counters heuristically.

Generated verification logs and reports are not replay or source inputs. They live outside the deterministic source archive.
