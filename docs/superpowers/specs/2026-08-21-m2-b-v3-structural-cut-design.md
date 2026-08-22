# M2.B V3 Structural Cut Design

**Status:** design v3 after review, pending implementation-plan approval

**Issue:** [#49 — M2.B: V3 Structural Cut](https://github.com/chrismaghuhn/Manafold/issues/49)

**Starting source:** `a4e769eb940611d34df05fc79effd9430891d897`

**Branch:** `chris/m2-b-v3-structural-cut`

## 1. Goal

Implement the structural and versioning half of M2 as one atomic cross-layer change. The current runtime will use the accepted Decision V2, Knowledge V2, PerspectiveIdentity V2, typed-continuation, InformationStateDigestV2, FullStateDigestV3, EnvironmentCheckpointV3, and Replay V3 identities.

The change will preserve historical V1 and V2 meanings without introducing a second `EngineState`, a compatibility state engine, or a sidecar V3 runtime.

The M1 synthetic shell will remain executable through a structurally valid narrow `ChooseOne`/`SelectOne` path. M2.B will not add later M2 behavior or real Magic semantics.

## 2. Hard scope boundary

M2.B includes:

- structural Decision V2 DTOs, exact candidate ordering, and validation;
- typed continuation storage and cross-component references;
- Knowledge V2 and PerspectiveIdentity V2 state representation;
- one perspective-local `VisibleSequence` cursor;
- player-safe Information V2 shapes and `InformationStateDigestV2`;
- the rules-neutral persisted semantic codec;
- detached `FullStateDigestInputV3` and `FullStateDigestV3`;
- `EnvironmentCheckpointV3` and `CheckpointDigestV3`;
- structural Replay V3 identities and chain validation;
- coordinated Rust, schema, wire-fixture, and mechanical Python changes;
- historical V1/V2 compatibility classification and negative evidence.

M2.B does not include:

- general `ChooseMany`, `ChooseNumber`, or `Order` execution;
- continuation creation, advancement, rejection, or completion semantics beyond structural storage and the preserved M1 path;
- knowledge acquisition, reveal, private-look, hidden-transition, randomization, or retirement behavior;
- observed-event audience/redaction lifecycle;
- the legal-choice oracle or soundness/completeness closure;
- paired-state noninterference closure;
- the Python semantic environment adapter;
- production Python/native transport;
- semantic action keys or trajectory encoding;
- real Magic rules, cards, decks, Commander behavior, M2.5, or M3 work;
- final M2 closure tooling.

These exclusions are acceptance criteria, not deferred implementation notes. Any implementation that requires one of them must fail closed and remain outside this issue.

## 3. Architecture and crate ownership

The new persistence layer is a small rules-neutral crate with a concrete ownership boundary. It performs byte-level persisted-semantic encoding and hashing, but it does not own `EngineState`, rules, legality, public wire JSON, or filesystem/network I/O.

```text
mtgml-model
    primitive IDs, digest wrappers, small identity/value constants
    EnvironmentLimitCounters and CheckpointCodecIdentity

mtgml-random
    typed RNG state and canonical stream identity

mtgml-persistence
    mtgml.digest-envelope.v1
    mtgml.canonical-cbor.v1
    strict byte codec, limits, and PersistenceDecodeErrorV1
    one CheckpointDigestV3 input encoder/calculator

mtgml-decision
    Decision V1/V2 DTOs, candidate ordering, bindings, local validation

mtgml-state
    the single current EngineState, detached state-input conversion,
    cross-component validation, state digest production

mtgml-observation
    perspective-safe observation/information/event/step DTOs and projection

mtgml-replay
    replay identity, detached environment identity, Replay V1/V2/V3 validation

mtgml-wire
    public canonical UTF-8 JSON, InformationStateDigestV2 bytes,
    and shared wire fixtures

mtgml-environment
    trusted checkpoint/controller/endpoint ownership and replay execution
```

The dependency direction is:

```text
model / random / persistence
        ↓
decision / state / replay
        ↓
observation / rules
        ↓
wire
        ↓
environment / conformance
        ↓
Python client and ML orchestration
```

`mtgml-model` will not own the persistence codec. `mtgml-wire` will not own CBOR. `mtgml-state` will not depend on `mtgml-wire`. The workspace manifest, crate ownership documentation, and dependency documentation will be updated together when the new crate is introduced.

`EnvironmentLimitCounters` and `CheckpointCodecIdentity` move from `mtgml-environment` to `mtgml-model` as small rules-neutral shared value types. Their field names, Serde representation, validation meaning, and V2 historical meaning remain unchanged. `mtgml-replay` may then use the shared values without depending on `mtgml-environment`; `EnvironmentCheckpointV3` continues to be owned by the environment crate.

The dependency graph has two explicit single-owner byte paths:

```text
mtgml-persistence
    calculate_checkpoint_digest_v3(...)
    ↓                         ↓
mtgml-environment          mtgml-replay

mtgml-observation
    semantic PlayerInformationStateV2
    ↓
mtgml-wire
    canonical JSON bytes and calculate_information_state_digest_v2(...)
    ↓
mtgml-environment
```

Neither semantic producer depends upward on its byte owner. `mtgml-wire` depends on `mtgml-observation`; `mtgml-environment` may depend on `mtgml-wire`; no reverse dependency is introduced.

## 4. Old-to-new migration matrix

| Historical/current M1 representation | Current M2.B representation | Compatibility treatment |
| --- | --- | --- |
| `DecisionId` exposed in `PlayerDecisionRequest` | trusted `DecisionId` remains only in `AuthoritativeDecisionRequestV2`; player DTO uses `PlayerDecisionIdV1` | V1 meaning and fixtures remain immutable |
| `ActionCandidate { String candidate_id, semantic_key, intent }` | co-located authoritative candidate `{ CandidateIdV1, visible_intent, trusted_binding }`; projection removes binding | V1 is historical; no mandatory semantic key in V2 |
| `CandidateAssignment { candidate_id, ordinal }` | closed `DecisionAnswerV2`: `SelectOne`, `SelectMany`, `ChooseNumber`, `Order` | V1 response is never reinterpreted |
| separate `candidate_bindings` map | binding stored in each authoritative candidate record | old map is removed from current authority |
| `ContinuationRecord { id, label }` | typed continuation payload with actor, creation revision, stage index, and partial synthetic values | no legacy label-based current producer |
| global `next_opaque_*` allocator fields | per-player allocators in `PerspectiveIdentityState` | global allocator retains only trusted/global identities |
| persisted bidirectional perspective maps | one canonical persisted opaque-to-live direction plus validated runtime reverse maps | reverse maps are derived and not a second persisted authority |
| knowledge keyed by live `GameObjectId` with separate public/private lengths | active/retired opaque-keyed knowledge, location facts, provenance, invalidation, and one `next_visible_sequence` | old V2 state meaning is not decoded into current state |
| `EventSequence` in historical public events | `VisibleSequence` in M2 perspective-safe public/provenance records | V1 event values retain their meaning |
| `InformationStateDigest` / `mtgml.information-state-digest.v1` | `InformationStateDigestV2` / `mtgml.information-state-digest.v2` | V1 digest is historical and never reinterpreted |
| `FullStateDigestV2` from canonical JSON runtime serialization | detached `FullStateDigestInputV3` encoded with the persisted codec and digest envelope | V2 is readable/verifiable evidence only; no current producer |
| `EnvironmentCheckpointV2` embedding unversioned `EngineState` | `EnvironmentCheckpointV3` with current state, V3 state identity, environment identity, and V3 checkpoint identity | V2 is unsupported by the current engine |
| Replay V2 state-only identity | Replay V3 complete before/after environment identity | V2 remains detached/readable/verifiable only |
| `StateDelta { before_digest: FullStateDigestV2, after_digest: FullStateDigestV2 }` | current `StateDelta` uses `FullStateDigestV3` before/after identities and `EngineState::digest()` uses V3 | historical V2 delta evidence is detached and immutable |
| `CandidateSetDigest` V1 | no Decision V2 producer | V1 remains historical/dormant; no V2 reinterpretation |

Historical fixtures are never rewritten, deleted, or silently upgraded.

### Shared leaves and numeric identity widths

The new model values have explicit widths and do not all use the existing `canonical_id!` macro:

```text
PlayerDecisionIdV1  u64
VisibleSequence     u64
CandidateIdV1      u32
```

`PlayerDecisionIdV1` and `VisibleSequence` use the existing canonical decimal-string public convention. `CandidateIdV1` is a dedicated `u32` newtype with its own range-aware Serde implementation. Its M2 public JSON representation is a canonical unsigned JSON integer, and the schema/decoder reject negative, fractional, non-integer, and greater-than-`u32::MAX` values. Its persisted CBOR representation is an unsigned integer that must also fit the `u32` semantic range. `CandidateIdV1` is never generated through the u64-only ID macro.

## 5. Decision V2 design

`mtgml-decision` will add versioned types without changing the meaning of the V1 types:

```text
AuthoritativeDecisionRequestV2
    DecisionId
    PlayerDecisionIdV1
    StateRevision
    actor
    visibility
    DecisionDomainV2
    ordered Vec<AuthoritativeCandidateV2>
    Option<ContinuationId>

AuthoritativeCandidateV2
    CandidateIdV1
    visible_intent
    trusted_binding

PlayerDecisionRequestV2
    schema identity
    PlayerDecisionIdV1
    StateRevision
    actor
    visibility-safe DecisionDomainV2
    ordered player-safe candidates

DecisionResponseV2
    schema identity
    PlayerDecisionIdV1
    expected StateRevision
    DecisionAnswerV2
```

The player projection contains no `DecisionId`, `ContinuationId`, trusted binding, authoritative object/ability identity, allocator state, hidden context, or mandatory semantic action key.

`CandidateOrderingV1` is a named comparator with explicit ranks and numeric payload comparisons. Candidate generation first rejects duplicate public ordering keys, then sorts by `(variant_rank, payload_value)`, then assigns dense `CandidateIdV1` values from zero. Numeric comparison must distinguish `OpaqueObjectId(2)` from `OpaqueObjectId(10)` independently of textual rendering.

Local decision validation owns schema identity, closed variants, bounds, candidate ID representation, canonical `SelectMany` set order, and response shape. `validate_engine_state()` additionally validates authoritative ordering, dense IDs, duplicate public keys, visible/trusted variant equality, `ChooseNumber` candidate absence, and perspective-bound binding integrity.

## 6. State closure

`mtgml-state` remains the only current state authority. The current `EngineState` will contain the accepted state components:

```text
revision
core
zones
trusted/global allocators
execution
random
knowledge V2
perspective identity V2
format
```

`ExecutionState` will store the authoritative Decision V2 request and a closed typed continuation representation. M2 effect, waiting-trigger, and delayed-effect collections remain structurally present where required by the state closure, but their detached V3 representation must reject non-empty unsupported M2 values rather than persisting free-form M1 labels.

`KnowledgeState` will store retained active and retired knowledge facts. It will not store a live `OpaqueObjectId -> GameObjectId` association. `PerspectiveIdentityState` is the sole owner of that association and stores active mappings, derived/runtime reverse maps, local opaque/ability/player-decision allocators, and retired IDs.

Each required perspective receives one total `VisibleSequence` cursor. Public/private classification remains a property of each provenance/event record; separate public/private counters are not introduced.

Projection is read-only. No observation, information-state, visible-decision, or event projection may allocate an ID, advance a sequence, mutate knowledge, consume RNG, append replay, or change a digest.

## 7. Persistence codec ownership and contract

`mtgml-persistence` will implement the Manafold-owned restricted `mtgml.canonical-cbor.v1` profile without delegating semantic identity to a library default.

It owns:

- exact digest-envelope framing for `mtgml.digest-envelope.v1`;
- SHA-256 envelope hashing and raw digest references;
- definite-length arrays, shortest integer/length encodings, and explicit nulls;
- fixed-position records and exact enum representation helpers;
- UTF-8, duplicate/order, trailing-data, and re-encode checks;
- payload, string, byte-string, array, depth, and item limits;
- the closed `PersistenceDecodeErrorV1` categories and precedence;
- canonical encode/decode test vectors and hostile-input handling.

It does not own schema-specific `EngineState` conversion or rules semantics. State, replay, and environment producers provide detached semantic values and invoke the codec only after their own Rust-authoritative structural validation.

`mtgml-persistence` is also the sole owner of the detached `environment-checkpoint-digest-input.v3` layout and its calculation. It exposes one shared operation conceptually equivalent to:

```rust
calculate_checkpoint_digest_v3(
    full_state_digest: &DigestReferenceV1,
    status: &EpisodeStatus,
    counters: &EnvironmentLimitCounters,
    codec: &CheckpointCodecIdentity,
) -> Result<CheckpointDigestV3, PersistenceError>
```

The operation encodes exactly one canonical payload, applies the digest envelope exactly once, and returns the raw-byte-based typed V3 wrapper. `EnvironmentCheckpointV3` calls it when constructing and validating a checkpoint. Replay V3 calls the same operation when validating an initial, before, or after environment identity. Neither consumer owns a second checkpoint-input struct or digest calculation.

The V3 digest construction path is deliberately separate from the existing `domain_digest!` path:

```text
historical V1/V2 wrappers:
    existing from_canonical_bytes semantics remain unchanged

persisted V3 wrappers:
    mtgml-persistence computes SHA-256 over the exact digest envelope once
    FullStateDigestV3 / CheckpointDigestV3 accept raw [u8; 32] digest bytes
    wrappers expose canonical lowercase hex and raw bytes for DigestReferenceV1
```

`FullStateDigestV3` and `CheckpointDigestV3` must not be added to the historical macro and must not hash envelope bytes through `from_canonical_bytes()`. `InformationStateDigestV2` remains a separate canonical-JSON/domain path and is not routed through the persisted CBOR envelope.

The codec must reject maps, floats, tags, bignums, indefinite values, shared references, undefined, malformed UTF-8, noncanonical primitives, wrong record lengths, unknown variants, out-of-range values, duplicate semantic keys, noncanonical order, trailing data, and re-encode mismatch according to the accepted precedence. Length limits are checked before allocation.

## 8. Digest, checkpoint, and replay identity

### InformationStateDigestV2

`mtgml-observation` owns the player-safe `PlayerInformationStateV2` shape and the semantic `InformationStateDigestInputV2` view. It does not own canonical JSON byte encoding or the digest calculation. The digest input contains exactly the schema identity, perspective, state revision, current `ObservationEnvelopeV1`, next `VisibleSequence`, and canonical retained `PlayerKnownObjectV1[]`, with the digest field omitted.

It excludes episode status, environment counters, trusted IDs, other-player knowledge, authoritative events, RNG state, checkpoint/replay identity, and any hidden location or ordering data.

`mtgml-wire` is the sole owner of canonical JSON bytes for this input and exposes one shared operation conceptually equivalent to:

```rust
compute_information_state_digest_v2(
    input: &InformationStateDigestInputV2,
) -> Result<(Vec<u8>, InformationStateDigestV2), WireError>
```

It uses the existing canonical JSON key-sorting/duplicate-rejection path and applies the accepted `mtgml.information-state-digest.v2` domain separation exactly once. `mtgml-observation` must not implement a second JSON canonicalizer or depend on `mtgml-wire`.

`mtgml-environment` orchestrates projection, asks `mtgml-wire` to encode and calculate the digest, verifies the returned digest before exposing or committing `PlayerInformationStateV2`, and retains no alternate digest authority.

### FullStateDigestV3

`mtgml-state` converts the validated runtime state into detached `FullStateDigestInputV3`. The persisted payload follows the fixed eleven-element schema in `docs/STATE_HASHING.md`:

```text
schema, domain, revision, core_v1, zones_v1, allocators_v3,
execution_v2, random_v1, knowledge_v2, perspective_identities_v2,
format_v1
```

The conversion explicitly sorts unordered collections by their declared semantic key, preserves semantic sequence order, validates both directions of runtime mappings, rejects unsupported M2 state, and emits no runtime Serde representation. The resulting bytes are wrapped with `mtgml.digest-envelope.v1` and hashed as `mtgml.full-state-digest.v3`.

`FullStateDigestV3`, `CheckpointDigestV3`, and `InformationStateDigestV2` are distinct typed model values with independent domains. No earlier digest wrapper changes meaning.

### EnvironmentCheckpointV3

`mtgml-environment` owns the current checkpoint structure:

```text
schema identity
complete current EngineState
FullStateDigestV3
EpisodeStatus
EnvironmentLimitCounters
CheckpointCodecIdentity
CheckpointDigestV3
```

The checkpoint digest binds a complete `FullStateDigestV3` reference, status, all limit counters, and codec identity through `environment-checkpoint-digest-input.v3` and the same persisted codec/envelope. Restore validates state, state digest, status, counters, codec, and checkpoint digest before replacing backend state. A rejected restore has no mutation effect.

`EnvironmentCheckpointV2` is retired from the current runtime/controller API when the state cut lands. The implementation must not leave a compiling `EnvironmentCheckpointV2 { state: EngineState, ... }`, create `LegacyEngineStateV2`, or adapt the new `EngineState` into a historical checkpoint wrapper. Historical V2 preservation is limited to constants, documentation, detached digest/domain evidence, and immutable fixtures. Existing V2 checkpoint tests must move to detached evidence tests or V3 current-runtime tests.

### Replay V3

`mtgml-replay` owns detached Replay V3 manifest, step, and complete environment-identity values. Every step carries the actor, before checkpoint digest/revision, `DecisionResponseV2`, accepted flag, and the complete after revision/state/status/counter/checkpoint identity. Rejected steps preserve the complete before identity. Empty replay final identity equals initial identity.

Replay validation recomputes checkpoint identities and enforces continuity. It never samples host wall-clock time to reconstruct semantic replay values. Non-game-state environment values must come from explicit trusted replay-control input or remain unchanged under the accepted deterministic model.

M2.B introduces no `ReplayControlV1`. In the preserved synthetic path, `resource_units_consumed` and `wall_clock_elapsed_millis` remain unchanged. A later environment feature that needs externally supplied progression must allocate an explicit versioned replay-control input rather than adding an implicit clock read or an unversioned control field to Replay V3.

## 9. Public wire and Python boundary

`mtgml-wire` remains the canonical compact UTF-8 JSON owner. It will add only the public DTO contracts required by M2.B, with exact schema IDs, closed fields/variants, canonical scalar forms, positive fixtures, and negative fixtures.

`ObservationEnvelopeV1` may remain V1 because its payload codec identity is independently versioned. Information, observed-event, PlayerStep, decision, and replay surfaces receive new versions where their meanings change.

Python receives mechanical DTO, schema-ID, canonical JSON, and shared-fixture support only where needed for parity. Python does not generate candidates, resolve opaque IDs, mutate `EngineState`, implement RNG, restore checkpoints, execute replay, or decide legality.

Persisted CBOR state and trusted checkpoint/replay identities are never exposed through the player-facing JSON surface.

M2.B does not introduce an `EnvironmentCheckpointV3` public JSON schema, a Python `EnvironmentCheckpointV3` DTO, a durable checkpoint file format, or a public JSON `FullStateDigestInputV3` DTO unless an already accepted contract explicitly requires one. Persisted CBOR known-answer and negative fixtures remain required because they test trusted semantic identity, not public wire compatibility.

## 10. Validator ownership matrix

| Surface | Local owner | Cross-component owner | Required result |
| --- | --- | --- | --- |
| model IDs/digests/shared leaves | `mtgml-model` | none | range, canonical text, domain type, preserved V2 Serde meaning |
| persisted bytes | `mtgml-persistence` | detached producer | profile, limits, canonicality, error precedence |
| CheckpointDigestV3 | `mtgml-persistence` | environment/replay consumers | one exact checkpoint-input layout and one envelope hash |
| Decision V2 DTOs | `mtgml-decision` | `mtgml-state` | closed shape, bounds, candidate order, binding/reference coherence |
| authoritative state | `mtgml-state` component modules | `validate_engine_state()` | complete state closure and no hidden duplicate authority |
| player information/event/step | `mtgml-observation` | environment endpoint | perspective/revision coherence and privileged-type absence |
| InformationStateDigestV2 bytes | `mtgml-wire` | environment endpoint | one canonical JSON encoding and one V2 digest calculation |
| FullStateDigestInputV3 | detached state producer | `mtgml-state` | exact layout, sorting, unsupported-state rejection |
| checkpoint | `mtgml-environment` | backend restore boundary | complete identity and no-mutation rejection |
| replay | `mtgml-replay` | environment execution | revision, actor, digest, status, counter, and checkpoint continuity |
| public JSON | `mtgml-wire` / mechanical Python codec | Rust semantic types | canonical bytes and schema parity; no independent legality |

An invariant failure is a trusted implementation/service failure, not a player-facing legal rejection. Malformed wire bytes fail before semantic response creation. Typed semantic rejection preserves the complete state and identity fingerprint.

## 11. M2.B versus later milestones

| Concern | M2.B responsibility | Explicitly later |
| --- | --- | --- |
| decision families | types, closed answer union, structural validation | general family execution and exact legal-space closure, M2.C/F |
| continuations | typed storage, identity, payload validation, digest/checkpoint/replay inclusion | lifecycle create/advance/reject/complete, M2.C |
| knowledge | active/retired representation, provenance fields, validation shape | acquisition, reveal, look, hidden transition, invalidation behavior, M2.E |
| opaque identity | mapping/allocator/retirement fields and structural invariants | distinguishability lifecycle and retirement behavior, M2.E |
| observed events | V2 structural safe surface and sequence field | audience/redaction lifecycle, M2.E |
| legal choices | DTO soundness constraints | independent oracle, soundness, completeness, M2.F |
| information safety | no privileged fields in structural surfaces | paired-state noninterference and multi-endpoint closure, M2.G |
| Python | mechanical DTO/codec parity | semantic environment adapter, M2.H |
| closure status | owned executable contract/version gate evidence | final M2 closure and downstream gate promotion, M2.Final |

## 12. Implementation order

The implementation will use one coherent PR and reviewable internal commits in this order:

1. inventory current producers, readers, fixtures, and historical contracts;
2. move shared environment leaf values to `mtgml-model` with unchanged V2 Serde meaning;
3. add `mtgml-persistence` and its sole CheckpointDigestV3 input/calculation path;
4. add Decision V2 types, comparator, and focused positive/negative tests;
5. migrate `StateDelta` and the authoritative transition product to `FullStateDigestV3`;
6. retire `EnvironmentCheckpointV2` from current runtime/controller APIs and isolate its detached evidence;
7. migrate execution, knowledge, perspective identity, and synthetic construction;
8. make `validate_engine_state()` own the new cross-component invariants;
9. add InformationStateDigestV2 semantic DTOs in observation and its sole canonical byte/digest path in wire;
10. implement the persisted codec, envelope, detached V3 state input, and raw-byte digest construction;
11. implement Checkpoint V3 and restore no-mutation validation through the shared persistence calculator;
12. implement Replay V3 and complete environment identity chaining through the same calculator, without ReplayControlV1;
13. update schemas, wire fixtures, Python mechanical codecs, and compatibility negatives;
14. migrate M1 synthetic tests without changing historical fixtures;
15. run focused evidence, native fallbacks where possible, and exact hosted checks;
16. self-review the final diff for information leakage, hidden state, version reinterpretation, duplicate byte authorities, and M2.C+ scope creep.

No intermediate commit may leave the current runtime producing a new semantic V2/V3 value that lacks its corresponding validator, detached identity, or public/fixture contract.

## 13. Evidence and verification

Required focused evidence includes:

- candidate ordering, including numeric `2 < 10`, dense IDs, duplicate-key rejection, variant mismatch, and `ChooseNumber` candidate rejection;
- `CandidateIdV1` u32 construction, canonical public integer decoding, and overflow rejection above `u32::MAX`;
- typed continuation/reference and allocator validation;
- all accepted canonical-CBOR primitives, resource limits, forbidden forms, precedence cases, re-encode equality, and trailing-data rejection;
- digest-envelope vectors and SHA-256 vectors;
- FullStateDigestV3 known-answer and mutation coverage across every authoritative M2 component;
- InformationStateDigestV2 known-answer and exclusion/mutation cases;
- CheckpointDigestV3 known-answer, corrupt-state/digest/status/counter/codec rejection, and restore no-mutation cases;
- Replay V3 empty/accepted/rejected identity chains, final identity equality, and no wall-clock sampling;
- immutable V1/V2 fixture preservation and explicit historical support negatives;
- absence of current `EnvironmentCheckpointV2`/legacy-state producers and detached-only V2 checkpoint evidence;
- Rust/Python mechanical byte and digest parity where the public/persisted tooling applies;
- M1 regression evidence on the final exact head.

Verification reports will distinguish focused unit evidence, integration evidence, hosted CI, M1 regression, and the M2 owned gate. `just check-fast` was attempted on the baseline and is currently `BLOCKED` by `HCS_E_HYPERV_NOT_INSTALLED`; this is environment evidence, not a code pass. No unavailable command will be reported as `PASS`.

The owned result can become:

```text
M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT = PASS
```

only after the executable structural evidence passes on the exact final source head. This design does not promote any later M2 gate.

## 14. Design self-review

- The persistence codec has a separate lower-layer owner and does not introduce `mtgml-state -> mtgml-wire` or I/O into `mtgml-model`.
- CheckpointDigestV3 has one calculation owner in `mtgml-persistence`; environment and replay are consumers only.
- InformationStateDigestV2 has one canonical JSON/digest owner in `mtgml-wire`; observation owns semantic DTOs only and environment verifies the result.
- The design has one current `EngineState` and no legacy duplicate or V3 sidecar state.
- V1/V2 meanings, fixtures, and historical support classifications remain immutable.
- `DecisionId`, `ContinuationId`, authoritative object IDs, physical IDs, RNG internals, and checkpoint/replay identities are excluded from player surfaces.
- `PerspectiveIdentityState` is the sole live opaque-to-authoritative mapping authority.
- InformationStateDigestV2 is player-safe and excludes episode/environment/trusted replay data.
- FullStateDigestV3 and CheckpointDigestV3 use detached semantic inputs and the accepted persisted codec, not runtime Serde output.
- Replay V3 binds complete environment/checkpoint identity and preserves rejected-step identity.
- Validation ownership is explicit, and rejected restore/submission paths remain fail-closed and nonmutating.
- The M2.B/M2.C–H boundary is explicit; no later behavior is required by this design.
- No unresolved placeholder, speculative V4 compatibility layer, or undocumented producer is part of the design.
