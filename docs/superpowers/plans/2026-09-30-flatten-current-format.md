# Flatten the Current Format (Stage 3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dissolve the internal layering of the current format into one flat state, one digest encoder, one event enum, one operation enum and one information-state projection, without changing a single byte of any remaining fixture or of real games.

**Architecture:**
- A pinned trajectory fingerprint comes first (Task 0).
- Then the users of each old layer are rewritten before the layer itself is deleted:
  1. digest
  2. information state
  3. basic land, events and delta
  4. flat state
  5. names, files and always-true checks
  6. documents
- Every commit builds, passes the fast gate and the short fingerprint, and deletes the path it replaces.

**Tech Stack:** Rust workspace (cargo, clippy), Python client (`python/src/mtgml`, unittest/pytest), gate scripts under `scripts/`.

**Spec:** `docs/superpowers/specs/2026-09-30-flatten-current-format-design.md`

## Global Constraints

- **D1 — byte-neutral.**
  - These values never change:
    - the two fingerprint pins from Task 0;
    - `9e8064…` in `crates/mtgml-state/tests/g0e_digest.rs`;
    - `full-state-digest-v7-kat`, `checkpoint-digest-v8-kat` (`1454acca…`), and the replay V8 `checkpoint_digest` (`d11105b…`);
    - the information-state V3 digests.
  - After every task, this command shows only the paths that task allows:

    ```bash
    git diff --name-status master -- wire persistence schemas examples
    ```

    | Task | Allowed paths |
    |---|---|
    | 1 | `D persistence/golden/full-state-digest-v6-kat.v1.json`, `D schemas/negative/m4-phase2-v6-state-shapes.json` |
    | 2 | `D wire/golden/{observation-envelope.v1,information-state-envelope.v2}.json`; `D` the seven `wire/negative` files named in Task 2; `M wire/golden/manifest.json`, `M wire/negative/manifest.json`; `D schemas/{observation-envelope.v1,information-state-envelope.v2}.schema.json`; `M schemas/README.json` |
    | 5 | eight `R100` renames in `schemas/negative`; `M schemas/replay-manifest.v8.schema.json`, `M schemas/authoritative-replay.v8.schema.json` |
- **Frozen strings stay** (D1):
  - wire `schema_version`, `payload_codec` and `"decision_domain_v2"`;
  - `"full-state-digest-input.v7"`, `"mtgml.full-state-digest.v7"`, `"zones_v2"`, `"execution_v4"`, `"card-rules-authoritative-state.v1"`;
  - every constant that names a wire string (for example `OBSERVATION_SCHEMA_V2`, `FULL_STATE_DIGEST_DOMAIN_V7`).

  A mechanical rename must never touch a string literal.
- **D3 — no transition code.** The old path goes in the same commit as its replacement. The only bridge allowed across tasks is the existing `EngineState` conversion around `apply_perspective_lifecycle`, until Task 4.
- **D12 — tests.**
  - A test of deleted code only is deleted.
  - A test of remaining behavior is ported with its assertions.
  - A test that passes before the implementation it demands is a finding. Characterization ports of existing behavior are expected to pass and are labeled so in the ledger.
- **Fixture ownership:** a fixture, schema or manifest entry goes in the task that deletes its last reader.
- **Hidden current use:** if the compiler shows that a deleted item is reached by remaining code, the item stays and the executor records a ruling.
- **AGENTS.md §6:** commit before running gate scripts. A failure caused only by uncommitted files is not a code failure.
- **Commands:**
  - `cargo check --workspace --all-targets --locked`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo test --workspace --locked` (runs the short fingerprint)
  - `cargo test --release -p mtgml-environment --test random_smoke --locked` (runs the long fingerprint)
  - `.venv/Scripts/python.exe scripts/run_python_tests.py --profile full`
  - `.venv/Scripts/python.exe scripts/run_checks.py fast` (per commit) / `integration` (Task 6)

## Review Focus

1. **Latent execution wipes when `execution_v4` becomes `execution` (Task 4).** These sites compile against V4 but change behavior:
   - `turn_progression.rs` ~608 (`parts.execution = Default`) and ~756;
   - `delta_v3.rs` ~579 (`old_before.execution != old_after.execution`);
   - `validation/allocators_execution.rs` ~20-35 and ~83-98.

   The random games never hold continuations or effects at those points, so the fingerprint cannot catch a wipe. Task 4 Step 5 deletes each site and greps that none is re-pointed.
2. **Validation order and error mapping after flattening.** The former predecessor checks run before `validate_stack` and `validate_execution_records`. Tests that assert a specific `EngineStateViolation` or `EngineStatePartsV3Error` keep passing only if that order stays. The Task 4 ported tests pin it.
3. **Information safety of the direct basic-land projection (Task 2).** It no longer re-validates V2 parts itself. The ported test `magic_basic_land_projection_uses_opaque_public_ids_and_verified_content_faces` must still reject trusted ids in the payload, and `verified_catalog_rejects_*` must still fail closed.
4. **Digest of states the random games never reach:** stack payloads, continuations, triggers, effects, combat with blockers, and non-default card-rules families. The Task 1 sensitivity tests and the g0e stack/effect/trigger tests cover them; the fingerprint does not.
5. **Python import surface.** Removing `_observation_v1`, `_information_v2`, `observation_v3`, `_events_v3` and `_replay_v2`–`_v7`, and renaming classes, must not break `import mtgml` or the golden round trip. Two changes are deliberate and must be reported, not silently absorbed:
   - `mtgml.ObservationEnvelope` changes meaning from V1 to V2;
   - `ObservedEventV3` disappears.

---

### Task 0: Trajectory fingerprint (D2)

**Files:** Modify `crates/mtgml-environment/tests/random_smoke.rs`.

**Interfaces:**
- Produces:
  - `play(seed: u64, last_turn: u64) -> (Vec<Entry>, TrustedEnvironmentController)`
  - `fingerprint(trajectory: &[Entry]) -> String`
  - consts `SHORT_LAST_TURN: u64 = 3`, `SHORT_FINGERPRINT: &str`, `LONG_FINGERPRINT: &str`

- [ ] **Step 1: Write the pins with placeholders.**
  - `Entry.checkpoint_digest` stores `checkpoint_digest.as_str().to_owned()` instead of the `Debug` string.
  - `fingerprint` hashes one buffer with `mtgml_model::Digest::from_bytes(&buffer).as_str().to_owned()`. For each entry the buffer gets:
    - `actor.0.to_le_bytes()`;
    - then, for each of `response`, `step`, `knowledge[0]`, `knowledge[1]` and `checkpoint_digest` (as bytes), `(len as u64).to_le_bytes()` followed by the bytes.
  - New test `short_game_matches_its_pinned_fingerprint`, with no `ignore`, so it also runs in debug:

    ```rust
    assert_eq!(fingerprint(&play(FIRST_SEED, SHORT_LAST_TURN).0), SHORT_FINGERPRINT);
    ```

  - In `random_games_run_thirty_turns_deterministically_and_replay`, for `game == 0`:

    ```rust
    assert_eq!(fingerprint(&trajectory), LONG_FINGERPRINT, "seed {seed:#x}");
    ```

  - Both constants start as `"0000…"` (64 zeros).
- [ ] **Step 2: Run.** `cargo test -p mtgml-environment --test random_smoke --locked`, then the release command. Expected: both pin assertions FAIL and print the actual hex. Every other assertion passes.
- [ ] **Step 3: Pin** the two printed values. Rerun both commands. Expected: PASS.
- [ ] **Step 4: Mutation check.**
  - Temporarily change the initial `next_rule_event_id` in `crates/mtgml-state/src/construction.rs` to one higher. Run the debug command. Expected: `short_game_matches_its_pinned_fingerprint` FAILS.
  - Revert and rerun. Expected: PASS.
  - Record both runs in the ledger.
- [ ] **Step 5: Commit** `test: pin random-game trajectory fingerprints`. Run the fast gate. Expected: PASS.

### Task 1: Single-pass digest (D5)

**Files:**
- **`crates/mtgml-state/src/persisted_v7.rs`:**
  - new `fn state_value(state: &EngineStatePartsV3) -> Result<Value, StateDigestError>`;
  - delete `FullStateDigestInputV7`, `from_canonical_payload`, `state_value_validated`, every `validate_*` value validator (~89-1182) and `mod g0_validation_tests` except `hand_size_discard_purpose_round_trips_through_the_persisted_codec` (keep its encoder assertion only).
- **`crates/mtgml-state/src/digest_v7.rs`:** delete `calculate_full_state_digest_v7_payload` and `verify_full_state_digest_v7`.
- **`crates/mtgml-state/src/digest_v5.rs`:**
  - keep the component encoders and the V3-execution encoders that `PersistedExecutionV3` reaches;
  - delete `FullStateDigestInputV5`, `calculate_full_state_digest_v5*`, `full_state_digest_input_v5`, the V2-only `execution_value`, the V5 stack-row/payload-rejection part of `zones_value`, and everything the compiler then reports unused.
- **`crates/mtgml-state/src/digest_v6.rs`:** delete.
- **`crates/mtgml-state/src/persisted_v6.rs`:**
  - delete `FullStateDigestInputV6` (all impls, `from_phase2_v5_payload`), the `validate_legacy_components` chain, the card-rules `from_value` decoders and `parse_*` helpers;
  - keep the card-rules types, their `validate`s and encoders, `PersistedV6Error`, `PersistedExecutionV3` and the `validate_execution_v3` chain. `EngineStatePartsV2::validate` still calls those until Task 4.
- **`crates/mtgml-state/src/engine.rs`:** delete `FULL_STATE_DIGEST_INPUT_SCHEMA`, `canonical_digest_bytes`, `digest`.
- **`crates/mtgml-state/src/lib.rs`**, **`crates/mtgml-state/src/digest.rs`** (unused `StateDigestError` variants).
- **`crates/mtgml-model/src/lib.rs`:** delete `FullStateDigestV5`, `FullStateDigestV6`, and the test `full_state_digest_v6_has_distinct_typed_reference_identity`.
- **Tests:**
  - `crates/mtgml-state/src/tests/digest.rs`;
  - `crates/mtgml-state/src/tests.rs`;
  - `src/tests/{batch_d,validation,zones_allocators}.rs`;
  - `crates/mtgml-state/tests/{g0e_digest,p0_red,g0d_state_authority}.rs`;
  - `crates/mtgml-persistence/src/tests.rs` (~699);
  - new `crates/mtgml-state/tests/digest_sensitivity.rs`.
- **Fixtures (delete):**
  - `persistence/golden/full-state-digest-v6-kat.v1.json`;
  - `crates/mtgml-state/tests/fixtures/magic-sba-graveyard-order-v5-input.hex`;
  - `schemas/negative/m4-phase2-v6-state-shapes.json`. Its only reader is the deleted V6 test; record a ruling, since spec §4 does not name it.
  - `full-state-digest-v7-kat.v1.json` keeps its now-dangling `parent_v6_fixture` text. It is byte-frozen, and no code reads it.
- **Python:**
  - `python/src/mtgml/persistence.py`: delete the dead V5/V6 constants (~27-28, ~37-38).
  - `python/tests/test_authoritative_state_coverage.py`: see Step 5.
- **`scripts/verify_repository.py`:** ~333 token `FullStateDigestInputV5` → `full-state-digest-input.v7`; ~335 token `canonical_digest_bytes` → `canonical_state_bytes`.

**Interfaces:**
- Produces:
  - `pub fn full_state_digest_v7_from_payload(payload: &[u8]) -> Result<FullStateDigestV7, StateDigestError>`: `encode_envelope` plus `hash_envelope`, with no decoding. It is the hash step after encoding.
  - `canonical_state_bytes_v7[_structural_only]` and `calculate_full_state_digest_v7[_structural_only]` keep their signatures and now validate once, encode once and hash once.
  - `state_value` destructures `EngineStatePartsV3 { predecessor_v5, execution_v4, card_rules_state }` and `EngineStateParts { .. }` exhaustively (`execution: _`).
- Encoder invariants (spec D1, B6):
  - the 14-element order;
  - `zones_v2` built from the V5 object, location and ordered-zone encoders plus `[id, controller, stack_payload_value]` rows;
  - streams and ordered zones sorted by canonical key bytes;
  - literal `[]` at `execution_v4[5]`;
  - `combat_value` with its 3/5-element branch;
  - `foundation_sources_value` unchanged.

- [ ] **Step 1: Write the V7 sensitivity tests** in `crates/mtgml-state/tests/digest_sensitivity.rs`, on the `g0e_digest` baseline state:
  - `v7_digest_changes_for_each_state_component_mutation`: port the 48 named cases of `m3_p0_full_state_digest_v5_mutation_matrix`, minus the V2-execution and foundation cases.
    - Each mutated state must pass `validate()`. Adjust coupled fields, for example `card_rules_state.turn_history.turn_number` together with `core.turn_number`.
    - Assert that every digest differs from the baseline and that all digests are pairwise distinct.
  - `v7_digest_binds_combat_inner_values`: port of `v5_digest_binds_combat_inner_values`. It asserts both the 3-element and the 5-element combat forms change the digest.
  - `v7_digest_binds_knowledge_history_and_provenance`: ports `knowledge_history_is_digested_without_a_player_level_aggregate` and the four provenance tests, using `canonical_state_bytes_v7` text.
  - `v7_digest_binds_ordered_zone_order`: port of `batch_d` `fnd_006b_valid_canonical_reorder_changes_the_v4_digest`.
  - `v7_digest_binds_each_card_rules_family`: one mutation per family (mana, history, counter, attachment, face, ability), taken from the V6 KAT case names.
- [ ] **Step 2: Run** `cargo test -p mtgml-state --test digest_sensitivity --locked`. Expected: PASS. These characterize existing behavior before the refactor; record them as characterization in the ledger.
- [ ] **Step 3: Implement** the single-pass path and delete the items listed in Files.
  - Port the test sites (agent inventory in the ledger):
    - delete the `full_state_digest_v6_*`, the `execution_v3_*` V6 tests, `v5_digest_binds_foundation_source_inner_values`, `v5_digest_payload_is_nonempty_canonical_cbor` and `v7_input_decoder_rejects_wrong_identity_truncation_and_unknown_stack_tags`;
    - drop only the V5 `digest()` assertions in `tests.rs:139`, `validation.rs:18/175`, `zones_allocators.rs:5`, `g0d_state_authority.rs:947` and `p0_red.rs:16`;
    - in `batch_d` :588 and :604, assert the `validate_engine_state` error instead;
    - g0e :255 uses `full_state_digest_v7_from_payload`;
    - persistence ~699 builds the V6 `DigestReferenceV1` by hand;
    - `typed_collection_insertion_order_is_irrelevant`: rebuild it without the V6 fixture, or delete it with a ledger note if it only restates `BTreeMap` ordering.
- [ ] **Step 4: Compile** (`cargo check`, clippy). Expected: PASS.
- [ ] **Step 5: Python coverage test.** `test_authoritative_state_coverage.py` now asserts two things:
  - the field set of `pub struct EngineState {` (engine.rs) equals the field set destructured by `let EngineStateParts {` … `} = ` in `persisted_v7.rs`;
  - the field set of `pub struct EngineStatePartsV3 {` equals the one destructured by `let EngineStatePartsV3 {`.

  Run it. Expected: PASS.
- [ ] **Step 6: Run** `cargo test --workspace --locked` and the full Python profile. Expected: PASS, including g0e `9e8064…`, the V7 KAT, `checkpoint_digest_v8_g0_known_answer`, the Python g0c fixtures and `short_game_matches_its_pinned_fingerprint`. Check the byte-identity table.
- [ ] **Step 7: Commit** `refactor: compute the full-state digest in one pass`. Run the fast gate. Expected: PASS.

### Task 2: Direct information state (D6)

**Files:**
- **`crates/mtgml-environment/src/player_projection.rs`:**
  - new `pub(crate) fn project_magic_basic_land_observation(parts: &EngineStatePartsV3, perspective: PlayerId, execution_identity: &ExecutionIdentityV1, semantic_manifest: &SemanticContractManifestV1, rules_manifest: &RulesContractManifestV1, catalog: &VerifiedContentCatalogV1) -> Result<MagicBasicLandObservationV1, PlayerEndpointError>`:
    - it keeps the authority and catalog checks (~46-121) and the value build (~134-231), reading `parts.predecessor_v5` directly;
    - `pending_sba_ordering: None`, because the shared observation overwrites it from V4;
    - the caller validates the state;
  - new `fn project_retained_knowledge(knowledge: &PlayerKnowledgeStateV2) -> Vec<PlayerKnownObjectV1>`: the rule moved verbatim from `project_information_state_from_observation` (~921-957);
  - `_v3_inner` becomes: validate → basic value → shared observation → `ObservationEnvelopeV2` → `project_retained_knowledge` → V3 digest;
  - delete `project_magic_basic_land_observation_v1` (and its `lib.rs` export), `project_magic_basic_land_observation_from_verified_faces`' envelope part, `project_sba_ordering`, `project_information_state_from_observation` and the V2/V1 imports.
- **`crates/mtgml-observation`:** delete `src/information.rs` and `src/observation.rs`; `lib.rs` mods, exports, `OBSERVATION_SCHEMA` and `INFORMATION_STATE_SCHEMA_V2`.
- **`crates/mtgml-wire`:**
  - `observation.rs`: delete the V1/V2 `WireContract` impls, `verify_information_state_digest_v2` and `compute_information_state_digest_v2`;
  - `fixtures.rs`: delete the two `decode_named` arms;
  - `lib.rs`.
- **`crates/mtgml-model/src/lib.rs`:** delete `InformationStateDigestV2`.
- **Python:**
  - move `observation_digest_from_payload` into `observation_v3.py`;
  - delete `_observation_v1.py` and `_information_v2.py`;
  - update `observation.py`, `__init__.py`, and `wire.py`: remove the `_DECODERS` keys `observation-envelope.v1`, `information-state-envelope.v2` and `information-state-digest-input.v2`, `compute_information_state_digest_v2`, and the unused `hashlib`;
  - `python/tests/test_player_api.py:17` import.
- **Fixtures (delete):**
  - `wire/golden/observation-envelope.v1.json`, `information-state-envelope.v2.json`;
  - `wire/negative/{observation-invalid-base64,observation-digest-mismatch,information-v2-forged-digest,information-v2-future-provenance,information-v2-invalid-cause-channel,information-v2-initial-invalidation,information-v2-unknown-field}.json`;
  - their manifest entries;
  - `schemas/observation-envelope.v1.schema.json`, `schemas/information-state-envelope.v2.schema.json`;
  - `schemas/README.json` lines;
  - `scripts/validate_schemas.py` `WIRE_MAPPING` lines.

**Interfaces:**
- Consumes: Task 1 (nothing specific).
- Produces: the two functions above. `project_successor_information_state_v3[_structural_only]` keep their signatures.

- [ ] **Step 1: Write the failing test.** Extend `DELETED_CONTRACTS` in `python/tests/test_wire_contracts.py` with `"observation-envelope.v1"`, `"information-state-envelope.v2"` and `"information-state-digest-input.v2"`.
- [ ] **Step 2: Run** `.venv/Scripts/python.exe -m pytest -q python/tests/test_wire_contracts.py -k deleted`. Expected: FAIL; the decoders still exist.
- [ ] **Step 3: Write in-code replacements for the deleted fixtures' coverage.**
  - **Rust**, in `crates/mtgml-observation/src/tests.rs` and `provenance_tests.rs`:
    - port `information_state_input_excludes_trusted_fields` to `InformationStateDigestInputV3`;
    - port `observation_digest_binding_accepts_matching_payload_and_rejects_mismatch` to `ObservationEnvelopeV2`, plus an invalid-Base64 case;
    - port the four provenance tests to `PlayerInformationStateV3`, whose envelope `view_sequence` equals `next_visible_sequence`, plus an initial-invalidation case;
    - rewrite the helper `observation()` for V2.
  - **Rust**, in `crates/mtgml-wire`:
    - port `observation_envelope_v1_constructs_the_golden_bytes` so it constructs `wire/golden/observation-envelope.v2.json`;
    - replace `information_state_envelope_v2_constructs_the_golden_bytes` with `information_state_v3_with_rich_retained_knowledge_round_trips`: all four causes, active and retired; construct → digest → encode → decode → equal;
    - delete `information_state_digest_v2_known_answer`.
  - **Python:**
    - port `ObservationDigestBindingTests` (3 tests) to V2;
    - port `InformationProvenanceParityTests` (2 tests) to V3 built inline from the V2 golden's `retained_knowledge` JSON;
    - port `InitialConfigurationCursorParityTests` (2 tests) to V3;
    - add one test per deleted wire negative. It asserts the same `expected_error_code` that the deleted `wire/negative/manifest.json` entry names (`semantic.observation` or `semantic.information_state`).
  - These characterize existing V2/V3 behavior. Run them: PASS.
- [ ] **Step 4: Port the environment tests** in `crates/mtgml-environment/src/tests/magic_basic_land_observation.rs`:
  - The helper `basic_land_parts` builds `EngineStatePartsV3` directly.
  - Tests 1, 4, 5, 6 and 7 call `project_magic_basic_land_observation` and/or `project_successor_information_state_v3`, keeping their assertions:
    - test 1 asserts `schema_version == MAGIC_BASIC_LAND_OBSERVATION_SCHEMA_V1` instead of the codec;
    - tests 6 and 7 compare the full V3 information-state bytes and digest.
  - Tests 2 and 3 change only the helper.
- [ ] **Step 5: Implement and delete** as listed in Files.
- [ ] **Step 6: Run** `cargo check`, clippy, `cargo test --workspace --locked` and the full Python profile. Expected: PASS, including `test_deleted_contracts_are_unknown`, `every_shared_negative_fixture_is_rejected_with_the_expected_code`, `test_every_golden_fixture_roundtrips_to_identical_bytes` and the short fingerprint. Check the byte-identity table.
- [ ] **Step 7: Commit** `refactor: project the information state directly`. Run the fast gate. Expected: PASS.

### Task 3: Basic land, events and delta on current types (D7)

**Files:**
- **`crates/mtgml-state/src/delta_v3.rs`:**
  - `SemanticDeltaOperationV3` absorbs every V1 variant (23) and the V2 variants `LandPlayCountChanged`, `AbilityIdentityChanged`, `AttachmentChanged`, `ObjectFaceChanged`, `ObjectEntered`, `AbilityAuthorityAdded` and `AbilityAuthorityRemoved`. `Existing` goes (45 variants).
  - Coverage: `has_v3`, `has_v2` and `has_legacy` become one `has`; the V2 `ObjectTapped`, `ManaAdded`, `ManaPoolEmptied` and `CounterChanged` alternatives go.
  - Delete `delta.rs` and `delta_v2.rs`.
- **`crates/mtgml-rules/src/events_v3.rs`:**
  - `AuthoritativeRuleEventKindV3` absorbs the 22 V1 kinds other than `PerspectiveOccurrence`. `Existing` goes (36 variants).
  - `semantic_delta` merges into `semantic_operations`.
  - `PerspectiveObservationPolicyV1` moves here.
  - Delete `events.rs`, including the V1 `AuthoritativeRuleEvent` struct.
  - Update the validators and projections that matched `Existing(…)`.
- **`crates/mtgml-rules/src/zone_incarnation.rs`:** the zone-move primitive returns the `ZoneTransition` and the per-player `PerspectiveLifecycleAuditV1` list, in today's order, instead of V1 events with placeholder ids.
- **`crates/mtgml-rules/src/turn_progression.rs`:** consume that result; match the flat variants.
- **`crates/mtgml-rules/src/basic_land.rs`:**
  - absorbs what survives of `basic_land_v4.rs`. Delete that file, together with:
    - the V2 draft (`BasicLandTransitionDraftV1`, `execute_basic_land_decision_draft`, `AuthoritativeRuleEventV2`/`KindV2`, `SuccessorObservationPolicyV1`, `BasicLandFaceV1`, `push_successor_event`, `push_visible_occurrence`);
    - `derive_basic_land_candidates` (V3), `candidate_state_v2`;
    - the three `convert_*`/`causal_*` functions.
  - `crates/mtgml-rules/src/lib.rs` exports follow.
- **`crates/mtgml-decision/src/v4.rs`:** `CandidateOrderingV3::assign_dense`.
- **`crates/mtgml-environment/src/successor_projection.rs`:** match the flat variants and inline the `MovedInSight` policy.
- **Tests:**
  - `crates/mtgml-rules/src/{basic_land,turn_progression,events_v3}.rs` tests;
  - `crates/mtgml-rules/src/tests/{semantic_delta,zone_incarnation}.rs`;
  - `crates/mtgml-state/tests/{untap_delta,g0e_digest}.rs`;
  - `crates/mtgml-decision/src/{v3.rs,tests.rs}`;
  - `crates/mtgml-environment/src/{basic_land_runtime_v8.rs,tests/magic_basic_land_observation.rs}`.

**Interfaces:**
- Produces:
  - `impl CandidateOrderingV3 { pub fn assign_dense(candidates: Vec<(CandidateIntentV4, EngineCandidateBindingV4)>) -> Result<Vec<AuthoritativeCandidateV4>, DecisionValidationError> }`. It runs `validate_candidate_capacity`, then a stable `sort_by(|a, b| a.0.compare(&b.0))`, rejects any adjacent `Ordering::Equal` pair with `DuplicateOrderingKey`, and sets `candidate_id = u32::try_from(index)`.
  - `derive_basic_land_candidates_v4`, `install_basic_land_request_v4`, `selected_basic_land_action_v4`, `execute_basic_land_response_v4` and `BasicLandTransitionProductV4` keep their signatures.
- Not in this task: deleting the V3 candidate types, the old orderings, `authoritative.rs`, `ActionCandidate`, `CandidateIntent` and `VisibleCandidateV2` moves to Task 4 (spec D7).

**Emission rule** (reproduces the bridge byte for byte; spec §7 risk):
- Work on `next = before.clone()`, with the pending decision cleared and the revision incremented by 1.
- Build a pending list of kinds and of occurrences, where an occurrence carries a lifecycle and a source index.
- **Always:** `DecisionCleared(pending id)` first.
- **Land play:**
  - `ZoneTransition`, then one occurrence per player in `core.players` order, with the source set to the ZoneTransition.
  - No `LandPlayed` or `ObjectMoved`, and no id is allocated for them.
- **Mana ability:**
  - `ObjectTapped{source, false, true}`;
  - then one occurrence per player in `knowledge.players` order, sourced to that event;
  - then `ManaPoolChanged{actor, before, after, Produced}`;
  - then one occurrence per player, sourced to it.
- **Then:** `PriorityChanged` if `consecutive_passes != 0`. Install the request and append `DecisionCreated(new id)`.
- **Numbering:**
  - `first = before.next_rule_event_id`; `event_id = first + index`; `source_event_id = first + source_index`;
  - `state_revision` = the new revision;
  - `after.next_rule_event_id = first + len`;
  - checked additions, failing with `IdentityExhausted`.
- **Operation order:**
  - Land play: DecisionCleared, [AbilityAuthorityRemoved*], PerspectiveLifecycle per player, [AbilityIdentityChanged Some→None*], AbilityAuthorityAdded, AbilityIdentityChanged None→Some per player, ObjectEntered, LandPlayCountChanged, [PriorityChanged], PendingRequestChanged, DecisionCreated. There is no ZoneTransition operation.
  - Mana ability: DecisionCleared, ObjectTapped, PL, PL, ManaPoolChanged(Produced), PL, PL, [PriorityChanged], PendingRequestChanged, DecisionCreated.
- Lifecycles are applied in today's order, through the existing `EngineState` conversion around `apply_perspective_lifecycle`.

- [ ] **Step 1: Write the failing tests** for `CandidateOrderingV3::assign_dense`:
  - port `v3.rs` `v3_order_inserts_play_land_between_pass_and_cast` and `play_land_order_uses_only_opaque_object_identity`;
  - port `tests.rs` :257, :308 and :349 (insertion independence, duplicate key → `DuplicateOrderingKey`, dense ids) to V4 intents.
- [ ] **Step 2: Run** `cargo test -p mtgml-decision --locked`. Expected: FAIL to compile; `assign_dense` does not exist.
- [ ] **Step 3: Implement `assign_dense`.** Run again. Expected: PASS.
- [ ] **Step 4: Collapse the two enums, rewrite basic land and zone incarnation** as in Files and the emission rule.
  - Port the tests:
    - `basic_land.rs` 1285/1522/1538 onto V4 candidates on a V3 state;
    - `events_v3` 1890/1917/1935/1970/2085/2537/2640, plus 2135/2432/2503, which now build the flat variants directly;
    - `semantic_delta.rs` 46/55/67/77 onto `semantic_operations`;
    - the `untap_delta.rs` helper;
    - `turn_progression` 1485/1657;
    - `zone_incarnation` 111, whose placeholder-id assertions go;
    - `g0e_digest` `state_delta_v3_*` and the two environment tests that build `Existing(…)` onto the flat variants.
  - Delete `v3.rs` :438 (covered by `v4.rs` :1720).
- [ ] **Step 5: Compile**, then run `cargo test --workspace --locked`. Expected: PASS, including the short fingerprint. A fingerprint failure means the emission rule was not reproduced; debug it, do not re-pin.
- [ ] **Step 6: Run** the full Python profile. Expected: PASS. Check the byte-identity table: no output.
- [ ] **Step 7: Commit** `refactor: basic land emits current candidates, events and operations directly`. Run the fast gate. Expected: PASS.

### Task 4: Flat state (D4)

**Files:**
- **`crates/mtgml-state/src/engine_state_parts_v3.rs`:**
  - the struct becomes flat: `revision, core, combat, zones, allocators, execution: ExecutionStateV4, random, knowledge, perspective_identities, format, card_rules: CardRulesAuthoritativeStateV1`, with no Serde;
  - delete `new`;
  - `validate_structure` runs, in this order:
    1. `validate_engine_state(self)`, ported to the flat state;
    2. `card_rules.validate()`;
    3. the cross-checks moved from `EngineStatePartsV2::validate` (V2-5…V2-14 of the ledger inventory; drop the V3-pending term of `has_content_authority`);
    4. `validate_stack`, without the source-authority check;
    5. `validate_execution_records`.
  - `validate` adds the profile-dependent pending check.
  - Error changes: `PredecessorState` becomes `StateInvariant`; delete `CardRulesState`, `DuplicateExecutionAuthority` and `DuplicateStackSourceAuthority`.
- **Delete:**
  - `engine.rs` (old `EngineState`, `EngineStateParts`, the dead RNG helpers);
  - `engine_state_parts_v2.rs` (port its card-rules tests 397/450/480/507/538 to the flat state; delete 586/606/657);
  - `validation/decision.rs`.
- **`execution.rs`:** delete the V2 `ExecutionState`, `ExecutionStateV3` and `TriggerRecord`.
- **`engine_state_shape/continuation.rs`:** delete `PendingDecisionRecordV2`, `ContinuationPayloadV2`, `ContinuationRecordV2` and `validate_program_coherence`.
- **`engine_state_shape.rs`:** `validate_engine_state_shape` loses its pending/continuation parameters and ~160-236.
- **`validation.rs` / `validation/*`:**
  - delete the `EngineStateViolation::{FoundationSource, PendingDecisionMismatch, ExecutionMismatch, MissingContinuation}` variants;
  - delete `control_history_is_coherent` and `_binding_type_marker`;
  - delete allocator checks A3, A5, A7 and A8. **Do not re-point them to V4.**
  - A6 keeps only its opaque-ability part.
- **`zones.rs`:** delete `StackRecord.source_object/source_ability`, the custom serde and `LegacyStackRecord`, and the `ZoneState` serde derive.
- **`core.rs`:** delete `FoundationSourceKind`, `BaseCharacteristics`, `ControlHistory` and `FoundationCreatureSource`.
- **`identity.rs`:** delete the `PerspectiveIdentityResolver` impl.
- **`lifecycle.rs`:** `apply_perspective_lifecycle(state: &mut EngineStatePartsV3, …)`.
- **`persisted_v6.rs` / `digest_v5.rs`:**
  - delete `PersistedExecutionV3`, the `validate_execution_v3` chain and the V3-execution encoders;
  - `state_value` writes `Value::Array(vec![])` at index 11 instead of encoding `foundation_sources`.
- **`construction.rs`:**
  - returns the flat state, with `card_rules` holding a default mana pool and default turn history per player and `turn_history.turn_number = core.turn_number`;
  - the execution is the default, with no pending decision;
  - every allocator and cursor value stays unchanged: decision 2, player decision 2, object 3, opaque 2/3, sequence 1;
  - drop `SyntheticV4Setup.foundation_sources`.
- **`crates/mtgml-decision`:**
  - delete `authoritative.rs`, `v1.rs`, `CandidateIntent` (`common.rs` keeps `DecisionVisibility`), `CandidateOrderingV1/V2` (`ordering.rs` keeps `validate_candidate_capacity`), `VisibleCandidateV2`, and the V3 candidate/request types, `validate_candidate_binding_v3` and the `From` impls (`v3.rs` keeps `DecisionResponseV3`);
  - delete `CandidateBindingError` and the tests `tests.rs` 32/44/56/74 and `v3.rs` 512.
- **`crates/mtgml-rules`:**
  - `zone_incarnation.rs`: flat state; delete the stack-source (~50-59), V2-pending (~60-78) and foundation (~79-86, ~325) checks and the error variants that become unused;
  - `turn_progression.rs`: delete the foundation clause (~206); at ~512, ~592-624 and ~752-757, work on `next` directly and **delete** `parts.execution = Default` and the lifecycle `parts()` copy;
  - `events_v3.rs`: flat state at ~596-650 and ~1344-1545; delete the foundation marked-damage branches (~786-789, ~1381-1389);
  - `characteristic_query.rs`: `InconsistentState(EngineStatePartsV3Error::StateInvariant)`;
  - `snapshots.rs`.
- **`crates/mtgml-state/src/delta_v3.rs`:** delete the V2-execution comparison (~579) and the foundation comparison (~940).
- **`crates/mtgml-environment`:**
  - `successor_projection.rs` ~130-131 borrow instead of converting;
  - `player_projection.rs`, `basic_land_runtime_v8.rs`, `checkpoint_v8.rs`;
  - test fixtures (`basic_land_runtime_v8` ~594-708, `magic_basic_land_observation` ~152-216, `tests/current_successor_api.rs` ~366, `tests/common/mod.rs` ~196-250) use the flat builder and drop their hand-built card rules.
- **Tests to delete:**
  - `p0_red.rs` foundation cases ~142-163;
  - `tests/continuation.rs` ~445;
  - `g0d_state_authority.rs` ~1229-1290 (legacy StackRecord serde);
  - `turn_progression` `unsupported_state_fails_closed`: retarget it to another `validate_slice` clause, or delete it with a ruling if none remains reachable.
- **Items that lose their last user.** The compiler does not report unused `pub` items, so delete these by hand:
  - `SYNTHETIC_COUNT_MIN`, `AssemblyStageV2::stage_index`, `TurnPosition::canonical_rank`;
  - `PerspectiveIdentityResolver`;
  - the four unused `EngineStateShapeViolation` variants;
  - `ZoneIncarnationError::{StackSourceReference, PendingDecisionReference}`;
  - `register_ability_authorities` and `attachment_state_after_transition` (test-only; they move into the test module that uses them, or go with their tests).
- **`scripts/verify_repository.py`:** unchanged (Task 5).
- **`python/tests/test_authoritative_state_coverage.py`:** compare `pub struct EngineStatePartsV3 {` with the single flat destructure in `state_value`.

**Interfaces:**
- Consumes: Task 3 (basic land no longer uses `EngineStatePartsV2`).
- Produces: the flat `EngineStatePartsV3` fields listed above (renamed in Task 5), and `EngineStatePartsV3Error::StateInvariant`.

- [ ] **Step 1: Write the failing test** `flat_state_exposes_every_component_at_the_top_level` in `crates/mtgml-state/tests/g0d_state_authority.rs`. It builds the state with `construct_synthetic_engine_state` and asserts:
  - `state.card_rules.mana.pools.len() == 2`;
  - `state.execution.pending_decision.is_none()`;
  - the decision allocator cursor in `state.allocators` is still 2, as the builder sets it today;
  - `state.validate_structure().is_ok()`.
- [ ] **Step 2: Run** `cargo test -p mtgml-state --test g0d_state_authority --locked`. Expected: FAIL to compile (no such fields).
- [ ] **Step 3: Flatten.**
  - Rename mechanically: `.predecessor_v5.` → `.`; the field `execution_v4` → `execution`; `card_rules_state` → `card_rules`. Rename identifiers only; the string literals `"execution_v4"` in `persisted_v7.rs` and `g0e_digest.rs` stay.
  - Then make the deletions and ports listed in Files.
- [ ] **Step 4: Compile** (`cargo check`, clippy). Expected: PASS.
- [ ] **Step 5: Hazard check** (Review Focus 1). Run:

  ```bash
  rg -n "execution = Default|ExecutionStateV4::default\(\)|execution != .*execution" crates/mtgml-rules/src crates/mtgml-state/src --glob '!**/tests*'
  ```

  Expected: no match in `turn_progression.rs`, `zone_incarnation.rs`, `delta_v3.rs` or `validation/`. `construction.rs` is allowed.
- [ ] **Step 6: Run** `cargo test --workspace --locked` and the full Python profile. Expected: PASS, including `flat_state_exposes_every_component_at_the_top_level`, the short fingerprint and g0e `9e8064…`. Check the byte-identity table: no output.
- [ ] **Step 7: Commit** `refactor: flatten the engine state`. Run the fast gate. Expected: PASS.

### Task 5: Names, files and always-true checks (D8, D9, D10)

**Final rename list (D8, finalized).** Rust, with Python where a mirror exists:

| Current | New | Python |
|---|---|---|
| `EngineStatePartsV3` / `EngineStatePartsV3Error` | `EngineState` / `EngineStateError` | – |
| `ExecutionStateV4` | `ExecutionState` | – |
| `ContinuationRecordV3` / `ContinuationPayloadV3` | `ContinuationRecord` / `ContinuationPayload` | – |
| `StateDeltaV3`, `SemanticDeltaOperationV3` | `StateDelta`, `SemanticDeltaOperation` | – |
| `DeltaApplicationV3Error` | `DeltaApplicationError` | – |
| `FullStateDigestV7` | `FullStateDigest` | – |
| `PersistedV6Error` | `CardRulesStateError` | – |
| `AuthoritativeRuleEventV3` / `KindV3` | `AuthoritativeRuleEvent` / `AuthoritativeRuleEventKind` | – |
| `EventDeltaV3Error`, `RuleEventCursorV3Error` | `EventDeltaError`, `RuleEventCursorError` | – |
| `AuthoritativeDecisionRequestV4`, `AuthoritativeCandidateV4` | without suffix | – |
| `CandidateIntentV4`, `EngineCandidateBindingV4`, `VisibleCandidateV4`, `CandidateOrderingV3` | without suffix | `CandidateIntentV4`, `VisibleCandidateV4` too |
| `ObservationEnvelopeV2`, `PlayerInformationStateV3`, `InformationStateDigestV3`, `InformationStateDigestInputV3` | without suffix | yes (`InformationStateDigestV3` is a string in Python) |
| `BasicLandTransitionProductV4` | `BasicLandTransitionProduct` | – |

Functions follow their types:
- `calculate_full_state_digest[_structural_only]`, `canonical_state_bytes[_structural_only]`, `full_state_digest_from_payload`
- `compute_information_state_digest` (Rust and Python), `project_successor_information_state[_structural_only]`
- `allocate_rule_events`, `validate_event_delta_parity`, `validate_event_delta_state`, `validate_rule_event_cursor`
- `selected_bindings[_validated]`
- the basic-land functions `derive_basic_land_candidates`, `validate_basic_land_pending_request`, `install_basic_land_request`, `selected_basic_land_action`, `execute_basic_land_response`
- `execute_magic_response`, `validate_magic_pending_request`

Wire-string constants keep their names. Not renamed: the families listed in spec D8, and `SyntheticV4Setup`.

**File moves (D9, finalized):**
- **mtgml-state:**
  - `engine_state_parts_v3.rs` → `engine.rs`;
  - `digest_v5.rs`, `digest_v7.rs` and `persisted_v7.rs` merge into `digest.rs` with `StateDigestError`;
  - `persisted_v6.rs` → `card_rules.rs`;
  - `delta_v3.rs` → `delta.rs`.
- **mtgml-rules:** `events_v3.rs` → `events.rs`.
- **mtgml-replay:** `v2.rs` → `randomness.rs`, `v7.rs` → `contract_material.rs`.
- **mtgml-decision:**
  - `v2.rs` → `answer.rs`, `v3.rs` → `response.rs`;
  - `CandidateOrdering` moves from `v4.rs` into `ordering.rs`;
  - `v4.rs` keeps its name, like the other current-version files (`replay/v8.rs`, `observed_event_v4.rs`, `checkpoint_v8.rs`).
- **mtgml-observation:**
  - `detached_v2.rs` is split: the envelope and `OBSERVATION_SCHEMA_V2` go to `observation.rs`; the information state, digest input, `validate_retained_knowledge` and their consts go to `information.rs`;
  - `observed_event_v3.rs` merges into `observed_event_v4.rs`.
- **Python:**
  - `_replay_v2.py`, `_replay_v4.py` and `ExecutionIdentityV1` from `_replay_v5.py` → `_replay_common.py`;
  - `_replay_v6.py` → `_replay_v8.py`;
  - `_replay_v7.py` → `_contract_material.py`. `SemanticContractMaterialV5` is deleted: its field parsing is inlined into `SemanticContractMaterialV7.from_wire`, and its `validate` must not be merged. `_replay_v5._VALID_PROGRAM_KINDS` reuses `persistence._VALID_PROGRAM_KINDS`;
  - `_player_step_v2.py` → `_player_step_v4.py`;
  - `_events_v3.py` → `_events_v4.py` (`ObservedEventV3` merges into `ObservedEventV4`, and its export goes);
  - `observation_v3.py` → `_information_state.py`, re-exported by `observation.py`.
- **Schemas:** the eight `schemas/negative/replay-v7-content-*.json` renames from spec D9. Update `scripts/validate_schemas.py` and `python/tests/test_g0h_replay_v8.py`.
- **Gates:** `scripts/verify_repository.py` ~325 token `EngineStateParts` → `pub struct EngineState`. `test_authoritative_state_coverage.py` points at `engine.rs` and `digest.rs`.

**Always-true checks (D10):**
- Delete `execution_program_matches_rules_authority` (`crates/mtgml-model/src/execution_identity.rs`), its re-export, its test `adr_0055_program_authority_pairs_are_closed_and_symmetric`, and its callers:
  - `crates/mtgml-card-ir/src/preflight.rs` ~341;
  - `crates/mtgml-environment/src/checkpoint_v8.rs` ~264, where `(MagicRules && (…))` simplifies to `(…)`;
  - `crates/mtgml-replay/src/v8.rs` ~123.
- Delete the `program_kind` comparisons at `preflight.rs` ~344, `basic_land.rs` (the former ~877 and v4 ~97), and `player_projection.rs` ~63-67, including the always-true `ComprehensiveRules` match.
- Delete the top-level program/authority `allOf` in `schemas/replay-manifest.v8.schema.json` (~513-564) and under `manifest` in `schemas/authoritative-replay.v8.schema.json` (~805-870). Leave `$defs.semantic_contract_material.allOf` alone.
- Delete the Python `program_authority` map (`_replay_v8.py` ~250-255).

- [ ] **Step 1: Write the failing test** `test_execution_identity_validate_rejects_unknown_program_kind` in `python/tests/test_g0h_replay_v8.py`:

  ```python
  identity = ExecutionIdentityV1(program_kind="synthetic_rules_compat", ...)  # otherwise the V8 example's values
  with self.assertRaises(WireError):
      identity.validate()
  ```

- [ ] **Step 2: Run it.** Expected: FAIL if `validate()` does not check the program kind. If it already raises, record that the pairing map was redundant and continue.
- [ ] **Step 3: Implement.**
  - `ExecutionIdentityV1.validate()` checks `_VALID_PROGRAM_KINDS`.
  - Delete the D10 items.
  - Apply the renames with a word-boundary identifier replace that skips string literals, file by file; the compiler and ruff check the result.
  - Update the `__all__` lists.
  - Do the file moves with `git mv` so history follows.
- [ ] **Step 4: Compile and run.** `cargo check`, clippy, `cargo test --workspace --locked`, and the full Python profile (ruff and mypy included). Expected: PASS, including the short fingerprint, `validate_schemas.py` and `test_schema_parity`.
- [ ] **Step 5: Name check.** Run:

  ```bash
  rg -n "\b(EngineStatePartsV3|ExecutionStateV4|StateDeltaV3|SemanticDeltaOperationV3|FullStateDigestV7|AuthoritativeRuleEvent(Kind)?V3|AuthoritativeDecisionRequestV4|AuthoritativeCandidateV4|CandidateIntentV4|EngineCandidateBindingV4|VisibleCandidateV4|CandidateOrderingV3|ObservationEnvelopeV2|PlayerInformationStateV3|InformationStateDigest(Input)?V3|PersistedV6Error)\b" crates python scripts
  ```

  Expected: no match.
- [ ] **Step 6: Commit** `refactor: drop collapsed version suffixes, move files, remove always-true checks`. Run the fast gate. Expected: PASS. Check the byte-identity table.

### Task 6: Documents and final verification (D11, §6)

**Files:**
- **`docs/STATE_HASHING.md`:**
  - the status line;
  - the digest table (delete the V5, V6 and `InformationStateDigestV2` rows);
  - "FullStateDigestV7 current identity" becomes a 14-element preimage table (index, content, frozen tags);
  - "FullStateDigestV6 input layer" and "# FullStateDigestV5 input layer" become component descriptions of the preimage: card-rules record, SBA encoding, combat 3/5-element form, and `foundation_sources` as the fixed empty array;
  - delete "# InformationStateDigestV2";
  - the components intro no longer says "through its V6/V5 layers";
  - `zones_v2` and `execution_v4` get top-level shape descriptions naming `crates/mtgml-state/src/digest.rs` as the normative encoder;
  - "Conversion and reader rules" loses its `EngineState` → detached V3 conversion.
- **The other normative docs:**
  - `docs/contracts/ENGINE_STATE_CLOSURE.md`: tree, validation ownership, versioning, current closure;
  - `docs/DOMAIN_MODEL.md`: tree and `validate_engine_state`;
  - `docs/INFORMATION_MODEL.md`: rename the V2 retained-knowledge section to the current type and keep its content; update the current products and validation sections;
  - `docs/contracts/WIRE_CONTRACT.md`: Versioning, and digest ownership now names V3;
  - `docs/ML_ENVIRONMENT.md`, `docs/EXECUTION_MODEL.md`, `docs/DECISION_PROTOCOL.md` (authoritative forms and candidate ordering), `docs/REPLAY_AND_DETERMINISM.md`, `docs/ARCHITECTURE.md`, `README.md`;
  - `docs/contracts/{ACCEPTANCE_GATES,CARD_DEFINITION_CONTRACT,ML_CONTRACT}.md`, `docs/RNG_CONTRACT.md`, `docs/PROJECT_STRUCTURE.md`, `docs/maintenance/{API_LIFECYCLE,SCHEMA_EVOLUTION}.md`.
- **This plan and the spec:** set Status to IMPLEMENTED.

- [ ] **Step 1: Edit** as listed. Every sentence must describe the code as it now is; grep type names in `crates` and `python/src` when unsure.
- [ ] **Step 2: No leftovers.** Run:

  ```bash
  rg -n "predecessor_v5|EngineStatePartsV2|digest_v5|digest_v6|FullStateDigestV[56]|FullStateDigestInputV[567]|ObservationEnvelopeV1|PlayerInformationStateV2|InformationStateDigest(Input)?V2|observation-envelope\.v1|information-state-envelope\.v2|AuthoritativeRuleEvent(Kind)?V2|SemanticDeltaOperationV2|CandidateOrderingV[12]|VisibleCandidateV2|ExecutionStateV3|PendingDecisionRecordV2|FoundationCreatureSource" crates python scripts .github justfile schemas wire persistence contracts docs README.md --glob '!docs/superpowers/**' --glob '!docs/reviews/**' --glob '!docs/adr/**' --glob '!docs/M1_1_*' --glob '!docs/CONTRACT_CLOSURE_M0_1_1.md' --glob '!docs/rules/M3_*'
  ```

  Expected: no match, except byte-frozen fixture text, which is exempt under spec §6:
  - the V7 `schemas` map in the eight `content-contract-*` negatives;
  - the `purpose` and `parent_v6_fixture` fields of `full-state-digest-v7-kat.v1.json`.
- [ ] **Step 3: Coverage table (D12).** Check each row against `cargo test --workspace --locked -- --list` and the Python test list. Fix any row that differs and fill in the counts.

| Module | Tests that still exercise it |
|---|---|
| `mtgml-state` `engine.rs` (flat state, validation) | `g0d_state_authority` (incl. `flat_state_exposes_every_component_at_the_top_level`), `src/tests/{validation,zones_allocators,batch_d}.rs`, ported card-rules tests from `engine_state_parts_v2` |
| `mtgml-state` `digest.rs`, `card_rules.rs` | `digest_sensitivity` (5), `g0e_digest` (`9e8064…`, V7 KAT, stack/effect/trigger), `semantic_mutations::tests`, Python `test_g0c_digest_identity_fixtures`, `test_authoritative_state_coverage` |
| `mtgml-state` `delta.rs` | `g0e_digest` `state_delta_*`, `untap_delta` |
| `mtgml-rules` `events.rs`, `basic_land.rs`, `turn_progression.rs`, `zone_incarnation.rs` | their module tests, `tests/{semantic_delta,zone_incarnation}.rs`, `random_smoke` (short and long pins), `production_turn` |
| `mtgml-decision` `v4.rs`, `ordering.rs`, `answer.rs`, `response.rs` | `v4::tests`, the ported `assign_dense` tests, `src/tests.rs`, Python `test_decision_v4`, `test_decision_response_v3` |
| `mtgml-observation` `observation.rs`, `information.rs`, `knowledge.rs`, `observed_event_v4.rs`, `player_step_v4.rs`, `magic_*` | `src/tests.rs`, `provenance_tests` (ported), their module tests, wire goldens/negatives, Python `test_observation_v3`, `test_observation_digest_binding`, `test_player_api`, `test_g0g_player_products` |
| `mtgml-environment` projection, runtime, checkpoint | `magic_basic_land_observation` (7), `basic_land_runtime_v8`, `successor_turn_projection`, `checkpoint_v8` tests, `current_successor_api`, `production_turn`, `random_smoke` |
| `mtgml-wire` observation, fixtures | `tests.rs` manifest tests, `constructive_producer_tests` (ported) |
| `mtgml-replay` `randomness.rs`, `contract_material.rs`, `v8.rs` | `v8::tests`, `tests/content_contract_material.rs`, Python `test_g0h_replay_v8` |

- [ ] **Step 4: Commit** `docs: describe the flat current format`. Then run `.venv/Scripts/python.exe scripts/run_checks.py integration`. Expected: PASS, including the release `random_smoke` with the long pin. Run the byte-identity command one last time. Expected: exactly the Task 1, 2 and 5 paths.
- [ ] **Step 5: Report the known gaps.** Report them; do not fix them.
  - There is no prose byte specification of the nested `zones_v2` / `execution_v4` encodings or of `InformationStateDigestV3` (spec D11).
  - `mtgml.ObservationEnvelope` changed meaning (Review Focus 5).
  - The stage-2 gaps still stand: no wire golden for `decision-response.v3` or `magic-basic-land-observation.v1`.
