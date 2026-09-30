# Wire Contract

**Status:** provisional public wire contract; the current family is listed under Versioning  
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
   └─ DecisionResponseV3
          ↓
      PlayerEndpoint.submit
          ↓
      accepted PlayerStepV4
      OR typed rejected PlayerStepV4
      OR closed endpoint service failure
```

The adapter/transport must not synthesize a semantic `PlayerStepV4` by reading current state after malformed bytes.

## M2 player identities

Public M2 decision wire may contain only:

- `PlayerDecisionIdV1`;
- request-local `CandidateIdV1`;
- perspective-safe opaque object/ability IDs;
- public player IDs and explicitly authorized payload.

It must not contain internal `DecisionId`, `ContinuationId`, authoritative candidate bindings, trusted object IDs, RNG/checkpoint/replay internals, or global allocator state.

`CandidateIdV1` is encoded in the M2 JSON schema according to its declared canonical scalar representation and is dense/request-local. It is not a stable semantic label.

## Versioning

The current wire family is Decision request V4 / response V3,
ObservationEnvelopeV2, PlayerInformationStateV3 / InformationStateDigestV3,
ObservedEventEnvelopeV4, PlayerStepV4, `magic-shared-execution-observation.v1`
and `magic-basic-land-observation.v1`. FullStateDigestV7 / CheckpointDigestV8 /
Replay V8 bind these identities. `ObservationEnvelopeV1` and
`PlayerInformationStateV2` remain as the inner layers the V3 information state
is produced through. Replay V8 carries the content child as lowercase
ContentContractIdV1 plus the canonical ContentContractManifestV1 CBOR payload in
bounded, canonical padded standard Base64; no JSON/Serde representation of the
manifest is introduced.

Each format has one current version and is changed in place (AGENTS.md §4).
Earlier wire versions were removed; readers reject their names as unknown
contracts.

## M2 digest and persistence ownership

`mtgml-observation` owns the semantic `InformationStateDigestInputV2` view and
player-information DTOs. It does not encode canonical JSON or calculate the
digest. `mtgml-wire` is the single owner of the canonical JSON bytes and
`InformationStateDigestV2` calculation; `mtgml-environment` projects the
semantic input, requests that calculation, verifies the result, and only then
exposes or commits the player information state.

Because `InformationStateDigestV2` is the canonical identity of the
player-safe semantic payload, canonical decoders verify it: Rust
`decode_canonical` and the Python `from_wire()`/`validate()` path recompute
the digest over the decoded payload and reject forged digest values as closed
semantic errors before any trusted value is constructed.

`mtgml-persistence` separately owns the restricted canonical CBOR/envelope
codec and the single `CheckpointDigestV8` calculation. There is no
`EnvironmentCheckpointV8` public JSON schema, Python checkpoint DTO, durable
checkpoint file format, or public JSON full-state-input DTO.
