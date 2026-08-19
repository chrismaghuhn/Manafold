# Deterministic RNG Contract

**Status:** accepted architecture contract; implemented  
**Stability:** normative deterministic-randomness contract  
**Owner:** deterministic-kernel maintainers  
**Decision:** ADR 0035  
**Contract identity:** `mtgml.rng.v1`

## Purpose

This document defines the exact authoritative random-bit, stream-derivation, bounded-sampling, and generic permutation semantics for Manafold's reference engine.

The contract exists so that reset, checkpoint, restore, fork, replay, regression fixtures, dependency upgrades, and future optimized backends can reproduce the same random results exactly.

The contract is implemented. See the executable migration evidence for required gate status.

## Contract composition

`mtgml.rng.v1` freezes this semantic tuple:

```text
raw generator:        hmac-sha256-counter.v1
stream-key codec:     random-stream-key.v1
stream derivation:    hmac-sha256-stream-key.v1
raw word extraction:  u64-big-endian-lanes.v1
bounded integer:      uniform-below-u64-rejection.v1
shuffle:              fisher-yates-descending.v1
```

The composite ID is the persisted semantic authority. Component names are diagnostic/specification names, not independently selectable runtime knobs.

Any change that alters a raw output bit, canonical stream-key bytes, cursor advancement, bounded result, zero/one-bound consumption, or shuffle permutation requires a new composite RNG contract identity.

## Authoritative state

Conceptually:

```text
RandomStateV1 {
    contract_id: MtgmlRngV1,
    root_seed: RootSeed256,
    streams: BTreeMap<RandomStreamKeyV1, RandomStreamCursorV1>,
}

RandomStreamCursorV1 {
    next_raw_u64: u64,
}
```

All authoritative continuation state lives inside `EngineState`. The following are derivable caches and must never be authoritative:

- derived stream keys;
- HMAC/SHA contexts;
- computed raw blocks;
- lane buffers;
- speculative prefetch buffers;
- thread-local RNG objects;
- process-global RNG state.

Destroying every derivable RNG cache and reconstructing it from `RandomStateV1` must not change any semantic result.

## Root seed

The root seed is exactly 32 bytes:

```text
RootSeed256([u8; 32])
```

All 256-bit values are valid, including all-zero for deterministic fixtures.

Cryptographic functions consume the bytes in stored order; the seed is never interpreted as a platform integer.

Where a trusted canonical textual representation is required, it is exactly 64 lowercase hexadecimal characters, two characters per byte in byte order.

Entropy acquisition is outside authoritative semantic execution. A controller/orchestrator may obtain external entropy before reset, but reset receives the resulting explicit 32-byte value. Authoritative seed selection must not depend implicitly on:

- wall clock;
- process ID;
- memory address;
- thread/worker identity;
- environment/fork handle;
- filesystem ordering;
- network responses;
- process-global RNG.

Root seed and all derived RNG internals remain trusted information and never cross `PlayerEndpoint` or published trajectory boundaries.

## Typed stream identity

Authoritative streams use typed keys:

```text
RandomStreamKeyV1 {
    kind: RandomStreamKindV1,
    scope: RandomStreamScopeV1,
}
```

Initial scope variants:

```text
Global
Player(PlayerId)
```

Initial kind codes:

```text
0x0000 = reserved / invalid
0x0001 = SyntheticM1
```

`SyntheticM1` is only a synthetic development purpose. Real random semantics receive reviewed purpose-specific kind codes as later capabilities require them.

The canonical `RandomStreamKeyV1` byte encoding is:

```text
offset  width      field
0       1 byte     key-codec version = 0x01
1       2 bytes    kind code, unsigned big-endian
3       1 byte     scope tag
4...    variable   scope payload
```

Scope encoding:

```text
0x00 = Global
       payload length = 0

0x01 = Player
       payload = PlayerId as unsigned u64 big-endian
       payload length = 8
```

Unknown key versions, unknown kind codes, unknown scope tags, invalid kind/scope combinations, malformed payload lengths, trailing bytes, and invalid referenced players fail closed.

Free-form string stream names are not authoritative semantics.

### Additive stream kinds

`mtgml.rng.v1` freezes the encoding and derivation semantics of stream-kind codes, not the forever-complete set of gameplay purposes.

New reviewed purpose codes may be added without changing the RNG contract only when:

- existing code points retain identical meaning;
- no code point is reused;
- canonical key bytes remain unchanged for all existing keys;
- raw/sampling/shuffle semantics remain unchanged.

Older runtimes may fail closed on an unknown additive kind.

## Stream derivation

Definitions:

```text
STREAM_DOMAIN = ASCII("mtgml.rng.stream-key.v1")
S             = canonical RandomStreamKeyV1 bytes
root          = RootSeed256 bytes
```

The 32-byte derived stream key is:

```text
K_stream = HMAC-SHA-256(
    key  = root,
    data = STREAM_DOMAIN
           || 0x00
           || u32_be(length(S))
           || S
)
```

The complete HMAC result is used. There is no truncation or text normalization.

Stream derivation consumes no cursor and mutates no semantic state.

## Raw block and raw-word semantics

Definition:

```text
RAW_DOMAIN = ASCII("mtgml.rng.raw-block.v1")
```

For unsigned `u64` block index `c`:

```text
block(c) = HMAC-SHA-256(
    key  = K_stream,
    data = RAW_DOMAIN
           || 0x00
           || u64_be(c)
)
```

Each 32-byte block yields four raw `u64` words:

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

`raw_u64(i)` is the corresponding lane above.

The v1 authoritative raw API has no semantic `usize`, native-endian integer, floating-point draw, bit reservoir, `fill_bytes`, or implicit `u32` mapping.

## Cursor semantics

`next_raw_u64` is the raw-word index of the next value to consume.

A successful raw draw:

1. locates the exact typed stream;
2. fails with zero mutation if the stream is absent;
3. reads `i = next_raw_u64`;
4. fails with zero mutation when `i == u64::MAX`;
5. computes `raw_u64(i)`;
6. advances the workspace cursor to `i + 1`;
7. returns the word.

The cursor never wraps.

Valid consumed raw-word indices are therefore `0 .. u64::MAX - 1` inclusive, after which the cursor equals `u64::MAX` and further consumption fails closed.

## Canonical stream-state representation

The runtime may use:

```text
BTreeMap<RandomStreamKeyV1, RandomStreamCursorV1>
```

but the runtime map is not itself a canonical persisted representation.

Every canonical digest/checkpoint/replay/wire representation that directly includes the stream map must encode it as an explicit entry array:

```text
[
  {
    key: <canonical RandomStreamKeyV1 representation>,
    next_raw_u64: <u64>
  },
  ...
]
```

Entries are sorted lexicographically by the canonical binary stream-key bytes defined above.

Readers reject duplicate keys before constructing a map and never apply last-value-wins semantics.

Canonical ordering must not depend on incidental Rust `Ord`, hash-map iteration, insertion order, or arbitrary structured JSON map-key behavior.

## Whole-state validation ownership

Generic `validate_engine_state()` owns RNG relationships that are true for every valid authoritative state, including:

- supported RNG contract identity;
- structurally valid stream keys;
- valid kind/scope combinations;
- player-scoped keys referencing existing players;
- duplicate-free decoded canonical state;
- valid cursor values and state shape.

Generic whole-state validation must not encode fixture/reset policy such as requiring `SyntheticM1/Global` or forbidding other otherwise-valid streams.

A particular reset/configuration constructor owns the exact stream plan it requires.

## Uniform bounded integer

The authoritative unbiased mapping to `[0, n)` is:

```text
uniform_below_u64(n, stream):

    if n == 0:
        return InvalidRandomBound
        consume zero raw words

    if n == 1:
        return 0
        consume zero raw words

    threshold = (2^64 mod n)
                computed exactly in u128

    loop:
        x = next_raw_u64(stream)
        if x >= threshold:
            return x mod n
```

The accepted raw interval has length divisible by `n` and therefore avoids modulo bias.

Cursor advancement equals the number of examined raw words, including raw words rejected by the sampler.

All request/range validation and integer-width conversion occurs before the first authoritative draw.

Authoritative code does not sample `usize` directly.

### Checked half-open ranges

For `[lower, upper)`:

```text
require lower < upper
width = checked_sub(upper, lower)
return lower + uniform_below_u64(width)
```

All checks occur before RNG consumption.

### Sequence selection

```text
length 0 -> error, zero draws
length 1 -> index 0, zero draws
length > 1 -> checked conversion to u64, then uniform_below_u64(length)
```

## Generic permutation

The authoritative generic permutation primitive is descending Fisher-Yates:

```text
shuffle(values, stream):
    validate length before drawing

    for i from len - 1 down to 1:
        j = uniform_below_u64(i + 1, stream)
        swap(values[i], values[j])
```

Length zero or one consumes zero raw words.

The stream is explicit. The caller's input order must already be authoritative/canonical; unordered-container iteration must never supply the initial sequence.

This primitive does not define Magic-specific library randomization, zone movement, knowledge invalidation, opaque-identity invalidation, semantic events, or observed-event redaction. Those belong to later rule capabilities.

## Excluded v1 sampling semantics

`mtgml.rng.v1` does not define:

- weighted choice;
- floating-point uniform draws;
- arbitrary probability distributions;
- Gaussian/continuous distributions;
- random bit packing/reservoir semantics;
- experiment-policy/exploration RNG.

A future authoritative sampler requires a separately reviewed versioned contract and cannot silently reuse a dependency convenience API under the existing contract identity.

## Transaction and rejection semantics

Authoritative randomness is consumed only after a transition workspace has been created.

Player-response decoding and validation—including actor, decision identity, expected revision, candidate binding, cardinality, numeric constraints, legality, and capability support—consume zero authoritative randomness.

Any random execution operates on the workspace's candidate `RandomStateV1`.

State, events, delta, replay step, next decision, and episode status commit atomically. Rejection, unsupported semantics, invariant failure, malformed transition product, or RNG exhaustion discards the workspace.

A rejected response therefore preserves exactly:

- RNG contract ID;
- root seed;
- stream-key set;
- every stream cursor;
- RNG-derived trusted event/history state.

A process-global/shared mutable RNG is forbidden.

## Trusted audit data

A Manafold-owned random sampling operation must provide enough trusted audit information to prove event/state/delta consistency, including as applicable:

- typed stream key;
- sampler identity;
- cursor before;
- cursor after;
- requested bound/range;
- sampled semantic result.

The exact event representation is owned by the rules/event contract of the capability using randomness.

Player-visible observed events may expose a random result only when rules make that result visible. They never expose root seed, typed stream key, raw words, derived keys, or stream cursor.

## Checkpoint and restore

Complete authoritative RNG continuation state is contained in `EngineState`.

A checkpoint is semantically complete with respect to RNG because the state includes:

- `mtgml.rng.v1` identity;
- 32-byte root seed;
- exact typed stream-key set;
- every `next_raw_u64` cursor.

Restore validates the complete checkpoint/state before backend mutation and reconstructs only derivable caches.

No third-party RNG struct or private buffer is serialized as Manafold state.

## Fork semantics

Fork copies the complete checkpoint exactly.

The following are not RNG derivation inputs:

- fork ID;
- environment handle;
- episode ID;
- worker/thread/process ID;
- search-branch ID;
- wall clock.

Two forks restored from the same checkpoint and given identical later semantic inputs produce identical RNG outputs and cursor progression.

A future search system that wants search-only stochastic variation must define separate trusted search randomness; it may not silently perturb authoritative fork RNG state.

## Replay identity

A trusted reset-based replay must identify:

```text
contract_id = "mtgml.rng.v1"
root_seed   = exact trusted 32-byte value / canonical trusted encoding
```

Initial per-stream cursors belong to the initial authoritative state/checkpoint.

A replay beginning from a non-reset position must embed or reference a complete checkpoint/equivalent explicitly versioned state identity and must not reconstruct nonzero stream cursors heuristically.

Old replays never execute under a different RNG contract. Unsupported RNG contract IDs fail closed.

Root seed and RNG internals are forbidden from published player trajectories.

## Full-state digest compatibility

Implementing `RandomStateV1` changes the semantic and canonical representation of the `random` component of `EngineState`.

Therefore the executable migration implementing this contract must introduce a new full-state canonical digest input version and domain rather than reusing `full-state-digest-input.v1` / `mtgml.full-state-digest.v1` with changed meaning.

The stream map inside that digest input must use the explicit canonical entry-array representation defined above.

## Replay and checkpoint versioning

The executable migration requires a new replay-manifest version rather than reinterpreting existing randomness fields in place.

A `CheckpointDigest` version bump is not automatic. Its own canonical input and semantic meaning must be evaluated separately. If it already binds the newly versioned `FullStateDigest` identity without changing its own semantics, it may remain unchanged. If its own contract changes, it receives an explicit version migration.

This contract does not resolve OD-017, the future durable persisted checkpoint codec/version decision.

## Versioning rules

`mtgml.rng.v1` is immutable once executable artifacts use it.

Any change to the following requires a new RNG contract identity:

- root-seed byte length/interpretation;
- stream-key codec bytes;
- domain strings;
- HMAC preimages;
- block/lane ordering;
- cursor unit or overflow behavior;
- zero/one-bound draw consumption;
- bounded-integer mapping;
- generic shuffle order.

Dependency crate versions are build provenance, not RNG semantics.

A `sha2`/`hmac` upgrade may remain under `mtgml.rng.v1` only when all required primitive and Manafold known-answer, sampling, shuffle, replay, and checkpoint tests remain byte-identical.

A value-changing dependency upgrade must be rejected or introduced under a new semantic contract with explicit migration/provenance.

## Known-answer vectors

The following vectors were independently recomputed from this specification before acceptance. They become executable normative fixtures only when committed and run by the executable migration.

### Base fixture

```text
root_seed =
0000000000000000000000000000000000000000000000000000000000000000

stream = SyntheticM1 / Global

canonical stream bytes S =
01000100
```

Stream-derivation message:

```text
6d74676d6c2e726e672e73747265616d2d6b65792e7631000000000401000100
```

Derived stream key:

```text
73635feaa9e90effe337e2cc9e1d801f63c9ede8d51b21a1120e624da2d648f9
```

Raw block 0:

```text
6818e6bd053d9b770e26253e8d724b0403c524aeb6b3cff52508069342e336e4
```

Raw words 0–3:

```text
0x6818e6bd053d9b77
0x0e26253e8d724b04
0x03c524aeb6b3cff5
0x2508069342e336e4
```

Raw block 1:

```text
ac6a5d827f0dcbbf060d1adce197e55569da50c9030d2a2b2a7f637923566d45
```

Raw words 4–7:

```text
0xac6a5d827f0dcbbf
0x060d1adce197e555
0x69da50c9030d2a2b
0x2a7f637923566d45
```

### Bounded sample

For the same base fixture:

```text
uniform_below_u64(10) = 7
raw words consumed     = 1
```

A sampler unit fixture using stub raw words `[0, 6]` with bound `10` must reject `0`, accept `6`, return `6`, and consume two words.

### Shuffle

For the same base fixture:

```text
input              = [0, 1, 2, 3, 4]
output             = [1, 3, 4, 0, 2]
raw words consumed = 4
```

### Cursor boundary

```text
input cursor = u64::MAX - 1
block index  = 0x3fffffffffffffff
lane         = 2
raw word     = 0x021a6c120112e7b3
result cursor = u64::MAX
next draw     = counter-exhaustion error, zero mutation
```

## Required executable evidence

The implementation migration must add and execute evidence for at least:

1. standard HMAC-SHA-256 primitive vectors;
2. all Manafold KATs above;
3. canonical stream-key encoding and decoding;
4. canonical stream-entry ordering and duplicate rejection;
5. bounded-integer normal, rejection, zero, one, and invalid-bound cases;
6. generic shuffle vectors;
7. stream isolation;
8. cursor block/lane transitions and exhaustion;
9. malformed root/contract/key/scope/player/stream state negatives;
10. reset determinism and zero consumption during construction;
11. complete rejection nonmutation;
12. checkpoint/restore continuation parity;
13. fork parity;
14. replay parity and unsupported-version rejection;
15. full-state digest fixtures using a nonempty typed stream map;
16. dependency-upgrade value-stability regression.

Multi-backend/architecture SHA implementation comparisons are valuable hardening evidence but do not block the first M1 deterministic reset when optional backends are unavailable. Unavailable optional comparisons are `NOT_RUN`; the pinned reference implementation still must pass all required primitive and project vectors.

A one-time long-stream statistical smoke analysis may be retained as research evidence, but statistical testing is not a substitute for the deterministic known-answer and state-transition proof obligations above.

## Performance policy

The reference contract is chosen for semantic clarity and reproducibility, not a claim of best raw throughput.

Manafold RNG performance is `NOT_RUN` until measured on a pinned representative workload.

Performance review is triggered when evidence shows RNG is a material fraction of a relevant simulation/search workload or a future optimized/vector/GPU backend cannot meet an accepted budget while preserving v1 byte parity.

Before changing the semantic RNG contract for speed, first consider an optimized implementation of the same HMAC-SHA-256 contract and prove byte/cursor parity. A new algorithm requires a new contract identity and compatibility/provenance review.

## Information-safety requirements

Protected RNG values include:

- root seed;
- typed stream keys when they reveal hidden random purpose;
- stream cursors;
- derived stream keys;
- raw words;
- authoritative RNG audit data.

These values may be used by trusted kernel/controller/replay/conformance/debug tooling only.

They must not appear in:

- `PlayerEndpoint` responses;
- player observations;
- player information state;
- player-safe errors;
- published model trajectories.

Visible random outcomes may be projected only when the rules/capability explicitly makes the result visible.

## Failure behavior

The implementation must fail closed for at least:

- invalid root seed shape;
- unsupported RNG contract;
- unknown stream-key codec version;
- unknown stream kind;
- unknown scope tag;
- invalid kind/scope combination;
- player-scoped stream referencing an absent player;
- duplicate stream key;
- missing requested stream;
- cursor exhaustion;
- invalid bound/range/sequence length;
- malformed checkpoint RNG state;
- incompatible replay RNG identity;
- random consumption attempted during pre-workspace validation.

Failure before commit preserves authoritative RNG state exactly.

## Implementation boundary

ADR 0035 resolves the architecture decision. This document owns the normative detailed contract.

The executable migration is a separate cross-layer change and must update every affected representation together, including as applicable:

- `mtgml-random` typed state and primitives;
- `EngineState` integration and generic invariants;
- full-state digest input/domain version;
- replay-manifest version;
- Rust/Python/schema/golden/negative fixtures;
- compatibility/migration documentation.

That migration must be merged and its required checks executed before M1.1 performs the first production deterministic reset.

The M1.1 synthetic constructor owns only its required stream plan (`SyntheticM1/Global` at cursor zero); generic `validate_engine_state()` does not.
