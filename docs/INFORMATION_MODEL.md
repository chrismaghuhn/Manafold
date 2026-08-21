# Information Model

**Status:** accepted M2 information-safety contract freeze candidate  
**Stability:** normative

## Three distinct concepts

- `EngineState`: complete authoritative semantic state.
- `PlayerObservation`: what one perspective may perceive now.
- `PlayerInformationState`: current observation plus checkpointable retained knowledge from that perspective's authorized visible history.

Information state cannot be reconstructed solely from current public zones.

`EpisodeStatus` is not part of `PlayerInformationState`; it remains environment/step semantics. In particular, technical truncation must not become part of `InformationStateDigest`.

## Perspective-local visible history

M2 uses one total `VisibleSequence` per perspective. Public/private classification belongs to each observed event or knowledge provenance record.

There is no player-visible global event count and no independent public/private history counter whose gaps could reveal hidden events.

The authoritative state stores the next unused sequence for each perspective.

## Knowledge facts

M2 retained knowledge explicitly represents:

- known definition/face or synthetic kind when authorized;
- optional physical-card identity internally;
- current live object association internally;
- known current location when authorized;
- ordered historical location facts;
- acquisition/update provenance;
- explicit invalidation/retirement reason and visible sequence.

Active known-object records are keyed by that perspective's `OpaqueObjectId`, not by the current `GameObjectId`.

Trusted records may retain internal object/physical/definition identities for validation. Player projection never exposes them.

## Knowledge lifecycle

Required synthetic lifecycle cases include:

```text
public knowledge
own-private knowledge
private look
public reveal
tracked hidden transition
known-location update/history
explicit forget
hidden randomization
```

A tracked zone/incarnation transition may replace the authoritative `GameObjectId` while preserving the same opaque identity when distinguishability is preserved.

If the destination becomes unknown, the previous current location moves into history and the current-known location becomes absent.

Invalidation is typed. M2 reasons include:

```text
hidden_transition
randomization
shuffle
explicit_forget
```

A future Magic capability may add versioned typed reasons. No free-form invalidation string becomes semantic state.

## Opaque identity

Each perspective has deterministic bidirectional mappings for visible/distinguishable object and ability identities.

Perspective state also owns its player-visible allocators:

- next opaque object ID;
- next opaque ability ID;
- next player-decision ID;
- retired opaque identities.

These counters do not live in the global internal allocator because player-visible gaps must not depend on hidden allocation history.

Authoritative IDs never cross the player boundary, even for public objects.

### Persistence while distinguishable

```text
visible
→ hidden but still distinguishable
→ new authoritative incarnation
→ revealed
```

The same perspective-local opaque ID persists and is remapped to the new live incarnation. The opaque allocator does not advance solely because the authoritative object ID changed.

### Invalidation after indistinguishability

```text
visible
→ hidden
→ randomized/shuffled into an indistinguishable set
→ later revealed
```

The old mapping is removed, the old opaque identity is retired and never reused, retained knowledge is invalidated/retired, and the later visible object receives the next deterministic perspective-local opaque identity.

## Allocation rule

Opaque/player-decision IDs are allocated only by accepted authoritative reset/transition logic when new player-visible distinguishability/request identity is created.

Projection is strictly read-only. Calls to `observation()`, `information_state()`, and `visible_decision()` cannot allocate, mutate knowledge, advance visible sequence, or alter any digest.

## Projection

Projection is deterministic from explicit state, perspective, and declared schema. It cannot query hidden mutable history or global counters.

Canonical ordering, candidate count/order, opaque IDs, event order/count, errors, optional fields, payload sizes, digests, and protocol metadata are all information-safety surfaces.

## Observed events

Authoritative events may include internal IDs and full RNG provenance. `ObservedEventV2` contains only perspective-authorized public/opaque values plus the perspective-local `VisibleSequence`.

Authoritative event families declare trusted audience semantics such as public, private-to-player, selected-player set, hidden, or mixed field policy. Rules own audience semantics; observation code owns redaction/opaque projection; environment validates the complete per-perspective product before commit.

For each perspective, visible events are assigned contiguous sequence values. Hidden events emit nothing and advance no sequence for that perspective.

A visible random result may differ when the synthetic/rules visibility contract authorizes it. Root seed, typed stream key, derived key, stream cursor, raw words, rejection count, and hidden permutation remain trusted.

## Noninterference

For perspective `P`, two valid authoritative states that differ only in unauthorized information must produce byte-identical:

- observation;
- information state;
- visible decision and candidate IDs/order;
- observed events and sequence;
- typed semantic rejection;
- wire/endpoint error class where applicable;
- `PlayerStep`;
- player-facing schema/protocol metadata.

Required difference axes include opponent hidden identity/order, another player's private knowledge, hidden face-down identity, trusted object-ID renaming, root seed, hidden RNG cursor, and global allocator history.

Wall-clock timing is outside the semantic byte guarantee, but timing never enters semantic DTOs or ordering/identity decisions.

## Validation

`validate_engine_state()` rejects at least:

- missing/mismatched perspective state;
- non-bijective active mapping;
- active use of a retired opaque ID;
- opaque/player-decision allocator not strictly ahead of issued IDs;
- active knowledge without the required live mapping;
- active mapping inconsistent with trusted current-object association;
- retired knowledge still active;
- current known location contradicting an authorized live association;
- future or non-monotonic visible-sequence provenance;
- malformed historical location sequence;
- invalid knowledge cause/channel combination.

Checkpoint, restore, fork, replay and digest include all authoritative knowledge, mapping, allocator, retirement, and visible-sequence state needed to continue equivalent execution.
