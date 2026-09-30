# Flatten the Current Format (Stage 3) — Design

**Status:** IMPLEMENTED on branch `chore/flatten-current-format`, 2026-10-01 (approved by the owner; executed inline).

**Goal:** the current format exists in one layer. There is one flat `EngineState`, one digest encoder, one delta-operation enum, one rule-event enum and one information-state projection. A new state field, event or operation for the R1/W1 creature and combat work is then added in one place.

**Behavior:** unchanged, byte for byte (D1). What the engine gains is protection, not play:
- a committed trajectory fingerprint that pins the bytes of real games (D2);
- one place, instead of three layers, for the next state field, event or operation.

## 1. Why

Stage 2 left each format once. Internally, however, the current format is still built on its predecessors.

- **State.**
  - `EngineStatePartsV3` is `{ predecessor_v5: EngineStateParts, execution_v4, card_rules_state }`.
  - Its `validate` builds an `EngineStatePartsV2`, materializes the old `EngineState` and runs the V2 validation on it.
  - `.predecessor_v5` appears 855 times, 340 of them in production code.
  - The old V2 execution, the V3 execution and `foundation_sources` are always empty in production.
- **Digest.**
  - `FullStateDigestV7` is computed in steps: produce V6 bytes (themselves built on V5), decode them, replace four elements, then re-validate by rebuilding a V6 value.
  - Per digest, this runs the state validation twice and the V6 validators three times, and decodes CBOR three times.
  - The V7 preimage carries the V5/V6 component values inline. A single-pass encoder can therefore produce identical bytes.
- **Delta.** `SemanticDeltaOperationV3` wraps V2, which wraps V1 (16, 12 and 23 variants).
- **Events.**
  - `AuthoritativeRuleEventKindV3` wraps the V1 kind.
  - The basic-land rules draft V2 events. A bridge then converts them, drops two kinds, reorders them and renumbers them.
- **Candidates.** The basic-land rules build V3 candidates with `CandidateOrderingV2`; `basic_land_v4.rs` bridges them to V4.
- **Information state.** Every V3 projection first builds:
  - a V1 observation envelope, with Base64 and a digest;
  - a decode of that envelope;
  - a full `PlayerInformationStateV2` with its own digest.

  It keeps only `retained_knowledge` from all of that. `observation-envelope.v1` and `information-state-envelope.v2` survive as wire contracts only for this inner use.
- **Cost for R1/W1.** Today a new state field touches four places:
  - the V5 encoder;
  - the V6 validator;
  - the V7 re-encoding;
  - two validation layers.

  A new basic-land event also needs a V2→V3 conversion.

## 2. Stage position

This is stage 3 of three (`docs/superpowers/specs/2026-09-30-one-runtime-cleanup-design.md` §2). Stage 1 (PR #263) and stage 2 (PR #264) are merged. The creature and combat work starts after this stage.

## 3. Decisions

- **D1 — Byte-neutral.**
  - What stays byte-identical to `master`: every remaining wire golden, wire negative, schema example, persistence KAT and pinned digest.
  - The only deleted fixtures are those listed in §4 that belong to removed layers.
  - Wire strings stay: `schema_version`, `payload_codec`, the JSON key `"decision_domain_v2"`, and the digest domain and input-schema strings.
  - The single-pass encoder writes the frozen preimage values:
    - the tags `"zones_v2"` and `"execution_v4"`;
    - stack rows `[id, controller, payload]`;
    - the empty `foundation_sources` value at index 11;
    - the combat component's 3/5-element encoding.
- **D2 — Trajectory fingerprint first.**
  - Before any other change, `crates/mtgml-environment/tests/random_smoke.rs` pins a SHA-256 fingerprint of its trajectory.
  - The fingerprint hashes a length-prefixed concatenation of every entry. Each entry contains:
    - the actor;
    - the canonical response bytes;
    - the canonical `PlayerStepV4` bytes;
    - both players' canonical information states;
    - the checkpoint digest, which binds the full-state digest.
  - There are two pins, both for seed `0x4D41_4E41`:
    - the long one: 30 turns, release build;
    - the short one: stops when turn 4 begins, runs in the debug build, so every `cargo test --workspace --locked` checks it.
  - When the pins are added, a mutation check shows that each one detects a change.
  - The pins do not change during stage 3. A change is a bug in the commit that caused it.
- **D3 — Consumers before layers, no transition code.**
  - Each commit removes the users of an old layer, or the layer itself, and deletes the path it replaces (AGENTS.md §4).
  - The order is in §4. Every commit builds and passes the fast gate, `cargo test --workspace --locked`, and the short fingerprint.
- **D4 — Flat state.**
  - The state becomes one struct with these fields:
    - `revision`, `core`, `combat`, `zones`, `allocators`;
    - `execution`, the current `ExecutionStateV4`;
    - `random`, `knowledge`, `perspective_identities`, `format`;
    - `card_rules`, the current `CardRulesAuthoritativeStateV1`.
  - Deleted:
    - the old `EngineState`/`EngineStateParts`;
    - `EngineStatePartsV2`;
    - the V2 execution (`ExecutionState`, `PendingDecisionRecordV2`, `ContinuationRecordV2`) and the V3 execution (`ExecutionStateV3`);
    - `AuthoritativeDecisionRequestV2/V3`;
    - `foundation_sources` and `FoundationCreatureSource`;
    - `StackRecord.source_object` and `StackRecord.source_ability`;
    - the round trips `materialize`, `parts` and `from_state`.
  - Validation is `validate()` and `validate_structure()` on the flat state. The profile-dependent pending-decision check stays in `validate()`. Checks that concern only deleted fields are deleted.
  - The flat state derives no Serde traits unless a remaining user needs them.
  - `mtgml-state/src/construction.rs` stays as the test-state builder and builds the flat state.
- **D5 — Single-pass digest.**
  - Computing the digest does three things, each once: validate the state, encode the 14-element preimage, hash it. The structural-only variant differs only in which validation it runs.
  - The component encoders move into the digest module unchanged.
  - Deleted:
    - the V5 and V6 producers, envelopes and decoders;
    - `FullStateDigestV5` and `FullStateDigestV6`;
    - the V2-execution encoders that nothing else reaches. The V3-execution encoder and validator (`PersistedExecutionV3`, `validate_execution_v3`, and the encoders they call) are still run by `EngineStatePartsV2::validate`, so they go with the flat state (D4);
    - `EngineState::digest` and `canonical_digest_bytes`, with their test calls;
    - the V7 payload decoder (`FullStateDigestInputV7::from_canonical_payload`, `calculate_full_state_digest_v7_payload`) and its rejection tests;
    - `persistence/golden/full-state-digest-v6-kat.v1.json`, `crates/mtgml-state/tests/fixtures/magic-sba-graveyard-order-v5-input.hex`, and their tests.
  - `full-state-digest-v7-kat.v1.json` stays. It is checked through the same hash step production runs after encoding.
  - For each state component in the preimage, a test shows that changing it changes the digest. Existing V7 tests count. Components without one get a test ported from the V6 KAT's mutation cases.
- **D6 — Information state projected directly.**
  - The projection builds `MagicBasicLandObservationV1` from the state, then the shared execution observation, then the observation envelope, then the player information state.
  - `retained_knowledge` is computed by the existing rule, moved out of the V2 path.
  - Deleted:
    - `ObservationEnvelope` (V1), `PlayerInformationStateV2`, `InformationStateDigestInputV2`, `InformationStateDigestV2` and `compute_information_state_digest_v2`;
    - the envelope-returning basic-land projection API;
    - the wire contracts `observation-envelope.v1` and `information-state-envelope.v2` everywhere they appear: Rust codecs, Python modules, schemas, examples, negatives, goldens and manifests.
  - Both contract names join `DELETED_CONTRACTS` in `python/tests/test_wire_contracts.py`.
  - `magic-basic-land-observation.v1` stays; it is a field of the shared observation.
  - The information-safety tests in `crates/mtgml-environment/src/tests/magic_basic_land_observation.rs` are ported with their assertions intact.
  - The deleted fixtures are the only ones that carry non-empty `retained_knowledge` and the only negatives for knowledge provenance, cause/channel pairing, initial invalidation, invalid Base64 and a digest mismatch. That coverage moves into in-code Rust and Python tests on `ObservationEnvelopeV2` and `PlayerInformationStateV3`. D1 allows no new fixture bytes.
- **D7 — Basic land, events and delta on current types.**
  - The basic-land rules produce three things directly:
    - V4 candidates, ordered by the current candidate ordering, with the same dense ids;
    - V3 operations;
    - V3 events, in their final order and numbering.
  - Deleted:
    - the bridge in `basic_land_v4.rs`;
    - `AuthoritativeRuleEventV2` and `AuthoritativeRuleEventKindV2`;
    - the V1 `AuthoritativeRuleEvent` struct; `zone_incarnation` returns the zone transition and the perspective lifecycles instead.
  - Deleted with the flat state (D4), because the V2/V3 execution states still hold or validate them until then:
    - the V3 candidate types;
    - `CandidateOrderingV1` and `CandidateOrderingV2`;
    - `VisibleCandidateV2`, `ActionCandidate`, `CandidateIntent` and `mtgml-decision/src/authoritative.rs`.
  - One rule-event kind enum and one semantic-operation enum remain, without `Existing` wrappers. **Only the wrappers are dissolved; no current vocabulary is removed.** Each enum holds every variant that exists today at any level, except:
    - **duplicates across levels:** the V1 `PerspectiveOccurrence` event, which the V3 `PerspectiveObservationOccurrence` replaces; and the V2 operations `ObjectTapped` (V1), `ManaAdded` and `ManaPoolEmptied` (V3 `ManaPoolChanged`) and `CounterChanged` (V3);
    - **what only the deleted bridge used:** the V2 event kinds, including `LandPlayed` and `ObjectMoved`.
  - The result is 36 event kinds (22 from V1, 14 from V3) and 45 operations (23 from V1, 7 from V2, 15 from V3). The unproduced stack, spell, cost, trigger, damage, counter, effect and combat variants stay for R1/W1 and the spell work, together with their validators, projections and tests.
- **D8 — Names.**
  - The rule: a state, digest, delta, event, candidate or information-state type loses its version suffix when stage 3 deletes every other version of its family.
  - Functions named after a renamed type follow it.
  - Constants that name a wire string, for example `OBSERVATION_SCHEMA_V2`, keep their names.
  - The expected list, which the plan finalizes against the code:
    - **state:**
      - `EngineStatePartsV3` → `EngineState`
      - `EngineStatePartsV3Error` → `EngineStateError`
      - `ExecutionStateV4` → `ExecutionState`
      - `ContinuationRecordV3` → `ContinuationRecord`
      - `ContinuationPayloadV3` → `ContinuationPayload`
      - `StateDeltaV3` → `StateDelta`
      - `SemanticDeltaOperationV3` → `SemanticDeltaOperation`
      - `FullStateDigestV7` → `FullStateDigest`
    - **events:**
      - `AuthoritativeRuleEventV3` → `AuthoritativeRuleEvent`
      - `AuthoritativeRuleEventKindV3` → `AuthoritativeRuleEventKind`
    - **decisions:** `AuthoritativeDecisionRequestV4`, `AuthoritativeCandidateV4`, `CandidateIntentV4`, `EngineCandidateBindingV4`, `VisibleCandidateV4` and `CandidateOrderingV3` drop the suffix.
    - **information state:** `ObservationEnvelopeV2`, `PlayerInformationStateV3`, `InformationStateDigestV3` and `InformationStateDigestInputV3` drop the suffix, in Rust and Python.
  - Not renamed:
    - families left with one version by stage 2, such as `PlayerStepV4`, `PlayerDecisionRequestV4`, `DecisionResponseV3`, `DecisionDomainV2`, `EnvironmentCheckpointV8`, `CheckpointDigestV8` and `ReplayManifestV8`;
    - pairs of types that have different roles, such as the state `CostRoute` and the wire `CostRouteV1`.
- **D9 — Files.**
  - A file whose name carries an old version, or whose main type is renamed, is named after its content.
  - Expected moves in `mtgml-state`:
    - `engine_state_parts_v3.rs` → `engine.rs`;
    - `digest_v5.rs`, `digest_v6.rs`, `digest_v7.rs`, `persisted_v6.rs` and `persisted_v7.rs` → `digest.rs` and `card_rules.rs`;
    - `delta.rs`, `delta_v2.rs` and `delta_v3.rs` → `delta.rs`.
  - Expected moves in `mtgml-rules`:
    - `events_v3.rs` → `events.rs`;
    - `basic_land_v4.rs` → merged into `basic_land.rs`.
  - Expected moves in `mtgml-replay`: `v2.rs` → `randomness.rs`, `v7.rs` → `contract_material.rs`.
  - Expected moves in `mtgml-decision`: `v2.rs` → `answer.rs`, `v3.rs` → `response.rs`; `v1.rs` is deleted.
  - Expected moves in `mtgml-observation`: `observed_event_v3.rs` → merged into `observed_event_v4.rs`.
  - Expected moves in Python:
    - `_replay_v2.py` … `_replay_v7.py` → merged into the current replay modules; the Python-only `SemanticContractMaterialV5` merges into `SemanticContractMaterialV7`;
    - `_player_step_v2.py` → `_player_step_v4.py`;
    - `_events_v3.py` → `_events_v4.py`;
    - `_observation_v1.py` and `_information_v2.py` are deleted (D6).
  - The eight `schemas/negative/replay-v7-content-*.json` files are V7-shaped manifests. Only their content-contract child and presence parts are tested, against the V8 schema definitions and `ContentContractMaterialV1` / `SemanticContractMaterialV7`. They are renamed after that content, with unchanged bytes:
    - `replay-v7-content-child-<case>.json` → `content-contract-child-<case>.json`;
    - `replay-v7-content-presence-mismatch.json` → `content-contract-presence-mismatch.json`;
    - `replay-v7-content-id-without-child.json` → `content-contract-id-without-child.json`.
  - `python/tests/test_authoritative_state_coverage.py` reads Rust source by path; it follows the moves.
- **D10 — Always-true checks are deleted.**
  - `execution_program_matches_rules_authority` and its callers.
  - The `program_kind == MagicRules` checks, and the always-true `ComprehensiveRules` match next to one of them in `player_projection.rs`.
  - Not deleted: the irrefutable `RulesAuthorityV1` binding in `mtgml-replay/src/v8.rs`. It is how `snapshot_id` is read, not a check.
  - The program/authority `if`/`then` blocks in `schemas/replay-manifest.v8.schema.json` and `schemas/authoritative-replay.v8.schema.json`.
  - The duplicate program-kind maps in Python.
  - `RulesAuthorityV1` and `ExecutionProgramV1` stay; their single values are wire vocabulary.
- **D11 — Documents.**
  - Normative documents describe the flat format only:
    - `docs/STATE_HASHING.md` describes the V7 preimage directly. The V5/V6 input-layer sections become the component descriptions of that preimage.
    - `docs/contracts/ENGINE_STATE_CLOSURE.md`, `docs/DOMAIN_MODEL.md`, `docs/INFORMATION_MODEL.md`, `docs/contracts/WIRE_CONTRACT.md`, `docs/ML_ENVIRONMENT.md`, `docs/ARCHITECTURE.md`, `docs/EXECUTION_MODEL.md`, `docs/DECISION_PROTOCOL.md`, `docs/REPLAY_AND_DETERMINISM.md` and `README.md` also describe the flat format.
    - So do the other normative documents that name a deleted or renamed type: `docs/contracts/ACCEPTANCE_GATES.md`, `docs/contracts/CARD_DEFINITION_CONTRACT.md`, `docs/contracts/ML_CONTRACT.md`, `docs/RNG_CONTRACT.md`, `docs/PROJECT_STRUCTURE.md`, `docs/maintenance/API_LIFECYCLE.md` and `docs/maintenance/SCHEMA_EVOLUTION.md`.
  - No document has a prose byte specification of the nested `zones_v2` / `execution_v4` encodings or of `InformationStateDigestV3`. `STATE_HASHING.md` describes their top-level shape and names the encoder as the normative source. A full prose specification is a known gap, reported and not written here.
  - Process documents stay unchanged. So do milestone specification records, whose file names carry a milestone label (`docs/M1_1_*`, `docs/CONTRACT_CLOSURE_M0_1_1.md`, `docs/rules/M3_*`).
- **D12 — Tests** follow the stage-1/2 rule:
  - a test that exercises only deleted code is deleted;
  - a test of remaining behavior that uses a deleted type is ported, with its assertions kept;
  - the plan names, for every remaining module of the touched crates, the tests that still cover it.

## 4. Commit order and inventory

0. **Fingerprint (D2).** Both pins are added to `random_smoke.rs`, and the mutation check is run and recorded.
1. **Single-pass digest (D5).** The encoder reads `EngineStatePartsV3`. The V5/V6 layers, the V7 decoder, the V6 KAT and the V5 `.hex` fixture are deleted.
2. **Direct information state (D6).** The V1 envelope, the V2 information state and their two wire contracts are deleted.
3. **Basic land, events, delta (D7).** The basic-land rules work on `EngineStatePartsV3` and current types. The bridge, the V2 events, the V1 event struct, and the operation and event wrappers are deleted.
4. **Flat state (D4).** `predecessor_v5` is dissolved. The old state, the V2 parts, the V2/V3 execution and their validation are deleted, together with the old candidate types and orderings they hold (D7).
5. **Names, files, always-true checks (D8, D9, D10).**
6. **Documents (D11) and final verification (§6).**

## 5. Out of scope

- Changing the preimage or any wire string (D1).
- Suffixes of families that stage 2 already left with one version, and role pairs (D8).
- `RulesAuthorityV1` and `ExecutionProgramV1` (D10).
- New rules behavior.

## 6. Verification

- **Fingerprints:** the short pin passes at every commit, and the long pin passes before the PR.
- **Byte identity:** `git diff --stat master -- wire persistence schemas/examples schemas/negative` shows only the deletions of D5 and D6 and the renames of D9. The existing pins also still pass:
  - `full-state-digest-v7-kat`;
  - `checkpoint-digest-v8-kat`;
  - the `g0e_digest.rs` state digest;
  - the replay V8 `checkpoint_digest`;
  - the information-state V3 digests.
- **Gate:** `scripts/run_checks.py integration` passes. It includes `cargo test --workspace --locked`, clippy, the full Python profile, and the release `random_smoke`.
- **No leftovers:** a grep outside process documents finds none of the following:
  - `predecessor_v5`, `EngineStatePartsV2`, `digest_v5` or `digest_v6`;
  - the deleted type names;
  - the two deleted contract names.

  Byte-frozen fixtures that carry a deleted name as inert data are exempt. One example is the V7 `schemas` map in the renamed `content-contract-*` negatives.
- **Coverage:** the plan's table (D12) is checked against `cargo test --workspace --locked -- --list` and the Python test list.

## 7. Risks

- **Event order, event numbering and candidate ids** in the basic-land rewrite (D7). All three are in the fingerprinted bytes:
  - event numbers are in the state digest;
  - the event count is in the checkpoint digest;
  - candidate ids are in every request.
- **855 path edits** in the flat-state commit. They are mechanical and compiler-checked.
- **Less digest sensitivity coverage** once the V6 KAT is gone. D5 requires one V7 test per component.
- **Python class renames** break client code. There are no external users (AGENTS.md §4).
- **Size.** Each commit in §4 is reviewable and bisectable on its own, and the fingerprint gates every one of them.
