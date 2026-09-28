# M4 Shared G0 — Contract-Growth Boundary

**Task:** `M4_SHARED_G0_CONTRACT_GROWTH_SPEC_AND_IMPLEMENTATION_PLAN`
**Status:** Base G0B and bounded payload/stage amendments ACCEPTED. **M4.2 preservation clarification: PROPOSED / NOT ACCEPTED; no G0j implementation authority.** ContinuationRecordV3 ownership and payload inventory Exact-Head PASS at `f0a493c740f9002e5551d317a1411c11067a7bb3`; CostRoute V4 descriptor reconciliation ACCEPTED at independent Exact-Head PASS `69cb222dcb8d9e50861322e2536a7e12103bf014`; actor-only route visibility and immutable-profile symbol binding reconciliation ACCEPTED at independent Exact-Head PASS `3bfbb2d5bdbfc2eb31f85b1c14b47d2dcccb4bba`; preservation of the existing Magic SBA graveyard-order Decision ACCEPTED at independent Exact-Head PASS `d6ba1c00a955792bbf96c1067f5fca3b08ccb650`; exact safe trigger descriptor-to-record binding rule ACCEPTED at independent Exact-Head PASS `e883162e1bfdce1df1c8fb24e603738da54e6441`; activated-ability public-mode projection amendment REVIEWED PASS at independent Exact-Head `2764eadf51fbc09271122a37995160f60f929460` (integration branch, pending G0 activation); cast/activation stack-creation and once-per-turn history receipt amendment independently reviewed PASS at `1da3e963f07bdf79ba397bbe99f2e087202fd5ba` (integration branch, pending G0 activation); the `ProfileDecisionDomainContextV1` proposal was rejected for forgeable Rules provenance at exact-head review `5ddd7d5a54afe670366db60d9bb0711c88c35aff`; revised fail-closed G0 admission boundary independently reviewed PASS at `e012913d7f2fa55823bcc4c95f46a0460517f21b`; detached implementation baseline remains `f5c1ed2aa0719edaebd95f80f7c1c5c38b3dea3d`; current-writer authority remains gated on G0j
**G0a acceptance record:** Independent exact-head G0 Spec/Plan review PASS at `308e465669f66ad63b01d5fb381214c08ce413bc`; PR #250 required CI PASS; merged at `8642db7a389d5363d52224bd040082811f626084` with tree identical to the reviewed head. This accepts the G0a design boundary for G0b only. It does not accept G0b or authorize G0 implementation.
**Verified `origin/master`:** `85f967f641528e43772c63be14679af398dcac86`
**Date:** 2026-09-27

## 1. Authority and baseline

The fetched `origin/master` exactly matches the task baseline and is the merge commit of PR #249. The accepted Shared Foundation inputs are its final-tree Spec and Plan, with the final R1 input cited there at `e2d0aeb63b679ab1bb500a05a7f56c0244e9626c`. Their committed front matter still said `PROPOSED / NOT ACCEPTED`; the status sync records the independent exact-head review PASS at `2c36ccacf1513bc70cb15055a28c197f4feb0ca6`, merged tree-identically by PR #249 at `85f967f641528e43772c63be14679af398dcac86`. The semantic content is unchanged. The Shared design separates targeted cast setup from casting/stack commit and separates persistent temporary effects from source-derived Aura/Role contributions. This G0 Spec carries those accepted boundaries forward; it does not reopen them.

At the G0 task baseline the Shared documents' front matter still said `PROPOSED / NOT ACCEPTED`, despite the independently accepted review and tree-identical merge recorded above. This design change synchronizes only those two status lines and records the acceptance provenance in the Shared Spec. No Shared semantic text is changed by that metadata repair. PR #249's body remains the historical description as written when that PR was opened.

Current status is M4.2 `COMPLETE` only for Mountain + Plains / `basic-land@1.0.0`; M4 remains `IN_PROGRESS`; M4.3/M4.4 production implementation remains `NOT_STARTED`. The 14-row capability registry still consists of eleven `covered` Foundation entries and three M4.2 entries at lifecycle `specified`. G0 does not promote those rows, create new capability rows, or establish card support.

#222 remains the locked matchup scope authority. #237 is coordination/provenance only and its recorded baseline/count is stale; its Cxx labels are not canonical IDs. #225 is closed with bounded M4.2 acceptance evidence. PR #249 merged the Shared Spec and Plan but did not change runtime contracts. Existing PR/issue evidence is inspected as historical evidence, not reused as proof for future implementation gates.

## 2. Purpose and boundary

G0 designs the cross-layer authoritative contract needed before Shared S1–S7 runtime implementation. It owns the identity cut for:

```text
stack spell/ability payloads
pending trigger authority
captured choices and paid-cost facts
temporary-effect records and expiry
staged-action and resolving-stack continuations
state/delta/authoritative-event consistency
Decision request and trusted bindings
public observation/event/PlayerStep projection
digest/checkpoint/replay/wire compatibility
```

G0 does not implement these semantics or any card/capability. It does not admit R1 or W1 content, define an Oracle interpreter, or add a parallel state/RulesKernel authority. Existing `ZoneState` remains the sole stack-record/order owner; existing `ExecutionState` remains the sole pending-decision/continuation/trigger/effect owner. The design does not create `StackState`, `ReplacementState`, a generic resolution VM, or an environment-side action cache.

## 3. Verified current contract inventory

Verified from current master source, schemas, and normative identity documents:

| Current family | Current meaning and present limitation |
|---|---|
| `EngineStatePartsV2` | Successor aggregate over predecessor state, `ExecutionStateV3`, and the six closed M4 card-rules families. It is the complete current FullStateDigestV6 runtime authority. |
| `ZoneState` / `StackRecord` | Stack map plus ordered `stack_order`; record is `id`, `controller`, optional source object, optional source ability. `zones_v1` canonically encodes exactly these four stack-record values and a separate order vector. Current gameplay rejects/avoids nonempty generic stack use. |
| `ExecutionStateV3` / `PersistedExecutionV3` | Pending V3 decision, continuation records, effects, waiting triggers, delayed effects. Its fixed five-element persisted shape currently rejects nonempty effect/trigger/delayed-effect arrays. |
| `ContinuationRecordV2` / `ContinuationPayloadV2` | Closed typed payload with only synthetic assembly and the bounded M3 SBA graveyard-order continuation. At most one active linear continuation is accepted. No casting/activation/trigger-placement/paused-stack-resolution continuation exists. |
| `TriggerRecord` | Only `TriggerInstanceId` and controller. It cannot preserve event/source facts or resolution meaning. |
| `EffectRecord` | Only `EffectInstanceId` and free-form label. Current V6 state validation/digest fails closed on any nonempty execution-effect collection. It is not a semantic effect record. |
| `StateDeltaV2` | In-memory full replacement of `EngineStatePartsV2` plus ordered `SemanticDeltaOperationV2`; no independent persisted public wire schema. It carries before/after FullStateDigestV6. |
| `AuthoritativeRuleEventV2` | Internal RulesKernel event family; no standalone durable event log or public wire schema. Current selected M4 events cover the basic-land slice, not casts, stack, triggers, or effects. |
| `FullStateDigestV6` | Fixed `full-state-digest-input.v6` / `mtgml.full-state-digest.v6`, restricted canonical CBOR under the accepted envelope. It binds `zones_v1`, `PersistedExecutionV3`, and the M4 card-rules record. |
| `EnvironmentCheckpointV7` / `CheckpointDigestV7` | In-memory checkpoint over `EngineStatePartsV2`, FullStateDigestV6, environment status/counters, `ExecutionIdentityV1`, and codec `in-memory-reference / 7`. No durable checkpoint JSON/file codec exists. |
| Replay V7 | Typed replay family binds FullStateDigestV6, CheckpointDigestV7, Decision request V3, DecisionResponseV2, ObservedEvent V3, PlayerStep V3, and the named Magic observation payload. Each step re-executes one explicit response and separately binds trusted before/after global revisions. |
| Decision request V3 | Closed `CandidateIntentV3` / `EngineCandidateBindingV3`, `CandidateOrderingV2`, DecisionDomainV2 (`ChooseOne`, `ChooseMany`, `ChooseNumber`, `Order`), and global `state_revision`. V3 includes PlayLand, CastSpell, ActivateAbility, SelectObject/Player, SelectMode, ChooseBoolean, DeclareNumber, Confirm. Candidate keys are public-only and duplicates reject. |
| `DecisionResponseV2` | Closed answer union bound to request-local candidate IDs and global `state_revision`. The revision echo cannot be reinterpreted as a perspective cursor; a successor response is required to prevent leaking hidden staged-transition counts. |
| ObservedEvent V3 / PlayerStep V3 | Closed public event union with global `state_revision` and no stack-item/effect lifecycle variant. PlayerStep V3 embeds PlayerInformationStateV2, ObservedEvent V3 and request V3. |
| `ObservationEnvelopeV1` / `PlayerInformationStateV2` / `InformationStateDigestV2` | Envelope and InfoStateV2 expose global `state_revision`; V2 digest input binds it. These cannot carry multi-step private action projections without leaking hidden step count. The basic-land payload also lacks stack/effect projection. |
| `IdentityAllocatorState` | Already has next stack/effect/trigger/decision/continuation/rule-event IDs. No second allocator is proposed. Existing counters are trusted state and already digest-bound. |

The current source and exact closed meanings above are the baseline. A convenient Rust field addition is not evidence of semantic or persisted compatibility.

## 4. Persistence classification

| Semantic value | Classification | Why / lifecycle | Contract consequences |
|---|---|---|---|
| Spell stack record and captured spell identity/profile/face | `PERSISTENT_AUTHORITATIVE` | Must survive priority passes, restore/fork/replay, and source/card-zone changes until resolution or counter. The stack card's immutable definition is resolved only under the bound content contract. | New typed stack payload in the sole `ZoneState` stack owner; digest, checkpoint and replay identity grow. |
| Activated-ability stack record and source context | `PERSISTENT_AUTHORITATIVE` | A non-mana ability remains on the stack if its source leaves. Preserve ability/profile identity and only the typed source/LKI facts required by its resolution. Never resolve through a stale live AbilityAuthorityState lookup. | Same stack payload family; `AbilityInstanceId` remains trusted and is not projected. |
| Targets and modes after an action is committed | `PERSISTENT_AUTHORITATIVE` | Resolution, legality, target history, and response outcomes depend on the exact selected slots and values. | Captured in stack payload in semantic slot/order; trusted GameObjectId/StackObjectId/PlayerId only. |
| Cost route and rule-relevant paid status | `PERSISTENT_AUTHORITATIVE` | Kicker or an admitted alternative/additional-cost branch can affect resolution after the original choice and source event. | Captured as closed profile-local typed facts in stack payload; no free-form cost receipt or inferred status from total mana. |
| Complete mana payment allocation before action-cost commit | `TRANSACTION_LOCAL` | The allocation is an explicit final choice over the derived current pool plus provisional source outputs. It is consumed by one atomic cast, non-mana activation, or paused-resolution commit; no selected R1/W1 resolution clause needs the exact assignment of indistinguishable mana units after spending. | Preserve it in the trusted pending Decision binding until response; persist no separate payment ledger. A rejected response leaves the prior continuation intact. If a later accepted clause reads exact spend provenance, amend G0 first. |
| Partial cast/activation/trigger-placement/resolution selections between agent steps | `PERSISTENT_AUTHORITATIVE` | An accepted staged choice must resume with the exact same pending action or resolving stack item; rejected next answers preserve that prior stage unchanged. | Typed cast/activation/trigger-placement/stack-resolution continuation, pending request, digest/checkpoint/fork/replay. No closure, label, or controller-local state. |
| Shared `ManaPaymentStaging` source activations, source incarnations, typed outputs and order | `PERSISTENT_AUTHORITATIVE` | Each accepted source-choice response constrains later source choices and payment candidates, survives checkpoint/fork/replay, and commits exactly once with its parent cast, non-mana activation, or paused resolution. Sources remain unmodified while the action is staged. | Persist one common ordered typed staging record embedded in `CastContinuation`, `NonManaActivationContinuation`, or `StackResolutionContinuation`; digest/checkpoint/replay bind it. |
| Provisional available mana during staged casting, activation, or resolution | `DERIVED_AUTHORITATIVE` | It is determined by existing ManaState plus the parent's reserved action-cost facts and ordered selected source activations; no partial mana payment has happened. | Recompute and validate the 12-bucket provisional pool from shared staging and parent continuation; do not store a second mutable pool or partial-spend vector. |
| Determined cast/activation/resolution cost facts and selected cost operands | `PERSISTENT_AUTHORITATIVE` while the action is staged | Mana-source legality, counter-cost application, and final payment must use the exact determined mana cost, reserved costs, and selected trusted object incarnation; recomputing after a staged choice must preserve the same action. | Store closed typed `action_cost_facts` in the owning continuation; the shared staging validator excludes objects reserved by tap/sacrifice costs from mana-source candidates. |
| Resolving stack item identity and paused resolution stage | `PERSISTENT_AUTHORITATIVE` while a player choice pauses resolution | Ward pay/decline and optional mana sourcing interrupt trigger resolution; the exact resolving stack object and stage must resume after the choice. | `StackResolutionContinuation` references the still-present resolving stack record and embeds `ManaPaymentStaging` only on the payment path; digest/checkpoint/fork/replay bind both. |
| Pending trigger record and trigger-time event/source facts | `PERSISTENT_AUTHORITATIVE` | Trigger waits for APNAP/order/target choices and may resolve after the source departs. Its event facts may not be reconstructed from current board state or an event log. | Typed `TriggerRecord` payload in `ExecutionState`; explicit pending-order continuation; digest/checkpoint/replay. `TriggerInstanceId` remains trusted. |
| Trigger ordering after a player chooses | `PERSISTENT_AUTHORITATIVE` | The chosen semantic order determines stack order. It cannot be recovered from TriggerInstanceId, BTreeMap order, or allocation sequence. | Store the selected per-player order in typed continuation/placement data and final `stack_order`; preserve APNAP group ordering. |
| Temporary P/T/keyword operation and expiry | `PERSISTENT_AUTHORITATIVE` | A changing temporary effect must affect future legality/characteristics until its exact expiry, across intervening decisions. | Typed effect record in existing effects owner; initially end-of-turn expiry keyed to the existing turn number. Existing EffectInstanceId reused. Only additive P/T, Haste, and Double strike are admitted by G0; no label, callback, or arbitrary map. |
| Aura/Role static characteristic contribution | `DERIVED_AUTHORITATIVE` | It follows live source profile + current AttachmentState and ceases when that relation/source ceases. It is not an until-EOT effect. | Derived through the single Characteristic Derivation path; no temporary-effect record or second state family. |
| Current characteristic query / replacement eligibility set | `DERIVED_AUTHORITATIVE` | Recomputed from content identity and current state at the semantic query/event boundary. | No persisted query cache or ReplacementState. |
| Eligible candidate/domain relation | `DERIVED_AUTHORITATIVE` | Rules generate all-and-only legal options from current state and typed continuation. | Exact authoritative pending Decision (with candidate bindings) persists while waiting; no separately cached environment candidate set. |
| Pending authoritative Decision and trusted bindings | `PERSISTENT_AUTHORITATIVE` | It is current M2/M4 decision authority and survives checkpoint/fork/replay until accepted/rejected under current protocol. | Successor request type and candidate ordering; trusted IDs remain inside authoritative state. |
| Global `StateRevision` | `PERSISTENT_AUTHORITATIVE`, trusted-only in player projection | It remains the rules transaction revision and digest/checkpoint/replay value, but exposing it lets another perspective count private staged-action decisions. | Keep it in EngineState, authoritative request, Delta, checkpoint, and replay; remove it from successor Observation/PlayerInformation/PlayerDecision/Response/ObservedEvent/PlayerStep fields. |
| Per-perspective public-view cursor (`next_visible_sequence`) | `PERSISTENT_AUTHORITATIVE`, existing state reused | It already advances only for occurrences visible to that perspective and is checkpointed. It can version that perspective's products without disclosing hidden global revisions. | Reuse as the cursor in successor ObservationEnvelope, PlayerInformationState, request, response, and event products. No new counter/allocator. Public projection changes require an authorized visible occurrence/cursor advance. |
| Accepted/rejected response bytes and replay control input | `HISTORICAL_EVIDENCE_ONLY` after the transition | The response is replay input; observations/events are outputs and never reconstruct control. | Successor Replay records the exact successor player response plus trusted global before/after revisions and new typed state/checkpoint identities. DecisionResponseV2 remains historical only. |
| Authoritative event cursor, replacement iteration, local cost calculation | `TRANSACTION_LOCAL` | These are workspace calculations for one atomic transition unless a typed player decision pauses the process. | Discard after commit; if a real replacement choice pauses, specify a closed continuation amendment before implementation. |
| Authoritative events / StateDelta operation trace | `TRANSACTION_LOCAL` | They are validated transition products, not state logs or alternate authority. | Reconstruct complete after-state from replacement; verify events/operations against state cursor; replay reexecutes responses. |
| PlayerObservation, PlayerInformationState, ObservedEvent, PlayerStep | `PLAYER_PRODUCT_ONLY` | Safe perspective products; they do not own rules state or replay control. | New stack/effect projection uses opaque identities and paired-world noninterference. Successor products use the existing per-perspective visible sequence, never global StateRevision. |
| Historical V6 digest, V7 checkpoint/replay, V3 request/event/step values | `HISTORICAL_EVIDENCE_ONLY` under a future successor runtime | Their existing closed meanings cannot gain new stack/effect meaning. | Preserve exact readers/verifiers where present; no automatic migration or current writer after the successor identity cut. |

### 4.1 Typed semantic record boundary

The successor state shape reuses two existing owners and introduces no semantic sidecar:

```text
ZoneState.stack_records / stack_order
  → typed StackRecord payload for spell, activated-ability, triggered-ability items

ExecutionState.waiting_triggers
  → typed pending TriggerRecord + captured event/source context

ExecutionState.effects
  → typed TemporaryEffectRecord only

ExecutionState.continuations + pending authoritative request
  → typed staged cast/activation/trigger-placement state
```

No separate `StackState`, `PendingActionCache`, `ReplacementState`, `AuraEffectState`, or `OpaqueStackId` family is proposed. `StackObjectId`, `TriggerInstanceId`, `EffectInstanceId`, `ContinuationId`, `RuleEventId`, and `GameObjectId` reuse current trusted allocator families. Their numeric values never define semantic order. The stack's explicit order vector and explicit trigger-order result are the order authorities.

The proposed stack record has a closed origin tag (`Spell`, `ActivatedAbility`, `TriggeredAbility`) and a closed payload containing only:

* controller and exact stack identity;
* spell card-object/definition/face/profile identity, or ability source incarnation/AbilityKey/profile identity;
* modes and target bindings in semantic slot order;
* typed cost-route and paid-additional/alternative-cost facts that can affect resolution;
* the bounded typed resolution context required after source departure;
* captured source/event facts only where the pinned rules/profile require them.

The typed payload is divided by origin so a spell does not carry ability-only fields and vice versa. In successor-schema pseudocode (all names provisional and unversioned):

```text
StackRecordSuccessor {
  stack_object_id: StackObjectId,
  controller: PlayerId,
  item: SpellStackItem | ActivatedAbilityStackItem | TriggeredAbilityStackItem
}

SpellStackItem {
  stack_card_object: GameObjectId,
  card_definition_id: CardDefinitionId,
  face_key: FaceKey,
  semantic_profile_id: CardSemanticProfileId,
  cast_modes: ordered typed mode selections,
  targets: ordered typed target bindings,
  cost_facts: typed selected route + paid additional-cost keys,
  // no separate resolution_context: see the closed capture rule below
}

ActivatedAbilityStackItem {
  source_context: AbilitySourceContext,
  targets: ordered typed target bindings,
  cost_facts: typed selected route + paid additional-cost keys
}

TriggeredAbilityStackItem {
  originating_trigger: TriggerInstanceId,
  source_context: AbilitySourceContext,
  captured_trigger_context: TriggerEventSnapshot,
  targets: ordered typed target bindings,
}
```

`CostFactsV1.paid_additional_cost_ids` is a set-valued paid-status fact represented as a sorted, duplicate-free vector of profile-local `u32` IDs. It does not encode player selection order. `selected_route` remains a single typed route value or `null`.

When a pending trigger is placed on the stack, its `AbilitySourceContext`, controller, captured event facts, and chosen targets transfer into the triggered stack item before the pending record is removed. `AbilitySourceContext` carries the exact `ability_key` and `semantic_profile_id`; `originating_trigger` remains as trusted provenance only. Duplicate controller/source fields are removed; controller belongs to `StackRecord`, and source identity belongs to the typed payload. Spell card objects in Stack Zone and `stack_records` must validate bidirectionally. A triggered/activated ability can have no stack-zone card object. Payload origin/profile/source identities join only under the immutable execution content contract.

Target references are a closed trusted union (current object incarnation, player, and stack item where a triggered/counter rule needs it). Ability/card semantics resolve through the immutable content and semantic contract bound by `ExecutionIdentityV1`; no card-name switch or process-global catalog lookup. A stack payload is not Oracle text, arbitrary JSON, an opcode, a callback, a closure, or an untyped `EffectRecord` label. A profile that needs a new captured value must add a typed reviewed variant before its card can be admitted.

The pending trigger record contains controller; immutable source/profile/ability identity; the closed trigger-time `TriggerEventSnapshot`; and the exact `target_timing_tag`. It cannot depend on a live source after departure. Triggers already detected survive source departure. The record is removed when the typed payload is transferred to the stack. No intervening-if receipt is stored because no locked R1 × W1 witness requires one. APNAP placement order and each player's chosen order are explicit vectors, never BTreeMap/TriggerInstanceId order. Reflexive/delayed trigger frameworks are not added here; their exact profiles remain later Shared or deck-exclusive work.

The trigger event context is a typed snapshot, never just `RuleEventId` or a lookup into an unpersisted event log. Its closed event facts are exactly the `SpellCast`, `AbilityActivated`, `TargetBecame`, `ObjectEntered`, `ObjectLeftOrDied`, `BeginningOfCombat`, `AttackDeclared`, `CardDrawn`, `CounterChanged`, `DamageApplied`, and `LifeChanged` variants defined below. Each carries only the rule facts needed for its admitted predicates/effects. A new event fact or predicate shape outside those families requires G0 amendment; no arbitrary map or callback payload is admitted.

The G0 temporary-operation vocabulary is bounded to additive P/T and the keyword grants `Haste` and `DoubleStrike`, with direct R1 Prowess/Rockface and W1 Origin of Spider-Man chapter III witnesses. No temporary type-addition or protection operation is admitted. Hexproof from Shardmage's Rescue and other Aura/Role abilities remain live source/profile/AttachmentState-derived contributions where applicable. Origin of Spider-Man chapter II's permanent type change is W1-exclusive, has no end-of-turn duration, and requires separate persistent-effect characterization before W1 implementation; it is not represented by `TemporaryEffectRecord`. Duration in G0 represents only `UntilEndOfTurn { turn_number }`; duration semantics outside those witnesses remain unsupported. The effect record stores a trusted `EffectInstanceId`, target incarnation(s), closed operation, expiry, and a rule timestamp only where the admitted operation's order can affect a query. Static Aura/Role contributions stay live-source/AttachmentState-derived and never produce a temporary record. `delayed_effects` remain outside Shared G0 unless a separate Shared witness is accepted.

The record's expiry is checked against the current authoritative turn/turn-position at cleanup. Effects that are independent of their creating source have no source-liveness gate. If a selected profile proves a source-dependent temporary duration, it gets a distinct typed source reference and explicit invalidation rule before admission; the evaluator may not infer dependence from a source ID's presence.

`ObjectSnapshot` alone is not treated as complete Last Known Information: current snapshots omit `FaceKey` and derived characteristics. Stack/trigger payloads capture a closed source-context variant with the exact immutable profile identity and only the last-known typed facts the profile uses. They do not persist a generic snapshot of the world or infer old incarnations through `PhysicalCardId`.

## 5. Atomicity and staged actions

Casting, activation, trigger placement, cost-operand selection, and a paused stack resolution can span multiple meaningful agent steps, but each submitted response remains one atomic RulesKernel/environment transaction.

* One trusted `ContinuationId` is the parent logical-action identity internally. It uses the existing continuation allocator and is embedded in the authoritative request; it is never exposed to players.
* Each perspective-visible stage gets its existing perspective-local `PlayerDecisionIdV1`. The successor request adds a typed public decision purpose/slot and may point to the first request ID of this action for the same actor. This reuses the already visible perspective-local decision identity; it adds no global or new action allocator and is not a stable dataset/action key.
* A `CastContinuation` records actor, spell incarnation/profile, stage, selected route, modes/targets/additional-cost facts and optional shared `ManaPaymentStaging`. A `NonManaActivationContinuation` records source incarnation/ability, stage, selected modes/targets, closed `action_cost_facts`, and optional shared `ManaPaymentStaging`. `Evershrike's Gift` `{1}{W}`, Blight 2 activation stores one selected creature incarnation as a typed `PutCounters { MinusOneMinusOne, 2 }` cost operand. Fanatical Firebrand's `{T}, sacrifice this creature` cost is represented by the existing closed `tap_source` and `sacrifice_source` cost facts. For example, Rockface Village’s `{R}, {T}` activation and Abandoned Air Temple’s `{3}{W}, {T}` activation both store their mana cost and reserve their own source for tapping. Mana cost values and selected non-mana cost operations are typed and immutable across the staged action. Trigger placement remains a separate typed variant. Each follows RulesKernel-owned stage order; clients cannot choose stages.
* Provisional targets/modes/routes and both kinds of mana-payment staging do not create public rule events, turn-history facts, stack items, tap/mana mutations, or pay costs. Once all required choices and a complete payment plan are valid, source activation costs/outputs, cast or ability mana payment, the action's own non-mana costs, object/stack changes, target-announcement events, history updates, trigger detection, deltas, and projections commit atomically.
* Casts, non-mana activations, and a paused stack resolution use the same embedded `ManaPaymentStaging` contract whenever the owning action cost has a mana component. `SelectManaSource` candidates identify an authorized opaque source and ability plus a complete typed 12-bucket output; the trusted binding fixes the current source incarnation, AbilityInstanceId/profile, activation cost, and output. Each accepted response appends one activation to the shared ordered list; the source remains untapped/unmodified in authoritative game state. The provisional pool is derived by replaying that list from unchanged current ManaState. Before enumerating sources, the validator reads the parent action's reserved cost facts and excludes tapped/sacrificed objects and every source already selected in this staging sequence. Thus a non-mana ability source with `{T}` reserved by its own activation cannot also appear as a mana source. Other game actions cannot interleave with the pending action; no pre-action snapshot is stored. When the derived pool can pay the action mana cost, the `ManaProductionChoice` domain includes a typed `FinalizeManaProduction` candidate; selecting it advances to the explicit final payment-allocation request. If exactly one required source activation remains legal and no competing source/finalize outcome exists, that forced progression creates no synthetic step; where multiple legal outcomes exist, the player receives the complete explicit domain. If no mana cost exists, the shared staging field is null and no mana-production/payment step is fabricated. At final commit, selected mana-source activations execute in stored semantic order, then the chosen complete allocation is spent together with the parent action costs. For a non-mana activation this includes the ability's own typed non-mana cost (for example its reserved tap-source cost) and stack-item creation. Costs, resource changes, stack creation, history, triggers, deltas, and public events commit atomically. Rejected responses preserve the prior continuation and leave game resources unchanged.
* When a resolving triggered ability presents an optional mana payment (Skyward Spider's native Ward {2} and Sheltered by Ghosts' granted Ward {2}), the current top resolving stack record stays in `ZoneState`; `StackResolutionContinuation` stores its trusted StackObjectId and pause stage. The pay/decline answer uses `OptionalCostPayment`. Decline completes that resolution; pay initializes common `ManaPaymentStaging`, enumerates legal sources while the resolving item and targeted stack item remain unchanged, and after final allocation applies the profile result and retires the resolving item atomically. The paused continuation contains only these typed stages and facts, not a generic resolution VM.
* A cost operand such as Evershrike's Gift `{1}{W}`, Blight 2 is selected by one explicit `SelectObject` choice under typed `CostOperandSelection`. Its candidate domain is all and only currently legal creatures controlled by the activating player; the choice is an additional-cost operand, not a spell/ability target. The trusted `SelectedCostOperand` stores exact object incarnation, counter type, and count in parent `action_cost_facts`; the player product exposes only OpaqueObjectId plus the public typed operation. Selection itself does not place counters. Mana-source availability respects action-reserved tap/sacrifice costs but does not treat a counter-placement operand as an unavailable tap source. Final activation commit applies the selected counter cost, mana sources/payment, and ability stack creation in one atomic transition; all derived cost targets are revalidated before commit.
* Trigger APNAP/order/target choices are explicit. Partial choices survive in the typed continuation while waiting; no trigger is silently ordered. Final stack placement preserves the exact selected APNAP/order sequence.
* Stale, fabricated, malformed, illegal, or incomplete responses mutate none of the current after-state bytes, allocators, RNG, candidate bindings, continuation stage, information, events, PlayerStep status, or replay history. Atomic cost operations never leave a partial sacrifice/tap/counter/mana payment after a rejected response.

The current player products expose global StateRevision in ObservationEnvelopeV1, PlayerInformationStateV2, PlayerDecisionRequestV3, DecisionResponseV2, and ObservedEventEnvelopeV3. Because each accepted staged choice advances the trusted rules revision, keeping those fields in successor player products would let another perspective distinguish worlds that differ only in how many hidden cast/activation stages occurred. G0 therefore removes the global revision from all player-visible successor values. The trusted pending request still retains its state revision and is bound by its perspective-local PlayerDecisionId; a stale answer is rejected by exact pending-request identity, decision ID, and view sequence. The successor player view uses the existing perspective-local `next_visible_sequence` as its view cursor: authorized visible occurrences advance it, hidden occurrences consume nothing, and every public observation change must have an authorized visible event/occurrence. The authoritative Replay and checkpoint continue to use global StateRevision.

## 6. Decision and candidate contract

No new `DecisionAnswer` or numeric/order `DecisionDomain` family is required. The current `ChooseOne`, `ChooseMany`, `ChooseNumber`, and `Order` semantics remain useful. The closed player request must grow because V3 has no semantic context for staged cost/payment/trigger choices and no visible binding type for several required domains.

The successor request contract therefore requires:

1. a closed `DecisionPurpose` identifying the immediate stage (priority action, cost route, target slot, cost-operand selection, optional payment, mana-source activation/finalization, payment plan, trigger order, or replacement choice if later justified), including only public/profile-local slot facts;
2. an optional same-actor `parent_player_decision_id` linking a stage to its first request; the authoritative `ContinuationId` remains trusted-only;
3. closed successor `CandidateIntent` and trusted-binding variants for cast-cost route, mana-source activation/finalization, complete mana-payment allocation, selected cost operand, optional payment, and pending trigger-order item; existing CastSpell, ActivateAbility, SelectObject/Player, SelectMode, and ChooseBoolean are retained where their meaning is exact;
4. one new canonical candidate comparator covering each new visible typed payload. It uses only request-authorized visible material, never a trusted ID, source iteration, allocator history, or hidden tiebreaker.

The proposed request record is the current closed fields plus these two required-in-successor fields (no identity assigned):

```text
SuccessorPlayerDecisionRequest {
  schema_version,
  player_decision_id, view_sequence, actor, visibility,
  decision_domain_v2,
  purpose: DecisionPurpose,
  parent_player_decision_id: PlayerDecisionIdV1 | null,
  candidates: SuccessorVisibleCandidate[]
}

SuccessorDecisionResponse {
  schema_version,
  player_decision_id,
  view_sequence,
  answer: DecisionAnswerV2
}
```

The player request/response must not contain `StateRevision`. `view_sequence` is the actor's existing perspective-local `next_visible_sequence` at the request boundary; the response echoes it with PlayerDecisionIdV1. The trusted pending request still stores global StateRevision and candidate bindings. Server validation matches the actor-bound endpoint, exact pending PlayerDecisionId/view-sequence pair, answer domain, dense candidate IDs, and exact trusted bindings before mutation. A new response wire identity is required because DecisionResponseV2's required global `state_revision` field cannot be reinterpreted as a perspective cursor.

`DecisionPurpose` is a closed tagged union: `PriorityAction`, `AttackerDeclaration`, `SbaGraveyardOrder`, `CastCostRoute`, `ModeSelection { mode_slot }`, `TargetSelection { target_slot }`, `CostOperandSelection { cost_slot, operation, counter_kind, count }`, `ManaProductionChoice`, `ManaPayment`, `OptionalCostPayment { profile_local_cost_id }`, `AbilityAction`, `TriggerOrder`, `TriggerTarget { target_slot }`, or `SyntheticAssembly { stage }`. Purpose IDs are typed profile-local ordinals, not strings or `AbilityKey`. `AttackerDeclaration` preserves the already implemented M3 ChooseMany over visible SelectObject candidates and is required for combat in both locked decks. `SbaGraveyardOrder` preserves the existing Magic M2 graveyard-order continuation: each APNAP owner explicitly orders that owner's simultaneous graveyard-bound object set using the existing `Order` domain and safe `SelectObject` candidates. It is a general rules decision, not a card or capability mechanic. `SyntheticAssembly` preserves only the existing non-Magic deterministic test program's entry/count/member/order decisions; it adds no Magic rule, card capability, or execution-program identity. Its closed stage values are `entry`, `choose_count`, `choose_members`, and `order_members`. The successor candidate intent/binding pairs add:

Purpose and answer-domain compatibility is closed: `PriorityAction`, `CastCostRoute`, `CostOperandSelection`, `ManaProductionChoice`, `ManaPayment`, `OptionalCostPayment`, and `AbilityAction` require `ChooseOne`; `AttackerDeclaration` requires `ChooseMany`; `SbaGraveyardOrder` and `TriggerOrder` require `Order`; `ModeSelection`, `TargetSelection`, and `TriggerTarget` accept `ChooseOne` or `ChooseMany` only as required by their rule-defined slot/group. `SyntheticAssembly` maps exactly by stage: `entry → ChooseOne`, `choose_count → ChooseNumber`, `choose_members → ChooseMany`, `order_members → Order`. C58 remains deferred and receives no numeric-declaration purpose. `AttackerDeclaration`, `CastCostRoute`, `SbaGraveyardOrder`, and `TriggerOrder` require `visibility = acting_player_only`: their candidates or descriptors are authorized only to the acting player, and partial cast-route choices are private under the Information Model. `SyntheticAssembly` requires `visibility = public`, matching its existing non-Magic test program. For other purposes, `visibility` is derived from candidate authorization under the Information Model; purpose alone does not widen the audience.

The permitted public candidate intent variants are also purpose-closed: `PriorityAction` permits `PassPriority`, `PlayLand`, `CastSpell`, and `ActivateAbility`; `AttackerDeclaration` and `SbaGraveyardOrder` permit `SelectObject`; `CastCostRoute` permits `SelectCostRoute`; `ModeSelection` permits `SelectMode`; `TargetSelection` and `TriggerTarget` permit `SelectObject` or `SelectPlayer`; `CostOperandSelection` permits `SelectObject`; `ManaProductionChoice` permits `SelectManaSource` or `FinalizeManaProduction`; `ManaPayment` permits `SelectManaPayment`; `OptionalCostPayment` permits `ChooseBoolean`; `AbilityAction` permits `ActivateAbility`; `TriggerOrder` permits `SelectTrigger`. `SyntheticAssembly` permits `SelectObject` for `entry`, `choose_members`, and `order_members`; `choose_count` has no candidate entries. Candidate tags outside this relation reject at schema and typed DTO validation before any trusted binding is resolved.

* `SelectCostRoute { descriptor: CostRouteDescriptorV1 }` ↔ `SelectCostRoute { route: CostRouteV1 }`;
* `SelectObject { opaque_object_id }` under `SbaGraveyardOrder` ↔ the existing trusted `SelectObject { GameObjectId }` binding for one current object owned by the current APNAP ordering actor;
* `SelectObject { opaque_object_id }` under `CostOperandSelection` ↔ trusted `SelectedCostOperand { cost_slot, GameObjectId, typed_operation }`; for the bounded Blight witness, the operation is `PutCounters { kind: MinusOneMinusOne, count: 2 }`;
* `SelectManaSource { source_opaque_id, ability_opaque_id, produced_buckets_u32[12] }` ↔ trusted source incarnation, AbilityInstanceId/profile, admitted activation-cost receipt, and exact output;
* `FinalizeManaProduction` ↔ the exact trusted cast continuation and current derived provisional pool;
* `SelectManaPayment { spent_buckets_u32[12] }` ↔ the same typed bucket vector, validated against the derived provisional pool/cost;
* `SelectTrigger { safe_trigger_descriptor }` ↔ `SelectTrigger { trigger_instance_id }`. When resolving a trusted binding, the request validator loads that exact `PendingTriggerRecord`, projects its captured source/event snapshot through the ordering actor's authorized opaque-identity view, and requires exact equality with the candidate descriptor. A descriptor rebound to a different trigger, stale trigger, fabricated safe subject, or mismatched visible source identity rejects before an Order response can mutate state. The candidate domain is all and only currently pending triggers controlled by the current APNAP ordering actor; duplicate projected descriptors fail closed rather than being distinguished by trusted IDs or allocator order.

The trigger descriptor is a closed `SafeTriggerDescriptorV1` value. It contains nullable authorized opaque source-object and source-ability identities, a `TriggerEventKindV1`, and one `SafeTriggerSubjectV1` whose tag must match that event kind. If `source_ability` is non-null, `source_object` must also be non-null and visible; both may be null when neither is authorized. This preserves the perspective identity map from each visible `OpaqueAbilityId` to its one source object. It never contains `TriggerInstanceId`, `StackObjectId`, `GameObjectId`, allocator-derived occurrence, or hidden card identity. The subject union mirrors the accepted `TriggerEventSnapshot` event union and projects only facts visible to the ordering actor:

| Rank | Event tag | Safe subject fields |
|---:|---|---|
| 0 | `spell_cast` | actor opaque player identity; spell source object opaque identity only when the spell is publicly identifiable; creature-spell boolean; public typed paid-cost facts. |
| 1 | `ability_activated` | actor opaque player identity; source object/ability opaque identities; public target descriptors; public typed paid-cost facts. |
| 2 | `target_became` | actor opaque player identity; public target descriptor. The source stack identity is omitted. |
| 3 | `object_entered` | object opaque identity only when the entering object is public to the ordering actor. |
| 4 | `object_left_or_died` | last-known object opaque identity only when public to the ordering actor; public destination-zone tag. |
| 5 | `beginning_of_combat` | active-player opaque identity; public turn number. |
| 6 | `attack_declared` | controller opaque identity; attacker/defending-player pairs sorted by attacker opaque identity. This is a descriptor-only canonical projection; it does not change the event snapshot or combat semantics. |
| 7 | `card_drawn` | drawing-player opaque identity. The drawn card identity is never included unless a separate public rule event has revealed it. |
| 8 | `counter_changed` | object opaque identity only when public; closed counter-kind tag; public before/after counts. |
| 9 | `damage_applied` | optional source object opaque identity only when public; public recipient descriptor; amount; closed damage-kind tag. |
| 10 | `life_changed` | player opaque identity; public before/after life totals; closed cause tag. |

These ranks are the exact `TriggerEventKindV1` wire/order discriminants for `SafeTriggerDescriptorV1`; declaration order, Rust enum layout, and lexical string order are not substitutes. They are used before the event-specific safe subject tuple in `CandidateOrderingV3`.

The event-specific subject tuple and its field order are fixed as follows; each tuple compares lexicographically using the value ordering stated here:

| Event tag | Subject comparison tuple |
|---|---|
| `spell_cast` | actor; nullable spell-source object; creature-spell boolean (`false < true`); `CostFactsV1`. |
| `ability_activated` | actor; activated-source object; activated-source ability; target descriptors in captured target-slot vector order; `CostFactsV1`. |
| `target_became` | actor; target descriptor. |
| `object_entered` | nullable object identity. |
| `object_left_or_died` | nullable last-known object identity; destination-zone rank. |
| `beginning_of_combat` | active player; turn number. |
| `attack_declared` | controller; attacker/defending-player pair vector, already sorted by attacker opaque identity. |
| `card_drawn` | player. |
| `counter_changed` | nullable object identity; counter-kind rank; before count; after count. |
| `damage_applied` | nullable source object; recipient descriptor; amount; damage-kind rank. |
| `life_changed` | player; before total; after total; cause rank. |

Nullable values sort before present values. Numeric values use unsigned numeric order for IDs/counts/turns and signed numeric order for life totals. Vectors compare element-by-element, then by length. `CostFactsV1` compares selected route (`null < normal < alternative(route_id)`), then the ascending `paid_additional_cost_ids` vector. Enum ranks are fixed: zones `library=0, hand=1, battlefield=2, graveyard=3, exile=4, stack=5, command=6, ante=7, outside=8`; counter kinds `plus_one_plus_one=0, minus_one_minus_one=1, lore=2`; damage kinds `combat=0, noncombat=1`; life-change causes `damage=0, non_damage=1`. After the event tag and subject tuple, compare nullable source-object opaque identity and then nullable source-ability opaque identity. These are semantic comparator rules, not serialization field-order requirements.

The `public target descriptor` fields in the `ability_activated` and `target_became` subjects use this closed `SafeTargetDescriptorV1` union:

| Variant | Player-facing value | Canonical variant rank |
|---|---|---:|
| `object` | perspective-local `OpaqueObjectId` for an object visible to the ordering actor | 0 |
| `player` | public `PlayerId` | 1 |
| `stack_item` | `stack_position_from_top_u32` in the current public stack view | 2 |

The trusted binding remains `GameObjectId`, `PlayerId`, or `StackObjectId`, respectively. A StackItem target is projected through its position in the public top-to-bottom stack view; `StackObjectId` is never serialized. The comparator orders target descriptors first by variant rank, then by their visible payload (opaque object ID numerically, public player ID numerically, or stack position numerically). Target vectors preserve the target-slot order captured by the event. A target that cannot be represented to the ordering actor under these rules makes the trigger-order request fail closed; the serializer may not substitute a trusted identity or omit a target needed to distinguish legal choices.

An opaque object/player/ability value is admitted only when that identity is already visible to the ordering actor under the Information Model; a hidden-zone object maps to `null` or is omitted by its event-specific subject variant. Subject variants use fixed fields and closed enums, never extension maps, rendered text, or trusted IDs. The request candidate comparator orders by event tag, then the event-specific safe subject tuple, then safe source opaque identities, consistent with the rank/payload definition below. Attacker/defender pairs in this descriptor are sorted by attacker opaque identity; the underlying event snapshot retains its own existing representation. Candidate construction requires the mapping from pending trigger instances to safe descriptors to be injective for every simultaneous ordering domain in the admitted locked profiles. Any duplicate descriptor rejects the entire request before candidate IDs are assigned; G0f/G0i must prove that no admitted locked interaction reaches this case. G0 does not quotient duplicate triggers or use `TriggerInstanceId`, `StackObjectId`, allocator order, or collection order to distinguish them. If a locked interaction requires ordering colliding descriptors, stop and amend G0 with an explicitly safe distinguishing fact before implementation continues.

Mana-source descriptors similarly expose only authorized opaque source/ability IDs and the typed produced-mana vector; source activations and the optional finalize candidate share one `ManaProductionChoice` request purpose; their binding carries current trusted incarnations and profile authority. New tags are appended under a newly reviewed candidate-order identity; V3 rank/payload keys and duplicate rejection remain exact.

The payment candidate is a complete allocation over the six existing mana colors and two existing restriction buckets (fixed W/U/B/R/G/C order for each). It has no physical mana-unit IDs. Source activations are selected one at a time and append typed outputs in semantic order to the cast continuation; the provisional pool is derived from that sequence without mutating ManaState. All and only legal next source activations/finalization choices are enumerated, then all and only legal allocations from the resulting pool are enumerated; a complete allocation is selected in one ChooseOne step. The cost-route choice is separately explicit when the profile offers real alternatives. The final stack payload preserves the selected route and paid-cost facts; the exact post-commit mana vector and payment event preserve relevant resource consequences.

`SelectCostRoute` carries a public, typed route descriptor derived from the authorized card/profile view. The exact closed value is:

```text
CostRouteClassV1 = normal | alternative
PrintedManaSymbolsV1 = [colored_wubrg_counts_u32[5], colorless_count_u32,
                        generic_count_u32]
CostRouteDescriptorV1 = [route_class, printed_mana_symbols,
                         profile_local_option_ordinal_or_null]
trusted route key = CostRouteV1::Normal
                 | CostRouteV1::Alternative { route_id_u32 }
```

`normal` has a null option ordinal and binds only to `CostRouteV1::Normal`. `alternative` has a present ordinal which equals the bound `CostRouteV1::Alternative.route_id`. The three printed-symbol fields encode the admitted typed mana symbols in fixed W/U/B/R/G, colorless, generic order; they are display/domain facts, not a cost language. The descriptor has no rendered text, card name, CapabilityKey, CardDefinitionId, or arbitrary symbol tag. CandidateOrderingV3 compares the route class first (`normal=0`, `alternative=1`), then the alternative option ordinal; two candidates with one route key but differing printed-symbol values collide and reject. The trusted route key and descriptor must agree with the exact immutable profile definition before binding is accepted: the profile-bound validator resolves the route under the already-authorized card/profile context and compares every printed-symbol count as well as the route class and ordinal. A structurally matching `CostRouteV1` key alone is insufficient. The option ordinal is unique within that profile and is not allocated at runtime. The same profile-local-ID rule applies to paid additional-cost facts.

Target selection reuses opaque `SelectObject`/public `SelectPlayer` values with a typed target-slot purpose; trusted target bindings remain current `GameObjectId`/`PlayerId` incarnations. Modes reuse the current numeric mode intent with profile-defined slot meaning. Mana-source ordering is the player-selected sequence in shared `ManaPaymentStaging`, not an allocator-derived order. Trigger ordering binds request-local visible trigger descriptors to trusted TriggerInstanceIds, never exposing those IDs. Any duplicate safe descriptor makes the candidate request fail closed; G0f/G0i must prove that all admitted locked simultaneous-trigger domains are injective, and a collision requires G0 amendment before implementation continues. C58 remains `NEEDS_FURTHER_CHARACTERIZATION`; G0 reserves no combat-damage DecisionPurpose, candidate, binding, comparator rank, or wire shape. Characterize the exact legal relation first, then amend G0 only if the accepted relation requires contract growth. No replacement-choice binding is admitted for G0 because the currently locked Ojer and Dryad replacement profiles do not present a non-equivalent player choice for one event; if characterization finds one, stop and amend G0 before implementation.

The request context is player-visible only to its authorized actor. The parent request ID reuses that actor's existing perspective-local allocator and is confined to one logical staged action. It is not copied to another actor's request, PlayerInformationState, or dataset key. One policy-relevant choice remains one PlayerStep/agent response; a mandatory forced rule consequence is not a fabricated Decision.

For one actor's staged cast/activation, the first request uses `parent_player_decision_id = null`; subsequent requests in that logical action carry the first request's PlayerDecisionIdV1. APNAP trigger placement may ask a different actor: the trusted ContinuationId remains the one rules action, but each actor's own visible chain gets its own first request/root. The continuation stores those roots only to correlate that actor's later stages. No other perspective receives them. Parent IDs are per-episode interaction metadata, not semantic action keys or training labels.

| Player choice | Existing domain/intent usable? | New binding or context | One agent step / boundary |
|---|---|---|---|
| Select spell / activate ability | Yes: CastSpell / ActivateAbility + ChooseOne | Parent action purpose on successor request; trusted object/ability binding remains internal | One response starts the staged action. |
| Select modes | Yes: SelectMode + ChooseOne/ChooseMany | Typed profile-defined mode slot in request context where multiple mode groups exist | One response per meaningful mode choice. |
| Order a simultaneous Magic SBA graveyard group | Yes: existing `Order` domain + visible `SelectObject` | New `SbaGraveyardOrder` purpose binds exactly the current APNAP owner's complete object set; public candidate values use perspective-authorized opaque IDs and the existing trusted object bindings | One complete Order response per owner with at least two objects; singleton groups are forced and create no synthetic response. |
| Select target(s) | Yes: SelectObject/SelectPlayer + ChooseOne/ChooseMany | Target group/slot purpose; bind exact object incarnation/player | One response for each rule-defined target group, not hidden auto-targeting. |
| Select alternative/additional cost route | Not fully: Boolean lacks route semantics | New closed cost-route candidate/binding; `ChooseBoolean` is allowed only for an explicitly typed pay/decline purpose | One response per meaningful route choice. |
| Select a cost operand (Blight 2) | Yes: `SelectObject`/ChooseOne with a typed cost-slot purpose | Trusted binding records exact object incarnation and `PutCounters { MinusOneMinusOne, 2 }`; player sees only an authorized opaque object and operation descriptor | One response selects the creature; the selected operand persists in the activation continuation until atomic cost commit. |
| Resolve Ward optional payment | Yes: existing Boolean/ChooseOne response under `OptionalCostPayment` | `StackResolutionContinuation` retains resolving stack identity/stage and `{2}` cost; payer derives from captured target-stack controller relation; pay attaches shared `ManaPaymentStaging`, decline completes the trigger profile | One pay/decline response; if pay requires mana sources, each meaningful source/payment choice is a further explicit step. |
| Select mana source / finish producing mana | No current typed source-choice candidate | New `SelectManaSource` descriptor and trusted source/ability/profile binding; `FinalizeManaProduction` is included only when current provisional mana can pay | One response per selected source activation; choosing finalize ends source selection and advances to payment. |
| Choose mana payment | No current typed allocation candidate | New candidate carrying all 12 color/restriction-bucket counts; trusted binding validates against derived provisional pool and cost | One ChooseOne response selects one all-and-only legal complete allocation; no AutoPay. |
| Order pending triggers | `Order` answer exists but V3 intents cannot identify trigger choices | New safe trigger descriptor intent bound to TriggerInstanceId; no trusted ID in request. Simultaneous admitted triggers must have injective safe descriptors; collisions fail closed and are a G0 conformance failure for a locked witness. | One Order response for each player's simultaneous group; APNAP player sequence remains rules-owned. |
| Replacement choice | No choice required by the presently characterized Ojer/Dryad events; identical duplicate Dryad replacements have equivalent outcomes | No G0 binding admitted. If a non-equivalent choice is found, amend G0 before implementation. | No agent step unless characterization proves a distinct legal outcome. |
| Combat-damage assignment (C58) | Not characterized for this G0 cut | No Decision purpose, candidate, binding, rank, or schema shape reserved | Deferred until accepted C58 characterization establishes its exact legal relation and whether contract growth is needed. |
| Accept a permission | Not a player choice for the automatic permissions identified by R1 analysis | None | No synthetic accept/decline step; later play/cast is an ordinary action. |
| Choose which stack item a rule targets | No player choice for current Ward profile; its trigger captures the original targeted StackObjectId | Trusted StackObjectId stays in trigger/stack payload | No new player binding for the current lock. A future counter spell with a player-selected stack target requires a G0 amendment. |

No new top-level answer or DecisionDomain variant is proposed. `CostOperandSelection` and `OptionalCostPayment` are typed request purposes over existing `SelectObject`/`ChooseBoolean` candidates and answer domains; trusted bindings retain the exact cost operand or resolving stack identity. The cost-operand domain is sound and complete over profile-legal current incarnations and rejects stale/fabricated choices without mutation. Required growth is the closed request purpose/context, candidate-intent/binding vocabulary, and comparator. If implementation shows a candidate value or target group cannot be described completely and safely by these shapes, stop and amend G0 rather than encoding it as a mode, boolean, string, or hidden ordinal.

The successor comparator preserves all existing V3 visible-intent ranks and appends the new ranks, so predecessor candidate meaning is never reordered: `0 pass_priority`, `1 play_land`, `2 cast_spell`, `3 activate_ability`, `4 select_object`, `5 select_player`, `6 select_mode`, `7 choose_boolean`, `8 declare_number`, `9 confirm`, `10 select_cost_route`, `11 select_mana_source`, `12 finalize_mana_production`, `13 select_mana_payment`, `14 select_trigger`. Within new ranks it compares CostRoute class (`normal=0`, `alternative=1`) then its profile-local alternative ordinal; source descriptors by authorized opaque object/ability IDs and the 12-count output vector; the finalization singleton; the fixed 12-count payment vector lexicographically; then the typed public trigger descriptor tuple by closed trigger-kind rank, event-specific safe subject tuple, and authorized opaque source object/ability IDs. Two route descriptors with one route key but different printed-symbol values compare equal and reject as duplicate public keys. `SafeTargetDescriptorV1` uses variant rank `object`, `player`, `stack_item`, then visible payload numeric order; target vectors preserve captured target-slot order. AttackDeclared subject pairs are ordered by attacker opaque ID before comparison. No comparison uses trusted bindings, TriggerInstanceId, StackObjectId, profile lookup order, or collection order. Duplicate public ordering keys reject before candidate IDs are assigned densely; for trigger ordering, any such collision is a failed request and a conformance failure if reached by an admitted locked interaction.

## 7. State, delta, event, and digest growth

### 7.1 State ownership and canonical forms

The new current state is one successor aggregate. It keeps the current component ownership but replaces the closed `zones_v1`/`PersistedExecutionV3` meaning with new closed nested components:

* Zone stack records are a map encoded by numeric StackObjectId ascending; the explicit `stack_order` vector is bottom-to-top, with the final entry the top item. The vector—not ID or map order—defines resolution order. Spell records cross-check their current spell-object incarnation and Stack zone; ability/trigger records remain valid after their source leaves if their captured payload permits it.
* Pending triggers are stored by trusted TriggerInstanceId in canonical ID order, with an explicit typed pending batch/placement continuation. The ID map is lookup/canonical-storage order only. APNAP grouping and selected trigger order have their own semantic vectors.
* Temporary effects are stored by trusted EffectInstanceId in canonical order. Their rule timestamp is a typed `(creation StateRevision, operation ordinal)` only when the operation needs relative ordering; allocation order never substitutes for it. Expiry is typed and validated against CoreRulesState/turn boundaries.
* Continuations are typed and closed. The G0 continuation families are staged cast, staged non-mana activation, trigger ordering/target placement, selected typed cost operands, and a paused resolving stack item with optional payment. No generic interpreter stage, string key, or arbitrary nested payload is allowed.
* Existing next_stack_object_id, next_trigger_id, next_effect_id, next_continuation_id, next_decision_id, and next_rule_event_id are reused. Overflow rejects the transition without advancing any allocator. No new global allocator, opaque stack allocator, or hidden controller counter is introduced.

The current M4 `EngineStatePartsV2` is not extended in place with live stack/effect/trigger payload semantics: it is the accepted complete V6 state aggregate and must remain a coherent detached historical verifier value. A successor aggregate is required. It remains the one complete current runtime authority after its single activation boundary, not a sidecar to `EngineState`.

### 7.1.1 Proposed fixed-shape records

The following field order and tagged-union boundary are the proposed canonical record model. The quoted schema/domain identifiers are deliberately unassigned until the version ADR; they are not implementation names. Every array below has fixed arity per variant, every absent optional is explicit CBOR `null`, and each enumerated tag is closed.

```text
zones_successor = [
  <zone_component_identity>,
  objects_v1[], locations_v1[], ordered_zones_v1[],
  stack_records_successor[], stack_order_bottom_to_top[]
]

stack_record = [stack_object_id_u64, controller_player_id_u64, stack_item]

stack_item =
  ["spell", stack_card_object_id, card_definition_id, face_key_u32,
   semantic_profile_id, modes[], targets[], cost_facts]
| ["activated_ability", ability_source_context, modes[], targets[], cost_facts]
| ["triggered_ability", originating_trigger_id, ability_source_context,
   trigger_event_snapshot, targets[]]

mode_binding = [mode_slot_u32, selected_mode_u32]
target_binding = [target_slot_u32, target_ref]
target_ref = ["object", game_object_id]
          | ["player", player_id]
          | ["stack_item", stack_object_id]

cost_facts = [selected_route_or_null, paid_additional_cost_ids_u32[]]
route = ["normal"] | ["alternative", profile_local_route_id_u32]

execution_successor = [
  <execution_component_identity>, pending_request_or_null,
  continuations[], temporary_effects[], waiting_triggers[], delayed_effects[]
]
continuation_record = [continuation_id, created_at_revision_u64, continuation_payload_v3]
continuation_payload_v3 =
  ["synthetic_assembly", actor_player_id, stage, selected_count_or_null,
   selected_piece_keys_u32[], ordered_piece_keys_u32[]]
| ["magic_sba_graveyard_order_v1", round_start_revision_u64,
   selected_sba_actions[], apnap_owners[], next_owner_index_u32, completed_owner_orders[]]
| cast_continuation
| nonmana_activation_continuation
| trigger_placement_continuation
| stack_resolution_continuation

`ContinuationRecordV3` is the sole owner of `continuation_id` and creation revision. It has no wrapper `actor` or `stage_index`: an action actor is stored in its cast/activation payload; trigger placement derives the current actor from `apnap_actors[current_actor_index]`; a paused resolution's choice actor is the actor of the pending authoritative request; existing SBA order derives the current owner from its APNAP cursor. No child payload repeats the wrapper continuation ID. The V3 closed payload inventory retains current `SyntheticM2Assembly` behavior under the neutral tag `synthetic_assembly` and current `MagicSbaGraveyardOrderV1` behavior, alongside Cast, NonManaActivation, TriggerPlacement, and StackResolution. Predecessor V2 bytes/tags retain their historical meaning and are not relabeled or automatically migrated.

trigger_record = [trigger_id, controller, ability_source_context,
  trigger_context,
  target_timing_tag]
target_timing_tag = "no_targets" | "captured_from_event" | "choose_on_placement"

trigger_placement_continuation = ["trigger_placement", apnap_actors[],
  current_actor_index_u32, pending_trigger_ids[], completed_orders[],
  selected_trigger_targets[], actor_request_roots[[player_id, first_player_decision_id][]]]

cast_continuation = ["cast", actor_player_id,
  spell_object_id, card_definition_id, face_key_u32, semantic_profile_id,
  stage_tag, selected_route_or_null, modes[], targets[], paid_cost_choices[],
  action_cost_facts, mana_payment_staging_or_null]
cast_stage_tag = "selecting_cost_route" | "selecting_modes" | "selecting_targets"
              | "selecting_additional_costs" | "selecting_cost_operands" | "paying_mana"
nonmana_activation_continuation = ["nonmana_activation", actor_player_id,
  source_object_id, source_ability_instance_id, ability_key_u32,
  semantic_profile_id, stage_tag, modes[], targets[], action_cost_facts,
  mana_payment_staging_or_null]
nonmana_activation_stage_tag = "selecting_modes" | "selecting_targets"
                             | "selecting_cost_operands" | "paying_mana"
action_cost_facts = [mana_cost_or_null, reserved_nonmana_costs[], selected_cost_operands[]]
mana_cost = [colored_wubrg_counts_u32[5], colorless_count_u32, generic_count_u32]
reserved_nonmana_cost = ["tap_source"] | ["sacrifice_source"]  // only locked witnesses
selected_cost_operand = ["put_counters", game_object_id, "minus_one_minus_one", count_u32]
mana_payment_staging = ["mana_payment_staging", payment_stage_tag,  // SelectingSources | AwaitingFinalAllocation
  mana_source_activations[]]  // embeds no duplicate cost or derived pool
stack_resolution_continuation = ["stack_resolution", resolving_stack_object_id,
  resolution_stage_tag, action_cost_facts_or_null,
  mana_payment_staging_or_null]
resolution_stage_tag = "awaiting_optional_payment" | "paying_mana"
mana_source_activation = [source_object_id, source_ability_instance_id,
  ability_key_u32, semantic_profile_id, activation_cost_receipt,
  produced_buckets_u32[12]]
activation_cost_receipt = ["tap_source"]
payment_choice = [spent_buckets_u32[12]]

temporary_effect_record = [effect_id, affected_game_object_ids[], operation,
  expiry, timestamp_or_null]
operation = ["power_toughness_delta", power_i32, toughness_i32]
         | ["grant_keyword", temporary_keyword]
temporary_keyword = "haste" | "double_strike"
expiry = ["until_end_of_turn", turn_number_u64]
timestamp = [creation_state_revision_u64, operation_ordinal_u32]
```

The G0 source capture uses these closed records:

```text
SourceContext = { snapshot: ObjectSnapshot, face_key: FaceKey,
                  semantic_profile_id: CardSemanticProfileId }
AbilitySourceContext = { source: SourceContext,
                         ability_instance_id: AbilityInstanceId, ability_key: AbilityKey }
```

`ObjectSnapshot` binds the old incarnation and printed definition/owner/controller/location facts; the other fields bind the immutable face/profile/ability semantics. The source zone remains the captured `ObjectSnapshot.location`; no permanent/spell/source-kind tag duplicates it. `ActivatedAbilityStackItem`, `TriggeredAbilityStackItem`, `TriggerRecord`, and `TriggerEventSnapshot.AbilityActivated` use `AbilitySourceContext` directly, so source object/profile/ability facts have one owner. `DamageApplied.source` may use `SourceContext` because a damage source may be an object in any zone. It contains no derived-characteristic cache, physical-card lookup, arbitrary fact map, or second live-object record. This is sufficient for the locked source-departure cases: an activated ability continues from its captured immutable origin while any effect that refers to the original object checks that exact incarnation. If a selected profile requires an additional last-known derived fact, stop and amend G0 before admitting it.

`TriggerEventSnapshot` is the following closed event-fact union. It is captured when the trigger is created, never reconstructed later from `RuleEventId` or live source state:

```text
TriggerEventSnapshot =
  SpellCast { actor: PlayerId, stack_item: StackObjectId, spell: SourceContext,
              is_creature_spell: bool, cost_facts: CostFacts }
| AbilityActivated { actor: PlayerId, stack_item: StackObjectId, source: AbilitySourceContext,
                     targets: Vec<TargetBinding>, cost_facts: CostFacts }
| TargetBecame { actor: PlayerId, source_stack_item: StackObjectId, target: TargetRef }
| ObjectEntered { object: ObjectSnapshot }
| ObjectLeftOrDied { last_known: ObjectSnapshot, destination: ZoneLocation }
| BeginningOfCombat { active_player: PlayerId, turn_number: u64 }
| AttackDeclared { controller: PlayerId, attackers: Vec<AttackerFact> }
| CardDrawn { player: PlayerId }
| CounterChanged { object: GameObjectId, kind: CounterKindV1, before: u32, after: u32 }
| DamageApplied { source: Option<SourceContext>, recipient: DamageRecipient,
                  amount: u32, damage_kind: DamageKind }
| LifeChanged { player: PlayerId, before: i64, after: i64, cause: LifeChangeCause }

CostFacts = { selected_route: Option<CostRoute>, paid_additional_cost_ids: Vec<u32> }
AttackerFact = { object: GameObjectId, defending_player: PlayerId }
TargetRef = Object(GameObjectId) | Player(PlayerId) | StackItem(StackObjectId)
DamageRecipient = Object(GameObjectId) | Player(PlayerId)
DamageKind = Combat | Noncombat
LifeChangeCause = Damage | NonDamage
```

All numeric values use these exact bounded types: damage amount and counter totals are `u32`; turn number is `u64`; life totals are `i64`; player/object/stack identities use existing typed IDs. All vectors preserve semantic event order. `SpellCast.is_creature_spell` is the rules-derived value at the cast event, not a later live query. `AttackDeclared.attackers` records the declared attacker and defending player pairs; creature-type qualification is evaluated at the event and is not re-evaluated from later board state. `BeginningOfCombat` records the active player and current turn before beginning-of-combat triggers are detected. `CounterChanged` records the exact existing `CounterKindV1` and before/after totals; this includes Lore-counter changes required by Saga chapter detection. `DamageApplied` carries the post-replacement amount and recipient. `LifeChanged` carries actual before/after totals so life loss is distinct from damage. `CardDrawn` represents one card-draw occurrence and never captures a hidden drawn-card identity. This union is closed for the accepted Shared foundation; adding another event family or fact requires a reviewed G0 amendment.

`TriggerRecord` has no intervening-if receipt field: the locked R1 × W1 trigger witnesses contain no intervening-if clause. `target_timing_tag` has exactly three values: `no_targets` when the trigger has no targets, `captured_from_event` when the trigger itself is caused by an object becoming a target (the Ward path retains that exact target), and `choose_on_placement` when the rules require target selection as the trigger is put on the stack. Reflexive-trigger follow-up mechanics remain outside G0; if any accepted profile requires an intervening-if receipt or a fourth target-timing value, stop and amend G0 before admitting it.

There is no independent `resolution_context` field in G0 stack or trigger payloads. For the locked R1 × W1 closure, resolution is determined by the immutable semantic profile/ability identity bound in the payload, the selected mode/target slots, typed cost route/paid facts, selected cost operands, and (for triggered abilities) the captured `TriggerEventSnapshot`. No separate arbitrary profile-owned value bag is necessary. Any future clause that needs a new captured resolution value must add a named typed field/variant through a G0 amendment before that profile is admitted. Unknown event/profile/effect tags reject.

The shared `ManaPaymentStaging` record is embedded in the one owning `CastContinuation`, `NonManaActivationContinuation`, or `StackResolutionContinuation`, never duplicated as a sidecar. The parent record owns typed `action_cost_facts = [mana_cost_or_null, reserved_nonmana_costs[], selected_cost_operands[]]`; the mana cost is `[colored_wubrg_counts_u32[5], colorless_count_u32, generic_count_u32]`. Cast cost facts carry the exact determined total; activation cost facts carry the ability's exact `{R}` / `{3}{W}` and its own reserved costs such as `tap_source`. The shared staging record stores each accepted source activation in selection order. There is no partial mana spend or partial-allocation cursor: the accepted source sequence and its derived provisional pool are the complete payment progress until the player chooses one complete final allocation. The provisional mana pool is derived, not stored as a second mutable value: validation starts from current ManaState and excludes from mana-source eligibility only objects made unavailable by the parent's `reserved_nonmana_costs` (currently `tap_source` or `sacrifice_source`) and sources already selected in this staging sequence. It then applies each typed source-activation cost receipt and adds each exact 12-bucket output in vector order, rejecting duplicate, exhausted, reserved, or profile-mismatched sources. `selected_cost_operands` are validated and persisted but do not by themselves reserve their objects; only a typed operation that explicitly makes an object unavailable may do so. The fixed G0 payment domain is evaluated against that derived pool. `FinalizeManaProduction` is offered in the `ManaProductionChoice` domain only when a complete legal allocation exists; if no further source activation is legal and payment is possible, advancement to payment is forced without a synthetic choice. The selected final allocation is bound by the pending authoritative request and need not be duplicated in the continuation; `ManaPaymentStaging.stage` is the sole authority for `SelectingSources` versus `AwaitingFinalAllocation`. Final acceptance commits in the same transition, so no persisted commit-ready stage exists. At final acceptance the engine revalidates the whole sequence against unchanged authoritative sources, then applies source costs, mana production, payment and the parent cast, non-mana activation, or paused-resolution commit atomically. For the currently locked R1 × W1 mana-source witnesses, the admitted mana-source activation-cost receipt is only `tap_source`; outputs are typed 12-bucket vectors, including restricted buckets such as creature-spell-only mana. Cast, non-mana activation, and paused-resolution `action_cost_facts` represent the determined mana cost, reserved non-mana costs, and selected operands. For the cited activated-ability witnesses, these facts include `{R}` / `{3}{W}` and `tap_source`; the shared candidate validator therefore cannot offer the ability source as a mana source. A paused Ward resolution carries its `{2}` cost facts in the same common shape. A future source-ability profile whose activation cost or output cannot fit these closed records blocks admission and requires a G0 amendment; no generic cost script is implied. `StackResolutionContinuation` leaves the resolving item in its existing ZoneState stack record/order owner and stores only its trusted identity and coarse pause stage (`AwaitingOptionalPayment` or `PayingMana`); while `PayingMana`, the embedded `ManaPaymentStaging.stage` is the sole authority for source-selection versus final-allocation progress. It has no copied stack payload, profile interpreter, or arbitrary resolution state. The Ward witnesses are Skyward Spider native Ward {2} and Ward {2} granted by Sheltered by Ghosts; the W1 profile remains exclusive. The typed trigger/stack payload captures the affected StackObjectId and controller-at-trigger facts needed for resolution, even if the affected item later leaves the stack. The W1 profile supplies the authorized payer from those captured facts and whether its pay/decline choice still applies. `OptionalCostPayment` uses the existing `ChooseBoolean` intent and Boolean/ChooseOne response protocol, bound to its typed profile-local cost id: decline completes the profile resolution, while pay attaches shared `ManaPaymentStaging` using the `{2}` action cost and resumes the same resolving item after explicit source/allocation choices. No priority or unrelated action can interleave; the triggering stack item and its captured target StackObjectId remain authoritative until the final atomic resolution commit. The successor request binds the authorized payer through the captured stack-payload/controller relation and never exposes either trusted StackObjectId.

The representation records route choice and paid-cost keys because those may branch resolution; it does not persist a completed payment allocation. A cost profile must define its local route/cost keys and exact permitted fields in its own reviewed body. This is typed profile data, not a shared cost language. A profile requiring X or another captured value must add a named typed field through G0 review before admission; G0 does not insert an open-ended `captured_values` map.

The execution record retains the existing delayed-effects slot only as an empty closed array for this Shared cut; no Warp/delayed-permission payload is admitted. The stack record map is sorted by numeric StackObjectId; the explicit bottom-to-top order vector is preserved exactly. Trigger/effect maps sort by numeric identity only for canonical encoding; APNAP placement, chosen trigger orders, effect timestamps, mode/target slots, and mana-source activation sequences are semantic vectors/fields and never sort by identity allocator value. `affected_game_object_ids[]` is a set and is unique/numerically sorted. No combat-assignment candidate ordering is frozen by this G0 proposal.

### 7.2 StateDelta and authoritative events

The in-memory `StateDeltaV2` is not a persisted public wire artifact, but its replacement type and closed semantic operation/event vocabulary are part of the current Rust transition contract. The successor delta type is reviewed with the successor state aggregate; no separate JSON schema is implied.

Minimum semantic operations/events are grouped by actual mutation, not one event per field:

* stack item create/remove with origin and typed payload identity; stack-order replacement/position proof;
* typed continuation advance/removal and selected cost-operand capture; these change authoritative State/Delta but do not fabricate a Magic event or public observed event for a private choice;
* mana pool spend/production/emptying and full atomic cost commit, with typed payment bucket changes;
* counter/tap/sacrifice/zone transitions and existing entry/LKI facts;
* target declarations tied to exact stack/action source and target incarnation;
* cast and non-mana activation commit; rule-relevant mode/cost outcome receipt, including typed selected cost operands;
* paused stack-resolution continuation advance; optional payment outcome (paid/declined) and eventual resolving-item completion/counter result;
* trigger detection/creation, pending trigger removal, explicit APNAP/order result, trigger-to-stack placement;
* temporary effect create/expire and typed effect operation;
* one authoritative result event for damage/life/counter/zone consequences after replacement application.

`SpellCast` and `AbilityActivated` identify a stack object created by that same transition: the identity is absent from the before-state, present in the after-state with the exact typed payload, and paired with one exact `StackItemCreated` operation. These events cannot report a cast or activation against a pre-existing stack record.

`AbilityActivated` carries one explicit rules-derived `once_per_turn_use_committed: bool` receipt. `true` means the bound immutable ability profile has a once-per-turn use restriction and this activation committed its history fact; the same transition adds exactly `(source GameObjectId, AbilityKey)` to `TurnHistoryStateV1.once_ability_used`. `false` means it does not add that pair. The receipt is trusted event/delta evidence, not a player choice or profile interpreter input. The RulesKernel may emit `true` only when the exact bound profile authorizes it; validators bind the receipt to the matching source ability, new stack item, operation, and exact history-set change. No other ability activation may authorize that history mutation. This is generic once-use bookkeeping only; no card-specific restriction is introduced.

No generic `ReplacementState` or replacement-iteration event is persisted. Existing source-derived replacement eligibility and application remain transaction-local. A replacement-modified outcome is captured in the final semantic event and exact Delta. If one event must pause for a genuinely non-equivalent replacement choice, a typed continuation and Decision must be added before implementation.

Every new authoritative event defines cursor precondition, cursor update, corresponding Delta operation, and exact after-state projection. Event vectors are ordered by rules semantics, not BTree/hash iteration. Player observed events are projections, not event authority. A transition with missing events, event-only mutation, delta/state mismatch, or projection mismatch rejects before commit.

### 7.3 FullStateDigest canonical successor

`FullStateDigestV6` cannot absorb these values. It fixes `full-state-digest-input.v6`, `mtgml.full-state-digest.v6`, the 14-element top-level input, `zones_v1` stack records with four fields, and `PersistedExecutionV3` with empty effects/triggers/delayed arrays. The typed validator currently rejects nonempty arrays. Admitting typed nonempty records or changing `zones_v1` would alter the closed semantic input; current V6 golden bytes and verifier remain exact.

The successor digest uses the accepted independent envelope and codec unchanged (`mtgml.digest-envelope.v1`, unkeyed SHA-256, `mtgml.canonical-cbor.v1`) with a newly assigned domain and input-schema identity. The proposed top-level value remains a fixed 14-element array, preserving V6 component order but replacing the identity header and closed `zones`/`execution` children:

```text
[
  <successor_input_schema_id>, <successor_digest_domain>, revision_u64,
  core_v1, zones_successor, allocators_v3, execution_successor,
  random_v1, knowledge_v2, perspective_identities_v2, combat,
  foundation_sources, format_v1, card_rules_authoritative_state_v1
]
```

At G0a this was a proposed preimage layout with no allocated V7 identity. G0b subsequently accepted ADR 0056, which assigns `FullStateDigestV7`, `FullStateDigestInputV7`, `full-state-digest-input.v7`, and `mtgml.full-state-digest.v7`; ADR 0056 is the canonical source for those exact names. The layout here remains the proposed fixed 14-element preimage. The nested Zone and Execution encodings are new closed schemas with fixed array lengths, explicit tags, numeric IDs sorted ascending, semantically ordered vectors preserved, and nullable fields encoded as CBOR `null`.

Canonical primitive rules remain the accepted restricted CBOR rules: definite-length arrays/strings/bytes, shortest integer representation, signed/unsigned ranges fixed by each typed field, no maps/floats/tags/bignums/indefinite values, strict UTF-8, bounded payload/depth/item counts, and byte-identical canonical re-encoding. There is no host endianness: CBOR's network-order integer/length representation is used. Enum discriminants are exact closed tags in fixed schema order; Rust enum declaration order and Serde output are never hashed. Digest values nested in other preimages remain 32-byte byte strings. Per-item counts and encoded records remain subject to existing 64 MiB/resource bounds unless an accepted contract explicitly lowers them.

The successor semantic input must include stack payload, trigger payload, temporary records, pending continuation partial choices, relevant identity allocators, stack/trigger/effect ordering, and all existing M4 state. Every field mutation must change the successor digest; trusted identity renaming must not alter safe player bytes. G0a left the digest domain/schema names unassigned; accepted ADR 0056 now assigns them, and the exact V7 input shape and writer/verifier evidence remain owned by G0c/G0e.

## 8. Checkpoint, fork, and replay

The current `EnvironmentCheckpointV7` embeds `EngineStatePartsV2` and FullStateDigestV6. It cannot checkpoint the successor aggregate. A new complete checkpoint contract and checkpoint-digest identity are required. The new checkpoint binds the successor state digest, status, complete environment-limit counters, codec identity, and full unchanged `ExecutionIdentityV1` before restore admission. Restore validates the complete nested state, contract/catalog binding and both digests before mutating a backend. The existing in-memory codec family may be reused only with a new codec semantic version after compatibility review.

Replay V7 step/initial/final fields directly type FullStateDigestV6 and CheckpointDigestV7; its schema inventory closes over Decision V3, DecisionResponseV2, ObservedEvent V3, and PlayerStep V3. It cannot reinterpret new state or new wire semantics in place. A successor replay family binds successor digest/checkpoint/request/response/event/step identities and continues to carry one explicit response per agent step. It retains global before/after StateRevision values internally; these are never player products. Observations and events remain outputs; no replay command is reconstructed from PlayerStep/ObservedEvent bytes. Old Replay V7 remains detached readable/verifiable under its exact meaning and archived runtime, not current execution authority after activation.

The direct/restore/fork/replay equivalence tuple includes complete state and digest, status/counters, event and StateDelta sequence, next authoritative/player request and bindings, public events, observation/information bytes, and final replay identity. Every staged-choice boundary (before cast, between target/mode/route/mana-source/payment steps, selected Blight cost operand, pending Ward pay/decline and mana-payment stage, pending trigger order/targets, after stack commit, before expiry) needs restore/fork/replay vectors.

## 9. Decision, information, wire, and Python compatibility

No new `DecisionDomain` or `DecisionAnswer` algebra is required: existing ChooseOne/Many/Number/Order remains the answer envelope. C58 candidate completeness, ordering, and binding are deferred; no combat-assignment wire shape is frozen here. The closed V3 candidate intent/binding vocabulary and CandidateOrderingV2 cannot represent typed payment plans, route selection, trigger-order items, or mana-source activation/payment values. C58 combat-damage assignment is outside this contract until its exact legal relation is characterized and accepted. A successor request identity is required, with:

* a closed `DecisionPurpose`/target-slot context so Boolean/Number/selection steps have unambiguous meaning;
* same-actor parent linkage via existing perspective-local PlayerDecisionIdV1, while trusted ContinuationId stays hidden;
* typed visible cost-route, mana-source activation/finalization, mana-payment-allocation, and trigger-choice intents plus exact trusted bindings;
* a new canonical ordering contract using only authorized public values; duplicate/unresolvable public keys fail closed;
* exact all-and-only legal domains, stale/fabricated/out-of-domain rejection, and one agent response for each meaningful choice.

Existing CastSpell/ActivateAbility, SelectObject/SelectPlayer, SelectMode, and ChooseBoolean are reused only where their current meanings are exact. The closed answer union stays unchanged, but DecisionResponseV2 cannot remain the successor wire value because it echoes global StateRevision; the successor response binds PlayerDecisionIdV1 plus the perspective-local view sequence. It remains one explicit response per replay step. Permission is not a separate Decision unless its actual rules text supplies one; automatic permission creation is a rules transition. No CardDefinitionId, raw FaceKey/AbilityKey, trusted object/stack/trigger/effect ID, physical-card ID, continuation, global StateRevision, or allocation history crosses the player boundary.

The public event union needs a successor for safe stack-item add/remove/cast/activation/target/effect lifecycle observations and any relevant mana-spent outcome that cannot be represented by current V3 variants. The event projection may use current opaque object/ability identity and public stack position; it never serializes StackObjectId, TriggerInstanceId, EffectInstanceId, GameObjectId, AbilityInstanceId, or trusted candidate bindings. PlayerStep requires a successor because it composes the new request and public-event families.

The basic-land observation codec is closed and has no stack/effect records. Introduce a separately named closed Magic shared-execution payload codec. ObservationEnvelopeV1 cannot remain the successor because its `state_revision` is global; a successor envelope uses `view_sequence` and omits global revision. PlayerInformationStateV2/InformationStateDigestV2 also cannot be extended in place because their V2 digest input includes global `state_revision`. A successor PlayerInformationState/InformationStateDigest uses the existing per-perspective `next_visible_sequence` as its public view cursor and removes global revision. No new public-revision allocator is needed. Retained-knowledge and opaque identity semantics otherwise remain unchanged. Any new retained opaque stack/trigger identity would require separate State/Digest growth and is not proposed.

Rust owns all semantic validation and candidate generation. Python receives strict request/event/PlayerStep DTO codecs only. The public request, event, PlayerStep, observation payload, replay, and catalog vocabulary require coordinated Rust/Python/schema positive/negative fixture parity. Do not hand-edit generated contract vocabulary. G0 pairs admitted worlds that differ in hidden library/card identity/order, global revision, allocator/RNG history, and trusted stack/effect identities while their authorized products agree. Pairs that differ in profile-dependent continuations, trigger facts, target/cost choices, or mana-source staging are deferred to the first Shared Rules batch that admits those requests; the batch must prove its own exact paired-world noninterference before activation.

### 9.1 Information-safety matrix

| Authoritative datum | Player-visible? | Projection | Noninterference obligation |
|---|---|---|---|
| StackObjectId, TriggerInstanceId, EffectInstanceId, RuleEventId | Never directly | Stack order position only where public; opaque object/ability references for known sources | Renaming these trusted IDs while preserving relations leaves all player bytes, domains, ordering, and events equal. |
| Spell card on stack / public spell face | Yes under normal public stack rules | Perspective-local OpaqueObjectId and an authorized known-definition/face fact; no CardDefinitionId or FaceKey | Hidden worlds equal until a rules-authorized reveal/cast; public spell identity differs only when the rules reveal it. |
| Ability/trigger source | Only to perspectives authorized to know it | OpaqueObjectId/OpaqueAbilityId or typed public source descriptor; no GameObjectId/AbilityInstanceId/AbilityKey | A departed/hidden source cannot be reconstructed from a trusted identifier; private pending trigger facts go only to its authorized decision actor. |
| Stack order and public target list | Public when the spell/ability/trigger is on stack | Ordered payload list with opaque object/player references; no stack object ID | Equal public stacks produce equal order and target projection regardless of internal allocation history. |
| Pending trigger payload/order choice | Only the ordering/targeting actor before placement, then public as rules require on stack | Actor-only request with typed trigger descriptors; after placement project a public stack item | Paired worlds differ only for the authorized actor where hidden trigger facts are authorized; no opponent candidate/event leak. |
| Temporary effect | Public only to the extent its effect is public | Typed bounded operation and duration over OpaqueObjectId; no EffectInstanceId/profile executor metadata | Same authorized public effects yield equal observations/events even if effect IDs differ. |
| Mana payment allocation | Acting player while choosing; only public consequences after commit | Actor-only typed candidate of color/restriction-bucket counts; later pool/stack/cost facts per rules | Opponents receive no provisional allocation; after commit, only public pool and cast/payment results are projected. |
| Blight cost operand | Activating player while choosing; public counter change at commit | Opaque controlled-creature identity plus typed `-1/-1 × 2` operation; trusted incarnation stays in continuation | Opponent products do not expose provisional selected object before the atomic activation-cost commit. |
| Paused Ward resolution | Ward decision actor only until public stack/counter outcome | Pay/decline request and mana candidates in that actor's Decision; trusted resolving/target StackObjectIds remain hidden | Other perspective outputs remain equal across private pay/stage counts until the resolution outcome becomes public. |
| Partial cast/activation/trigger choices | Current actor only | Successor request purpose, safe candidates, same-actor parent PlayerDecisionId; trusted ContinuationId remains hidden | Other endpoints' request/event/information bytes are equal until a public event commits the action. |
| Source LKI / trigger-event snapshot | Not as raw state | Only rule-public facts from the captured profile are projected; full snapshot stays trusted | Changing unobservable captured details cannot change unauthorized output; any permitted public difference has a pinned rules cause. |
| Face-up exile and public zone transitions | Public | Opaque identity plus authorized public known-definition/location facts | A card is not kept hidden solely because it originated in a hidden zone; hidden library identity remains private before reveal. |
| Seed, RNG cursor, allocator history, checkpoint, continuation body, capability/profile internals | Never | No projection | Paired states differing only in these trusted internals produce identical authorized outputs where public semantic facts agree. |

The successor PlayerInformationState remains separate from both FullGameState and PlayerObservation. Its current observation child records the exact new codec, bytes, and perspective-local view sequence; retained knowledge still uses the existing perspective-local provenance and identity lifecycle. The view cursor equals the perspective's next-visible-sequence value and advances only with authorized visible occurrences. Projection remains read-only and allocates no semantic/player IDs.

### 9.2 Public observation and event boundary

The new named observation payload retains the exact active M4.2 `MagicBasicLandObservationV1` public field family under its new codec identity: active player, turn number/position, priority, pending SBA ordering, mana pools, counters, attachments, and faces. Their existing closed meanings and field shapes carry forward; the new codec does not reuse or reinterpret `magic-basic-land-observation.v1`. The older standalone `magic-combat-observation.v2` through `.v4` payloads remain exact historical codec families and are not nested or merged by this G0 payload. G0 adds an ordered public stack view and the bounded public temporary-effect view needed to make the current legal action understandable. The stack view is top-to-bottom and each item exposes only kind, public controller, public spell/source references using OpaqueObjectId, public ability reference using OpaqueAbilityId when authorized, and rule-public modes/targets/cost outcomes. It has no StackObjectId. The temporary-effect view exposes only rule-public typed operation, affected OpaqueObjectId(s), and duration; it has no EffectInstanceId, capability key, card profile internals, or trusted source reference. Public objects already represented by retained knowledge continue through the successor PlayerInformationState contract, which preserves existing retained-knowledge semantics while removing global StateRevision.

The successor ObservedEvent envelope removes global `state_revision`; its existing per-perspective visible sequence is the only public chronology. It needs a closed public event union for `StackItemAdded`, `StackItemRemoved` (resolved/countered), `ManaPoolChanged` with a `Spent` cause, and `TemporaryEffectCreated/Expired`, plus existing V3 event meanings under a new successor identity. Stack item events use current stack position and safe source/target views; temporary-effect events use typed public operations. Cast, activation, target, trigger-placement and counter meanings are explicit event subcases or exact typed fields in those two stack-item events, never inferred from arbitrary prose. Public card zone transitions continue through ObjectMoved. Audience policy is rules-owned and the environment projects only after authoritative validation.

The new public stack and temporary-effect views use these exact closed payloads in both `MagicSharedExecutionObservationV1` and ObservedEvent V4:

```text
PublicStackItemV1 =
  Spell { controller: PlayerId, card_object: OpaqueObjectId,
          modes: Vec<PublicModeV1>, targets: Vec<SafeTargetDescriptorV1>,
          cost_facts: CostFactsV1 }
| ActivatedAbility { controller: PlayerId,
                     source_object: Option<OpaqueObjectId>,
                     source_ability: Option<OpaqueAbilityId>,
                     modes: Vec<PublicModeV1>,
                     targets: Vec<SafeTargetDescriptorV1>, cost_facts: CostFactsV1 }
| TriggeredAbility { controller: PlayerId,
                     source_object: Option<OpaqueObjectId>,
                     source_ability: Option<OpaqueAbilityId>,
                     targets: Vec<SafeTargetDescriptorV1> }

PublicModeV1 = { mode_slot: u32, selected_mode: u32 }
PublicTemporaryEffectV1 = {
  affected_objects: Vec<OpaqueObjectId>,
  operation: PowerToughnessDelta { power: i32, toughness: i32 }
           | GrantKeyword { keyword: haste | double_strike },
  expiry: UntilEndOfTurn { turn_number: u64 }
}
```

Stack arrays are top-to-bottom; `stack_position_from_top` in an event names the item's position immediately before the indicated addition/removal. The `StackItemAdded` and `StackItemRemoved` event records carry `{ stack_position_from_top: u32, item: PublicStackItemV1 }`; removal adds `cause: resolved | countered`. `ManaPoolChanged` retains its existing `pool_after` value and has the closed cause set `produced | emptied | spent`. `TemporaryEffectCreated` and `TemporaryEffectExpired` each carry one `PublicTemporaryEffectV1`. V3 event shapes otherwise retain their exact member meanings with the V4 envelope/variant identity.

For every perspective projection, the spell's `card_object` is that perspective's visible OpaqueObjectId; no definition/face key is embedded in the stack record, and authorized card identity comes through retained knowledge. Ability source opaque IDs are nullable only when not authorized; a present `source_ability` requires a present visible `source_object`. Mode vectors on spell and activated-ability stack items are ascending by unique `mode_slot`; target vectors preserve their captured target-slot order and use SafeTargetDescriptorV1. `affected_objects` is ascending and duplicate-free. The temporary-effect view list is sorted lexicographically by affected-object vector, operation rank (`power_toughness_delta=0`, `grant_keyword=1`), typed operation payload, and expiry turn; equivalent public records may repeat and their multiplicity is preserved. For `PowerToughnessDelta`, compare signed `power_i32` then signed `toughness_i32`; for `GrantKeyword`, compare `haste=0` before `double_strike=1`. Expiry compares unsigned numeric turn number. No stack/trigger/effect instance ID, source profile/AbilityKey/CardDefinitionId, trusted target ID, timestamp, allocator order, or continuation field is projected. A field that is not authorized to the perspective makes that projection omit the containing public fact only where the rules permit omission; otherwise projection fails closed. It never replaces hidden identity with a trusted ID.

The successor PlayerStep keeps the existing submission/status/information/event/next-decision composition principles with successor child types but contains no global StateRevision. It never contains trusted candidate bindings, a continuation, stack object identity, or a second rules command. Its information/event chronology uses the perspective-local view sequence. PlayerInformationStateV2 and PlayerStepV3 remain exact historical readers.

The public view cursor shapes are:

```text
ObservationEnvelopeSuccessor {
  schema_version, perspective, view_sequence: VisibleSequence,
  payload_codec, payload_base64, ObservationDigest
}
PlayerInformationStateSuccessor {
  schema_version, perspective, current_observation,
  next_visible_sequence, retained_knowledge, InformationStateDigestSuccessor
}
ObservedEventEnvelopeSuccessor {
  schema_version, sequence: VisibleSequence, event
}
PlayerStepSuccessor {
  schema_version, information_state, observed_events,
  next_decision, status, submission
}
```

`current_observation.view_sequence` equals `information_state.next_visible_sequence`; the InfoState digest binds both the view sequence and observation envelope. The event vector contains strictly increasing event sequences below the after-state next-visible cursor. The next request carries the same actor view sequence, but no global state revision. The authoritative pending request and replay/checkpoint carry StateRevision privately. A public state/projection change without an authorized visible event/cursor advance is an invariant failure; hidden stages consume no sequence for an unauthorized perspective. This uses the already checkpointed perspective-local cursor, not a global revision rewritten or a new per-player counter.

### 9.3 Profile-dependent candidate-domain admission

Exact visible-to-trusted binding equality does not prove that a profile-dependent candidate domain is sound or complete. Route symbol counts, legal mana-source outputs/costs, payment allocations, legal target sets, Blight creature operands, and profile-local optional costs must be rederived from the exact immutable Rules/profile view. State alone does not own those card/profile semantics and must not duplicate or infer them from card names, request descriptors, trusted bindings, or profile labels.

The G0 implementation baseline contains only the accepted basic-land profile in Card IR. It has no R1/W1 semantic profile handlers that can independently derive the locked decks' spell routes, mode/target domains, mana-source outputs, payment allocations, Blight operands, or optional costs. A public context value containing a caller-supplied request and caller-supplied identity IDs is not evidence of domain completeness; the rejected `ProfileDecisionDomainContextV1` proposal at `5ddd7d5a54afe670366db60d9bb0711c88c35aff` demonstrated that copying the persisted request could forge the alleged proof.

Therefore G0 does not define or expose a profile-domain proof DTO or caller-supplied context-aware State/Digest/Delta API. The typed V4 request/candidate/binding vocabulary remains available as detached wire/state vocabulary. Generic state-only admission does not admit profile-dependent pending requests: `EngineStatePartsV3::validate()`, the typed FullStateDigestV7 producer, StateDelta construction/application, checkpoint save/restore, fork, replay execution, and runtime response admission all fail closed absent RulesKernel-derived exact-domain authorization. `PriorityAction`, `AttackerDeclaration`, `CastCostRoute`, `ModeSelection`, `TargetSelection`, `CostOperandSelection`, `ManaProductionChoice`, `ManaPayment`, `OptionalCostPayment`, `AbilityAction`, and `TriggerTarget` are profile-dependent, including a `PriorityAction` domain containing only `PassPriority`. The sole proposed exception is the exact previously accepted M4.2 Basic Land PriorityAction domain described in §9.4; it is not admitted until that clarification is independently accepted and G0j verifies its RulesKernel-owned admission. `SyntheticAssembly`, `SbaGraveyardOrder`, and `TriggerOrder` remain profile-independent and may use their existing typed owner and state-derived all-and-only validators.

For other profile-dependent requests, the first Shared Rules batch that produces them must add the RulesKernel-owned exact-domain derivation and its ephemeral admission path in the same reviewed semantic cut. It must derive the complete ordered request from the verified active content/rules binding, typed profile facts, and current authoritative state; it must never copy a persisted request to manufacture expected evidence. The RulesKernel/environment call path must validate that exact derived request before any state, digest, delta, checkpoint, restore, fork, replay, or response operation accepts the profile-dependent state. The proposed M4.2 exception uses this same path at G0j with only the existing verified Basic Land domain. Any transient admission value remains nonpersistent, unhashed, unprojected, and absent from replay control input. No generic callback, arbitrary extension map, profile interpreter, new contract version, allocator, or execution-program identity is permitted. If that Rules-owned derivation cannot be implemented without changing these constraints, stop and amend G0 before the affected capability is implemented.

Detached positive/negative Decision fixtures may establish wire shape and structural binding only. They do not establish authoritative candidate soundness/completeness or permit profile-dependent state acceptance. G0 tests require fail-closed behavior at generic state-only boundaries. The proposed M4.2 Basic Land route additionally requires the complete V7↔V8 semantic parity matrix in §9.4 before activation. Actual R1/W1 route, payment, target, cost, and staged-action domain completeness plus checkpoint/fork/replay parity remain entry gates for the first Shared batch that emits each such Decision family.

### 9.4 Proposed preservation of the accepted M4.2 Basic Land slice

The exact contradiction has four separate parts:

1. **Normative prohibition:** the accepted G0 §9.3 and G0f Plan explicitly classify every `PriorityAction` as profile-dependent, including pass-only requests, and prohibit admitting such pending state through the detached G0 path. That is real normative text, not merely a validator implementation detail.
2. **Validator behavior:** `DecisionPurposeV4::is_profile_dependent()` and `EngineStatePartsV3::validate()` implement that accepted prohibition. The validator is not independently wrong under the current G0 contract.
3. **Successor representation:** Decision V4 already has the purpose and closed candidate/binding shapes for `PassPriority`, `PlayLand`, and `ActivateAbility`; the M4.2 action set needs no new candidate, wire field, schema, or identity.
4. **Runtime integration:** the accepted V7 runtime still owns `EngineStatePartsV2`, request V3/response V2, and the Basic Land RulesKernel candidate producer. No G0 V8 RulesKernel/environment route yet rederives and admits that exact producer's PriorityAction under V4. This missing integration route is where G0j must preserve M4.2.

The acceptance authority is closed Issue #225 and PR #248: reviewed activation head `a43d151c636544246223c2a3f9a08a72f502e484`, merged/post-merge master `6c6ee4c9b237696c50e944cae998f85c2d358e1c`. The general G0 rule accurately describes the state-only validator but excludes PriorityAction requests needed by that already accepted M4.2 executable, Mountain and Plains under `basic-land@1.0.0`. G0j must preserve that existing bounded behavior while changing the current writer. This subsection is **PROPOSED / NOT ACCEPTED** until independent exact-head review and acceptance.

The existing successor Decision V4 representation is sufficient and remains unchanged:

```text
DecisionPurposeV4::PriorityAction / ChooseOne
  PassPriority
  PlayLand { object: OpaqueObjectId }
  ActivateAbility { ability: OpaqueAbilityId }
```

Their existing V4 trusted bindings carry `PassPriority`, `PlayLand { object: GameObjectId }`, and `ActivateAbility { ability: AbilityInstanceId }`. The M4.2 preservation admission is limited to this exact domain for the verified `basic-land@1.0.0` profile: legal pass, legal Mountain/Plains land play, and activation of the existing intrinsic basic-land mana ability. `CastSpell`, non-mana abilities, and every other profile-dependent request remain unsupported. `PriorityAction` remains profile-dependent in general; the generic `EngineStatePartsV3::validate()` and state-only acceptance APIs remain fail-closed for it.

The sole M4.2 runtime exception is a RulesKernel-owned path. It must derive the complete, canonically ordered V4 request from the verified immutable `ExecutableProfileAdmissionV1` and current authoritative state, porting the existing Basic Land candidate/domain/transition semantics to V3/V4 without changing them. The current V2 functions `derive_basic_land_candidates` and `validate_basic_land_pending_request` are the source witnesses for those semantics, not a second Rules authority. The exact domain must be validated before transition acceptance and before digest, delta, checkpoint, restore, fork, replay, or response operations admit the state. Restore and replay must rederive it under the same exact verified admission. A pending request copied from state, caller-supplied candidate context, or structural binding equality alone never authorizes admission. Any evidence used to carry this RulesKernel authorization across internal calls is transient: it is nonpersistent, unhashed, unprojected, and absent from replay input. This does not create a public context DTO, second state owner, callback mechanism, or new identity.

This clarification changes no state bytes, canonical representation, Decision V4 fields, schema, wire tag, response meaning, allocator, digest domain, checkpoint/replay identity, or execution-program identity. ADR 0056 successor names remain in force. G0j performs only the migration needed to retain the accepted M4.2 behavior; it adds no spell casting, new ability/card/profile semantics, capability promotion, or Shared S1–S7 behavior.

Before V8 becomes the current writer, G0j must pass this semantic equivalence matrix against the accepted V7 executable. Compare legal domains and trusted bindings; accepted PlayLand, zone-incarnation, intrinsic mana activation, priority, turn-step and mana-emptying outcomes; rejection nonmutation; public observations/events; deterministic candidate ordering; state/event/delta consistency; and direct/checkpoint-restore/fork/authoritative-replay outcomes. Verify `FullStateDigestV6` and `FullStateDigestV7` under their own identities; their bytes are intentionally unequal. Verify CheckpointDigestV7/V8 independently as well. Also prove unsupported profile-dependent actions continue to fail closed and paired-world privacy remains intact.

## 10. Contract compatibility and exact version disposition

The compatibility result is `VERSION_IDENTITY_GROWTH_REQUIRED` for the state/digest/checkpoint/replay and closed player request/event products. This section records the G0a semantic incompatibility analysis and why each predecessor identity cannot be extended. G0a deliberately left exact names unassigned. Proposed exact successor names and the full historical reader policy are now listed in [ADR 0056](../../adr/0056-g0-successor-version-identities-and-compatibility.md); they remain proposed until that ADR is accepted. If accepted, ADR 0056 is the canonical source for exact names/dispositions and supersedes only the `unassigned` naming statements in this G0a table. No producer is authorized by the proposed ADR.

| Current identity | Can represent G0 semantics without changing closed meaning? | G0 disposition | Historical disposition required before implementation |
|---|---|---|---|
| `EngineStatePartsV2` / `ExecutionStateV3` | No: stack payload fields are absent; V3 effect/trigger arrays are explicitly empty-only; continuation variants are closed. | New complete typed state/Execution aggregate identity required; exact name unassigned. Keep one successor writer. | Preserve exact V2/V3 detached meaning; no legacy executable engine. |
| `ZoneState` / `zones_v1` / `StackRecord` | No: V1 stack row has only id/controller/source refs. | New nested canonical zone/stack payload identity required. | V1 bytes/validators unchanged. |
| `PersistedExecutionV3` | No: fixed five-element layout explicitly rejects nonempty effect/trigger/delayed rows. | New typed execution encoding required; exact identity unassigned. No nonempty Serde extension. | V3 verifier remains exact/empty-only. |
| `ContinuationPayloadV2` | No: only two closed tags. | Successor typed cast/activation/trigger-placement/paused-resolution variants and selected cost-operand values required; exact name unassigned. | Preserve existing V2 tags and digest bytes. |
| `StateDeltaV2` / `SemanticDeltaOperationV2` | The full-replacement principle still applies, but typed replacement and audit vocabulary change. No independent persisted public schema exists. | Successor Rust DTO/operation vocabulary required as a coordinated internal API cut; no standalone wire identity. Exact names unassigned. | No stored V2 artifact reader needed; retain source compatibility only where it does not create a second authority. |
| `AuthoritativeRuleEventV2` | No current cast/stack/trigger/effect event families; internal Rust-only event records. | Successor internal event type/validator required; no independent durable event schema. | No authoritative event-log migration; replay re-executes responses. |
| `FullStateDigestV6` | No: fixed V6 headers/nested V1/V3 data and known-answer input. | New domain + input-schema identity required; assign neither numeric/name here. | V6 exact detached verifier/read-only evidence; no auto migration. |
| `EnvironmentCheckpointV7` / `CheckpointDigestV7` | No: checkpoint embeds parts V2/V6 state digest. | New checkpoint type, digest domain/input, codec semantic identity required; names unassigned. | V7 in-memory value cannot restore under new runtime; exact digest verifier remains detached. No durable state codec is claimed. |
| Replay V7 | No: initial/step/final refs and schema inventory close over V6/V7/V3 identities and DecisionResponseV2. | Successor replay family/schema inventory required; names unassigned. It binds successor request/response, event, PlayerStep, observation payload, digest, and checkpoint identities. | V7 exact detached reader/verifier only; no current replay writer/executor under successor semantics. |
| Decision request V3 / CandidateOrderingV2 | No: request context/candidate union/order vocabulary is closed and player request carries global StateRevision. | Successor request/candidate/binding/ordering family required; remove public StateRevision and add safe view context. | V3 exact reader only; paired with its exact historical response/runtime. |
| DecisionDomainV2 / DecisionAnswerV2 | Yes for one/select-many/numeric/order answer meaning. | Reuse unchanged; no new top-level domain or answer variant. | Existing tags/answer bytes retain exact meaning. |
| DecisionResponseV2 | No: required global state_revision echoes the hidden staged-step count. | Successor response keeps PlayerDecisionIdV1/view_sequence/answer and omits global StateRevision; exact identity unassigned. | V2 exact reader/verifier only; no current successor endpoint/replay writer. |
| ObservedEvent V3 | No: closed union has global state revision and lacks public stack/effect lifecycle values. | Successor event envelope/schema removes global StateRevision and adds bounded public events; name unassigned. | V3 exact reader only. |
| PlayerStep V3 | No: embeds InfoState V2, request V3, event V3 and global revision constraints. | Successor PlayerStep/schema composes view-cursor InfoState and successor request/response/event children; name unassigned. | V3 exact reader only. |
| ObservationEnvelopeV1 / PlayerInformationStateV2 / InformationStateDigestV2 | No: envelope and InfoState digest expose/bind global StateRevision. | Successor envelope/info/digest removes global revision and binds existing perspective-local view sequence; new named observation payload codec also required. Names unassigned; no new cursor allocator. | V1/V2 exact readers/verifiers only; old basic-land payload remains exact. |
| SemanticContractManifestV1 / RulesContractManifestV1 / `ExecutionIdentityV1` / MagicRules | Yes as identity structure: new accepted manifests produce new `SemanticContractIdV1` values. Dispatch family is still MagicRules. | Reuse identity shapes; bind new capability/profile semantic values and new rules scope through the existing manifest path. | Existing IDs retain exact content meaning. Do not add a new program-kind variant. |
| Capability Registry V1 / CardDefinition V1 / content identity | Not a G0 implementation surface. | Unchanged by G0; later capability/card specs own additions. | Existing 14 registry rows and card/content bytes unchanged. |
| RNG V1 / `IdentityAllocatorState` | Yes; current typed RNG and stack/effect/trigger/continuation/event allocator slots suffice. | Reuse; no new allocator. | Existing canonical fields unchanged. |

### 10.1 Explicit historical reader matrix

After the successor cut becomes current, preserve exact old meanings as follows:

| Family | Successor-runtime disposition |
|---|---|
| FullStateDigestV6 | `READABLE_VERIFIABLE_ONLY`, exact V6 verifier and fixtures; no V6 writer. |
| CheckpointDigestV7 | `READABLE_VERIFIABLE_ONLY`, exact detached V7 verifier; V7 checkpoint state is not restore-executable under successor state. |
| EnvironmentCheckpointV7 | `UNSUPPORTED` for current successor restore because it embeds EngineStatePartsV2; archived exact build is required if retained in memory. |
| Replay V7 | `READABLE_VERIFIABLE_ONLY`, exact structural/identity verifier; semantic execution requires archived matching runtime. |
| Request V3 / ObservedEvent V3 / PlayerStep V3 | `READABLE_VERIFIABLE_ONLY`; never relabelled or emitted by successor writer. |
| DecisionResponseV2 | `READABLE_VERIFIABLE_ONLY` under the successor because it echoes global StateRevision; exact V2 bytes/meaning preserved. |
| ObservationEnvelopeV1 / PlayerInformationStateV2 / InformationStateDigestV2 | `READABLE_VERIFIABLE_ONLY`; exact V1/V2 bytes and global-revision meaning preserved, no current writer. |
| V2 delta/event Rust values | No persisted wire/history identity exists. Old Rust API may be retired; no duplicate legacy runtime is introduced. |

No automatic migration is defined. A migration, if later justified, reads/verifies the source under its exact original contract, writes a new target artifact with source provenance, and never overwrites or relabels the source.

## 11. Entry, exit, and stop conditions

Future G0 implementation entry requires:

```text
G0A_ACCEPTANCE = PASS
SHARED_SPEC_REVIEW = PASS
SHARED_PLAN_REVIEW = PASS
VERSION_IDENTITY_DECISION = ACCEPTED by the required ADR/compatibility review
CONTRACT_GROWTH_BOUNDARY = ACCEPTED
EXACT_IMPLEMENTATION_BASELINE = FROZEN
```

G0a acceptance authorized preparation and review of G0b only. Those G0b conditions are now satisfied: ADR 0056 passed independent exact-head review, required PR #251 CI, merge, and post-merge tree verification. The accepted identity decision and matrices authorize G0c onward from the exact implementation baseline above. G0c–G0i remain detached from current runtime producers; only G0j may activate the successor writer.

G0 exit requires exact typed/canonical state records; one state authority; sound/complete Decision domains for every G0-admitted request; fail-closed rejection of unsupported or unadmitted profile-dependent requests; total rejection nonmutation; state/event/delta/projection equality for the admitted scope; Rust/Python/schema parity; noninterference; direct/restore/fork/replay parity for every admitted state; exact historic compatibility; and exact-head CI and independent review. If §9.4 is accepted, the exact RulesKernel-admitted M4.2 Basic Land domain and its V7↔V8 state/checkpoint/replay parity must also pass before the G0j current-writer activation. Each later Shared Rules batch must establish exact profile-domain soundness/completeness and staged restore/fork/replay parity before it emits or admits its profile-dependent request. G0 unblocks Shared S1–S7 contract use only. It does not implement game semantics or complete M4/R1/W1.

Stop and amend this design if any source finds a second owner, a new allocator, a needed hidden callback, a new source-departure fact not representable in the typed payload, an unhandled decision descriptor collision, a history/digest mismatch, a public stack/effect projection that leaks trusted identity, a non-replayable staged action, or a historical value whose meaning would change. If a later card/profile requires a new payload tag, decision binding, observation field, or persistence value, update G0 and re-review it before implementing that profile.

## 12. Relationship to Shared S1–S7 and M4

G0 owns only the successor contract boundary. It adds no capability key/version or registry evidence. S1 derives current characteristics and live source contributions. S2/S2b reuse mana state and attachment state. S3 constructs complete safe targets. S4 writes cast/activation choices and typed stack records. S5 writes trigger records/order. S6a writes the narrow temporary records. S6b applies replacement/damage/counter/zone operations in the atomic rules transaction. S7 proves the complete interaction closure. Each later batch consumes these same G0 owners; none repeats a persistence migration.

R1-only Haste/Prowess/Warp/Ojer/Kicker/history/Trample profiles and W1-only Ward/Dryad/Aura/Role/Saga/Blight profiles remain exclusive. No W1/R1 CardDefinitions, deck manifests, full-game setup/mulligan, M4.3/M4.4 implementation, or certification is authorized by G0.

The implementation Plan is strictly subordinate to this Spec. Discovery that changes any accepted data shape, identity disposition, decision meaning, lifetime, projection, or historical support matrix means STOP → amend this Spec → independent review → regenerate the Plan.
