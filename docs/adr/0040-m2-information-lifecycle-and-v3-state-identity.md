# ADR 0040: M2 information lifecycle, perspective-local visible identity, and V3 state identity

- **Status:** proposed
- **Date:** 2026-08-21
- **Supersedes:** none; existing information/identity principles remain
- **Unblocks:** M2 V3 structural migration after the byte-level state-digest specification in `STATE_HASHING.md` is accepted

## Context

Manafold already separates full authoritative state, current player observation and retained player information. M1 also checkpoints per-player opaque mappings and a minimal knowledge ledger.

The executable M1 representation is intentionally too narrow for M2:

- active knowledge is keyed by the current live `GameObjectId`;
- known location validation expects the current authoritative location;
- public/private history use separate counters;
- player-visible opaque allocators live alongside global internal allocators;
- the same internal `DecisionId` is currently visible to the player;
- general observed-event projection is not yet executed;
- player error documentation still lists wrong-actor/binding conditions as ordinary player codes;
- `FullStateDigestV2` directly includes current execution, knowledge and perspective-identity runtime structures.

M2 must prove retained knowledge across incarnation change, loss of distinguishability after randomization, byte-level noninterference, safe events/errors, and deterministic replay without creating hidden projector/controller state.

## Decision

### One perspective-local visible sequence

Each perspective has one total `VisibleSequence`.

Public/private is a property of the observed event or knowledge provenance, not a separate observable counter.

Hidden events emit nothing and advance no sequence for that perspective. No global authoritative event count becomes player metadata.

### Active knowledge is associated with perspective-visible identity

Active retained object knowledge is keyed by `OpaqueObjectId`.

Trusted knowledge records may contain:

- physical-card identity;
- definition identity;
- current known location;
- historical location facts;
- typed provenance.

`PerspectiveIdentityState` is the **sole authoritative owner** of the current `OpaqueObjectId -> GameObjectId` association. Knowledge does not duplicate that live relation; validation/projection resolves the current incarnation through the perspective mapping. Historical incarnation/location facts may remain explicit knowledge.

Player projection contains only authorized opaque/synthetic/public values.

Information state remains:

```text
current observation + retained perspective knowledge
```

It does not absorb `EpisodeStatus`. Technical truncation/environment outcome remains a separate PlayerStep/environment value and is excluded from information-state digest identity.

### Opaque identity follows distinguishability

For:

```text
visible
→ hidden but distinguishable
→ new authoritative incarnation
→ visible
```

the same perspective-local opaque ID is remapped to the new current object. Historical location knowledge is retained and the opaque allocator does not advance merely because the authoritative `GameObjectId` changed.

For:

```text
visible
→ hidden
→ randomized/shuffled into an indistinguishable set
→ visible
```

the old mapping is removed, old opaque identity is retired and never reused, relevant knowledge is invalidated/retired, and later visibility allocates the next perspective-local ID.

### Perspective-local visible allocators

Perspective identity state owns:

- next opaque object ID;
- next opaque ability ID;
- next player-decision ID;
- retired visible IDs.

These values are part of checkpointed `EngineState`.

They do not derive from global internal object/decision/event allocator history.

### Projection is read-only

Identity/knowledge/visible-sequence mutation occurs only in accepted authoritative reset/transition workspaces.

Observation/information/decision/event projection cannot allocate, mutate state, consume RNG, advance sequence, change counters or append replay.

A candidate transition and every perspective projection required for that commit are validated before atomic commit.

### Authoritative and observed events remain separate

Rules own authoritative event semantics and trusted audience policy.

Observation projection owns redaction and opaque-ID substitution.

The environment validates the per-perspective observed event/PlayerStep product before commit.

Observed events contain no authoritative event ID and no hidden RNG provenance.

### Error layers

The public boundary has three distinct layers:

1. malformed/noncanonical wire bytes → closed `malformed_response` wire error, no typed semantic response and no PlayerStep;
2. typed semantic rejection → closed submission code with complete nonmutation;
3. invariant/internal failure → closed `service_unavailable`, trusted detail only.

A perspective-bound endpoint does not expose `wrong_actor` as a distinct oracle. Internal `DecisionId` is not a V2 player value. Candidate-binding mismatch is an implementation/invariant failure rather than normal player illegality.

### Coordinated V3 semantic identity cut

M2 changes authoritative execution, knowledge and perspective-identity meaning.

Therefore introduce:

```text
FullStateDigestV3
EnvironmentCheckpointV3 / CheckpointDigestV3
ReplayManifestV3 / ReplayStepV3 / AuthoritativeReplayV3
```

alongside Decision/Information/Event/Step V2 public contracts and the explicit `InformationStateDigestV2` identity `mtgml.information-state-digest.v2` / `information-state-digest-input.v2`.

V3 full-state/checkpoint digests are the first new persisted semantic identities after ADR 0038 and use its common envelope plus `mtgml.canonical-cbor.v1` detached input specified normatively in `STATE_HASHING.md`.

Replay V3 binds complete environment identity, not only game-state identity: its initial record contains `FullStateDigestV3`, `EpisodeStatus`, `EnvironmentLimitCounters`, checkpoint codec identity and `CheckpointDigestV3`, and step continuity verifies the corresponding after checkpoint identity. Environment values that are not deterministic functions of game state/responses (for example recorded wall-clock progression) are explicit trusted replay-control data and are never reconstructed from the replay host's wall clock.

### V2 runtime retirement

`EnvironmentCheckpointV2` embeds the then-current unversioned `EngineState`. After M2 changes `EngineState`, that runtime type cannot retain historical V2 executable meaning.

Manafold therefore:

- does not reinterpret V2;
- does not create a permanent legacy `EngineStateV2` only for compatibility;
- stops producing V2 full-state/checkpoint identity from the current engine;
- preserves historical schemas/fixtures/domain evidence;
- classifies `FullStateDigestV2` evidence and Replay V2 as `READABLE_VERIFIABLE_ONLY` in the current engine;
- classifies `EnvironmentCheckpointV2` as `UNSUPPORTED` by the current engine after the state cut because it embeds the unversioned runtime `EngineState` and has no detached durable state codec;
- defines no V2→V3 migration in M2.A;
- uses archived matching M1 engine builds where historical semantic execution is required.

Migration, if later required, is Rust-authoritative, versioned and provenance-preserving.

### Persistence decode precedence refinement

For M2 V3 persisted identities, the grouped precedence described by `STATE_HASHING.md` is refined to this total `PersistenceDecodeErrorV1` order whenever an input has multiple detectable defects:

```text
1  unsupported_historical_version
2  envelope_identity
3  envelope_length
4  payload_too_large
5  string_too_large
6  array_too_large
7  depth_exceeded
8  item_limit_exceeded
9  disallowed_cbor_form
10 noncanonical_primitive
11 invalid_utf8
12 wrong_record_length
13 unknown_variant
14 value_out_of_range
15 duplicate_semantic_key
16 noncanonical_order
17 schema_identity_mismatch
18 trailing_data
19 reencode_mismatch
20 digest_mismatch
21 semantic_validation
```

Negative fixtures with multiple defects assert the first applicable category in this order. Implementations must not choose an arbitrary error from the same conceptual group. This refinement does not make persistence errors player-facing.

## Compatibility

This is a semantic break for current full-state/checkpoint/replay identity and for information/event/player-step public values.

M2 uses new versions rather than changing V1/V2 meaning in place.

`ObservationEnvelopeV1` may remain because it already binds an independently versioned payload codec; M2 uses a new `synthetic-m2-observation.v1` payload.

OD-009 production Python/native transport and OD-011 semantic action keys remain open.

## Consequences

Positive:

- retained knowledge survives exactly when authorized;
- opaque identity semantics are explicit across incarnation/randomization;
- projection/query order cannot mutate identity;
- global hidden history cannot create player-visible ID gaps;
- observed event count/order cannot leak hidden events;
- error classes no longer reveal private request/binding state;
- checkpoint/fork/replay close over all player-information state;
- V2 history remains honest instead of being made executable by reinterpretation.

Costs:

- one coordinated cross-layer V3 migration;
- stronger validation and negative fixtures;
- perspective-local allocator/retirement state;
- explicit historical support classifications;
- paired-state byte proof over more surfaces.

## Rejected alternatives

- key all retained knowledge only by current `GameObjectId`;
- reconstruct knowledge from current public zones;
- invalidate every identity merely because it becomes hidden;
- preserve identity through a shuffle/randomization that destroys distinguishability;
- reuse retired opaque IDs;
- allocate opaque IDs during projection;
- derive visible IDs from global allocator counters;
- expose global rule-event sequence;
- put `EpisodeStatus` inside information state/digest;
- synthesize a semantic PlayerStep for undecodable wire bytes;
- expose wrong actor/binding mismatch as detailed player errors;
- reinterpret V2 state/checkpoint/replay against new `EngineState`;
- maintain a duplicate legacy rules/state engine solely for historical execution.

## Evidence required

This ADR does not mark M2 gates `PASS`.

M2 executable evidence must cover:

- tracked hidden identity persistence;
- explicit forget and randomization retirement/new identity;
- retained current/historical knowledge/provenance without duplicating the live opaque→object association;
- public/private/mixed/hidden observed events;
- perspective-local sequence continuity;
- query purity;
- complete typed rejection nonmutation;
- paired-state byte equality over all declared hidden axes;
- two-endpoint isolation;
- checkpoint/fork/replay V3 parity;
- Rust/Python public DTO parity;
- rules-free Python adapter boundary;
- immutable V1/V2 historical evidence.

## Review trigger

Revisit if real M3 rules require simultaneously pending private decisions, more complex distinguishability semantics, player-visible event fan-out greater than one per authoritative event, or a different persisted semantic codec/identity architecture.
