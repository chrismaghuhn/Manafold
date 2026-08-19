# State and Artifact Hashing

**Status:** accepted V0.2.1 digest contract
**Stability:** normative identity separation; full-state input v1 accepted

## Digest domains

The project uses distinct Rust types and domain tags:

| Digest | Includes | Excludes | Consumer |
|---|---|---|---|
| `FullStateDigest` | complete authoritative `EngineState` | derivable caches, environment status/counters | trusted kernel/replay |
| `PublicStateDigest` | information public to all current players | private knowledge/state | diagnostics/search keys |
| `InformationStateDigest(player)` | current observation plus retained knowledge for one perspective | unauthorized hidden state | agent datasets/search |
| `ObservationDigest(player)` | exact current observation bytes | historical knowledge not in observation | transport/tests |
| `CandidateSetDigest` | ordered visible candidates and decision constraints | authoritative bindings | soundness/dataset diagnostics |
| `ContentDigest` | canonical manifests/definitions/bundle closure | build-local paths/timestamps | certification |
| `ReplayDigest` | canonical replay manifest and steps | non-normative logs | provenance |
| `CheckpointDigest` | canonical checkpoint schema, full-state identity, episode status, limit counters, and codec identity | replay steps, logs, backend-local caches | trusted checkpoint storage |

Digests from different domains are never compared directly. The Rust types make accidental equality checks a compile-time mismatch.

## Full-state canonical input v1

`EngineState::digest()` is fallible and consumes `FullStateDigestInputV1`, not the internal `EngineState` serializer directly.

The input contains:

```text
schema_version = full-state-digest-input.v1
domain = mtgml.full-state-digest.v1
revision
core
canonical zones
allocators
execution
random
knowledge
perspective identities
format
```

`ZoneState.ordered_zones` has structured `ZoneKey`s and is represented as a deterministically ordered array of `{key, objects}` entries. All JSON object keys in the resulting DTO are recursively sorted before encoding. Empty and absent values retain distinct declared Serde representations.

A mandatory regression test hashes a valid state with a nonempty ordered zone. Tests limited to empty maps are insufficient.

## Domain separation

The SHA-256 preimage is:

```text
ASCII domain tag
0x00 separator
canonical bytes
```

The canonical DTO also carries its domain and input schema for diagnostics and migrations. A canonicalization or semantic-coverage change requires a new input schema/domain version.

## State identity versions

### V1 identity (historical)

`FullStateDigest` (V1) identifies `EngineState` including the legacy `random` component using placeholder RNG semantics (free-form algorithm/derivation strings, string stream names, u64 counters). There is no current-engine V1 producer; V1 exists only in historical fixtures and migration reference material.

`EnvironmentCheckpointV1::checkpoint_digest` is a typed `CheckpointDigest` over this canonical semantic input:

```text
schema_version = environment-checkpoint.v1
domain = mtgml.checkpoint-digest.v1
FullStateDigest
EpisodeStatus
EnvironmentLimitCounters
CheckpointCodecIdentity
```

### V2 identity (current)

`FullStateDigestV2` replaces `FullStateDigest` as the current full-state identity. It consumes `FullStateDigestInputV2`, which includes `EngineState` with typed RNG state (`mtgml.rng.v1` contract ID, `RootSeed256`, typed `RandomStreamKeyV1`/`RandomStreamCursorV1` stream map) instead of placeholder algorithm strings and string stream names.

`EnvironmentCheckpointV2` replaces `EnvironmentCheckpointV1`. The V1 checkpoint embeds an unversioned `EngineState` serializer whose canonical meaning can shift when the `random` component migrates from placeholder to typed RNG. V2 binds the full-state digest input version explicitly, so checkpoint identity cannot silently drift from a changed state representation.

```text
schema_version = environment-checkpoint.v2
domain = mtgml.checkpoint-digest.v2
FullStateDigestV2
EpisodeStatus
EnvironmentLimitCounters
CheckpointCodecIdentity
```

The complete checkpoint validator recomputes both the embedded `FullStateDigestV2` from `EngineState` and the `CheckpointDigest` from the remaining checkpoint identity. Altering status, limits, codec identity, or state identity without updating the corresponding digest is rejected. The in-memory semantic checkpoint contract is frozen by V0.2.1; a durable checkpoint wire codec remains a separate future contract.

## Security note

Digests provide content identity and divergence detection. They do not provide authenticity. Signed releases or attestations are a separate layer.
