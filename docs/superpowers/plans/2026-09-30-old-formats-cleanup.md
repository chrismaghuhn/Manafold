# Old Formats Cleanup (Stage 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status:** IMPLEMENTED on branch `chore/old-formats`, 2026-09-30 (approved by the owner; executed inline).

**Goal:** Delete every historical format that no current type and no production path reaches, one format family per commit, leaving the current format byte-identical.

**Architecture:** Deletion runs in dependency order, removing consumers before what they consume:
1. replay
2. checkpoint and historical digests
3. catalog and synthetic path
4. observation
5. decision
6. documents

Each family goes vertically through Rust, wire codecs, schemas, fixtures, Python and gate pins in one commit. Mixed files keep their current items in place (spec D1); moving and renaming is stage 3.

**Tech Stack:** Rust workspace (cargo, clippy), Python client (`python/src/mtgml`, unittest), gate scripts under `scripts/`.

**Spec:** `docs/superpowers/specs/2026-09-30-old-formats-cleanup-design.md`

## Global Constraints

- **Current format stays byte-identical** (spec §6). After every task, this command:

  ```bash
  git diff --name-only --diff-filter=M master -- wire schemas persistence examples
  ```

  prints nothing except these paths:
  - `wire/golden/manifest.json`
  - `wire/negative/manifest.json`
  - `schemas/README.json`
  - `examples/golden-path/index.json`
  - from Task 3 on, `persistence/golden/semantic-contract-kat.v1.json`, which loses only its two synthetic cases
  - any `persistence/*/manifest.json` that lists deleted files
- **The kept current contracts:**
  - `replay-manifest.v8`, `replay-step.v8`, `authoritative-replay.v8`
  - `player-decision-request.v4`, `decision-response.v3`
  - `player-step.v4`, `observed-event-envelope.v4`
  - `observation-envelope.v2`, `information-state-envelope.v3`
  - `magic-shared-execution-observation.v1`, `magic-basic-land-observation.v1`
  - `episode-status.v1`
  - `observation-envelope.v1` and `information-state-envelope.v2`, which the V3 information state is produced through (spec §4 item 4, "stays")

  Their schemas, goldens, negatives, Rust decoders and Python decoders stay.
- **D1:** in a mixed file only historical items go; current items keep their name and file.
- **D7:**
  - a test that exercises only deleted code is deleted;
  - a test of remaining code that uses a deleted type is ported;
  - a test that passes before its implementation step is a finding.
- **Honest reporting (AGENTS.md §6).** Commit before running gate scripts; a failure caused only by uncommitted files is not a code failure.
- **Fixture ownership:** a fixture, example, negative, schema or manifest entry goes in the task that deletes the last code reading it. If two tasks could own it, it goes in the earlier one.
- **Hidden current use.** If the compiler or the Python import graph shows that an item this plan deletes is reached by current code, the item stays. The executor records a ruling (spec §7).
- **Commands used throughout:**
  - `cargo check --workspace --all-targets --locked`
  - `cargo test --workspace --locked`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `.venv/Scripts/python.exe scripts/run_python_tests.py --profile full`
  - `.venv/Scripts/python.exe scripts/run_checks.py fast`

## Review Focus

1. **A client loads a historical document** (for example a V7 replay, a `decision-response.v2`, a `player-step.v3`) through `mtgml.wire.decode_canonical`. It must fail with the precise `fixture.unknown_contract` code, never be parsed as something else. `DeletedContractTests` in Tasks 1, 4 and 5 pins this.
2. **A stored manifest or identity with `synthetic_legacy` / `synthetic_rules_compat`** must fail decoding with an unknown-variant error in Rust and Python, not panic and not fall back. The Task 3 tests pin this.
3. **Shrinking a mixed Python module must not break the import of `mtgml` or of the current modules.** Examples are `_replay_v8` importing `_replay_v2`–`_replay_v7`, and `_player_step_v4` importing `_player_step_v2`. The golden round-trip `test_every_golden_fixture_roundtrips_to_identical_bytes` and `verify_repository.py` run after every task.
4. **The V3 information state keeps its bytes while observation V1–V3 go.** The V1 envelope and the V2 digest stay (Task 4); the kept goldens and `test_g0g_player_products` must pass unchanged.
5. **Checkpoint V8 restore still rejects tampering** under the real basic-land admission after its tests leave the catalog (Task 3, the ported `v8_restore_rejects_state_digest_and_checkpoint_identity_tampering`).

---

### Task 1: Replay V1–V7

**Files:**
- Delete:
  - `crates/mtgml-replay/src/{v1,v3,v4,v5,v6,manifest,recorder,tests}.rs`
  - `crates/mtgml-replay/tests/{gen_v5_fixtures,gen_v6_fixtures,p0_red,replay_v5_red,replay_v6_red,replay_v7_red}.rs`
- Modify (mixed, keep current items):
  - `crates/mtgml-replay/src/v2.rs`: keep `RandomnessIdentityV2`, `deserialize_root_seed_hex`.
  - `crates/mtgml-replay/src/v7.rs`: keep `ContentContractMaterialV1`, `SemanticContractMaterialV7`, `MAX_CONTENT_MANIFEST_*` and their serde impls.
  - `crates/mtgml-replay/src/identity.rs`: keep `KernelIdentityV1`, `DeckIdentityV1`.
  - `crates/mtgml-replay/src/validation.rs`: drop variants nothing constructs.
  - `crates/mtgml-replay/src/lib.rs`
- Modify: `crates/mtgml-wire/src/{replay.rs,fixtures.rs,lib.rs}` (V1–V7 codec impls and `decode_named` arms).
- Delete schemas and fixtures:
  - schemas `replay-manifest.v{1..7}`, `authoritative-replay.v{1..7}`
  - replay V7 examples and negatives in `schemas/examples`, `schemas/negative`
  - replay V1–V7 wire goldens and negatives, with their manifest entries
  - `persistence/golden/m4-phase9-replay-v7-admission-fixtures*` and `m4-phase2-wire-vector-index*`, if replay tests were their last readers
- Python:
  - delete `python/src/mtgml/_replay_v1.py`, `_replay_v3.py`;
  - shrink `_replay_v2.py` and `_replay_v4.py`–`_replay_v7.py` to what `_replay_v8.py` imports;
  - update the `replay.py`, `wire.py` (`_DECODERS`) and `__init__.py` facades;
  - delete `python/tests/test_v5_replay.py`, `test_v6_replay.py`, `test_v7_replay.py`;
  - update `test_schema_parity.py` for the schema names.
- Gates:
  - `scripts/verify_repository.py`: delete the replay token checks, the required `schemas/replay-manifest.v1` file, and the replay `tests.rs` / test-name pin (lines ~115-117, ~305-326);
  - `scripts/validate_schemas.py` (`WIRE_MAPPING`, example lists);
  - `schemas/README.json`;
  - `examples/golden-path/index.json`: `wire_fixtures` becomes the current goldens `player-decision-request.v4`, `observation-envelope.v2`, `information-state-envelope.v3`, `observed-event-envelope.v4`, `player-step.v4`, `episode-status.v1`, `replay-manifest.v8`, `authoritative-replay.v8`, using exact file names from `wire/golden/manifest.json`.
- Test: `python/tests/test_wire_contracts.py`.

**Interfaces:**
- Produces: `python/tests/test_wire_contracts.py::DELETED_CONTRACTS`, a module-level tuple that Tasks 4 and 5 extend; `DeletedContractTests.test_deleted_contracts_are_unknown`.

- [ ] **Step 1: Write the failing test** in `python/tests/test_wire_contracts.py`:

```python
DELETED_CONTRACTS: tuple[str, ...] = tuple(
    f"{name}.v{version}"
    for name in ("replay-manifest", "authoritative-replay")
    for version in range(1, 8)
)


class DeletedContractTests(unittest.TestCase):
    def test_deleted_contracts_are_unknown(self) -> None:
        for contract in DELETED_CONTRACTS:
            with self.subTest(contract=contract):
                with self.assertRaises(WireError) as caught:
                    decode_canonical(contract, b"{}")
                self.assertEqual(caught.exception.code, "fixture.unknown_contract")
```

- [ ] **Step 2: Run it.** `.venv/Scripts/python.exe -m pytest -q python/tests/test_wire_contracts.py -k deleted`. Expected: FAIL. The historical decoders still exist and reject `{}` with a different code.
- [ ] **Step 3: Delete** the Rust files, codec impls, `decode_named` arms, schemas, examples, negatives, fixtures and Python modules listed above. Shrink the mixed files to their current items.
- [ ] **Step 4: Compile.** Run `cargo check --workspace --all-targets --locked`, then `cargo clippy --workspace --all-targets --locked -- -D warnings`. Expected: PASS. A compile error from a current crate means a hidden current use (Global Constraints).
- [ ] **Step 5: Run.**
  - `cargo test --workspace --locked`: PASS.
  - `.venv/Scripts/python.exe scripts/run_python_tests.py --profile full`: PASS, including `test_deleted_contracts_are_unknown`, `test_g0h_replay_v8`, and the golden round-trip.
  - The byte-identity command from Global Constraints: only allowed paths.
- [ ] **Step 6: Commit and gate.** `git commit -m "chore: remove replay formats V1–V7"`, then `.venv/Scripts/python.exe scripts/run_checks.py fast`. Expected: PASS.

### Task 2: Checkpoint V4–V7 and historical digests

**Files:**
- Delete:
  - `crates/mtgml-environment/src/checkpoint.rs`, `checkpoint_v7.rs`
  - `crates/mtgml-environment/tests/checkpoint_v5_red.rs`, `checkpoint_v6_red.rs`
  - `crates/mtgml-state/src/digest_v3.rs`, `digest_v4.rs`
  - `crates/mtgml-persistence/tests/checkpoint_digest_v5_red.rs`, `s3_p0_checkpoint_digest_v6_red.rs`, `p0_red.rs` (V4)
  - `crates/mtgml-state/tests/s3_p0_digest_v5_red.rs` and the V4/V6 parts of `crates/mtgml-state/src/tests/digest.rs`
  - `crates/mtgml-model/tests/s3_p0_identity_red.rs`, and `crates/mtgml-model/tests/p0_red.rs` if it only tests V4 identities
- Modify:
  - `crates/mtgml-environment/src/{lib.rs,errors.rs}`: drop the checkpoint re-exports and `ControllerError::{CheckpointValidation, CheckpointV7}`.
  - `crates/mtgml-environment/src/tests/magic_basic_land_observation.rs`: delete its checkpoint-V7 test (around lines 961-1003).
  - `crates/mtgml-persistence/src/checkpoint_digest.rs`: keep `calculate_checkpoint_digest_v8` and its helpers; delete V3–V7, `validate_checkpoint_digest_inputs`, `checkpoint_payload_v5`, `checkpoint_payload` and the V3–V7 constants.
  - `crates/mtgml-state/src/digest_v5.rs`, `digest_v6.rs`: delete `calculate_full_state_digest_v5*`, `calculate_full_state_digest_v6*`, `calculate_full_state_digest_v6_payload`, `verify_full_state_digest_v6`, `canonical_state_bytes_v6`. Keep what `persisted_v7.rs` calls.
  - `crates/mtgml-state/src/engine.rs`: delete `EngineState::digest`.
  - `crates/mtgml-state/src/engine_state_parts_v2.rs`: delete `full_state_digest_v6`.
  - `crates/mtgml-state/src/delta.rs`, `delta_v2.rs`: delete the `StateDelta`, `StateDeltaV2` structs and their errors; keep the operation enums.
  - `crates/mtgml-model`: delete the historical digest newtypes nothing uses any more (`FullStateDigest`, `FullStateDigestV2`–`V6`, `CheckpointDigestV3`–`V7`).
  - `crates/mtgml-state/src/lib.rs`, `crates/mtgml-persistence/src/lib.rs`.
  - Not here: `ExecutionStateV3::selected_bindings` is removed in Task 5 with `DecisionResponseV2`, although only `checkpoint_v7` tests call it.
- Delete fixtures:
  - `persistence/golden/checkpoint-digest-v{5,6,7}-kat*`, `full-state-digest-v6-kat*`, `m4-phase2-historical-byte-invariance*`
  - `persistence/negative/m4-phase2-checkpoint-v7-negatives.v1.json`, `m4-v6-*.cbor`
- Python:
  - `persistence.py`: delete the checkpoint-digest V3–V7 functions;
  - delete `python/tests/test_v5_persistence.py`, `test_v6_persistence.py`, `test_m4_phase2_successor_vectors.py`, `m4_phase2_reference.py`;
  - `test_persistence_codec.py`: delete the checkpoint V3 KAT.

**Interfaces:**
- Consumes: Task 1 (no replay code uses these digests any more).
- Produces: nothing new. `EnvironmentCheckpointV8` and `calculate_checkpoint_digest_v8` are unchanged.

This task deletes dead code only and adds no behavior, so it has no failing-first test. Its evidence is the unchanged current KATs: `checkpoint-digest-v8-kat` and `full-state-digest-v7-kat` (through `g0e_digest` and the persistence tests) must stay green and byte-identical.

- [ ] **Step 1: Delete** the files and items listed above.
- [ ] **Step 2: Compile.** `cargo check --workspace --all-targets --locked` and clippy `-D warnings`. Expected: PASS.
- [ ] **Step 3: Run.**
  - `cargo test --workspace --locked`: PASS, including `g0e_digest` (`v7_detached_g0c_identity_fixture_matches_its_domain_kat`), the persistence checkpoint V8 KAT test, and the environment `checkpoint_v8` tests.
  - The full Python profile: PASS.
  - The byte-identity command: only allowed paths.
- [ ] **Step 4: Commit and gate.** `git commit -m "chore: remove checkpoint formats V4–V7 and historical digests"`, then run the fast gate. Expected: PASS.

### Task 3: Semantic catalog and synthetic-legacy path (D3, D4)

**Files:**
- Modify: `crates/mtgml-environment/src/checkpoint_v8.rs`: tests, `validate_program_state`, and the non-profile API.
- Delete:
  - `contracts/catalog/semantic-contracts.v1.json`
  - `scripts/generate_semantic_contract_catalog.py`
  - `crates/mtgml-environment/src/semantic_catalog_generated.rs`, `semantic_catalog_kat.rs`
  - `python/tests/test_semantic_contract_catalog_generator.py`
- Modify:
  - `crates/mtgml-environment/src/lib.rs`: remove the catalog modules and re-exports (lines ~18-19, 52-73).
  - `crates/mtgml-environment/src/errors.rs`: remove `ControllerError::SemanticContractUnsupported` only if nothing but `tests/current_successor_api.rs:33-52` constructs it; if so, delete that probe as well.
  - `justfile`: remove the catalog check line.
  - `python/tests/test_python_test_profiles.py`: remove `test_justfile_contracts_includes_the_catalog_check`.
  - `crates/mtgml-model/src/{semantic_contract.rs,execution_identity.rs}`: remove `RulesAuthorityV1::SyntheticLegacy`, `ExecutionProgramV1::SyntheticRulesCompat`, `RulesContractManifestValidationError::SyntheticLegacyClosurePresent` and their arms; update the doc comments.
  - `crates/mtgml-persistence/src/{checkpoint_digest.rs,semantic_contract_digest.rs}`: remove the synthetic arms.
  - `persistence/golden/semantic-contract-kat.v1.json`: remove the cases `rules_synthetic_legacy` and `semantic_synthetic_null_null`.
  - `crates/mtgml-persistence/tests/semantic_contract_digest_red.rs` and `python/tests/test_v5_contract_digest.py`: remove the synthetic cases.
  - Python `persistence.py` (`_VALID_PROGRAM_KINDS`, `_validate_rules_contract_manifest`, rules-id encoding) and `_replay_v5.py` / `_replay_v7.py` remnants: remove `synthetic_legacy` / `synthetic_rules_compat`.
- Tests:
  - `crates/mtgml-model/tests/semantic_contract_red.rs`, `execution_identity_red.rs`
  - `python/tests/test_v5_contract_digest.py`
  - the `checkpoint_v8.rs` test module

**Interfaces:**
- Consumes: `crate::basic_land_runtime_v8::fixtures::{game_admission, state_with_two_lands}`, and the profile API of `EnvironmentCheckpointV8`: `new_for_basic_land_profile`, `restore_detached_for_basic_land_profile`, `fork_detached_for_basic_land_profile`, `restore_with_verified_contracts_for_basic_land_profile(&admission, admission.semantic_contract_manifest(), admission.rules_contract_manifest(), Some(admission.verified_catalog()))`.
- Produces: `ExecutionProgramV1` with the single variant `MagicRules`, and `RulesAuthorityV1` with the single variant `ComprehensiveRules`.

- [ ] **Step 1: Port the checkpoint V8 tests.**
  - `checkpoint()` builds its checkpoint with `EnvironmentCheckpointV8::new_for_basic_land_profile(&admission, state_with_two_lands(), EpisodeStatus::Running, EnvironmentLimitCounters::default(), admission.execution_identity().clone())`, using `admission = game_admission()`.
  - `v8_restore_and_fork_preserve_detached_state_and_identity` uses the three profile methods and keeps its assertions, including `calculate_full_state_digest_v7(&checkpoint.state) == checkpoint.state_digest`.
  - `v8_restore_rejects_state_digest_and_checkpoint_identity_tampering` keeps its expectations, `StateDigest` and `Identity`, through `restore_detached_for_basic_land_profile`.
- [ ] **Step 2: Run.** `cargo test -p mtgml-environment --locked checkpoint_v8`. Expected: PASS. The ported tests exercise existing profile code.
- [ ] **Step 3: Write the failing tests.**
  - In `crates/mtgml-model/tests/semantic_contract_red.rs`:

    ```rust
    #[test]
    fn synthetic_legacy_rules_authority_is_rejected() {
        let error = serde_json::from_str::<RulesAuthorityV1>(r#"{"variant":"synthetic_legacy"}"#)
            .unwrap_err();
        assert!(error.to_string().contains("unknown variant `synthetic_legacy`"));
    }
    ```

  - In `crates/mtgml-model/tests/execution_identity_red.rs`:

    ```rust
    #[test]
    fn synthetic_rules_compat_program_is_rejected() {
        assert!(serde_json::from_str::<ExecutionProgramV1>(r#""synthetic_rules_compat""#).is_err());
    }
    ```

  - In `python/tests/test_v5_contract_digest.py`, as `test_synthetic_legacy_rules_authority_is_rejected`: `calculate_rules_contract_id_v1({"rules_authority": {"variant": "synthetic_legacy"}, "capability_closure": None})` raises the persistence error with code `semantic_validation` and message `rules authority variant is unknown`.
- [ ] **Step 4: Run them.** `cargo test -p mtgml-model --locked synthetic`, and the Python test by name. Expected: all three FAIL, because the variants still decode.
- [ ] **Step 5: Delete the catalog and synthetic path.**
  - Delete the catalog files, the variants and all their arms.
  - Delete the synthetic branch of `validate_program_state`. If the function is then empty, delete it and its call, and delete `CheckpointV8Error::ProgramState`.
  - Delete `restore_detached`, `restore_with_verified_contracts` and `fork_detached` if `rg "\.restore_detached\(|\.restore_with_verified_contracts\(|\.fork_detached\(" crates` finds no caller. `new` stays, because the runtime tests use it.
  - Update the existing tests that assert both program values: `execution_program_decodes_both_canonical_values`, `execution_program_wire_values_are_exact`, and `adr_0055_program_authority_pairs_are_closed_and_symmetric` in `execution_identity.rs`. They now assert the single remaining pair.
- [ ] **Step 6: Run.**
  - `cargo check` and clippy: PASS.
  - `cargo test --workspace --locked`: PASS, including the three new tests and `semantic_contract_digest_red`.
  - The full Python profile: PASS.
  - The byte-identity command: only allowed paths. `git diff master -- persistence/golden/semantic-contract-kat.v1.json` shows only the two removed cases.
- [ ] **Step 7: Commit and gate.** `git commit -m "chore: remove the semantic catalog and the synthetic-legacy path"`, then run the fast gate. Expected: PASS.

### Task 4: Observation, steps and events V1–V3

**Files:**
- `crates/mtgml-observation/src`:
  - delete `observed_event.rs` and `player_step_v3.rs`;
  - in `player_step.rs`, delete `PlayerStep` and `PlayerStepV2`, and keep `PlayerSubmissionCodeV1`, `PlayerServiceErrorCodeV1`, `PlayerStepSubmissionV1`;
  - in `observed_event_v3.rs`, delete `ObservedEventKindV3`, `ObservedEventEnvelopeV3`, the observation `ManaPoolChangeCauseV1` and `TryFrom<V2>`, and keep `ObservedCounterKindV3`, `ManaPoolAfterV1`, `ObservedFaceV1`;
  - in `information.rs`, delete `InformationStateEnvelope`;
  - in `magic_observation.rs`, delete `MagicObservation` and `MagicObservationV2`–`V4` with their combat structs and schema constants, and keep lines ~14-135;
  - in `synthetic_observation.rs`, delete `SyntheticObservation` and `SYNTHETIC_OBSERVATION_SCHEMA_V1`, and keep the step and priority enums;
  - update `lib.rs` constants and re-exports;
  - delete the historical tests in `src/tests.rs` and `src/tests/batch_f.rs`, and `tests/p0_red.rs`.
- `crates/mtgml-wire/src`:
  - `observation.rs`: delete the impls for the deleted types, and keep `compute_information_state_digest_v2/v3` and the current impls;
  - `fixtures.rs`: delete the matching `decode_named` arms;
  - `tests.rs`: delete lines ~22-44 (V7/V3 example includes);
  - `constructive_producer_tests.rs`: delete the V1/V2 parts;
  - delete `tests/p0_red.rs`.
- `crates/mtgml-environment/src/tests/magic_basic_land_observation.rs`: delete the `PlayerStepV3` test, which uses `schemas/examples/player-step-v3-event-next-decision.json`.
- Delete schemas:
  - `observed-event-envelope.v{1,2,3}`, `player-step.v{1,2,3}`, `information-state-envelope.v1`
  - `magic-m3-observation.v1`, `magic-combat-observation.v{2,3,4}`, `synthetic-m3-observation.v1`
  - their examples (`observed-event-v3-*`, `player-step-v3*`), negatives, and wire goldens and negatives
- `scripts/generate_contracts.py`: stop generating `observed-event-envelope.v1.schema.json`. In `contracts/catalog/contract-vocabulary.v1.json`, remove the entries that only the V1 event schema or the deleted payload codecs used; regenerate the Python `_generated_contract_vocab.py` with the script, not by hand.
- Gates: `validate_schemas.py`, `schemas/README.json` and the wire manifests, for the observation schemas.
- `scripts/verify_repository.py`: remove the `observed_event.rs` / `events.py` / V1 event-schema checks (around lines 332-345) and the required `observed-event-envelope.v1` and `player-step.v1` schemas.
- Python:
  - delete `_player_step_v3.py`, `_events_v2.py`, `_information_v2.py`, `_magic_combat_observation.py`, `_magic_combat_observation_v3.py`, `_magic_combat_observation_v4.py`;
  - shrink `_player_step_v2.py`, `_events_v3.py`, `_magic_observation.py`, `_synthetic_observation.py` and `events.py` to what current modules import;
  - `_observation_v1.py` keeps `observation_digest_from_payload` and the V1 envelope;
  - update the facades;
  - delete `test_magic_m3_observation.py`, `test_player_step_revision_progression.py`, `test_batch_f.py` (V2), `test_p0_red.py`, `test_m3_p0_green03.py`;
  - shrink `test_observation_v3.py`, `test_observation_digest_binding.py`, `test_player_api.py` and `test_schema_parity.py`;
  - delete `test_constructive_producers.py` if all its cases are V1/V2. It is a smoke member: remove it from `SMOKE_TESTS` in `scripts/run_python_tests.py` and repoint `test_smoke_profile_rejects_an_empty_allowlisted_member` in `test_python_test_profiles.py` to another smoke member.

**Interfaces:**
- Consumes: `DELETED_CONTRACTS` from Task 1.
- Produces: unchanged `PlayerStepV4`, `ObservedEventEnvelopeV4`, `PlayerInformationStateV3` and `ObservationEnvelopeV2`.

- [ ] **Step 1: Extend the failing test.** Add these to `DELETED_CONTRACTS`:
  - `observed-event-envelope.v1`, `.v2`, `.v3`
  - `player-step.v1`, `.v2`, `.v3`
  - `information-state-envelope.v1`
  - `magic-m3-observation.v1`
  - `magic-combat-observation.v2`, `.v3`, `.v4`
  - `synthetic-m3-observation.v1`
- [ ] **Step 2: Run.** `.venv/Scripts/python.exe -m pytest -q python/tests/test_wire_contracts.py -k deleted`. Expected: FAIL on the observation contracts.
- [ ] **Step 3: Delete** the items listed above.
- [ ] **Step 4: Compile.** `cargo check --workspace --all-targets --locked` and clippy. Expected: PASS.
- [ ] **Step 5: Run.**
  - `cargo test --workspace --locked`: PASS, including `player_step_v4::tests`, `detached_v2::tests`, `provenance_tests`, `successor_turn_projection`, `magic_basic_land_observation` and `production_turn`.
  - The full Python profile: PASS, including `test_g0g_player_products`, `test_observation_v3` and the golden round-trip.
  - `.venv/Scripts/python.exe scripts/generate_contracts.py --check`: PASS.
  - The byte-identity command: only allowed paths.
- [ ] **Step 6: Commit and gate.** `git commit -m "chore: remove observation, step and event formats V1–V3"`, then run the fast gate. Expected: PASS.

### Task 5: Decision V1–V3, player-facing

**Files:**
- `crates/mtgml-decision/src`:
  - `v1.rs`: keep only `ActionCandidate`.
  - `v2.rs`: delete `PlayerDecisionRequestV2`, `DecisionAnswerV2::validate_for`, `DecisionResponseV2` and the V2 schema constants. Keep `DecisionDomainV2`, `validate_candidates`, `DecisionAnswerV2` with `validate_for_candidate_ids`/`validate_shape`, and `VisibleCandidateV2`.
  - `v3.rs`: delete `PLAYER_DECISION_REQUEST_V3_SCHEMA`, `PlayerDecisionRequestV3` and `AuthoritativeDecisionRequestV3::project_player_request`, and the historical part of the test module. Keep `DecisionResponseV3`, its schema constant, and the candidate, binding and authoritative-request items listed in spec §4 item 5.
  - `authoritative.rs`: delete `AuthoritativeDecisionRequestV2::project_player_request`.
  - `lib.rs`; `src/tests.rs`: delete the V1/V2 request and response tests.
- `crates/mtgml-state/src/execution.rs`: delete `ExecutionStateV3::selected_bindings`.
- `crates/mtgml-wire/src/{decision.rs,fixtures.rs,lib.rs}`: delete the V1/V2 impls and `decision_response_v2`.
- `crates/mtgml-environment/src/tests/magic_basic_land_observation.rs`: delete or port the `PlayerDecisionRequestV3` test (around lines 1218-1237). Port it only if it asserts current behavior; otherwise delete it.
- Delete:
  - schemas `player-decision-request.v{1,2,3}` and `decision-response.v{1,2}`, with their examples (`player-decision-request-v3-*`), negatives (`pdr-v3-*`, `m4-phase2-decision-v3-semantic-negatives`), and wire goldens and negatives;
  - `wire/historical/`.
- Python:
  - delete `decision_v3.py`;
  - shrink `decision.py` to `DecisionResponseV3`, `DecisionAnswerV2`, `DecisionSpec` and what `decision_v4.py` imports;
  - update the facades;
  - delete `test_decision_v3.py` and `test_m2_b_staging_fixtures.py`, which read `wire/historical` and are a smoke member (update `SMOKE_TESTS` and its profile test);
  - shrink `test_player_api.py` and `test_schema_parity.py`.
- Gates: `verify_repository.py`, `validate_schemas.py`, `schemas/README.json` and manifests, for the decision schemas.

**Interfaces:**
- Consumes: `DELETED_CONTRACTS` from Task 1, extended in Task 4.
- Produces: unchanged `PlayerDecisionRequestV4`, `DecisionResponseV3` and `decision_response_v3::decode_submission`.

- [ ] **Step 1: Extend the failing test.** Add these to `DELETED_CONTRACTS`:
  - `player-decision-request.v1`, `.v2`, `.v3`
  - `decision-response.v1`, `.v2`
- [ ] **Step 2: Run** the test by name. Expected: FAIL on the decision contracts.
- [ ] **Step 3: Delete** the items listed above.
- [ ] **Step 4: Compile.** `cargo check --workspace --all-targets --locked` and clippy. Expected: PASS.
- [ ] **Step 5: Run.**
  - `cargo test --workspace --locked`: PASS, including `v4::tests`, `v4::hand_size_discard_tests`, `decision::successor_response_tests`, `current_successor_api`, `production_turn` and `random_smoke` (debug).
  - The full Python profile: PASS, including `test_decision_v4` and `test_decision_response_v3`.
  - The byte-identity command: only allowed paths.
- [ ] **Step 6: Commit and gate.** `git commit -m "chore: remove player-facing decision formats V1–V3"`, then run the fast gate. Expected: PASS.

### Task 6: Documents and final verification (D6, §6)

**Files:**
- `docs/REPLAY_AND_DETERMINISM.md`: delete V1 (§8), V2 (§12), V3 (§18), V5 (§55), V6 (§66) and V7 (§105) replay, "ReplayStepV3" (§150), and "Historical support policy" (§235).
- `docs/STATE_HASHING.md`:
  - delete "FullStateDigestV6 historical identity", "Historical V1 and V2", "V3 requirement", "Historical V5", "CheckpointDigestV3" and "Historical CheckpointDigestV6";
  - keep the `FullStateDigestInputV3` component sections (`core_v1` … `format_v1`) and `InformationStateDigestV2`, and give them a one-line lead-in: "These component encodings are still produced by FullStateDigestV7 through its V6/V5 layers".
- `docs/DECISION_PROTOCOL.md`: delete "M1 historical surface" and "Historical PlayLand successor". `docs/INFORMATION_MODEL.md`: delete "Historical M4 successors", keep the `PlayerInformationStateV2` section. `docs/contracts/WIRE_CONTRACT.md`: delete the compatibility notes on deleted contracts. `contracts/ENGINE_STATE_CLOSURE.md`, `docs/RNG_CONTRACT.md`: delete their references to deleted formats (inventory §2.8).
- `docs/PROJECT_STRUCTURE.md` (lines ~14, 37) and `docs/TESTING_AND_CONFORMANCE.md` (~54): stop describing `mtgml-conformance` as current.
- `docs/superpowers/specs/2026-09-30-one-runtime-cleanup-design.md` §4 and `docs/superpowers/plans/2026-09-30-one-runtime-cleanup.md` Task 5: add one line each, "The semantic catalog was deferred to stage 2 and deleted there."
- `README.md`: the paragraph "older format versions (V7 and earlier) keep readers and fixtures until they are removed" now says they are removed.
- This plan and the spec: Status → IMPLEMENTED.

- [ ] **Step 1: Edit** as listed. Every deleted section must be one the code no longer produces or reads. When in doubt, grep the section's type names in `crates` and `python/src`.
- [ ] **Step 2: No leftovers.** Run:

  ```bash
  rg -n "ReplayManifestV[1-7]\b|AuthoritativeReplayV[1-7]\b|EnvironmentCheckpointV7|PlayerDecisionRequestV[23]?\b|DecisionResponseV2|PlayerStepV[23]\b|ObservedEventEnvelopeV[23]?\b|MagicObservationV[234]?\b|SyntheticObservation\b|SyntheticLegacy|SyntheticRulesCompat|synthetic_legacy|semantic_catalog|generate_semantic_contract_catalog" crates python scripts .github justfile schemas wire persistence contracts docs --glob '!docs/superpowers/**' --glob '!docs/reviews/**' --glob '!docs/adr/**'
  ```

  Expected: no match. Any hit is either deleted, or recorded as a ruling explaining why it is current. `PlayerDecisionRequestV4` and `ObservedEventEnvelopeV4` do not match the pattern.
- [ ] **Step 3: Coverage table (D7).** Check each row against `cargo test --workspace --locked -- --list` and the Python test list, and fix any row that differs.

| Module | Tests that still exercise it |
|---|---|
| `mtgml-replay` `v8.rs` (+ kept items of `v2`, `v7`, `identity`, `validation`) | `v8::tests` (7), `v7::tests` (1), `tests/content_contract_material.rs` (4, ported in Task 1), Python `test_g0h_replay_v8` (6), the replay V8 goldens in `test_wire_contracts` |
| `mtgml-environment` `checkpoint_v8.rs` | its module tests (3, ported in Tasks 2–3), `basic_land_runtime_v8` tests (18), `production_turn` (8) |
| `mtgml-environment` runtime, projection, boundary | `basic_land_runtime_v8` tests (18), `successor_turn_projection` (6), `magic_basic_land_observation` (7), `production_turn` (8), `current_successor_api` (2), `random_smoke` (2) |
| `mtgml-decision` `v4.rs`, `v3.rs` (response, candidates), `v2.rs` (domain, answer), `authoritative.rs` | `v4::tests` (12), `v4::hand_size_discard_tests` (1), `v3::tests` (6), `src/tests.rs` (incl. the ported answer/domain tests), Python `test_decision_v4` (14), `test_decision_response_v3` (2) |
| `mtgml-observation` `player_step_v4`, `observed_event_v4`, `detached_v2`, `magic_shared_execution_observation_v1`, V1 envelope, `information.rs` (V2) | `player_step_v4::tests` (2), `detached_v2::tests` (2), `magic_shared_execution_observation_v1::tests` (4), `provenance_tests` (4), the basic-land and digest tests in `src/tests.rs`, Python `test_observation_v3` (5), `test_g0g_player_products` (3), `test_magic_shared_execution_observation_v1` (6) |
| `mtgml-state` digest V7 and the V6/V5 encoders | `g0e_digest` (18), `persisted_v7` tests (5), `src/tests/digest.rs` (V6-layer tests through `decode_v6_payload`, V5-probe tests) |
| `mtgml-state` delta operations, execution, parts V2/V3 | `g0e_digest` (`state_delta_v3_*`), `engine_state_parts_v2::tests` (8), `g0d_state_authority` (26), `semantic_mutations::tests` |
| `mtgml-persistence` `checkpoint_digest.rs` (V8), `semantic_contract_digest.rs` | `checkpoint_digest_v8_g0_known_answer`, the ported FND-017a tests, `semantic_contract_digest_red` (6), Python `test_v5_contract_digest` (6), Python `test_persistence_codec` |
| `mtgml-wire` decision (V4/V3), observation (current impls), replay (V8), fixtures | `decision::successor_response_tests`, the V2/V3 digest KATs in `tests.rs`, the golden and negative fixture verification |
| `mtgml-model` identities, semantic contract | `execution_identity_red` (9), `semantic_contract_red` (20, with the Task 3 tests), unit tests in `lib.rs` |

- [ ] **Step 4: Commit and run the full gate.** `git commit -m "docs: describe only current formats"`, then `.venv/Scripts/python.exe scripts/run_checks.py integration`. Expected: PASS, including the release `random_smoke` games. Then run the byte-identity command one last time. Expected: only allowed paths, and `semantic-contract-kat.v1.json` differing only by the two cases.
- [ ] **Step 5: Report the known gap** (spec §6): the current `decision-response.v3` has no wire golden. Report it, do not fix it.
