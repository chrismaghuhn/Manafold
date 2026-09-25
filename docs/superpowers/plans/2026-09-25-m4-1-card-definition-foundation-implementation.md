# M4.1 CardDefinition / Card IR Foundation — Implementation Plan

- **Status:** candidate for independent plan review
- **Task:** `M4.1_IMPLEMENTATION_PLAN`
- **Accepted Spec SHA:** `696c308b4ca5c46c226b62dc55921632a70dac19`
- **Plan baseline master:** `49165aeeb68858580e4f7bc0286d0afa227cae23`
- **Preparation branch:** `codex/m4-1-spec-20260925`
- **Prepared:** 2026-09-25

This plan is derived from the accepted M4.1 Spec at the exact SHA above and
the verified `origin/master` baseline above. At plan preparation, a fresh
fetch confirmed that `origin/master` is still the stated baseline and that the
accepted Spec commit is the exact remote head of the preparation branch. If
master advances before implementation, the implementation owner must compare
the new master against this plan and the accepted Spec, document the drift,
and obtain review of any changed contract assumptions before editing. Do not
force the pinned baseline or silently regenerate content identity.

This document is an implementation sequence, not a new semantic authority.
The normative contract surfaces listed below must own the persistent and
wire-facing parts of the implementation. The accepted process Spec does not
supersede them.

---

## 1. Objective and hard boundary

Implement the immutable CardDefinition/content foundation specified in
[`2026-09-25-m4-1-card-definition-foundation-spec.md`](../specs/2026-09-25-m4-1-card-definition-foundation-spec.md): typed definition values,
content-scoped identity, provenance, local identities, bounded printed
characteristics, deterministic definition closure, mandatory/additive
requirement roots, structural validation, and a non-authorizing
`ContentValidationOnly` preflight.

M4.1 must finish before executable semantics begin:

```text
M4.1 = immutable content definition, identity, validation, and preflight
M4.2 = first admitted executable profile and first real selected content
```

Every gameplay-construction attempt in M4.1 remains rejected with
`NoExecutableProfileAdmitted`. A structurally valid definition is not
supported, a covered capability is not a certified card, and a certified
card is not a certified bundle.

## 2. Authority and entry gate

Implementation may begin only after independent acceptance of this exact
plan. The M4.1 Spec SHA above remains pinned as the semantic authority.

Before creating an implementation worktree or changing production files:

1. Fetch `origin`; record exact `origin/master`, local `HEAD`, branch, and
   worktree status. Reconcile any master drift against both the accepted Spec
   and this plan.
2. Verify M3 remains accepted and M4 remains unblocked from current status
   artifacts.
3. Verify M4.0 PR #223 is merged at the pinned baseline and inspect actual
   post-merge exact-master gate evidence. The merge alone is not evidence.
   If the required gate is absent, blocked, or failing, stop M4.1 production
   implementation until the prerequisite is resolved or explicitly
   reauthorized by the project owner.
4. Confirm #221 and #222 remain open/current owners for M4.1 and M4 tracking;
   do not change issue or milestone status as part of implementation.
5. Confirm this plan and accepted Spec are the reviewed exact commits. The
   implementation branch/worktree must be dedicated and based on the accepted
   current master while retaining the accepted Spec and Plan commits (the
   current review branch is a docs-only descendant of the pinned master).
   Preserve other worktrees and unrelated local work.

The CR snapshot recorded in the accepted Spec is the only Magic-rule source
needed for this foundation. Do not add normative Magic rulings or card
semantics while implementing the content envelope.

## 3. Baseline inspection and ownership map

At the pinned baseline, the following current surfaces were inspected:

| Concern | Existing authority/surface | Planned treatment |
| --- | --- | --- |
| Card IR | `crates/mtgml-card-ir/src/lib.rs`; `docs/CARD_IR.md` | Replace/quarantine the executable-looking experimental scaffold only as specified; keep the stable definition foundation separate from all executable profile languages. |
| Definition ID | `mtgml_model::CardDefinitionId` in `crates/mtgml-model/src/lib.rs` | Retain this typed u64 ID; add no process-global meaning or ID-family substitution. |
| Content ID | Reserved `ContentContractIdV1` in `crates/mtgml-model/src/lib.rs` | Add only verified content-manifest construction through the designated digest owner; do not treat arbitrary digest bytes as a verified catalog. |
| Digest and canonical CBOR | `crates/mtgml-persistence/src/{cbor.rs,envelope.rs,semantic_contract_digest.rs}`; `docs/STATE_HASHING.md`; ADR 0038 / ADR 0055 | Reuse the existing restricted codec and digest-envelope framing; add the content-manifest schema/domain and vectors without modifying state/checkpoint/replay identities. |
| Capability source | `cards/capabilities/registry.json`; `docs/cards/CAPABILITY_MODEL.md` | Keep this as the single capability/lifecycle/support source of truth. Any compiled/runtime projection must be generated or loaded from this source and must have a drift check. |
| Capability closure | `scripts/maintainer_common.py::capability_census` and `scripts/capability_census.py` | Reuse or extend the existing registry validation/closure operation. Do not create a Card-IR support registry or a second authored dependency/lifecycle catalog. |
| Existing card manifest schema | `schemas/card-definition-manifest.v1.schema.json`; `scripts/maintainer_common.py::validate_card_manifest` | This is maintainer lifecycle/provenance metadata. Do not mistake it for, rename it into, or hash it as the production `CardDefinitionEnvelopeV1`. |
| Semantic execution catalog | `contracts/catalog/semantic-contracts.v1.json` and its generator | Preserve current entries, IDs, generated files, and M3 capability closures byte-for-byte unless a separate accepted review authorizes otherwise. M4.1 content identity is not added to execution identity. |
| State and runtime IDs | `docs/DOMAIN_MODEL.md`, `docs/contracts/ENGINE_STATE_CLOSURE.md`, `crates/mtgml-state` | Add no general runtime spell/ability state and no new authoritative `EngineState` family. |
| Player decisions/information | `docs/DECISION_PROTOCOL.md`, `docs/INFORMATION_MODEL.md`, player DTOs | No new Decision family, player metadata, trusted-ID exposure, or projection changes. |
| Persistence versions | `FullStateDigestV5`, `EnvironmentCheckpointV6`, `CheckpointDigestV6`, Replay V6 | Preserve their exact current meaning, bytes, schemas, fixtures, and producers. |

The plan's implementation may add generated/read-only adapters around the
existing registry source where a typed preflight needs them. Such an adapter
must contain no independently authored capability keys, versions,
dependencies, or lifecycle claims. Its drift check must compare against
`cards/capabilities/registry.json`; a stale or invalid projection rejects the
gate.

## 4. Normative and executable contract surfaces

Before or alongside the Rust implementation, land the durable contract in
reviewed repository surfaces rather than leaving it only in the provisional
process Spec:

1. Add `docs/contracts/CARD_DEFINITION_CONTRACT.md` as the normative
   owner for the stable envelope, local identities, closed characteristic
   types, provenance separation, profile-body seam, reference relation,
   structural validation, and non-authorizing preflight. Register it as
   `role=normative`, with the repository's accepted stability/change-process
   conventions. Link it from `docs/CARD_IR.md`.
2. Keep `docs/CARD_IR.md` explicit that executable Card-IR vocabulary remains
   experimental. Its status must not imply that `ExperimentalEffect` is a
   stable profile or that M4.1 executes the IR.
3. Add the complete `ContentContractManifestV1` digest preimage and
   `ProvenanceCatalogV1` encoding to `docs/STATE_HASHING.md` as a normative
   content-identity section. It must match the accepted Spec's byte layout
   and use the existing ADR 0038 / ADR 0055 digest envelope. Do not modify
   existing FullStateDigest, checkpoint, or replay schemas/fixtures.
4. Update `docs/DOMAIN_MODEL.md` only to link/clarify the content-scoped
   identity tuple and preserve distinctions from runtime and player-visible
   identities. Do not add definition data to `EngineState`.
5. If implementation exposes a JSON interchange format for content
   manifests, add a separately named, closed schema such as
   `schemas/content-contract-manifest.v1.schema.json`, register it in the
   schema inventory and validators, and explicitly keep it distinct from
   `card-definition-manifest.v1`. Its fields/closedness must match the Rust
   decoder and normative contract. Do not create a generic extension field.
   If no JSON interchange is exposed, do not add an unused JSON schema.
6. Add content-identity known-answer and negative fixtures under the existing
   `persistence/golden/` and fixture conventions. The CBOR preimage remains
   the identity authority; JSON formatting is never digest input.

Any persistent content or wire identity contract discovered to lack an owner
must stop at this stage. Do not treat this provisional Plan or the accepted
process Spec as an executable substitute for a normative contract or schema.

## 5. Proposed implementation artifacts

The expected file set is limited to the following focused areas; the
implementation branch must confirm paths against the current post-entry-gate
tree before edits.

### Production Rust

- `crates/mtgml-card-ir/src/lib.rs` and focused child modules for the closed
  envelope/characteristic/profile-binding types, typed validation errors,
  deterministic closure, requirement-root derivation, immutable content
  catalog, and `ContentValidationOnly` result.
- `crates/mtgml-card-ir/Cargo.toml` only for direct dependencies actually
  needed by those closed values; do not add a scripting, plugin, or second
  rules-engine dependency.
- `crates/mtgml-model/src/lib.rs` only if a content ID or new local key type
  must live at the shared identity layer. Reuse `CardDefinitionId` and the
  reserved `ContentContractIdV1`; keep ID constructors/verification scoped to
  the content digest boundary.
- `crates/mtgml-persistence/src/lib.rs`, `cbor.rs`, `envelope.rs`, and a
  focused `content_contract_digest.rs` (or equivalent module) to encode the
  exact accepted payload through the existing codec/envelope. Reuse codec
  primitives; do not hand-roll another CBOR encoder or alter state digest
  calculations.
- Generated registry projection only if required for Rust preflight; its
  source must remain `cards/capabilities/registry.json`, with a single
  generator and byte-exact `--check` gate. Do not add a separate authored
  registry in Card IR.

### Conformance and content tooling

- Focused unit/integration tests under `crates/mtgml-card-ir` and
  `crates/mtgml-persistence/tests/` for the exact RED matrix below.
- Content digest KAT/negative fixtures under `persistence/golden/` with
  manifest membership updated through the existing fixture inventory.
- Reuse/extend `scripts/maintainer_common.py` and
  `scripts/capability_census.py` only where needed to route derived and
  explicit roots through the canonical Capability Registry closure. The
  pre-existing bundle certification path and its meanings must remain
  unchanged.
- `schemas/README.json`, schema validators, fixture indexes, or generator
  inventories only when a new production interchange artifact is actually
  added, as described in §4.

### Normative and navigation documentation

- `docs/contracts/CARD_DEFINITION_CONTRACT.md` (new normative contract).
- `docs/CARD_IR.md`, `docs/STATE_HASHING.md`, and narrowly scoped
  identity/reference additions to `docs/DOMAIN_MODEL.md`.
- `docs/normative-document-register.v1.json` and relevant schema/golden
  inventories for mechanical registration only.
- No issue body, milestone status, unrelated roadmap, or broad architecture
  rewrite.

## 6. RED-first task sequence

The implementation PR is one cohesive M4.1 slice. Keep RED tests and their
fixes in reviewable commits/steps, but do not split one invariant across
unreviewed source commits that leave the branch falsely appearing complete.

### Task 0 — Reconfirm authority and M4.0 prerequisite

- Fetch `origin`; record exact master and confirm accepted plan/spec SHA.
- Check exact-master post-merge M4.0 evidence required by #221.
- Inspect current Rust, generators, schemas, fixture inventories, and docs for
  drift since the plan baseline.
- If a changed authority affects any promised type, digest bytes, or gate,
  stop and reconcile with maintainers before implementation.

### Task 1 — Contract surfaces and decoder/identity RED

Add the normative content-definition contract and golden/negative fixture
shapes before the producer implementation. Add failing evidence for:

- exact empty/minimal valid `UnprofiledV1` definition and manifest;
- wrong top-level/nested array arity, unknown field/variant, unknown profile,
  malformed UTF-8, noncanonical integer/array/order, duplicate fields/keys,
  invalid local references, unsupported symbol, out-of-range scalar, trailing
  data, and strict re-encode mismatch;
- canonical content-envelope preimage, exact SHA-256 known-answer vector,
  domain/schema/codec identity, and digest mismatch;
- mutation of every rule-relevant envelope field changes the content
  manifest bytes/identity; provenance-only/lowering-audit changes do not;
- duplicate `CardDefinitionId` in one manifest rejects whether equal or
  conflicting; one `(ContentContractIdV1, CardDefinitionId)` cannot resolve
  to more than one canonical immutable definition;
- profiled production content rejects before content identity minting;
  test-only profile-body contrast compares canonical manifest bytes only and
  must not call production ID minting.

Keep golden JSON/CBOR fixture conventions and identity inventories in sync.
The fixture must cover the accepted Hybrid symbol encodings in both printed
orders and repeated symbols, without testing payment behavior.

### Task 2 — Definition model, provenance, local identities, and validation

Implement the exact accepted closed model. Structural validation must prove:

- nonempty face list; contiguous canonical face ordinals; uniqueness of
  `FaceKey` and `AbilityKey`; every ability face binding resolves locally;
- closed profile-binding variant and canonical `CardSemanticProfileId`
  grammar; M4.1 production accepts only `UnprofiledV1`;
- strict characteristic text/numeric/set rules and exact
  `PrintedManaSymbolV1` closed variants, including ordered two-color hybrid;
- references use only the specified relation, resolve only within the
  verified content contract, and reject duplicate edges, invalid local face
  references, missing targets, mismatched identities, and cross-catalog
  fallback;
- provenance has exactly one record per content definition, no missing,
  duplicate, or extra records, and does not affect content ID bytes;
- no runtime identity, controller/zone, target, Decision/Candidate, RNG,
  continuation, event, or timestamp fields can be represented in the closed
  definition DTO.

Test constructors and validation using synthetic fixtures only. Do not add
any of the 26 audited R1×W1 names as production content or test evidence of
support.

### Task 3 — Content identity and immutable catalog

Implement the content digest adapter in the existing persistence identity
owner using the exact fixed positional CBOR schema and the existing digest
envelope. Construct the immutable catalog only after canonical decode,
structural validation, digest recomputation/equality, and provenance
validation all succeed.

Evidence:

- same manifest input yields exact same ID across fresh processes and
  shuffled catalog construction;
- all semantically ordered sequences preserve order; all set-like sequences
  enforce their declared order; duplicate IDs/keys reject;
- source/provenance changes are absent from content hash, while every
  rule-relevant definition mutation changes preimage bytes and the KAT digest;
- canonical decode then re-encode yields exact byte equality;
- arbitrary caller-supplied `ContentContractIdV1` cannot authorize a catalog
  until the digest verifies;
- references are looked up by `(verified_content_id, definition_id)` only.

No content contract identity is added to `FullStateDigestV5` or execution
identity in this task.

### Task 4 — Recursive definition closure

Implement deterministic closure over the one verified immutable catalog.
Traverse roots and outgoing references in the canonical order declared by
the Spec; return the unique reachable IDs in numeric sorted order and the
canonical active-path cycle diagnostic specified there.

RED/evidence matrix:

- root with no edges;
- one-level and multi-level chains;
- repeated root requests and shared descendants;
- missing target and target present only in another content contract;
- duplicate/conflicting definition identity and duplicate outgoing edge;
- self-cycle and multi-node cycle with exact canonical path;
- malformed relation and invalid target face;
- shuffled catalog insertion and reference input ordering produce identical
  success/error results and closure ordering.

No cross-content generated/token/copy relation semantics are introduced.

### Task 5 — Requirement roots and canonical Capability Registry closure

Implement deterministic structural/profile-derived roots plus additive
explicit roots. M4.1 has no admitted production semantic profile, so no
production profile catalog or executable descriptor is added. The only
profile descriptor used to prove non-suppressible derived roots is a closed
test-only fixture unavailable to production construction.

Route normalized `CapabilityRequirementV1 { key, version }` roots through the
existing Capability Registry and closure contract. Preserve exact key/version
matching, transitive registered dependency versions, lifecycle threshold,
missing-key/dependency, cycle, malformed-registry, and deterministic order
behavior. If the Rust content preflight requires a registry projection,
generate it from the existing `cards/capabilities/registry.json` source and
test generator drift; do not author parallel dependency/lifecycle data in
Card IR. Compare the Rust preflight closure result against the established
registry census on the same fixtures.

RED/evidence matrix:

- deterministic derived roots and sorting;
- explicit roots add only; they cannot remove/replace/version-substitute a
  derived root;
- omission of an explicit declaration cannot suppress a known structural
  derived root;
- duplicate root pair normalizes only where the Spec allows; two versions
  for one key reject;
- unknown capability and unknown/mismatched version reject;
- missing dependency and cycle reject with canonical path;
- requested lifecycle threshold is explicit and a below-threshold entry
  rejects;
- registry changes flow from the one canonical source and stale generated
  outputs fail the drift gate.

Capability keys remain metadata. No Card IR opcode, dispatcher, support
registry, lifecycle field, or certification result is added.

### Task 6 — ContentValidationOnly preflight

Compose strict decoding, structural/local validation, content identity,
provenance, recursive definition closure, requirement-root derivation,
existing-registry closure, and requested lifecycle checks in the exact order
of the accepted Spec. Return a typed internal report with deterministic
stage/error identity and canonical path data.

Prove fail-closed behavior for each listed Spec error class, including
unknown/unreviewed profile, invalid profile/body, unknown characteristic or
reference variant, missing/conflicting definition, bad provenance,
identity mismatch, invalid/cyclic definition closure, unknown capability,
dependency failure, below-threshold lifecycle, and invalid registry
projection.

Prove gameplay construction remains unconditionally rejected with
`NoExecutableProfileAdmitted`, regardless of successful content validation
or capability lifecycle. There is no partial `GameplayAdmission` request,
SupportProfile catalog, profile×support-policy comparison, or test-only
production escape hatch.

### Task 7 — Query purity, determinism, privacy, and non-regression

Add focused property/conformance checks that validation, requirement
derivation, closure, digest encoding, and preflight do not consult RNG,
wall-clock time, allocators, pointer identity, randomized/hash iteration,
locale, filesystem order, process-global state, or unstable debug rendering.
Discarding all caches and recomputing from identical values yields identical
semantic results.

Inspect exact DTO and persistence diffs to prove:

- no new authoritative EngineState member/family, runtime instance, event,
  transition, StateDelta write, or continuation payload;
- no new Decision/choice/candidate or fallback behavior;
- no player DTO, observation, retained information, or public diagnostic
  exposes content IDs, provenance, capability/preflight or debug metadata;
- trusted IDs remain separate from player-visible opaque IDs;
- no FullStateDigestV5, Checkpoint V6 / CheckpointDigestV6, Replay V6,
  semantic execution catalog, current M3 identity, or existing persisted
  fixture meaning changes.

This is regression evidence for the M4.1 slice, not completion of cumulative
M4.8–M4.10 closure audits.

### Task 8 — ExperimentalEffect disposition

Do not promote or rename `ExperimentalEffect`. Retire the execution scaffold
and `CARD_IR_STABILITY` declaration from production if it has no consumers;
remove any internal consumers only where removal is required to leave no
stable/executable meaning in M4.1. If removal would require a consumer,
state, persistence, semantic catalog, or behavior change outside this Spec,
stop and report the contradiction. Do not replace it with another executable
language. Update the normative Card IR/documentation surface so remaining
experimental examples are clearly non-production and confer no compatibility
entitlement.

### Task 9 — Cumulative focused and repository verification

After all source edits and fixture generation:

1. Run exact generator and drift checks for every changed generated source.
2. Run direct Card IR and persistence Rust tests, including all new RED
   regressions and existing relevant modules.
3. Run repository schema validation, fixture/golden validation, repository
   integrity, Rust source structure, documentation/register/link validation,
   and maintainer-artifact checks that own any changed representations.
4. Run `just check-fast`.
5. Run `just check`.
6. Run `just check-all` because this slice adds persistent identity and
   cross-crate content contracts.
7. Run `just release-candidate` as its own required gate. `check-all` does
   not substitute for this command. The release-candidate result must contain
   no `NOT_RUN` or `FAIL`. If any required gate is technically blocked or
   unavailable, report it as `BLOCKED` or `NOT_RUN`; no narrower or
   substitute suite may be relabeled as its PASS.
8. After all source and fixture changes, run the repository archive and
   reproducibility check. Make no further source changes afterward except an
   implementation evidence record explicitly excluded from the archive
   contract.
9. Run `git diff --check` and confirm the committed source worktree is clean.
10. Require hosted `PR Fast`, `PR Integration`, and stable aggregate
    `manafold-pr-gate` on the exact implementation head before accepting the
    implementation.

Do not run soak, performance, search, or gameplay benchmarks merely to
inflate evidence. M4.1 adds no playable game.

## 7. Test matrix ownership

| Risk | Primary owner | Required evidence |
| --- | --- | --- |
| Closed schema and scalar validation | `mtgml-card-ir` | Positive minimal definition; negative unknown/duplicate/range/order/reference/profile cases. |
| Content identity | `mtgml-persistence` + `mtgml-card-ir` | Exact preimage/digest KAT, mutation matrix, provenance separation, duplicate/conflict rejection. |
| Canonical CBOR parity | `mtgml-persistence` | Existing codec suite plus content-specific fixed-array, hybrid variant, malformed and re-encode-equality fixtures. |
| Definition graph | `mtgml-card-ir` | Chain, fan-in, missing, conflict, invalid edge, cycles, canonical deterministic outputs. |
| Capability closure | canonical registry source + existing closure owner | Derived/additive roots, exact version/dependency closure, lifecycle and cycle failures, generated drift/parity if Rust projection is needed. |
| Preflight | `mtgml-card-ir` and registry closure seam | Every fail-closed error stage; valid content never authorizes gameplay. |
| Privacy and Decision stability | existing observation/decision conformance owners | Exact diff census plus focused no-new-field/family regression checks. |
| State/replay identity | model, persistence, environment, replay owners | Existing V5/V6 KATs and exact schemas unchanged; no new state identity. |
| Docs/schema consistency | docs/schema validator owners | normative document register, local links, JSON schema and fixture inventories pass. |

No test may use real R1/W1 cards as supported content. The 26-name
characteristic audit in the accepted Spec is schema-fit input only.

## 8. Migration and versioning decisions

- Add `CardDefinitionEnvelopeV1`, `ContentContractManifestV1`, local identity,
  profile binding, reference, requirement, and provenance contracts exactly
  as specified. Do not redefine the existing maintainer
  `card-definition-manifest.v1`.
- Reuse the existing `ContentContractIdV1` newtype/domain. Its verified
  constructor consumes only exact canonical manifest bytes and the existing
  digest envelope. No ID is minted for profiled production content in M4.1.
- Keep source provenance outside the content digest. Changing provenance
  does not change content identity.
- Keep `CardSemanticProfileId`, envelope version, capability key/version,
  support profile, and content contract as separate axes. Add no empty
  placeholder profile identity and no M4.1 support-profile admission.
- Retain ordered hybrid printed mana symbols as data only. No hybrid-payment
  logic is introduced.
- Remove/quarantine `ExperimentalEffect` according to §6 Task 8; never
  promote its variants to stable V1 semantics.
- `FullStateDigestV5`, `EnvironmentCheckpointV6`, `CheckpointDigestV6`, and
  Replay V6 remain unchanged. Do not bind content ID into EngineState or
  checkpoint/replay identity here.
- No wire, state, checkpoint, replay, semantic-contract, M3 rules, or current
  generated execution-profile version is bumped by this work.
- Any required version bump, content-execution binding, state member,
  executable profile, real card, or persistent change not specified above is
  a stop-and-review condition, not an implementation-plan refinement.

## 9. Expected commit structure

Keep a single implementation PR but use small reviewable commits in this
order, squashing only if the repository's PR policy requires it and without
losing RED evidence from the test history:

1. `docs: publish normative M4.1 content and digest contracts`
2. `test: add M4.1 content identity and validation RED fixtures`
3. `feat: add closed CardDefinition foundation types`
4. `feat: add canonical content contract identity and catalog closure`
5. `feat: add fail-closed content preflight and registry closure adapter`
6. `test: close M4.1 determinism and non-regression evidence`

This order is binding: normative persistent/wire contracts land before RED
fixtures, and RED evidence lands before any producer or preflight
implementation. Do not commit production implementation ahead of its
normative contract or failing evidence.

Each commit stays within M4.1. Do not update #221/#222 automatically or
change their milestone status. Issue updates, implementation PR creation, and
acceptance-state changes follow the separately authorized implementation
workflow after this Plan receives explicit acceptance.

## 10. Stop conditions

Stop implementation and return to Spec/Plan review if any of the following
is discovered:

- current master drift changes the accepted contract, schema authority,
  persistence codec, Capability Registry, or M4.0 prerequisite;
- M4.0 post-merge exact-master evidence required by #221 is absent or not
  PASS at the point production implementation is about to begin;
- a typed content preflight cannot use the canonical registry data without
  introducing an independently authored support authority;
- a needed characteristic/layout cannot be expressed by the accepted V1
  envelope;
- `ContentContractIdV1` cannot be implemented through the existing canonical
  CBOR/digest-envelope authority with exact bytes and re-encode checks;
- correct behavior would require a profiled production body, executable
  profile, real R1/W1 definition, RulesKernel path, Magic semantic claim,
  runtime state, Decision/continuation, or M4.2+ work;
- a state, digest, checkpoint, replay, player-information, or decision
  contract must change;
- removing `ExperimentalEffect` requires changing its semantic behavior or
  migrating content not in this M4.1 scope;
- persistent/wire contracts cannot be made normative and kept in parity with
  their executable representation;
- any required conformance or repository gate is blocked/failing and no
  authorized remedy is available.

On a Spec defect:

```text
STOP implementation
→ amend M4.1 Spec
→ independent Spec review
→ regenerate this Plan from the newly accepted exact SHA
→ independent Plan review
→ resume only after explicit acceptance
```

## 11. Implementation exit evidence

The PR may be presented for implementation review only when all of these
claims have direct evidence on its exact head:

- every M4.1-owned contract in the accepted Spec is implemented;
- the closed immutable definition/catalog and exact content identity pass
  their positive, mutation, conflict, provenance, and canonical-byte tests;
- definition closure and mandatory/additive capability roots are
  deterministic and fail closed through the canonical registry authority;
- structural validation and `ContentValidationOnly` reject every required
  invalid/unresolved/unsupported input;
- valid Foundation content still cannot enter gameplay;
- `ExperimentalEffect` remains non-authoritative and no executable
  replacement profile exists;
- no real R1/W1 support is added;
- privacy, Decision V2, state ownership, FullStateDigestV5, Checkpoint V6,
  and Replay V6 retain their exact contracts;
- normative/executable content and digest contracts agree with the code,
  schema, fixtures, and generators;
- all required local gates and hosted CI report actual PASS on the exact
  implementation head, with every skipped/unavailable check separately
  recorded;
- independent code review has no unresolved BLOCKER or MAJOR.

These are implementation-review prerequisites, not `M4.1_FINAL_ACCEPTANCE`.
The owning issue's hosted CI, exact-head review, merge, and post-merge
verification sequence still applies.

## 12. Unchanged contracts and explicit non-goals

The following remain unchanged throughout this plan:

```text
RulesKernel execution authority and TransitionProduct commit flow
Decision V2 and current continuation model
information projection and player-visible opaque identity
EngineState and FullStateDigestV5
EnvironmentCheckpointV6 / CheckpointDigestV6
Replay V6
current M3 semantic and capability identities
existing bundle certification meaning
canonical capability registry source and lifecycle meanings
```

Explicitly excluded: executable Card-IR profile; real card execution or
support; Mountain, Plains, Lightning Strike, or any other R1/W1 definition;
general casting, stack, mana/payment, targeting, SpellPlan, triggers,
replacement, continuous effects, grouped operations, or copy; runtime
spell/ability/continuation state; new EngineState; new Decision family;
SupportProfile catalog/admission; R1/W1 closure; cross-deck/full-game closure;
ZERO-REACHABLE-UNSUPPORTED; final observation/Decision/replay closure; bundle
freeze/certification; Oracle parser/import; performance/search/trajectory/ML;
M4.2.

---

## 13. Stop point

After this Plan is committed and pushed for independent review:

```text
STOP
```

No implementation branch, Rust source, schema, fixture, CardDefinition,
semantic profile, issue update, PR, or M4.2 work begins until this exact Plan
is explicitly accepted.
