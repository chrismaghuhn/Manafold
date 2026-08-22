# M2.B V3 Structural Cut Implementation Plan

**Status:** provisional process plan, requested changes integrated, pending final review

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Issue #49 as one atomic cross-layer structural cut in which the current Manafold runtime uses Decision V2, the M2 `EngineState` shape, `InformationStateDigestV2`, `FullStateDigestV3`, `EnvironmentCheckpointV3`, and Replay V3 while preserving V1/V2 historical evidence without reinterpretation.

**Architecture:** Keep `mtgml-state` as the only current `EngineState` authority. Add the rules-neutral `mtgml-persistence` lower layer for the restricted canonical CBOR/envelope codec and the single `CheckpointDigestV3` calculation; keep semantic state producers above it. Let `mtgml-observation` own semantic player-information DTOs, `mtgml-wire` own their one canonical JSON/digest byte path, and `mtgml-environment` orchestrate and verify the result.

**Tech Stack:** Rust 2021 workspace with pinned Serde/SHA-256/thiserror dependencies, manually enforced `mtgml.canonical-cbor.v1` profile, canonical compact UTF-8 JSON, JSON Schema Draft 2020-12, Python 3.11–3.13 rules-free mechanical DTO/codec tooling, shared golden/negative fixtures, and the repository `just`/Python verification commands.

---

## Starting point and non-negotiable boundaries

Implementation starts only from:

```text
source commit: a4e769eb940611d34df05fc79effd9430891d897
branch:        chris/m2-b-v3-structural-cut
```

The local `just check-fast` baseline is already known to be `BLOCKED` because Windows cannot start WSL2/Hyper-V (`HCS_E_HYPERV_NOT_INSTALLED`). It must remain reported as `BLOCKED`, never upgraded to `PASS`, and native Rust/Python fallback evidence must be labeled separately.

The implementation is one PR with reviewable internal commits. No commit may leave a current producer using a newly reinterpreted V2 value. V1/V2 fixtures, domain vectors, detached V2 readers, and historical support classifications remain unchanged. M2.B must not add general `ChooseMany`, `ChooseNumber`, or `Order` execution, knowledge lifecycle behavior, event-redaction lifecycle, legality completeness, noninterference closure, semantic action keys, Python transport, real Magic rules, cards, decks, M2.5, or M3 behavior.

## Commit policy and the one atomic runtime cut

Every commit must be green under its narrowest applicable test command. A red phase is allowed in the worktree while a test or fixture is being driven to green, but the red state is never committed. Preparatory commits may add only dormant, detached, or rules-neutral material that cannot be reached by the current runtime and cannot reinterpret a historical V2 value. The model ownership move for `EnvironmentLimitCounters` and `CheckpointCodecIdentity` is allowed as a rules-neutral exception only when its Serde/byte meaning remains identical.

Tasks 5 and 6 therefore prepare dormant M2 state structures and a detached V3 digest calculator. They must not change the current `EngineState`, current `EngineState::digest()`, current `StateDelta`, controller, checkpoint, replay, rules, or endpoint contract. Tasks 7 through 11 are one worktree operation. The agent may use internal compiler-guided substeps, but must not commit any SHA between them.

The single **atomic cut commit** at the end of Task 11 must make the complete current workspace coherent in one step and must contain all of these changes together:

```text
EngineState M2 structure
EngineState::digest() -> FullStateDigestV3
StateDelta -> FullStateDigestV3
RulesKernel -> DecisionResponseV2
TransitionResult ->
    next_state: EngineState
    events: AuthoritativeRuleEvent[]
    delta: StateDelta
    next_decision: Option<AuthoritativeDecisionRequestV2>
    status: EpisodeStatus
Environment/perspective endpoint -> PlayerDecisionRequestV2 projection
EnvironmentCheckpointV3
remove current EnvironmentCheckpointV2
Replay V3 current producer/executor
EnvironmentBackend V3
synthetic endpoint V2
```

No intermediate SHA may expose a current M2 `EngineState` with a V2 digest, a current V2 checkpoint containing the new state meaning, a V2 transition product, or a partially migrated controller/replay path. If a compiler/test failure occurs while Tasks 7–11 are being assembled, fix it in the same worktree and continue; do not make that intermediate state reviewable through a commit.

## Ownership and dependency map

| Surface | Sole owner | Consumers | Explicit prohibition |
|---|---|---|---|
| Shared IDs, digest wrappers, `EnvironmentLimitCounters`, `CheckpointCodecIdentity` | `crates/mtgml-model` | all declared consumers | no persistence I/O or codec implementation |
| Canonical CBOR, digest envelope, resource limits, decode precedence | new `crates/mtgml-persistence` | state, environment, replay, trusted Python parity | no rules, `EngineState`, public JSON, or filesystem ownership |
| `CheckpointDigestV3` input layout/calculation | `mtgml-persistence` | environment and replay | no second checkpoint input struct or hash path |
| Decision V2 semantic/trusted/player forms and local validation | `crates/mtgml-decision` | state, wire, environment | no legality oracle or semantic action keys |
| Current `EngineState`, detached full-state input, cross-component state validation | `crates/mtgml-state` | rules, environment, replay | no second state crate, sidecar state, or `mtgml-wire` dependency |
| Semantic player-information DTOs and semantic digest input view | `crates/mtgml-observation` | wire and environment | no canonical JSON bytes or digest calculation |
| Canonical public JSON and `InformationStateDigestV2` bytes/hash | `crates/mtgml-wire` | environment endpoint, Python parity | no CBOR and no dependency on environment |
| Replay V3 identity and structural validation | `crates/mtgml-replay` | environment execution | no dependency on environment; V2 remains detached only |
| Checkpoint/controller/replay execution orchestration | `crates/mtgml-environment` | trusted callers | no current V2 checkpoint API and no duplicate digest authority |
| Mechanical public/persistence DTO and byte parity | `python/src/mtgml` | tests and tooling | no rules, state, legality, migration, or transport authority |

The dependency direction after the cut is:

```text
mtgml-model / mtgml-random / mtgml-persistence
        ↓
mtgml-decision / mtgml-state / mtgml-replay
        ↓
mtgml-observation / mtgml-rules
        ↓
mtgml-wire
        ↓
mtgml-environment / mtgml-conformance
        ↓
Python DTO/codec tooling
```

The two digest paths are intentionally separate:

```text
state/environment/replay semantic values
        ↓
mtgml-persistence::calculate_checkpoint_digest_v3
        ↓
CheckpointDigestV3

mtgml-observation::InformationStateDigestInputV2
        ↓
mtgml-wire::compute_information_state_digest_v2
        ↓
InformationStateDigestV2
```

## File and module map

### New files

- `crates/mtgml-persistence/Cargo.toml` — pinned lower-layer crate metadata and path dependency on `mtgml-model`.
- `crates/mtgml-persistence/src/lib.rs` — public codec/envelope/checkpoint-digest API.
- `crates/mtgml-persistence/src/cbor.rs` — restricted CBOR value model, canonical encoder, bounded decoder, and re-encode check.
- `crates/mtgml-persistence/src/envelope.rs` — exact `mtgml.digest-envelope.v1` framing and SHA-256 calculation.
- `crates/mtgml-persistence/src/error.rs` — `PersistenceDecodeErrorV1` with the accepted total precedence.
- `crates/mtgml-persistence/src/checkpoint_digest.rs` — the only `environment-checkpoint-digest-input.v3` builder/calculator.
- `crates/mtgml-persistence/src/tests.rs` — primitive, envelope, limit, precedence, and checkpoint known-answer tests.
- `crates/mtgml-state/src/digest_v3.rs` — detached `FullStateDigestInputV3` semantic conversion and canonical-value construction.
- `crates/mtgml-replay/src/v3.rs` — `ReplayManifestV3`, `InitialEnvironmentIdentityV3`, `ReplayStepV3`, `AuthoritativeReplayV3`, and V3 recorder.
- `crates/mtgml-persistence/README.md` — codec ownership and non-I/O boundary.
- `python/src/mtgml/persistence.py` — mechanical restricted-CBOR/envelope/vector reader; no state conversion or rules semantics.
- `python/tests/test_persistence_codec.py` — Python parity against committed Rust-produced vectors and negative fixtures.
- `python/tests/test_m2_b_staging_fixtures.py` — staging-only file/schema-id/historical-fixture checks; never calls live wire decoders.
- `scripts/run_m2_b_contract_cut.py` — executable owner of the single `M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT` gate and its external evidence report.
- `crates/mtgml-state/src/m2_shape.rs` — private pre-cut namespace for collision-free dormant M2 state building blocks.
- `schemas/player-decision-request.v2.schema.json` — public Decision V2 request shape.
- `schemas/decision-response.v2.schema.json` — public Decision V2 response shape.
- `schemas/information-state-envelope.v2.schema.json` — public player-information V2 shape.
- `schemas/observed-event-envelope.v2.schema.json` — public perspective-safe observed-event V2 shape.
- `schemas/player-step.v2.schema.json` — public PlayerStep V2 shape.
- `schemas/replay-manifest.v3.schema.json` — public Replay Manifest V3 shape.
- `schemas/authoritative-replay.v3.schema.json` — public Replay V3 file shape.
- `wire/staging/m2-b/manifest.json` and `wire/staging/m2-b/*.json` — staging-only V2/V3 public fixtures prepared before their live decoder/manifest promotion.
- `wire/golden/player-decision-request.v2.json`, `decision-response.v2-select-one.json`, `information-state-envelope.v2.json`, `observed-event-envelope.v2.json`, `player-step.v2.json`, `replay-manifest.v3.json`, and `authoritative-replay-empty.v3.json` — promoted public fixtures; promotion is performed by Tasks 4, 8, and 10 only after the corresponding decoder exists.
- `wire/negative/decision-v2-candidate-id-overflow.json`, `decision-v2-noncanonical-select-many.json`, and `replay-v3-checkpoint-digest-mismatch.json` — promoted public negative fixtures; their active manifest registration is performed by the corresponding decoder task, not Task 1.
- `wire/historical/v1-v2-fixtures.json` — immutable list/baseline references used by the M2.B historical/source evidence check; the runner must prove that this inventory exactly covers the V1/V2 fixture set present at the starting SHA.
- `persistence/golden/manifest.json` and `persistence/golden/*.cbor` — persisted CBOR known-answer fixtures.
- `persistence/negative/manifest.json` and `persistence/negative/*.cbor` — cross-language codec, envelope, resource-limit, primitive-range, and detached-schema rejection fixtures only.
- `persistence/rust-negative/manifest.json` and `persistence/rust-negative/*.cbor` — Rust-authoritative `EngineState`, checkpoint, replay, and `semantic_validation` rejection fixtures; Python must not enumerate this directory.
- `docs/superpowers/plans/2026-08-21-m2-b-v3-structural-cut.md` — this plan.

### Existing files to modify

- `Cargo.toml`, `Cargo.lock` — register `mtgml-persistence` and its locked dependencies.
- `crates/mtgml-model/src/lib.rs` — add shared leaves, manual `CandidateIdV1`, `PlayerDecisionIdV1`, `VisibleSequence`, `DigestReferenceV1`, and raw-byte V3 wrappers while preserving historical macro types.
- `crates/mtgml-decision/src/lib.rs` — add Decision V2 forms, `CandidateOrderingV1`, answer union, and local validation; retain V1 forms unchanged.
- `crates/mtgml-state/Cargo.toml`, `crates/mtgml-state/src/engine.rs`, `execution.rs`, `identity.rs`, `knowledge.rs`, `validation.rs`, `construction.rs`, `delta.rs`, `digest.rs`, `lib.rs`, `tests.rs` — depend on the persistence lower layer, migrate the one current state authority, and remove free-form current continuation/state shapes.
- `crates/mtgml-rules/src/transition.rs`, `contract.rs`, `validation.rs`, `tests.rs` — carry V3 state/delta identity and preserve only the structural synthetic path.
- `crates/mtgml-observation/src/lib.rs` — add semantic `PlayerInformationStateV2`, retained-knowledge projection types, and `InformationStateDigestInputV2` without canonical byte code.
- `crates/mtgml-wire/src/lib.rs` — register V2/V3 public contracts and own `compute_information_state_digest_v2` through the existing canonical JSON implementation.
- `crates/mtgml-replay/Cargo.toml`, `crates/mtgml-replay/src/lib.rs`, `identity.rs`, `manifest.rs`, `recorder.rs`, `validation.rs`, `tests.rs` — depend on the persistence lower layer, preserve V1/V2 detached contracts, and add V3 identity/continuity validation.
- `crates/mtgml-environment/Cargo.toml`, `checkpoint.rs`, `controller.rs`, `replay.rs`, `synthetic.rs`, `endpoint.rs`, `errors.rs`, `lib.rs`, `tests.rs` — retire current V2 checkpoint/controller APIs, wire shared digest owners, and keep the synthetic path structurally valid.
- `crates/mtgml-conformance/src/lib.rs` — update authoritative transition expectations to V3 and add structural evidence cases.
- `python/src/mtgml/decision.py`, `observation.py`, `replay.py`, `wire.py`, `canonical.py`, `__init__.py` — mechanical V2/V3 DTO/codec parity while retaining V1 readers.
- `python/tests/test_wire_contracts.py`, `test_player_api.py`, `test_schema_parity.py`, `test_documentation_contracts.py` — public V2/V3 parity and boundary tests.
- `contracts/catalog/contract-vocabulary.v1.json`, `scripts/generate_contracts.py`, generated outputs under `crates/mtgml-model/src/generated_contract_vocab.rs`, `python/src/mtgml/_generated_contract_vocab.py`, `schemas/`, and `docs/generated/` — update only mechanically shared vocabulary from the catalog.
- `schemas/README.json`, `wire/golden/manifest.json`, `wire/negative/manifest.json`, `wire/README.md` — register public versions and preserve every historical fixture.
- `.github/workflows/pr-fast.yml` — check out `${{ github.event.pull_request.head.sha }}` with `fetch-depth: 0`, assert the exact PR head, and invoke the executable M2.B slice runner after the pinned Rust toolchain is installed; preserve the existing `pull_request` trigger.
- `docs/ARCHITECTURE.md`, `docs/PROJECT_STRUCTURE.md`, `crates/README.md`, `docs/DECISION_PROTOCOL.md`, `docs/INFORMATION_MODEL.md`, `docs/STATE_HASHING.md`, `docs/REPLAY_AND_DETERMINISM.md`, `docs/contracts/WIRE_CONTRACT.md`, `docs/ERROR_MODEL.md` — record implemented ownership/version boundaries without claiming M2 gates `PASS`.

## Task 1: Freeze test-first fixtures and source inventory

**Files:**
- Create: `persistence/golden/manifest.json`, `persistence/negative/manifest.json`
- Create: `wire/staging/m2-b/manifest.json` and the staged public V2/V3 JSON fixture paths listed in the file map
- Create: `wire/historical/v1-v2-fixtures.json` as an immutable historical fixture path/hash inventory
- Modify: `python/tests/test_m2_b_staging_fixtures.py`, new persistence fixture tests
- Do not modify: live `wire/golden/manifest.json`, live `wire/negative/manifest.json`, or the active schema-parity mapping in this task
- Test: staging-only fixture checks plus the unchanged historical `python/tests/test_wire_contracts.py`

- [ ] **Step 1: Record the exact current producers and historical readers.**

  Search and record the current locations of `FullStateDigestV2`, `CheckpointDigestV2`, `EnvironmentCheckpointV2`, `AuthoritativeReplayV2`, `PlayerDecisionRequest`, `DecisionResponse`, `public_history_length`, `private_history_length`, and `ContinuationRecord.label`. The inventory must distinguish current producers from historical fixtures/tests before any rename.

  ```powershell
  rg -n "FullStateDigestV2|CheckpointDigestV2|EnvironmentCheckpointV2|AuthoritativeReplayV2|PlayerDecisionRequest|DecisionResponse|public_history_length|private_history_length|ContinuationRecord|label" crates python schemas wire docs
  ```

- [ ] **Step 2: Add staged public V2/V3 fixtures without registering live decoders.**

  Place the exact new fixture files under `wire/staging/m2-b/` and record their contract/schema IDs in the staging manifest only. Keep every existing live V1/V2 path and active manifest entry in place. The staged fixtures must contain closed fields only, explicit `schema_version`, `PlayerDecisionIdV1` as a canonical decimal string, `CandidateIdV1` as an unsigned JSON integer, and no semantic action key. Do not add a staged contract to `wire/golden/manifest.json`, `wire/negative/manifest.json`, or the active schema-parity mapping before its decoder task.

- [ ] **Step 3: Add red staging tests for file contents and historical immutability.**

  Add `python/tests/test_m2_b_staging_fixtures.py` assertions that the old fixture bytes/text are unchanged, staged manifest paths exist, the staging manifest records the expected contract/schema IDs, V2 public requests contain no trusted `DecisionId`, and a staged V3 replay fixture contains full initial/final environment identity. This test may parse JSON and inspect closed-field/schema identity shape, but it must not require schema files that are created by later promotion tasks, call `mtgml.wire.decode_canonical()`, or otherwise require future V2/V3 decoders. Run:

  ```powershell
  python -m pytest python/tests/test_m2_b_staging_fixtures.py python/tests/test_wire_contracts.py -q
  ```

  Expected result: only the new staging test is red until the staged files/inventory exist; the live V1 tests must remain green when the native Python environment is available.

- [ ] **Step 4: Implement the minimum staging inventory and turn the staging red phase green.**

  Add only the staged fixture files, staging manifest, schema-ID references, and immutable historical inventory needed for the tests in Step 3 to pass. The staging test intentionally does not require the future schema files or perform their JSON-Schema validation; each decoder/promotion task owns creating, registering, and validating its schema. Re-run the same focused command and confirm the staging test is green while the historical V1 tests remain green. Do not commit the intentionally red phase from Step 3 and do not promote any new contract into a live manifest.

- [ ] **Step 5: Commit the green fixture/inventory boundary.**

  ```powershell
  git add persistence wire schemas python/tests
  git commit -m "test: freeze M2.B public fixture boundaries"
  ```

## Task 2: Add model-owned shared leaves, IDs, and digest wrappers

**Files:**
- Modify: `crates/mtgml-model/src/lib.rs`
- Modify: `crates/mtgml-environment/src/checkpoint.rs`, `crates/mtgml-environment/src/lib.rs` only to remove the duplicate leaf definitions and re-export the model-owned types if existing callers still need that path
- Test: `crates/mtgml-model/src/lib.rs` unit tests

- [ ] **Step 1: Add red tests for type width, Serde meaning, and raw V3 construction.**

  The tests must assert:

  ```rust
  assert_eq!(std::mem::size_of::<CandidateIdV1>(), 4);
  assert_eq!(serde_json::to_string(&CandidateIdV1(7)).unwrap(), "7");
  assert_eq!(serde_json::to_string(&PlayerDecisionIdV1(7)).unwrap(), "\"7\"");
  assert_eq!(serde_json::to_string(&VisibleSequence(7)).unwrap(), "\"7\"");
  assert_eq!(FullStateDigestV3::from_digest_bytes([0xabu8; 32]).raw_bytes(), [0xabu8; 32]);
  assert!(serde_json::from_str::<CandidateIdV1>("4294967296").is_err());
  assert!(serde_json::from_str::<CandidateIdV1>("-1").is_err());
  ```

  Also retain the existing V1/V2 digest known-answer assertions unchanged.

- [ ] **Step 2: Implement manual `u32` and `u64` ID semantics.**

  Keep the existing `canonical_id!` macro untouched for historical types. Implement `CandidateIdV1(pub u32)` separately with numeric JSON Serde, checked construction, `Display`, and ordered/hash traits. Implement `PlayerDecisionIdV1(pub u64)` and `VisibleSequence(pub u64)` with the existing canonical decimal-string behavior under their own named types. No new type may use `canonical_id!` for `CandidateIdV1`.

- [ ] **Step 3: Add model-owned environment leaves without changing their V2 representation.**

  Move `EnvironmentLimitCounters` and `CheckpointCodecIdentity` exactly, including field names, order, `deny_unknown_fields`, derives, and validation meaning. Their five counters remain unsigned `u64`; `codec_id` and `semantic_version` remain exact non-empty strings. Remove the environment-local struct definitions rather than copying them. A temporary `pub use mtgml_model::{EnvironmentLimitCounters, CheckpointCodecIdentity};` is allowed only as a re-export of the same types.

- [ ] **Step 4: Add `DigestReferenceV1` and raw-byte V3 wrappers.**

  Add owned model values with these construction rules:

  ```rust
  pub struct DigestReferenceV1 {
      pub semantic_domain: String,
      pub input_schema_id: String,
      pub digest_bytes: [u8; 32],
  }

  impl FullStateDigestV3 {
      pub fn from_digest_bytes(bytes: [u8; 32]) -> Self;
      pub fn raw_bytes(&self) -> [u8; 32];
      pub fn as_digest_reference(&self) -> DigestReferenceV1;
  }

  impl CheckpointDigestV3 {
      pub fn from_digest_bytes(bytes: [u8; 32]) -> Self;
      pub fn raw_bytes(&self) -> [u8; 32];
  }
  ```

  V3 wrappers parse/render exactly 64 lowercase hexadecimal characters and never expose `from_canonical_bytes`. Do not add them to `domain_digest!`; the persistence layer supplies the already-computed SHA-256 bytes exactly once. Add `InformationStateDigestV2` separately with the `mtgml.information-state-digest.v2` domain and its existing JSON/domain `from_canonical_bytes` semantics; it is not routed through the persisted envelope.

- [ ] **Step 5: Run model tests and commit.**

  ```powershell
  cargo test -p mtgml-model --locked
  git add crates/mtgml-model crates/mtgml-environment/src/checkpoint.rs crates/mtgml-environment/src/lib.rs
  git commit -m "feat: add M2 shared identities and digest values"
  ```

  Expected result: all historical model vectors pass and the new width/raw-byte tests pass.

## Task 3: Implement the rules-neutral persistence codec and envelope

**Files:**
- Create: `crates/mtgml-persistence/Cargo.toml`, `src/lib.rs`, `src/cbor.rs`, `src/envelope.rs`, `src/error.rs`, `src/checkpoint_digest.rs`, `src/tests.rs`, `README.md`
- Modify: root `Cargo.toml`, `Cargo.lock`
- Test: `crates/mtgml-persistence/src/tests.rs`, `persistence/golden/`, and the mechanical-only `persistence/negative/`

- [ ] **Step 1: Register the crate with only lower-layer dependencies.**

  Add `mtgml-persistence` to the workspace. Its dependencies are `mtgml-model`, workspace `sha2`, workspace `thiserror`, and workspace `serde` only where a trusted typed leaf needs it. It must not depend on `mtgml-state`, `mtgml-environment`, `mtgml-replay`, `mtgml-wire`, `mtgml-rules`, or any filesystem/network crate API.

- [ ] **Step 2: Add red codec tests for the complete primitive/profile surface.**

  Test canonical encodings for unsigned integers, signed integers, booleans, null, byte strings, UTF-8 text, and fixed arrays. Add negative cases for maps, floats, tags, bignums, indefinite arrays/strings, shared references, undefined, malformed UTF-8, non-shortest integer encodings, wrong array length, unknown variant, out-of-range values, duplicate keys, noncanonical order, trailing bytes, and re-encode mismatch. Use the exact accepted precedence:

  ```text
  unsupported_historical_version
  envelope_identity
  envelope_length
  payload_too_large
  string_too_large
  array_too_large
  depth_exceeded
  item_limit_exceeded
  disallowed_cbor_form
  noncanonical_primitive
  invalid_utf8
  wrong_record_length
  unknown_variant
  value_out_of_range
  duplicate_semantic_key
  noncanonical_order
  schema_identity_mismatch
  trailing_data
  reencode_mismatch
  digest_mismatch
  semantic_validation
  ```

- [ ] **Step 3: Implement the restricted CBOR value model and bounded decoder.**

  Use an internal value enum with only `Null`, `Bool`, `Unsigned(u64)`, `Signed(i64)`, `Bytes(Vec<u8>)`, `Text(String)`, and `Array(Vec<Value>)`. Encode definite-length arrays and shortest legal integer/length forms. Decode headers into checked lengths before allocating, track depth/items, reject all excluded CBOR forms, preserve exact UTF-8 bytes, and require `encode(decoded) == input` before returning a canonical value. Do not delegate semantic state or runtime Serde representation to this module.

- [ ] **Step 4: Implement exact digest-envelope framing.**

  Implement the `mtgml.digest-envelope.v1` frame from `docs/STATE_HASHING.md` with non-empty ASCII identity fields, 255-byte identifier limits, explicit payload length, SHA-256 over the complete envelope bytes, and a `DigestReferenceV1` whose canonical six-element rendering is `["mtgml.digest-envelope.v1", "sha-256", semantic_domain, "mtgml.canonical-cbor.v1", input_schema_id, digest_bytes_32]`. The envelope must not hash a hex rendering and must not domain-separate a second time.

- [ ] **Step 5: Add committed mechanical persistence vectors and parity assertions.**

  Store canonical payload/envelope bytes as `.cbor` fixtures under `persistence/golden/`; store only codec/envelope/resource-limit/primitive-range/detached-schema malformed byte fixtures and expected error categories under `persistence/negative/`. Do not store `EngineState`, checkpoint, replay, continuation, knowledge, or `semantic_validation` cases in this cross-language directory; those belong in `persistence/rust-negative/` and are verified by Rust-authoritative state/checkpoint/replay tests.
  Add Rust tests that read every mechanical manifest entry, assert exact bytes and digest hex, and assert the first error category for multi-defect mechanical inputs.

- [ ] **Step 6: Run focused persistence tests and commit.**

  ```powershell
  cargo test -p mtgml-persistence --locked
  git add Cargo.toml Cargo.lock crates/mtgml-persistence persistence
  git commit -m "feat: add strict persisted semantic codec"
  ```

  Expected result: the codec test suite is green; no public JSON fixture or Python module is used to define persisted CBOR semantics.

## Task 4: Implement Decision V2 and `CandidateOrderingV1`

This task adds dormant Decision V2 trusted/player forms, local validators, schemas, and fixtures. It must not change the current `RulesKernel`, current `TransitionResult`, current controller, or current endpoint producers; those consumers switch only inside the atomic cut in Tasks 7–11.

**Files:**
- Modify: `crates/mtgml-decision/src/lib.rs`
- Modify: `crates/mtgml-wire/src/lib.rs`
- Modify: `schemas/player-decision-request.v2.schema.json`, `schemas/decision-response.v2.schema.json`
- Modify: `python/src/mtgml/decision.py`, `python/src/mtgml/wire.py`
- Modify: `python/tests/test_schema_parity.py` for the active Decision V2 contract-to-schema mapping
- Modify: new public Decision V2 fixtures and manifests
- Test: `crates/mtgml-decision/src/lib.rs`, `python/tests/test_wire_contracts.py`, `python/tests/test_schema_parity.py`

- [ ] **Step 1: Add red Rust tests for all closed domains and answers.**

  Define and test these exact semantic forms:

  ```rust
  pub enum DecisionDomainV2 {
      ChooseOne,
      ChooseMany { minimum: u32, maximum: u32 },
      ChooseNumber { minimum: i64, maximum: i64 },
      Order { minimum: u32, maximum: u32 },
  }

  pub enum DecisionAnswerV2 {
      SelectOne { candidate_id: CandidateIdV1 },
      SelectMany { candidate_ids: Vec<CandidateIdV1> },
      ChooseNumber { value: i64 },
      Order { candidate_ids: Vec<CandidateIdV1> },
  }
  ```

  Red tests must reject inverted bounds, wrong answer/domain pair, missing or extra candidates, duplicate IDs, nonascending `SelectMany`, duplicate `Order` IDs, out-of-range `ChooseNumber`, and any `ChooseNumber` request that emits candidates.

- [ ] **Step 2: Implement co-located authoritative candidates and projections.**

  Add `AuthoritativeCandidateV2 { candidate_id: CandidateIdV1, visible_intent: CandidateIntent, trusted_binding: EngineCandidateBinding }`, `AuthoritativeDecisionRequestV2`, `PlayerDecisionRequestV2`, and `DecisionResponseV2`. The player request contains `PlayerDecisionIdV1`, actor, revision, visibility, domain, and public candidates only. It contains no trusted `DecisionId`, continuation, binding, hidden context, allocator history, or mandatory semantic key.

- [ ] **Step 3: Implement the exact semantic comparator and dense assignment.**

  Add `CandidateOrderingV1` with ranks `pass_priority = 0` through `confirm = 8`. Compare payloads numerically by the underlying ID/number rules, reject duplicate `(variant_rank, payload_value)` keys, sort deterministically, and assign `CandidateIdV1(0..n-1)`. Never compare JSON/text, enum declaration order, bindings, trusted IDs, insertion order, RNG, or allocator history.

- [ ] **Step 4: Implement local validation and exact binding checks.**

  Keep wire/shape checks in the decision crate, including candidate width/range, canonical `SelectMany` order, `Order` semantic order, and variant/value equality between visible intent and trusted binding. Leave contextual legality in Rust rules/environment; do not add a conformance oracle to production.

- [ ] **Step 5: Register the Rust decoder, then promote only Decision V2 fixtures and run focused tests.**

  Add V2 dataclasses/readers/writers in `python/src/mtgml/decision.py`, reject JSON candidate values above `4294967295`, and keep V1 readers unchanged. In `crates/mtgml-wire/src/lib.rs`, implement and register the Decision V2 `WireContract` implementations and `decode_named()` cases before promoting any live fixture entry; the Rust verifier must recognize every Decision V2 contract ID that will be present in the manifests. Only after that decoder change is present, move the staged Decision V2 positive/negative fixture files into their live `wire/golden/`/`wire/negative/` locations, register them in the live manifests and active schema-parity mapping, and remove those files/entries from `wire/staging/m2-b/`. Do not promote Information V2 or Replay V3 entries here. Run:

  ```powershell
  cargo test -p mtgml-decision --locked
  cargo test -p mtgml-wire --locked
  python -m pytest python/tests/test_wire_contracts.py python/tests/test_schema_parity.py -q
  ```

- [ ] **Step 6: Commit Decision V2.**

  ```powershell
  git add crates/mtgml-decision crates/mtgml-wire schemas python/src/mtgml/decision.py python/src/mtgml/wire.py python/tests/test_schema_parity.py wire
  git commit -m "feat: add Decision V2 and canonical candidate ordering"
  ```

## Task 5: Prepare dormant typed M2 state structures (no current-runtime change)

**Files:**
- Create: `crates/mtgml-state/src/m2_shape.rs` as a private pre-cut module namespace
- Modify: `crates/mtgml-state/src/lib.rs` only to keep `m2_shape` private and detached before the cut
- Modify: `crates/mtgml-state/src/tests.rs` for detached M2-shape tests
- Do not modify current `EngineState` field types, current constructors, current `validate_engine_state()`, current `StateDelta`, rules consumers, or synthetic call sites in this task
- Test: detached state-shape unit tests and structural source checks

- [ ] **Step 1: Add red detached M2-shape tests.**

  Add tests against the dormant typed building blocks that reject a free-form continuation label, missing continuation reference, stale stage revision, duplicated/invalid active or retired opaque identity, missing player-decision allocator state, knowledge keyed by live `GameObjectId`, duplicated live opaque mapping, non-monotonic `VisibleSequence`, and reverse-map disagreement. Add a positive fixture with one typed synthetic continuation and active/retired knowledge records. The tests must not construct or reinterpret the current `EngineState`.

- [ ] **Step 2: Define the dormant `PendingDecisionRecordV2`/`AuthoritativeDecisionRequestV2` building blocks.**

  Define `PendingDecisionRecordV2` and the typed V2 pending-decision shape with co-located candidates without changing current `PendingDecisionRecord`. Keep the trusted `DecisionId` and perspective-local `PlayerDecisionIdV1` separate. Store only an optional trusted `ContinuationId` reference in the dormant shape. Current storage changes happen only in the atomic cut.

- [ ] **Step 3: Define the dormant bounded typed continuation payload.**

  Define the current M2 payload as:

  ```rust
  pub enum ContinuationPayloadV2 {
      SyntheticM2Assembly {
          stage: AssemblyStageV2,
          selected_count: Option<u32>,
          selected_piece_keys: Vec<u32>,
          ordered_piece_keys: Vec<u32>,
      },
  }

  pub struct ContinuationRecordV2 {
      pub id: ContinuationId,
      pub actor: PlayerId,
      pub created_at_revision: StateRevision,
      pub stage_index: u16,
      pub payload: ContinuationPayloadV2,
  }
  ```

  `EffectRecord`, `TriggerRecord`, and `delayed_effects` remain structurally present in the current state until the atomic cut, while the dormant M2 converter rejects non-empty unsupported values. No label interpreter or callback state is introduced.

- [ ] **Step 4: Define dormant `PerspectiveIdentityStateV2` allocator and retirement state.**

  Define `PerspectiveIdentityStateV2` with `next_player_decision_id`, `retired_object_ids`, and `retired_ability_ids` per player. Keep one canonical persisted mapping direction and runtime reverse maps only for bijection validation. Allocators must be checkpointed state and must not derive from global allocation history. Do not wire these fields into current `EngineState` yet.

- [ ] **Step 5: Define dormant `KnowledgeStateV2` records keyed by opaque identity.**

  Define `KnowledgeStateV2` with active/retired knowledge maps and records keyed by `OpaqueObjectId`, add `next_visible_sequence`, preserve typed provenance/history/invalidation, and exclude any duplicated live `OpaqueObjectId -> GameObjectId` association from knowledge. This task adds detached structural records and validation only; it does not implement later hidden-transition/randomization lifecycle semantics.

- [ ] **Step 6: Add a dormant M2-shape validator.**

  Validate co-located candidate/binding identity, dense candidate IDs, exact visible/trusted variants and payloads, valid continuation references/stages, allocator ceilings, mapping bijection/retirement, knowledge provenance and sequence order, and complete player coverage. Keep the first error category deterministic. Do not extend the current `validate_engine_state()` entry point until the atomic cut.

- [ ] **Step 7: Add a detached structural `ChooseOne`/`SelectOne` fixture.**

  Construct a valid detached V2 request with dense `CandidateIdV1` and a perspective-local `PlayerDecisionIdV1`. Do not change or execute the current synthetic transition path here. Do not execute `ChooseMany`, `ChooseNumber`, `Order`, or continuation advancement in this task.

- [ ] **Step 8: Turn the dormant state-shape tests green and commit only the dormant material.**

  ```powershell
  cargo test -p mtgml-state m2_shape --locked
  git add crates/mtgml-state
  git commit -m "feat: add dormant M2 state structures"
  ```

  This commit is permitted only if source inspection confirms that no current runtime producer or consumer has changed and the new types are reachable only through the private `m2_shape` module. The current EngineState migration is deferred to the one atomic cut in Tasks 7–11.

## Task 6: Prepare detached `FullStateDigestInputV3` (no current-runtime switch)

**Files:**
- Create: `crates/mtgml-state/src/digest_v3.rs`
- Modify: `crates/mtgml-state/Cargo.toml`
- Modify: `crates/mtgml-state/src/lib.rs`, `digest.rs`, `validation.rs`, `tests.rs` only for detached conversion and tests
- Do not change current `EngineState::digest()`, current `EngineState` field wiring, or any current transition/checkpoint producer
- Test: `crates/mtgml-state/src/tests.rs`, `persistence/golden/`

- [ ] **Step 1: Add red known-answer and mutation tests.**

  Build a minimal valid M2 state and assert the detached payload contains exactly the eleven fields from `docs/STATE_HASHING.md`: `full-state-digest-input.v3`, domain, revision, `core_v1`, `zones_v1`, `allocators_v3`, `execution_v2`, `random_v1`, `knowledge_v2`, `perspective_identities_v2`, and `format_v1`. Mutate every authoritative component one at a time and assert the V3 digest changes. Assert non-empty effects/triggers/delayed effects fail as `semantic_validation`.

- [ ] **Step 2: Implement detached conversion with explicit numeric sorting.**

  Add a detached conversion function over the dormant M2 state shape, returning a persistence-owned canonical value or typed detached input. Sort player/object/stack/knowledge/identity/format maps by declared semantic numeric keys, preserve semantic sequence order, encode IDs with declared widths, and encode only the one canonical mapping direction for perspective identity. Never use runtime Serde output as the V3 definition. Do not wire this conversion into the current `EngineState` yet.

- [ ] **Step 3: Implement the dormant V3 calculator exactly once through the persistence envelope.**

  Add a detached/testable V3 calculator that accepts the detached input, calls the single persistence encoder/envelope hash, and constructs `FullStateDigestV3::from_digest_bytes`. Keep the current `EngineState::digest()` and all current V2 producers unchanged until the atomic cut. The calculator must not call the historical `domain_digest!` path. Keep frozen V2 bytes/domain tests detached under explicit historical test names.

  The eventual current-runtime method will be wired only inside the atomic cut and will then have this shape:

  ```rust
  impl EngineState {
      pub fn canonical_digest_bytes(&self) -> Result<Vec<u8>, StateDigestError>;
      pub fn digest(&self) -> Result<FullStateDigestV3, StateDigestError>;
  }
  ```

  Those method signatures are a cut-time obligation, not a preparatory change. V2 historical tests must not be reachable as current `EngineState::digest()` behavior after the cut.

- [ ] **Step 4: Run detached V3 digest tests and commit only the dormant material.**

  ```powershell
  cargo test -p mtgml-state detached_state_digest_v3 --locked
  cargo test -p mtgml-persistence --locked
  git add crates/mtgml-state persistence
  git commit -m "feat: add dormant FullStateDigestV3 conversion"
  ```

  Before committing, verify that `EngineState::digest()` still has its starting V2 meaning and that no current checkpoint, replay, rules, or environment path consumes the dormant calculator.

## Task 7: Atomic-cut substep 1 — wire current state, digest, delta, and rules

**Files:**
- Modify: `crates/mtgml-state/src/engine.rs`, `execution.rs`, `identity.rs`, `knowledge.rs`, `validation.rs`, `construction.rs`, `digest.rs`, `delta.rs`, `lib.rs`, `tests.rs`
- Modify: `crates/mtgml-rules/src/transition.rs`, `contract.rs`, `validation.rs`, `tests.rs`
- Modify: `crates/mtgml-conformance/src/lib.rs`
- Modify: current environment/replay imports only as required by compiler errors
- Test: state delta, transition-contract, conformance, and source-boundary tests

- [ ] **Step 1: Add the red current-cut identity tests in the worktree.**

  Assert that the current `StateDelta` has `before_digest: FullStateDigestV3` and `after_digest: FullStateDigestV3`, `between()` records V3 identities, `apply()` rejects a mismatched V3 before/after identity, and replacement reapplication produces the same V3 digest. Add source-boundary assertions that the current state has no `EngineStateV2`/sidecar and that the current checkpoint/controller/replay changes are not left half-wired. These tests may be red while the cut is assembled, but no red state is committed.

- [ ] **Step 2: Wire the dormant M2 shape into the one current `EngineState` and switch its digest.**

  Promote the private `m2_shape::{PendingDecisionRecordV2, ContinuationRecordV2, KnowledgeStateV2, PerspectiveIdentityStateV2}` definitions into the one current `EngineState` representation, replacing the current unversioned field types in this same worktree. Then wire the detached V3 conversion into the current `EngineState::digest()` and `canonical_digest_bytes()`. Replace only current authoritative references to `FullStateDigestV2` with `FullStateDigestV3`. Preserve V1/V2 fixture readers and detached historical tests. Do not add `FullStateDigestV2 -> FullStateDigestV3` adapters, `LegacyEngineStateV2`, or a sidecar state.

- [ ] **Step 3: Change the current transition product and conformance expectations in the same worktree.**

  Make `RulesKernel` consume `DecisionResponseV2` together with the trusted actor supplied by the environment. Keep `TransitionResult` authoritative and state/event/delta-auditable:

  ```text
  next_state: EngineState
  events: AuthoritativeRuleEvent[]
  delta: StateDelta
  next_decision: Option<AuthoritativeDecisionRequestV2>
  status: EpisodeStatus
  ```

  The environment then binds/projects `next_decision` into the perspective-safe `PlayerDecisionRequestV2`; `TransitionResult` must never carry that player-safe projection. Update current conformance expectations to the V3 state/delta identity. Preserve only the structural `ChooseOne`/`SelectOne` synthetic path; do not add M2.C–H behavior.

- [ ] **Step 4: Run the partial-cut compiler/test loop without committing.**

  ```powershell
  cargo test -p mtgml-state -p mtgml-rules -p mtgml-conformance --locked
  rg -n "FullStateDigestV2|EnvironmentCheckpointV2|AuthoritativeReplayV2" crates/mtgml-state crates/mtgml-rules crates/mtgml-conformance
  ```

  Expected source result after the complete Tasks 7–11 worktree cut: no current `EngineState::digest`, current `StateDelta`, transition expectation, or conformance producer uses V2. At this substep, remaining compiler failures and V2 matches are expected until Tasks 8–11 complete. Do not commit this partial state.

## Task 8: Atomic-cut substep 2 — add semantic Information V2 and the wire-owned digest path

**Files:**
- Modify: `crates/mtgml-observation/src/lib.rs`
- Modify: `crates/mtgml-wire/Cargo.toml`, `crates/mtgml-wire/src/lib.rs`
- Modify: `crates/mtgml-environment/Cargo.toml`, `crates/mtgml-environment/src/synthetic.rs`, `endpoint.rs`, `tests.rs`
- Modify: `python/src/mtgml/observation.py`, `python/src/mtgml/wire.py`
- Modify: `wire/golden/manifest.json`, `wire/negative/manifest.json`, and the active schema-parity mapping only for Information/Event/PlayerStep V2 entries
- Test: observation, wire, environment endpoint tests and information KATs

- [ ] **Step 1: Add red semantic DTO tests in observation.**

  Add `PlayerKnownObjectV1`, location/provenance/invalidation values, `PlayerInformationStateV2`, `ObservedEventEnvelopeV2`, `PlayerStepV2`, and `InformationStateDigestInputV2` with exact fields from `docs/INFORMATION_MODEL.md` and `docs/STATE_HASHING.md`. `ObservationEnvelopeV1` remains the independently versioned current-observation payload. Test that the information DTO excludes physical/live authoritative IDs, other-player knowledge, status, environment counters, RNG, checkpoint/replay identity, and the digest field itself; V2 observed events use one `VisibleSequence` and no authoritative event ID.

- [ ] **Step 2: Implement observation-owned semantic validation.**

  Validate perspective/revision coherence, active/retired shape, one occurrence per opaque ID, strictly increasing historical visible sequences, required retired invalidation, and public-safe location fields. Do not import `mtgml-wire` and do not sort/encode JSON bytes here.

- [ ] **Step 3: Add the wire-owned digest operation.**

  Add:

  ```rust
  pub fn compute_information_state_digest_v2(
      input: &InformationStateDigestInputV2,
  ) -> Result<(Vec<u8>, InformationStateDigestV2), WireError>;
  ```

  The function must call the existing canonical JSON key-sorting/duplicate-rejection path, use `information-state-digest-input.v2` as the schema identity, apply `SHA256(ASCII("mtgml.information-state-digest.v2") || 0x00 || canonical_json)` exactly once, and return both bytes and the typed digest. No observation module or environment module may calculate a second digest.

- [ ] **Step 4: Make environment the only orchestrator/verifier in the same cut worktree.**

  Add the normal `mtgml-wire` dependency to `mtgml-environment`. The environment must project semantic input, call `mtgml-wire`, verify the returned digest before exposing or committing `PlayerInformationStateV2`, and map failures to the trusted/internal endpoint error. It must not implement canonicalization itself.

- [ ] **Step 5: Add mutation/exclusion and cycle tests.**

  Assert that mutations to perspective, revision, observation, next sequence, or retained knowledge change the digest; mutations to status, counters, trusted IDs, RNG, or replay/checkpoint fields do not. Add a source/dependency assertion that observation does not depend on wire and wire does not depend on environment.

- [ ] **Step 6: Promote Information V2 fixtures and run focused tests without committing.**

  Move only the staged Information V2, observed-event V2, and PlayerStep V2 fixtures into their live golden/negative locations, register them in the live manifests and active schema-parity mapping now that their semantic DTOs and wire decoders exist, and remove those files/entries from `wire/staging/m2-b/`. Do not promote Replay V3 entries here.

  ```powershell
  cargo test -p mtgml-observation -p mtgml-wire -p mtgml-environment information --locked
  python -m pytest python/tests/test_player_api.py -q
  ```

  These changes remain uncommitted until the Checkpoint V3, Replay V3, and endpoint substeps are coherent. Do not expose a SHA that has a new information surface but an old current controller or checkpoint contract.

## Task 9: Atomic-cut substep 3 — retire current V2 checkpoints and wire Checkpoint V3

**Files:**
- Modify: `crates/mtgml-environment/src/checkpoint.rs`, `controller.rs`, `replay.rs`, `synthetic.rs`, `errors.rs`, `lib.rs`, `tests.rs`
- Modify: `crates/mtgml-environment/Cargo.toml` to add the normal `mtgml-persistence` dependency
- Modify: `crates/mtgml-replay/Cargo.toml` to add the normal `mtgml-persistence` dependency
- Test: checkpoint construction, validation, restore nonmutation, and V2 retirement tests

- [ ] **Step 1: Add red checkpoint-retirement and V3 checkpoint tests.**

  Test the exact current fields:

  ```text
  schema_version = environment-checkpoint.v3
  state
  state_digest: FullStateDigestV3
  status: EpisodeStatus
  limit_counters: EnvironmentLimitCounters
  codec: CheckpointCodecIdentity
  checkpoint_digest: CheckpointDigestV3
  ```

  Assert a known-answer checkpoint digest, rejection for corrupt state/digest/status/counter/codec, and no backend mutation on rejected restore.

- [ ] **Step 2: Remove `EnvironmentCheckpointV2` from current runtime/controller APIs before wiring V3.**

  Change `EnvironmentBackend::checkpoint`, `restore`, `TrustedEnvironmentController::checkpoint`, `restore`, `execute_replay_from_checkpoint`, replay traces/reports, and synthetic backend construction away from V2 in the cut worktree. Delete the live V2 struct and its current Serde-derived state wrapper. Do not add `LegacyEngineStateV2` and do not adapt new `EngineState` into V2. Do not add a public checkpoint JSON schema or durable checkpoint file format in M2.B.

- [ ] **Step 3: Move Checkpoint V3 calculation to the shared persistence owner.**

  `EnvironmentCheckpointV3::new` and `validate` must call:

  ```rust
  mtgml_persistence::calculate_checkpoint_digest_v3(
      &state_digest.as_digest_reference(),
      &status,
      &limit_counters,
      &codec,
  )
  ```

  `mtgml-environment` must not retain `CheckpointDigestInputV2`, `calculate_digest`, or another V3 input encoder. `mtgml-replay` calls the same persistence operation in Task 10.

- [ ] **Step 4: Preserve V2 only as detached evidence.**

  Keep immutable V2 digest/domain vectors, documentation, and detached Replay V2 DTO validation. Move old V2 checkpoint assertions to a historical evidence test that never accepts a current `EngineState` and never feeds the current controller.

- [ ] **Step 5: Run checkpoint tests and the retirement search.**

  ```powershell
  cargo test -p mtgml-environment checkpoint --locked
  rg -n "EnvironmentCheckpointV2|LegacyEngineStateV2|FullStateDigestV2|CheckpointDigestV2" crates/mtgml-environment
  ```

  Remaining V2 matches must be detached evidence only. The current controller/backend API and current synthetic backend must expose V3 exclusively.

  Do not commit this substep. Replay and endpoint integration still belongs to the same atomic runtime cut.

## Task 10: Atomic-cut substep 4 — wire Replay V3 and complete identity chaining

**Files:**
- Create: `crates/mtgml-replay/src/v3.rs`
- Modify: `crates/mtgml-replay/Cargo.toml`
- Modify: `crates/mtgml-replay/src/lib.rs`, `identity.rs`, `validation.rs`, `recorder.rs`, `tests.rs`
- Modify: `crates/mtgml-environment/src/replay.rs`, `controller.rs`, `synthetic.rs`, `errors.rs`, `tests.rs`
- Modify: `crates/mtgml-wire/src/lib.rs`
- Modify: `schemas/replay-manifest.v3.schema.json`, `schemas/authoritative-replay.v3.schema.json`, `wire/golden/`, `wire/negative/`, `python/src/mtgml/replay.py`
- Modify: `wire/golden/manifest.json`, `wire/negative/manifest.json`, and the active schema-parity mapping only for Replay V3 entries
- Test: Replay V3 unit/integration tests and public schema/fixture tests

- [ ] **Step 1: Add red Replay V3 identity-chain tests.**

  Test empty, accepted, and rejected segments with these exact identities:

  ```text
  InitialEnvironmentIdentityV3:
      state_revision
      FullStateDigestV3
      EpisodeStatus
      EnvironmentLimitCounters
      CheckpointCodecIdentity
      CheckpointDigestV3

  ReplayStepV3:
      step_index
      actor
      checkpoint_digest_before
      state_revision_before
      DecisionResponseV2
      accepted
      state_revision_after
      FullStateDigestV3 after
      EpisodeStatus after
      EnvironmentLimitCounters after
      checkpoint_digest_after
  ```

  Assert step 0 starts at the manifest identity, each later step equals the prior after identity, accepted revisions advance, rejected steps preserve the complete checkpoint identity, and final identity equals the backend checkpoint.

- [ ] **Step 2: Implement Replay V3 detached DTOs and validation.**

  Add `ReplayManifestV3`, `InitialEnvironmentIdentityV3`, `ReplayStepV3`, `AuthoritativeReplayV3`, and `ReplayRecorderV3`. Validate schema identities, complete environment identity, actor binding as trusted replay input, V3 digest domains, response version, continuity, and final identity. V2 modules remain readable/verifiable detached DTOs only.

- [ ] **Step 3: Recompute checkpoint identities through `mtgml-persistence`.**

  Replay validation must call the shared `calculate_checkpoint_digest_v3` for initial, before, and after identities. It must compare the complete `DigestReferenceV1`, status, every counter, and codec identity. There is no duplicated checkpoint encoder in replay.

- [ ] **Step 4: Keep M2.B replay control narrow.**

  Do not add `ReplayControlV1`. For the preserved synthetic path, `resource_units_consumed` and `wall_clock_elapsed_millis` remain unchanged. Later external progression is outside this plan and would require an explicit versioned input.

- [ ] **Step 5: Update environment execution and tests.**

  Make `mtgml-environment/src/replay.rs` execute only Replay V3, validate before mutation, preserve rejected-step identity, and use the V3 checkpoint returned by the backend. Test no host wall-clock sampling, accepted/rejected parity, corrupt before/after status/counter/codec/digest rejection, and no mutation on failure.

- [ ] **Step 6: Register the Rust decoder, then promote Replay V3 fixtures and add JSON/schema/Python parity without committing.**

  In `crates/mtgml-wire/src/lib.rs`, implement and register the Replay V3 `WireContract` implementations and `decode_named()` cases before promoting any live fixture entry; the Rust verifier must recognize every Replay V3 contract ID that will be present in the manifests. Only after that decoder/validator change is present, move the staged Replay V3 fixtures into their live golden/negative locations, register them in the live manifests and active schema-parity mapping, and remove those files/entries from `wire/staging/m2-b/`. Keep all historical V1/V2 entries and readers intact. After this promotion, delete the empty `wire/staging/m2-b/manifest.json` and `wire/staging/m2-b/` directory; assert that no staged M2.B fixture remains.

  ```powershell
  cargo test -p mtgml-replay -p mtgml-environment --locked
  cargo test -p mtgml-wire --locked
  python -m pytest python/tests/test_schema_parity.py python/tests/test_wire_contracts.py -q
  ```

  Replay V3 detached types and current executor changes remain part of the same uncommitted cut worktree. Do not publish a Replay V3 SHA while the current endpoint/controller is still V1/V2-typed.

## Task 11: Atomic-cut substep 5 — finish the synthetic path and commit the one runtime cut

**Files:**
- Modify: `crates/mtgml-environment/src/synthetic.rs`, `endpoint.rs`, `controller.rs`, `errors.rs`, `tests.rs`
- Modify: `crates/mtgml-observation/src/lib.rs`, `crates/mtgml-wire/src/lib.rs`
- Modify: `python/src/mtgml/player_client.py`, `decision.py`, `observation.py`, `wire.py`
- Test: environment endpoint/controller tests and Python public-boundary tests

- [ ] **Step 1: Add red endpoint tests for the V2 public boundary.**

  Assert a bound player receives only `PlayerDecisionRequestV2`, `PlayerInformationStateV2`, observed/public values, and `PlayerStepV2`; the response contains only `PlayerDecisionIdV1`, current revision, and the closed answer. Assert trusted `DecisionId`, continuation, bindings, checkpoint/replay identity, RNG, raw state, and hidden fields never appear.

- [ ] **Step 2: Migrate synthetic reset/submit/checkpoint flow atomically.**

  On reset, construct valid M2 state, derive V3 full-state identity, construct V3 checkpoint, and initialize Replay V3. On accepted `ChooseOne`/`SelectOne`, let Rules produce the authoritative `TransitionResult` (`next_state`, ordered events, exact `StateDelta`, `next_decision: Option<AuthoritativeDecisionRequestV2>`, and `EpisodeStatus`). The environment validates that complete product, then projects the actor-bound authoritative request into `PlayerDecisionRequestV2`, computes the wire-owned information digest, and validates the checkpoint/replay/projection product before committing state, counters, replay, or exposed bytes. On rejection, return the closed typed rejection and preserve every state/identity/counter/sequence value.

- [ ] **Step 3: Keep unsupported M2.B behavior fail-closed.**

  A request for `ChooseMany`, `ChooseNumber`, `Order`, unsupported continuation stage, or later lifecycle behavior must not be silently executed, guessed, randomized, or transformed into M2.C–H behavior. Return the trusted/internal unsupported path while leaving state untouched.

- [ ] **Step 4: Prove the complete cut is coherent before creating its one SHA.**

  ```powershell
  cargo test -p mtgml-state -p mtgml-rules -p mtgml-conformance -p mtgml-observation -p mtgml-wire -p mtgml-replay -p mtgml-environment --locked
  python -m pytest python/tests/test_player_api.py python/tests/test_wire_contracts.py python/tests/test_schema_parity.py -q
  rg -n "EngineStateV2|LegacyEngineStateV2|ReplayControlV1|EnvironmentCheckpointV2|AuthoritativeReplayV2|FullStateDigestV2" crates python schemas wire
  ```

  The exact source review must classify every remaining V1/V2 match as detached historical evidence, an immutable fixture, or a detached reader. There must be no current V2 producer, current V2 checkpoint API, partial controller/replay path, or second digest owner. If any focused check is red, continue fixing the same worktree; do not commit.

- [ ] **Step 5: Commit the single atomic current-runtime cut.**

  ```powershell
  git add crates/mtgml-state crates/mtgml-rules crates/mtgml-conformance crates/mtgml-observation crates/mtgml-wire crates/mtgml-replay crates/mtgml-environment python/src/mtgml python/tests schemas wire
  git diff --cached --check
  git commit -m "feat: perform atomic M2.B current runtime V3 cut"
  ```

  This is the only commit that changes current `EngineState`, current digest identity, `StateDelta`, rules transition products, checkpoint/controller APIs, Replay V3 execution, environment backend, or the synthetic endpoint. Tasks 7–11 must not have produced any earlier SHA for those changes.

## Task 12: Add mechanical Python persistence parity without adding a Python rules engine

**Files:**
- Create: `python/src/mtgml/persistence.py`, `python/tests/test_persistence_codec.py`
- Modify: `python/src/mtgml/__init__.py`, `python/src/mtgml/canonical.py`
- Modify: `python/tests/test_wire_contracts.py`, `python/tests/test_schema_parity.py`

- [ ] **Step 1: Add red vector tests before the Python implementation.**

  Load every cross-language persistence manifest entry, compare Python-produced canonical CBOR/envelope bytes and lowercase digest text to the committed Rust vectors, and assert only the shared mechanical negative fixtures report the same category. The cross-language negative manifest may cover malformed CBOR, canonicality, envelope identity/length, resource limits, primitive ranges, UTF-8, detached-schema shape, duplicate/order, trailing data, and re-encode mechanics. It must not contain `EngineState`, checkpoint, replay, continuation, knowledge, or other `semantic_validation` cases. Do not enumerate `persistence/rust-negative/` from Python; those fixtures are Rust-authoritative semantic evidence. Add a test that Python cannot construct or validate `EngineState`, transition legality, knowledge lifecycle, or migration semantics.

- [ ] **Step 2: Implement only mechanical persistence operations.**

  Mirror the restricted primitive encoder/decoder, envelope framing, SHA-256, digest-reference rendering, and fixture reader using only the Python standard library. Preserve exact UTF-8 bytes, length/depth/item limits, canonical re-encode behavior, and error-category order for the shared mechanical categories. The module must accept already detached semantic values; it must not derive them from runtime state or duplicate Rust validation. No Python package dependency is added for CBOR.

- [ ] **Step 3: Run Python parity and lint/type checks.**

  ```powershell
  python -m pytest python/tests/test_persistence_codec.py python/tests/test_wire_contracts.py -q
  ruff format --check python scripts
  ruff check python scripts
  mypy --config-file python/pyproject.toml
  ```

- [ ] **Step 4: Commit mechanical parity.**

  ```powershell
  git add python
  git commit -m "feat: add mechanical Python persistence parity"
  ```

## Task 13: Synchronize schemas, generated vocabulary, fixtures, and historical classifications

**Files:**
- Modify: `contracts/catalog/contract-vocabulary.v1.json`, `scripts/generate_contracts.py` only for mechanically shared identifiers
- Regenerate: `crates/mtgml-model/src/generated_contract_vocab.rs`, `python/src/mtgml/_generated_contract_vocab.py`, generated schema/docs outputs
- Modify: all V2/V3 schema and manifest files, `wire/golden/`, `wire/negative/`, `persistence/golden/`, `persistence/negative/`, `persistence/rust-negative/`
- Modify: `docs/STATE_HASHING.md`, `docs/REPLAY_AND_DETERMINISM.md`, `docs/contracts/WIRE_CONTRACT.md`, `docs/ERROR_MODEL.md`, `docs/PROJECT_STRUCTURE.md`, `docs/ARCHITECTURE.md`, `crates/README.md`
- Test: generator drift, schema validation, Rust/Python fixture verification, documentation checks

- [ ] **Step 1: Update authoritative sources before generated outputs.**

  Add only stable vocabulary that is mechanically duplicated across Rust/Python/schema. Keep semantic ordering, information-flow policy, state invariants, persistence layouts, and cross-field validation in explicit reviewed Rust/Python logic rather than the catalog generator.

- [ ] **Step 2: Regenerate and verify no drift.**

  ```powershell
  python scripts/generate_contracts.py
  python scripts/generate_contracts.py --check
  ```

  Expected result: the generator reports that catalog-derived outputs match the source catalog. Hand-edited generated files are forbidden.

- [ ] **Step 3: Add negative evidence for version retirement and unsupported scope.**

  Cover V2 checkpoint current-runtime rejection, V2 full-state/replay current-producer absence, CandidateId overflow, digest double-domain-separation, duplicate/order/limit/CBOR precedence, corrupt checkpoint identity, replay identity divergence, public privileged-field injection, and malformed wire bytes producing no semantic PlayerStep. Classify codec/envelope/detached-schema negatives in the cross-language manifest; classify EngineState/checkpoint/replay/continuation/knowledge and `semantic_validation` negatives in `persistence/rust-negative/` and verify them only through Rust-authoritative tests. Python must not be required to reproduce these semantic decisions.

- [ ] **Step 4: Document the implementation boundary without upgrading status.**

  State that `mtgml-model` owns the moved leaves but their V2 Serde meaning is byte-identical; `mtgml-persistence` owns the checkpoint calculator; `mtgml-wire` owns Information V2 canonical bytes; no public checkpoint/full-state JSON or durable checkpoint file format is added; M2.C–H remain out of scope; and the baseline environment evidence remains `BLOCKED` when Hyper-V/WSL is unavailable.

- [ ] **Step 5: Run contract checks and commit.**

  ```powershell
  python scripts/verify_repository.py
  python scripts/check_rust_source_structure.py
  python scripts/check_documentation.py
  python scripts/validate_schemas.py
  python scripts/validate_golden_path.py
  git add contracts scripts crates/mtgml-model/src/generated_contract_vocab.rs python/src/mtgml/_generated_contract_vocab.py schemas wire persistence docs crates/README.md
  git commit -m "docs: synchronize M2.B contracts and historical evidence"
  ```

## Task 14: Add conformance, transition, and regression evidence

**Files:**
- Modify: `crates/mtgml-conformance/src/lib.rs` and tests added beside relevant crates
- Modify: `crates/mtgml-state/src/tests.rs`, `crates/mtgml-environment/src/tests.rs`, `crates/mtgml-replay/src/tests.rs`, `crates/mtgml-wire/src/lib.rs` tests
- Modify: `python/tests/test_player_api.py`, `test_wire_contracts.py`, `test_schema_parity.py`
- Modify: `persistence/negative/manifest.json`, `persistence/rust-negative/manifest.json`
- Create: `scripts/run_m2_b_contract_cut.py`
- Modify: `scripts/run_m1_closure.py` only if the current V3 names require re-pointing its M1 regression `GATE_TESTS`; preserve the M1 gate names, historical report format, and existing historical V2 evidence
- Modify: `.github/workflows/pr-fast.yml` to invoke the M2.B slice runner after the pinned Rust toolchain is installed

### Executable gate owner

Create `scripts/run_m2_b_contract_cut.py` as the sole executable owner of:

```text
M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT
```

It must follow the `scripts/run_m1_closure.py` pattern without becoming M2.Final closure tooling. The runner must have an explicit `GATE_TESTS` table with these exact named Rust tests:

```text
mtgml-model:
  tests::m2_b_candidate_id_is_u32_and_v3_digest_is_raw

mtgml-persistence:
  tests::canonical_cbor_v1_complete_profile_matrix
  tests::digest_envelope_v1_known_answer_matrix
  tests::checkpoint_digest_v3_known_answer

mtgml-decision:
  tests::candidate_ordering_v1_exact_matrix
  tests::candidate_id_overflow_is_rejected

mtgml-state:
  tests::full_state_digest_v3_known_answer
  tests::m2_b_full_state_digest_v3_mutation_matrix
  tests::state_delta_uses_full_state_digest_v3
  tests::deterministic_structural_identity_repeats_exactly

mtgml-rules:
  tests::synthetic_m2_choose_one_returns_authoritative_transition_product

mtgml-observation:
  tests::information_state_input_excludes_trusted_fields

mtgml-wire:
  tests::information_state_digest_v2_known_answer

mtgml-environment:
  tests::checkpoint_v3_validation_and_restore_nonmutation_matrix
  tests::synthetic_endpoint_returns_v2_surface

mtgml-replay:
  tests::replay_v3_empty_accepted_rejected_identity_matrix

historical/source evidence owned by the runner:
  source_check::v1_v2_fixtures_are_immutable
  source_check::no_current_v2_producer
```

The matrix tests are not placeholders for smaller unowned checks. `candidate_ordering_v1_exact_matrix` must cover numeric ordering (`2 < 10`), dense IDs, duplicate public-key rejection, `ChooseNumber` with no candidates, and noncanonical ordering rejection. `canonical_cbor_v1_complete_profile_matrix` and `digest_envelope_v1_known_answer_matrix` must cover the complete accepted/forbidden profile and exact envelope bytes/hash. `checkpoint_v3_validation_and_restore_nonmutation_matrix` must cover all current checkpoint fields, corrupt state/digest/status/counter/codec rejection, and restore nonmutation. `replay_v3_empty_accepted_rejected_identity_matrix` must cover empty, accepted, and rejected identity chains, complete rejected-step identity preservation, continuity, and final identity equality. `v1_v2_fixtures_are_immutable` must read `wire/golden/manifest.json` and `wire/negative/manifest.json` from starting SHA `a4e769eb940611d34df05fc79effd9430891d897`, derive the complete baseline V1/V2 fixture set, and require exact set equality with `wire/historical/v1-v2-fixtures.json`—no missing, extra, or duplicate inventory entries—before comparing current fixture bytes/hashes to that baseline. Active manifests may append newly promoted M2.B V2/V3 entries, but such additions must not change the historical inventory or baseline comparison. The historical/source family must reject every current V2 producer outside detached readers/evidence. The deterministic structural identity test is the explicit identity-repeat member of the same controlled set.

The runner must also execute these exact Python/fixture checks:

```text
python/tests/test_persistence_codec.py::test_cross_language_mechanical_golden_vectors
python/tests/test_persistence_codec.py::test_cross_language_mechanical_negative_categories
python/tests/test_schema_parity.py::test_m2_b_detached_schema_fixtures
python/tests/test_player_api.py::test_v2_public_boundary_excludes_privileged_fields

python scripts/generate_contracts.py --check
python scripts/check_rust_source_structure.py
python scripts/check_documentation.py
python scripts/validate_schemas.py
python scripts/validate_golden_path.py
```

The Rust entries use `cargo test --package <package> --locked --lib -- <exact-test-name> --exact`; the Python entries use exact pytest node IDs; `source_check::...` entries are named read-only checks implemented by the runner itself and must produce their own logs/evidence. The runner must record each command/check, return code where applicable, log path, and status. It must additionally record the source SHA/tree, clean-tree status, pinned Python/Rust toolchain identity, host identity, and an overall `PASS`, `FAIL`, `BLOCKED`, or `NOT_RUN`. The default mode is authoritative and requires a clean source tree. An explicit `--development` mode may run the same underlying checks while the worktree is dirty, but its report must set `mode=development` and its gate status to `NOT_RUN` regardless of underlying test results; it must never emit `M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT = PASS`. Development mode may exit successfully only when its underlying checks pass; that exit status is not an authoritative gate result. The runner must accept `--output-dir` like `run_m1_closure.py`, default to the external marker-owned sibling directory `dist/m2-b-verification/`, and write only there (`logs/`, JSON report, Markdown report). A custom output directory must not be `dist/verification/` or any descendant, because that release root is exclusively owned by `scripts/run_verification.py`; the runner must never rewrite source or convert an unavailable tool/environment into `PASS`.

The authoritative runner's overall gate is `PASS` only when the source tree is clean and every named test/check, source identity check, and toolchain check is `PASS`. A missing tool is `NOT_RUN`; an execution/environment failure such as unavailable WSL/Hyper-V is `BLOCKED`; an executed failing test is `FAIL`. Rust-authoritative semantic negatives from `persistence/rust-negative/` are covered by the named Rust state/checkpoint/replay tests and are not part of Python parity.

Replace the bare checkout in `.github/workflows/pr-fast.yml` with an exact PR-head, full-history checkout before setup/toolchain steps:

```yaml
- uses: actions/checkout@v7
  with:
    ref: ${{ github.event.pull_request.head.sha }}
    fetch-depth: 0
```

Immediately after checkout, assert that the workspace is on that same head before running any gate:

```yaml
- name: Assert exact pull request head
  shell: bash
  run: test "$(git rev-parse HEAD)" = "${{ github.event.pull_request.head.sha }}"
```

This makes the runner's recorded source commit equal to the PR `headRefOid` and guarantees that starting SHA `a4e769eb940611d34df05fc79effd9430891d897` is available for the historical-fixture source check. Add the M2.B slice step after the existing pinned toolchain-install step (and before the final handoff review):

```yaml
- run: python scripts/run_m2_b_contract_cut.py
```

This keeps the gate executable on Linux CI while preserving the existing `pull_request` trigger; it is still a slice gate for Issue #49, not M2.Final closure.

- [ ] **Step 1: Prove decision and state structural evidence.**

  Cover numeric `2 < 10`, dense IDs, duplicate ordering keys, CandidateId overflow, variant mismatch, `ChooseNumber` candidate rejection, typed continuation/reference/allocator validation, state mutation coverage, and StateDelta V3 reapplication.

- [ ] **Step 2: Prove codec and digest evidence.**

  Cover all accepted CBOR primitives, every forbidden form, resource-limit-before-allocation behavior, all precedence cases, re-encode equality, trailing-data rejection, envelope SHA-256 vectors, FullStateDigestV3 mutation vectors, InformationStateDigestV2 inclusion/exclusion vectors, and CheckpointDigestV3 known-answer/corruption vectors.

- [ ] **Step 3: Prove checkpoint/replay/nonmutation evidence.**

  Cover restore failure with no mutation, empty/accepted/rejected Replay V3 chains, complete rejected identity preservation, final identity equality, no wall-clock sampling, fork/checkpoint/replay parity, and exact synthetic M1 `ChooseOne`/`SelectOne` regression behavior.

- [ ] **Step 4: Prove cross-language public and mechanical parity.**

  Run Rust/Python canonical JSON fixture parity, V2/V3 schema parity, Python persistence byte/digest parity for `persistence/negative/` only, historical V1/V2 fixture immutability, and player-boundary privileged-field exclusion. Run the Rust-authoritative semantic negatives from `persistence/rust-negative/` only through Rust state/checkpoint/replay validation. Do not claim paired-state noninterference closure; that is outside M2.B.

- [ ] **Step 5: Run the underlying gate checks in development mode and commit the gate/evidence changes.**

  ```powershell
  python scripts/run_m2_b_contract_cut.py --development
  git add crates python schemas wire persistence scripts/run_m2_b_contract_cut.py scripts/run_m1_closure.py .github/workflows/pr-fast.yml docs
  git commit -m "test: add M2.B executable contract-cut evidence"
  ```

  The development report may show underlying test/check `PASS` entries, but its gate status is `NOT_RUN` because the worktree is dirty. Fix every red underlying check before this commit; do not treat the development report as authoritative evidence and never commit an expected failure. The runner report remains external evidence and is not added to the source tree.

## Task 15: Final exact-head verification and handoff

**Files:**
- Modify only if a verification-discovered contract/documentation defect is real: the exact affected source file
- External output only: `dist/m2-b-verification/`, `dist/m1-regression/`, and `dist/verification/` (the last one is exclusively owned by `just release-candidate` / `scripts/run_verification.py`)

- [ ] **Step 1: Inspect the final diff and source boundary.**

  ```powershell
  git status --short --branch
  git diff --check a4e769eb940611d34df05fc79effd9430891d897...HEAD
  git diff --stat a4e769eb940611d34df05fc79effd9430891d897...HEAD
  rg -n "EngineStateV2|LegacyEngineStateV2|ReplayControlV1|mtgml-state.*mtgml-wire|EnvironmentCheckpointV2" crates python schemas wire docs
  ```

  Review every remaining V2 match and classify it as immutable fixture, detached reader/verifier, documentation, or an illegal current producer. Stop on any illegal producer, second digest authority, current checkpoint V2 API, public checkpoint JSON contract, or M2.C–H behavior.

- [ ] **Step 2: Run all local/native exact-head evidence before any remote operation.**

  The following is the first authoritative execution of the M2.B gate; Task 14's development-mode run can never satisfy this requirement:

  Keep the M2.B and M1 regression reports in sibling directories outside the release-owned root. The M2.B runner writes to `dist/m2-b-verification/`, the M1 runner writes to `dist/m1-regression/`, and only `just release-candidate` may create or replace `dist/verification/`. This prevents either slice runner from colliding with `scripts/run_verification.py`'s root marker and exclusive output ownership.

  ```powershell
  python scripts/run_m2_b_contract_cut.py --output-dir dist/m2-b-verification
  python scripts/run_m1_closure.py --output-dir dist/m1-regression
  cargo fmt --all -- --check
  cargo test --workspace --all-features --locked
  python scripts/generate_contracts.py --check
  python scripts/verify_repository.py
  python scripts/check_rust_source_structure.py
  python scripts/check_documentation.py
  python scripts/validate_schemas.py
  python scripts/validate_golden_path.py
  python scripts/run_python_tests.py
  ```

  Record each command as `PASS`, `FAIL`, `NOT_RUN`, or `BLOCKED`. A command that cannot execute because of WSL/Hyper-V is `BLOCKED`; it is not a pass by inference from another command. Run the repository profiles on this same exact SHA:

  ```powershell
  just check-fast
  just check
  just check-all
  just release-candidate
  ```

  If the current V3 naming requires it, update only the current regression mapping in `scripts/run_m1_closure.py`; do not rewrite historical M1 V2 reports or silently drop a gate. If the Bash/WSL wrapper remains unavailable, preserve that result and attach native fallback evidence. The release/archive check is the last source-changing operation; do not modify archived source afterward. Both the M2.B and M1 runner reports must show the same `git rev-parse HEAD` as this verification pass, and their sibling output directories must remain separate from the release-owned `dist/verification/` directory.

- [ ] **Step 3: Perform the exact-head self-review before push.**

  ```powershell
  git status --short --branch
  git rev-parse HEAD
  git diff --check a4e769eb940611d34df05fc79effd9430891d897...HEAD
  ```

  Review the exact final SHA against Spec v3 and this plan. Confirm that Tasks 7–11 produced one runtime-cut commit, that the M2 runner names and records the intended tests, moved leaves remain Serde-identical, Python covers mechanics only, and no source change remains after the archive/reproducibility check. Do not push until this self-review is complete.

- [ ] **Step 4: Push the reviewed branch and open the draft PR.**

  The hosted check cannot exist before the PR because `.github/workflows/pr-fast.yml` is triggered only by `pull_request`. Run:

  ```powershell
  git push --set-upstream origin chris/m2-b-v3-structural-cut
  gh pr create --draft --base master --head chris/m2-b-v3-structural-cut --title "Issue #49: atomic M2.B V3 structural cut" --body "Implements the approved Spec v3 structural/versioning cut. Remains unmerged pending exact-head review."
  gh pr view <number> --json headRefOid,baseRefName,isDraft,state
  ```

  Confirm that `headRefOid` equals the locally reviewed SHA before treating any hosted result as relevant.

- [ ] **Step 5: Inspect hosted PR Fast evidence on the exact PR head.**

  ```powershell
  gh pr checks <number> --watch
  gh pr view <number> --json headRefOid,statusCheckRollup
  ```

  Wait for the PR-triggered `PR Fast` run and inspect its logs/results, including the M2.B slice-runner report. Do not transfer a result from another SHA. If the check fails, is missing, or the head SHA changes, the plan is not complete; fix/re-run from the local exact-head steps and obtain a new PR-triggered result.

- [ ] **Step 6: Perform the independent final-head review after hosted evidence and leave unmerged.**

  Re-read the final diff and compare `git rev-parse HEAD` with the PR `headRefOid` after all hosted checks finish. Confirm the single runtime-cut commit boundary, V2 checkpoint retirement before its V3 wiring within the cut, both single-owner digest paths, Rust-only semantic-negative ownership, and the exact M2.B scope. Leave the draft PR open and unmerged. Any source change after PR Fast requires a new local exact-head pass and a new PR-triggered run; no prior hosted result remains valid.

## Commit sequence

The implementation should normally produce these focused commits, each green under its narrowest available test command before the next. Tasks 7–11 deliberately produce one runtime commit; the omitted substep commits are forbidden:

```text
test: freeze M2.B public fixture boundaries
feat: add M2 shared identities and digest values
feat: add strict persisted semantic codec
feat: add Decision V2 and canonical candidate ordering
feat: add dormant M2 state structures
feat: add dormant FullStateDigestV3 conversion
feat: perform atomic M2.B current runtime V3 cut
feat: add mechanical Python persistence parity
docs: synchronize M2.B contracts and historical evidence
test: add M2.B executable contract-cut evidence
```

The plan is complete only when the final exact-head review has evidence for the requested structural/versioning cut, the named `M2_EXECUTABLE_CONTRACT_AND_VERSION_CUT` report, and a hosted `PR Fast` result on the exact draft-PR head. It is not complete merely because the workspace compiles, Python tests pass, or the design documents look consistent.
