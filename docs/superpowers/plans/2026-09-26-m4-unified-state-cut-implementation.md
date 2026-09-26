# Unified M4 State Cut Implementation Plan

**Task:** `M4_UNIFIED_STATE_CUT_IMPLEMENTATION_PLAN`
**Status:** accepted; implementation authorized only within the ordered plan below
**Normative source:** [M4 Unified State Cut Semantic Spec](../specs/2026-09-26-m4-unified-state-cut-semantic-spec.md)
**M4.2 status:** implementation in progress under this plan; M4.2 is NOT COMPLETE until final slice acceptance
**Date:** 2026-09-26

## 1. Purpose and exact baseline

This plan implements the Semantic Spec exactly. It adds no contract behavior, fields, event meaning, capability, candidate or mechanic beyond that Spec. If implementation requires such a choice, stop, amend and independently re-review the Spec, then regenerate this Plan before proceeding.

The accepted implementation baseline is `origin/master = 6faf1b970def779adc2a8d8bd14ae8aff331dba4`, which contains the independently accepted Spec, Plan and Replay V7 content-child wire amendment and descends from the accepted M4.1 baseline. #225 records Phase 1 authorization. Implementation must use a dedicated worktree and the non-current integration branch created from that exact master. Do not implement from an earlier SHA or an unreviewed replacement head. Before each stage, verify #222 authority, #225 open/authorized status, accepted audit/spec/plan status and clean isolation; preserve unrelated work.

No stage makes an incomplete FullStateDigestV6 / checkpoint V7 / replay V7 current. Stages 1–5 are built and reviewed on a dedicated integration branch with V5/V6 runtime retained on master. The final activation change is the only commit/PR allowed to make the successor identity current. Stacked implementation PRs target the integration branch; only the fully integrated, fully gated final branch is proposed to master.

## 2. Invariants and fixed acceptance boundary

```text
normative contracts → RED evidence → detached successor DTOs/codecs
→ state-family producers → RulesKernel integration
→ candidate/observation/replay parity → atomic successor activation
```

All accepted transitions pass through the existing environment boundary:

```text
V3 pending decision + V2 response
→ exact trusted binding validation
→ closed MagicActionRequestV1
→ MagicRulesKernel / RulesKernel::apply
→ complete TransitionResult product
→ candidate-state, event, delta, observation, PlayerStep validation
→ atomic commit
```

No direct EngineState mutation from content, Card IR, environment projection, or Python. No hidden policy for land selection or mana source. No trust-ID exposure. A rejection preserves complete before identity and state.

Expected final state identities are only those specified by the Spec: FullStateDigestV6, StateDeltaV2/EngineStatePartsV2, EnvironmentCheckpointV7, CheckpointDigestV7, Replay V7, request V3, response V2, observed-event V3, PlayerStep V3, observation-envelope V1, information-state V2, and `magic-basic-land-observation.v1`.

## 3. Phase 0 — entry audit and work isolation

**Goal:** establish reviewed implementation authority before edits.

**Files likely touched:** none.

**Actions:**

1. Verify independent acceptance of both this Spec and Plan; until then no implementation starts.
2. Merge the accepted documentation branch to master through the repository's independent review process. Fetch origin and verify new `origin/master` contains both accepted docs and descends from the accepted M4.1 baseline.
3. Verify #222 remains M4 authority and #225 remains open/paused. Record actual merge SHA; never assume `7a26e519…` remains current.
4. Create a dedicated implementation worktree/branch from that new exact master, then a non-current integration branch for stacked PRs. Preserve existing audit/spec worktrees and user work.

**RED first:** none; no production change.

**Focused validation:** `git status --short --branch`, `git rev-parse HEAD`, `git rev-parse origin/master`, issue review.

**Commit boundary:** none.

**Stop:** baseline drift not reviewed; #225 resumed without independent approval; Spec or Plan not independently accepted; unrelated dirty tree would be overwritten.

## 4. Phase 1 — freeze successor contracts and historical identities

**Goal:** publish the exact closed Rust/schema/digest identities before successor bytes are emitted.

**Files likely touched:**

- `docs/STATE_HASHING.md`
- `docs/REPLAY_AND_DETERMINISM.md`
- `docs/DECISION_PROTOCOL.md`
- `docs/INFORMATION_MODEL.md`
- `docs/contracts/ENGINE_STATE_CLOSURE.md`
- `docs/contracts/WIRE_CONTRACT.md`
- `docs/contracts/ML_CONTRACT.md`
- `docs/maintenance/SCHEMA_EVOLUTION.md`
- `docs/normative-document-register.v1.json`
- `schemas/README.json`
- `schemas/player-decision-request.v3.schema.json`
- `schemas/observed-event-envelope.v3.schema.json`
- `schemas/player-step.v3.schema.json`
- `schemas/magic-basic-land-observation.v1.schema.json`
- `schemas/replay-manifest.v7.schema.json`
- `schemas/authoritative-replay.v7.schema.json`
- `schemas/examples/`, `schemas/negative/`
- `scripts/validate_schemas.py`
- relevant `docs/adr/` only if existing accepted policy requires a new durable ADR (do not restate an ADR unnecessarily)
- contract vocabulary source under `contracts/catalog/` and generated vocabulary outputs only through their generator

**RED first:** add schema identity and unknown-version/unknown-field negative cases before Rust writer code. Add the proposed exact successor schemas, closed enum tags, content profile body array, canonical digest-input description, historical disposition table, and V7 replay schema inventory. Do not add production types first.

Schema examples and negatives are schema-only fixtures validated by
`scripts/validate_schemas.py`; they do not claim current Rust/Python DTO
support. The V2/V6 runtime decoders remain current until the later detached
implementation stages.

Replay V7's `content_contract` JSON child must use the exact Spec-defined closed transport object: ContentContractIdV1 plus the canonical ContentContractManifestV1 CBOR payload in bounded canonical padded standard Base64. Keep the manifest a closed typed CBOR contract; do not add a Serde/JSON manifest DTO.

**Implementation work:**

- add the exact ProfiledV1 `basic-land@1.0.0` ID under the unchanged M4.1 grammar and its `basic-land-profile.v1` body contract;
- populate `SemanticContractManifestV1.content_contract_id` with the verified Mountain/Plains `ContentContractIdV1`; keep that immutable identity external to EngineState/state digest/checkpoint fields;
- add proposed type/schema identities from the Spec;
- reserve no extension maps or future enum placeholders;
- mark V5/V6/request-V2/event-V2/PlayerStep-V2 fixtures historical and immutable;
- update doc register and schema register mechanically.

**Focused checks:** `just check-generated`, `.venv/bin/python scripts/check_documentation.py`, `.venv/bin/python scripts/validate_schemas.py`, normative/register validator, `git diff --check`.

**Commit boundary:** one docs/catalog/schema specification commit on the integration branch.

**Stop:** generator source cannot express the exact closed variants; existing schemas conflict with the Spec; any old fixture needs reblessing; ProfiledV1 outer seam would change.

## 5. Phase 2 — successor RED fixtures and canonical vectors

**Goal:** make every successor contract fail on missing/wrong bytes before implementing writers.

**Files likely touched:**

- `persistence/golden/`
- `persistence/negative/` (or the current persistence negative-fixture owner)
- `wire/golden/`
- `wire/negative/`
- `schemas/`
- `crates/mtgml-model/tests/`
- `crates/mtgml-state/tests/`
- `crates/mtgml-persistence/tests/`
- `crates/mtgml-replay/tests/`
- Python schema/parity tests under `python/tests/`

**RED first:** author exact known-answer preimage bytes and expected digest constants for a minimal state with each new field independently mutated, as well as full-state and checkpoint cases. Add negatives for unknown successor tags, malformed V3 profile, bad order, duplicate IDs, overflows, incorrect V3 decision binding, bad checkpoint digest, Replay V7 schema mismatch, and V5/V6 artifacts presented as successor.

**Implementation work:** fixtures and validators only; do not add current writers. Keep predecessor golden bytes unchanged. Any new fixture has its own successor filename and identity.

**Focused tests:** targeted existing persistence/wire/Python fixture validators; generator drift check; schema validation; historical V5/V6 golden comparison.

**Commit boundary:** one RED/fixture commit.

**Stop:** cannot derive bytes from the Spec alone; golden requires implementation output to invent encoding; any predecessor golden changes; Python needs to make a rules decision.

## 6. Phase 3 — detached V6 state DTO and canonical digest

**Goal:** implement the new state representation and digest while leaving current EngineState/V5 writer executable on master.

**Files likely touched:**

- `crates/mtgml-model/src/` for `FullStateDigestV6` and typed identity newtypes
- `crates/mtgml-state/src/digest_v6.rs` (new detached codec owner)
- `crates/mtgml-state/src/persisted_v6/` (new detached state/input types if needed)
- `crates/mtgml-state/src/lib.rs`
- `docs/STATE_HASHING.md`
- KAT/negative fixtures from Phase 2
- Python mechanical digest mirror only in its persistence codec module

**RED first:** mutation-of-every-authoritative-component vectors, insertion-order independence, duplicate/noncanonical order rejection, zero/range boundaries, exact old V5 digest invariance.

**Implementation work:**

- implement exact detached canonical V6 input, including `execution_v3` and the fixed `card-rules-authoritative-state.v1` payload;
- implement closed Mana/TurnHistory/Counter/Attachment/Face/AbilityAuthority encoders and structural validation;
- use only arrays, integer tags, shortest canonical CBOR and declared ordering;
- preserve all V5 codecs/verifiers and never hash Serde output;
- Python may mirror encoding/digest mechanics only, not state/rules validation.

**Focused tests:** `cargo test -p mtgml-model --all-features --locked`; `cargo test -p mtgml-state --all-features --locked`; `cargo test -p mtgml-persistence --all-features --locked`; relevant Python digest/schema tests. Then `just check-fast` on the integration branch.

**Commit boundary:** detached state types; then detached canonical digest/KATs as a separate commit if the compiler/module ownership makes the seam independently reviewable.

**Stop:** current runtime starts producing V6 prematurely; missing exact encoder detail; a new state field has no Spec owner; historical V5 changes; use of allocator/order/debug details leaks into canonical bytes.

## 7. Phase 4 — StateDeltaV2, checkpoint V7 and restore/fork

**Goal:** bind detached V6 identity to a complete replacement and restorable environment without activating it.

**Files likely touched:**

- `crates/mtgml-state/src/delta_v2.rs`
- `crates/mtgml-state/src/engine_state_parts_v2.rs` or detached DTO owner established in Phase 3
- `crates/mtgml-environment/src/checkpoint_v7.rs`
- `crates/mtgml-persistence/src/checkpoint_digest_v7.rs`
- model type exports
- `crates/mtgml-environment/tests/`
- `persistence/golden/`

**RED first:** exact before/after digest apply; invalid replacement rejects; checkpoint construct/validate/restore; corruption in every new state family fails before state exposure; restore then same response yields identical state and digest; fork starts exact.

**Implementation work:**

- implement StateDeltaV2 with V6 digests and complete EngineStatePartsV2 replacement;
- implement exact CheckpointDigestV7 preimage and codec `/7`;
- implement EnvironmentCheckpointV7 detached from current backend aliases;
- validate all structural and execution/content admission relationships before restore;
- do not add automatic V6 checkpoint migration.

**Focused tests:** targeted state, persistence, environment checkpoint tests; historical Checkpoint V6 bytes unchanged; then `just check-fast`.

**Commit boundary:** StateDeltaV2 contract/codec; checkpoint V7/digest/restore as separate review commits.

**Stop:** checkpoint needs uncaptured external state; restore can expose partially validated state; status/counters/execution identity are omitted; old checkpoint reader silently constructs new EngineState.

## 8. Phase 5 — Decision V3 and PlayLand request

**Goal:** add a complete legal land-play candidate while preserving the response contract.

**Files likely touched:**

- `crates/mtgml-decision/src/v3.rs`
- `crates/mtgml-decision/src/ordering.rs`
- `crates/mtgml-decision/src/authoritative.rs` or successor module
- `crates/mtgml-state/src/validation/decision.rs`
- `schemas/player-decision-request.v3.schema.json`
- `wire/golden/`, `wire/negative/`
- `python/src/mtgml/decision_v3.py` or current versioned DTO module
- `python/tests/`

**RED first:** all legal PlayLand candidates are present; wrong candidate binding, duplicate public key, malformed rank/order, stale/fabricated object, wrong zone/type/actor/phase/priority/stack/entitlement reject. DecisionResponseV2 selects a V3 request candidate unchanged. All V2 request goldens remain exact.

**Implementation work:**

- create exact V3 request/intent/binding/candidate structs and CandidateOrderingV2;
- insert PlayLand after pass priority and preserve all other relative intent order as Specified;
- retain DecisionAnswerV2 and DecisionResponseV2 unchanged;
- construct no player-visible trusted binding and no default candidate.

**Focused tests:** decision crate RED + V2 regression; state pending-request validation and Python/schema parity. `just check-fast`.

**Commit boundary:** Rust request/binding/order; then schema/Python/golden parity.

**Stop:** answer protocol needs a new response family; order needs a trusted tiebreaker; response body starts naming schema V3; V2 meanings/fixtures change.

## 9. Phase 6 — ObservedEvent V3, PlayerStep V3 and observation codec

**Goal:** represent newly public state facts without expanding trust exposure, and compose the separately authoritative Decision V3 request through `PlayerStepV3.next_decision`.

**Files likely touched:**

- `crates/mtgml-observation/src/observed_event_v3.rs`
- `crates/mtgml-observation/src/player_step_v3.rs`
- `crates/mtgml-environment/src/observation/magic_basic_land_v1.rs` (or the established codec owner)
- `schemas/observed-event-envelope.v3.schema.json`
- `schemas/player-step.v3.schema.json`
- `schemas/magic-basic-land-observation.v1.schema.json`
- `wire/golden/`, `wire/negative/`
- `python/src/mtgml/` observation/step DTO modules and tests
- `crates/mtgml-conformance/src/isolation/`

**RED first:** event audiences and opaque substitution; public mana/counter/attachment/face projection; hidden-state paired noninterference; trusted-ID renaming invariance; wrong sequence/revision and rejected-step empty-event cases; PlayerStep V3 plus V3 request and V2 information state. Prove the observation payload rejects a `candidates`/decision field under its closed schema, while PlayerStep V3 carries the optional complete request in `next_decision`. Where paired states have equivalent legal-decision semantics, compare the separately projected V3 domain/candidate bytes; do not include candidate equality in the observation-payload noninterference claim.

**Implementation work:**

- implement only the Spec event union and exact public values;
- retain exactly one V3 `object_moved` wire tag with the specified V3 payload (superseding the V2 shape); include face/tapped on entry and never emit a fake transform on Ojer-like entry;
- use OpaqueObjectId for all object references;
- implement strict named observation payload schema and canonical array orders;
- keep `magic-basic-land-observation.v1` limited to public state facts; do not duplicate decision domain, CandidateId, CandidateIntent, or candidate arrays there;
- compose only an already supplied and validated `PlayerDecisionRequestV3` as `PlayerStepV3.next_decision`; legal candidate generation/completeness remains a RulesKernel/content producer obligation in Phase 10;
- preserve ObservationEnvelopeV1 and InformationStateDigestV2 identities.

**Focused tests:** observation, information, conformance isolation, schema/Python parity, and a closed-payload negative proving candidate/decision fields are rejected; `just check-fast`.

**Commit boundary:** event/PlayerStep DTOs; then payload projection/privacy/schema parity.

**Stop:** a trusted ID appears in any public output; information digest needs reinterpretation; event duplicate loses/duplicates a semantic occurrence; paired-state public bytes differ without authorized cause; observation and `next_decision` duplicate or disagree on candidate authority; implementation attempts to generate or infer legal candidates in the observation layer.

## 10. Phase 7 — Replay V7 detached support

**Goal:** bind successor inputs/state/checkpoint identities end-to-end without changing the current Replay V6 runtime.

**Files likely touched:**

- `crates/mtgml-replay/src/v7.rs`
- `crates/mtgml-replay/src/recorder_v7.rs`
- `crates/mtgml-environment/src/replay_v7.rs`
- `schemas/replay-manifest.v7.schema.json`
- `schemas/authoritative-replay.v7.schema.json`
- `wire/golden/`, `wire/negative/`
- Python replay version module/tests

**RED first:** detached manifest identity validation; strict content-child schema and wire codec vectors for exact canonical padded Base64; reject bad alphabet/padding, nonzero pad bits, whitespace, over-limit text, invalid/noncanonical CBOR, wrong child digest, mismatch with semantic manifest, and null/present child mismatch; replay from V7 checkpoint with one V2 response per step; direct/replay digest/event/delta/next-request parity; malformed links, counters, schema IDs and final identity reject; complete replay catches a changed state field.

**Implementation work:**

- add exact V7 manifest/step/file/recorder/schema-inventory/initial identity;
- use FullStateDigestV6 and CheckpointDigestV7 only;
- add `SemanticContractMaterialV7` with the semantic manifest, rules manifest, and `ContentContractMaterialV1` child exactly when `content_contract_id` is non-null; recompute and validate child and parent IDs plus equality with all `ExecutionIdentityV1` references;
- encode the child as the exact Spec-defined JSON object (`content_contract_id`, `manifest_canonical_cbor_base64`); strict-decode the bounded standard Base64 to the existing canonical CBOR manifest decoder and require exact re-encoding. Do not derive a new JSON schema for the typed manifest;
- require detached replay validation to verify the canonical content manifest against its `ContentContractIdV1`; require restore/runtime admission to match the supplied verified catalog to the replay/checkpoint's semantic child before executable state is exposed;
- keep Replay V6 detached historical verifier intact;
- do not store observed event output as a second replay authority.

**Focused tests:** replay crate + environment replay tests; old Replay V6 goldens/fixtures unchanged; `just check-fast`.

**Commit boundary:** V7 Rust DTO/validation; then execution/recorder/Python parity.

**Stop:** replay omits an identity already present in V6; deterministic control data is implicit; replay trusts event logs rather than re-execution; historical fixture is changed.

## 11. Phase 8 — semantic family constructors and validation

**Goal:** implement exact state-local invariants and reusable mutation primitives while still detached from current master runtime.

**Files likely touched:**

- `crates/mtgml-state/src/mana.rs`
- `crates/mtgml-state/src/turn_history.rs`
- `crates/mtgml-state/src/counters.rs`
- `crates/mtgml-state/src/attachments.rs`
- `crates/mtgml-state/src/faces.rs`
- `crates/mtgml-state/src/ability_authority.rs`
- `crates/mtgml-state/src/validation/`
- conformance/red fixture support

**RED first:** every Spec family invariant, overflow/zero/stale reference, target occurrence control-change sequence, Role timestamp/uniqueness, Ojer direct-entry face/tapped fact, registry active referent and privacy-safe projection. Ability authority has its own focused sequence: (1) ability-key/source state record plus verified ExecutionIdentity content join, (2) source zone/face/profile existence and stale-incarnation validation, (3) per-perspective opaque-ID allocation sorted by `(source OpaqueObjectId, profile-defined visible ability ordinal)`, and (4) candidate generation and checkpoint/fork/replay parity.

**Implementation work:**

- implement typed records, checked helpers, canonical forms and structural/cross-family validation;
- model the two-stage validation boundary: state-local validation and environment admission join against content/execution identity;
- allocate AbilityInstanceId only through existing allocator in canonical trusted `(source GameObjectId, AbilityKey)` order; allocate OpaqueAbilityId independently in public-key order and prove global AbilityInstanceId renaming/hidden allocation differences cannot change player-facing identities;
- validate registry rows in any zone where the bound profile declares the ability identity exists; keep current-zone activation legality in profile/rules candidate generation;
- no new allocator, extensions or source-derived mutable state;
- leave general effect/trigger/stack/setup state absent.

**Focused tests:** state/conformance tests that directly construct detached V6 records. `just check-fast`.

**Commit boundary:** state type/validation; then mutation primitives/RED evidence.

**Stop:** state facts need string keys, uncontrolled timestamp, new allocator, callbacks, history reconstruction or unsupported counter/face types.

## 12. Phase 9 — Profiled CardDefinition and reusable capability requirements

**Goal:** admit the exact Mountain/Plains profile structurally and derive its complete bounded requirements.

**Files likely touched:**

- `crates/mtgml-card-ir/src/lib.rs`
- `crates/mtgml-card-ir/src/preflight.rs`
- capability owner specs and `cards/capabilities/registry.json`
- `crates/mtgml-card-ir/src/generated_capability_registry.json` only by generation
- `cards/definitions/` or the repository's current content directory, only after source provenance record bytes are pinned
- profile schema/codec and content golden tests

**RED first:** exact valid Mountain/Plains catalog/profile bodies; unknown/malformed body; mismatch with type line/FaceKey/AbilityKey; author-suppressed requirements; content digest semantic mutation; provenance snapshot/record/source digest verification.

**Implementation work:**

- add only `basic-land@1.0.0` under the unchanged ID grammar and its typed Mountain/Plains body;
- construct the executable `SemanticContractManifestV1` with the verified Mountain/Plains `ContentContractIdV1`, and verify it is carried through `SemanticContractIdV1` and `ExecutionIdentityV1` into checkpoint/replay identity;
- validate FaceKey 0 and local AbilityKey 0;
- derive `rules/land-play@0.1.0`, `rules/basic-land-mana@0.1.0`, `rules/mana-pool@0.1.0` plus recursive existing requirements from Spec;
- add missing capability definitions as specified/implemented lifecycle only through their normal owner docs and RED obligations; do not claim covered/certified;
- use exact Oracle bulk source record bytes, UUID, codec identity and computed digest; do not use the later live Scryfall response;
- keep preflight fail-closed for execution until atomic final integration.

**Focused tests:** Card IR/content/prefight tests and generated registry/schema drift checks; `just check-fast`.

**Commit boundary:** profile contract/validator; then content records and recursive requirements.

**Stop:** outer CardDefinition envelope or ContentContractId identity needs change; pinned snapshot record bytes cannot be recovered/verified; name selects a handler; requirement closure misses a root.

## 13. Phase 10 — RulesKernel transition producers and M4.2 vertical slice

**Goal:** integrate the completed successor state family, V3 requests and Mountain/Plains profile on the non-current integration branch only after every detached predecessor is ready. This phase does not activate successors on master.

**Files likely touched:**

- `crates/mtgml-rules/src/magic.rs`
- `crates/mtgml-rules/src/transition.rs`
- `crates/mtgml-rules/src/contract.rs`
- `crates/mtgml-rules/src/events.rs`
- `crates/mtgml-state/src/engine.rs`
- `crates/mtgml-state/src/delta.rs`
- `crates/mtgml-environment/src/` current kernel/controller/commit path
- `crates/mtgml-environment/tests/` focused RED tests

**RED first:** `MagicActionRequestV1` can only be formed from the exact selected pending candidate; legal land all-and-only; Hand→Battlefield incarnation; source ability ID authority; immediate mana resolution/tap; step/phase emptying; event/delta/state exactness; atomic rejection. Use positive Mountain and Plains cases and adversarial type/name/profile cases.

**Implementation work:**

- integrate `ManaState`, `TurnHistoryState`, `CounterState`, `AttachmentState`, `FaceState`, `AbilityAuthorityState` into successor EngineState on the integration branch;
- bind the immutable content catalog through SemanticContractManifestV1 and verify its Replay V7 child; do not duplicate ContentContractId in EngineState or per-ability rows;
- keep current master/runtime aliases on predecessor identities until the separate Phase 13 merge boundary;
- route the validated PlayLand and ActivateAbility bindings through the closed internal request and MagicRulesKernel;
- derive intrinsic mana ability from subtype/profile, never name;
- keep mana payment/casting and non-basic abilities fail-closed;
- build full transition product and validate all per-player projections before atomic commit.

**Focused tests:** `cargo test -p mtgml-rules --all-features --locked`; `cargo test -p mtgml-state --all-features --locked`; `cargo test -p mtgml-environment --all-features --locked`; then `just check-fast` and `just check`.

**Commit boundary:** integrated successor-runtime PR(s) target only the non-current integration branch. No commit in this phase activates the successor on master.

**Stop:** any direct state write outside RulesKernel; partial commit; inability to validate full candidate set; an incomplete successor becomes the current writer; card-specific name switch; choice or source selected automatically.

## 14. Phase 11 — checkpoint/fork/replay/privacy/conformance closure

**Goal:** prove successor trajectory parity and information safety across all execution modes.

**Files likely touched:**

- `crates/mtgml-conformance/src/`
- `crates/mtgml-environment/tests/`
- `crates/mtgml-replay/tests/`
- `python/tests/`
- `wire/golden/`, `persistence/golden/`
- conformance catalogue/fixtures under existing ownership

**RED first:** the complete Spec RED matrix, paired-state noninterference, restore at just-before-land and just-before-mana ability boundaries, fork and same response, full replay, stale/fabricated candidate, hidden-world candidate/event parity, exact Ojer state constructor semantics, and all historical regression vectors.

**Implementation work:**

- execute all RED and conformance vectors;
- validate expected events and StateDelta independently of final board shape;
- add no new semantics discovered only by these tests without returning to Spec review.

**Focused tests:** full relevant Rust/Python/schema/conformance sets; `just check-all` after integration; `just release-candidate` separately; `just archive-check` after last source change.

**Commit boundary:** focused conformance/privacy/replay evidence commit(s), still on integration branch until complete.

**Stop:** any direct/replay/restore divergence, hidden ID exposure, incorrect projection, changed old golden, unsupported branch reaches a legal candidate, or required gate unavailable/failing.

## 15. Phase 12 — cross-language, archive and repository gates

**Goal:** satisfy all exact current repository gates before a PR is review-ready.

**Commands:**

```text
just check-fast
just check
just check-all
just release-candidate
just archive-check
.venv/bin/python scripts/check_documentation.py
.venv/bin/python scripts/validate_schemas.py
.venv/bin/python scripts/generate_contracts.py --check
.venv/bin/python scripts/generate_semantic_contract_catalog.py --check
git diff --check
```

The plan follows the actual Justfile: `just check-all` is the certification test profile; `just release-candidate` is a separate verification report; `just archive-check` is the source-archive reproducibility gate. Do not substitute one for another. Verification must not mutate the source being verified; run archive check after the final source change.

**Hosted CI:** on the exact implementation PR head, require current mandatory jobs `PR Fast`, `PR Integration`, and aggregate `manafold-pr-gate`; run/require platform smoke and CodeQL where current branch/workflow triggers require them. Verify job names from live workflows/PR checks at implementation time, not from this planning text alone.

**Independent acceptance:** keep separate results for `IMPLEMENTATION_PASS`, `HOSTED_CI_PASS`, `CODE_REVIEW_PASS`, `FINAL_ACCEPTANCE_PASS`. Code review and final acceptance must inspect the exact final diff/head. Green local or hosted tests alone do not accept the slice.

**Commit boundary:** no source changes after final archive gate. Any fix reruns affected gates and the final archive/repro gate.

**Stop:** any gate not executed successfully is `NOT_RUN` or `BLOCKED`; no acceptance status is upgraded.

## 16. Phase 13 — sole master activation and post-merge verification

**Goal:** move one complete, reviewed successor family to current runtime authority.

**Actions:**

1. Confirm all detached successor DTOs and producers are exact Spec behavior; current V5/V6 historical verifiers are unchanged.
2. Confirm current runtime switches all coupled families together: EngineState, FullStateDigestV6, StateDeltaV2, checkpoint V7/digest V7, Replay V7 with verified content child, Decision request V3, ObservedEvent V3, PlayerStep V3, Magic observation codec, and admission for only `basic-land@1.0.0`.
3. Require exact-head hosted CI and independent code review; submit the complete integrated successor PR to master only after phases 1–12 are accepted. This is the only activation/merge boundary.
4. Merge only through accepted repository process; fetch origin and verify the actual post-merge exact master, run required post-merge verification and record final acceptance evidence.
5. Update #225 without closing it. M4.2 remains IN PROGRESS and incomplete until independent implementation acceptance of the real card slice. Implementing the substrate alone does not establish M4.2 completion or broader card support.

**RED first:** an activation contract gate asserts all successor schema identities agree and no old writer/type is current for new state; old-family reader/disposition matrix remains exact.

**Stop:** partially current version family; any automatic migration/relabel; exact-head differs from reviewed head; CI/review/final acceptance incomplete.

## 17. Historical compatibility gates

At every stage preserve byte-exact historical evidence for:

```text
FullStateDigestV5
EnvironmentCheckpointV6 / CheckpointDigestV6
Replay V6
Decision request V2 / DecisionResponseV2
ObservedEventEnvelopeV2
PlayerStepV2
ContentContractV1 UnprofiledV1
```

V5 full digest and checkpoint/replay predecessor readers keep the Spec disposition. No historical bytes are regenerated from current runtime. If any predecessor golden changes, stop and locate the first divergence before proceeding. No “bless all” operation is permitted.

## 18. Out of scope

This plan does not authorize Lightning Strike, casting/payment, general stack execution, mana-source selection for payment, arbitrary mana restrictions, role/Aura execution, counters on selected cards, Ojer card support, triggers, linked exile, general temporary effects, setup/mulligan, full R1/W1, cross-deck closure, playable-game certification, ZERO-UNSUPPORTED, bundle freeze or M5 trajectories. State-family presence and RED substrate tests are not certification or card support.

## 19. Expected review/commit decomposition

Target six independently reviewable implementation PRs on the dedicated integration branch, followed by one final PR that alone activates the integrated successor on master:

1. successor normative schemas, catalog identities and RED/golden fixtures;
2. detached state DTOs and canonical FullStateDigestV6;
3. StateDeltaV2 plus checkpoint/digest V7;
4. Decision V3, ObservedEvent/PlayerStep V3, payload codec and Python/schema parity;
5. Replay V7 plus state-family validation/conformance support;
6. content profile, content-child replay verification, reusable requirements and RulesKernel producer integration on the non-current integration branch;
7. final master-activation PR containing the full coherent successor runtime, exact-head gates and all accepted review fixes.

If repository PR policy cannot support stacked PRs, keep semantic commits on one integration branch and open one final master PR after all component reviews. PR count never justifies exposing an incomplete successor as current authority.
