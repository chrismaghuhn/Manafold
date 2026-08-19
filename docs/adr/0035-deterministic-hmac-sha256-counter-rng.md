# ADR 0035: Deterministic HMAC-SHA-256 counter RNG and typed stream derivation

- **Status:** accepted
- **Date:** 2026-08-19
- **Owners:** architecture maintainers; deterministic-kernel maintainers
- **Supersedes:** none
- **Superseded by:** none
- **Resolves:** OD-008

## Context

Manafold requires authoritative randomness that is exactly reproducible across reset, checkpoint, restore, fork, replay, regression fixtures, and future optimized backends. Every semantic RNG value must be derivable from explicit checkpointable state. Rejected responses consume no randomness, and player/model-facing APIs must never expose trusted RNG internals.

The existing RNG state is placeholder vocabulary: free-form algorithm/derivation strings, a canonical seed string, free-form stream names, and one counter per stream. A seeded third-party generator alone is insufficient because long-lived replay stability also depends on stream derivation, raw-word extraction, bounded sampling, and shuffle semantics.

The detailed byte-level contract is owned by [`../RNG_CONTRACT.md`](../RNG_CONTRACT.md). This ADR records the durable architectural choice and its consequences; it does not duplicate that specification.

## Decision

Adopt the immutable authoritative RNG contract `mtgml.rng.v1` defined in `docs/RNG_CONTRACT.md`.

The reference contract uses:

- a 256-bit explicit root seed;
- counter-addressed HMAC-SHA-256 raw blocks;
- closed typed stream keys with canonical versioned encoding;
- one `u64` next-raw-word cursor per authoritative stream;
- fixed big-endian raw `u64` extraction;
- Manafold-owned unbiased bounded-integer rejection sampling;
- Manafold-owned descending Fisher-Yates permutation;
- complete RNG continuation state inside `EngineState`;
- exact checkpoint/fork copy semantics;
- zero authoritative RNG consumption before transition-workspace creation or on rejection.

Free-form authoritative stream names, process-global RNG, opaque third-party RNG serialization, native-endian/platform-sized sampling, and generic dependency-owned range/shuffle semantics are rejected.

`RandomStreamKindV1` may gain new reviewed purpose codes without changing `mtgml.rng.v1` only when the existing key encoding and random-output semantics remain unchanged. Existing codes never change meaning or get reused; older runtimes may reject unknown additive codes.

Implementing this decision is a separate cross-layer contract migration. It must not be hidden inside M1.1 Issue #20. The migration must introduce typed RNG state and update affected digest/replay/schema/fixture representations coherently before the first production deterministic reset.

Because changing the canonical semantic shape of `RandomState` changes full-state identity, the migration requires a new full-state digest input/domain version and a new replay-manifest version. A `CheckpointDigest` version bump is evaluated on its own contract and is not automatic.

## Consequences

Positive:

- byte-exact project-owned random semantics;
- minimal explicit checkpoint state and exact fork continuation;
- stream isolation without incidental cross-subsystem cursor coupling;
- no dependency on third-party range/shuffle value stability;
- direct block addressing and a clean future batching/vectorization path;
- strong known-answer testing and deterministic dependency-upgrade gates.

Costs:

- HMAC-SHA-256 is selected for semantic clarity, not maximum raw throughput;
- Manafold owns a small permanent stream-key/cursor/sampling/permutation contract;
- the executable migration touches RNG state, full-state digest identity, replay identity, schemas/fixtures, and related documentation;
- new authoritative random distributions require explicit versioned semantics rather than convenience APIs.

Performance is not assumed. If a pinned representative workload later shows RNG is a material bottleneck, the implementation may be optimized under byte-exact parity or a new RNG contract may be proposed. A future optimized backend does not justify changing reference semantics without measured evidence and migration review.

Root-seed secrecy is not a gameplay-correctness guarantee. Trusted replay material may make future random values computable; reproducibility and unpredictability are separate properties.

## Alternatives considered

- **ChaCha12:** strongest stateful alternative; portable and deterministic, but still needs typed derivation, project-owned sampling/shuffle, and explicit persisted position conventions.
- **Philox / Threefry:** strong counter-based candidates for later high-throughput/GPU work, but a Rust reference adoption would add more implementation/audit surface now.
- **BLAKE3 keyed/XOF:** technically strong and likely faster, but adds another foundational hash primitive when SHA-256 is already pinned.
- **PCG / xoshiro:** fast and compact but stateful and still require separate typed stream derivation.
- **HKDF / HMAC-DRBG:** add lifecycle or extract/expand machinery not required for deterministic simulation semantics.
- **Plain SHA-256 concatenation:** smaller dependency surface but a less explicit keyed-PRF boundary than HMAC.
- **One global RNG, free-form streams, or upstream range/shuffle helpers:** rejected because they increase coupling or weaken long-lived replay value stability.

## Evidence and follow-up

The proposed `mtgml.rng.v1` known-answer vectors were independently recomputed from the specification before this ADR was finalized. That review evidence validates the written construction, but it is not executable repository evidence and does not make an M1 gate PASS.

A separate executable RNG-contract migration must add the normative fixtures and run the required checks under the pinned toolchain. At minimum it must cover primitive/project KATs, canonical stream encoding/order, bounded sampling, shuffle, rejection nonmutation, digest/replay versioning, checkpoint/fork continuation, malformed-state negatives, and dependency-upgrade value stability.

Multi-backend/architecture SHA comparisons are hardening evidence, not a blocker for the first M1 reset when those optional backends are unavailable. The pinned reference implementation must still pass all required KATs.

Acceptance of this ADR resolves OD-008 only. `ENGINE_STATE_CONSTRUCTION_AND_INVARIANTS`, `DETERMINISTIC_RNG_AND_ALLOCATORS`, checkpoint/fork/replay gates, and all other executable M1 gates remain `NOT_RUN` until their actual evidence executes successfully.
