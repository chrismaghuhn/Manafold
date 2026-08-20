# ADR 0038: Persisted semantic digests and canonical state codec

- **Status:** accepted
- **Date:** 2026-08-20
- **Resolves:** OD-017
- **Supersedes:** none; earlier accepted identities retain their historical meaning
- **Unblocks:** persisted-codec specification and implementation required before the first durable persisted checkpoint

## Context

Manafold already requires canonical, versioned, domain-separated semantic digests and has a complete in-memory checkpoint contract. It intentionally does not yet define a durable historical checkpoint codec.

Runtime Rust types, Serde representations, container iteration, serializer-library behavior, and file packaging must not become accidental persisted semantics. Long-lived checkpoints and other semantic artifacts need identities whose meaning survives changes to runtime types and libraries.

Digest algorithm, digest framing, semantic domain, canonical payload codec, semantic input schema, artifact schema, and migration are independent compatibility dimensions. Historical versions must never be silently reinterpreted under current runtime semantics.

## Decision

### Persisted digest algorithm

New persisted semantic identities use **unkeyed SHA-256** with a 32-byte output. Canonical text uses 64 lowercase hexadecimal characters.

The digest provides stable content identity and divergence detection, not authenticity.

### Common digest envelope

New persisted digest contracts use one versioned, unambiguously framed envelope binding:

- envelope identity/version;
- algorithm identity;
- semantic domain identity;
- canonical payload codec identity;
- semantic input schema identity;
- canonical payload bytes.

These remain separate compatibility dimensions rather than being collapsed into one version string or inferred from a runtime type. Exact envelope bytes belong to the persistence-codec specification.

### Canonical semantic codec

The first persisted semantic payload codec is `mtgml.canonical-cbor.v1`, a Manafold-owned restricted profile of RFC 8949 deterministic CBOR.

The profile requires one canonical byte representation per declared schema, definite lengths, shortest permitted primitive encodings, deterministic collection representation, duplicate rejection, bounded fail-closed decoding, valid UTF-8 without codec-level normalization, explicit integer ranges, and preservation of semantic sequence order.

It excludes features that add unnecessary ambiguity to authoritative persistence, including floating point, tags, bignums, indefinite values, shared references, malformed UTF-8, and trailing values.

A CBOR library is an implementation detail. The Manafold profile specification and shared golden/negative fixtures are authoritative. Exact field encoding, enum tags, optional-value representation, collection layouts, limits, and error taxonomy are deferred to that specification.

### Detached versioned semantic inputs

Persisted semantic inputs and persisted checkpoint state use explicit, detached, version-specific representations.

Current runtime `EngineState` converts fallibly into the current persisted representation. Persisted contracts do not make current `EngineState`, arbitrary nested runtime types, or their Serde output the historical definition.

Historical representations may remain decodable and verifiable without becoming current runtime types or requiring a permanent legacy rules kernel.

### Canonicalization principles

Persistence schemas preserve semantic order while defining canonical representation for unordered data. Runtime map/set iteration, `Ord`, insertion order, source-language declaration order, platform-sized integers, native endianness, pointers, and process-local identities never implicitly define persisted meaning.

Record fields and enum variants have stable schema identities independent of Rust names. Missing values are not silently equivalent to present defaults. Strings preserve exact valid UTF-8 bytes; any semantic normalization occurs before the persistence codec under its own contract.

The exact encoding mechanisms implementing these principles are deferred to the persistence-codec specification.

### Independent versioning

The following evolve independently:

```text
digest algorithm
digest envelope
semantic domain
canonical payload codec
semantic input schema
artifact/container schema
migration
```

A new authoritative state field normally changes the semantic input schema when it changes state identity. A codec change receives a new codec identity even if semantics are unchanged. A future hash-algorithm change does not reinterpret old digests.

### Historical artifacts

Historical persisted artifacts retain their exact original meaning and are explicitly classified as:

- `EXECUTABLE`;
- `MIGRATION_REQUIRED`;
- `READABLE_VERIFIABLE_ONLY`;
- `UNSUPPORTED`.

Migration verifies the source under its original contract, converts through a versioned Rust-authoritative migration, validates the resulting current semantic state, writes a new artifact with a new identity and provenance, and never overwrites or relabels the source.

Manafold does not maintain a complete legacy rules engine solely to decode historical checkpoints. Historical execution may require an archived matching engine build.

### Rust and Python boundary

Rust remains authoritative for semantic state, rules, legality, migration semantics, checkpoint restoration, and replay execution.

Trusted Python tooling may mechanically reproduce persistence vocabulary, canonical bytes, envelope framing, persisted digest values, and shared fixtures. Python must not become a second rules engine or independently define semantic migration.

### Raw bytes and authenticity

Raw files, compressed archives, SQLite databases, or packaging bytes are not semantic identity unless a separately typed raw-byte checksum is explicitly intended. Semantic digests identify canonical semantic inputs; raw-byte checksums may separately identify acquisition or transport bytes.

Signing and attestation remain separate from this decision.

### Existing and provisional contracts

Existing accepted digest, replay, and checkpoint identities retain their original meaning. Any version accepted before this architecture is implemented also retains its exact accepted meaning.

The first durable implementation under this ADR allocates the next unused versions from the then-current repository and never redefines an earlier version to use the new codec or detached representation.

## M1 boundary

This ADR does **not** block M1's in-memory deterministic kernel work, checkpoint/restore/fork parity, replay parity, or RNG integration.

It also does not make durable checkpoint persistence complete merely by being accepted. Before a durable persisted-checkpoint claim, Manafold still requires the codec/profile specification, detached persisted state/checkpoint representations, cross-language mechanical fixtures, compatibility/migration behavior, and executable evidence.

No M1 acceptance gate becomes `PASS` because this ADR is accepted.

## Consequences

Positive:

- historical semantic identity no longer depends on mutable Rust or Serde representation;
- algorithm, codec, semantic schema, and artifact schema can evolve independently;
- Rust/Python parity is mechanically testable without duplicating game semantics;
- canonical JSON wire contracts may coexist with a binary persisted-state codec;
- checkpoint compatibility becomes explicit rather than inferred from a versioned Rust type name.

Costs:

- version-specific persisted representations and migrations require maintenance;
- the strict CBOR profile needs a dedicated specification and adversarial fixtures;
- the first durable checkpoint still requires implementation and verification.

## Rejected alternatives

- hashing arbitrary Serde/runtime serialization;
- direct durable persistence of current `EngineState`;
- generic library-defined "canonical CBOR" without a Manafold profile;
- canonical JSON as the default durable authoritative-state codec;
- deterministic Protobuf or Rust-native serializer output as canonical semantic identity;
- keyed hashing for universally verifiable semantic identity;
- raw database/archive/checkpoint file bytes as semantic identity;
- BLAKE3 or SHA-512/256 as the first persisted algorithm without demonstrated need;
- immediate Merkle, rolling, or incremental state hashing.

## Implementation boundary

This ADR intentionally does not freeze envelope field numbers, persistence field numbers, enum tags, optional encoding, unordered-collection layout, decoder limits/error codes, a concrete CBOR library, checkpoint file layout, or migration implementation details.

Those belong to a separately reviewed persistence-codec specification and executable conformance fixtures.

## Review trigger

Revisit this ADR if SHA-256 no longer meets the required interoperability/security horizon, measured profiling shows persisted canonical hashing is a material bottleneck, a later deterministic-CBOR standard materially supersedes the V1 profile, cross-language parity proves disproportionately complex, or authoritative persistence requires data types intentionally excluded from `mtgml.canonical-cbor.v1`.
