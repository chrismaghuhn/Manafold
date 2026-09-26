# M4 Unified Authoritative State Cut Feasibility Audit

**Status:** architecture feasibility audit; no version is allocated and no successor identity is authoritative.
**Audit date:** 2026-09-26.

## 1. Baseline and authority

Fetched origin before analysis. Current origin/master is 7a26e519a42743d2307f52d99539e1ae01ffe417. This audit branch began from that exact master. Audit branch head at start was 4d48223055e7b4274930089a56648aae285eb85c, matching origin/codex/m4-versioned-contract-growth-audit. The accepted growth audit commit is not merged into master. It is used as an accepted architecture input, while current master remains the code/normative implementation baseline. No unrelated worktree was reset or changed.

Tracker authority: #221 CLOSED, #222 OPEN, #224 MERGED at current master, #225 OPEN and PAUSED in its body pending a state/version-cut design. The current issue body records the accepted audit commit and unresolved ability-authority and observed-event work.

Authority precedence followed: current master code/tests/docs; official Wizards rules; current Scryfall Oracle/rulings; locked lists in #222; accepted audit and ADR/spec evidence. The exact locked R1/W1 composition supplied by #222 and the task is unchanged. The repository still lacks a repository-owned locked deck manifest, so #222 is the current composition authority; provenance/bundle work remains downstream.

Normative sources inspected include DOMAIN_MODEL, EXECUTION_MODEL, DECISION_PROTOCOL, INFORMATION_MODEL, STATE_HASHING, REPLAY_AND_DETERMINISM, CARD_DEFINITION_CONTRACT, CARD_IR, CAPABILITY_MODEL, ACCEPTANCE_GATES, EngineState and validation, execution/continuation/zone/identity types, Decision V2, observed event and PlayerStep DTOs, Card IR, and the capability registry.

### Current Magic source snapshots

Wizards currently serves the Comprehensive Rules effective September 25, 2026: [official rules page](https://magic.wizards.com/en/rules), [official text artifact](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt). Artifact SHA-256 is 8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca. Relevant sections include CR 101.4, 103.5, 106.4, 111, 113, 120, 122, 303.4/303.7, 400.7, 603, 605, 611, 613, 614, 704, 712 and 714. Exact clauses must be re-cited by any implementation spec that freezes a family.

Scryfall Oracle Cards bulk was updated 2026-09-25T21:01:58.069Z, archive oracle-cards-20260925210158.jsonl.gz, SHA-256 c607300fe03ce0d9f59181b1bb33d8001e2fa339b8c68eefd70a6501fa757623. Rulings bulk was updated 2026-09-25T21:00:32.670Z, archive rulings-20260925210032.jsonl.gz, SHA-256 375fb0ef1d3338055bb406930b76813b8609256cd7c9592ead940984bf59d520. All 26 normalized identities from the accepted audit were rechecked; no Oracle text changes or missing identities were found. These snapshots supersede the earlier same-day Scryfall inputs for this audit. See [Scryfall bulk data](https://scryfall.com/docs/api/bulk-data).

Locked-card census boundary (counts unchanged): R1 contains 4 Fanatical Firebrand, 4 Hired Claw, 4 Magebane Lizard, 4 Emberheart Challenger, 4 Razorkin Needlehead, 4 Hearthborn Battler, 4 Ojer Axonil, Deepest Might, 4 Nova Hellkite, 4 Burst Lightning, 4 Lightning Strike, 16 Mountain and 4 Rockface Village. W1 contains 16 Plains, 4 Ethereal Armor, 3 Spellbook Vendor, 3 Ruin-Lurker Bat, 2 Feather of Flight, 4 Optimistic Scavenger, 4 Shardmage's Rescue, 4 Sheltered by Ghosts, 4 Seam Rip, 4 Origin of Spider-Man, 4 Skyward Spider, 3 Abandoned Air Temple, 1 Evershrike's Gift and 4 Dryad Militant. Provenance aliases remain A Most Helpful Weaver → Origin of Spider-Man and Wonderweave Aerialist → Skyward Spider.

## 2. Accepted previous audit and question

The accepted growth audit at commit 4d48223055e7b4274930089a56648aae285eb85c established the H1 need for ManaState and land-play-use state; a successor full-state identity and dependent StateDelta/checkpoint/checkpoint-digest/replay families; a PlayLand request and PlayerStep successor; an ObservedEventEnvelope successor for public mana evidence; and a new Magic observation payload codec. ObservationEnvelope, ObservationDigest, InformationStateDigest, ContentContractIdV1 outer envelope, ExecutionIdentityV1, SemanticContract V1 and RNG V1 were assessed as stable absent contrary evidence.

It also left ability authority unresolved and cautioned against pre-freezing counters, attachments, face state, effects, triggers or setup. This audit asks whether a subset of those later needs has sufficiently exact semantics to be frozen with H1 in one state-identity cut, reducing repeated digest/persistence/replay work without freezing unclear generic mechanics.

Independent review correction record: current Emberheart Challenger Oracle text requires its controller's first target event each turn; current Scryfall rulings confirm retargeting and spell-copy cases count. Consequently, the closed TurnHistory field is keyed by target GameObject incarnation and the controller of the targeting spell/ability at the target event, not by target object alone or targeting-source identity. The exact control-change interaction remains a focused rules/conformance obligation. Ojer Axonil’s Oracle instruction to return “tapped and transformed” means the returning new battlefield incarnation is created tapped on the back face. It does not enter front-face-up and then transform. The current Comprehensive Rules snapshot cited above governs double-faced card face/zone behavior; this report corrects the event/state description accordingly.

Current master facts: EngineState has fixed fields for core, combat, foundation sources, zones, allocators, execution, RNG, knowledge, perspective identities and format. FullStateDigestV5 uses a closed fixed canonical input. StateDelta contains before/after FullStateDigestV5 and a full EngineStateParts replacement. Checkpoint V6 and Replay V6 bind those identities. GameObject currently carries identity, optional PhysicalCardId, CardDefinitionId, owner/controller, tapped and face_down; it has no counters, attachment, or active face. ZoneState has objects, locations, order, stack records and stack order. ExecutionState has continuation, effect, waiting-trigger and delayed-effect records, but EffectRecord is only ID+label and TriggerRecord only ID+controller; V5 digest and validation reject non-empty effects/triggers/delayed effects. ContinuationPayloadV2 is a closed enum with synthetic assembly and a Magic SBA graveyard-order variant. These placeholder fields are not semantic support.

## 3. Strategies compared

| Strategy | Proposed first identity cut | Later cuts | Semantic risk | Duplicate identity work avoided | Review and compatibility burden | Immediate next-cut risk |
|---|---|---|---|---|---|---|
| A — CUT_A only | ManaState, land-play-use, plus only the ability result once decided | Turn history/counters/face/attachment before their cards; effect/trigger/stack/permission/link later; setup may be another | Lowest initial design surface, but creates known near-term state cuts | Low | Repeated historic disposition, digest/checkpoint/replay vectors and schema inventory changes | High: likely 3–4 cuts over M4 if families are landed separately |
| B — CUT_A + mature CUT_B | Mana, a closed M4 TurnHistory, typed M4 counter kinds, attachment relation, active FaceKey; ability result included only if focused design proves state; setup only if its focused contract is accepted before cut | Complex temporary effects/permissions, linked exile, trigger/stack payload, and setup if not accepted remain later | Moderate and bounded: freezes only exact locked-pool dimensions with owners/invariants | High for M4’s first card-state families | Larger single review; old identity disposition happens once for this combined state meaning | Moderate: likely one further state cut for unresolved execution/effect families, plus setup if deferred |
| C — single M4-wide state cut | A+B plus every known state family and London mulligan/setup | Intends no further M4 state cut | High: requires freezing effect, permission, linked relation, trigger/stack and setup continuation contracts whose owners/semantics are not currently settled; pressure toward a generic effect/trigger VM | Maximum in theory | Very large semantic and persistence review; historical burden still real, only shifted earlier | Uncertain, not credibly one: discoveries at cross-deck/playable-game horizons may still force cuts |

### Recommendation

RECOMMENDATION = UNIFIED_CUT_A_PLUS_MATURE_B.

A single cut for every M4 state family is not supported by present semantic maturity. Strategy B is the best balance: fold H1 with typed turn history, the bounded counter family, attachment relation and active face state, which are all demonstrably required by locked R1/W1 and can be represented with closed typed data. Do not fold the open-ended execution families or London setup merely to avoid another version. The initial cut must remain blocked until ability authority is resolved enough to know whether it changes EngineState/PerspectiveIdentity.

This recommendation expects 2–3 M4 state-identity cuts: first the unified mature cut; a later cut for unresolved stateful execution families; possibly one for setup if its continuation contract remains deferred. This is a structural range, not an engineering-hours or guaranteed delivery estimate.

## 4. Readiness matrix

“Ready” means suitable to specify in the first unified cut, not already implemented. “No new state” still allows new immutable CardDefinition/profile content, typed transition products, or executable capability work.

| State family | Locked cards requiring it | New authoritative state? | Exact reusable model understood now? | First unified cut disposition | Reason |
|---|---|---:|---:|---|---|
| ManaState | Mountain, Plains; Rockface Village later | Yes | Yes, closed six colors and two proven mana restriction classes | READY_TO_FREEZE_IN_FIRST_M4_CUT | Per-player pool; public; empties at each step/phase; generic is cost, not mana type. Rockface proves creature-spell-only restriction. |
| Land-play history | Mountain, Plains and every land in R1/W1 | Yes | Yes, bounded normal land entitlement/use for locked pool | READY_TO_FREEZE_IN_FIRST_M4_CUT | Cannot reconstruct after played land leaves battlefield. Fold as a named TurnHistory field, not separate arbitrary history. |
| Generic M4 TurnHistory | Hired Claw, Magebane Lizard, Hearthborn Battler, Emberheart Challenger, Ojer Axonil, Ruin-Lurker Bat; land play | Yes | Yes, if the exact finite field set below is frozen and no arbitrary key/value extension is allowed | READY_TO_FREEZE_IN_FIRST_M4_CUT | Player/turn facts survive source departure; object facts are incarnation-scoped and pruned when that incarnation ceases; all relevant facts cross Decision boundaries. |
| CounterState | Hired Claw, Optimistic Scavenger, Origin of Spider-Man, Abandoned Air Temple, Evershrike’s Gift | Yes | Yes for exactly +1/+1, -1/-1 and lore counters | READY_TO_FREEZE_IN_FIRST_M4_CUT | Closed set, per current object incarnation, explicit clearing/annihilation/canonical form. Prowess is a temporary effect, not a counter. |
| AttachmentState | Ethereal Armor, Spellbook Vendor Role, Feather of Flight, Shardmage’s Rescue, Sheltered by Ghosts, Seam Rip, Evershrike’s Gift | Yes | Yes for an M4 relation-only contract, with explicit timestamp and SBA obligations | READY_TO_FREEZE_IN_FIRST_M4_CUT | Keep relation distinct from generated continuous effects and linked exile. Role newest-timestamp rule requires a timestamp in this family. |
| Generated token identity | Spellbook Vendor Role; Origin of Spider-Man Spider | No additional identity fact proven | Existing GameObjectId + CardDefinitionId + physical_card=None may suffice | NO_NEW_AUTHORITATIVE_STATE_REQUIRED | Token creation/characteristics need RulesKernel support, but do not create TokenInstanceState absent a demonstrated missing fact. |
| Face/transform state | Ojer Axonil, Deepest Might | Yes | Yes: current FaceKey on the current GameObject incarnation | READY_TO_FREEZE_IN_FIRST_M4_CUT | FaceKey already exists in immutable CardDefinition; transform changes face without zone incarnation; zone change uses new object/front-face rule. Use FaceKey, never transformed:bool. |
| TemporaryDuration / ContinuousEffectState | Rockface Village, Emberheart Challenger, Origin of Spider-Man, Sheltered by Ghosts and static Aura effects | Yes for resolving temporary effects; static effects themselves are derived | Not as one general state family | NOT_READY_FOR_FIRST_M4_CUT | Must handle duration, source dependency, timestamp/layers, characteristic deltas and granted abilities. A generic EffectRecord label is insufficient; no universal effect VM. |
| PermissionState | Emberheart Challenger, Nova Hellkite Warp | Yes | No | NOT_READY_FOR_FIRST_M4_CUT | Exiled-card permission differs in grant timing, permitted zone/action, owner/controller, and end-turn/next-turn expiry; exact owner and target identity need a typed design. |
| LinkedExileState | Sheltered by Ghosts, Seam Rip | Yes | No | NOT_READY_FOR_FIRST_M4_CUT | Need source incarnation, exiled object/physical identity, return condition, token cessation, LKI and new incarnation on return. Not an attachment edge. |
| TriggerState / payload | Most R1 creatures and W1 ETB/combat/end-step abilities | Yes, current placeholder insufficient | No | NOT_READY_FOR_FIRST_M4_CUT | Must capture source/controller-at-trigger, event values, target/context, APNAP ordering and response continuation. Current TriggerRecord is not executable. |
| Stack payload/state | Burst Lightning, Lightning Strike, activated/triggered abilities and spells across both decks | Yes, current StackRecord insufficient | No | NOT_READY_FOR_FIRST_M4_CUT | StackRecord has source/controller only; no spell/ability payload, targets, costs or resolution context. |
| DelayedEffectState | Nova Hellkite Warp; delayed end-step effects | Yes | No | NOT_READY_FOR_FIRST_M4_CUT | Due-step identity, effect payload, source independence, permission creation and cleanup need a typed bounded design. |
| Target-context state | Damage spells, targeted triggers/abilities, Aura spells | Yes while on stack/continuation | No, but belongs in typed stack/trigger/continuation payload, not independent target-history map | NOT_READY_FOR_FIRST_M4_CUT | Targets must persist and be revalidated/resolved under rules; no trusted-ID exposure. |
| ReplacementState | Dryad Militant; Ojer Axonil | No separate persistent state for these static source-derived effects | Yes for source-derived statics: derive from live battlefield object + immutable profile/current face | NO_NEW_AUTHORITATIVE_STATE_REQUIRED | CR 614 replacement effects apply as events happen. No locked M4 temporary replacement/choice record is proven. Implementing the policy remains work. |
| Ability authority | Mountain/Plains intrinsic mana abilities and activated abilities throughout pool | UNKNOWN | No; currently AbilityInstanceId does not bind source/content/face/key | UNKNOWN_PENDING_FOCUSED_DESIGN | Do not choose persistent registry or derivation as proven. May affect Decision/PerspectiveIdentity and state digest. Resolve before cutting or implementation. |
| Mulligan/setup continuation | M4.6 all games | Yes across choices | Core CR flow known; exact trusted continuation/projection/replay owner not frozen | UNKNOWN_PENDING_FOCUSED_DESIGN | Needs round, player declarations, prior mulligans, pending actor, bottom-card choice/order, shuffle cursor, knowledge invalidation and initial replay boundary. Existing ContinuationPayloadV2 is closed and single-owner. |

## 5. Turn-history consolidation

A single typed TurnHistoryState is justified. It must be a closed record, not a map of arbitrary strings. It covers the H1 land-use fact and the exact currently proven turn facts needed by locked cards:

| Named field | Required scope | Oracle/rules purpose |
|---|---|---|
| land_plays_used | Per player; bounded normal allowance for locked pool | Enforce normal land entitlement even if the played land leaves battlefield. |
| spells_cast_total | Per player | Identify second-spell event for Hearthborn Battler and support shared count. |
| noncreature_spells_cast | Per player | Magebane Lizard damage amount; includes spells cast before Lizard entered, per current ruling. |
| lost_life_this_turn | Per player boolean | Hired Claw checks whether an opponent lost life; fact remains true if life later rises. |
| red_noncombat_damage_dealt | Per player checked total | Ojer’s back-face condition; count actual noncombat damage from red sources controlled by that player when damage is dealt. Apply after replacement/prevention as appropriate to “dealt.” |
| permanent_card_to_graveyard | Per owner/player boolean | Ruin-Lurker Bat’s descended condition survives a card later leaving graveyard. |
| object_became_target | Per `(target GameObjectId incarnation, controller of targeting spell/ability at the target event)` set/marker | Emberheart’s first target by a spell/ability controlled by its controller this turn. Opponent targeting does not consume the controller’s first-target occurrence; retargeting and spell copies count under current Oracle rulings. Do not key this fact by the targeting source object. |
| once_per_turn_ability_used | Per current source incarnation + immutable AbilityKey | Hired Claw activation limit. This row depends on the unresolved ability-authority design and must not be frozen until that key/binding relationship is accepted. |

The named fields are independently updated by semantic events/products; their semantic owner is a typed current-turn ledger in EngineState. Player maps and object/key collections use ordered canonical representation. Omitted entries mean zero/false; zero-valued counter/count entries are not encoded redundantly. Checked arithmetic rejects overflow rather than saturating. The ledger carries/validates its current turn number against CoreRulesState and is reset atomically at the turn boundary. It survives each Decision, checkpoint, restore, fork and replay. Player/turn facts remain after a source leaves; object-scoped target/use marks are pruned when that GameObject incarnation ceases, because a later incarnation is a new object. For Emberheart, the target occurrence is distinguished by both target incarnation and the controller of the spell/ability that made it a target at that event. This preserves the difference between an opponent targeting it before its controller does, and its controller targeting it twice. Retargeting and spell-copy cases are included by the current rulings; do not substitute targeting-source identity for controller identity. The exact handling when control of the target permanent changes remains a focused rules/conformance obligation. The player-visible facts are public, but only safe projected values/candidates are exposed; object and ability keys stay trusted.

This model can absorb land-play usage without a separate LandPlayState. It does not absorb temporary effect durations, attachment edges, counters, stack targets, or event log. If ability use cannot be keyed safely without a new ability registry, the ability-related turn-history field and registry decision remain UNKNOWN; do not block specifying the other exact fields on that unresolved sub-question, but do not finalize the shared cut until its state impact is known.

## 6. Counter-state consolidation

The locked pool proves exactly three counter kinds relevant to persistent card semantics: +1/+1, -1/-1 and lore. Use a closed typed enum, never arbitrary text. A counter collection belongs to the current GameObject incarnation and stores only positive counts in canonical enum order. For locked M4, counters are on battlefield objects; fail closed on uncharacterized countered objects/zones. Any zone change creates a new object and counters cease (CR 122.2); transform retains counters because it is the same object. CR 122.3 requires state-based annihilation of +1/+1 and -1/-1 counters by the smaller amount. Saga lore thresholds generate chapter triggers; trigger payload is a separate deferred family. Observation exposes public counter kind/count through opaque object identity. StateDelta/events need typed counter mutation evidence. Checkpoint/digest/replay include counts and resulting events. The model is ready for the first state cut, bounded to this locked set; future counter kinds require an explicit contract change, not an extension bag.

## 7. Attachments, generated objects and face state

### Attachment relation

The relation is mutable game state; its characteristic effects are not. Model an attachment as a typed edge from the attaching current GameObject incarnation to its target current GameObject incarnation (locked M4 needs creature targets only). An attaching source has at most one target; a target can have multiple Auras. Store an explicit semantic attachment timestamp sufficient to implement CR 303.7a: if multiple Roles controlled by one player attach to a permanent, all except the most recent timestamp are put in owners’ graveyards as a state-based action. Do not infer timestamp by sorting GameObjectId or allocation order.

When source leaves, remove its edge. A target leaving creates a new object and invalidates the old edge; process the Aura illegal-attachment state action before committing a decision boundary. Current legal characteristics determine Aura target legality. Multiple ordinary Auras coexist. Role uniqueness is a separate constraint using the same relation and timestamp. The edge is not a linked-exile record and does not store +1/+1, flying, lifelink, ward, or other derived effect output. The Aura/Role definition and profile supply those semantics; characteristic queries derive them. Public observation uses opaque identities. These exact relation obligations are sufficiently bounded for the first cut because the locked W1 Aura/Role family demonstrates the need and the rules prescribe the transition/validation cases.

### Generated objects

CR 111 distinguishes tokens from counters and gives tokens object identity; tokens that leave the battlefield cannot return and cease at the next SBA check. Current GameObject supports optional physical_card and CardDefinitionId; ZoneState allocates GameObjectId. A token can plausibly use physical_card=None plus a verified generated-token CardDefinition and new GameObjectId. Thus token identity is not a new state family. Immutable token definition/content references, creation event, owner/controller/zone, and later token cessation must be correct. Before using this representation, verify definition closure and that no future rule requires origin tracking; do not persist origin or a TokenInstanceState unless a locked behavior demonstrates need.

### Face and transform

Use the existing CardDefinition FaceKey as the authoritative current face selector on the current GameObject incarnation. A DFC object on the battlefield changes FaceKey without changing GameObjectId; it is not a boolean because definitions can have multiple typed faces and the key directly resolves characteristics/abilities. On zone change a new incarnation takes the rules-defined face for its destination; outside the battlefield/stack a DFC normally presents its front face (CR 712.8a). An effect may explicitly instruct a DFC to enter the battlefield transformed. Ojer Axonil’s death ability does exactly this: after the old battlefield incarnation dies, the returning new battlefield incarnation is created already tapped with its back FaceKey. There is no subsequent in-place transform event. The new-object entry snapshot and its entry event must carry that face so ETB characteristics/triggers, replacement evaluation, observations, deltas and replay see the correct state from entry. A transform action while already on the battlefield changes FaceKey on the same GameObjectId. Validate that a selected FaceKey belongs to the immutable definition. This is a bounded generic representation and is mature for CUT_B.

## 8. Effects, permissions, linked exile and execution payloads

Temporary effects are not mere derived caches. Effects from resolution can cross multiple Decision boundaries and checkpoint/replay boundaries. Locked examples include Prowess, +1/+0 and haste until end of turn, double strike until end of turn, and temporary hexproof/lifelink grants. Some grants are source/static-profile derived; others have explicit durations. Their active layers/timestamps/source dependence and expiry need a typed M4 effect model. Store no current power/toughness cache. No accepted generic effect payload exists; existing EffectRecord label is explicitly non-executable. Therefore defer this family rather than add an arbitrary operation enum or VM to the first cut.

Permission state is distinct: Emberheart’s exiled card may be played until end of turn; Nova Hellkite’s Warp creates a later-turn permission after delayed exile. A permission needs an exact card/object identity, player, allowed action/cost context, grant source semantics and expiry. This is not safely represented as a generic boolean on an object and is not ready for CUT_A/B.

Linked exile is distinct from attachment and permission. Sheltered by Ghosts and Seam Rip require an association from the source Aura incarnation to the exact exiled object/card, return-on-source-leave condition, new object incarnation on return, simultaneous zone-transition order, and token exception. Current state has no relation. The exact reusable link owner and identity behavior need focused design; defer.

Trigger state and stack payload are materially incomplete. TriggerRecord(id, controller) does not preserve source-at-trigger, captured event values, targets, APNAP order or resolution payload. StackRecord(id, controller, optional source object/ability) does not preserve spell definition, selected face, costs/kicker, targets, or resolution context. M4 cards create many trigger families; target and event context must be trusted, serializable state across choices and checkpoints. Existing fields are placeholders rejected by current validation/digest. Do not count their existing names as implementation. Defer until a focused typed stack/trigger design.

Delayed effects need due turn/step and typed payload (Nova Warp and end-step effects); they are not just source-derived static abilities. Target context belongs inside the relevant typed spell/ability/trigger object, not a second target-state table. These all remain NOT_READY_FOR_FIRST_M4_CUT.

## 9. Replacement semantics, immutable content and derived views

Dryad Militant’s replacement is static and derives from its active permanent and immutable profile; Ojer Axonil’s damage replacement likewise derives from source presence/current face/current power. CR 614 replacement effects apply continuously as events happen, rather than being locked into a persistent registry. For the currently locked replacement clauses, no separate ReplacementState is required. Runtime still must evaluate them at the correct event point, handle applicable ordering, and produce exact before/after events. Temporary replacement or player replacement choice is not proven by these locked cards and remains deferred.

Keep these in immutable content/profile/capability, not mutable EngineState: basic land subtype/intrinsic ability, mana costs, printed keywords, static replacement definition, Aura/Role ability definition, token definition, spell/trigger semantic definitions, capability requirements and rules contract values. CardDefinition is content-scoped by ContentContractId and remains outside EngineState.

Do not persist derivable current power/toughness, mana value, “has flying?”, number of controlled enchantments, modified status, legal candidate lists, or static keyword grants. Derive from current face, immutable profile, counters, attachment edges and any accepted active effect records. Any cache is discardable and cannot affect choices/order/digest/replay. Historical legality facts are different: land plays, spell counts, life loss, targeted-this-turn, permanent-to-graveyard, damage total and once-use must be explicit typed history because they can no longer be reconstructed from current board.

## 10. Ability authority audit

Current normative domain assigns AbilityInstanceId to one authoritative ability instance. PerspectiveIdentityStateV2 persists OpaqueAbilityId↔AbilityInstanceId and allocation/retirement; EngineCandidateBinding::ActivateAbility carries only AbilityInstanceId. Neither establishes the missing semantic tuple:

source GameObject incarnation → content-scoped definition → CardDefinitionId → FaceKey → AbilityKey → controller/profile.

| Model | Benefits | Required proof / risks | Audit outcome |
|---|---|---|---|
| A — persistent instance registry | Direct reverse lookup from AbilityInstanceId; explicit source/content/face/key proof; supports stack references | Registry lifetime on source zone change/control change; identity surviving on stack; allocator and stale-entry validation; new digest/checkpoint/replay state; perspective mappings still need privacy-safe projections | Viable only if a focused design proves lifetime semantics; not authorized now |
| B — derived trusted reference | Avoids duplicating static semantic identity; source incarnation + content-scoped definition + face/key is naturally checked against current definition; stale zone incarnation fails | Existing opaque mapping expects AbilityInstanceId, so mapping/binding needs redesign or derivable stable instance identity; source leaving while an activated ability is on stack requires a captured payload; content contract identity must be in execution context or binding | Preferred direction for static mana abilities, but not proven sufficient |
| C — typed candidate binding with current-source proof and captured stack identity | Candidate binds source GameObjectId plus immutable face/key; RulesKernel resolves against verified content; stack product captures a typed ability payload if source later leaves | Changes EngineCandidateBinding and player identity mapping contract; must prove opaque IDs remain stable within perspective; may require a persistent stack identity but not a standing registry | Focused design candidate; still unresolved |

All models must reject stale source incarnations, bind controller and ability definition, preserve candidate ordering without trusted-ID tiebreaks, survive fork/replay deterministically, and never expose trusted IDs. AbilityInstanceId/opaque mappings are already in EngineState and digest, so a changed binding may alter perspective-state shape even if no new standalone registry is added. Disposition: ABILITY_AUTHORITY = UNKNOWN_PENDING_FOCUSED_DESIGN; persistent ability state = UNKNOWN. Resolve before finalizing the unified state cut or implementing the H1 mana ability.

## 11. London mulligan and setup

CR 103.5 gives the basic flow: players, in turn order beginning with starting player, declare mulligan/keep; mulliganing players shuffle hand into library, draw starting hand size, then bottom a number equal to mulligans taken in any order; a player who keeps cannot mulligan again; repeat until none mulligan. The exact rule is known, and existing RandomStateV1 can carry shuffle cursors. But authoritative state ownership is not yet ready to freeze alongside H1 because the current continuation protocol is one actor/stage with a closed ContinuationPayloadV2, while setup must represent the per-player round declarations, mulligan counts, kept status, current declaration actor, bottom selections/order and cross-player progression. It also needs hidden-hand candidate projection, knowledge/opaque-ID invalidation through shuffle, choice visibility, starting-player initial input, and a declared replay boundary for setup transitions.

A typed SetupContinuation can likely be designed without universal machinery, but this audit does not claim the DTO and invariants are accepted. Existing ContinuationPayloadV2 cannot be reinterpreted or arbitrarily extended without a state-identity cut. Setup state is NOT_READY_FOR_FIRST_M4_CUT pending focused setup/replay/privacy design. Starting-player identity remains initial/setup control input unless a later rule proves a runtime field necessary; opening hands/library order already live in zones; terminal outcome remains EpisodeStatus.

## 12. Version-cut economics

A state identity cut is structural, not a one-file digest change. The following are rebuilt or re-versioned together whenever new authoritative EngineState fields enter current state:

| Family | Strategy A repeated cuts | Strategy B mature unified cut | Strategy C attempt |
|---|---|---|---|
| FullStateDigest / canonical input | Likely 3–4 schema/KAT updates | One mature-state update; likely one more for deferred state | Intended once; high risk a discovered owner forces another |
| StateDelta / exact event parity | Updated at each state cut | One combined replacement plus later complex family | Broad state/event product review at once |
| EnvironmentCheckpoint / restore / fork | Repeat codec, admission, roundtrip and fork parity at every cut | One initial migration boundary plus any later deferred-family cut | Large snapshot/restore surface; a later cut still possible |
| CheckpointDigest | Repeat input/schema/KAT work | One first cut plus possible second | One intended update, only safe if every included field is correct |
| Replay | Repeat manifest/step/recorder and version binding | One first cut plus later cut where justified | Large replay schema and execution update including setup |
| InitialEnvironmentIdentity | Rebind each digest/checkpoint family | One initial binding plus any justified later successor | Broad setup/state identity in same review |
| Goldens / KATs | Repeatedly update state and checkpoint/replay vectors | One consolidated baseline plus later focused state family | Highest single vector-review surface |
| Python parity | Repeat DTO/digest/replay parity per cut | One combined parity baseline plus any later cut | High simultaneous cross-language drift risk |
| Restore/fork tests | Repeat state-family roundtrip and branch-equality tests | One coordinated state roundtrip plus later unresolved-state tests | Many new family combinations in one suite |
| Historical readers/disposition | Each successor adds old-family disposition | Same one-time cost for included state; later only if justified | Does not disappear; old versions remain historical |
| Schema inventory / archive-repro | Repeated identity inventory and archive checks | One coordinated inventory and archive check per actual cut | One intended update but semantics remain high risk |

Strategy A’s 3–4 range assumes successive narrow additions at H1, stateful R1/W1, and setup/complex effects. Strategy B’s 2–3 range assumes typed TurnHistory/Counter/Attachment/Face families are fully specified before the first current writer; unresolved ability authority may add state to that same first cut if design finishes before it. Strategy C’s hoped-for single cut is not a safe expectation because unresolved state owners are precisely where another cut can arise. Version count is not a correctness metric.

## 13. Contract impact matrix

This matrix compares Strategy B first cut. CUT_REQUIRED means successor/current type family required; SOURCE_TYPE_CHANGE_ONLY means code/type grows but no independent wire identity; NEW_NAMED_PAYLOAD adds a separately identified payload while preserving its envelope; UNKNOWN awaits focused design.

| Contract | Impact | Reason |
|---|---|---|
| EngineState | CUT_REQUIRED | Add ManaState, TurnHistoryState, bounded CounterState, AttachmentState and active FaceKey; ability state only if design requires it. |
| FullStateDigest | CUT_REQUIRED | Closed V5 semantic input cannot acquire new fields by reinterpretation. |
| StateDelta | CUT_REQUIRED | Before/after digest and EngineStateParts replacement types advance together. |
| EnvironmentCheckpoint | CUT_REQUIRED | Complete state snapshot changes. |
| CheckpointDigest | CUT_REQUIRED | Current input binds V5 digest. |
| Replay / InitialEnvironmentIdentity | CUT_REQUIRED | Current family directly binds old digest/checkpoint and schema inventory. |
| Decision request | CUT_REQUIRED | PlayLand closed candidate action and binding. |
| Decision response | NO_CHANGE | Generic response can submit selected candidate/answer absent a new response shape. |
| EngineCandidateBinding | UNKNOWN | Ability authority model and trusted binding tuple unresolved; PlayLand binding is known. |
| PerspectiveIdentity | UNKNOWN | Ability opaque mapping may need semantic tuple instead of AbilityInstanceId; privacy/stability implications need design. |
| ObservedEvent | CUT_REQUIRED | Closed V2 union lacks public mana-change evidence; successor must also project included public counter, attachment and face transitions where event evidence is emitted. |
| PlayerStep | CUT_REQUIRED | Embeds new decision and observed-event family. |
| Observation payload | NEW_NAMED_PAYLOAD | Named codec projects public mana, counters, attachments and face; candidate/object identities remain opaque. |
| ObservationEnvelope | NO_CHANGE | Existing codec identifier + bytes envelope can carry new payload. |
| InformationStateDigest | NO_CHANGE | New facts are in observation bytes; no new retained knowledge proven. |
| ContentContract / CardSemanticProfile | NO_CHANGE outer envelope | Use M4.1 reserved closed typed profile seam; no arbitrary body. |
| SemanticContract | NO_CHANGE | Existing V1 references new typed identities/values. |
| RulesContract | NO_CHANGE schema | Capability closure changes as typed values if current manifest accommodates them. |
| Capability Registry | NO_CHANGE schema | Add reviewed typed capability entries, not registry meta-shape. |
| RNG | NO_CHANGE | H1 and mature CUT_B state transitions are deterministic; shuffle/setup deferred. |

## 14. First-cut inclusion contract

Recommended first cut includes these state families, subject to focused spec freezing their exact fields before implementation:

1. ManaState: per-player W/U/B/R/G/C counts; finite restriction classes unrestricted and creature-spell-only; bounded checked counts; deterministic canonical ordering; step/phase emptying; public projection; digest/checkpoint/replay binding.
2. TurnHistoryState: closed named fields in section 5; keyed only by PlayerId, current GameObjectId, the controller-at-event dimension for target occurrences, and—only after ability authority resolves—AbilityKey; zero/false entries omitted; checked values; explicit current-turn reset; public event evidence and hidden-ID-safe projection.
3. CounterState: per current object incarnation, fixed kinds +1/+1, -1/-1, lore; positive counts only; clear on zone change; retain through transform; apply annihilation/Saga-trigger facts; public safe projection.
4. AttachmentState: typed incarnation-bound source→target relation plus semantic timestamp; source/target zone and legality invariants; Role newest-timestamp SBA; derived effects remain outside this record.
5. FaceState: current FaceKey on object; validate membership in content definition; change on transform without new incarnation; choose the face as part of zone-entry/new-incarnation construction when rules or an effect specify it. Ojer returns as a new incarnation already tapped and on its back face; do not emit a later transform event for that entry.
6. Ability state: no inclusion/exclusion decision until focused authority design resolves whether an explicit registry or changed PerspectiveIdentity mapping is necessary. Any required fact discovered before first cut must be folded into this same cut; do not cut first and discover this immediately afterward.

First cut explicitly excludes: TemporaryDuration/ContinuousEffectState, PermissionState, LinkedExileState, TriggerState payload, stack spell/ability payload, DelayedEffectState, target/resolution context, and mulligan/setup continuation. It adds no token identity family, replacement registry, arbitrary history/effect map, or card-name state.

## 15. Multiple implementation PRs under one identity cut

Yes, one accepted versioned state contract can be implemented through several ordered PRs. A version cut is a semantic boundary; it does not require one monolithic Git PR. Safe sequencing:

1. Contract/spec PR freezes all included field semantics, canonical layout, validators, digest/checkpoint/replay schema identities, and historical dispositions; no producer activated.
2. Detached DTO/codec and KAT PR implements candidate successor types and exact bytes without making them the current EngineState writer or executable runtime.
3. State family PRs implement validators/producers in a non-current or feature-gated path; they may not serialize new state under V5, change V5 bytes, or expose incomplete checkpoints/replays as executable.
4. Integration PR atomically switches EngineState, current digest, StateDelta, checkpoint/restore, replay, PlayerStep/ObservedEvent, observation projection and conformance to the accepted cut. No intermediate master state may have new authoritative fields omitted from its digest/checkpoint/replay identity.
5. Follow-up RED/conformance/Python/archive evidence PRs can complete the release gates while the feature remains non-production; activation/acceptance only after all required gates pass.

Avoid publishing an intermediate “current” successor whose state family or restore/replay semantics are incomplete. Feature gates must not create two competing semantic authorities or allow training on partial semantics.

## 16. Final recommendation and next task

RECOMMENDATION = UNIFIED_CUT_A_PLUS_MATURE_B.

Freeze together the H1 mana/land facts and mature locked-M4 TurnHistory, Counter, Attachment relation and FaceKey state. This is materially safer and likely avoids an immediate second state identity cut for Hired Claw, Magebane Lizard, counter-bearing W1 cards, Auras/Roles and Ojer face changes. Keep the finite field/variant set explicit. Do not include setup or general effects/stack merely to pursue one cut.

Before authorizing any version implementation, the next task should be a focused **M4 Unified State Cut Semantic Design** that must:

- resolve ability authority and whether it changes persistent EngineState or PerspectiveIdentity;
- freeze the exact TurnHistory fields/reset/update rules, including Emberheart’s `(target incarnation, targeting controller)` marker and the AbilityKey-dependent row;
- specify CounterState operations and SBA/event ordering;
- specify AttachmentState edge and timestamp ownership, Role SBA and zone-transition behavior;
- specify FaceKey storage/default/transform/zone-change and Ojer’s direct back-face, tapped new-incarnation entry;
- bind new observed-event and PlayerStep contracts, and exact public mana/attachment/counter/face projections;
- specify one coordinated canonical digest, StateDelta, checkpoint, checkpoint-digest, replay and historical compatibility cut for the included state;
- record omitted families and their required later design gates.

Keep #225 PAUSED while this unified cut is specified and independently reviewed. This audit does not authorize M4.2 production implementation or allocate any successor number.
