# Wire Contract

**Status:** provisional public wire contract; M2 V2 shapes are freeze candidates  
**Stability:** normative

## Public wire codec

Player/Python/replay public wire remains canonical compact UTF-8 JSON with:

- lexicographically sorted object keys;
- no insignificant whitespace;
- canonical decimal identifier strings where the JSON contract uses textual IDs;
- canonical lowercase SHA-256 hex where a wire field renders a digest;
- canonical padded standard Base64 for declared byte payloads;
- duplicate-key rejection;
- closed variants/unknown-field rejection.

The ADR-0038 persisted semantic digest payload is **not** this JSON codec. V3 full-state/checkpoint digest preimages use the separate `mtgml.canonical-cbor.v1`/envelope contract in [`../STATE_HASHING.md`](../STATE_HASHING.md).

## Validation layers

1. JSON Schema: closed shape and obvious scalar bounds.
2. Decoder/encoder: closed variants, canonical scalar encoding, language ranges, duplicate handling, canonical bytes.
3. Semantic validation: cross-field invariants such as bounds, candidate uniqueness/canonical set order, event sequence/perspective, revision coherence, replay continuity and PlayerStep consistency.

Rust and Python use the shared fixtures under `wire/`.

A public contract change is not mergeable unless every applicable representation changes coherently:

- Rust DTO/codec;
- Python DTO/codec;
- JSON Schema;
- positive golden fixtures;
- negative fixtures with expected rejection layer/code;
- semantic validation;
- compatibility notes.

The encoder is fallible. Invalid domain objects are never serialized as plausible wire data.

## M2 semantic boundary

M2 distinguishes canonical bytes from a typed semantic submission.

```text
raw bytes
   ↓
canonical JSON decoder/schema
   ├─ failure → PlayerWireErrorCodeV1::malformed_response
   │           no PlayerStep
   │           no semantic submit/replay step
   │
   └─ DecisionResponseV2
          ↓
      PlayerEndpoint.submit
          ↓
      accepted PlayerStepV2
      OR typed rejected PlayerStepV2
      OR closed endpoint service failure
```

The adapter/transport must not synthesize a semantic `PlayerStepV2` by reading current state after malformed bytes.

## M2 player identities

Public M2 decision wire may contain only:

- `PlayerDecisionIdV1`;
- request-local `CandidateIdV1`;
- perspective-safe opaque object/ability IDs;
- public player IDs and explicitly authorized payload.

It must not contain internal `DecisionId`, `ContinuationId`, authoritative candidate bindings, trusted object IDs, RNG/checkpoint/replay internals, or global allocator state.

`CandidateIdV1` is encoded in the M2 JSON schema according to its declared canonical scalar representation and is dense/request-local. It is not a stable semantic label.

## Versioning

M2 uses new versions where meaning changes:

- Decision request/response V2;
- Information state V2;
- Observed event V2;
- PlayerStep V2;
- replay V3.

`ObservationEnvelopeV1` may remain because its payload codec identity is independently versioned; M2 uses `synthetic-m2-observation.v1`.

Old wire values retain their original meaning. A new reader may support multiple versions only through explicit per-version decode/validation; no enum/key value is repurposed.
