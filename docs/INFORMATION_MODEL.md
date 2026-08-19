# Information Model

**Status:** accepted  
**Stability:** normative

## Three distinct concepts

- `EngineState`: complete authoritative state.
- `PlayerObservation`: what one perspective may perceive now.
- `PlayerInformationState`: current observation plus checkpointable retained knowledge from that perspective’s visible history.

Information state cannot be reconstructed solely from current public zones.

## Knowledge facts

Knowledge records may represent:

- known card definition/face;
- known current or historical zone/location;
- known relative/absolute library position;
- known association between an opaque identity and an object incarnation;
- source of knowledge (public, reveal, private look, rules-preserved inference);
- visible sequence at which knowledge was learned;
- explicit invalidation reason and sequence.

M0.2 fixes ownership, not the final optimized encoding.

## Opaque identity

Each perspective has deterministic bidirectional mappings for visible object/ability identities. Mapping state is checkpointed. It may persist only while the rules/visibility contract preserves distinguishability. Shuffle or hidden-zone randomization can invalidate or replace identities.

Authoritative IDs never cross the player boundary, even for public objects.

## Projection

Projection is deterministic from explicit state, perspective, and declared schema. It cannot query hidden mutable history or global counters. Candidate ordering, event ordering, errors, payload sizes mandated by schemas, and semantic keys are reviewed for side channels.

## Observed events

Authoritative events may include internal IDs and full RNG provenance. Observed events contain only opaque/public identities and perspective-local sequence. A visible random result may be exposed when rules make it visible; root seed, typed stream key, derived stream key, stream cursor, and raw words remain trusted and never cross the player boundary.

## Noninterference

Paired states differing only in unauthorized information must produce identical bytes for that perspective’s observation, information state, visible decision/candidates, observed events, errors, and semantic trajectory fields. See [`testing/NONINTERFERENCE_TESTING.md`](testing/NONINTERFERENCE_TESTING.md).

## V0.2.1 knowledge provenance records

Each retained known-object fact carries:

```text
object incarnation
optional physical-card identity
optional card-definition identity
optional known location
learned_at { public|private, visible sequence }
learned_via { initial configuration | public event | private event |
              own-zone identity | explicit reveal }
```

Invalidation is a typed per-object record:

```text
object
invalidated_at { public|private, visible sequence }
reason { shuffle | hidden-zone transition | randomization | explicit forget }
```

Validation rejects facts outside the perspective's visible history, future authoritative event references, mismatched opaque mappings, and known identities/locations inconsistent with the current live incarnation. Future rules may extend the reason enums, but cannot fall back to unscoped strings or a global invalidation-reason list.
