# M4 Shared S1 — Characteristic and Object Query Authority Implementation Plan

**Task:** `M4_SHARED_S1_CHARACTERISTIC_OBJECT_QUERY_AUTHORITY`
**Status:** PROPOSED — PENDING DESIGN REVIEW
**Spec:** [`2026-09-28-m4-shared-s1-characteristic-object-query-authority.md`](../specs/2026-09-28-m4-shared-s1-characteristic-object-query-authority.md)
**Subordination:** This Plan is subordinate to the S1 Spec and the accepted Shared/G0 specifications. It sequences work and tests; it does not add semantic rules.
**Implementation authorized:** NO — this Plan requests review only.

## 1. Exact implementation starting point

```text
DESIGN_BASE = 137e0f2e6bffa667ab09943a686fa6411cd74059
              (verified origin/master at design preparation)
TARGET_IMPLEMENTATION_BASE = NOT_FROZEN
CURRENT_RUNTIME_IDENTITY = EngineStatePartsV3 / FullStateDigestV7 /
                           Checkpoint V8 / Replay V8 / Decision V4
REQUIRED_DEPENDENCIES = accepted G0 contracts; accepted Shared Spec/Plan;
                        reviewed Issue #237 reconciliation; accepted S1 Spec/Plan;
                        current Card IR/content and state owners
```

The exact implementation baseline must be fetched, verified and recorded immediately before implementation starts. It may differ from this design base. Do not create an implementation branch or worktree during design preparation. Any intervening master change to Card IR, state, profiles, query ownership, locked evidence, or G0 identities requires re-running the affected source audit and updating this plan before coding.

## 2. Current owners and planned file boundaries

Actual owner paths inspected at the design base:

| Concern | Existing owner | Planned use |
|---|---|---|
| Immutable Card IR and verified content catalog | `crates/mtgml-card-ir/src/lib.rs` | Reuse `CardDefinitionEnvelopeV1`, `FaceDefinitionV1`, `BaseCharacteristicsV1`, `CardSemanticBindingV1`, `VerifiedContentCatalogV1::get`; do not add a parser or content cache. |
| Current authoritative aggregate | `crates/mtgml-state/src/engine_state_parts_v3.rs` | Borrow the validated `EngineStatePartsV3` from RulesKernel; do not add state or copy it into S1. |
| Object incarnation, zone, owner/controller | `crates/mtgml-state/src/zones.rs` | Read `ZoneState.objects` and `locations` by exact `GameObjectId`; never resolve through physical-card identity. |
| Face, counter and attachment state | `crates/mtgml-state/src/persisted_v6.rs`, exported from `crates/mtgml-state/src/lib.rs` | Read existing `CardRulesAuthoritativeStateV1` maps. Add no persisted fields. |
| Snapshot/LKI / typed payload | `crates/mtgml-state/src/zones.rs`, `crates/mtgml-state/src/shared_execution.rs` | Reuse exact captured values only. Do not expand snapshots without G0 review. |
| Rules authority | `crates/mtgml-rules/src/` (current G0j RulesKernel and Basic Land owner) | Add one internal query module and route all S1 derivation through the existing RulesKernel. No new kernel/backend. |
| Conformance | `crates/mtgml-conformance/src/` | Add independently authored characteristic/object scenarios and expected-value oracle. |
| Environment / privacy tests | `crates/mtgml-environment/src/tests/`, current `successor_projection.rs` and endpoint tests | Verify results stay internal and only existing projector returns player products. Do not add a query endpoint. |

Likely new module: `crates/mtgml-rules/src/characteristic_query.rs` (or the repository's existing canonical rules-query module if one exists at implementation-base review). Keep errors and result types near this owner. Extend `crates/mtgml-rules/src/lib.rs` only as needed for crate-internal routing. Avoid production changes in `mtgml-state`, `mtgml-card-ir`, Python, wire schemas, registry, digest, checkpoint, replay, observation, and deck artifacts unless source review proves an invariant gap; any such required contract change stops implementation for G0/design amendment.

Test locations should follow current test ownership: unit tests beside the new rules module; semantic red/positive cases in `crates/mtgml-conformance`; integration/noninterference and checkpoint/fork/replay equivalence in existing environment test modules. Use test-only Card IR manifests and provenance fixtures with fixed content identities; they do not become admitted production cards.

### Files to keep unchanged

- `cards/capabilities/registry.json` and generated capability projections;
- all schemas, golden/negative wire fixtures, frozen identity constants and ADR 0056;
- R1/W1 deck evidence in #222 and source/provenance artifacts;
- Python rule or candidate logic;
- `ObjectSnapshot`, `CardRulesAuthoritativeStateV1`, state digest/checkpoint/replay identity fields;
- G0 runtime aliasing and the Basic Land RulesKernel behavior;
- S2, S2b, S3–S7 owners and card-specific profile execution.

## 3. Dependencies and ownership boundaries

| Consumer / owner | S1 provides | S1 does not provide |
|---|---|---|
| S2 cost/payment | Verified object type/face/counter query where an admitted cost predicate needs it | Cost determination, allocations, payment, source activation |
| S2b source-derived static contribution | Same live-object/face/type/counter/attachment query boundary | Aura/Role profile interpretation, contribution ordering/application |
| S3 target domain | Internal characteristic predicates for target legality | Target enumeration, candidate completeness, trusted target bindings |
| S4 cast/activation/stack | Exact source object and current characteristic reads at admitted boundaries | Cast/activation sequencing, stack payload, target capture, resolution |
| S5 triggers | Event-time query inputs already authorized by the typed event/source record | Trigger detection, trigger-time LKI reconstruction, APNAP/order |
| S6a temporary effects | Shared consumer path for later admitted typed effect results | Persistent effect record, expiry, temporary effect creation/application |
| S6b replacement/damage/counter/zone | Derived inputs needed by a characterized operation | Counter mutation, SBA, replacement iteration, damage or zone transition |
| Observation / endpoint | No direct API; existing projector may use authorized facts | Projection, knowledge acquisition, hidden identity disclosure |

If any downstream consumer needs final effective characteristics, it must provide its accepted typed contributor to this same RulesKernel-owned path. It must not consume S1's base/counter stage as though it were final. A later contributor's omission is an error.

## 4. RED-first verification matrix

Every expected output is hand-authored independently from the pinned 2026-09-25 CR/Oracle facts and a separately constructed authoritative state. Expected code/data must not call the production S1 derivation helper. For the locked cards, tests are detached semantic witnesses until their exact definitions and profile closures are accepted.

| Area | RED case / expected independent witness | Negative / invariant |
|---|---|---|
| A. Content identity | Build a verified catalog from canonical manifest/provenance; query exact definition/face and compare hand-authored name-independent characteristic vector | Wrong content contract, recomputed-ID mismatch, absent definition, broken provenance join → typed error, no partial/default result |
| A. Face authority | Mountain/Plains FaceKey 0; detached two-face fixture with explicit current face; expected type/color/base values recorded literally | Missing face-state entry, ordinal not in definition, definition with no matching face, unsupported face/profile → error, never first-face fallback |
| A. Card IR source | Validate each selected `FaceDefinitionV1` value against Card IR facts | Malformed/unsupported characteristic source or invalid type/color vocabulary → reject during catalog validation or typed query error; never infer from name |
| B. Live object | Query exact current ID; expected owner/controller/zone/definition from an independently assembled fixture | Unknown ID, mismatched map key vs object.id, missing location/player/content join → typed error |
| B. Stale incarnation | Apply a zone transition in a conformance fixture; query old and new IDs separately | Old ID is `StaleObjectIncarnation`, even if physical card identity matches; no PhysicalCardId search |
| B. Controller / owner | Change controller while retaining owner; expected owner unchanged/controller changed | Missing player or an ID from another current object is invalid |
| B. Snapshot | Capture `ObjectSnapshot`, mutate live object's control/zone/face/counters afterward; snapshot query returns only captured fields | Snapshot must not be silently enriched with current counters/face or treated as a complete characteristics snapshot |
| C. Base/face | Exact type-line vectors, color set, optional base P/T from fixture | Absent P/T remains `None`; name/mana cost does not default absent power/toughness |
| C. Counter-derived P/T | Base 2/3 + one +1/+1 → 3/4; base 2/3 + one -1/-1 → 1/2; both → 2/3; lore unchanged | u32 maximum values checked after promotion; corrupted maps, stale counter key, no object, invalid state → error; query does not mutate or perform counter annihilation |
| C. Type/color predicates | Exact membership over normalized supertype/type/subtype vectors; hybrid mana symbol contributes both colors; generic/colorless contributes none | Absent component is false; unverified face or malformed symbol is error, not false |
| C. Modified | Creature with a counter; creature equipped by Equipment; creature enchanted by a same-controller Aura; Aura controlled by another player does not satisfy Aura branch | Noncreature false; stale attachment endpoint, unknown attached face or unsupported source class → error |
| D. Attachment relation | Query source→target and target→sources from independent relation table; expected sort by timestamp | Duplicate timestamp/inconsistent state is rejected by validation; don't sort by GameObjectId or map insertion order |
| D. S2b seam | Live Aura relation changes relation-query result only; S1 never changes target P/T/keywords due to Aura text | Explicit test asserts an attached Aura with a contribution profile gets `UnsupportedContributor` in S1 until S2b admits it |
| D. Source lifecycle | Remove source or target, allocate new incarnation, then query relation | Old relation cannot attach to a new incarnation by matching physical identity; invalid edge fails closed |
| E. Repeated/deterministic | Repeated exact input gives byte/equality-identical result; reverse construction/insertion order of test inputs where semantics are maps | No hash-map iteration, pointer identity, wall time, RNG, environment or global cache dependency |
| E. Mutation independence | Change an unrelated player field/counter/object; expected query stays equal | Change the queried object's face, type source, counter or attachment edge; only justified result changes |
| E. State purity | Fingerprint state, allocators, RNG, pending request, digest, checkpoint and replay before/after every successful and failed query | All bytes/identities equal after query; no allocator/RNG advance or hidden cache writes |
| E. Restore/fork/replay | Query same state directly, after Checkpoint V8 restore, fork, and authoritative Replay V8 execution | Same result and state identity; no new checkpoint/replay fields or version |
| F. Information safety | Invoke queries internally on paired states with same authorized perspective but changed hidden IDs/definitions/order | Existing observations, information state, requests and events remain byte-equal; no trusted ID is projected |
| F. Existing projector only | Public result changes only when an accepted public event/knowledge rule allows it; use current projector/endpoint | No serialization endpoint for S1 result, no private errors/definition IDs/face identity leak |
| G. Unsupported contributors | Present a typed temporary effect or live Aura/Profile source outside current S1 stage | Error `UnsupportedContributor`; no apparently final P/T/keyword result |
| G. Profile/ordering | Unknown profile, missing ability/profile facts, face-down object, or requested keyword | `ProfileNotAdmitted`/`UnsupportedCharacteristic`; no `false`, default keyword set, or silent source omission |
| G. Invalid state | Invalid face map, attachment edge, counter key, object/location join, content binding | Invalid state rejected before a value is returned |

## 5. Implementation PR sequence

These are future implementation PRs, not created in this task. They run serially on one implementation lineage after entry gates. A stage may combine with the adjacent stage only when the same semantic invariant and tests remain reviewable; do not expose a partial final-characteristic API between stages.

### S1-A — RulesKernel query boundary and typed errors

```text
TASK_ID = S1-A
DEPENDENCIES = accepted S1 Spec/Plan; fresh exact implementation base
EXACT_SCOPE = private query context bound to validated state, verified content catalog,
              and exact admitted Rules execution identity; typed errors; query purity shell
FILES_TO_MODIFY = crates/mtgml-rules/src/characteristic_query.rs (new),
                  crates/mtgml-rules/src/lib.rs, focused unit tests
FILES_NOT_TO_MODIFY = state/Card IR schemas, Python, registry, observation,
                      digest/checkpoint/replay contracts, deck/card artifacts
RED_TESTS = wrong content ID; unknown/missing definition; invalid profile admission;
            missing object/location; stale object; malformed face binding; state purity
IMPLEMENTATION_STEPS = build private RulesKernel-owned context; validate exact joins;
                       add typed error/result shells; call no mutation APIs
NEGATIVE_TESTS = unverified catalog/context constructor unavailable to callers;
                 every failed lookup leaves all state bytes/identities unchanged
CONFORMANCE_WITNESSES = generic verified fixture plus existing Basic Land manifest only
COMPATIBILITY_IMPACT = none; no current producer/API/identity change
ACCEPTANCE_CRITERIA = one internal authority, typed errors, Rust unit tests green,
                      no public endpoint or hidden cache
STOP_CONDITIONS = requires new state field, arbitrary profile callback, or second kernel
```

### S1-B — Current object, face and base characteristics

```text
TASK_ID = S1-B
DEPENDENCIES = S1-A
EXACT_SCOPE = exact GameObjectId/ZoneLocation/owner/controller join; current FaceState;
              verified FaceDefinition; type line, colors, optional base P/T result
FILES_TO_MODIFY = same Rules query module; independent conformance fixture/oracle;
                  focused conformance tests
FILES_NOT_TO_MODIFY = object/snapshot/digest schemas; player DTOs; content identities
RED_TESTS = Mountain/Plains, two-face detached fixture, controller/owner, zone changes,
            incorrect/missing face and absent P/T
IMPLEMENTATION_STEPS = no fallback face; derive color from exact face's mana cost and
                       indicator; preserve validated type-line facts; promote P/T safely
NEGATIVE_TESTS = missing face, wrong definition, face-down, unknown type/color source,
                 old ID after zone transition
CONFORMANCE_WITNESSES = pinned Mountain/Plains facts; detached Ojer front/back face facts;
                        no Ojer execution claim
COMPATIBILITY_IMPACT = none; uses existing V8 state and catalog identity
ACCEPTANCE_CRITERIA = all supported fields match independent literals; stale IDs reject;
                      exact production Basic Land behavior unchanged
STOP_CONDITIONS = any required characteristic is absent from Card IR or face rule ambiguous
```

### S1-C — Counter-adjusted P/T and modified query

```text
TASK_ID = S1-C
DEPENDENCIES = S1-B; confirmed exact counter invariants at frozen implementation base
EXACT_SCOPE = +1/+1 and -1/-1 P/T adjustment; lore no-op; bounded modified predicate
              over counters/current Aura and Equipment type lines/attachment relation
FILES_TO_MODIFY = same Rules query module; rules/conformance tests
FILES_NOT_TO_MODIFY = counter mutation/SBA RulesKernel owners; CardRules state encoding;
                      S2b effect application; S6a effect records
RED_TESTS = positive/negative +1/+1 and -1/-1 cases; both kinds; lore; modified by counter,
            Aura controller condition, Equipment; stale source/target
IMPLEMENTATION_STEPS = checked integer promotion/arithmetic; no counter mutation;
                       modified uses only the exact pinned CR predicate and represented facts
NEGATIVE_TESTS = invalid counter refs/maps, unknown attachment profile/type, overflow,
                 request against face-down/unsupported profile
CONFORMANCE_WITNESSES = detached hand-authored creature/counter/Aura/Equipment fixtures
                        grounded in the pinned CR and locked C40/C41/C76 witnesses
COMPATIBILITY_IMPACT = none; no digest/schema change
ACCEPTANCE_CRITERIA = literal oracle parity; no state or allocator mutation; contributor
                      uncertainty errors rather than returning an incomplete final value
STOP_CONDITIONS = valid locked witness needs another counter kind or unrepresented layer
```

### S1-D — Attachment relation queries and S2b seam

```text
TASK_ID = S1-D
DEPENDENCIES = S1-A and S1-B; S1-C if modified is in same lineage
EXACT_SCOPE = attachment_of / attachments_to exact live IDs; deterministic timestamp order;
              explicit unsupported-contribution boundary for S1
FILES_TO_MODIFY = same Rules query module; focused relation/conformance tests
FILES_NOT_TO_MODIFY = Aura/Role profile interpretation, contribution values, attachment
                      mutation, Role uniqueness SBA, effects state, observation schema
RED_TESTS = no edge, one edge, multiple timestamp-ordered edges, controller change,
            source removal, target removal, incarnation replacement
IMPLEMENTATION_STEPS = consume validated AttachmentState; return typed relation facts;
                       do not evaluate attached source text or mutate target characteristics
NEGATIVE_TESTS = duplicate timestamp, stale endpoint, unverified attached definition,
                 Aura present but contribution query requested
CONFORMANCE_WITNESSES = W1 attachment state witnesses from accepted M4 feasibility/Shared
                        Spec; fixture is not Aura card support
COMPATIBILITY_IMPACT = none; consumes already hashed attachment state
ACCEPTANCE_CRITERIA = relation facts exactly match state, deterministic by timestamp;
                      S2b-only contribution is rejected in S1
STOP_CONDITIONS = relation contract disagrees with accepted AttachmentState invariants
```

### S1-E — Cross-layer conformance and S1 closure

```text
TASK_ID = S1-E
DEPENDENCIES = S1-A through S1-D; accepted S1 Spec; no dependent semantic owner substituted
EXACT_SCOPE = table-driven/property tests, checkpoint/fork/replay equivalence, hidden-world
              noninterference, historical compatibility and API/static review
FILES_TO_MODIFY = crates/mtgml-conformance/src/ characteristic scenarios;
                  existing environment query/internal privacy tests only
FILES_NOT_TO_MODIFY = production projector/API unless a separately reviewed issue proves
                      an existing defect; contract identities, registry, Python rules
RED_TESTS = full §4 matrix in this Plan, including map-order permutations and repeated
            failed query fingerprints
IMPLEMENTATION_STEPS = compare production values to independent fixed expected table;
                       exercise direct/restore/fork/replay state; exercise old fixture gates
NEGATIVE_TESTS = all unsupported boundaries and paired hidden worlds; verify absence of
                 query-related wire fields/new persisted records
CONFORMANCE_WITNESSES = bounded Basic Land facts plus detached exact R1/W1 characteristic
                        facts cited in S1 Spec §9
COMPATIBILITY_IMPACT = no intended change to wire, digest, replay or legacy readers
ACCEPTANCE_CRITERIA = all S1 exit gates in Spec pass; independent exact-head review and
                      hosted jobs green; docs/registry make no unearned lifecycle claim
STOP_CONDITIONS = state bytes, historical fixtures, player products, or R1/W1 scope drift
```

## 6. Verification gates and evidence status

**Design validation in this PR:** documentation register/local-link check, repository docs validation, generated contract/catalog `--check` (to prove no generated contract edits), `git diff --check`, and targeted cross-document review. Expensive Rust compilation is not applicable to documentation-only files. Proposed future test rows are not run and are never reported as passed.

**Future implementation gates:** the applicable documented repository profiles must run on exact source head:

```bash
just check-fast
just check
just check-all
cargo fmt --all -- --check
git diff --check
```

Use the cheapest early gate repeatedly; `just check`/`just check-all` are required before presenting the coupled query/conformance work as ready. Run relevant package tests during RED/GREEN iterations. `just check-generated` (or repository `just check-generated` recipe) and schema/catalog checks must remain clean even though S1 intends no generated changes. Historical V6/V7 fixtures must remain byte-identical. Run property/negative cases, existing checkpoint/fork/replay tests and paired-world noninterference tests that cover the newly consumed query path. Exact-head hosted Fast, Integration, and policy-required checks must pass before merge.

Status labels are evidence labels, not forecasts:

- `PASS`: actually executed successfully on the named exact commit.
- `FAIL`: executed and failed.
- `NOT_RUN`: not executed.
- `BLOCKED`: an unmet dependency prevented execution.
- `EXPERIMENTAL`: behavior exists without the required evidence level.

At design time, future implementation test and S1 exit gates are `NOT_RUN`; production behavior is `NOT_IMPLEMENTED` (not `PASS`).

## 7. S1 completion and handoff

S1 is complete only when:

1. bounded face/base/object, counter-derived P/T, type/color, modified and attachment relation semantics exactly match the Spec;
2. every supported query has all-and-only meaning and every unsupported profile, characteristic or contributor fails closed;
3. derivation is pure and deterministic, with no persisted cache, state owner, RNG/allocator mutation, or hidden environment input;
4. the current object incarnation and face are bound to verified immutable content and Rules authority;
5. snapshot/LKI use stays within exact captured facts; no stale physical-card resolution exists;
6. private internal facts cannot bypass the existing perspective projector; paired-world tests pass;
7. direct, checkpoint-restored, forked and replay-equivalent states produce equal query results;
8. independent conformance witnesses and historical compatibility fixtures pass;
9. no new schema/digest/checkpoint/replay identity, card definition, registry row, lifecycle promotion, or later-batch behavior was smuggled into S1;
10. required exact-head local and hosted gates pass and the source/docs agree.

`S1_IMPLEMENTED` or query coverage does not mean an R1/W1 card is supported; it does not mean M4, R1, W1, a playable bundle, or certification is complete.

## 8. Stop conditions

Stop the affected PR and return to Spec review if implementation needs an unreviewed characteristic layer/keyword/profile rule, extra counter kind, persistent query cache, source-derived Aura/Role operation, temporary effect, target/trigger/payment/casting behavior, new state or identity field, non-RulesKernel authority, Python rule behavior, card-name switch, arbitrary DSL/callback, or a player-facing query endpoint. Stop if an independent expected value cannot be derived from pinned authority or if any query leaks hidden facts, mutates state, or disagrees after checkpoint/fork/replay. Record the exact failing witness and required authority; do not fill gaps with inferred behavior.
