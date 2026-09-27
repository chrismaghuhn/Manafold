# M4.3 R1-Exclusive Capability Semantic Specification

**Task:** `M4_3_R1_EXCLUSIVE_CAPABILITY_SPEC_AND_IMPLEMENTATION_PLAN`
**Status:** PROPOSED / DESIGN ONLY / NOT ACCEPTED
**Baseline:** fetched `origin/master = 6c6ee4c9b237696c50e944cae998f85c2d358e1c`
**Production implementation authorized:** NO
**Date:** 2026-09-27

## 1. Authority and scope

This document specifies the semantic work that appears unique to the locked R1 Mono-Red deck after the M4.2 Mountain/Plains state cut. It is subordinate to the accepted architecture, the 2026-09-25 Comprehensive Rules snapshot, the accepted M4.2 state-cut Spec, and the exact card Oracle identities/text pinned for eventual content admission. It is a proposal for independent review; it does not authorize implementation or alter a lifecycle.

M4.3 full R1 closure remains:

```text
accepted Shared capability closure
+ accepted R1-exclusive capability closure
+ reviewed R1 CardDefinitions
+ R1 conformance and evidence
```

This Spec owns no Shared capability and no W1 work. Shared dependencies below are prerequisites, not work to duplicate here. When an R1 clause and a W1 clause depend on the same generic semantic owner, ownership remains Shared; R1-specific parameters or cases can be added there after coordinated review.

### Verified baseline and change since #237

The fetched `origin/master` is exactly the task's expected SHA. It contains the accepted M4.2 unified state cut and its Phase 13 activation; M4.2 is complete for Mountain and Plains under `basic-land@1.0.0`. This advances #237's inspected baseline (`b4122f4…`): the three M4.2 capability roots are now in `cards/capabilities/registry.json` with lifecycle `specified`, and the six accepted state families are current runtime state. This changes state-substrate assumptions, not card support. The M4.2 Spec explicitly disclaims payment, casting, general stack, general spells, Lightning Strike, triggers, Ojer, and all other R1/W1 behavior.

The registry currently has 14 entries: the 11 Foundation entries plus the M4.2 roots `rules/basic-land-mana@0.1.0`, `rules/land-play@0.1.0`, and `rules/mana-pool@0.1.0`. Those three M4.2 entries currently have lifecycle `specified`; this proposal does not promote their lifecycle or create registry entries. The accepted M4.2 runtime/state substrate remains bounded to its stated Mountain/Plains scope.

The status surfaces are still inconsistent at this baseline: README and ROADMAP say `M4.2 = IN_PROGRESS`, while #225's final acceptance comment reports `M4.2 = COMPLETE` for the bounded Mountain/Plains slice. This R1 proposal does not resolve that repository status drift. Merge the required post-M4.2 status sync before accepting this Spec; the 14-entry registry count and the scoped #225 completion statement above do not silently rewrite those files.

### Locked R1 deck and identity boundary

Use exactly the name/count list fixed by #222: 4 Fanatical Firebrand; 4 Hired Claw; 4 Magebane Lizard; 4 Emberheart Challenger; 4 Razorkin Needlehead; 4 Hearthborn Battler; 4 Ojer Axonil, Deepest Might; 4 Nova Hellkite; 4 Burst Lightning; 4 Lightning Strike; 16 Mountain; 4 Rockface Village. Mountain is the M4.2 closed slice and is not decomposed again here. No quantity, card, printing, or text substitution is authorized.

No repository-owned R1 deck manifest or exact printing IDs exist at this baseline. #222 fixes card names/counts and #237 records its coordination matrix; the persisted 2026-09-15 research report is explicitly non-normative. The M4.2 audit pins the Scryfall Oracle Cards snapshot `oracle-cards-20260925210158.jsonl.gz` (SHA-256 `c607300fe03ce0d9f59181b1bb33d8001e2fa339b8c68eefd70a6501fa757623`) and Rulings snapshot `rulings-20260925210032.jsonl.gz` (SHA-256 `375fb0ef1d3338055bb406930b76813b8609256cd7c9592ead940984bf59d520`), and states that all 26 normalized matchup identities were rechecked. Those snapshots ground this clause analysis, but exact printing/source provenance and per-record digests must be captured in reviewed definitions before content admission. No card support is claimed here.

Rules authority is the first-party Comprehensive Rules artifact effective 2026-09-25, SHA-256 `8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca`, already pinned by the M4.2 Spec. Relevant rules include CR 106 (mana and restrictions), 113/115 (abilities and targets), 117/601/602/603 (priority, casting, activation, triggers), 120/614 (damage/replacement), 302 (creatures/combat), 500.4 (turn boundary), 701.28/712 (transform and DFC zones), and the card-specific rules cited in each capability's eventual reviewed design.

## 2. Lifecycle language

For every R1 card, distinguish these facts:

```text
source imported      = source bytes/identity preserved
parsed               = source decodes under a content schema
definition possible  = a well-formed CardDefinition can be authored
capability specified = a reviewed semantic contract exists
implemented          = production transition authority executes it
covered              = required executable conformance/evidence passed
certified            = an exact locked bundle passed its certification gate
```

This proposal establishes none of the last four lifecycle promotions. State presence, parsing, examples, or a capability row is not card support.

## 3. Clause-level R1 decomposition

The clauses below summarize the pinned Oracle snapshot. They are semantic work items, not substitute Oracle text. Source identity, exact text, characteristics, faces, mana costs, and provenance belong in future CardDefinitions. “Shared dependency” means the actual Shared owner must be accepted/closed before the R1 path can use it.

| Card / clause | Classification and obligations |
|---|---|
| **Fanatical Firebrand** — Haste | Card keyword data plus R1-exclusive haste semantics, reusing Shared creature control-duration/attack eligibility. It affects attack eligibility and its `{T}` ability's summoning-sickness restriction. No extra state solely for haste if the accepted temporary-effect representation can express the intrinsic keyword and ongoing control duration. |
| — `{T}, Sacrifice: deal 1 damage to any target` | CardDefinition activated-ability/cost/effect data; Shared ability activation, sacrifice/zone-incarnation, targeting, stack/resolve, and damage capabilities. Target domain includes any legal player or creature as applicable at declaration and is revalidated at resolution. Activation is a decision; sacrifice and tap are atomic costs; source departure does not erase the stack ability. No R1-exclusive root established by this clause. |
| **Hired Claw** — attack with one or more Lizards; deal 1 to target opponent | Card type/tribal predicate is content data. Shared attack event and trigger detection/placement/resolution, targeting and damage. “One or more” yields one trigger for the event, not one per Lizard. Its target is an opponent, selected through the normal complete target domain. |
| — `{1}{R}: put a +1/+1 counter` | Card ability data; Shared activation/payment/tap-or-cost machinery as applicable, counter mutation, and priority. Counter kind/count substrate exists in M4.2 but counter operation does not thereby become supported. |
| — opponent lost life this turn; once each turn | R1-exclusive history predicates and usage-limit rule. Reuse M4.2 `TurnHistoryState` (`lost_life_this_turn`, source-incarnation/AbilityKey once-use set); implement actual life-loss event production and activation gating. Life gain later does not clear the bit. Ability-use key is trusted and never projected. |
| **Magebane Lizard** — whenever a player casts a noncreature spell, damage that player equal to their noncreature spells cast this turn | Card trigger/data plus Shared spell-cast, trigger, targetless damage, and damage replacement pipeline. R1-exclusive noncreature spell-count semantics consume the existing M4.2 history field. Count includes prior casts before this Lizard entered and the triggering cast; copies not cast do not count. Evaluate at the event boundary before trigger resolution. No player choice in the trigger amount. |
| **Emberheart Challenger** — Haste | Same reusable haste family as Fanatical Firebrand; no separate card handler. |
| — Prowess | R1-exclusive trigger condition/effect profile, depending on Shared spell-cast detection and the Shared temporary-effect mechanism also needed by W1. Trigger once for each qualifying noncreature spell cast, including a cast that later fails to resolve; +1/+1 until end of turn is a continuous temporary modification, not counters. |
| — first time each turn it becomes target of a spell/ability its controller controls: exile library top; may play it until end of turn | R1-exclusive Valiant event predicate uses M4.2 `TurnHistoryState` target-occurrence set (target incarnation, targeting controller). Shared target announcement/change/copy events and trigger processing. Re-targeting and copies count as new becomes-target events. The trigger's optional “may play” is a player decision; the permission is limited to that exiled card and this turn and composes with Shared normal play/cast restrictions. The card is exiled face up by default (CR 406.3), so its identity becomes public when the Valiant ability resolves; only the play entitlement is limited to its authorized player. |
| **Razorkin Needlehead** — first strike during your turn | Card keyword data plus Shared combat damage step scheduling/assignment/damage and active-turn predicate. M3 combat damage covers one normal step only; first-strike step expansion is required and is Shared with any W1 first-strike witness found during Shared closure (none established by #237's candidate list). Candidate identity must be rechecked against the entire locked W1 closure before ownership is accepted. |
| — opponent draws: deal 1 to that opponent | Card trigger data, Shared draw event/trigger placement and noncombat damage. Trigger is one per opponent draw event; no target choice. |
| **Hearthborn Battler** — Haste | Same reusable haste family. |
| — a player casts their second spell each turn: deal 2 to target opponent | Shared spell-cast and trigger infrastructure plus R1-exclusive per-player current-turn total-count predicate, using `TurnHistoryState.spells_cast_total`. Exactly the second cast event for each player each turn qualifies, not every later spell. Target is chosen from legal opponents when the trigger is put on the stack; it does not silently select. |
| **Ojer Axonil, Deepest Might / Temple of Power** — front-face Trample | R1-exclusive trample assignment semantics, extending Shared combat-damage assignment and decision domains. Its applicable combat damage can be assigned to defending player only after lethal damage is assigned to each blocker under current rules. |
| — red sources you control would deal less noncombat damage to an opponent than Ojer's power: replace with Ojer's power | R1-exclusive source-derived damage replacement. Derive eligibility from live source color/controller and current Ojer face/power; replace only qualifying noncombat damage to opponents when damage would be dealt. No persistent replacement registry/state is justified. Apply before actual damage and history recording; handle source departure/LKI as required by the replacement event rules. This is distinct from W1 Dryad Militant's exile replacement. |
| — when Ojer dies, return under owner's control tapped and transformed | R1-exclusive death-trigger/effect profile; Shared death/SBA, zone incarnation, trigger and permanent-entry capabilities. The returning object is a **new incarnation** already tapped on the back FaceKey; do not model a front-face entry followed by transform. Existing `FaceState` is substrate only. |
| — back-face `{T}: add {R}` | Reuse M4.2 basic-land-mana only if its accepted profile explicitly covers this nonbasic face; current contract is basic subtype-derived basic land mana, so this is an extension or a distinct Shared mana-ability scope. Ability tap and public mana event required. |
| — `{2}{R}, {T}: transform`, only if red sources you controlled dealt 4+ noncombat damage this turn and only as a sorcery | R1-exclusive transform activation legality, face mutation, mana payment and sorcery timing (Shared cast/priority/timing). Reuse `TurnHistoryState.red_noncombat_damage_dealt`; count actual damage dealt after replacement/prevention, exclude combat, and attribute source controller at damage time. The activation is player-controlled; do not auto-transform. Existing `FaceState` represents the face but does not implement transform. |
| **Nova Hellkite** — Flying, Haste | Flying is a reusable keyword/characteristic rule needed by other W1 cards and belongs to Shared. Haste uses the R1-exclusive family above. |
| — ETB: deal 1 to target creature an opponent controls | Shared entry trigger, trigger placement, target selection/revalidation, damage, and opponent-controlled creature legality. Target choice is mandatory when legal; if none legal use the rules-defined no-target behavior. |
| — Warp | R1-exclusive alternative cast cost and delayed exile at the next end step, followed by permission for its owner to cast that card after the current turn has ended for as long as it remains exiled (CR 702.185a). Requires Shared spell casting/payment, a due-step delayed effect bound to the exact battlefield incarnation the spell becomes, zone incarnation, and permission bound to the exact resulting exile incarnation. If that battlefield incarnation leaves before the delayed trigger resolves, it does not exile a later incarnation of the same physical card and grants no Warp permission. Warp and ordinary cast routes are distinct explicit choices when both are legal. |
| **Burst Lightning** — Kicker `{4}`; may pay the Kicker cost as cast; deal 2 or 4 if kicked | Shared cast/payment/target/damage/stack semantics plus the R1-exclusive Kicker choice and preserved typed Kicker-paid status. “Kick or decline” must be explicit where both are legal; all legal mana allocations are exposed. Do not infer paid status from total mana or mutate it after casting. |
| **Lightning Strike** — deal 3 to any target | Shared instant casting, target selection, stack, target legality at resolution, and damage. Target choice is mandatory from the complete legal domain; empty legal target set follows rules. No R1-exclusive capability identified. |
| **Rockface Village** — `{T}: add {C}` | M4.2 mana state plus a Shared generic land mana-ability path. `{C}` is colorless, not generic cost. |
| — `{T}: add {R}`, spend only to cast creature spell | R1-specific restriction tag production, but it extends the Shared mana payment/cost capability required by both decks. Preserve tags per unit; enumerate all legal payment allocations; restricted units only pay a creature spell, not noncreature spell/ability. This restriction is R1-only; payment ownership remains Shared. |
| — `{R},{T}: target Lizard/Mouse/Otter/Raccoon you control gets +1/+0 and haste until EOT`, sorcery timing | Shared activation, target visibility, timing, and typed temporary-effect infrastructure; R1-specific creature-type predicate and effect profile. Target selection is explicit and complete. Control and legal type are checked at activation and target legality at resolution. The granted haste changes attack and tap eligibility through turn end. |

## 4. Research-candidate reconciliation

Issue #237 is coordination evidence only. Its identifiers are provenance labels, not canonical capability keys. Canonical names below are proposed semantic names for review; none is admitted to the registry or assigned a version here.

| Research ID / #237 purpose and direct witnesses | Disposition at verified master | Proposed semantic identity / owner | Dependencies and rationale |
|---|---|---|---|
| C30 — enforce use tags on produced mana; R12 | **R1-exclusive requirement; extend Shared payment** | `rules/mana-restriction-enforcement` (new semantic family, Shared payment owns execution) | ManaState stores creature-spell-only buckets but M4.2 expressly excludes payment. W1 also needs generic payment, but not this restriction. Never create a parallel payment engine. |
| C46 — delayed trigger at future boundary; R08 | **R1-exclusive new reusable capability** | `rules/delayed-effect-scheduling` | Nova Warp needs next-end-step exile then later-turn permission; M4.2 explicitly defers DelayedEffectState. Depends on Shared triggers and turn boundary. No W1 delayed-effect witness was found in the locked list. |
| C47 — per-player spell/noncreature spell counts; R03,R06 | **R1-exclusive event/history extension** | `rules/turn-history-spell-counts` | Existing fields `spells_cast_total` and `noncreature_spells_cast` are state only; M4.2 defines semantics for producers/consumers but does not implement general casting. W1 has no requirement for second-spell or noncreature-spell count predicates. |
| C49 — turn-scoped actual life loss and qualifying noncombat damage; R02,R07 | **R1-exclusive event/history extension** | `rules/turn-history-life-and-red-damage` | Reuse `lost_life_this_turn` and `red_noncombat_damage_dealt`; exact actual-event producers and ordering are needed. W1’s current locked clauses do not consume either predicate. |
| C51 — once-per-turn ability use; R02 | **R1-exclusive extension to Shared activation** | `rules/ability-use-limit` | Reuse TurnHistoryState `(GameObject incarnation, AbilityKey)` set and AbilityAuthorityState. Hired Claw alone requires this limit; W1 activations do not make this predicate Shared. |
| C52 — first qualifying own target per turn; R04 | **R1-exclusive history-sensitive trigger predicate** | `rules/first-controller-target-per-turn` | Reuse target-occurrence substrate, Shared target/trigger events. W1 has targeted spells/ETBs but no first-own-target-this-turn predicate. |
| C60 — raise qualifying noncombat damage to live source-dependent threshold; R07 | **R1-exclusive new damage replacement rule** | `rules/source-power-damage-floor` | New rule family using Shared damage/replacement application and characteristic derivation. Not Dryad Militant’s replacement; no new authoritative replacement state. |
| C63 — allow specific exiled card to be played under ordinary constraints; R04 | **R1-exclusive permission family** | `rules/temporary-exile-play-permission` | Emberheart's same-turn permission, and Warp's later-turn permission with different duration. M4.2 defers PermissionState. W1 linked exile is not a play-from-exile permission. |
| C64 — change admitted faces without zone change; R07 | **R1-exclusive face-change action** | `rules/face-change-action` | Reuse existing FaceState and CardDefinition FaceKey; implement rules timing/cost/condition for Temple of Power. W1 Origin of Spider-Man is a Saga, not a transforming DFC; no W1 transform action found. |
| C65 — return tracked departed transforming card on specified face/entry state; R07 | **R1-exclusive entry-effect extension** | `rules/return-new-incarnation-on-face` | Reuse zone incarnation, face, tap and entry machinery. Ojer's death trigger creates tapped back-face new incarnation. No W1 return-transformed witness found. |
| C78 — haste waives control-duration restrictions; R01,R04,R06,R08,R12 | **R1-exclusive reusable keyword family** | `mechanic/haste` | Haste modifies attack and tap-symbol ability eligibility. Reuse Shared timing/control-duration queries; no haste witness in the locked W1 set established. Temporary granted haste on Rockface Village must use Shared temporary-effect owner. |
| C83 — trample excess damage after lethal assignment; R07 | **R1-exclusive reusable combat keyword extension** | `mechanic/trample-damage-assignment` | Extend Shared combat damage/assignment candidate domains and evidence. No trample witness in W1's locked set found. |
| C88 — prowess +1/+1 until end of turn on noncreature spell cast; R04 | **R1-exclusive trigger/effect profile; Shared effect substrate dependency** | `mechanic/prowess` | Depends on Shared cast trigger events and temporary-effect machinery. Do not make a second effect system; W1 also requires temporary effects (for example, end-of-turn grants), so the substrate is Shared. |
| C89 — Kicker additional cost and paid/unpaid resolution branch; R09 | **R1-exclusive Kicker mechanic** | `mechanic/kicker` | Shared casting/payment owns general cost composition, choice enumeration, and payment. Capture the R1-specific typed Kicker-paid status in the cast/stack payload for Burst Lightning's resolution branch. No W1 Kicker witness found; W1 Blight is a distinct activation-cost mechanic and does not justify a generic additional-cost capability here. |
| C90 — Warp from-hand alternate cost, delayed exile, later cast permission; R08 | **R1-exclusive composed mechanic** | `mechanic/warp` | Composes Shared casting/payment, zone change, turn scheduling, C46 delayed effect and C63 permission; no separate Warp interpreter or card-name branch. No W1 Warp witness. |

`R1_EXCLUSIVE_CONFIRMED = 15` research rows, subject to reviewed canonical identity and Shared dependency acceptance. None is already semantically satisfied by M4.2; its history, face, ability-use and mana fields are substrate only. A planning disposition split is 7 extensions/reuses of existing state or Shared semantic owners (C30, C47, C49, C51, C52, C64, C83) and 8 new reusable R1 semantic identities (C46, C60, C63, C65, C78, C88, C89, C90), with C89 narrowed to `mechanic/kicker`. These buckets describe the row's primary implementation path; the new identities still depend on existing Shared owners and the state substrate. None is “not actually required” on the locked R1 clauses. `R1_RECLASSIFY_TO_SHARED = 0` for the exact R1-only semantic predicates/effects in these rows. Shared roots appearing as dependencies (cast, payment, trigger, targeting, combat, effects) remain outside this Spec. If Shared closure discovers a W1 witness for a listed predicate, stop and reclassify before implementation.

## 5. Proposed R1-exclusive capability contracts

The identities below are semantic names, not registry keys yet. Each eventual capability Spec must assign a reviewed version, enumerate all admitted profiles, and freeze positive/negative cases before implementation.

### 5.1 Event-derived turn history consumers

`rules/turn-history-spell-counts` and `rules/turn-history-life-and-red-damage` consume typed authoritative events and update only the already closed finite `TurnHistoryState` fields. The event producer is the operation's semantic owner: a spell-cast event increments spell count exactly once; actual loss of life sets the persistent turn bit; qualifying red noncombat damage increments only after applicable replacement/prevention and when dealt. Counts use checked arithmetic. History carries across source departure and every Decision boundary and resets atomically at the turn boundary. Rejected operations mutate none of state, event sequence, digest, replay, or RNG.

No arbitrary history map, Oracle-string key, inferred net-life delta, or reconstructed history is allowed. The turn ledger's current turn number must match CoreRulesState.

### 5.2 Hired Claw activation limit and Emberheart first-target predicate

`rules/ability-use-limit` evaluates a trusted source incarnation and immutable AbilityKey at activation. A use is recorded only with successful activation commit; duplicate activation in the same turn is absent from the legal domain and rejected if fabricated. The record is removed by the existing turn reset and source-incarnation lifecycle.

`rules/first-controller-target-per-turn` sees each actual becomes-target event, including target change and spell copy. It compares target creature controller and targeting spell/ability controller at that event; only an own spell/ability targeting its controller's creature can trigger Valiant. Its occurrence marker is recorded once by canonical pair, not by targeting source object. Target/control changes are handled from event-time facts. Existing state substrate does not supply trigger detection, trigger placement, or the resulting decision.

### 5.3 Ojer damage replacement and face transition

`rules/source-power-damage-floor` is a source-derived replacement rule evaluated against the live Ojer face/power and damage source color/controller when a proposed noncombat damage event would affect an opponent. It replaces the proposed amount only when the source is red, controlled by the Ojer controller, and amount is below Ojer power. Combat damage and damage to non-opponents are unchanged. Apply relevant replacement/prevention ordering before the actual damage event and C49 history update. Because this is static/source-derived, it adds no persistent ReplacementState.

`rules/face-change-action` changes the current FaceKey on the same battlefield GameObject incarnation only when the back-face ability cost, sorcery timing and red-noncombat-damage threshold are satisfied. Ojer's death trigger is a separate transition: old incarnation leaves; a new owner-controlled incarnation enters tapped with the back FaceKey already selected. It is not a transform action. Transform/removal/return behavior must agree with CR 701.28 and 712 and the M4.2 FaceState contract.

### 5.4 Haste, trample and Prowess

`mechanic/haste` removes only the summoning-sickness restrictions on attacking and activating abilities with `{T}` for the applicable creature. The ability is derived from immutable printed keyword or an active effect; it does not bypass other attack/activation restrictions.

`mechanic/trample-damage-assignment` extends Shared combat damage: the attacker assigns lethal damage to each blocker in order before assigning excess to the defending player/planeswalker/battle as allowed. Any ordering/allocation that rules leave to the attacking player is a complete decision domain; no deterministic first blocker or heuristic allocation is allowed.

`mechanic/prowess` triggers on each noncreature spell cast by its controller and creates the defined end-of-turn +1/+1 continuous effect. It is not a counter and does not trigger on a spell copy that was not cast. Temporary effect state and duration machinery is Shared because W1 also requires temporary effect behavior; Prowess-specific trigger and layer parameters remain R1-specific.

### 5.5 Kicker and mana restrictions

`mechanic/kicker` contributes only the optional `{4}` Kicker cost and its paid/unpaid status for Burst Lightning. The chosen status and resulting resolution branch are frozen in typed cast/stack payload; the Kicker decision is explicit where both routes are legal. Shared casting/payment owns general cost composition, optional-cost decision plumbing, payment enumeration, and mana allocation for both decks. Every legal allocation is canonical; no automatic payment and no generic AdditionalCost capability is introduced by this R1 proposal.

`rules/mana-restriction-enforcement` associates Rockface's red restricted unit with creature-spell-only eligibility until spent or pool expiry. The M4.2 `ManaState` bucket is substrate; not payment support. Spending eligibility is determined by the cast spell's rules characteristics, not a string or card name. Rejection of a restricted unit in any other cost is atomic.

### 5.6 Exile play permissions and Warp scheduling

`rules/temporary-exile-play-permission` represents a grant for one exact exiled object incarnation, authorized player, permitted cast/play action, cost context and duration. Emberheart permission lasts until end of turn. Its top card is exiled face up, so its identity is public to both players as soon as the ability resolves (CR 406.3); the permission authorizes only its specified player to play it. Permission state and trusted bindings remain hidden, and only the authorized player's complete legal decision domain includes the permitted action. It preserves all ordinary timing, land entitlement, casting, payment and target requirements.

`rules/delayed-effect-scheduling` stores a typed Warp payload due at the next end step, bound to the exact battlefield `GameObjectId` incarnation the resolving spell becomes. At the due boundary it exiles only that still-present incarnation and creates permission for its owner to cast the resulting exact exile incarnation after the current turn has ended for as long as that card remains exiled (CR 702.185a). If the battlefield incarnation leaves before the delayed trigger resolves, the trigger cannot follow its `PhysicalCardId` to a later incarnation; no Warp exile or permission occurs. The permission lifetime predicate is `eligible_after_turn > warp_cast_turn && card remains in the exact warp exile relation`; `eligible_after_turn` is the next turn identifier, and the action is available only when `current_turn_id >= eligible_after_turn`. It has no turn-based expiry. The delayed trigger survives departure of the original spell/source as rules require and persists through checkpoint/restore/fork/replay.

`mechanic/warp` composes the above with Shared alternate-cost casting. It offers Warp or ordinary casting as distinct choices when both legal, records the selected cost route, does not retain the card in a forbidden zone, and preserves exact turn/step due semantics.

## 6. Decisions and completeness

No semantic family may silently choose on a player's behalf. The eventual Decision Specs must identify the current rules-owned domain and ensure the player DTO is a complete, canonical projection with trusted hidden bindings.

| Decision family | Owner / visible intent | Required completeness and rejection |
|---|---|---|
| Spell/ability targets (Firebrand, Hired Claw, Nova, Burst Lightning, Lightning Strike, Rockface) | Shared targeting rules produce legal target subsets/choice; player sees perspective-safe opaque targets and target role | Every legal target appears once in canonical order. Hidden/fabricated, stale, wrong-kind or wrong-controller targets reject atomically. Revalidate at resolution; no target fallback. |
| Optional kicker / Warp route / play permission | Shared cast Decision with typed R1 optional-cost/permission alternatives | Include every payable legal route, including decline when legal. Bind exact typed route and paid status. Stale/fabricated route rejects. Never auto-pay/auto-Warp. |
| Mana allocation | Shared payment owner; visible colored/restricted cost and own available pool | Enumerate all legal allocations and preserve restriction units; canonical order independent of insertion/global IDs. No auto-pay. |
| Hired Claw ability activation | Shared activation domain plus R1 once-use legality | Activation/decline remains normal priority control; once-used activation is excluded and forged submit rejects. Trusted AbilityInstanceId remains hidden. |
| Emberheart “may play” permission | Rules trigger and cast domain; player sees authorized card only if it is their own known card | Explicit play/decline while legal; only exact granted card is bound. Other players' observations, event bytes, errors and candidate sets are independent of its identity. |
| Ojer transform ability | Rules-owned ActivateAbility candidate visible as ordinary ability action with legality explanation only through safe public state | Explicit activate/pass; no automatic transform at threshold. Candidate omitted if sorcery timing, payment, tap, face or history condition fails. |
| Trample assignment | Shared combat damage assignment decision | Enumerate all rule-legal ordered blocker damage allocations including excess to the defended entity; soundness and completeness proved against independent oracle. Canonical ordering; no first-legal shortcut. |
| Trigger target/order choices | Shared trigger placement/targeting domain | All mandatory targets and meaningful orders represented; no synthetic trigger order or target fallback. |
| Attackers/blockers / passes | Existing Shared combat/priority domains | Haste eligibility updates candidate set; trample assignment follows blockers. All legal subsets/assignments once, including explicit empty response where required. |

For every family: trusted DecisionId/bindings remain in authoritative state; only dense request-local CandidateId and perspective-local opaque IDs are player-visible. Candidate soundness and completeness are separate tests. Reject stale/fabricated answers without state, event, delta, player bytes, checkpoint/replay bytes, counters, allocator or RNG change. Hidden mana/RNG/checkpoint/continuation and trusted object/ability/physical-card IDs never cross the endpoint.

## 7. State, events, privacy and replay

### State ownership

Reuse current `ManaState`, `TurnHistoryState`, `FaceState`, `AbilityAuthorityState`, `CounterState`, `AttachmentState`, `GameObjectId` incarnation and existing execution/knowledge/identity closure exactly where specified. State substrate does not implement producers, evaluators, continuations or coverage.

Additional authoritative state is expected for typed temporary effects, delayed Warp schedule, exile-play permissions, and spell/ability stack/cast resolution payload (including kicker route). Do not model these with arbitrary labels, card-name branches, card text interpretation, or controller-side maps. Each field requires a focused accepted owner/invariant and an identity-cut audit before implementation. No additional Ojer replacement registry is required. If the accepted state owner does not authorize these families, stop.

### Events and history

Every accepted cast, target event, actual life-loss/damage event, activation use, face change, zone incarnation, delayed action, permission grant/end, and temporary effect creation/end needs one authoritative ordered event/product representation. History updates from these semantic events, never from observations or aggregate totals. Publicly visible effects/events use existing `ObservedEventV3` redaction and sequence rules; hidden library identity and any face-down/private exile knowledge remain perspective-scoped. Face-up exile identity is public under CR 406.3.

### Player observations and noninterference

Expose public board/face/counter/mana facts through existing perspective-local opaque IDs only. Never expose `GameObjectId`, `PhysicalCardId`, `AbilityInstanceId`, `AbilityKey`, raw FaceKey, trusted decision binding, seed/RNG, checkpoint, continuation, or genuinely hidden card identity. Rockface mana pool is public to its owner/controller according to current M4.2 projection; a player's restricted mana does not expose another hidden card. Emberheart's library identity remains hidden before the Valiant ability resolves; the resulting face-up exile is public to both players under CR 406.3. Warp's resulting exile is public as well. Permission bindings and delayed/linked trusted identities stay internal, and the permission decision is projected only to its authorized player.

For every capability, pair states differing only in still-hidden library identity, face-down/private exile identity, global ID/RNG history, or hidden continuation payload; canonical products must remain equal for a perspective unauthorized to distinguish that difference. Once a different identity is exiled face up by Emberheart or Warp, the resulting public observations/events must differ accordingly; the authorized play permission itself is not projected to the other player's candidate domain. Public damage, trigger, counter, transform, and mana effects remain observable as the rules require. LKI is used only inside trusted resolution/replacement logic and projected through existing audience/opaque identity policy.

### Determinism and persisted products

All choices, event ordering, effect timestamps, delayed due-step keys, canonical arrays and state encodings are deterministic and independent of hash-map, insertion, allocation and host order. Semantic behavior changes affect `EngineStatePartsV2`, `StateDeltaV2`, authoritative event V2, `ObservedEventV3`, `PlayerStepV3`, `FullStateDigestV6`, `CheckpointV7`, Replay V7 and content/semantic contract closure whenever the corresponding meaning is carried there. RNG must not be consumed for deterministic rules operations.

The current M4.2 contracts already include the six accepted state families, but explicitly defer new effect, permission, delayed, stack payload and full cast/payment semantics. These semantics cannot be smuggled into the frozen persisted execution encoding. **`CONTRACT_GROWTH_REQUIRED`** for any new authoritative effect/permission/delayed/stack payload that is not exactly representable in the current closed state/event/delta/wire/replay contracts. Audit each payload against all products before implementation. Do not choose version numbers here. A version or schema increment is only proposed after an explicit compatibility review; historical FullStateDigestV5/V6 and checkpoint/replay identities keep their exact meanings.

## 8. Fail-closed boundary and deferred work

Reject unsupported card profiles, ambiguous definitions/printing provenance, unrecognized cost/restriction/face/keyword/event variants, overflow, stale targets/permissions, impossible event ordering, illegal payment, unsupported combat assignment, invalid delayed schedule and inconsistent checkpoint/replay state. Rejection is complete nonmutation. Never substitute a target, choose a mode/payment/order, or emit a heuristic action.

This Spec does not close Shared capabilities, W1 requirements, setup/mulligan, all M4 R1 closure, all card Definitions, certification, or playability. It does not add a generic effect VM or arbitrary Card IR interpreter. FaceState/counter/mana/history/ability substrate alone is not support. Any discovery that the above R1 requirements overlap a W1 semantic requirement, need an unresolved shared owner, or cannot be represented without a reviewed contract cut returns to the relevant Shared/architecture review.

## 9. Conformance obligations

Each capability PR requires RED-first exact positive and negative tests, independent legality oracle, soundness and completeness, rejection atomicity, event/delta/state identity equality, observed-event redaction, paired-state noninterference, checkpoint/restore/fork/replay parity, property/fuzz coverage for canonical bounds, and interaction regressions. Minimum capability cases include:

* Hired Claw life lost then gained; exactly one counter activation per turn; multiple Lizards attack gives one trigger.
* Magebane count includes casts before entry and the triggering cast; noncreature only; copies do not count.
* Emberheart opponent-first then own target; own second target; target change/copy; target/controller changes; library identity hidden before trigger resolution and face-up/public after exile; play/decline and end-of-turn permission end. Paired states must diverge in public products after different cards are exiled.
* Hearthborn exactly second cast per player/turn; target opponent complete domain; no repeat on third cast.
* Razorkin first-strike scheduling plus draw-trigger damage ordering.
* Ojer replacement threshold/equality, source controller/color changes, combat exclusion, opponent-only scope, prevention/replacement order, power changes, life vs creature recipient; death return is tapped back-face new incarnation; transform same incarnation; exact 4 damage condition and sorcery/payment legality.
* Nova ordinary vs Warp route; due next end step; exact battlefield incarnation leaves before due and no later-incarnation fallback; exact exile incarnation; permission unavailable through the Warp-cast turn, available after that turn ends, and valid only while that exact card remains exiled; restore/fork/replay at each boundary.
* Burst kick/decline and all legal payments; exact 2/4 damage branch; forged paid-status rejection.
* Rockface colorless vs restricted red mana; every illegal spend class rejected; valid creature cast spend; target type/control/timing and +1/+0/haste duration.
* Firebrand/Lightning Strike/Nova all-target domains; no legal target; stale at resolution; no target leakage.
* Trample all legal blocker allocations, lethal assignment boundaries, zero/excess, first/double-strike interactions, exact soundness/completeness.
* Pairwise interactions with all existing M3/M4.2 paths, both locked decks' Shared clauses, same-turn reset, priority and stack progression.

## 10. Entry and exit gates

### Entry gate for implementation planning acceptance

1. Independent review accepts this Spec's R1 semantic decomposition and candidate dispositions.
2. Exact R1 Oracle/source identities, printing provenance and 60-slot manifest are recorded; no ambiguity remains.
3. Shared capability owners and dependency versions are accepted; every W1-transitive collision has an explicit Shared disposition.
4. Focused decision/state/event/wire contract audits resolve every `CONTRACT_GROWTH_REQUIRED` item before implementation is authorized.
5. Each proposed semantic identity is accepted and assigned registry lifecycle `specified` only through the normal registry workflow.

### Exit gate for R1-exclusive closure

Every listed R1-exclusive semantic family is implemented and covered at accepted identities; all Shared prerequisites are accepted; all eleven R1 non-Mountain unique cards resolve to reviewed definitions with exact provenance; all 60 slots have closed recursive capability dependencies; all decision, information, interaction, checkpoint/restore/fork/replay, deterministic, nonmutation, conformance and evidence gates pass. Certification remains a separate exact-bundle gate; this Spec does not claim it.

## 11. Relationship to Issues #222 and #237

#222 owns the exact R1/W1 deck scope and milestone closure boundary. #237 coordinates candidate overlap only. This Spec corrects the time baseline and uses the accepted M4.2 state cut; its Cxx table is a reconciliation of #237 research rows, not a transfer of authority. Recommend updating #237 only after review: record verified master SHA, distinguish current M4.2 state substrate from support, document the proposed R1 semantic identities/dispositions, and retain Shared payment, trigger, effect, targeting, casting, combat and replacement foundations under Shared coordination.
