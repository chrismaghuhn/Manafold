# Manafold Implementation Planning Task
# OD-008 Executable RNG Contract Migration — Revised Plan v2

**Current date:** August 2026  
**Repository:** `https://github.com/chrismaghuhn/Manafold`  
**Verified default branch:** `master`  
**Accepted RNG contract:** `mtgml.rng.v1`  
**Decision:** ADR 0035  
**Task type:** implementation planning only  
**Status:** `IN_PROGRESS`  
**Implementation status:** `NOT_RUN`  
**Performance status:** `NOT_RUN`

> This document supersedes the earlier OD-008 implementation plans.
>
> This revision tightens V1 preservation semantics. `EnvironmentCheckpointV1`
> directly embeds the unversioned current `EngineState`. Once
> `EngineState.random` migrates to `RandomStateV1`, keeping that runtime type
> executable would silently assign new state semantics to
> `environment-checkpoint.v1`. Manafold must **not** create a second legacy
> `EngineState` merely to keep V1 executable. The current runtime therefore
> moves to `EnvironmentCheckpointV2`; V1 meaning is retained through immutable
> documentation/domain/golden evidence, and the V1 runtime type is retired if
> it cannot remain truthful. The same rule applies to
> `FullStateDigestInputV1`: do not force a current-engine V1 producer after the
> authoritative state representation changes.
>
> This migration still does **not** create a durable checkpoint wire codec and
> does **not** resolve OD-017.
>
> **Implementation-base rule:** implementation starts from the then-current
> `master` branch, not from any SHA recorded during planning. The implementation
> agent must resolve and record the actual starting `master` HEAD before edits.

---

## A. Executive summary

Manafold should implement OD-008 as **one atomic cross-layer migration PR** before M1.1 production state construction begins.

The current repository still contains a placeholder randomness representation:

```text
algorithm_id: String
derivation_version: String
root_seed_hex: String
streams: BTreeMap<String, RandomStreamState>
```

while ADR 0035 and the registered normative RNG contract require:

```text
mtgml.rng.v1

RootSeed256([u8; 32])

RandomStateV1 {
    contract_id: MtgmlRngV1,
    root_seed: RootSeed256,
    streams: BTreeMap<RandomStreamKeyV1, RandomStreamCursorV1>,
}
```

with exact HMAC-SHA-256 stream derivation, counter-addressed raw blocks,
big-endian raw-word extraction, project-owned unbiased bounded sampling, and
descending Fisher-Yates.

The migration changes four coupled identity surfaces together:

1. authoritative RNG state and primitives;
2. full-state canonical digest identity;
3. trusted runtime checkpoint identity;
4. authoritative replay identity.

The required versioned result is:

```text
RNG contract
    mtgml.rng.v1

Full state
    historical identity/evidence:
        FullStateDigest / mtgml.full-state-digest.v1
        historical full-state-digest-input.v1 semantics
        no current-engine V1 producer required

    current runtime identity:
        FullStateDigestV2 / mtgml.full-state-digest.v2
        FullStateDigestInputV2

Checkpoint
    historical identity/evidence:
        environment-checkpoint.v1
        CheckpointDigest / mtgml.checkpoint-digest.v1
        historical CheckpointDigestInputV1 semantics
        EnvironmentCheckpointV1 runtime type retired if it embeds current EngineState

    current runtime identity:
        EnvironmentCheckpointV2
        CheckpointDigestV2 / mtgml.checkpoint-digest.v2
        CheckpointDigestInputV2

Replay
    historical wire DTO/reader compatibility:
        ReplayManifestV1
        ReplayStepV1
        AuthoritativeReplayV1

    current writer/runtime identity:
        ReplayManifestV2
        ReplayStepV2
        AuthoritativeReplayV2
```

The distinction between **historical semantic evidence** and **current executable
producer** is essential. Manafold must not retain V1 by constructing a second
legacy `EngineState`, a second legacy RNG authority, or a converter that makes
current typed state masquerade as historical V1 state.

`ReplayStepV2` is an **embedded semantic contract** in this migration. It does
not require a standalone `replay-step.v2.schema.json` unless a later explicit
contract decision makes replay steps independently serialized artifacts.

The replay-v2 schema update set must also update:

```text
schemas/README.json
```

so its `wire_contracts` index lists the two new V2 replay schema files while
preserving the V1 entries. It must not list a standalone replay-step V2 schema
or a checkpoint V2 schema because this migration creates neither.

The current legacy RNG event/delta scaffold uses free-form stream strings and a
generic `counter`. It must not remain as if it represented `mtgml.rng.v1`.
However, replacing it with the complete typed transition-level RNG event path
would implement M1.5 early. The migration should therefore remove or quarantine
the obsolete scaffold and leave the following evidence explicitly:

```text
typed RNG AuthoritativeRuleEvent integration = NOT_RUN
typed RNG SemanticDeltaOperation integration = NOT_RUN
typed RNG semantic-cursor progression        = NOT_RUN
accepted transition RNG evidence              = NOT_RUN
transition rejection RNG evidence             = NOT_RUN
DETERMINISTIC_RNG_AND_ALLOCATORS               = NOT_RUN
```

until M1.5 Issue #24.

Implementation must begin from **current `master` at implementation time**.
Do not check out, reset to, or otherwise use the old planning SHA as the coding
base. Record the actual starting HEAD in PR/review evidence after updating the
local `master` view.

This migration must precede M1.1 because M1.1 should construct/reset an
`EngineState` against an already executable, already versioned RNG/digest/
checkpoint/replay contract rather than defining those contracts incidentally.

---

## B. Verified repository baseline

### Repository baseline

The repository default branch is:

```text
master
```

**Do not use a planning commit SHA as the implementation base.** The
implementation agent must begin by resolving the then-current `master`:

```bash
git switch master
git pull --ff-only          # when the normal local/remote workflow permits it
git rev-parse HEAD
```

If the worktree or repository workflow makes `git pull --ff-only` inappropriate,
the equivalent requirement is still: establish the current authoritative
`master` HEAD before editing and record it in the implementation/PR evidence.

Before the first code edit, re-read on that current HEAD:

```text
docs/adr/0035-deterministic-hmac-sha256-counter-rng.md
docs/RNG_CONTRACT.md
docs/STATE_HASHING.md
docs/REPLAY_AND_DETERMINISM.md
crates/mtgml-random/src/lib.rs
crates/mtgml-state/src/lib.rs
crates/mtgml-environment/src/lib.rs
crates/mtgml-replay/src/lib.rs
schemas/README.json
```

If current `master` has materially changed any affected contract or ownership
since this planning pass, update the implementation plan/diff accordingly rather
than forcing the repository back to an older snapshot.

The accepted RNG authority remains ADR 0035 plus `docs/RNG_CONTRACT.md`, unless
current `master` contains an accepted superseding ADR/contract.

The attached RNG contract used for this planning pass was byte-identical to the
merged repository `docs/RNG_CONTRACT.md` at inspection time. That observation is
planning provenance, not a reason to pin implementation to the old commit.

### Current `mtgml-random`

Current state is placeholder vocabulary:

```rust
pub struct RandomStreamState {
    pub counter: u64,
}

pub struct RandomState {
    pub algorithm_id: String,
    pub derivation_version: String,
    pub root_seed_hex: String,
    pub streams: BTreeMap<String, RandomStreamState>,
}
```

Current local validation only checks:

- algorithm/derivation strings nonempty;
- seed text is 64 lowercase hex characters;
- stream names nonempty.

It does not implement the accepted RNG algorithm.

### Current dependency layout

Workspace currently pins:

```toml
sha2 = "=0.11.0"
```

and Rust:

```text
1.85.1
```

`mtgml-random` does not currently depend on `sha2`, `hmac`, or `mtgml-model`.

### Current EngineState

`EngineState` directly contains:

```text
random: RandomState
```

`validate_engine_state()` delegates RNG validation to the current placeholder
`RandomState::validate()`.

### Current full-state digest

`EngineState::digest()` currently uses:

```text
FullStateDigestInputV1
schema_version = full-state-digest-input.v1
domain = mtgml.full-state-digest.v1
```

and directly includes the current `RandomState`.

The Rust type:

```text
FullStateDigest
```

owns the v1 domain constant.

Changing RandomState's canonical meaning while retaining this input/domain
would reinterpret full-state-digest v1.

### Current checkpoint contract

The current pre-migration source defines a trusted in-memory checkpoint
approximately as:

```rust
EnvironmentCheckpointV1 {
    schema_version: String,
    state: EngineState,
    state_digest: FullStateDigest,
    status: EpisodeStatus,
    limit_counters: EnvironmentLimitCounters,
    codec: CheckpointCodecIdentity,
    checkpoint_digest: CheckpointDigest,
}
```

Its digest input is approximately:

```rust
CheckpointDigestInputV1 {
    schema_version,
    domain,
    state_digest: &FullStateDigest,
    status,
    limit_counters,
    codec,
}
```

with historical identities:

```text
schema_version = environment-checkpoint.v1
domain         = mtgml.checkpoint-digest.v1
state_digest   = FullStateDigest / mtgml.full-state-digest.v1
```

The critical compatibility fact is stronger than merely “the digest field is
V1”: **`EnvironmentCheckpointV1` directly contains the unversioned current
`EngineState` type.**

Once:

```text
EngineState.random
```

changes from the placeholder RNG representation to `RandomStateV1`, recompiling
that same `EnvironmentCheckpointV1` struct against the new `EngineState` would
silently change the semantic state contained by `environment-checkpoint.v1`.

Therefore the migration must not attempt to keep an executable
`EnvironmentCheckpointV1` over the migrated `EngineState`.

Forbidden approaches include:

```text
EnvironmentCheckpointV1 { state: new EngineState, ... }
legacy EngineStateV1 created solely to keep checkpoint V1 alive
current EngineState -> legacy EngineState/RNG adapter for V1 checkpoint output
```

Instead:

- current runtime checkpointing moves to `EnvironmentCheckpointV2`;
- the V1 runtime type/API is retired/removed if necessary;
- historical V1 meaning is preserved through immutable normative documentation,
  domain constants where useful, and hard-coded canonical/golden evidence;
- no current runtime path produces a V1 checkpoint.

### Current durable checkpoint status

No standalone durable checkpoint JSON Schema / shared canonical checkpoint
wire corpus is currently part of the public wire surface.

This migration therefore versions the **trusted in-memory semantic checkpoint
contract**, but does not introduce persistence.

OD-017 remains open.

### Current replay

Rust currently exposes:

```text
ReplayManifestV1
ReplayStepV1
AuthoritativeReplayV1
```

Replay-v1 randomness identity is:

```text
algorithm_id
derivation_version
root_seed_hex
```

Replay-v1 uses `FullStateDigest` for:

- initial state digest;
- per-step state digest;
- final state digest.

The authoritative replay v1 JSON Schema embeds the complete v1 manifest and
step shape inline.

### Replay-step status

There is currently no independently registered standalone replay-step schema in
the shared wire fixture mapping.

The manifest carries a `schemas.replay_step` identity, but the actual replay
step JSON structure is embedded in `authoritative-replay.v1`.

Therefore V2 should preserve that architectural shape:

```text
ReplayStepV2 = typed embedded semantic contract
```

not automatically create a standalone schema.

### Current Python replay/wire

Python mirrors replay v1 and canonical wire dispatch.

Python contains no authoritative RNG execution, which is the correct boundary.

### Current legacy RNG transition scaffold

Current state/rules code contains placeholder semantic vocabulary similar to:

```text
RandomnessConsumed {
    stream: String,
    counter_before,
    counter_after,
    exclusive_upper_bound,
    value,
}

RandomStreamAdvanced {
    stream: String,
    counter_before,
    counter_after,
    exclusive_upper_bound,
    value,
}
```

The semantic validation cursor tracks:

```text
BTreeMap<String, u64>
```

This is incompatible with the accepted typed stream/cursor contract.

### Player-facing surfaces

Current player APIs expose only:

```text
observation
information_state
visible_decision
submit
```

The player DTO path does not expose trusted checkpoints, authoritative replays,
root seeds, or current RNG state.

No player-facing schema change is required.

### Normative documentation drift

Several older normative documents still contain terminology such as:

```text
named RNG streams and counters
RNG counter advancement
algorithm / derivation version
stream name
```

ADR 0035 and `RNG_CONTRACT.md` supersede that semantic vocabulary for current
RNG meaning.

The migration should update these documents coherently instead of leaving two
current descriptions.

---

## C. Contract-to-code mapping

| Normative requirement | Current implementation | Required change | Owner |
|---|---|---|---|
| `mtgml.rng.v1` | free-form algorithm/derivation strings | closed RNG contract identity | `mtgml-random` |
| 256-bit root seed | authoritative text string | `RootSeed256([u8; 32])` | `mtgml-random` |
| trusted textual root | same internal representation | explicit canonical lower-hex codec only | `mtgml-random` |
| stream kind | free-form stream name | `RandomStreamKindV1` | `mtgml-random` |
| stream scope | absent | `Global`, `Player(PlayerId)` | `mtgml-random` |
| stream key | `String` | `RandomStreamKeyV1` | `mtgml-random` |
| canonical key bytes | absent | exact v1 binary encoder/decoder | `mtgml-random` |
| cursor | generic `counter` | `RandomStreamCursorV1 { next_raw_u64 }` | `mtgml-random` |
| runtime streams | `BTreeMap<String,...>` | `BTreeMap<RandomStreamKeyV1,...>` | `mtgml-random` |
| canonical stream state | incidental map representation | explicit sorted entry array | `mtgml-random` |
| duplicate stream decode | potential map-collapse semantics | reject before map construction | `mtgml-random` |
| stream derivation | absent | exact HMAC-SHA-256 preimage | `mtgml-random` |
| raw block | absent | counter-addressed HMAC block | `mtgml-random` |
| raw u64 | absent | four big-endian lanes | `mtgml-random` |
| bounded sampler | absent | Manafold-owned rejection sampler | `mtgml-random` |
| generic shuffle | absent | descending Fisher-Yates | `mtgml-random` |
| generic state validation | placeholder local strings | typed state + player-scope closure | `mtgml-state` |
| M1 stream plan | ad hoc test strings | deferred configuration policy | M1.1 |
| current full-state digest | v1 RandomState shape | V2 input/domain/newtype | `mtgml-model`, `mtgml-state` |
| historical full-state digest | current type/domain | preserve unchanged | `mtgml-model` |
| checkpoint V1 | embeds unversioned current EngineState and binds FullStateDigest v1 | retire executable runtime type rather than reinterpret; preserve historical identity/evidence | `mtgml-environment` / docs/tests |
| current checkpoint | V1 cannot survive current EngineState migration truthfully | `EnvironmentCheckpointV2` becomes sole current runtime checkpoint | `mtgml-environment` |
| current checkpoint digest | only v1 | `CheckpointDigestV2` | `mtgml-model` |
| current checkpoint input | only v1 | `CheckpointDigestInputV2` | `mtgml-environment` |
| durable checkpoint wire | not defined | remain deferred | OD-017 |
| replay v1 | old RNG identity + v1 digests | preserve historical | `mtgml-replay` |
| current replay | none | ReplayManifest/Step/Replay V2 | `mtgml-replay` |
| standalone replay-step schema | absent | do not add automatically | deferred |
| Python RNG execution | absent | remain absent | Python |
| legacy RNG event scaffold | string/counter | remove/quarantine; typed replacement deferred | M1.5 |

---

## D. Compatibility classification

Manafold versions digest domains, checkpoint semantics, replay,
Rust/Python wire contracts, and schemas independently. Existing values never
gain new meaning.

| Surface | Compatibility classification | Required treatment |
|---|---|---|
| placeholder `RandomState` | semantic break / migration-required internal state | replace with typed `RandomStateV1` |
| `FullStateDigest` v1 newtype/domain | historical frozen identity | preserve the domain/type if useful for historical artifacts/tests; never use it as current EngineState identity |
| `FullStateDigestInputV1` current-engine producer | cannot remain truthful after RandomState changes | retire/remove from current EngineState producer path; preserve historical canonical bytes/meaning as immutable evidence rather than adapting current state |
| current state digest | migration-required | add `FullStateDigestV2` + `FullStateDigestInputV2` |
| `EnvironmentCheckpointV1` runtime type | semantic break if compiled against migrated unversioned `EngineState` | retire/remove from current runtime API rather than reinterpret; do not add legacy EngineState |
| checkpoint V1 semantic identity | historical frozen meaning | preserve through docs/domain/golden evidence: `environment-checkpoint.v1` + FullStateDigest v1 + CheckpointDigest v1 |
| `CheckpointDigest` v1 domain | historical frozen identity | preserve `mtgml.checkpoint-digest.v1` as historical evidence/type if useful; no current checkpoint producer uses it |
| `CheckpointDigestInputV1` | historical canonical meaning | may remain only as a detached historical helper if it does not bind current EngineState; otherwise retire it and preserve golden canonical bytes/domain evidence |
| current checkpoint | migration-required | introduce `EnvironmentCheckpointV2` as the sole current runtime checkpoint contract |
| current checkpoint digest | migration-required | add `CheckpointDigestV2` |
| current checkpoint canonical input | migration-required | add `CheckpointDigestInputV2` |
| durable checkpoint codec | unresolved | do not introduce one; OD-017 remains open |
| Replay V1 DTO/schema/fixtures | historical provisional-public wire contract | preserve parsing/canonical wire evidence; current-engine execution of V1 semantics is not implied |
| Replay V2 | migration-required | add manifest + embedded step + outer replay V2; current writer/export uses V2 |
| Python replay DTO | reader-compatible additive migration | keep V1 wire DTO support, add V2; no authoritative RNG |
| JSON Schema | reader-compatible additive migration | keep V1, add replay V2 schemas, update `schemas/README.json` |
| shared fixtures | additive | preserve V1, add V2 |
| legacy RNG event scaffold | obsolete internal pre-OD-008 representation | remove/quarantine; do not reinterpret; replacement evidence remains M1.5 `NOT_RUN` |

### V1 preservation principle

“Preserve V1” does **not** mean “keep every old Rust runtime type constructible
against new internal state.” It means old identifiers and artifacts never gain
new semantics.

For checkpoint V1, retaining the old runtime struct after `EngineState` changes
would violate that principle because the type directly embeds `EngineState`.
The safer compatibility action is retirement:

```text
old meaning remains documented and test-pinned
old version identifier is never reused
current runtime only constructs V2
```

Do not create:

```text
LegacyEngineStateV1
LegacyRandomState solely for checkpoint preservation
current-to-legacy EngineState conversion
```

unless a future explicit migration requirement independently justifies such a
subsystem. Nothing in the current checkpoint persistence surface requires it.

### FullStateDigest V1 preservation principle

The same reasoning applies to `FullStateDigestInputV1` if its Rust producer is
structurally coupled to current `EngineState`/`RandomState` fields.

The migration must not make this true:

```text
FullStateDigestInputV1 {
    random: RandomStateV1,
    ...
}
```

if V1 historically meant the placeholder RNG representation.

Instead preserve V1 with immutable evidence such as:

- `mtgml.full-state-digest.v1` domain constant/newtype where useful;
- a hard-coded historical canonical input byte vector or fixture;
- its exact expected digest;
- normative documentation of the historical coverage.

Current `EngineState` produces only V2.

### Checkpoint V1 preservation principle

The following is forbidden:

```text
schema_version = environment-checkpoint.v1
state           = migrated current EngineState
state_digest    = FullStateDigestV2 or newly produced v1 digest over new state semantics
checkpoint domain = mtgml.checkpoint-digest.v1
```

The new current runtime contract is:

```text
EnvironmentCheckpointV2
    -> current EngineState with RandomStateV1
    -> FullStateDigestV2
    -> CheckpointDigestV2
```

Historical checkpoint-V1 identity remains evidence, not a current checkpoint
construction path.

---

## E. Proposed module/type ownership

### `mtgml-random`

Recommended layout:

```text
crates/mtgml-random/src/
├── lib.rs
├── types.rs
├── hmac_counter.rs
└── sampling.rs
```

`types.rs` owns:

```text
RootSeed256
MtgmlRngV1
RandomStreamKindV1
RandomStreamScopeV1
RandomStreamKeyV1
RandomStreamCursorV1
RandomStateV1

CanonicalRandomStreamKeyV1
CanonicalRandomStreamEntryV1
CanonicalRandomStateV1
```

It also owns:

- root-seed canonical lowercase hex encoding/decoding;
- exact binary stream-key encoding/decoding;
- canonical stream-entry conversion;
- duplicate canonical key rejection;
- typed local RNG decode/state errors.

### RootSeed256 API

Internal representation:

```rust
RootSeed256([u8; 32])
```

Recommended properties:

- inner bytes not publicly mutable;
- trusted `as_bytes()` accessor;
- `from_lower_hex()` / `to_lower_hex()`;
- no platform-integer interpretation;
- no implicit seed acquisition;
- avoid a normal `Debug` implementation that prints secret/trusted seed bytes.

### Canonical stream-key encoding

Exact binary authority:

```text
byte 0      codec version = 0x01
bytes 1..3  u16 BE kind
byte 3      scope tag
remaining   scope payload
```

Global:

```text
01 0001 00
=> 01000100
```

Player:

```text
01 0001 01 <u64_be(PlayerId)>
```

This codec is the authority for:

- stream derivation;
- persisted stream ordering.

It is not replaced by JSON field ordering or Rust `Ord`.

### Canonical RandomState representation

Runtime:

```text
BTreeMap<RandomStreamKeyV1, RandomStreamCursorV1>
```

Canonical DTO:

```json
{
  "contract_id": "mtgml.rng.v1",
  "root_seed_hex": "0000000000000000000000000000000000000000000000000000000000000000",
  "streams": [
    {
      "key": {
        "kind": "synthetic_m1",
        "scope": {
          "kind": "global"
        }
      },
      "next_raw_u64": 0
    }
  ]
}
```

Canonical ordering is:

```text
sort by canonical binary RandomStreamKeyV1 bytes
```

not:

- `BTreeMap` incidental `Ord`;
- insertion order;
- hash-map order;
- JSON object-key behavior.

### `hmac_counter.rs`

Owns:

```text
STREAM_DOMAIN = "mtgml.rng.stream-key.v1"
RAW_DOMAIN    = "mtgml.rng.raw-block.v1"

derive_stream_key(...)
raw_block(...)
raw_u64_at(...)
next_raw_u64(...)
```

No authoritative cache.

No block cache initially.

No process/global RNG object.

### `sampling.rs`

Owns:

```text
uniform_below_u64(...)
uniform_range_u64(...)
shuffle(...)
```

Do not use upstream generic `gen_range`, `Uniform`, `SliceRandom`, or
platform-sized random sampling.

### `mtgml-model`

Owns digest domain types.

Historical identity evidence:

```text
FullStateDigest
    DOMAIN = mtgml.full-state-digest.v1

CheckpointDigest
    DOMAIN = mtgml.checkpoint-digest.v1
```

These may remain as distinct Rust newtypes if useful for historical replay DTOs,
wire validation, or golden tests. Keeping the domain newtypes does not require a
current EngineState producer for them.

Add current identities:

```text
FullStateDigestV2
    DOMAIN = mtgml.full-state-digest.v2

CheckpointDigestV2
    DOMAIN = mtgml.checkpoint-digest.v2
```

Do not mutate the old domain constants.

### `mtgml-state`

Owns the current authoritative state and current state identity:

```text
EngineState.random: RandomStateV1
FullStateDigestInputV2
EngineState::digest() -> FullStateDigestV2
StateDelta.before_digest: FullStateDigestV2
StateDelta.after_digest: FullStateDigestV2
```

Generic validation owns cross-component RNG validity, especially Player-scope
references.

It does not own HMAC/sampling.

#### Historical V1 digest producer rule

Do **not** force `FullStateDigestInputV1` to remain a producer from the migrated
current `EngineState`.

If the existing V1 DTO directly references fields whose meanings change, in
particular the old `RandomState`, retire/remove that producer from the current
state path rather than changing its fields in place.

Preserve V1 meaning through detached immutable evidence:

```text
historical domain tag
historical input schema identifier
hard-coded canonical historical bytes or fixture
hard-coded expected digest
normative documentation
```

Do not introduce a second legacy EngineState or legacy RandomState simply to
regenerate V1 digests.

### `mtgml-environment`

Owns the **current trusted runtime checkpoint contract**.

Current semantic contract after migration:

```text
EnvironmentCheckpointV2
CheckpointDigestInputV2
ENVIRONMENT_CHECKPOINT_V2_SCHEMA = "environment-checkpoint.v2"
```

`EnvironmentCheckpointV2` conceptually contains:

```text
schema_version
state: EngineState
state_digest: FullStateDigestV2
status
limit_counters
codec
checkpoint_digest: CheckpointDigestV2
```

`CheckpointDigestInputV2` conceptually contains:

```text
schema_version = environment-checkpoint.v2
domain = mtgml.checkpoint-digest.v2
state_digest: FullStateDigestV2
status
limit_counters
codec
```

A separate `state_digest_domain` field is not required because the V2 checkpoint
contract itself fixes the field type to `FullStateDigestV2` and the checkpoint
schema/domain is new.

#### EnvironmentCheckpointV1 runtime rule

Because pre-migration `EnvironmentCheckpointV1` directly embeds the unversioned
`EngineState`, it cannot remain an executable historical V1 checkpoint type
after `EngineState.random` changes without changing V1 meaning.

Therefore:

- remove/retire `EnvironmentCheckpointV1` from current controller/runtime APIs;
- if the Rust type itself cannot exist without referencing current EngineState,
  remove the runtime type rather than compiling it against new state semantics;
- do not create `LegacyEngineStateV1`;
- do not create a current-to-legacy state adapter;
- do not emit new V1 checkpoints.

Historical V1 meaning is preserved through immutable documentation/domain/golden
evidence, not through a fake executable legacy engine.

### `mtgml-replay`

Historical:

```text
RandomnessIdentityV1
ReplayManifestV1
ReplayStepV1
AuthoritativeReplayV1
```

Current:

```text
RandomnessIdentityV2
ReplayManifestV2
ReplayStepV2
AuthoritativeReplayV2
```

V2 randomness identity:

```json
{
  "contract_id": "mtgml.rng.v1",
  "root_seed_hex": "<64 lowercase hex>"
}
```

### ReplayStepV2 ownership clarification

`ReplayStepV2` is a semantic Rust/Python type embedded in
`AuthoritativeReplayV2`.

The manifest may advertise:

```text
schemas.replay_step = "replay-step.v2"
```

but this migration should **not** create a standalone:

```text
schemas/replay-step.v2.schema.json
wire/golden/replay-step.v2.json
```

unless an explicit later requirement establishes a standalone serialized
replay-step artifact.

The authoritative replay v2 schema should inline the V2 step shape exactly as
v1 currently does.

### Python

Python adds only trusted replay codecs/DTOs needed for cross-language parity.

Python must not implement:

- HMAC;
- stream derivation;
- raw RNG;
- cursor mutation;
- sampler;
- shuffle;
- authoritative checkpoint semantics.

---

## F. File-by-file change plan

### Cargo/dependencies

#### `Cargo.toml`

**Current role:** workspace dependency authority.

**Change:**

```toml
hmac = "=0.13.0"
```

**Reason:** accepted HMAC-SHA-256 implementation primitive.

**Tests affected:** locked workspace dependency gates, mtgml-random KATs.

---

#### `Cargo.lock`

**Current role:** committed exact Rust dependency graph.

**Change:** regenerate once under pinned Rust after adding HMAC.

**Reason:** lock new dependency and transitive graph.

**Evidence:** inspect diff for expected additions only.

---

#### `crates/mtgml-random/Cargo.toml`

**Current role:** RNG crate dependency declaration.

**Change:**

```text
add mtgml-model
add hmac.workspace
add sha2.workspace
optionally add serde_json.workspace as dev dependency
```

**Reason:** PlayerId scope + cryptographic implementation + canonical tests.

---

### `mtgml-random`

#### `crates/mtgml-random/src/lib.rs`

**Current role:** entire placeholder RandomState implementation.

**Change:** convert to small module root/reexports.

**Reason:** avoid a new RNG god file.

---

#### NEW `crates/mtgml-random/src/types.rs`

**Planned contents:**

- RootSeed256;
- contract identity;
- stream kind/scope/key/cursor/state;
- canonical stream DTO;
- binary key codec;
- root-seed text codec;
- duplicate detection;
- local errors.

**Tests:**

- seed canonicalization;
- key codec KAT;
- malformed key negatives;
- canonical state ordering;
- duplicate rejection;
- roundtrip.

---

#### NEW `crates/mtgml-random/src/hmac_counter.rs`

**Planned contents:**

- HMAC stream derivation;
- HMAC raw blocks;
- BE lane extraction;
- raw cursor-consuming draw.

**Tests:**

- standard HMAC vectors;
- all normative RNG raw KATs;
- lane/block boundary;
- stream isolation;
- exhaustion.

---

#### NEW `crates/mtgml-random/src/sampling.rs`

**Planned contents:**

- `uniform_below_u64`;
- checked range helper;
- generic Fisher-Yates.

**Tests:**

- zero/one bounds;
- bound-10 KAT;
- forced rejection;
- invalid range;
- shuffle KAT.

---

### `mtgml-model`

#### `crates/mtgml-model/src/lib.rs`

**Current role:** shared IDs and digest newtypes.

**Change:**

Preserve:

```text
FullStateDigest
CheckpointDigest
```

Add:

```text
FullStateDigestV2
CheckpointDigestV2
```

with exact new domains.

**Reason:** old digest values must not receive new meaning.

**Tests:**

- v1 digest-domain tests unchanged;
- new v2 domain KAT;
- v1/v2 same canonical bytes hash differently;
- type-level mismatch remains impossible.

---

### `mtgml-state`

#### `crates/mtgml-state/src/lib.rs`

#### `crates/mtgml-state/src/lib.rs`

**Current role:** complete EngineState, full-state canonical adapter,
StateDelta, generic state validation.

**Change:**

1. replace old RandomState with `RandomStateV1`;
2. add:
   ```text
   FULL_STATE_DIGEST_INPUT_SCHEMA_V2 =
       "full-state-digest-input.v2"
   FullStateDigestInputV2
   ```
3. V2 random field uses explicit canonical stream-entry array;
4. current `EngineState::canonical_digest_bytes()` produces V2;
5. current `EngineState::digest()` returns `FullStateDigestV2`;
6. `StateDelta` uses `FullStateDigestV2`;
7. generic RNG validation checks Player scope references;
8. remove legacy `RandomStreamAdvanced` string/counter audit operation;
9. retire/remove the current-engine `FullStateDigestInputV1` producer if it
   would otherwise be redefined over `RandomStateV1`.

**Historical V1 evidence:**

Keep only the smallest truthful evidence necessary to prevent identity reuse,
such as the v1 domain/newtype plus hard-coded historical canonical bytes and
expected digest. Do not create a legacy EngineState or RandomState to keep a V1
producer alive.

**Do not add:**

- SyntheticM1/Global requirement;
- exact stream-count requirement;
- cursor-zero requirement;
- typed RNG event replacement.

**Tests:**

- valid typed RNG state;
- absent-player scope rejection;
- nonempty typed stream digest V2;
- insertion-order independence;
- root seed changes digest;
- cursor changes digest;
- exact V2 golden digest;
- historical v1 domain/golden evidence remains immutable without current-state production.

### `mtgml-rules`

#### `crates/mtgml-rules/src/lib.rs`

**Current role:** authoritative events and sequential semantic validation.

**Change:**

- migrate test EngineStates to typed RNG;
- remove old string/counter `RandomnessConsumed` event path;
- remove old RNG semantic cursor map;
- remove old `two_rng_uses_of_one_stream_are_compositional` test;
- do not implement the typed replacement in this PR.

**Reason:** old event representation contradicts OD-008; typed event integration
belongs to M1.5.

**Required status after migration:**

```text
transition-level RNG event/delta evidence = NOT_RUN
```

---

### `mtgml-replay`

#### `crates/mtgml-replay/src/lib.rs`

**Current role:** replay-v1 domain and validation.

**Change:**

Preserve V1 exactly.

Add:

```text
RandomnessIdentityV2
ReplayManifestV2
ReplayStepV2
AuthoritativeReplayV2
```

V2 uses:

- RNG contract ID;
- canonical root seed;
- FullStateDigestV2 initial/step/final identity.

**Validation:**

- exact schema version;
- exact supported RNG contract;
- canonical seed;
- deck uniqueness;
- revision continuity;
- rejection identity nonmutation;
- final identity.

**Tests:**

- V1 still valid;
- V2 valid;
- mixed versions reject;
- unsupported RNG contract reject;
- V2 rejection digest mutation reject.

---

### `mtgml-wire`

#### `crates/mtgml-wire/src/lib.rs`

**Current role:** canonical Rust wire serializer/reader and fixture dispatcher.

**Change:**

Add WireContract implementations for:

```text
ReplayManifestV2
AuthoritativeReplayV2
```

Add named fixture dispatch:

```text
replay-manifest.v2
authoritative-replay.v2
```

Do not add standalone replay-step dispatch.

**Tests:** shared golden/negative directory tests automatically cover both
versions once manifests are updated.

---

### `mtgml-environment`

#### `crates/mtgml-environment/src/lib.rs`

#### `crates/mtgml-environment/src/lib.rs`

**Current role:** trusted checkpoint/controller and player endpoint.

**Change:**

1. add:
   ```text
   ENVIRONMENT_CHECKPOINT_SCHEMA_V2 =
       "environment-checkpoint.v2"

   EnvironmentCheckpointV2
   CheckpointDigestInputV2
   ```
2. current controller/backend checkpoint API moves to:
   ```text
   checkpoint() -> EnvironmentCheckpointV2
   restore(EnvironmentCheckpointV2)
   ```
3. current replay export returns `AuthoritativeReplayV2`;
4. V2 validation recomputes `FullStateDigestV2` and `CheckpointDigestV2`;
5. retire/remove `EnvironmentCheckpointV1` from current runtime if it directly
   embeds the migrated `EngineState`;
6. do not keep a V1 constructor/restore path by creating legacy state types.

**Historical V1 preservation:**

Preserve the immutable meaning of:

```text
environment-checkpoint.v1
mtgml.checkpoint-digest.v1
FullStateDigest v1
```

through normative documentation and hard-coded historical canonical/domain
evidence. `CheckpointDigestInputV1` may remain only if it is a detached,
truthful historical helper that does not require current `EngineState`; it is
not a current producer.

**Tests:**

- V2 valid checkpoint;
- V2 state digest mismatch;
- V2 checkpoint digest mismatch;
- V2 status/limits/codec coverage;
- V1 historical IDs/domain golden remains unchanged;
- no runtime V1 construction from migrated EngineState;
- V1 and V2 checkpoint domain identities cannot be confused.

**Do not add:**

- checkpoint wire schema;
- checkpoint Python DTO;
- persisted checkpoint file format;
- legacy second EngineState.

### `mtgml-conformance`

#### `crates/mtgml-conformance/src/lib.rs`

**Current role:** exact expected transition identity.

**Change:**

Current `expected_state_digest` becomes `FullStateDigestV2`.

No RNG event evidence added here yet.

---

### Python

#### `python/src/mtgml/replay.py`

**Current role:** replay-v1 DTO/codec.

**Change:**

Preserve V1 classes.

Add:

```text
RandomnessIdentityV2
ReplayManifestV2
ReplayStepV2
AuthoritativeReplayV2
```

No RNG execution.

---

#### `python/src/mtgml/wire.py`

Add decoder registrations:

```text
replay-manifest.v2
authoritative-replay.v2
```

No standalone replay-step decoder.

---

#### `python/src/mtgml/__init__.py`

Export V2 replay DTOs alongside V1.

---

#### `python/tests/test_schema_parity.py`

Add V2 replay schema mapping.

Add exact assertions that:

- `replay-manifest.v2` has V2 RNG identity;
- `authoritative-replay.v2` embeds V2 step shape;
- no standalone replay-step schema is assumed.

---

#### `python/tests/test_wire_contracts.py`

Keep dynamic shared-corpus roundtrip and negative rejection.

Add explicit coexistence assertions if necessary.

---

#### `python/tests/test_player_api.py`

Extend forbidden player fields/names to cover:

```text
root_seed
root_seed_hex
stream_key
next_raw_u64
raw_u64
checkpoint
authoritative_replay
```

---

### Schemas

#### NEW `schemas/replay-manifest.v2.schema.json`

V2 randomness:

```json
{
  "contract_id": {
    "const": "mtgml.rng.v1"
  },
  "root_seed_hex": {
    "type": "string",
    "pattern": "^[0-9a-f]{64}$"
  }
}
```

No algorithm/derivation strings.

Initial state digest remains a 64-lowercase-hex field but is semantically
`FullStateDigestV2`.

---

#### NEW `schemas/authoritative-replay.v2.schema.json`

Contains:

- schema version `authoritative-replay.v2`;
- complete ReplayManifestV2 shape following the current repository schema style;
- inline ReplayStepV2 shape;
- final FullStateDigestV2 textual representation.

Do not create standalone replay-step-v2 schema by default.

---

#### `schemas/README.json`

**Current role:** machine-readable index grouping maintainer schemas and wire
contract schemas.

**Change:** add:

```text
authoritative-replay.v2.schema.json
replay-manifest.v2.schema.json
```

to `wire_contracts`, preserving all V1 entries and the repository's existing
ordering convention.

Do **not** add:

```text
replay-step.v2.schema.json
environment-checkpoint.v2.schema.json
```

because neither standalone wire schema is introduced by this migration.

**Test/tool impact:** `scripts/validate_schemas.py`, repository verifier, and any
schema-index consistency test must recognize the updated index.

### Shared fixtures

#### `wire/golden/manifest.json`

Add:

```text
replay-manifest.v2
authoritative-replay.v2
```

while retaining all V1 entries.

---

#### NEW `wire/golden/replay-manifest.v2.json`

Canonical v2 manifest using:

```text
contract_id = mtgml.rng.v1
root_seed_hex = canonical lowercase
schemas.replay_step = replay-step.v2
```

---

#### NEW `wire/golden/authoritative-replay-empty.v2.json`

Canonical empty V2 replay.

---

#### `wire/negative/manifest.json`

Add V2 negatives.

---

#### NEW V2 negative fixture set

At minimum:

```text
replay-v2-unknown-rng-contract.json
replay-v2-root-seed-uppercase.json
replay-v2-root-seed-wrong-length.json
replay-v2-root-seed-nonhex.json
replay-v2-v1-manifest-mismatch.json
replay-v1-v2-manifest-mismatch.json
replay-v2-rejected-digest-mutation.json
replay-v2-final-digest-mismatch.json
```

Add wrong embedded `replay-step` semantic version if validator owns that
cross-field invariant.

---

### Verification scripts

#### `scripts/validate_schemas.py`

Add V2 replay contract mappings.

Do not add checkpoint schema mapping.

Do not add replay-step schema mapping.

---

#### `scripts/verify_repository.py`

Remove/replace literal checks that currently require:

```text
algorithm_id
derivation_version
old RNG compositional-event test
```

Add structural requirements for:

```text
RootSeed256
RandomStreamKeyV1
RandomStateV1
FullStateDigestV2
EnvironmentCheckpointV2
CheckpointDigestV2
ReplayManifestV2
AuthoritativeReplayV2
normative KAT test names
```

Preserve checks proving V1 replay fixture support remains.

Do not make the verifier claim M1.5 RNG-event support.

---

### Normative/documentation updates

#### `docs/RNG_CONTRACT.md`

Do not alter the accepted byte contract.

After executable implementation exists, change only implementation-status text
as appropriate.

---

#### `docs/STATE_HASHING.md`

Update to document:

Historical identity/evidence:

```text
FullStateDigest v1
environment-checkpoint.v1 historical meaning
CheckpointDigest v1
(no current EngineState-backed V1 producer required)
```

Current:

```text
FullStateDigestV2
EnvironmentCheckpointV2
CheckpointDigestV2
```

Explain explicitly why checkpoint V2 is required.

---

#### `docs/REPLAY_AND_DETERMINISM.md`

Replace legacy algorithm/derivation/named-counter wording with current typed
RNG contract terminology.

Document replay V1 historical + replay V2 current.

Document checkpoint V1 historical + checkpoint V2 current.

---

#### `docs/DOMAIN_MODEL.md`

Replace:

```text
named RNG streams and counters
```

with typed RNG stream keys and raw-word cursors.

---

#### `docs/EXECUTION_MODEL.md`

Update rejection preservation terminology to:

```text
RNG contract ID
root seed
stream-key set
stream cursors
```

Document current trusted checkpoint as V2 after migration, while retaining V1
as historical semantics.

---

#### `docs/INFORMATION_MODEL.md`

Replace legacy protected:

```text
stream name
counter
```

with:

```text
root seed
typed stream key
stream cursor
derived stream key
raw word
trusted RNG audit data
```

---

#### `docs/RULES_SEMANTICS.md`

Remove implication that the legacy counter event is the current executable RNG
event contract.

State that typed RNG semantic event/delta integration is required by the RNG
contract but remains M1.5 work.

---

#### `docs/contracts/SEMANTIC_CONTRACT.md`

Replace `seed/stream counters` shorthand with typed RNG state/cursors.

---

#### `docs/ML_TRAJECTORIES.md`

Strengthen forbidden-content wording to cover root seed, typed stream keys,
cursors, raw words, and trusted RNG audit state.

---

#### `docs/maintenance/API_LIFECYCLE.md`

Record replay v2 as current provisional-public alongside retained v1 reader
support.

Checkpoint V2 is a trusted semantic contract, not a newly introduced public
wire schema.

---

## G. Dependency changes

Add exact workspace dependency:

```toml
hmac = "=0.13.0"
```

Existing SHA-256 remains:

```toml
sha2 = "=0.11.0"
```

No `rand`.

No `rand_core`.

No generic randomness framework.

No new hex crate is required; a small canonical fixed-format hex codec is
sufficient and keeps malformed-input semantics project-owned.

### Lockfile update

Implementation agent should:

1. make Cargo manifest changes;
2. run one Cargo resolver operation without `--locked` under Rust 1.85.1;
3. inspect `Cargo.lock`;
4. verify expected HMAC-related additions;
5. reject unexplained unrelated dependency churn;
6. commit `Cargo.lock`;
7. use `--locked` for all later verification.

### Dependency policy implication

HMAC/SHA crate versions are build provenance.

They are not allowed to redefine:

```text
mtgml.rng.v1
```

A future dependency upgrade is accepted under the same RNG contract only if
every required KAT, digest, checkpoint, replay, and sampling/permutation value
remains byte-identical.

---

## H. Implementation sequence

### Phase 0 — preserve V1 evidence and add red V2 evidence

Before changing producer behavior:

- synchronize to current `master` and record the actual starting HEAD;
- preserve existing replay V1 fixture bytes;
- capture/preserve FullStateDigest-v1 historical domain + canonical golden evidence;
- capture/preserve checkpoint-v1 historical schema/domain/canonical evidence;
- explicitly identify `EnvironmentCheckpointV1` and current-engine
  `FullStateDigestInputV1` as retirement candidates rather than compatibility
  producers;
- add RNG KAT tests as red tests;
- add V2 replay schemas/fixtures/index updates as red cross-language evidence;
- add checkpoint-v2 identity tests.

**Success condition:**

```text
historical V1 meaning is pinned without a legacy second EngineState
new V2 identities are explicit before current producer migration
implementation base is actual current master HEAD
```

### Phase 1 — typed RNG values and canonical state

Implement:

```text
RootSeed256
MtgmlRngV1
RandomStreamKindV1
RandomStreamScopeV1
RandomStreamKeyV1
RandomStreamCursorV1
RandomStateV1
```

Implement canonical seed/key/state codecs.

**Success condition:**

No authoritative free-form stream name remains in current RNG state.

---

### Phase 2 — HMAC raw generator and cursor

Implement exact:

```text
root seed
-> canonical stream bytes
-> HMAC stream key
-> HMAC block(counter)
-> BE u64 lane
```

Implement consuming raw cursor logic.

**Success condition:**

Every normative raw KAT passes.

---

### Phase 3 — bounded sampler and permutation

Implement:

```text
uniform_below_u64
checked half-open u64 range
descending Fisher-Yates
```

**Success condition:**

Normative bounded sample and shuffle outputs match exactly and cursor counts
are exact.

---

### Phase 4 — EngineState typed RNG integration

Change current EngineState to `RandomStateV1`.

Generic validator checks:

- state contract shape;
- typed stream keys;
- Player scope closure.

Generic validator does not encode reset policy.

Remove old RNG delta operation.

**Success condition:**

Current state has one typed RNG authority and no M1-specific stream plan.

---

### Phase 5 — FullStateDigest V2

Add:

```text
FullStateDigestV2
FullStateDigestInputV2
full-state-digest-input.v2
mtgml.full-state-digest.v2
```

Current EngineState/StateDelta/conformance use V2.

Do not adapt `FullStateDigestInputV1` to current `RandomStateV1`. If the existing
V1 producer cannot remain semantically frozen after the state change, retire it
from the current code path and preserve V1 identity with detached immutable
domain/canonical golden evidence.

**Success condition:**

```text
exact nonempty typed-stream state digest V2 golden passes
current EngineState has no V1 digest producer
historical V1 identity has not gained new semantics
```

### Phase 6 — EnvironmentCheckpoint V2

Add:

```text
CheckpointDigestV2
EnvironmentCheckpointV2
CheckpointDigestInputV2
environment-checkpoint.v2
mtgml.checkpoint-digest.v2
```

Current controller APIs:

```text
checkpoint() -> EnvironmentCheckpointV2
restore(EnvironmentCheckpointV2)
```

Retire/remove `EnvironmentCheckpointV1` as an executable current-state contract
if it directly contains the migrated `EngineState`.

Do not preserve V1 runtime execution by introducing a second legacy EngineState,
legacy RNG state, or current-to-legacy converter.

Preserve V1 meaning using immutable docs/domain/golden evidence only.

Do not add a durable checkpoint wire codec.

**Success condition:**

```text
checkpoint V2 binds FullStateDigestV2 under a new checkpoint schema/domain
no current runtime path constructs environment-checkpoint.v1
V1 historical meaning is evidence-only and unchanged
```

### Phase 7 — Replay V2 cross-language

Add Rust/Python/schema/fixture support for:

```text
ReplayManifestV2
ReplayStepV2
AuthoritativeReplayV2
```

Keep ReplayStepV2 embedded.

**Success condition:**

V1 and V2 readers are distinct, shared V2 bytes roundtrip in Rust/Python, and
mixed versions fail closed.

---

### Phase 8 — legacy RNG transition scaffold removal

Remove pre-OD-008 string/counter RNG semantic event/delta/cursor scaffold.

Do not add the M1.5 typed replacement.

**Success condition:**

No executable event type claims superseded RNG semantics.

Explicit status:

```text
transition-level RNG event/delta evidence = NOT_RUN
```

---

### Phase 9 — documentation and verifier coherence

Update normative docs and structural verification assumptions.

**Success condition:**

No current normative source or verifier requires the placeholder RNG contract.

---

### Phase 10 — broad verification

Run §N.

**Success condition:**

Only actually executed successful checks are reported `PASS`.

M1 milestone gates remain `NOT_RUN` where their full required evidence belongs
to later issues.

---

## I. Test and fixture plan

### HMAC primitive confidence

Hard-code independent HMAC-SHA-256 standard vectors, such as RFC 4231 vectors.

Do not compute expected output using the implementation under test.

### Normative OD-008 base KAT

Hard-code:

```text
root_seed =
0000000000000000000000000000000000000000000000000000000000000000

stream =
SyntheticM1 / Global

canonical stream bytes =
01000100

stream derivation message =
6d74676d6c2e726e672e73747265616d2d6b65792e7631000000000401000100

K_stream =
73635feaa9e90effe337e2cc9e1d801f63c9ede8d51b21a1120e624da2d648f9

block 0 =
6818e6bd053d9b770e26253e8d724b0403c524aeb6b3cff52508069342e336e4

words 0..3 =
0x6818e6bd053d9b77
0x0e26253e8d724b04
0x03c524aeb6b3cff5
0x2508069342e336e4

block 1 =
ac6a5d827f0dcbbf060d1adce197e55569da50c9030d2a2b2a7f637923566d45

words 4..7 =
0xac6a5d827f0dcbbf
0x060d1adce197e555
0x69da50c9030d2a2b
0x2a7f637923566d45
```

### Root seed tests

Positive:

- all-zero 32-byte seed;
- arbitrary 32 bytes;
- lower-hex roundtrip.

Negative trusted text:

- length 63;
- length 65;
- uppercase;
- prefix `0x`;
- whitespace;
- nonhex;
- trailing newline.

### Stream-key tests

Hard-code Global key:

```text
01000100
```

Add Player(P1) exact key bytes.

Reject:

- version 0;
- unknown version;
- reserved kind 0;
- unknown kind;
- unknown scope tag;
- truncated player payload;
- trailing bytes.

### Cursor tests

- zero initialization;
- one successful raw draw increments once;
- lane 3 -> next block lane 0;
- `u64::MAX - 1` exact KAT;
- result cursor becomes `u64::MAX`;
- next draw errors;
- no mutation on MAX error;
- no mutation on missing-stream error.

Important:

```text
u64::MAX is structurally valid stored state
```

It represents an exhausted stream.

### Cursor boundary KAT

Hard-code:

```text
input cursor = u64::MAX - 1
block index  = 0x3fffffffffffffff
lane         = 2
raw word     = 0x021a6c120112e7b3
result cursor = u64::MAX
```

### Stream isolation

Create at least two typed streams.

Consume one.

Assert the other cursor and entire unrelated state are unchanged.

### Sampling tests

#### n = 0

```text
InvalidRandomBound
zero consumed words
```

#### n = 1

```text
result 0
zero consumed words
```

#### normative n = 10

```text
result = 7
consumed = 1
```

#### forced rejection

Stub raw words:

```text
[0, 6]
```

bound:

```text
10
```

Expected:

```text
reject 0
accept 6
return 6
consume exactly 2 raw words
```

The raw-source test seam remains private to mtgml-random tests.

### Checked range tests

Invalid range validates fully before RNG consumption.

No direct `usize` sampling.

### Shuffle tests

Normative:

```text
input  = [0, 1, 2, 3, 4]
output = [1, 3, 4, 0, 2]
raw words consumed = 4
```

Length 0 and 1 consume zero.

### Canonical RandomState tests

Prove:

- two insertion orders -> identical canonical bytes;
- canonical stream-key bytes determine ordering;
- duplicate entries reject before map construction;
- unknown/malformed keys reject;
- roundtrip preserves semantic state.

### EngineState validation

Positive:

- valid Global stream;
- valid Player(P1) stream;
- several valid streams;
- exhausted cursor MAX.

Negative:

- Player(P3) with no P3 in EngineState.

Do not add:

```text
requires SyntheticM1/Global
```

to generic validator tests.

### FullStateDigest V2 tests

Use a valid state with a nonempty typed stream map.

Hard-code exact V2 canonical bytes and exact V2 digest.

Prove:

- root seed changes full-state digest;
- cursor changes full-state digest;
- insertion order does not;
- canonical key-byte ordering controls representation;
- old V1 digest strings are not interpreted as V2;
- current `EngineState` has no V1 digest-production path after migration.

### Historical FullStateDigest V1 evidence

Do not require `FullStateDigestInputV1` to construct a digest from the migrated
current `EngineState`.

Preserve V1 meaning with a detached immutable vector such as:

```text
domain = mtgml.full-state-digest.v1
historical canonical input bytes = <hard-coded fixture/vector>
expected SHA-256 digest = <hard-coded value>
```

The expected bytes/value must come from preserved historical evidence, not be
regenerated by serializing `RandomStateV1` through a renamed V1 DTO.

If current repository tests do not already preserve adequate V1 canonical bytes,
capture the pre-migration V1 golden before deleting/retiring the producer in the
same atomic PR.

### Checkpoint V1 preservation tests

The goal is **historical semantic immutability**, not executable V1
checkpoint construction over the migrated EngineState.

Before retiring the runtime V1 path, pin immutable evidence for:

```text
environment-checkpoint.v1
mtgml.checkpoint-digest.v1
FullStateDigest v1
historical CheckpointDigestInputV1 canonical bytes
historical expected CheckpointDigest v1
```

After `EngineState.random` migrates:

- do not construct `EnvironmentCheckpointV1` with current `EngineState`;
- do not add `LegacyEngineStateV1`;
- do not add `LegacyRandomState` solely for checkpoint support;
- do not add a current-state-to-V1 checkpoint conversion;
- remove/retire the runtime V1 type/API if necessary.

If a detached `CheckpointDigestInputV1` helper can remain without referencing
current EngineState semantics, it may remain for historical fixture validation.
It is not a current checkpoint producer.

### Checkpoint V2 tests

At minimum:

- valid V2 checkpoint;
- exact `CheckpointDigestV2` golden;
- state/root change without state digest update rejects;
- stream cursor change without state digest update rejects;
- state digest change without checkpoint digest update rejects;
- status change without checkpoint digest update rejects;
- limit change without checkpoint digest update rejects;
- codec change without checkpoint digest update rejects;
- v1 checkpoint identity is rejected as v2;
- v2 checkpoint identity is rejected as v1;
- checkpoint V1 and V2 digest domains cannot be confused.

### Replay V1 preservation

Existing replay-v1 schemas and golden fixtures remain byte-for-byte unchanged.

Existing V1 Rust/Python readers remain.

### Replay V2 positive

Add exact shared canonical fixtures for:

```text
replay-manifest.v2
authoritative-replay.v2
```

Both Rust and Python:

```text
decode -> encode -> exact same bytes
```

### ReplayStep V2 test

Assert:

```text
ReplayStepV2
```

is the embedded step type used by AuthoritativeReplayV2.

Assert manifest semantic identity:

```text
schemas.replay_step = "replay-step.v2"
```

if that field remains required.

Do not require a standalone replay-step schema file.

### Replay V2 negatives

- unknown RNG contract;
- malformed seed;
- uppercase seed;
- wrong-length seed;
- V1 outer/V2 manifest mismatch;
- V2 outer/V1 manifest mismatch;
- rejected step changes digest;
- final digest mismatch;
- wrong replay-step semantic version if cross-field validated.

### Player leak tests

Player API must remain disjoint from:

```text
root_seed
root_seed_hex
RandomStreamKeyV1
RandomStreamCursorV1
next_raw_u64
raw_u64
derived stream key
checkpoint
authoritative replay
trusted RNG event
```

### Transition-level RNG evidence status

Removing the obsolete scaffold does **not** satisfy the future event contract.

After this migration:

```text
RNG primitive KATs                           executable migration evidence
RNG state/digest/checkpoint/replay identity executable migration evidence

typed AuthoritativeRuleEvent RNG path       NOT_RUN
typed SemanticDeltaOperation RNG path       NOT_RUN
semantic-event cursor RNG composition       NOT_RUN
accepted RNG transition                     NOT_RUN
rejected transition RNG nonmutation         NOT_RUN
DETERMINISTIC_RNG_AND_ALLOCATORS             NOT_RUN
```

M1.5 owns those.

---

## J. Replay/digest/checkpoint migration plan

### J.1 FullStateDigest

Historical identity:

```text
FullStateDigest
mtgml.full-state-digest.v1
full-state-digest-input.v1 historical canonical semantics
```

Current identity:

```text
FullStateDigestV2
mtgml.full-state-digest.v2
full-state-digest-input.v2
```

Current `EngineState::digest()` returns only V2.

Do not change the V1 domain constant.

Do not reinterpret historical digest strings as V2 simply because both are
64 lowercase hexadecimal characters.

#### No current-engine V1 producer requirement

The migration must not preserve `FullStateDigestInputV1` by changing its
`random` field from the placeholder RNG shape to `RandomStateV1`. That would
make the V1 input schema mean something new.

If `FullStateDigestInputV1` is structurally coupled to current `EngineState`,
retire/remove it from executable current-state production.

V1 is preserved with detached immutable evidence:

```text
historical domain
historical input-schema ID
historical canonical bytes/fixture
historical expected digest
```

Do not create a second legacy EngineState or RNG state to reproduce V1 from
current state.

### J.2 EnvironmentCheckpoint V1 retirement and V2 migration

#### Why runtime V1 cannot survive unchanged

Pre-migration `EnvironmentCheckpointV1` directly contains:

```text
state: EngineState
```

`EngineState` is not itself versioned as `EngineStateV1` inside that checkpoint.
When `EngineState.random` changes to `RandomStateV1`, the Rust field name and
outer checkpoint version could remain identical while the semantic content of
`state` changes.

Therefore keeping this runtime type:

```text
EnvironmentCheckpointV1 {
    state: migrated EngineState,
    ...
}
```

would silently reinterpret `environment-checkpoint.v1` even before considering
the state-digest field.

The earlier idea of retaining an executable historical V1 type is therefore
rejected.

#### Forbidden compatibility mechanisms

Do not create:

```text
LegacyEngineStateV1
EngineStateV1 solely for checkpoint history
LegacyRandomState solely for checkpoint history
RandomStateV1 -> old RandomState conversion
current EngineState -> EnvironmentCheckpointV1 conversion
```

There is no current durable checkpoint wire surface that justifies maintaining
a second semantic state implementation.

#### Historical V1 preservation

Preserve V1 meaning through immutable evidence:

```text
schema identity:
    environment-checkpoint.v1

historical state identity:
    FullStateDigest / mtgml.full-state-digest.v1

checkpoint digest identity:
    CheckpointDigest / mtgml.checkpoint-digest.v1

historical canonical checkpoint-digest input bytes
historical expected CheckpointDigest v1
normative documentation
```

The V1 runtime struct may be removed/retired if keeping it would bind it to the
new EngineState.

Dead compatibility code should not be retained merely as a memorial. If a V1
Rust newtype/constant is useful for replay DTO parsing or golden tests, keep it;
otherwise immutable docs/fixtures may carry the historical identity.

#### Current V2 runtime

Introduce:

```text
EnvironmentCheckpointV2
CheckpointDigestV2
CheckpointDigestInputV2
```

Identities:

```text
EnvironmentCheckpointV2.schema_version =
    "environment-checkpoint.v2"

CheckpointDigestV2::DOMAIN =
    "mtgml.checkpoint-digest.v2"
```

Canonical V2 input:

```text
schema_version = environment-checkpoint.v2
domain         = mtgml.checkpoint-digest.v2
state_digest   = FullStateDigestV2
status
limit_counters
codec
```

`EnvironmentCheckpointV2::new()`:

1. validates the current EngineState;
2. computes `FullStateDigestV2`;
3. computes `CheckpointDigestV2`;
4. constructs the V2 checkpoint;
5. validates the complete V2 checkpoint before returning.

`EnvironmentCheckpointV2::validate()`:

1. checks V2 checkpoint identity/codec;
2. validates current EngineState;
3. recomputes `FullStateDigestV2`;
4. compares it with `state_digest`;
5. recomputes `CheckpointDigestV2`;
6. compares it with `checkpoint_digest`;
7. validates EpisodeStatus;
8. validates limit counters;
9. validates completed-state/current-decision relationship.

Current trusted controller/backend signatures migrate to V2:

```text
checkpoint() -> EnvironmentCheckpointV2
restore(EnvironmentCheckpointV2)
```

No V1 fallback is implicit.

#### No durable checkpoint wire

This migration does not introduce:

- checkpoint JSON Schema;
- checkpoint shared wire fixture;
- Python checkpoint DTO;
- checkpoint file codec;
- persisted EngineState codec;
- checkpoint migration file format.

Therefore OD-017 remains open.

### J.3 Replay migration

Add:

```text
ReplayManifestV2
ReplayStepV2
AuthoritativeReplayV2
```

Replay V2 randomness identity:

```text
{
    contract_id: "mtgml.rng.v1",
    root_seed_hex: "<64 lowercase hex>"
}
```

Replay V2 initial/step/final state identities use `FullStateDigestV2`.

Replay V1 DTOs, schemas, fixtures, and canonical readers remain historical wire
compatibility. Their continued parse/roundtrip support does **not** require the
current engine to construct historical `EngineState` or execute V1 RNG semantics.

Current replay writer/export emits V2.

No automatic V1-to-V2 replay translation/execution is required.

### J.4 ReplayStepV2 status

`ReplayStepV2` is an **embedded semantic contract**.

The correct current design is:

```text
ReplayManifestV2.schemas.replay_step = "replay-step.v2"

AuthoritativeReplayV2.steps:
    Vec<ReplayStepV2>

authoritative-replay.v2.schema.json:
    embeds ReplayStepV2 shape inline
```

Do not create a standalone:

```text
replay-step.v2.schema.json
```

unless a later explicit contract decision makes replay steps independently
serialized public artifacts.

### J.5 Schema index update

Update:

```text
schemas/README.json
```

under `wire_contracts` to include:

```text
authoritative-replay.v2.schema.json
replay-manifest.v2.schema.json
```

while retaining their V1 counterparts.

Do not add a standalone replay-step V2 schema or EnvironmentCheckpointV2 schema
to that index because neither is introduced.

### J.6 Checkpoint/replay independence

Replay and checkpoint remain distinct version surfaces:

```text
Replay V2
    identifies reset RNG contract/root seed and V2 state digests

EnvironmentCheckpointV2
    contains complete current EngineState, V2 state digest,
    status, counters, codec, V2 checkpoint digest
```

A future replay that resumes from a non-reset persisted checkpoint still
requires the future explicitly versioned persisted checkpoint/state mechanism.
This migration does not solve that persistence problem.

---

## K. Information-safety review

Protected values include:

```text
RootSeed256
root seed text in trusted replay/checkpoint paths
RandomStreamKeyV1
RandomStreamScopeV1 where purpose is sensitive
RandomStreamCursorV1
derived HMAC stream key
raw HMAC blocks
raw u64 values
trusted RNG audit/event data
```

Allowed:

- trusted kernel;
- trusted state;
- trusted controller;
- EnvironmentCheckpointV2;
- authoritative replay;
- conformance/debug tooling.

Forbidden:

- PlayerEndpoint;
- observations;
- information state;
- player-visible decisions;
- PlayerStep;
- PlayerApiError;
- observed events except visible semantic outcome;
- Python player client;
- published ML trajectories.

No player DTO should receive a new RNG field.

Visible random outcome may remain:

```text
label
bound
semantic value
```

when rules make the outcome visible.

It must not include provenance such as stream key or cursor.

### Required leak regression

Final diff/test pass should search player-facing code for:

```text
RootSeed256
root_seed
root_seed_hex
RandomStreamKeyV1
RandomStreamCursorV1
next_raw_u64
raw_u64
derived_stream
checkpoint
authoritative_replay
```

Any transitive player exposure is:

```text
BLOCKER — information leak
```

---

## L. Determinism/replay review

| Risk | Severity | Control |
|---|---:|---|
| wrong stream-key byte order | MAJOR | exact key KAT |
| wrong domain string | BLOCKER | literal KAT preimages |
| wrong HMAC key/data ordering | BLOCKER | stream key/block KATs |
| native-endian extraction | MAJOR | explicit BE lane KATs |
| cursor treated as block counter | MAJOR | lane-3 boundary |
| cursor wrap | MAJOR | MAX tests |
| rejected raw sampler word not counted | MAJOR | forced rejection test |
| n=1 consumes draw | MAJOR | zero-consumption test |
| dependency-owned sampling | BLOCKER | no `rand` |
| modulo bias | MAJOR | exact u128 threshold |
| Rust `Ord` defines persisted order | MAJOR | canonical byte-key sort |
| duplicate decoded stream overwrites | MAJOR | reject before map |
| structured JSON map keys | MAJOR | explicit stream entry array |
| full-state V1 reused | BLOCKER | FullStateDigestV2 |
| checkpoint V1 gets V2 state digest | BLOCKER | EnvironmentCheckpointV2 |
| CheckpointDigest V1 hashes V2 meaning | BLOCKER | CheckpointDigestV2 |
| replay V1 reinterpreted | BLOCKER | V2 replay types/schema |
| root/cursor leaks | BLOCKER | player leak tests |
| Python becomes RNG authority | MAJOR | codecs only |
| hidden HMAC cache becomes state | BLOCKER | no semantic cache |
| M1 fixture policy in generic validator | MAJOR | constructor-only policy |
| old RNG event retained | MAJOR | remove |
| removing old event treated as completed event migration | BLOCKER evidence error | mark transition RNG evidence NOT_RUN |

---

## M. PR boundary recommendation

Use **one atomic cross-layer PR**.

Recommended internal commit ordering:

```text
1. red V1 preservation + V2 fixtures/KAT scaffolding
2. typed mtgml-random state/codecs
3. HMAC raw generator
4. sampler + permutation
5. EngineState + FullStateDigestV2
6. EnvironmentCheckpointV2 + CheckpointDigestV2
7. Replay V2 Rust/schema/Python
8. legacy RNG event scaffold removal
9. normative docs + verification-tool updates
```

Do not merge these as partially contradictory repository states.

Reasons:

1. RandomState changes state meaning.
2. State meaning changes full-state identity.
3. Full-state identity changes current checkpoint identity.
4. Checkpoint V1 cannot be reused safely.
5. Full-state identity changes current replay identity.
6. Replay is a joint Rust/Python/schema/fixture contract.
7. Existing verifier logic contains legacy assumptions.
8. Normative docs must not continue describing old current semantics.

No new RNG architecture ADR is required. ADR 0035 already decided the design.

Checkpoint V2 is a compatibility consequence of the accepted contract, not a
new RNG design.

---

## N. Verification commands

No implementation command was executed by this planning task.

All implementation gates remain:

```text
NOT_RUN
```

unless the later implementation agent actually executes them.

### Narrow iteration

```bash
cargo test -p mtgml-random --locked
```

### Affected Rust crates

```bash
cargo test -p mtgml-model --locked
cargo test -p mtgml-state --locked
cargo test -p mtgml-rules --locked
cargo test -p mtgml-replay --locked
cargo test -p mtgml-wire --locked
cargo test -p mtgml-environment --locked
cargo test -p mtgml-conformance --locked
```

### Python/contracts

```bash
PYTHONDONTWRITEBYTECODE=1 python scripts/run_python_tests.py
python scripts/validate_schemas.py
python scripts/generate_contracts.py --check
python scripts/check_documentation.py
python scripts/verify_repository.py
```

### Repository profiles

```bash
just check-fast
just check
just check-all
```

`just check-all` is appropriate before final review because this is a broad:

- authoritative state;
- digest;
- checkpoint;
- replay;
- Rust/Python/schema/fixture;
- deterministic contract

migration.

### Gate discipline

Do not mark these `PASS` merely because lower-level tests pass:

```text
ENGINE_STATE_CONSTRUCTION_AND_INVARIANTS
ACCEPTED_TRANSITION_EXACT_PRODUCT
REJECTED_RESPONSE_COMPLETE_NONMUTATION
SEQUENTIAL_EVENT_DELTA_PARITY
CHECKPOINT_RESTORE_COMPLETE_IDENTITY
FORK_PARITY
REPLAY_PARITY
DETERMINISTIC_RNG_AND_ALLOCATORS
```

In particular:

```text
DETERMINISTIC_RNG_AND_ALLOCATORS = NOT_RUN
```

after this migration because M1.5 still owns actual transition RNG integration.

---

## O. Deferred work

### M1.1

M1.1 still owns:

- synthetic complete EngineState construction;
- reset-by-reconstruction;
- explicit RootSeed256 reset input;
- exact stream plan:
  ```text
  SyntheticM1 / Global
  ```
- initial cursor zero;
- zero RNG consumption during construction;
- configuration-specific validation;
- exact synthetic state fixture;
- M1.1 construction/invariant gate.

M1.1 does not choose RNG algorithm/derivation anymore.

### M1.5

M1.5 owns:

- actual random draw in transition workspace;
- accepted synthetic random transition;
- typed RNG authoritative event;
- typed RNG SemanticDeltaOperation;
- semantic cursor RNG composition;
- cursor before/after audit;
- sampler identity/bound/result audit;
- allocator progression;
- rejection nonmutation with a real RNG-consuming accepted path;
- deterministic repeated execution evidence;
- actual checkpoint continuation after draws;
- actual fork/replay RNG parity.

Only M1.5 can close:

```text
DETERMINISTIC_RNG_AND_ALLOCATORS
```

### Later checkpoint/replay milestone evidence

The OD-008 migration can implement the V2 checkpoint/replay identities needed
for future work without claiming the complete M1 parity gates.

The complete:

```text
CHECKPOINT_RESTORE_COMPLETE_IDENTITY
FORK_PARITY
REPLAY_PARITY
```

remain milestone evidence.

### Later Magic semantics

Not part of this migration:

- library shuffling;
- card drawing;
- zone movement;
- hidden-zone randomization;
- opaque identity invalidation;
- knowledge invalidation;
- shuffle events;
- real cards or rules.

### OD-017

Still unresolved.

Do not add:

- durable checkpoint file codec;
- stable persisted EngineState codec;
- checkpoint wire schema;
- checkpoint file migration utility.

### Later performance

Reference implementation first.

Do not add:

- block caching;
- SIMD;
- parallel RNG;
- Philox;
- ChaCha;
- GPU RNG;
- vectorized sampler.

Future benchmark hook should measure a pinned workload for:

- repeated raw words;
- representative bounded samples;
- generic permutations.

Do not invent a performance budget.

---

## P. Risks/findings

### BLOCKER — checkpoint V1 reinterpretation

**Category:** compatibility / deterministic replay risk.

The incompatibility is two-layered:

1. `EnvironmentCheckpointV1` binds FullStateDigest v1 semantics; and
2. more fundamentally, it directly embeds the unversioned current `EngineState`.

After `EngineState.random` changes, keeping the V1 runtime type constructible
would change the meaning of its `state` field even if all checkpoint field names
remain identical.

**Resolution:**

- make `EnvironmentCheckpointV2` the sole current runtime checkpoint;
- add `CheckpointDigestV2` and `CheckpointDigestInputV2`;
- retire/remove the V1 runtime type if it cannot stay truthful;
- preserve V1 only as immutable documentation/domain/golden evidence;
- do not create a legacy second EngineState.

### BLOCKER — FullStateDigest V1 reinterpretation

**Category:** digest compatibility.

Changing `FullStateDigest::DOMAIN` in place is forbidden.

So is retaining `FullStateDigestInputV1` by redefining it to serialize
`RandomStateV1` from the migrated current EngineState. The outer V1 schema name
would remain the same while its semantic coverage changed.

**Resolution:**

- add V2 digest newtype/input/domain for current state;
- current EngineState produces only V2;
- retire the current-engine V1 producer if necessary;
- preserve V1 domain and canonical golden evidence without a legacy second
  EngineState.

### BLOCKER — Replay V1 reinterpretation

**Category:** public cross-language contract.

Changing replay-v1 RNG fields or digest meaning would repurpose existing
artifacts.

**Resolution:** add V2 beside V1.

---

### MAJOR — legacy RNG event scaffold contradicts OD-008

**Category:** cross-layer contract drift.

Current string stream/counter event vocabulary is not the accepted typed RNG
audit contract.

**Resolution:** remove/quarantine it in this migration.

Do not replace it until M1.5.

---

### MAJOR — transition RNG evidence gap after scaffold removal

**Category:** evidence discipline.

Removing obsolete RNG event code does not prove typed RNG event/delta/state
consistency.

**Required status:**

```text
NOT_RUN
```

until M1.5.

---

### MAJOR — canonical map ordering

**Category:** determinism/replay.

A runtime BTreeMap must not become persisted semantics by accident.

**Resolution:** sort explicit entries by canonical stream-key bytes.

---

### MAJOR — duplicate canonical keys

**Category:** validation gap.

Building a map before duplicate validation could collapse duplicate persisted
entries.

**Resolution:** detect duplicates in entry representation first.

---

### MAJOR — validation ownership creep

**Category:** architecture/maintainer boundary.

Putting:

```text
SyntheticM1 / Global required
```

in generic EngineState validation would encode test/reset policy in universal
state validity.

**Resolution:** M1.1 constructor owns exact stream plan.

---

### MAJOR — information leak

**Category:** information safety.

New typed RNG values are more structured and therefore easier to accidentally
surface through debugging or error types.

**Resolution:** explicit player-surface regression tests.

---

### MINOR — standalone replay-step schema creep

**Category:** scope/maintainer ergonomics.

Creating a new standalone replay-step schema merely because the semantic step
version becomes V2 would add an unnecessary independently maintained contract.

**Resolution:** keep ReplayStepV2 embedded unless a later explicit requirement
creates a standalone artifact.

---

### MINOR — old normative terminology

**Category:** documentation contract drift.

Older documents use named-stream/counter and algorithm/derivation terminology.

**Resolution:** update them in the atomic migration, while preserving historical
V1 descriptions where explicitly labeled historical.

---

### MAJOR — historical checkpoint runtime retirement

**Category:** compatibility / maintainer ergonomics.

A historical runtime V1 checkpoint is not worth a second authoritative state
model. Because V1 embeds the unversioned `EngineState`, attempting to keep it
constructible after the RNG migration would force one of two bad designs:

```text
reinterpret V1 against new state semantics
or
maintain a legacy second EngineState/RNG implementation
```

Both are rejected.

**Required treatment:** retire/remove the V1 runtime type/API if necessary and
preserve only immutable historical semantic evidence. This is cleaner and more
truthful than dead compatibility machinery for a checkpoint format that does
not yet have a durable wire contract.

### NIT — no speculative RNG abstraction

Do not create an RNG backend trait or pluggable algorithm registry now.

`mtgml.rng.v1` is one accepted reference semantic contract.

Future optimized implementations can prove byte parity behind the same semantic
surface.

---

## Q. Final implementation checklist

- [ ] Start implementation from the then-current `master`, not a planning SHA.
- [ ] Synchronize/resolve current `master` before edits.
- [ ] Record the actual starting `master` HEAD in implementation/PR evidence.
- [ ] Re-read affected contracts/code on that HEAD before applying the plan.
- [ ] Re-read ADR 0035 and `docs/RNG_CONTRACT.md`.
- [ ] Preserve all accepted byte-level RNG semantics exactly.
- [ ] Add red KAT/fixture evidence before behavior where practical.
- [ ] Add workspace `hmac = "=0.13.0"`.
- [ ] Add HMAC/SHA/model dependencies to `mtgml-random`.
- [ ] Regenerate `Cargo.lock` once under pinned Rust.
- [ ] Inspect lock diff for unrelated churn.
- [ ] Implement `RootSeed256([u8; 32])`.
- [ ] Implement canonical lowercase root-seed encode/decode.
- [ ] Add wrong-length seed negative.
- [ ] Add uppercase seed negative.
- [ ] Add nonhex seed negative.
- [ ] Add noncanonical-prefix/whitespace negatives.
- [ ] Implement closed `mtgml.rng.v1` contract identity.
- [ ] Implement `RandomStreamKindV1`.
- [ ] Reserve kind code `0x0000`.
- [ ] Implement `SyntheticM1 = 0x0001`.
- [ ] Implement `RandomStreamScopeV1`.
- [ ] Implement `Global`.
- [ ] Implement `Player(PlayerId)`.
- [ ] Implement `RandomStreamKeyV1`.
- [ ] Implement exact canonical key binary encoder.
- [ ] Implement fail-closed canonical key decoder.
- [ ] Add exact `01000100` key KAT.
- [ ] Add exact Player key KAT.
- [ ] Reject unknown key version.
- [ ] Reject unknown kind.
- [ ] Reject unknown scope.
- [ ] Reject malformed payload lengths.
- [ ] Reject trailing bytes.
- [ ] Implement `RandomStreamCursorV1 { next_raw_u64 }`.
- [ ] Implement `RandomStateV1`.
- [ ] Replace authoritative string stream names.
- [ ] Implement explicit canonical stream-entry array.
- [ ] Sort entries by canonical binary key bytes.
- [ ] Reject duplicates before constructing runtime map.
- [ ] Implement HMAC stream derivation.
- [ ] Implement exact stream-domain preimage.
- [ ] Implement HMAC raw block generation.
- [ ] Implement exact raw-domain preimage.
- [ ] Implement four big-endian u64 lanes.
- [ ] Implement direct raw-word addressing.
- [ ] Implement consuming `next_raw_u64`.
- [ ] Missing stream -> typed error/no mutation.
- [ ] Cursor MAX -> typed exhaustion/no mutation.
- [ ] Add standard HMAC-SHA-256 vectors.
- [ ] Add exact K_stream KAT.
- [ ] Add block-0 KAT.
- [ ] Add block-1 KAT.
- [ ] Add raw words 0-7 KAT.
- [ ] Add lane-3 -> next-block lane-0 test.
- [ ] Add MAX-1 boundary KAT.
- [ ] Add stream isolation test.
- [ ] Implement `uniform_below_u64`.
- [ ] Bound 0 -> error/zero draws.
- [ ] Bound 1 -> 0/zero draws.
- [ ] Implement u128 threshold exactly.
- [ ] Add bound-10 normative KAT.
- [ ] Add `[0,6]` forced-rejection test.
- [ ] Prove rejected sampler words consume cursor.
- [ ] Implement checked half-open u64 range helper.
- [ ] Validate range before draw.
- [ ] Do not sample `usize` directly.
- [ ] Implement generic descending Fisher-Yates.
- [ ] Add normative `[1,3,4,0,2]` shuffle KAT.
- [ ] Prove length 0/1 consume zero.
- [ ] Do not add Magic library shuffle semantics.
- [ ] Change current EngineState to `RandomStateV1`.
- [ ] Extend generic RNG state validation.
- [ ] Validate Player-scoped stream player references.
- [ ] Accept cursor MAX as structurally valid exhaustion state.
- [ ] Do not require any stream in generic validator.
- [ ] Do not require `SyntheticM1`.
- [ ] Do not require Global scope.
- [ ] Do not require cursor zero.
- [ ] Preserve `FullStateDigest` v1 type/domain.
- [ ] Do not force a current-engine `FullStateDigestInputV1` producer.
- [ ] Retire/remove the V1 producer if it would serialize `RandomStateV1` under V1 identity.
- [ ] Preserve V1 full-state meaning with immutable domain/canonical golden evidence.
- [ ] Do not create a legacy EngineState/RandomState to regenerate V1 digests.
- [ ] Add `FullStateDigestV2`.
- [ ] Add `FullStateDigestInputV2`.
- [ ] Add domain `mtgml.full-state-digest.v2`.
- [ ] Add schema `full-state-digest-input.v2`.
- [ ] Canonical V2 random field uses explicit sorted entries.
- [ ] Current `EngineState::digest()` returns V2.
- [ ] Current `StateDelta` uses V2 digests.
- [ ] Current conformance state digest uses V2.
- [ ] Add exact nonempty-stream V2 digest golden.
- [ ] Root change changes V2 digest.
- [ ] Cursor change changes V2 digest.
- [ ] Insertion order does not change V2 digest.
- [ ] V1 is never silently reinterpreted as V2.
- [ ] Preserve checkpoint-V1 historical semantics as immutable evidence, not as a current runtime producer.
- [ ] Retire/remove `EnvironmentCheckpointV1` runtime type/API if it directly embeds migrated `EngineState`.
- [ ] Do not construct V1 checkpoints from current EngineState.
- [ ] Do not create `LegacyEngineStateV1` or a legacy RNG state solely for V1 checkpoint support.
- [ ] Preserve `CheckpointDigest` v1 domain/golden evidence.
- [ ] Keep `CheckpointDigestInputV1` only if it is a detached truthful historical helper; otherwise retire it.
- [ ] Never place migrated current EngineState or `FullStateDigestV2` into `environment-checkpoint.v1`.
- [ ] Add `CheckpointDigestV2`.
- [ ] Add domain `mtgml.checkpoint-digest.v2`.
- [ ] Add `EnvironmentCheckpointV2`.
- [ ] Add identity `environment-checkpoint.v2`.
- [ ] Add `CheckpointDigestInputV2`.
- [ ] V2 checkpoint uses `FullStateDigestV2`.
- [ ] V2 checkpoint uses `CheckpointDigestV2`.
- [ ] Current controller checkpoint API returns V2.
- [ ] Current restore accepts V2, not implicit V1.
- [ ] Add V2 checkpoint exact digest golden.
- [ ] Add V2 state-digest mismatch negative.
- [ ] Add V2 checkpoint-digest mismatch negative.
- [ ] Add status/limit/codec digest coverage.
- [ ] Prove V1/V2 checkpoint identities cannot be confused.
- [ ] Do not create checkpoint JSON Schema.
- [ ] Do not create Python checkpoint DTO.
- [ ] Do not create persisted checkpoint codec.
- [ ] Do not resolve OD-017.
- [ ] Preserve `ReplayManifestV1`.
- [ ] Preserve `ReplayStepV1`.
- [ ] Preserve `AuthoritativeReplayV1`.
- [ ] Preserve V1 replay schemas unchanged.
- [ ] Preserve V1 replay fixtures unchanged.
- [ ] Preserve V1 Rust/Python readers.
- [ ] Add `RandomnessIdentityV2`.
- [ ] V2 randomness contains only `contract_id` + canonical root seed.
- [ ] Add `ReplayManifestV2`.
- [ ] Add `ReplayStepV2`.
- [ ] Add `AuthoritativeReplayV2`.
- [ ] Use FullStateDigestV2 for V2 initial/step/final identities.
- [ ] Current environment replay writer/export uses V2.
- [ ] Add `replay-manifest.v2.schema.json`.
- [ ] Add `authoritative-replay.v2.schema.json`.
- [ ] Update `schemas/README.json` to index both replay V2 schema files.
- [ ] Preserve existing replay V1 schema entries in `schemas/README.json`.
- [ ] Do not add replay-step-v2 or checkpoint-v2 standalone schemas to `schemas/README.json`.
- [ ] Embed ReplayStepV2 shape in authoritative-replay V2 schema.
- [ ] Do not add standalone replay-step-v2 schema without explicit need.
- [ ] Use `"replay-step.v2"` only as embedded semantic-version identity where applicable.
- [ ] Add Rust V2 wire dispatch.
- [ ] Add Python V2 replay DTOs.
- [ ] Add Python V2 wire dispatch.
- [ ] Do not implement RNG in Python.
- [ ] Add shared replay V2 golden fixtures.
- [ ] Add V2 unknown-RNG-contract negative.
- [ ] Add malformed-root negatives.
- [ ] Add V1/V2 mixed-version negatives.
- [ ] Add V2 rejected-digest-mutation negative.
- [ ] Add V2 final-digest mismatch negative.
- [ ] Remove legacy string/counter RNG authoritative event scaffold.
- [ ] Remove legacy RNG SemanticDeltaOperation scaffold.
- [ ] Remove legacy RNG semantic cursor map.
- [ ] Remove obsolete old RNG compositional event test.
- [ ] Do not replace them with M1.5 typed event semantics.
- [ ] Record typed RNG event integration as `NOT_RUN`.
- [ ] Record typed RNG delta integration as `NOT_RUN`.
- [ ] Record RNG event semantic-cursor evidence as `NOT_RUN`.
- [ ] Keep `DETERMINISTIC_RNG_AND_ALLOCATORS = NOT_RUN`.
- [ ] Verify player API has no RootSeed256.
- [ ] Verify player API has no typed stream key.
- [ ] Verify player API has no stream cursor.
- [ ] Verify player API has no raw RNG word.
- [ ] Verify player-safe errors have no RNG internals.
- [ ] Verify ObservedEvent exposes only rules-visible outcomes.
- [ ] Verify Python PlayerClient has no trusted RNG/checkpoint/replay fields.
- [ ] Verify future trajectory path remains RNG-internal-free.
- [ ] Update `scripts/validate_schemas.py`.
- [ ] Update `scripts/verify_repository.py`.
- [ ] Remove verifier assumptions requiring algorithm/derivation strings.
- [ ] Remove verifier assumption requiring old RNG event test.
- [ ] Add verifier checks for V2 digest/checkpoint/replay identities.
- [ ] Do not expand generated-contract catalog scope.
- [ ] Update `docs/STATE_HASHING.md`.
- [ ] Update `docs/REPLAY_AND_DETERMINISM.md`.
- [ ] Update `docs/DOMAIN_MODEL.md`.
- [ ] Update `docs/EXECUTION_MODEL.md`.
- [ ] Update `docs/INFORMATION_MODEL.md`.
- [ ] Update `docs/RULES_SEMANTICS.md`.
- [ ] Update `docs/contracts/SEMANTIC_CONTRACT.md`.
- [ ] Update `docs/ML_TRAJECTORIES.md`.
- [ ] Update `docs/maintenance/API_LIFECYCLE.md`.
- [ ] Update RNG contract implementation-status wording only after executable evidence exists.
- [ ] Run `cargo test -p mtgml-random --locked`.
- [ ] Run affected Rust crate tests.
- [ ] Run Python tests.
- [ ] Run schema validation.
- [ ] Run generated-contract drift check.
- [ ] Run documentation validation.
- [ ] Run repository verifier.
- [ ] Run `just check-fast`.
- [ ] Run `just check`.
- [ ] Run `just check-all`.
- [ ] Inspect final diff for accidental M1.1 scope.
- [ ] Inspect final diff for accidental M1.5 transition/event scope.
- [ ] Inspect final diff for Magic shuffle/card-draw scope.
- [ ] Inspect final diff for new Python rules/RNG authority.
- [ ] Inspect final diff for hidden RNG caches/state.
- [ ] Report only actually executed successful checks as `PASS`.
- [ ] Leave unexecuted M1 gates `NOT_RUN`.
- [ ] Independently review this implementation plan before coding begins.
