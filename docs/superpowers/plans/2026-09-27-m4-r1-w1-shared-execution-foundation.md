# M4 R1 × W1 Shared Execution Foundation — Implementation Plan

**Task:** `M4_R1_W1_SHARED_EXECUTION_FOUNDATION_SPEC_AND_IMPLEMENTATION_PLAN`
**Status:** PROPOSED / NOT ACCEPTED; executable only after Spec acceptance and entry gate
**Spec:** [`2026-09-27-m4-r1-w1-shared-execution-foundation.md`](../specs/2026-09-27-m4-r1-w1-shared-execution-foundation.md)
**Exact proposed baseline:** `6c6ee4c9b237696c50e944cae998f85c2d358e1c`
**Implementation authorized:** NO

## 1. Baseline and worktree model

The verified baseline is post-PR #248 `origin/master` at the SHA above. It contains the current M4.2 state cut and bounded Mountain/Plains activation. It has 14 registry rows (eleven covered Foundation roots and three M4.2 roots still `specified`). M4.2 is complete only for Mountain + Plains / `basic-land@1.0.0`; no R1/W1 playability is implied.

After the entry gate, freeze a new exact implementation base. Use an isolated worktree/branch from that frozen master for Shared contract growth, with one current writer and no parallel runtime authority. Do not branch from the stale R1-exclusive proposal head. Keep each independently reviewable contract/state boundary in its own PR; dependent R1 and W1 card/profile implementation remains paused until the corresponding Shared owner is accepted and evidenced. No branches or worktrees are created by this plan.

## 2. Dependency order and batch boundaries

The order follows the accepted Spec. A later batch must not duplicate a persistence migration already owned by an earlier contract cut.

| Batch | Owner / dependencies | Main work and likely paths | Persistent state / decision / contracts | R1 work unblocked | W1 work unblocked |
|---|---|---|---|---|---|
| **G0 — Contract-growth boundary** | Architecture/persistence/decision owners; entry gate first | Separate accepted contract-growth Spec; compatibility matrix and migration/disposition; Rust DTOs/codecs, catalog source, Python/schema/golden plan. Likely `docs/contracts`, `docs/STATE_HASHING.md`, decision/persistence/replay/observation contract owners, `contracts/catalog/`. | Decide typed stack/trigger/effect records, candidate bindings, event/delta forms and identity cut. Review EngineStatePartsV2, StateDeltaV2, FullStateDigestV6, CheckpointV7, Replay V7, Decision/ObservedEvent/PlayerStep and information projection together. No next versions chosen by this plan. | Required before casting, abilities, triggers, effects, Ojer/Kicker payload dependencies. | Same; required for W1 spells, Auras, Ward and triggers. |
| **S1 — Characteristic and object query authority** | G0; CardDefinition and M4.2 state accepted | One bounded derivation query path for base/face/counter values and admitted source-derived contribution inputs. Likely rules/state, Card IR profile source, conformance. | Reuse FaceState, CounterState, AttachmentState and snapshots. Queries are derived; persistent temporary records belong to S6a. No new Decision. | Ojer power/source queries, creature/color/type predicates, target and replacement legality. | Base/face and keyword/type/attachment predicates used by Auras, Role, modified, protection and combat. |
| **S2 — Cost and payment primitives** | G0; mana-pool and M4.2 mana state; S1 | Enumerate bounded cost terms and complete restricted allocations; atomic pay operation; cost-profile validation. Do not implement a complete spell cast in this batch. Likely rules mana/cost modules, delta/events, Decision/schema fixtures. | Reuse ManaState. Payment binding changes through G0; commit atomically. Paid-route fact is captured later by S4 stack payload. No AutoPay. | Ordinary costs, Burst kicker route, Rockface restriction, activation costs. | All spell/Aura costs and later Ward payment profile. |
| **S2b — Source-derived static contributions** | G0, S1; AttachmentState | Evaluate bounded contributions from live source profiles and attachment relations through S1 before target construction. Aura/Role profiles remain W1-owned; this is not effect expiry. | No shared effect record; derive from current source/profile/attachment. AttachmentState persists the relation. | Current-source characteristic queries only; no temporary lifetime behavior. | Aura/Role contributions while source and attachment apply; no until-EOT record semantics. |
| **S3 — Target domains and trusted bindings** | G0, S1, S2b | Build complete declaration-time target domains from bounded derived characteristics and perspective-safe identities. This batch owns target legality/selection bindings; it does not claim a spell/Aura has been fully cast. | Trusted targets are not persisted alone here; S4 stores them in stack payload. Add only required candidate/binding types. | Targeted damage/abilities, target-trigger inputs. | Aura/targeted spells, Ward target event and target protection. |
| **S4 — Cast/activation orchestration and stack authority** | G0, S1–S3; priority/zone owners; AbilityAuthorityState | Orchestrate spell selection, routes, modes/targets, final cost/payment in pinned CR order, atomic commit, cast/activation events, typed stack payload and resolution. Likely RulesKernel, state stack/execution owner, events and profile decoder. | Persist complete typed payload and captured choices. Extend delta/events; digest/checkpoint/replay via G0. Existing V3 CastSpell/ActivateAbility intents are reused only where exact binding suffices. | All R1 spells/abilities, cast triggers, Kicker paid-status payload. | All W1 spells, Auras/abilities, Ward's targeted-stack interaction. |
| **S5 — Trigger authority and ordering** | G0, S4; typed event cursor and stack owner | Enumerated event detectors, typed trigger snapshots, APNAP placement, same-controller order, target-at-trigger timing and resolution. Likely rules trigger modules, execution state, Decision and event conformance. | Persist waiting trigger and stack payload; capture source/event/LKI across boundaries. Exact patterns only, no universal trigger language. | Hired Claw, Magebane, Emberheart, Hearthborn, Ojer/Nova entry/death, Needlehead triggers. | Ward, Scavenger, Vendor/Role, Bat and other locked trigger clauses. |
| **S6a — Persistent temporary effects and expiry** | G0, S1, S5, turn/cleanup owner | Typed additive P/T, admitted keyword/type grants and selected conditions whose effects persist across decisions; explicit expiry/source dependence/timestamps. Likely state/effect modules, RulesKernel, cleanup, event/observation and property tests. | New typed temporary records in state/delta/digest/checkpoint/replay. Cleanup emits exact expiry operations. No Aura/Role static record is placed here. | Prowess and Rockface temporary effect; Haste remains R1 profile. | Only witnessed temporary W1 grants; Aura/Role contributions stay out of this lifetime owner. |
| **S6b — Replacement, damage, counter/entry and interaction closure** | G0, S1–S6a; existing zone-incarnation, damage/life, SBA owners | Replacement eligibility/application, stack counter/destination, noncombat damage, counter operations and entry/LKI consistency. Likely rules events/delta, observation and cross-layer tests. | Reuse counter/zone/mana/history state; no ReplacementState. Replacement stays transaction-local for known static/live profiles. | Burn, Ojer profile interface, R1 instant/sorcery destinations, countered spells. | Ward counter profile, Dryad replacement profile, Aura/counter/entry interactions. |
| **S7 — Cross-deck foundation conformance and handoff** | S1–S6b; all affected contract and gates green | Independent oracle closure, property/fuzzing, paired-state tests, full interaction vectors, documentation/capability evidence, exact head review. No card-definition closure or certification here. | Demonstrate direct/restore/fork/replay parity and compatibility for exact accepted Shared scope. Capability lifecycles move only at evidence gates. | Enables later R1-specific profile work to resume against accepted owners. | Enables later W1-specific profile work to resume against accepted owners. |

First strike, flying, lifelink, vigilance, reach, trample, Haste, Prowess, Ward, Dryad replacement, Warp, and other named mechanics are not bundled together as a keyword batch. Their profiles and exact combat/target/replacement requirements must be separately owned. A characterization showing that a semantic owner is not truly required by both decks removes it from Shared before implementation.

## 3. RED-first implementation sequence

For every batch: pin exact authority and accepted scope; add a failing behavior characterization; add rejection/fabrication and nonmutation cases; independently establish candidate soundness and completeness; implement only the accepted bounded operation; add exact state/event/delta projections; then run the enclosing batch regressions. A batch with no RED case demonstrating both the intended result and the nearest unsupported boundary is not ready to implement.

Suggested representative RED sequence:

1. Typed card/face characteristic queries; add/remove selected counter and attachment operations; ordering ambiguity rejects. Verify derived static contributions never allocate temporary-effect records.
2. Every legal payment allocation is emitted once, no illegal allocation is emitted, restricted mana is honored, and any stale/fabricated allocation leaves all bytes unchanged.
3. Target candidates contain all-and-only legal visible targets and trusted incarnation bindings. Cover Lightning Strike/Aura target selection before any targeted-cast claim; hidden IDs cannot affect order or bytes.
4. Cast orchestration consumes selected target/mode bindings, follows pinned CR cost ordering, and creates exact stack payload; source departs; resolution still uses captured choices. Kicker paid/unpaid branches differ only by captured paid status.
5. Non-mana activation consumes S3 target bindings, checks zone/timing/cost, and preserves payload after source departure; mana abilities resolve without stack only where profile/rules allow.
6. Trigger creation captures correct event-boundary values; simultaneous triggers use APNAP and explicit player order; no collection-order path exists.
7. Temporary effects persist and expire exactly at their admitted boundary. Separately, source-derived Aura/Role contributions appear only while the source/attachment profile applies and never become temporary records.
8. Replacement application modifies the attempted event before final state/history; countered stack items do not resolve; replacement choice is explicit when required.
9. For each new authoritative record, run digest/checkpoint/restore/fork/replay parity and paired hidden-state noninterference.

Any required meaning that is not expressible without changing an accepted Spec returns to Spec review. Do not patch semantics into implementation tests or this Plan.

## 4. Cross-layer contract sequence

G0 owns one compatibility decision before S1–S7 changes persisted/public shapes:

1. Enumerate fields and exact semantic transitions; prove what is ephemeral, derivable, or persistent.
2. Freeze state/event/decision/observation shapes and historical disposition in an accepted contract-growth Spec. Preserve historical bytes/meanings; do not dual-write executable authorities.
3. Add RED schemas, positive/negative wire fixtures, canonical goldens, and Rust/Python parity before production producers.
4. Add typed state and validation plus FullStateDigest/StateDelta/checkpoint/replay identity handling under the accepted cut; one current writer only.
5. Add Decision domains/bindings and PlayerStep/ObservedEvent/observation projection only after reviewing completeness and information exposure.
6. Implement RulesKernel producers and atomic environment commit after detached state contracts close.
7. Re-execute replay responses and compare full state/event/delta/request/status/observation products. Observations are never replay control input.

No numeric identity (EngineStateParts, Digest, Checkpoint, Replay, Decision, Event, PlayerStep, schema) is allocated by this plan. Migration is explicit per `docs/maintenance/SCHEMA_EVOLUTION.md` and `docs/contracts/COMPATIBILITY_POLICY.md`; no automatic relabeling of historical artifacts.

## 5. Likely repository owners and affected layers

Paths below are inspection targets, not a promise to touch all of them. The contract-growth review decides exact files.

* Rules/state: `crates/mtgml-rules/`, `crates/mtgml-state/`, `crates/mtgml-model/`; retain one RulesKernel and existing typed state owners.
* Decisions: `crates/mtgml-decision/`, `schemas/player-decision-request.*`, `schemas/player-step.*`, Python DTOs/codecs, positive and negative wire goldens.
* Products/privacy: `crates/mtgml-environment/`, `crates/mtgml-observation/`, `crates/mtgml-conformance/`, observed-event schemas/fixtures and `python/tests/`.
* Persistence/replay: `crates/mtgml-persistence/`, `crates/mtgml-replay/`, state digest/checkpoint codecs and goldens.
* Content/capability: `crates/mtgml-card-ir/`, authoritative `contracts/catalog/`, owner docs under `docs/rules/capabilities/`, then generated projections only via generators. Do not add CardDefinitions or registry lifecycle edits in Shared foundation work without the normal accepted owner spec.
* Documentation: `docs/STATE_HASHING.md`, `docs/REPLAY_AND_DETERMINISM.md`, `docs/INFORMATION_MODEL.md`, contract/compatibility docs, capability owner specs, test/conformance authoring and status surfaces where evidence changes.

## 6. Verification and review gates

Run the cheapest applicable checks early and the broad profile required by the accepted contract cut. Likely repository gates include `just check-fast`, `just check`, and for cross-layer semantic/persistence/privacy changes `just check-all`; run generated contract/catalog `--check`, schema validation, Rust formatting/clippy/tests, Python tests/toolchain, golden parity, property/fuzzing, noninterference, interaction/conformance, exact-head hosted CI, independent review, then post-merge exact-master verification. Use `just release-candidate` and archive verification only when the accepted slice's release/archive gate requires them; archive check runs after all source edits. Do not treat this design-task validation as execution of any future gate.

Every report distinguishes `PASS`, `FAIL`, `NOT_RUN`, `BLOCKED`, and `EXPERIMENTAL`. No gate is green by implication. Lifecycle promotion is capability-scoped and tied to evidence. Certification is outside Shared foundation exit.

## 7. Expected PR decomposition

1. Contract-growth/compatibility Spec and detached closed-contract fixtures (G0 review boundary).
2. State/event/Decision/persistence identity cut and language/schema parity (G0 implementation, if separately authorized).
3. Bounded characteristic/object query and counter/attachment read integration (S1).
4. Cost determination and complete mana payment (S2).
5. Source-derived static characteristic contributions (S2b), using S1 and existing attachment state without temporary-effect records.
6. Target-domain legality, candidate completeness, and trusted binding (S3).
7. Cast/activation orchestration, typed stack authority, and resolution (S4; targeted W1/R1 cast support only after S3).
8. Trigger creation, payload, APNAP placement/order (S5).
9. Persistent temporary-effect records/lifetime (S6a).
10. Replacement/counter/noncombat damage and zone-entry interaction owners (S6b; split by semantic/identity owner).
11. Shared cross-deck conformance and parity closure (S7).

PR boundaries follow actual review ownership and identity cuts. Shared implementation is a prerequisite for dependent content, but no batch claims either deck's full closure. R1/W1 card definitions, exclusive mechanics, full-game setup, and certification remain downstream.

## 8. Stop conditions

Stop before dependent implementation if any condition occurs:

* the live master baseline differs from frozen implementation base;
* current accepted #225/#248 evidence contradicts status or exact scope;
* #237 ownership claims collide or its refreshed matrix changes Shared owner;
* generic owner duplicates an accepted capability or conflicts with current state ownership;
* a persistent field cannot be versioned without reinterpreting a frozen identity;
* current Decision hierarchy cannot provide complete/sound player control;
* player projection leaks trusted identity or violates paired-state noninterference;
* direct, restored, forked, replayed products diverge;
* generic behavior needs a universal interpreter/DSL or reaches out-of-scope Magic;
* a test or required tool/gate is unavailable or fails.

For a semantic discovery: STOP → amend the Spec → independent re-review → regenerate this Plan. Do not silently shrink the Spec in implementation.

## 9. Must remain unchanged until separately authorized

This Plan authorizes no production implementation. At planning/design time, keep runtime Rust/Python, schemas, wire contracts, CardDefinitions, registry lifecycle, decklists, and R1/W1 production branches untouched. After future entry acceptance, every change stays within the exact accepted Shared batch and its contract closure. Do not promote M4.2 roots because M4.2 itself completed; do not claim M4.3/M4.4 support; do not create card-name dispatch; do not duplicate common engines below exclusive profiles; do not weaken a gate or historical contract to make implementation appear complete.
