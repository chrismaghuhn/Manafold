# M4.3 R1-Exclusive Capability Implementation Plan

**Task:** `M4_3_R1_EXCLUSIVE_CAPABILITY_SPEC_AND_IMPLEMENTATION_PLAN`
**Status:** PROPOSED / DESIGN ONLY / NOT ACCEPTED
**Baseline:** `6c6ee4c9b237696c50e944cae998f85c2d358e1c` (`origin/master`)
**Derives from:** [M4.3 R1-Exclusive Capability Semantic Specification](../specs/2026-09-27-m4-3-r1-exclusive-capabilities.md)
**Production implementation authorized:** NO
**Date:** 2026-09-27

## 1. Execution boundary

This plan is executable only after the Spec is accepted, the exact R1 content identities are pinned, Shared prerequisites are coordinated, and the contract-growth review is accepted. It does not authorize production work now. No production PR should begin while this plan is proposed.

The expected dependency shape is:

```text
M4.2 accepted state substrate
          ↓
External Shared-owned prerequisites (not R1 batches):
  SHARED-CAST-PAYMENT + typed cast/stack payload
  SHARED-TRIGGER-STACK + target/event foundations
  SHARED-TEMPORARY-EFFECT + characteristic derivation
  Shared combat, replacement, entry and zone foundations as needed
          ↓
R1-E1 event-derived turn history producers and consumers
          ↓
R1-E2 ability-use / Valiant       R1-E3 Ojer damage replacement
          ↓                                  ↓
          └──────────────┬───────────────────┘
                         ↓
             R1-E4 Haste / Prowess profiles

R1-E5 trample / face / return advances from its named Shared prerequisites.
SHARED-CAST-PAYMENT
          ↓
R1-E6a Kicker / restricted-mana profiles
          ↓
R1-E6b R1 exile-play permission
          ↓
R1-E7 delayed Warp composition

All accepted capability roots + all external Shared gates
          ↓
R1-E8 CardDefinitions and recursive closure
          ↓
R1 conformance + cross-deck evidence + closure
```

The graph is a dependency proposal, not an authorization to combine the distinct mechanism families into one implementation. A batch can split into separate PRs where contract review or ownership requires it. Shared work stays in the coordinated Shared stream and must not be copied into these batches.

## 2. Preconditions and shared prerequisites

Before R1-E1 is implementation-authorized:

1. Accept the R1 Spec and record exact source/Oracle/printing identities and hashes for all 60 slots in a repository-owned deck manifest without modifying #222's list.
2. Finish Shared ownership and Specs for object/characteristics, zone incarnation/entry, permanent lifetime, spell casting and stack payload, payment/cost, target legality, trigger detection/placement/ordering, damage/life, effects/duration, turn/priority, combat/first strike, and replacement application as demanded by the complete Shared closure.
3. Resolve which current PlayerDecision/DecisionV3 closed variants can represent every cast/activation/target/optional cost/payment/combat assignment. Any new public domain or serialized continuation requires an explicit contract-growth Spec first.
4. Review new state owners for temporary effects, permission, delayed Warp scheduling, and stack/cast resolution payload. M4.2 explicitly deferred those families. Shared state/contract growth belongs to the Shared stream; R1 batches may depend on an accepted Shared contract but must not create or implement it under R1 ownership. Run the complete identity/wire compatibility audit before selecting any new contract identity or version.
5. Give each semantic capability a canonical registry entry through the accepted registry workflow. Registry lifecycle starts at `specified`; no implementation status is inferred from this plan.

Shared dependencies do not block design work in isolation, but they block integrating dependent R1 candidates and any support claim. Reclassify a family to Shared immediately if W1 closure demonstrates it requires the same semantic predicate.

## 3. Dependency-ordered batches

### Batch R1-E1 — Turn-history event production and querying

**Capabilities:** `rules/turn-history-spell-counts`; `rules/turn-history-life-and-red-damage`
**Direct witnesses:** Magebane Lizard, Hearthborn Battler, Hired Claw, Ojer Axonil / Temple of Power.
**Dependencies:** accepted Shared spell-cast, actual life-change/damage, damage-replacement, turn boundary, event and StateDelta owners.
**State:** reuse `TurnHistoryState.spells_cast_total`, `noncreature_spells_cast`, `lost_life_this_turn`, `red_noncombat_damage_dealt`; no new field.
**Decisions:** none for counting; trigger target decisions remain Shared.
**RED sequence:** event-before/after assertions; casts before source entry; creature/noncreature split; copies not cast; exact second-spell boundary; life loss then life gain; damage after replacement/prevention only; combat exclusion; source controller attribution; overflow rejection.
**Main conformance:** Magebane threshold damage; Hearthborn exactly second spell for each player; Hired Claw life-loss predicate; Ojer four-damage threshold. Prove counters change exactly once with cast/damage events and reset at the turn boundary.
**Unblocks:** R1-E2 predicates and R1-E3 Ojer replacement/activation conditions.

### Batch R1-E2 — Once-per-turn activation and Valiant target history

**Capabilities:** `rules/ability-use-limit`; `rules/first-controller-target-per-turn`
**Direct witnesses:** Hired Claw, Emberheart Challenger.
**Dependencies:** E1 turn reset; Shared ability authority/activation and target announcement/change/copy events; Shared trigger/decision/stack owner.
**State:** reuse M4.2 `once_ability_used` and `target_occurrences`; no additional state.
**Decisions:** Hired Claw activation appears among ordinary legal priority actions; there is no separate activation-decline Decision. Successful Valiant trigger resolution automatically grants the play permission; any later cast is an ordinary legal action for its authorized player, not a permission accept/decline Decision.
**RED sequence:** used ability excluded and forged activation rejected; successful commit records one use; rejected activation records none; target paired by target incarnation and targeting controller; opponent first then owner; owner repeated target; target change/copy; control changes; target incarnation departure and turn reset. For Valiant, library identity stays hidden before resolution, then its face-up exiled card becomes public to both players.
**Main conformance:** exact occurrence/trigger count and first-time target event; no targeter/source identity used as a surrogate; checkpoint/fork/replay preserve markers.
**Unblocks:** Valiant trigger resolution and Hired Claw activation closure.

### Batch R1-E3 — Ojer source-derived damage replacement

**Capability:** `rules/source-power-damage-floor`
**Direct witness:** Ojer Axonil, Deepest Might.
**Dependencies:** E1 damage-history ordering; Shared replacement application, source/controller/color/characteristic queries, LKI, actual damage and event batching.
**State:** no persistent replacement state; derive from current Ojer/source state at the defined event point.
**Decisions:** no chooser for the replacement itself; any multiple-replacement ordering choice belongs to Shared replacement policy.
**RED sequence:** below/equal/above power; red/nonred; controlled/not controlled; opponent/nonopponent; combat/noncombat; source/Ojer departure and LKI; prevention and other replacements; recipient variants; P/T changes; exact actual damage/history.
**Main conformance:** qualifying damage replaced once; nonqualifying damage unchanged; post-replacement life and damage history exact.
**Unblocks:** Ojer face ability threshold correctness and shared R1 damage spell interactions.

### External prerequisite gate — SHARED-TEMPORARY-EFFECT (not an R1 batch)

**Owner:** Shared capability/contract work, outside M4.3 R1-exclusive PR ownership.
**Scope:** specify and implement the reusable typed temporary-effect/duration and characteristic-layer substrate needed by both R1 and W1, with state, delta, event, observation, digest, checkpoint and replay implications. Do not add R1-specific Prowess/Haste profile logic in this gate.
**Gate:** accepted Shared Spec, accepted contract-growth decision where required, implementation and conformance accepted under Shared coordination. This is an external prerequisite; this R1 Plan does not authorize its PR.

### External prerequisite gate — SHARED-TRIGGER-STACK (not an R1 batch)

**Owner:** Shared capability/contract work, outside M4.3 R1-exclusive PR ownership.
**Scope:** reusable trigger detection/placement/ordering and typed trigger/stack payload required by both decks, including candidate and continuation contracts.
**Gate:** accepted Shared Spec, any required persisted/wire growth reviewed, implementation and conformance accepted under Shared coordination. This gate is required before R1 trigger-producing profiles integrate.

### Batch R1-E4 — R1 Haste and Prowess profiles

**External prerequisites:** `SHARED-TEMPORARY-EFFECT` and `SHARED-TRIGGER-STACK` are accepted and closed. W1 also depends on the generic temporary-effect/trigger owners.
**R1-specific capabilities:** `mechanic/haste`; `mechanic/prowess`; Rockface Village's +1/+0 and granted-haste profile.
**Direct witnesses:** Fanatical Firebrand, Emberheart Challenger, Hearthborn Battler, Nova Hellkite, Rockface Village.
**Dependencies:** Shared effects/duration/characteristic layers, spell-cast trigger and turn/cleanup.
**State:** no R1-owned generic effect state. Bind the R1 profile to the accepted Shared typed effect record and identity closure. Any generic state or contract growth remains Shared-owned.
**Decisions:** Rockface target choice explicit; Prowess is forced once per qualifying cast; no player selection of effect duration.
**RED sequence:** Haste affects attack and tap ability only; cannot waive other restrictions; printed vs granted haste; Prowess each noncreature cast including before resolution; not spell copies; no counters; correct end-of-turn expiry; layered P/T and target departure.
**Main conformance:** legal attack/activation candidate set changes and returns at expiry; exact public effect observations and replay parity.
**Unblocks:** printed/granted Haste and Prowess card profiles and Rockface activated effect.

### Batch R1-E5a — Trample assignment extension

**Capability:** `mechanic/trample-damage-assignment`
**Direct witness:** Ojer Axonil front face.
**Dependencies:** Shared combat declaration, blocker relation, damage assignment, first strike and player damage.
**State:** use accepted combat assignment/blocked status. If current state cannot represent all legal allocations, contract growth is required before coding.
**Decisions:** complete attacking-player assignment domain, canonical order and exact trust binding; never auto-assign.
**RED sequence:** one/multiple blockers where Shared scope permits; lethal thresholds; deathtouch interaction if in scope; zero power; exact excess; damage prevention; blocked status after blocker departure.
**Main conformance:** independent oracle proves candidate soundness and completeness; exact simultaneous damage/delta/event/replay.
**Unblocks:** Ojer combat clause; does not unblock damage replacement.

### Batch R1-E5b — Face-change action and Ojer death return

**Capabilities:** `rules/face-change-action`; `rules/return-new-incarnation-on-face`
**Direct witness:** Ojer Axonil / Temple of Power.
**Dependencies:** E1 red noncombat damage history; Shared ability payment/activation, timing, death SBA, zone incarnation, entry and face/characteristic query.
**State:** reuse FaceState and new GameObject incarnation; no additional state unless trigger/stack payload review requires it.
**Decisions:** activation is explicit; never automatically transform at threshold. Trigger placement follows Shared APNAP/trigger protocol.
**RED sequence:** exactly four and threshold unmet; sorcery vs instant-speed timing; cost/tap failure; in-place transform preserves GameObjectId; death creates old departure plus new tapped back-face incarnation; control is owner; no synthetic transform-on-entry event; checkpoint/digest/replay and observation.
**Main conformance:** Ojer round trip with exact new-incarnation face/tap event and tapped-face mana activation.
**Unblocks:** full Ojer face profile and nonbasic back-face mana ability extension. The mana ability subfamily needs a separate Shared ownership check; do not claim it under basic-land-mana merely because Temple is a land face.

### External prerequisite gate — SHARED-CAST-PAYMENT (not an R1 batch)

**Owner:** Shared capability/contract work, outside M4.3 R1-exclusive PR ownership.
**Scope:** generic casting, cost composition, player choice, mana payment/allocation and typed cast/stack payload sufficient for both decks. Kicker and Rockface restriction profiles are not part of this gate.
**Gate:** accepted Shared Spec and any required contract-growth decision, implementation and conformance accepted under Shared coordination. This R1 Plan does not authorize Shared payment or stack-payload PRs.

### Batch R1-E6a — Kicker and Rockface restricted-mana profiles

**Capability:** `mechanic/kicker`; R1 restriction profile for Shared `rules/mana-payment`.
**Direct witnesses:** Burst Lightning; Rockface Village.
**Dependencies:** accepted `SHARED-CAST-PAYMENT`, including typed shared cast/stack payload, mana pool, spell characteristic query, and a Shared-owned extension point for mana restrictions.
**State:** the paid-Kicker status is an R1-specific typed value carried by the accepted Shared cast/stack payload; any payload shape growth is Shared-owned and must be an accepted prerequisite. Rockface's restriction tag is an R1-specific profile value over existing ManaState buckets; generic restriction validation/payment execution is Shared-owned and must be accepted before this profile integration.
**Decisions:** explicitly choose kicker/decline and every complete legal mana allocation.
**RED sequence:** unpaid 2 damage, paid 4 damage, no unaffordable kick candidate, every restricted/unrestricted allocation, illegal noncreature/ability spend, stale/fabricated paid status, rejection nonmutation.
**Main conformance:** Burst Lightning target plus cost route survives stack resolution/checkpoint/replay; restricted Rockface red spent only on creature spell.
**Unblocks:** Burst Lightning's R1 Kicker profile. Rockface's restriction profile integrates only after the Shared payment owner admits and covers restricted mana enforcement.

### Batch R1-E6b — Exile play permission

**Capability:** `rules/temporary-exile-play-permission`
**Direct witnesses:** Emberheart Challenger; Nova Hellkite later via Warp.
**Dependencies:** Shared exile/zone incarnation, casting/payment, permission validation, player knowledge/projection, E2 Valiant trigger.
**State:** new typed permission family likely requires contract growth and exact expiry semantics.
**Decisions:** permission is granted automatically on successful effect resolution. The exact card appears as an ordinary legal cast/play action only for its authorized player while the permission and normal timing/cost/entitlement rules allow it; not playing it is represented by choosing another legal action or passing, with no separate decline-permission Decision.
**RED sequence:** only exact card/player/action authorized; Emberheart permission ends at turn end; Warp permission begins only after the Warp-cast turn ends and has no turn-based expiry while the exact exile relation remains; card leaves exile; both players see face-up exile identity but only the authorized player sees the playable candidate; hidden library identity is noninterfering before public exile; permission present in checkpoint/fork/replay; fabricated/stale use rejected.
**Main conformance:** Emberheart exiles the top card face up; both perspectives observe the same public card identity after resolution, while only the authorized player's complete decision domain offers to play it. Before resolution, paired hidden library identities remain noninterfering.
**Unblocks:** Emberheart permission closure and Warp's permission half.

### Batch R1-E7 — Delayed effect and Warp composition

**Capabilities:** `rules/delayed-effect-scheduling`; `mechanic/warp`
**Direct witness:** Nova Hellkite.
**Dependencies:** E6b permission; E6a cast/payment; Shared turn/end-step scheduler, exile incarnation, trigger/effect and replay.
**State:** typed due-turn/step payload and exact card reference, contract growth required if absent from the accepted execution closure. Source departure must not erase it.
**Decisions:** Warp vs ordinary cast route and complete payment route are explicit at casting. Warp permission is granted automatically when its delayed effect resolves; a later cast is an ordinary legal action while the exact permission is valid, with no separate permission accept/decline Decision.
**RED sequence:** Warp unavailable/unaffordable; source permanent leaves before the due trigger; no fallback to a later incarnation; next end step only; permission unavailable until the Warp-cast turn ends; permission remains available across later turns only while the exact exile incarnation remains exiled; exact incarnation/face/zone; restore/fork/replay across pre-due and post-due boundaries.
**Main conformance:** cast via Warp, resolve, original permanent leaves before due and produces no exile/permission; in the positive case, next-end-step exile and later permission bind to exact incarnations, remain valid while exiled, and cease when that card leaves exile. Each boundary has exact event/state identity.
**Unblocks:** Nova Hellkite's full Warp definition.

### Batch R1-E8 — CardDefinition admission and R1 recursive closure

**Scope:** after capability roots and Shared closure are accepted/implemented/covered, author definitions for Fanatical Firebrand, Hired Claw, Magebane Lizard, Emberheart Challenger, Razorkin Needlehead, Hearthborn Battler, Ojer Axonil, Nova Hellkite, Burst Lightning, Lightning Strike, and Rockface Village. Mountain reuses the M4.2 definition.
**Dependencies:** exact source manifest; accepted CardDefinition contract; every referenced face, ability, characteristic and profile; capability admission.
**State/decisions:** definition data only; no embedded semantics, opaque strings, auto-selected targets or default modes.
**RED sequence:** bad provenance, wrong Oracle record/printing, missing face/ability/capability, unsupported node, duplicate or unresolved profile reference; fail closed at definition/profile preflight.
**Main conformance:** recursively derive each definition's complete capability roots and compare to approved closure; all 60 slots resolve exactly; missing/extra/unsupported dependencies reject.
**Unblocks:** R1 card-specific conformance matrix and bundle-level interaction work; it does not certify R1.

## 4. Files and modules likely affected

Exact paths depend on accepted design, but implementation review should inspect/update only relevant existing owners, likely:

* `crates/mtgml-state/` — typed effect/permission/delayed/stack payload state only after state-owner and identity review; existing `mana`, `turn_history`, `face`, counters and authority state reused.
* `crates/mtgml-rules/` — typed cast, payment, target, trigger, damage replacement, combat assignment, activation, turn-history producer and transform transition owners; do not create card-name handler modules.
* `crates/mtgml-decision/` — only reviewed closed domains/answers/ordering/bindings required for options, costs, target sets, payment and assignment.
* `crates/mtgml-environment/` and `crates/mtgml-observation/` — atomic product validation, safe player projection, hidden-zone knowledge and candidate/event redaction.
* `crates/mtgml-persistence/`, `crates/mtgml-replay/` — only if accepted state/event/decision identity changes; maintain one current writer and historical readers per accepted policy.
* `crates/mtgml-card-ir/`, `cards/definitions/`, `cards/capabilities/registry.json`, and generated catalog/vocabulary artifacts — content definitions and authoritative registry/generator source. Never hand-edit generated output.
* Rust and Python public DTOs, schemas, golden/negative fixtures, conformance catalog, and documentation only when a reviewed public/persisted contract actually changes. Python remains rules-free.

## 5. Contract and lifecycle transitions

* Before implementation: capabilities remain `proposed` in this planning document; no registry lifecycle change.
* After accepted capability Specs/registry source: `specified` only.
* After production behavior merges with exact acceptance evidence: `implemented` only for the admitted bounded semantics.
* After independent RED/green conformance, interaction, privacy, and replay gates pass: `covered` for precisely that scope.
* `certified` remains unavailable until the full exact R1 × W1 bundle closure and M4 certification gate passes.
* Each state/payload extension triggers a source-of-truth compatibility audit across EngineStatePartsV2, StateDeltaV2, authoritative and observed events, PlayerStepV3, FullStateDigestV6, CheckpointV7, Replay V7, content contract, semantic contract, capability catalog, observation DTOs and schemas. If any closed identity is insufficient, mark `CONTRACT_GROWTH_REQUIRED`, stop the dependent batch, and write a separate accepted contract-growth Spec. No version number is selected here.

## 6. RED-first quality sequence per batch

1. Add failing rule-level tests against a small independent reference oracle for the exact bounded family and all boundary cases.
2. Add rejection/fabrication/overflow tests before implementation; assert full semantic fingerprint and player bytes are unchanged.
3. Add exact state, event order, StateDeltaV2 full reapplication and digest assertions.
4. Add decision soundness and completeness independently, canonical candidate ordering, trusted binding and stale-answer rejection.
5. Add information noninterference and perspective-specific observed-event tests.
6. Add checkpoint restore, fork and Replay V7 parity at every newly authoritative boundary.
7. Add property/fuzz tests for bounded values, identity lifecycle, event sequences and canonical encodings.
8. Add interaction regressions with already accepted Shared/M3/M4.2 semantics before promoting the capability lifecycle.

Do not author CardDefinitions merely to drive a mechanic implementation. Use typed test fixtures until the capability profile and contract are accepted; then add definitions in R1-E8.

## 7. Expected PR decomposition

The following are semantic batches, not card PRs. Split any batch at an identity-cut boundary or distinct owner review. Shared changes must land under Shared coordination before dependent R1 merges.

1. **SHARED-CAST-PAYMENT** external Shared-owned casting/payment/cost and typed cast/stack payload contract/implementation/conformance gate; it is not an R1 PR.
2. **SHARED-TRIGGER-STACK** external Shared-owned trigger/stack contract/implementation/conformance gate; it is not an R1 PR.
3. **R1-E1a** spell-cast history event production and count consumers.
4. **R1-E1b** actual life-loss/red-noncombat-damage history producers and consumers.
5. **R1-E2a** Hired Claw once-use activation predicate.
6. **R1-E2b** Emberheart target occurrence/Valiant trigger predicate.
7. **R1-E3** Ojer noncombat damage floor replacement.
8. **SHARED-TEMPORARY-EFFECT** external Shared-owned contract/implementation/conformance gate; it is not an R1 PR.
9. **R1-E4** R1-only Haste/Prowess profile integration after the external gate passes.
10. **R1-E5a** trample assignment decision and damage semantics after Shared combat prerequisites.
11. **R1-E5b** Ojer transform action and return-as-new-incarnation entry profile after Shared activation, entry and trigger prerequisites.
12. **R1-E6a** R1 Kicker and restricted-mana profiles after `SHARED-CAST-PAYMENT`; any generic restriction/payment code change remains in the Shared stream.
13. **R1-E6b** R1 exile-play permission state and Emberheart profile.
14. **R1-E7** R1 delayed Warp schedule/profile after E6b and Shared trigger/zone/turn prerequisites.
15. **R1-E8a** CardDefinitions for the ten nonbasic R1 permanent/spell identities except Rockface if its profile dependency is not ready.
16. **R1-E8b** Rockface Village and final exact 60-slot recursive closure after Shared restricted-mana enforcement is accepted.
17. **R1 conformance closure** exact card and R1-internal interaction matrix; lifecycle promotion only at evidence boundaries.

Do not combine E3 Ojer replacement with E5 trample just because they share Ojer. Do not combine E6 payment/permission or E7 scheduling. Each has a separate state/decision contract.

## 8. Repository gates

Use repository-defined gates after authorization and only on the exact source identity under review:

* `just check-fast` during each implementation batch, after source generation is synchronized.
* `just check` before independent review for each integrated batch.
* `just check-all` for each state/wire/replay/information boundary, broad semantic interaction, or final R1 closure.
* Applicable conformance, paired-state noninterference, property/fuzzing, checkpoint/fork/replay, and exact-source/provenance gates from `docs/TESTING_AND_CONFORMANCE.md`, `docs/testing/CONFORMANCE_AUTHORING.md`, `docs/testing/NONINTERFERENCE_TESTING.md`, `docs/testing/PROPERTY_AND_FUZZING.md`, and `docs/contracts/ACCEPTANCE_GATES.md`.
* `just release-candidate` only at an explicitly authorized release/certification candidate; it is not a gate for this design-only task.
* Generated-contract drift/schema parity for every accepted contract change; source/archive reproducibility last, after all source edits.

This plan has run no tests or repository gates. Any later report must distinguish `PASS`, `FAIL`, `NOT_RUN`, `BLOCKED`, and `EXPERIMENTAL` exactly.

## 9. Stop conditions

Stop the affected batch and return to Spec/architecture review if:

* an R1 capability is found in W1 closure or needs a Shared owner not accepted;
* accepted generic capability already owns the semantics and the proposal would duplicate it;
* exact source/printing/deck identity remains ambiguous;
* required state lacks an accepted owner or contradicts M4.2 closure;
* a meaningful choice cannot be represented with a complete/sound Decision domain;
* player-safe projection/noninterference cannot be proved;
* any required state/event/delta/observation/decision/replay/schema contract cannot carry the semantics;
* generated artifacts, pinned tools, or required exact-head gates are unavailable;
* an interaction requires unsupported game setup, timing, trigger, replacement, stack, or zone semantics.

Never paper over a stop condition with a card-specific handler, hidden semantic state, auto-choice, guessed printing, broadened support statement, or test weakening.

## 10. Must remain unchanged

Until the explicit future implementation authorization: production Rust, Python runtime, schemas, CardDefinitions, capability registry, deck lists, branches for production implementation, and PR state remain unchanged. For the eventual work, preserve locked R1 and W1 card names/counts/text/source provenance; M4.2 historical identities and support scope; one RulesKernel authority; deterministic state closure; perspective boundaries; fail-closed behavior; and all historical replay/checkpoint meanings. No M4.3 implementation begins by accepting these proposed documents alone.
