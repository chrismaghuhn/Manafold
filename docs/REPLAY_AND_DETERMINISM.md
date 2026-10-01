# Replay and Determinism

**Status:** Replay V8 is the only replay format; earlier replay versions were removed (docs/superpowers/specs/2026-09-30-old-formats-cleanup-design.md)
**Stability:** provisional-public replay identity; changed in place (AGENTS.md §4)

## Replay identity

### Replay V8

Replay V8 binds `FullStateDigest`, `CheckpointDigestV8`,
`InitialEnvironmentIdentityV8`, request V4, `DecisionResponseV3`,
ObservedEvent V4, PlayerStep V4, and
`magic-shared-execution-observation.v1`. It re-executes response records through
the single current V8 environment and never uses observations or observed
events as commands. The manifest RNG root seed must equal the seed in the
admitted initial state. A replay cannot continue after a terminal or truncated
episode status.

The recorder validates each appended step against its validated prefix's final
identity. Export revalidates the complete replay at the artifact boundary.
G0j parity covers only the preserved M4.2 basic-land slice and does not admit
broader card or deck support.

## Detached and backend-verified replay

`AuthoritativeReplayV8::validate()` is detached structural validation: schema
and DTO shape, manifest and deck consistency, the identity chain, and the
revision and counter rules the replay DTO encodes. It does not execute a
response or prove that a backend reaches the recorded state.

`TrustedEnvironmentController::execute_replay()` validates the detached
artifact and then re-executes it through the V8 runtime. Its
`BasicLandReplayV8ExecutionReport` is the backend-verified evidence of
transition, counter and identity parity. The report and its privileged fields
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

## Dataset relationship

Published player trajectories derive from player-safe endpoints, not authoritative replays. Trusted replay may reproduce/verify them but never crosses into the model process.

Request-local player decision/candidate IDs are not dataset labels. OD-011 remains open.

## Source-artifact reproducibility

Generated verification logs and reports are not replay/source inputs. They live outside the deterministic source archive. The final source/archive gate runs after source-changing operations.

## Evidence boundary

Replay evidence is a V8 replay recorded and re-executed by the current engine.
