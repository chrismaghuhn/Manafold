# ADR 0035: Deterministic HMAC-SHA-256 counter RNG and typed stream derivation

- **Status:** accepted
- **Date:** 2026-08-19
- **Owners:** architecture maintainers; deterministic-kernel maintainers
- **Supersedes:** none
- **Superseded by:** none
- **Resolves:** OD-008

## Context

Manafold requires authoritative randomness that is exactly reproducible across reset, checkpoint, restore, fork, replay, regression fixtures, and future optimized backends. Every semantic RNG value must be derivable from explicit checkpointable state. Rejected player responses consume no randomness. Player-facing and model-facing APIs must never expose root seeds, stream identities, stream cursors, raw random words, or trusted RNG diagnostics.

The existing `mtgml-random::RandomState` is placeholder vocabulary: free-form algorithm/derivation strings, a canonical seed string, free-form stream names, and one `u64` counter per stream. It does not yet define raw output, typed stream identity, bounded sampling, shuffle semantics, or a persisted value-stability contract.

A seeded third-party RNG alone is insufficient for Manafold because replay stability also depends on the mapping from raw randomness to bounded integers and permutations. Those mappings must not silently change through dependency upgrades.

## Decision

Manafold adopts the immutable authoritative randomness contract:

```text
mtgml.rng.v1
```

The contract consists of:

```text
raw generator:        HMAC-SHA-256 counter-addressed blocks
root seed:            exactly 256 bits / 32 bytes
typed stream keys:    canonical RandomStreamKeyV1 values
stream state:         one u64 next-raw-word cursor per stream
raw word extraction:  four big-endian u64 words per 32-byte block
bounded integer:      Manafold-owned rejection sampling
shuffle:              Manafold-owned descending Fisher-Yates
```

No process-global RNG, opaque third-party RNG struct serialization, generic library `gen_range`, generic shuffle helper, floating-point sampling, native-endian integer conversion, or platform-sized integer is part of the authoritative v1 contract.

### Composite contract identity

`mtgml.rng.v1` freezes this semantic tuple:

```text
hmac-sha256-counter.v1
random-stream-key.v1
hmac-sha256-stream-key.v1
u64-big-endian-lanes.v1
uniform-below-u64-rejection.v1
fisher-yates-descending.v1
```

The composite ID is the persisted authority. The component names are normative diagnostics; implementations may not mix and match them under the same contract ID.

Any change that alters a raw output bit, canonical stream-key bytes, cursor advancement, bounded result, zero/one-bound consumption, or shuffle permutation requires a new RNG contract identity.

### Root seed

The authoritative root seed is exactly 32 bytes:

```text
RootSeed256([u8; 32])
```

All 256-bit values are valid, including the all-zero value used by deterministic fixtures. Cryptographic functions consume the bytes in stored order. Trusted canonical text representation, where required, is exactly 64 lowercase hexadecimal characters.

Entropy acquisition is controller/orchestrator policy outside the semantic kernel. Reset receives explicit root bytes. Wall clock, process ID, memory address, thread identity, environment handle, filesystem state, or implicit OS randomness are never seed inputs inside authoritative execution.

### Typed stream identities

Authoritative streams use closed typed keys, conceptually:

```text
RandomStreamKeyV1 {
    kind: RandomStreamKindV1,
    scope: RandomStreamScopeV1,
}
```

Initial scope variants are:

```text
Global
Player(PlayerId)
```

Initial kind code:

```text
0x0000 = reserved/invalid
0x0001 = SyntheticM1
```

`SyntheticM1` is only a synthetic development purpose. Real random semantics add reviewed purpose-specific kind codes later.

Canonical `RandomStreamKeyV1` bytes are:

```text
offset  width      field
0       1 byte     codec version = 0x01
1       2 bytes    kind code, unsigned big-endian
3       1 byte     scope tag
4...    variable   scope payload
```

Scopes encode as:

```text
0x00 = Global, no payload
0x01 = Player, payload = PlayerId as u64 big-endian
```

Unknown versions, unknown kind codes, unknown scope tags, invalid kind/scope combinations, invalid player references, malformed lengths, or trailing bytes fail closed.

The set of semantic purpose codes may grow additively under `mtgml.rng.v1` when the existing encoding and output semantics remain unchanged. Existing codes never change meaning and are never reused. Older runtimes may fail closed on unknown additive codes.

Free-form string stream names are not authoritative semantics.

### Stream derivation

Let:

```text
STREAM_DOMAIN = ASCII("mtgml.rng.stream-key.v1")
S             = canonical RandomStreamKeyV1 bytes
root          = RootSeed256 bytes
```

The derived stream key is:

```text
K_stream = HMAC-SHA-256(
    key  = root,
    data = STREAM_DOMAIN
           || 0x00
           || u32_be(length(S))
           || S
)
```

The complete 32-byte HMAC output is used. Stream derivation consumes no cursor and mutates no semantic state. A derived stream key may be cached only as disposable, derivable state.

### Raw output semantics

Let:

```text
RAW_DOMAIN = ASCII("mtgml.rng.raw-block.v1")
```

For unsigned `u64` block index `c`:

```text
block(c) = HMAC-SHA-256(
    key  = K_stream,
    data = RAW_DOMAIN || 0x00 || u64_be(c)
)
```

Each 32-byte block yields four raw words:

```text
word 0 = u64_be(block[ 0.. 8])
word 1 = u64_be(block[ 8..16])
word 2 = u64_be(block[16..24])
word 3 = u64_be(block[24..32])
```

For raw-word index `i`:

```text
block_index = i / 4
lane        = i % 4
```

The authoritative API exposes raw `u64` semantics only. There is no v1 authoritative float, `usize`, native-endian, bit-reservoir, `fill_bytes`, or implicit `u32` contract.

### Stream cursor semantics

Per stream:

```text
RandomStreamCursorV1 {
    next_raw_u64: u64
}
```

The cursor is the index of the next raw `u64` word. A successful raw draw:

1. locates an existing typed stream;
2. fails with zero mutation if the stream is absent;
3. fails with zero mutation if `next_raw_u64 == u64::MAX`;
4. computes the raw word at the current cursor;
5. advances the workspace cursor by exactly one;
6. returns the word.

The cursor never wraps. A stream can therefore successfully consume raw-word indices `0 .. u64::MAX - 1`.

### Authoritative state and canonical representation

Conceptually:

```text
RandomStateV1 {
    contract_id: MtgmlRngV1,
    root_seed: RootSeed256,
    streams: BTreeMap<RandomStreamKeyV1, RandomStreamCursorV1>,
}
```

The runtime map is not itself the persisted/canonical contract. Any canonical digest, checkpoint, replay, or wire representation that includes stream state MUST encode streams as an explicit entry array:

```text
[
  {
    key: <canonical RandomStreamKeyV1>,
    next_raw_u64: <u64>
  },
  ...
]
```

Entries are sorted lexicographically by the canonical stream-key bytes. Readers reject duplicate keys before constructing a map. They never apply last-value-wins behavior.

Implementations must not rely on arbitrary structured JSON map keys or incidental Rust `Ord` as the canonical persisted ordering rule.

Derived keys, HMAC contexts, raw blocks, lane buffers, prefetch state, and thread-local RNG objects are never authoritative.

### Whole-state validation ownership

Generic `validate_engine_state()` may validate RNG relationships that are true for all valid engine states, including:

- supported RNG contract identity;
- structurally valid typed stream keys;
- valid kind/scope combinations;
- player-scoped keys referencing existing players;
- duplicate-free canonical state;
- valid cursor representation.

It MUST NOT encode M1 fixture policy such as requiring `SyntheticM1/Global` or forbidding unrelated valid streams. Required stream plans for a particular reset/configuration belong to that reset/configuration constructor and its tests, not to generic whole-state validation.

### Uniform bounded integer

For `uniform_below_u64(n, stream)`:

```text
if n == 0:
    error, consume zero raw words

if n == 1:
    return 0, consume zero raw words

threshold = (2^64 mod n), computed exactly in u128

loop:
    x = next_raw_u64(stream)
    if x >= threshold:
        return x mod n
```

The accepted raw interval has a length divisible by `n`; the mapping is unbiased. Cursor advancement equals the number of examined raw words, including rejected raw words.

All range validation and integer-width conversion occurs before the first draw. Authoritative code never samples `usize` directly.

### Shuffle

Generic deterministic permutation uses descending Fisher-Yates:

```text
for i from len - 1 down to 1:
    j = uniform_below_u64(i + 1, stream)
    swap(values[i], values[j])
```

Lengths zero and one consume zero draws. The stream is always explicit. Input ordering must already be authoritative/canonical; unordered container iteration must never define the pre-shuffle sequence.

This generic primitive does not define Magic library movement, knowledge invalidation, opaque-ID changes, events, or visibility. Those belong to later rules capabilities.

### Rejection and transaction behavior

Player-response decoding, actor/decision/revision checks, candidate binding, cardinality validation, numeric validation, and legality validation consume no authoritative randomness.

Randomness may be consumed only after creating the transition workspace. The workspace owns its candidate `RandomState`. State, events, delta, replay step, next decision, and episode status commit atomically. Any rejected response, unsupported capability, invariant failure, malformed transition result, or RNG exhaustion discards the workspace.

A rejected response therefore preserves the complete authoritative RNG state exactly.

### Checkpoint and fork behavior

Complete RNG continuation state is inside `EngineState`: contract ID, root seed, typed stream-key set, and every stream cursor. No hidden RNG object may exist in a controller or backend.

Restore validates complete checkpoint/state identity before backend mutation and reconstructs only derivable caches.

Fork copies the complete checkpoint exactly. Fork ID, environment ID, episode ID, worker ID, process ID, thread ID, and search-branch ID do not perturb authoritative RNG state. Identical forks with identical later semantic inputs produce identical RNG results.

### Replay identity

The executable migration implementing this ADR MUST introduce a replay-manifest version that identifies `mtgml.rng.v1` and the trusted root seed without reinterpreting existing replay-manifest-v1 fields in place. Initial stream cursors come from the initial authoritative state/checkpoint.

A replay beginning from a non-reset position must start from a complete checkpoint or equivalent explicitly versioned state identity; it must not reconstruct nonzero stream cursors heuristically.

Old replays are never silently executed under newer RNG semantics. Unsupported RNG contracts fail closed.

### Digest and checkpoint compatibility

Implementing this ADR changes the semantic and canonical shape of authoritative `RandomState`. Therefore the executable migration MUST introduce a new full-state canonical digest input version and domain rather than reusing `full-state-digest-input.v1` / `mtgml.full-state-digest.v1` with changed meaning.

The replay manifest likewise requires a new version.

A `CheckpointDigest` version bump is NOT automatic. The migration must evaluate that digest's own canonical input and semantic identity separately. If its own contract does not change and it already binds the newly versioned `FullStateDigest` identity adequately, it may remain unchanged. If its own meaning/input changes, it must receive an explicit version migration.

This ADR does not resolve OD-017, the future durable persisted checkpoint codec decision.

### Known-answer evidence

The executable migration MUST include independent primitive and Manafold known-answer tests. At minimum, for an all-zero 32-byte root and `SyntheticM1/Global` with canonical stream bytes `01000100`:

```text
K_stream =
73635feaa9e90effe337e2cc9e1d801f63c9ede8d51b21a1120e624da2d648f9

raw block 0 =
6818e6bd053d9b770e26253e8d724b0403c524aeb6b3cff52508069342e336e4

raw words 0..3 =
6818e6bd053d9b77
0e26253e8d724b04
03c524aeb6b3cff5
2508069342e336e4
```

The same fixture must establish the project-owned bounded sampler and shuffle output. These vectors are contract fixtures, not evidence of a PASS until they execute successfully under the pinned toolchain.

Multi-backend/architecture SHA implementation comparisons are valuable hardening evidence but are not a prerequisite for the first M1 deterministic reset when those backends are unavailable. Unavailable optional backends remain `NOT_RUN`; the pinned reference implementation must still pass all primitive and project KATs.

## Consequences

Positive consequences:

- byte-exact, project-owned randomness semantics;
- explicit checkpointable continuation state;
- deterministic, cheap fork semantics;
- typed stream isolation instead of stringly named streams;
- direct block addressing without hidden generator buffers;
- no dependency on third-party bounded-range or shuffle value stability;
- a clean path to future batching/vectorization under exact parity;
- failure/rejection can prove complete RNG nonmutation.

Costs and constraints:

- HMAC-SHA-256 is not selected for maximum raw throughput; Manafold-specific RNG performance is currently `NOT_RUN`;
- the project owns a small but durable stream-key, counter, sampling, and shuffle contract;
- implementing the decision requires explicit RandomState, full-state-digest, replay, schema/fixture, and cross-language migration work;
- adding a new authoritative random distribution requires a reviewed, versioned semantic contract rather than a convenience library call.

`RootSeed256` secrecy is not a gameplay correctness requirement. If a trusted replay exposes the root, all stream outputs are computable. Reproducibility and unpredictability are separate properties.

## Alternatives considered

- **ChaCha12:** strongest stateful alternative; deterministic and portable, but still needs separate typed stream derivation, Manafold-owned sampling/shuffle, and explicit persisted position conventions.
- **ChaCha8/ChaCha20:** no demonstrated Manafold requirement justifies selecting reduced rounds for speed or extra rounds for security margin.
- **Philox / Threefry:** excellent counter-based candidates for a future optimized/vector/GPU backend, but adopting or maintaining a Rust reference implementation now adds disproportionate implementation surface.
- **PCG / xoshiro:** fast and compact but stateful; they still require a separate typed stream-derivation contract and provide less direct addressability for the reference design.
- **BLAKE3 keyed/XOF:** technically strong and likely faster, but adds another foundational hash implementation when SHA-256 is already pinned.
- **HKDF-SHA-256 / HMAC-DRBG:** add lifecycle or extract/expand conventions that do not improve the required deterministic simulation semantics.
- **Plain SHA-256 concatenation:** smaller dependency surface but gives a less explicit keyed-PRF boundary than HMAC for stream/domain separation.
- **One global RNG / free-form string streams / upstream range or shuffle APIs:** rejected because they increase cross-subsystem coupling or weaken long-lived replay value stability.

## Evidence and follow-up

Acceptance of this ADR resolves the OD-008 architecture decision only. It does not make any M1 acceptance gate PASS and does not itself implement `mtgml.rng.v1`.

Before the first production M1 deterministic reset, create and merge a separate executable RNG-contract migration that updates every affected representation coherently, including at least:

- `mtgml-random` typed state and deterministic primitives;
- authoritative `EngineState` RNG integration and generic validation;
- a new full-state digest input/domain version;
- replay-manifest versioning;
- affected Rust/Python/schema/golden/negative fixtures and documentation;
- exact known-answer, rejection-nonmutation, canonical-ordering, and compatibility tests.

That migration is a cross-layer contract change and must follow the repository's schema-evolution and compatibility policies. It must not be hidden inside Issue #20's synthetic state construction work.

M1.1 may proceed only after that migration is merged and its required checks actually pass. M1.5 then exercises the already-fixed draw/cursor/sampling semantics rather than making a new RNG architecture decision.

A later optimization may replace the implementation strategy only after profiling and exact parity evidence. A measured material RNG bottleneck, a future GPU/vector requirement, a relevant cryptographic break, or a lower-risk mature implementation with demonstrated byte parity are valid review triggers; convenience alone is not.
