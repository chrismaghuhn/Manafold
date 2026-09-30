# Old Formats Cleanup (Stage 2) — Design

**Status:** IMPLEMENTED on branch `chore/old-formats`, 2026-09-30 (approved by the owner; executed inline).

**Goal:** delete every historical format that no current type and no production path reaches, so that R1/W1 card work starts on a base where each format exists once.

**Behavior:** unchanged. The current format stays byte-identical. The only fixture that changes is `persistence/golden/semantic-contract-kat.v1.json`, which loses its two synthetic-legacy cases (D4).

## 1. Why

- **Stage 1 left one runtime, but many formats.** Replay V1–V7, checkpoint V4–V7, decision requests V1–V3, player steps and event envelopes V1–V3, and old observation payloads still compile, have tests, schemas, fixtures and Python modules. Nothing in production writes or reads them.
- **They cost on every change.**
  - Gates pin their files: `verify_repository.py` requires tokens that exist only in replay V1/V2 code, and `validate_golden_path.py` requires ten V1 wire goldens.
  - The Python client carries nine historical modules and seventeen modules that mix historical and current parts.
- **Size:**
  - Rust: about 13,000 lines (16 whole source files, 14 integration-test files, and parts of about 28 mixed files).
  - Python: about 1,800 lines of pure historical modules, 4,300 lines of their tests, and the historical parts of 17 mixed modules.
  - Fixtures: 33 of 54 schemas, 51 of 80 wire goldens, 93 of 120 wire negatives.
- **The semantic contract catalog is test-only.** Stage 1 kept it because the checkpoint V7/V8 tests use its synthetic-legacy fixtures; no production code reads it.

## 2. Stage position

This is stage 2 of three (`docs/superpowers/specs/2026-09-30-one-runtime-cleanup-design.md` §2). Stage 1 (one runtime) is merged (PR #263). Stage 3 disentangles the current format's internal layering; it is out of scope here (§5).

## 3. Decisions

- **D1 — Delete what nothing current reaches; leave mixed files in place.**
  - A module, type, function, schema, fixture or Python module is deleted when neither the production path nor a current-format type reaches it.
  - In a file that mixes historical and current items, only the historical items go. The current items stay in their file under their name, even when the file name carries an old version (for example, `mtgml-replay/src/v7.rs` keeps only what Replay V8 embeds). Moving and renaming is stage 3.
- **D2 — Deletion runs per format family, vertically.** Each family is removed across Rust, wire codecs, schemas, examples, negative fixtures, wire and persistence fixtures, Python modules, Python tests and the gate pins that name its files, in one commit that builds and passes the fast gate. The families and their order are listed in §4.
- **D3 — The semantic contract catalog is deleted.** This covers the catalog JSON, generator, generated module, KAT module, their Python test, the `justfile` check line and its profile pin, and the environment re-exports. The checkpoint V8 tests move to the real basic-land admission (`new_for_basic_land_profile`, `restore_with_verified_contracts_for_basic_land_profile`).
- **D4 — The synthetic-legacy path is deleted.**
  - `RulesAuthorityV1::SyntheticLegacy` and `ExecutionProgramV1::SyntheticRulesCompat` are removed, together with every match arm, parser branch, Python mirror and the synthetic branch of checkpoint V8 program validation.
  - `persistence/golden/semantic-contract-kat.v1.json` loses its two synthetic cases (`rules_synthetic_legacy`, `semantic_synthetic_null_null`); its other cases stay byte-identical.
  - The non-profile checkpoint V8 API (`new`, `restore_detached`, `restore_with_verified_contracts`, `fork_detached`) is deleted if no remaining test needs it after the switch in D3.
- **D5 — Gates follow the files.**
  - Text pins that exist only to require historical files or tokens are deleted, as in stage 1. These are the replay-token, required-file and observed-event checks in `verify_repository.py`, and the historical schema names in `test_schema_parity.py`.
  - `validate_schemas.py`, `schemas/README.json` and the wire manifests list only what remains.
  - `generate_contracts.py` stops generating `schemas/observed-event-envelope.v1.schema.json`. Vocabulary in `contracts/catalog/contract-vocabulary.v1.json` that only that schema or other deleted formats used is removed.
  - The golden-path example keeps its census and certification checks; its `wire_fixtures` point to current wire goldens instead of the ten V1 goldens.
- **D6 — Normative documents describe only current formats.**
  - Sections that specify deleted formats are deleted, not marked. Git history keeps them.
  - Sections whose bytes the current format still produces stay, even under a historical heading, for example the `FullStateDigestInputV3` component sections of `docs/STATE_HASHING.md` that V7 hashes through V6/V5.
  - Process documents (`docs/superpowers/**`, `docs/reviews/**`) are history and stay unchanged. The one exception is the stage-1 spec and plan, which each get one line recording that the catalog moved to stage 2.
  - `docs/PROJECT_STRUCTURE.md` and `docs/TESTING_AND_CONFORMANCE.md` stop describing `mtgml-conformance` as current. This was deferred from stage 1.
- **D7 — Tests** follow stage 1's rule:
  - a test that exercises only deleted code is deleted;
  - a test of remaining code that uses a deleted type is ported;
  - the plan names, for every remaining module of the touched crates, the tests that still cover it.

## 4. What is deleted (inventory, in commit order)

"Stays" names the current items kept in a mixed file.

Consumers are removed before what they consume, so every commit builds: replay uses the synthetic variants, historical digests and decision responses; checkpoint V7 tests use the catalog; player steps V1–V3 embed decision requests V1–V3.

1. **Replay V1–V7.**
   - Delete:
     - `mtgml-replay` `v1.rs`, `v3.rs`, `v4.rs`, `v5.rs`, `v6.rs`, `manifest.rs`, `recorder.rs`, `src/tests.rs`
     - `tests/gen_v5_fixtures`, `gen_v6_fixtures`, `p0_red`, `replay_v5_red`, `replay_v6_red`, `replay_v7_red`
     - the replay-only items of `v2.rs` and `v7.rs`; stays: `RandomnessIdentityV2` and `deserialize_root_seed_hex` from `v2.rs`, and `ContentContractMaterialV1`, `SemanticContractMaterialV7` and their limits from `v7.rs`
     - the V1/V4 items of `identity.rs`; stays: `KernelIdentityV1` and `DeckIdentityV1`
     - `ReplayValidationError` variants nothing constructs afterwards
     - the replay V1–V7 wire codecs, schemas, examples, negatives and wire fixtures
     - the Python `_replay_v1` and `_replay_v3` modules, and the historical parts of `_replay_v2` and `_replay_v4`–`_replay_v7` (stays: what `_replay_v8` imports)
     - their tests
2. **Checkpoint V4–V7 and historical digests.**
   - Delete:
     - environment `checkpoint.rs` (V4–V6) and `checkpoint_v7.rs`
     - the `ControllerError::{CheckpointValidation, CheckpointV7}` variants
     - `tests/checkpoint_v5_red.rs` and `tests/checkpoint_v6_red.rs`
     - checkpoint digests V3–V7 in `mtgml-persistence/src/checkpoint_digest.rs` (stays: V8 and its helpers)
     - `mtgml-state` `digest_v3.rs` (dead) and `digest_v4.rs`
     - the V5 and V6 digest producers and verifiers (`calculate_full_state_digest_v5*`, `calculate_full_state_digest_v6*`, `verify_full_state_digest_v6`, `EngineState::digest`); stays: the encoders V7 calls, `canonical_state_bytes_v6_with_execution_v3`, `full_state_digest_input_v5`, `zone_location_value`, the V6 domain constants and `FullStateDigestInputV6::from_canonical_value`
     - the `StateDelta` and `StateDeltaV2` structs and their application errors; stays: `SemanticDeltaOperation` and `SemanticDeltaOperationV2`, which `StateDeltaV3` embeds
     - the historical digest newtypes in `mtgml-model` that nothing uses afterwards
     - the V5–V7 checkpoint-digest and V6 state-digest KATs and negatives
     - the Python checkpoint-digest V3–V7 functions
     - the tests of all of these
3. **Catalog and synthetic path (D3, D4).**
   - First, the checkpoint V8 tests move to the basic-land admission.
   - Then delete:
     - `contracts/catalog/semantic-contracts.v1.json`
     - `scripts/generate_semantic_contract_catalog.py`
     - `crates/mtgml-environment/src/semantic_catalog_generated.rs` and `semantic_catalog_kat.rs`
     - `python/tests/test_semantic_contract_catalog_generator.py`
     - the synthetic variants and branches
     - the two KAT cases
4. **Observation, steps and events V1–V3, and old observation payloads.**
   - Delete:
     - `PlayerStep` (V1) and `PlayerStepV2`; `player_step_v3.rs`
     - `observed_event.rs` (V1/V2)
     - the V3 event envelope and kind, and the V2→V3 conversion
     - `InformationStateEnvelope` (V1)
     - `MagicObservation` and `MagicObservationV2`–`V4` with their combat structures
     - `SyntheticObservation` and its schema constant
     - the matching wire codecs, schemas (including the magic-m3, combat and synthetic payload codecs), examples, negatives and fixtures
     - the Python `_player_step_v3`, `_events_v2`, `_information_v2` and `_magic_combat_observation*` modules, and the historical parts of `_player_step_v2`, `_observation_v1`, `_events_v3`, `_magic_observation` and `_synthetic_observation`
     - their tests
   - Stays:
     - the V1 `ObservationEnvelope`, `PlayerInformationStateV2`, `InformationStateDigestInputV2` and `compute_information_state_digest_v2`, because the V3 information state is produced through them
     - `PlayerStepSubmissionV1` and the submission and error codes
     - the V3 event leaf types that V4 events use (`ObservedCounterKindV3`, `ManaPoolAfterV1`, `ObservedFaceV1`)
     - the basic-land observation V1 parts and the synthetic step enums V4 uses
5. **Decision V1–V3, player-facing.**
   - Delete:
     - the player decision requests V1–V3 and the responses V1 and V2
     - `AuthoritativeDecisionRequestV2/V3::project_player_request`
     - `ExecutionStateV3::selected_bindings`
     - the historical wire codecs, including `decision_response_v2`
     - the schemas, examples, negatives and wire fixtures
     - Python `decision_v3` and the historical parts of `decision.py`
     - their tests
   - Stays: response V3, request V4, and the V1/V2/V3 decision types that state, rules and V4 embed (`ActionCandidate`, `DecisionDomainV2`, `DecisionAnswerV2`, `VisibleCandidateV2`, the V3 candidate and authoritative request types).
6. **Documents (D6)**, the remaining gate adjustments, and final verification.

## 5. What stays for stage 3

- The current format's layering:
  - `EngineStatePartsV3` built on `EngineState`/`EngineStatePartsV2`
  - `FullStateDigestV7` computed through the V6/V5 encoders
  - `StateDeltaV3` on the V2/V1 operation enums
  - the V3 information state through the V1 envelope and the V2 digest
  - basic-land logic on V2/V3 types
  - rule events V2→V3
- The current items left in historical-version files by D1, and every version suffix on current type names.
- `mtgml-state/src/construction.rs` (the synthetic state builder that current-runtime tests use).
- `EngineState::digest` and the V5 digest producer, kept during implementation as a test probe of the V5 input encoding (it is not the state identity; see its doc comment).

## 6. Verification

- **Byte identity:** the current fixtures are byte-identical to `master`:
  - replay V8
  - player decision request V4 and player step V4
  - observed event V4
  - information state V3 and observation envelope V2
  - episode status
  - shared-execution observation
  - `checkpoint-digest-v8-kat`, `full-state-digest-v7-kat`, `m4-g0c-digest-identity-fixtures`
  - content and CBOR codec vectors

  `semantic-contract-kat.v1.json` differs from `master` only by the two removed cases.
- **Gate:** `scripts/run_checks.py integration` passes, including the release `random_smoke` games.
- **No leftovers:** a grep for the deleted type, module, schema and fixture names finds no match outside process documents.
- **Coverage:** the plan's coverage table (D7) is checked against `cargo test --workspace --locked -- --list` and the Python test list.
- **Known gap, reported and not fixed here:** the current `decision-response.v3` has no wire golden, only a schema example.

## 7. Risks

- **Large diff.** About 20,000 deleted lines. The vertical family commits (D2) keep each step reviewable and bisectable.
- **Hidden current use of a historical item.** The compiler, the Python import graph and the byte-identity check (§6) catch it. When one is found, the item stays, following D1, and the plan records a ruling.
- **Gates that assumed historical files exist.** They fail loudly in the family commit that removes the file; D5 says how each is resolved.
