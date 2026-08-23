# Information Model

**Status:** accepted M2 information-safety architecture contract; executable M2 evidence `NOT_RUN`  
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

**Occurrence binding (M2.E).** One perspective-visible occurrence consumes
exactly the next visible sequence of its perspective; every observed
provenance created or updated by that same occurrence references exactly this
value, and provenance never allocates a sequence independently. Hidden
occurrences consume nothing and create no values, so no consumed number is
ever attributable to a hidden event. A checkpoint may legitimately retain only
a subset of past numbers as provenance: earlier numbers may belong to
occurrences whose events were observed but not retained as knowledge.

## Knowledge facts

M2 retained knowledge explicitly represents:

- known definition/face or synthetic kind when authorized;
- optional physical-card identity internally;
- known current location when authorized;
- ordered historical location facts;
- acquisition/update provenance;
- explicit invalidation/retirement reason and visible sequence.

Active known-object records are keyed by that perspective's `OpaqueObjectId`, not by the current `GameObjectId`.

Trusted knowledge records may retain physical/definition identities for validation, but they do **not** duplicate the live `OpaqueObjectId -> GameObjectId` association. `PerspectiveIdentityState` is the sole owner of that current live association. Player projection never exposes physical-card identity or live authoritative object identity. A known `CardDefinitionId` may be projected only when that perspective is already authorized to know the definition.

## PlayerInformationStateV2 retained-knowledge contract

`PlayerInformationStateV2` contains exactly one current observation plus retained perspective knowledge. Its retained object array is `PlayerKnownObjectV1[]`; retired records remain present because retirement ends live identity resolution, not the player's historical memory.

`PlayerKnownObjectV1` uses the following exact canonical-JSON semantic shapes. Object keys are serialized by the global canonical JSON rule; every field shown is required, and nullable fields are encoded as explicit `null` rather than omission. IDs use their canonical decimal-string wire representation.

Active record:

```json
{
  "kind": "active",
  "opaque_object_id": "7",
  "known_definition": "42",
  "current_known_location_fact": {
    "location": {"zone": "exile", "player": "2"},
    "provenance": {"kind": "observed", "channel": "public", "sequence": "4", "cause": "public_event"}
  },
  "historical_locations": [],
  "acquisition": {"kind": "initial_configuration"}
}
```

`known_definition` and `current_known_location_fact` may be `null`. Current-location update provenance is deliberately part of `PlayerInformationStateV2`; it is not dropped merely because the fact is current rather than historical.

Retired record:

```json
{
  "kind": "retired",
  "opaque_object_id": "7",
  "known_definition": null,
  "last_known_location_fact": {
    "location": {"zone": "library", "player": "2"},
    "provenance": {"kind": "observed", "channel": "private", "sequence": "3", "cause": "private_look"}
  },
  "historical_locations": [],
  "acquisition": {"kind": "observed", "channel": "private", "sequence": "3", "cause": "private_look"},
  "invalidation": {
    "provenance": {"kind": "observed", "channel": "public", "sequence": "5", "cause": "public_event"},
    "reason": "shuffle"
  }
}
```

`known_definition` and `last_known_location_fact` may be `null`; `invalidation` is mandatory for retired records. Last-known-location provenance remains player-visible retained information when the location fact itself is retained.

The public-safe `PlayerKnownLocationV1` shape is exactly:

```json
{"zone": "<ZoneKind>", "player": "<PlayerId>"}
```

where `player` may be `null`. It never exposes hidden zone position, partition identity, internal ordering, or a trusted `ZoneLocation`. M3 may version this public location surface if a certified capability requires additional authorized location knowledge.

A `PlayerKnownLocationFactV1` is exactly:

```json
{
  "location": {"zone": "<ZoneKind>", "player": null},
  "provenance": {"kind": "initial_configuration"}
}
```

`PlayerKnowledgeProvenanceV1` has exactly two JSON variants:

```json
{"kind": "initial_configuration"}
```

and:

```json
{
  "kind": "observed",
  "channel": "public",
  "sequence": "4",
  "cause": "explicit_reveal"
}
```

`channel` is exactly `public | private`; `cause` is exactly `public_event | private_look | explicit_reveal | own_private_identity`. `VisibleSequence` uses canonical decimal-string wire representation.

`PlayerKnowledgeInvalidationV1` is exactly:

```json
{
  "provenance": {"kind": "observed", "channel": "public", "sequence": "5", "cause": "public_event"},
  "reason": "hidden_transition"
}
```

`reason` is exactly `hidden_transition | randomization | shuffle | explicit_forget`.

Canonical retained-knowledge order is ascending numeric `OpaqueObjectId` across active and retired records. One opaque ID may occur at most once. Historical-location arrays preserve semantic history order and their visible sequences must be strictly increasing. Active records have no invalidation. Retired records have exactly one invalidation and no active mapping in `PerspectiveIdentityState`.

The public DTO excludes `PhysicalCardId`, `GameObjectId`, trusted `ZoneLocation`, authoritative event IDs, RNG provenance, and another perspective's knowledge.

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
- active knowledge without exactly one required live mapping in `PerspectiveIdentityState`;
- any knowledge record that attempts to carry a second authoritative live `GameObjectId` association;
- retired knowledge still active;
- current known location contradicting an authorized live association;
- future or non-monotonic visible-sequence provenance;
- malformed historical location sequence;
- invalid knowledge cause/channel combination.

Checkpoint, restore, fork, replay and digest include all authoritative knowledge, mapping, allocator, retirement, and visible-sequence state needed to continue equivalent execution.
