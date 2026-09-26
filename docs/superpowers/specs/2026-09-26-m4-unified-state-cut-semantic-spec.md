# M4 Unified State Cut Semantic Specification

**Task:** `M4_UNIFIED_STATE_CUT_SEMANTIC_SPEC`
**Status:** candidate for independent review; successor identities below are proposed contract allocations and are not current runtime authority before acceptance and implementation
**Production implementation authorized:** NO
**M4.2 status:** PAUSED
**Date:** 2026-09-26

## 1. Purpose and boundary

This specification freezes one coordinated first M4 authoritative-state identity cut for the closed state families whose semantics are mature enough: mana, bounded current-turn history, three counter kinds, attachment relations, current face, and the ability-authority register required by current opaque ability references. It establishes the exact state contract needed before Mountain and Plains can be executed through the real content and RulesKernel path.

The selected M4.2 content family is Mountain and Plains under one `basic-land@1.0.0` semantic profile, parameterized by typed basic land subtype. Both real cards are required for the proof: they demonstrate that one generic land-type rule derives distinct red and white intrinsic mana abilities without card-name dispatch. This is a bounded state/content proof, not support for the rest of either deck.

The normative execution boundary is:

```text
verified CardDefinition + closed profile
→ derived requirements and legal candidates
→ validated DecisionResponseV2
→ RulesKernel::apply against current EngineState
→ TransitionProduct (next EngineState + StateDelta successor + authoritative events)
→ environment validation of state/event/delta/projections
→ atomic commit
```

The exact typed, rules-owned internal request is a closed ephemeral `MagicActionRequestV1`:

```text
PlayLand { actor: PlayerId, object: GameObjectId }
ActivateManaAbility { actor: PlayerId, ability: AbilityInstanceId }
```

`MagicRulesKernel::apply` validates the pending V3 authoritative request and V2 response, resolves the selected trusted binding, constructs exactly one of these two variants, and produces the complete transition product. No caller may construct or submit this request directly. Card content never writes EngineState, constructs StateDelta, chooses a land, chooses a mana source, or bypasses RulesKernel. Basic-land mana abilities resolve immediately under CR 605.3b and create no stack object.

This specification authorizes neither version implementation nor card implementation. Independent Spec acceptance is followed by independent Plan acceptance; only then may the implementation issue proceed.

## 2. Authority and baseline

Repository baseline is `origin/master = 7a26e519a42743d2307f52d99539e1ae01ffe417`, fetched at task entry. The accepted architecture inputs are the independently reviewed contract-growth audit at `4d48223055e7b4274930089a56648aae285eb85c` and unified-cut feasibility audit at `19b4da74567336ecb531637dd965fd28fe657f26`. #222 fixes the locked R1/W1 content. #225 remains OPEN and PAUSED.

Normative repository sources include `docs/NORMATIVE_HIERARCHY.md`, `docs/DOMAIN_MODEL.md`, `docs/contracts/ENGINE_STATE_CLOSURE.md`, `docs/STATE_HASHING.md`, `docs/EXECUTION_MODEL.md`, `docs/DECISION_PROTOCOL.md`, `docs/INFORMATION_MODEL.md`, `docs/REPLAY_AND_DETERMINISM.md`, `docs/RULES_SEMANTICS.md`, `docs/CARD_IR.md`, `docs/contracts/CARD_DEFINITION_CONTRACT.md`, `docs/cards/CAPABILITY_MODEL.md`, `docs/contracts/WIRE_CONTRACT.md`, `docs/contracts/ML_CONTRACT.md`, `docs/contracts/ACCEPTANCE_GATES.md`, `docs/maintenance/SCHEMA_EVOLUTION.md`, and ADRs 0006–0009, 0011–0019, 0025–0029, 0038–0041, 0049–0051, 0054 and 0055.

### Rules and card-source snapshot

The current first-party Comprehensive Rules artifact is effective 2026-09-25:

```text
https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt
SHA-256 = 8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca
snapshot id = wotc-cr-2026-09-25-txt-20260925-sha256-8d860e451f20f38865b725b42d82feb714c725373dd8f3b32b8652b3eeb070ca
```

The applicable rules are CR 106.1–106.6 (mana types, pool, restrictions and emptying), 107.4a (mana symbols), 115.1 and 115.7 (target events and target changes), 116 (special actions), 117 (priority), 122.1–122.3 and 704.5q (counters and annihilation), 303.4 and 303.7/303.7a (Auras and Roles), 305.1–305.6 (land play, land entitlement, basic land types and intrinsic abilities), 500.1 and 500.4 (turn/step/phase boundaries), 601 (spell cast event counted by history), 602 (ability activation), 603 (trigger event boundary), 605.1a and 605.3a–c (mana ability criteria and immediate resolution), 701.28 (transform action), and 712.8a, 712.11 and 712.13 (double-faced card face/zone/entry rules). CR 305.1–305.3 requires a land play to be by the priority holder during a main phase of their turn with an empty stack, and says it does not use the stack; CR 305.2a defines entitlement by land plays already made this turn, not current battlefield lands. CR 305.6 supplies the intrinsic `{T}: Add {W}` / `{T}: Add {R}` abilities from Plains/Mountain land subtypes. CR 106.4 empties pools at every step and phase end.

The accepted audit pins Scryfall Oracle Cards bulk snapshot `oracle-cards-20260925210158.jsonl.gz`, SHA-256 `c607300fe03ce0d9f59181b1bb33d8001e2fa339b8c68eefd70a6501fa757623`, and Rulings snapshot `rulings-20260925210032.jsonl.gz`, SHA-256 `375fb0ef1d3338055bb406930b76813b8609256cd7c9592ead940984bf59d520`. The selected identities are the Oracle records named `Mountain` and `Plains` (the authoritative definition is the exact record from that pinned snapshot, with its Oracle UUID as source record ID and exact encoded-record digest captured when the repository-owned provenance records are authored). Their type lines are `Basic Land — Mountain` and `Basic Land — Plains`; Oracle reminder text does not replace CR 305.6 as the source of their intrinsic ability. Other rulings used by this state contract are pinned in that same Rulings snapshot: Emberheart Challenger's 2024-07-26 Valiant rulings confirm target changes and copies count if the relevant controller has not previously targeted the creature that turn.

## 3. Exact M4 scope

### Included in the first unified state contract

```text
ManaState
TurnHistoryState (closed fields below)
CounterState (+1/+1, -1/-1, lore only)
AttachmentState (typed edge plus semantic timestamp)
FaceState (current FaceKey)
AbilityAuthorityState (persistent live-source authority register)
```

AbilityAuthorityState is included because `AbilityInstanceId` already has authoritative and player-opaque identity roles, while current state has no mapping from it to its semantic source. This Spec chooses a small explicit source registry; it does not introduce a universal ability interpreter.

### M4.2 execution scope

```text
real Mountain and Plains definitions
basic-land@1.0.0 profile
legal PlayLand candidate and special-action transition
Hand → Battlefield new incarnation
basic subtype-derived intrinsic mana ability
explicit ActivateAbility candidate
 tap cost
 colored mana production
 step/phase pool emptying
 authoritative state/event/delta and safe public projection
 checkpoint/restore/fork/replay parity
 rejection nonmutation
```

M4.2 does not implement payment, casting, a general stack, general spells, Lightning Strike, or any other R1/W1 behavior. It does not claim support for Ojer, Auras, counter-producing cards, or any card merely because their state substrate exists.

### Explicitly deferred

```text
TemporaryEffectState / general continuous-effect result
PermissionState
LinkedExileState
Trigger payload
General spell/ability StackPayload
DelayedEffectState
General target/resolution context
SetupContinuation / London mulligan
full casting/payment
full R1 or W1 closure
```

These families require further M4 state cuts if exact locked-card semantics later prove them necessary. No extension field or opaque reserve is created for them.

### First closed executable profile and requirements

The first `CardSemanticProfileId` is exactly `basic-land@1.0.0`, which follows the accepted M4.1 `<name>@<major>.<minor>.<patch>` grammar. The M4.1 parser and grammar remain unchanged. The independently versioned body label remains `basic-land-profile.v1`. The ProfiledV1 body is the closed typed value:

```text
BasicLandProfileV1 { subtype: BasicLandSubtypeV1 }
BasicLandSubtypeV1 = Mountain | Plains
```

Its canonical content-CBOR body is the fixed two-element array:

```text
["basic-land-profile.v1", "mountain" | "plains"]
```

The outer M4.1 ProfiledV1 binding remains unchanged:

```text
["profiled", ["basic-land@1.0.0", ["basic-land-profile.v1", "mountain" | "plains"]]]
```

There are no optional fields, maps, extension members, opcodes, dispatch strings, or generic expressions. Each definition has exactly one face at FaceKey 0, type line `Basic Land — Mountain` or `Basic Land — Plains` matching the body, and one profile-defined intrinsic ability identity `(FaceKey 0, AbilityKey 0)`. Key 0 is a stable local identity, not a card-name switch. The body chooses the typed subtype; CR 305.6 derives `{T}: Add {R}` for Mountain and `{T}: Add {W}` for Plains. Oracle reminder text is retained as source information but is not used as an alternate ability implementation.

The selected pinned-snapshot Oracle record IDs are Mountain `a3fb7228-e76b-4e96-a40e-20b5fed75685` and Plains `bc71ebf6-2056-41f7-be35-b2e5c34afa99`. Repository-owned provenance must use the exact selected record bytes from the cited 2026-09-25 bulk snapshot and their computed SHA-256; a later printing's record or a live API response is not a substitute.

The pinned Oracle reminder texts are `{T}: Add {R}.` for Mountain and `{T}: Add {W}.` for Plains. Those text lines identify the corresponding ability for provenance/display; the rule implementation still derives intrinsic ability semantics from the typed land subtype under CR 305.6, so deleting reminder text from a normalized rendering cannot remove the ability.

The profile validator checks body variant/subtype, exact type line, one-face shape, and the unique local ability identity before content identity is calculated. Requirements are derived; authors cannot suppress them. Direct roots for either parameter value are:

```text
rules/land-play@0.1.0
rules/basic-land-mana@0.1.0
rules/mana-pool@0.1.0
```

Their recursive requirements include the existing `rules/basic-priority`, `rules/turn-structure`, and `rules/zone-incarnation` versions admitted by the verified registry. Missing nodes remain MISSING until specified/implemented/covered; a profile does not make them supported. The roots are generic reusable capabilities and apply identically to both definitions. Requirement closure, not a card-name dispatch, selects execution semantics.

## 4. Proposed contract identities and compatibility cut

These identities are proposed as the smallest coherent successors. They become authoritative only after this Spec is accepted, implementation passes, and the exact current runtime is atomically activated.

| Surface | Current | Proposed successor identity | Exact reason |
|---|---|---|---|
| Full state | `FullStateDigestV5` | `FullStateDigestV6`; `full-state-digest-input.v6`; domain `mtgml.full-state-digest.v6` | Closed digest input must bind all six new authoritative state families. |
| Delta | `StateDelta` / `SemanticDeltaOperation` | Rust DTOs `StateDeltaV2`, `EngineStatePartsV2`, `SemanticDeltaOperationV2`; no independent persisted wire schema is allocated because current StateDelta is not a persisted public wire artifact | Typed digests and replacement shape change together. |
| Authoritative event | `AuthoritativeRuleEvent` / `AuthoritativeRuleEventKind` | internal Rust DTOs `AuthoritativeRuleEventV2` / `AuthoritativeRuleEventKindV2`; no independent public wire schema | The closed trusted event set grows with typed state-family evidence. |
| Checkpoint | `EnvironmentCheckpointV6` | `EnvironmentCheckpointV7`; schema `environment-checkpoint.v7`; in-memory codec `in-memory-reference / 7` | Complete EngineState and its full identity change. |
| Checkpoint digest | `CheckpointDigestV6` | `CheckpointDigestV7`; `environment-checkpoint-digest-input.v7`; domain `mtgml.checkpoint-digest.v7` | It binds FullStateDigestV6 and checkpoint codec `/7`. |
| Replay | Replay V6 family | Replay V7 family: `ReplayManifestV7`, `ReplayStepV7`, `AuthoritativeReplayV7`, `ReplayRecorderV7`, `ReplaySchemaVersionsV7`, `InitialEnvironmentIdentityV7`; schemas `replay-manifest.v7`, `replay-step.v7`, `authoritative-replay.v7` | V6 directly types old state/checkpoint and old player schema inventory. |
| Player decision request | `AuthoritativeDecisionRequestV2`, `PlayerDecisionRequestV2`, `CandidateIntent`, `EngineCandidateBinding` | `AuthoritativeDecisionRequestV3`, `PlayerDecisionRequestV3`, `CandidateIntentV3`, `EngineCandidateBindingV3`, `AuthoritativeCandidateV3`, `VisibleCandidateV3`; wire `player-decision-request.v3`; `CandidateOrderingV2` | Adds distinct `PlayLand { object: OpaqueObjectId }` and trusted binding, and changes candidate ordering contract. |
| Decision response | `DecisionResponseV2` | unchanged `decision-response.v2` | The response is request-agnostic: same PlayerDecisionIdV1, expected revision, closed answer and request-local CandidateIdV1 work with V3 request. No answer variant changes. |
| Observed event | `ObservedEventEnvelopeV2` | `ObservedEventEnvelopeV3`; wire `observed-event-envelope.v3` | Closed event union gains public mana/counter/attachment/face values. |
| Player step | `PlayerStepV2` | `PlayerStepV3`; wire `player-step.v3` | It embeds V3 request and V3 observed-event families. It continues embedding `PlayerInformationStateV2`. |
| Magic observation payload | existing named Magic codecs | new closed codec `magic-basic-land-observation.v1` | Public mana and current state projections need a typed payload. |
| Observation envelope/digest | Envelope V1 / `ObservationDigest` | unchanged | Envelope already binds the exact named payload codec and bytes. |
| Information state/digest | `PlayerInformationStateV2` / `InformationStateDigestV2` | unchanged | The same current observation bytes and retained knowledge remain the information-state inputs; schema V3 is not needed merely because its payload codec changes. |
| Content | `CardDefinitionEnvelopeV1`, `ContentContractIdV1` | unchanged outer contract; profile `basic-land@1.0.0` and one newly specified closed ProfiledV1 body | M4.1 explicitly reserved ProfiledV1. No arbitrary payload is admitted. |
| Execution/semantic/rules identity | `ExecutionIdentityV1`, `SemanticContractIdV1`, Rules/Content Contract V1 | unchanged identity families; the M4.2 SemanticContractManifestV1 MUST carry the verified Mountain/Plains ContentContractIdV1 | ExecutionIdentity already binds SemanticContractId; ADR 0055 owns this content binding. Content ID is not copied into EngineState, FullStateDigest, or checkpoint fields. |
| RNG | `mtgml.rng.v1` | unchanged | No selected transition uses random input. |

### Historical disposition

No historical bytes are reinterpreted and no automatic migration is defined.

| Historical family | Disposition after V6/V7 current activation |
|---|---|
| FullStateDigestV5 | `READABLE_VERIFIABLE_ONLY`; exact detached V5 known-answer verifier, no current writer. |
| EnvironmentCheckpointV6 | `UNSUPPORTED` by current runtime because it embeds unversioned EngineState; historical schema/digest evidence remains, execution requires archived matching runtime. |
| CheckpointDigestV6 | `READABLE_VERIFIABLE_ONLY`; detached V6 verifier, not recomputed from V6 runtime state. |
| Replay V6 | `READABLE_VERIFIABLE_ONLY`; detached V6 structural/identity verifier, not executed by current runtime. |
| Decision request V2 | `READABLE_VERIFIABLE_ONLY`; exact old wire parse/validate; cannot become current request. |
| Decision response V2 | remains `EXECUTABLE` as the response family used by the successor request. Historical meaning is unchanged. |
| ObservedEventEnvelopeV2 | `READABLE_VERIFIABLE_ONLY`; exact old closed union remains parseable, never relabeled V3. |
| PlayerStepV2 | `READABLE_VERIFIABLE_ONLY`; exact old embedded request/event shapes remain historical. |

Historical V5/V6 goldens and fixtures remain byte-identical. Unknown successor IDs fail closed. Detached verification never creates an executable legacy EngineState.

## 5. EngineState V6 composition

The runtime `EngineState` after atomic activation adds the following closed typed components. Each is semantic state; none may be cached in RulesKernel, environment backend, or projector. The immutable content catalog remains external. Its `ContentContractIdV1` is bound by `SemanticContractManifestV1`, then `SemanticContractIdV1` and `ExecutionIdentityV1`; it is not copied into EngineState, FullStateDigest input, or checkpoint state. Definition references in EngineState are interpreted only under that admitted execution identity and its verified content catalog.

```text
ManaState
TurnHistoryState
CounterState
AttachmentState
FaceState
AbilityAuthorityState
```

All maps are `BTreeMap`/`BTreeSet` with explicit key order. Validation has two explicit layers: `validate_engine_state` proves state-local shape, player/object membership, ordering, ranges, and cross-family relationships; environment admission/restore joins a Magic `ExecutionIdentityV1` and verified content catalog to prove definition, face, profile and content-contract relationships. A Magic-admitted state has exactly one FaceKey per live content-backed object. A synthetic-compat state has no content FaceState or live ability-authority rows, and all Magic-only pool/history/counter/attachment state remains at its empty baseline. Missing, duplicate, stale, future, or unknown variants fail closed. Rejected transitions preserve every component and every allocator byte-for-byte.

No new allocator is added. Existing `next_ability_id` allocates live authority rows. Attachment ordering uses accepted transition revision plus operation ordinal, not an allocator.

## 6. ManaState

### Shape and bounds

For every player in `CoreRulesState.players`, exactly one pool exists:

```text
ManaPoolV1 {
  unrestricted: [u32; 6],          // W, U, B, R, G, C
  creature_spell_only: [u32; 6]    // W, U, B, R, G, C
}
ManaStateV1 { pools: BTreeMap<PlayerId, ManaPoolV1> }
```

Array order is fixed as white, blue, black, red, green, colorless. Index and value are typed by `ManaColorV1`; generic mana is a cost symbol, never a pool type. Zero counts are present at each fixed slot and are not omitted. The pool map key set equals the player key set. Count ceiling is `u32::MAX` per color/restriction bucket; additions use checked arithmetic and reject atomically on overflow. No saturation, wrapping, arbitrary strings, provenance metadata, snow mana, delayed triggers, or custom restriction expressions exist.

Restriction tags are exactly 0=`Unrestricted` and 1=`CreatureSpellOnly`. Each unit retains its bucket while pooled. Unrestricted units may pay any otherwise legal cost. CreatureSpellOnly units may be spent only as part of casting a creature spell, never on an ability or noncreature spell. This Spec does not implement payment or choose which unit is spent. A future payment choice must expose a complete legal payment domain and cannot add AutoPay.

### Ownership, lifecycle, evidence

The pool belongs to the player, not the producing source. CR 106.3/106.4 moves produced mana to that player's pool. CR 106.4 empties each pool at the end of every step and phase. The transition that crosses a boundary clears all nonzero buckets before the next step/phase begins; emptying a zero pool emits no mutation. No locked M4 rule preserves mana across such a boundary.

Mana is public. Observation shows each player's six color totals separated by the two closed restriction classes. This exposes no trusted IDs. Checkpoint/fork/replay and FullStateDigest bind every bucket.

Trusted events/delta operations are `ManaAdded { player, color, restriction, amount, source_object, source_ability }` and `ManaPoolEmptied { player, previous_pool }`; individual operation amounts are positive and checked. Tap cost is a separate `ObjectTapped` operation. The public event is one `ManaPoolChanged { player, pool_after }` per player per resulting pool value change; production and emptying reason is a closed enum, and the event contains no source object/ability ID. `pool_after` uses only public color/restriction/count values.

## 7. TurnHistoryState

### Closed representation

```text
TurnHistoryStateV1 {
  turn_number: u64,                          // equals CoreRulesState.turn_number
  players: BTreeMap<PlayerId, PlayerTurnHistoryV1>,
  object_target_occurrences: BTreeSet<(GameObjectId, PlayerId)>,
  once_per_turn_ability_used: BTreeSet<(GameObjectId, AbilityKey)>
}
PlayerTurnHistoryV1 {
  land_plays_used: u8,                       // only 0 or 1 in locked-M4 contract
  spells_cast_total: u32,
  noncreature_spells_cast: u32,
  lost_life_this_turn: bool,
  red_noncombat_damage_dealt: u32,
  permanent_card_to_graveyard: bool
}
```

The player map keys equal the current player set. Omitted fields, arbitrary keys, string maps, zero-count normalization and lossy overflow are forbidden. Object sets sort lexicographically by numeric `(GameObjectId, PlayerId)` and `(GameObjectId, AbilityKey)`.

`land_plays_used` counts a land put onto the battlefield as a land-play special action (including via a resolving effect if one is ever admitted), not an effect that merely puts a land onto the battlefield. The locked-M4 entitlement is one; extra land-play effects are unsupported. This marker remains set if the played land leaves the battlefield. It resets atomically at turn start before turn-start triggers/choices.

`spells_cast_total` increments on each spell cast event (CR 601), including creature and noncreature spells and irrespective of whether the spell later resolves. `noncreature_spells_cast` increments for each cast spell whose stack characteristics are noncreature at the cast event. Copies that were not cast do not increment either count. Both are checked u32 increments; overflow rejects the semantic transaction.

`lost_life_this_turn` becomes true on any actual life-loss event for that player, including damage that causes life loss; later life gain does not clear it. Life loss and life gain are distinct authoritative operations, not inferred from a net before/after total. `red_noncombat_damage_dealt` adds actual noncombat damage dealt by red sources controlled by the player at the damage event, after applicable replacement/prevention. It includes any recipient; combat damage is excluded. Checked overflow rejects. `permanent_card_to_graveyard` becomes true when a physical permanent card is put into that player's graveyard from any zone; it is evaluated using the object's last-known card characteristics and physical-card identity, not whether that card remains there. A token is not a card.

### Emberheart target occurrence

The history key is exactly `(target GameObjectId incarnation, controller of the targeting spell or ability at the target event)`. It is not keyed by target alone, the targeting source object, or AbilityInstanceId. On every event that an object becomes a target, record the pair for every targeter, including an opponent. For an Emberheart trigger, determine the targeted creature's controller at that event; the Valiant occurrence qualifies only when that controller equals the targeting spell/ability controller and the pair was absent before this target event. The pair is then recorded once. This makes an opponent's earlier target not consume the controller's first own target, while the controller's second own target does not trigger. A target change or spell copy is a new becomes-target event and is processed by the same rule (CR 115.7; pinned Oracle/rulings). The stored identity is the target incarnation and targeting controller, never the source object identity.

If control changes during the turn, the prior set is retained. For each new event, compare the controller of the target permanent and the controller of the targeting spell/ability at that event, then test the pair for that targeting controller. A prior target by player B while the creature was controlled by A remains evidence that it was targeted by a spell B controlled that turn; it does not create a Valiant trigger at that earlier event. If B later controls it and targets it again, the previous `(object,B)` pair prevents a second first-occurrence trigger. If A controls it and A has no previous pair, A's target event qualifies. No control-change action is in locked M4, but this fully specified history behavior is RED-tested as a generic rule boundary.

Target occurrence and once-per-turn ability entries are pruned only when their referenced GameObject incarnation ceases to exist. Control change does not prune them. Player history facts survive source departure. All fields reset at turn start; current turn number must equal `CoreRulesState.turn_number`. Mana clearing is separate from turn reset.

`once_per_turn_ability_used` records the source incarnation and immutable local `AbilityKey` when activation costs are paid and activation is accepted. The AbilityKey must resolve through the active ability-authority registry. The field is not executable in M4.2 for a card ability with a once-per-turn clause; it establishes only the locked typed history semantics. A source departure prunes its entry because a later incarnation is a new object.

History is trusted state and digest/checkpoint/replay input. Public observations do not expose the internal set or trusted IDs. A resulting visible trigger/decision/product conveys only authorized game facts.

## 8. CounterState

Only `PlusOnePlusOne`, `MinusOneMinusOne`, and `Lore` are admitted. Their canonical tags are exactly `0=PlusOnePlusOne`, `1=MinusOneMinusOne`, `2=Lore`. Counts are positive u32 values on current battlefield GameObject incarnations:

```text
CounterStateV1 { objects: BTreeMap<GameObjectId, BTreeMap<CounterKindV1, NonZeroU32>> }
```

Both maps sort by numeric GameObjectId and fixed counter tag order 0, 1, 2. Empty inner maps and zero counts are invalid and omitted. Every key names a live battlefield object. A zone change removes the old incarnation's counters; a new incarnation has none. Transform retains counters because GameObjectId is unchanged. Checked add rejects overflow. Remove rejects a count greater than present; exact zero removes the key.

After a +1/+1 or -1/-1 counter addition/removal, state-based actions annihilate equal numbers of the two kinds on each object as required by CR 704.5q/122.3 before the next decision. This operation is ordered in the same transition product and emits explicit typed before/after counter mutation evidence. A Saga's lore-counter chapter trigger is deferred with trigger payload; the counter family alone does not claim Saga support.

Authoritative operation `CountersChanged { object, kind, from, to, cause }` records each typed mutation; no negative or zero resulting record is persisted. Observed event `CountersChanged { opaque_object, public_counter_kind, from, to }` is emitted only for public battlefield objects. Counts and kinds are public. Observation, full digest, checkpoint and replay bind exact values.

## 9. AttachmentState

The state is relation-only:

```text
AttachmentStateV1 {
  by_source: BTreeMap<GameObjectId, AttachmentEdgeV1>
}
AttachmentEdgeV1 {
  target: GameObjectId,
  timestamp: AttachmentTimestampV1
}
AttachmentTimestampV1 { revision: StateRevision, operation_ordinal: u32 }
```

A source has at most one attachment edge. A target can have multiple ordinary Aura edges. The timestamp is the accepted semantic attachment occurrence order: resulting state revision followed by the zero-based ordinal of this attachment operation in that `TransitionProduct`. It is never GameObjectId, allocation order, container order, wall time, or request-local candidate ID. Ordinal overflow rejects atomically. Same-transition attachment operations are ordered only by their semantically specified operation sequence; where locked M4 does not define a simultaneous attachment order (notably simultaneous Role attachments), that state is unsupported and fails closed pending a later rule design.

Every edge is between live battlefield incarnations. The source definition must be an attachment type; the edge target must satisfy the profile's typed attachment domain. A source leaving removes its outgoing edge. A target leaving invalidates incoming edges before a decision boundary; the applicable Aura illegal-attachment SBA is processed in the same fixed point. Reattachment replaces the source's prior edge with a new timestamp. Ordinary Auras can coexist. Linked-exile relations and continuous-effect outputs are separate deferred families and are never stored in this map.

For Roles, uniqueness is evaluated by current controller of the Role source and current target. Where one player controls multiple Role Auras attached to the same permanent, the newest timestamp survives and older Roles are put into their owners' graveyards under CR 303.7a; the edge/state actions are explicit in the same validated transition. Controller is read from the source object, not copied into the edge. The Role newest-timestamp rule is characterized as state semantics, but Role creation/continuous effects are not M4.2 support. Multiple simultaneous Role attachments are rejected until ordering is defined.

Trusted operation `AttachmentChanged { source, from_target, to_target, timestamp }`; observed event uses opaque source/target IDs only. Current attachment relation is public. Its projection exposes safe current relation and never trusted IDs. State digest/checkpoint/replay bind edge and timestamp.

## 10. FaceState

`FaceStateV1` is a closed BTreeMap from each live GameObjectId to its current CardDefinition-local `FaceKey`. It has exactly one entry per live object and no extra keys. Each FaceKey must belong to the object's definition in the environment's verified immutable content catalog. It is a FaceKey, never `transformed: bool`.

For the included definitions, nonbattlefield and ordinary battlefield entry defaults to the front FaceKey. Explicit transformed battlefield entry chooses the back FaceKey during construction of the new battlefield incarnation. A face change through the transform action while the object remains on the battlefield keeps the same GameObjectId and changes only FaceKey (CR 701.28/712). A zone transition creates a new GameObjectId and determines the new incarnation's face as part of its entry snapshot; prior incarnation face is available only through the typed last-known event snapshot. A transforming DFC that changes zone outside battlefield/stack returns front-face-up as required by CR 712.8a. Face-down cards, modal-face casting, face selection while casting, copy effects, and other uncharacterized face operations fail closed; state presence is not support for those mechanics.

Ojer Axonil's Oracle text says its death trigger returns it “to the battlefield tapped and transformed.” The transition is exactly old battlefield incarnation → graveyard incarnation → a new battlefield incarnation with `tapped=true` and the back FaceKey already present. The entry snapshot/event includes the back face and tapped state. There is no subsequent in-place transform event for that return. A separate transform of a permanent already on the battlefield changes FaceKey on that same incarnation. RED proves both.

Trusted operations distinguish `ObjectFaceChanged` for in-place transformation from `ZoneTransition`/`ObjectEntered` whose new-object snapshot carries face and tapped values. The observed V3 `ObjectMoved` event may carry the authorized visible face orientation on entry; a separate `ObjectFaceChanged` observed event represents later in-place transformation. No raw FaceKey or definition ID is exposed. The bounded M4 payload uses `front | back`; any content needing additional distinguishable face forms requires a new named codec or successor. Full digest/checkpoint/replay bind exact internal FaceKey.

## 11. Ability Authority (resolved)

Choose **Model A, a minimal persistent live-source authority registry**. Model B by itself cannot satisfy the normative meaning of `AbilityInstanceId` or the current `OpaqueAbilityId ↔ AbilityInstanceId` mappings: a request/candidate would have no persistent authoritative referent to validate. Replacing those mappings with a derived tuple would change the identity contract and create a separate identity redesign. Model C alone also fails because it supplies no exact authority for the current opaque ability identity before activation.

The new closed state is:

```text
AbilityAuthorityStateV1 {
  live: BTreeMap<AbilityInstanceId, AbilityAuthorityRecordV1>
}
AbilityAuthorityRecordV1 {
  source: GameObjectId,
  ability_key: AbilityKey
}
```

The semantic key is `(source GameObjectId, AbilityKey)`. The source's existing CardDefinitionId, current FaceKey, verified content catalog selected by ExecutionIdentity, and the definition's ability identities form the authoritative join; none is redundantly stored in the registry. There is at most one live AbilityInstanceId per semantic key. Controller is not copied: it is resolved from the current source GameObject at candidate validation.

The registry covers an ability identity on a current GameObject incarnation when that definition's closed semantic profile says the ability exists in that object's current zone and face. It is not battlefield-only. The profile owns zone-existence semantics; candidate generation separately checks whether activation is legal now. This distinction is required for locked M4 content such as Evershrike's Gift, whose activated ability can exist in a graveyard, without making that card executable in M4.2. Unknown profile/zone semantics fail closed. Zone change creates a new GameObject incarnation and retires the old authority row; it does not transfer an AbilityInstanceId across incarnations.

Registry IDs are allocated through existing `next_ability_id` in ascending trusted semantic key order `(source GameObjectId, AbilityKey)` when a transition creates or changes currently existing ability identities. This internal allocator order is deterministic but never participates in player-facing candidate ordering or opaque-ID allocation.

Cross-state validation proves for every row: source is a current GameObject incarnation in a zone/face where the admitted profile declares that ability identity; the object's CardDefinitionId resolves in the content manifest bound through ExecutionIdentity; FaceState selects the face containing the AbilityKey; instance ID is below `next_ability_id`; and no two rows share a semantic key. Every `OpaqueAbilityId → AbilityInstanceId` mapping resolves to a live row; inverse mappings remain bijective; candidate actor equals the controller resolved for the source under the current rules; and the ability is currently activatable under rules and costs. Invalid, stale, departed, wrong-face, wrong-definition, wrong-content-catalog, unknown-key, or wrong-controller candidates reject before transition workspace creation.

When a source incarnation leaves the zone/face where an ability identity exists or changes face, all no-longer-existing rows are removed and active perspective mappings are retired using the existing identity lifecycle; retired opaque IDs are never reused. Newly existing face/zone abilities receive fresh AbilityInstanceIds. For each perspective, newly visible ability identities are ordered before assigning any new OpaqueAbilityId by the independent public key `(source OpaqueObjectId, profile-defined visible ability ordinal)`. The ordinal is a zero-based position in the closed profile's player-visible ability list for that source face; it is not AbilityKey, AbilityInstanceId, allocation order, or hidden metadata. The basic-land@1.0.0 profile has one visible ability at ordinal 0. Only after sorting by this key are fresh perspective-local OpaqueAbilityIds assigned monotonically; CandidateOrderingV2 then sorts ActivateAbility candidates by those assigned opaque IDs. This removes the former ordering/allocation cycle. Global AbilityInstanceId renaming, hidden-zone allocator differences, and internal insertion order must not change OpaqueAbilityIds or candidate bytes when the authorized public state and prior perspective identity state are equal. Checkpoint/restore/fork/replay preserve registry and perspective mappings exactly. A mana ability resolves immediately and is not assigned a fake StackObjectId. If a later non-mana activated ability puts a payload on the stack and its source leaves, its future stack payload must capture the required semantics; that deferred stack contract is not solved by this live-source registry and must not retain a stale registry lookup as authority.

Thus `PERSISTENT_ABILITY_STATE_REQUIRED = YES`; no PerspectiveIdentity schema successor is needed because its existing mapping still names AbilityInstanceId, while the new registry closes that ID's referent. Its rows contain only `(source GameObjectId, AbilityKey)` under the ExecutionIdentity-selected catalog and are included in FullStateDigestV6, checkpoint V7 and Replay V7. ContentContractId is not embedded in those identities: the state/checkpoint binds it transitively through SemanticContractId in ExecutionIdentity.

## 12. PlayLand decision

V3 candidate vocabulary adds only:

```text
CandidateIntentV3::PlayLand { object: OpaqueObjectId }
EngineCandidateBindingV3::PlayLand { object: GameObjectId }
```

`CandidateOrderingV2` preserves the prior variant rank order except it inserts `play_land` after `pass_priority` and before `cast_spell`; within PlayLand, compare OpaqueObjectId numerically ascending. The remaining existing ranks retain relative order. Duplicate public keys still fail closed. CandidateIdV1 remains dense request-local. The trusted binding is never projected.

The complete legal candidate set contains every and only land in the actor's hand for which: actor is active player and priority holder; current step is a main phase; stack is empty; it is the actor's turn; the object's current content characteristics make it a land; the player has remaining normal land entitlement this turn; the episode is running; and all current rules restrictions permit the special action. The locked pool has one land entitlement and no additional-land effect. A land candidate is not CastSpell, SelectObject, Confirm, or a generic action.

Selecting it invokes the RulesKernel special-action path. It does not use the stack, pay mana, create a spell, or ask for a target. It increments `land_plays_used` atomically and moves the physical card from Hand to Battlefield as a new GameObject incarnation with same owner/controller, untapped, front face. Rejection (wrong actor, phase, priority, stack, object/zone/type, entitlement, stale/fabricated opaque mapping) changes no state, ID allocator, event, revision, digest, or RNG.

`DecisionResponseV2` remains sufficient: one candidate choice is still `SelectOne { CandidateIdV1 }` addressed by existing PlayerDecisionIdV1 and revision. There is no hidden fallback or automatic land choice. All legal lands and mana abilities are player-visible candidates in canonical order.

## 13. Authoritative events and StateDelta

Trusted `AuthoritativeRuleEventKindV2` and `SemanticDeltaOperationV2` remain separate from player-visible events. The V2 internal families add typed operations sufficient to audit:

```text
LandPlayCountChanged { player, from, to }
ObjectTapped { object, from, to }
ManaAdded { player, color, restriction, amount, source_object, source_ability }
ManaPoolEmptied { player, previous_pool }
CounterChanged { object, kind, from, to, cause }
AttachmentChanged { source, from_target, to_target, timestamp }
ObjectFaceChanged { object, from_face, to_face }
ObjectEntered { old_object?, new_object, source_zone, destination_zone, tapped, face }
AbilityAuthorityAdded / AbilityAuthorityRemoved { instance, typed source tuple }
```

Only operations applicable to accepted before/after values are emitted; IDs and ordering are deterministic. Each product's operations compose to its exact replacement. `StateDeltaV2` carries before/after StateRevision, FullStateDigestV6, full `EngineStatePartsV2` replacement, and ordered semantic operations. Applying it verifies before revision/digest, validates replacement state, recomputes after digest and checks after revision/digest. StateDelta is never used to reconstruct a different transition than its authoritative product. The existing `TransitionResult` is the ephemeral RulesKernel transition product; its after-state, delta, event vector and next pending request use the successor types. It has no independent wire identity. The environment verifies state, events, delta, status and all projections before atomic commit.

Land play has a trusted `LandPlayed` event with old/new incarnation, actor and land-play count; it is not duplicated as a second player-visible “land played” event. Player-facing `ObjectMoved` Hand→Battlefield plus resulting public board/next Decision expresses the move. Mana production and clearing use explicit public `ManaPoolChanged` because the pool is not inferable from tap alone. No event is invented for unchanged state.

## 14. ObservedEventEnvelopeV3 and PlayerStepV3

The new closed public event variants are:

```text
ManaPoolChanged { player, pool_after, cause: produced | emptied }
CountersChanged { object: OpaqueObjectId, kind, from, to }
AttachmentChanged { source: OpaqueObjectId, old_target: Option<OpaqueObjectId>, new_target: Option<OpaqueObjectId> }
ObjectFaceChanged { object: OpaqueObjectId, face: front | back }
ObjectMoved {
  old_object: Option<OpaqueObjectId>,
  new_object: Option<OpaqueObjectId>,
  from: ZoneKind,
  to: ZoneKind,
  entering_face: Option<front | back>,
  tapped: Option<bool>
}
```

V3 has exactly one `object_moved` wire tag and the six listed payload fields are all required; optional values serialize as explicit JSON `null`, matching the V2 event's explicit-null wire discipline. `old_object` or `new_object` must be non-null. `entering_face` and `tapped` are non-null only when a visible new incarnation enters the battlefield and the applicable observation policy authorizes those values; otherwise they are null. This V3 payload supersedes the V2 `ObjectMoved` payload shape; V3 does not contain a parallel `ObjectMovedV3` variant. All other V2 event meanings carry forward unchanged with V3 envelope sequence/revision rules. For Ojer-like entry the one `object_moved` event carries the new visible object's face and tapped state, not a second transform event. Event audiences are evaluated from authoritative event policy and per-perspective opaque identity mappings. No `GameObjectId`, `PhysicalCardId`, `AbilityInstanceId`, raw `FaceKey`, `AbilityKey`, `CardDefinitionId`, content ID, capability metadata, source binding, or continuation internals are serialized.

`PlayerStepV3` embeds `PlayerInformationStateV2`, `ObservedEventEnvelopeV3[]`, optional `PlayerDecisionRequestV3`, status and existing submission result. Revision monotonicity, perspective-local sequence monotonicity, rejection-empty-events, endpoint actor binding and next-decision consistency preserve the V2 invariants. `DecisionResponseV2` is unchanged.

## 15. Magic observation payload and information safety

The new codec ID is `magic-basic-land-observation.v1`. It is a closed, versioned typed payload—not arbitrary JSON/CBOR. It extends the currently admitted public Magic products with:

- every player's public mana counts by six types and two restriction classes;
- public counters for public battlefield objects using OpaqueObjectId;
- public attachment relations using opaque source/target identities;
- current public face orientation for an authorized object;

The payload does not carry a decision domain or candidate list. These are not observation facts and must not be duplicated into a second player-facing authority. At a running decision boundary, `PlayerStepV3` composes `PlayerInformationStateV2` (whose `current_observation` carries this payload), the optional `PlayerDecisionRequestV3`, the V3 observed-event sequence, status, and submission result. The optional `PlayerStepV3.next_decision` is the sole player-facing authority for the current decision domain and complete legal candidate set. `CandidateIdV1` remains request-local and is not represented in the observation payload. `DecisionResponseV2` selects only from that request.

The payload never includes GameObjectId, PhysicalCardId, AbilityInstanceId, CardDefinitionId, FaceKey raw ordinal, AbilityKey, ContentContractId, capability closure, registry row, CandidateId, CandidateIntent, trusted candidate binding, continuation, debug state, root seed, or RNG internals. Object definition/name is projected only through an already authorized `PlayerKnownObjectV1` knowledge fact; it is not added to reveal hidden card identity. Candidate intent/object references are projected only through the existing opaque V3 request contract, never copied into the observation payload.

The named payload is a strict JSON object with `additionalProperties: false`, exact schema field `schema_version: "magic-basic-land-observation.v1"`, and required fields. Its added fields have these closed shapes: `mana_pools` is an array sorted by PlayerId with one record per player (`player`, six-element `unrestricted`, six-element `creature_spell_only`, array order W/U/B/R/G/C); `counters` is sorted by OpaqueObjectId, then counter-kind tag; `attachments` is sorted by source OpaqueObjectId and carries one target OpaqueObjectId; `faces` is sorted by OpaqueObjectId and uses only `front | back`. Existing M3 observation fields retain their exact V1 field meaning. All visible numeric identity fields use existing canonical decimal-string JSON forms; counts use bounded unsigned integer wire forms as fixed by the successor schema. A public object omitted from the authorized projection is omitted from these lists, never represented with a trusted-ID placeholder. Golden JSON bytes use the repository canonical JSON key order; semantic arrays use the explicit orders above.

`ObservationEnvelopeV1` remains unchanged: codec ID and canonical payload bytes are covered by `ObservationDigest`. `InformationStateDigestV2` remains unchanged in schema and domain but its normal input includes the new envelope bytes. A new payload codec does not authorize a new information digest definition.

RED proves paired states differing only in unauthorized hidden facts have identical observation payload/envelope canonical bytes and observation digests, information-state canonical bytes/digests, authorized event sequence, PlayerStep and public errors wherever their authorized semantics are equal. Decision evidence is checked separately: where legal-decision semantics are equal, the `PlayerStepV3.next_decision` domains, candidate intents, ordering, and request-local IDs are equal; they are not compared as observation-payload fields. Trusted-ID renaming that preserves public relationships leaves player-facing observation/event/decision semantic bytes invariant.

## 16. FullStateDigestV6 canonical input

V6 uses SHA-256, existing `mtgml.digest-envelope.v1`, `mtgml.canonical-cbor.v1`, semantic domain `mtgml.full-state-digest.v6`, input schema `full-state-digest-input.v6`. Existing envelope framing remains exact. No runtime Serde output is hashed.

The top-level value is a fixed 14-element array. Elements 0–12 preserve the V5 semantic order and exact nested encodings where unchanged; element 13 is the new closed record:

```text
[
  "full-state-digest-input.v6", "mtgml.full-state-digest.v6",
  revision, core_v1, zones_v1, allocators_v3, execution_v3, random_v1,
  knowledge_v2, perspective_identities_v2, combat, foundation_sources,
  format_v1,
  card_rules_authoritative_state_v1
]
```

`execution_v3` retains the existing five-element execution shape and the V5
encodings of continuation/effect/trigger/delayed-effect entries. Its pending
decision retains the existing fixed request layout but adds one closed
candidate variant. A V3 request is:

```text
[decision_id, player_decision_id, state_revision, actor, visibility_tag,
 decision_domain, candidates[], continuation_id_or_null]
```

Each candidate is `[candidate_id_u32, visible_intent, trusted_binding]`.
All V2 intent/binding tags keep their exact encodings; V3 adds only:

```text
visible_intent: ["play_land", opaque_object_id]
trusted_binding: ["play_land", game_object_id]
```

V3 candidate rank tags are `pass_priority=0`, `play_land=1`,
`cast_spell=2`, `activate_ability=3`, `select_object=4`,
`select_player=5`, `select_mode=6`, `choose_boolean=7`,
`declare_number=8`, `confirm=9`. Within PlayLand, OpaqueObjectId is compared
numerically. The decision domain, visibility, candidate density and
continuation field retain their prior layout. `ChooseNumber` still has no
candidates. No V5 bytes are reinterpreted.

The final element is exactly:

```text
[
  "card-rules-authoritative-state.v1",
  mana_state_v1,
  turn_history_v1,
  counter_state_v1,
  attachment_state_v1,
  face_state_v1,
  ability_authority_state_v1
]
```

Canonical nested arrays:

- Mana: `[player_id, unrestricted_counts[W,U,B,R,G,C], creature_spell_only_counts[W,U,B,R,G,C]]` in ascending PlayerId, exactly one per player; color indices are W=0, U=1, B=2, R=3, G=4, C=5; counts are CBOR unsigned u32 values, zero explicit.
- Turn history: `[turn_number, [[player_id, land_plays_used, spells_cast_total, noncreature_spells_cast, lost_life_bool, red_noncombat_damage_dealt, permanent_card_to_graveyard_bool], ...], target_occurrences[[game_object_id, targeting_player], ...], once_ability_used[[game_object_id, ability_key], ...]]`. Player and tuple arrays are sorted by declared numeric key; sets reject duplicates.
- Counters: `[[game_object_id, [[counter_kind_tag, positive_u32], ...]], ...]`; outer and inner arrays follow numeric object and fixed tags 0, 1, 2.
- Attachments: `[[source_object_id, target_object_id, timestamp_revision, operation_ordinal], ...]` sorted by source ID. Timestamp fields are unsigned and operation ordinal is u32.
- Face: `[[game_object_id, face_key_u32], ...]` sorted by object ID.
- Ability authority: `[[ability_instance_id, source_object_id, ability_key_u32], ...]` sorted by AbilityInstanceId; no duplicate `(source_object_id, ability_key)` tuple. Content contract, definition, and face are validated joins through ExecutionIdentity-selected immutable content, the source GameObject's CardDefinitionId, and FaceState; they are not repeated in the state digest.

All arrays are definite length; integers use shortest canonical CBOR encodings and exact declared widths/ranges; booleans use CBOR booleans; IDs use their existing unsigned numeric semantics. Empty sets encode as empty arrays. Maps, floats, tags, indefinite-length values, bignums, shared references, unknown tags/variants, duplicate keys, noncanonical ordering and trailing values reject. Decoder re-encodes and requires byte equality. The execution identity's SemanticContractId transitively binds the ContentContractId according to ADR 0055; the child ID is not duplicated as a state field. This V6 input never changes V5 bytes or meanings.

## 17. Checkpoint, restore, fork and replay

`EnvironmentCheckpointV7` carries complete current EngineState, `FullStateDigestV6`, status, environment limit counters, codec identity `in-memory-reference / 7`, full `ExecutionIdentityV1`, and `CheckpointDigestV7`. The content child is not duplicated into checkpoint state: its identity is bound as `ExecutionIdentityV1.semantic_contract_id`, and restore admission requires the supplied verified immutable catalog's `ContentContractIdV1` to equal the non-null child ID in the verified SemanticContractManifest. `environment-checkpoint-digest-input.v7` / `mtgml.checkpoint-digest.v7` use the existing framed canonical digest envelope with this exact seven-element payload:

```text
["environment-checkpoint-digest-input.v7", "mtgml.checkpoint-digest.v7",
 full_state_digest_reference_v1_for_v6, episode_status,
 environment_limit_counters, ["in-memory-reference", "7"],
 [[execution_program_variant, null], semantic_contract_id_bytes32]]
```

The full-state reference identifies `mtgml.full-state-digest.v6` /
`full-state-digest-input.v6`. Restore validates the whole state, every new
family and both digests before exposing executable state. Fork clones the
exact state and identities.

Replay V7 is the successor of every currently V6-typed family. `ReplaySchemaVersionsV7` has exactly the current eight string fields, with these values for this profile: observation `observation-envelope.v1`, payload `magic-basic-land-observation.v1`, information state `information-state-envelope.v2`, decision `player-decision-request.v3`, response `decision-response.v2`, observed event `observed-event-envelope.v3`, PlayerStep `player-step.v3`, replay step `replay-step.v7`. `InitialEnvironmentIdentityV7` has exactly state revision, FullStateDigestV6, status, environment counters, checkpoint codec identity, CheckpointDigestV7, and ExecutionIdentityV1. `ReplayManifestV7` retains the exact V6 provenance/control fields and carries `SemanticContractMaterialV7` with:

```text
SemanticContractMaterialV7 {
  semantic_contract_id: SemanticContractIdV1,
  manifest: SemanticContractManifestV1,
  rules_manifest: RulesContractManifestV1,
  content_contract: Option<ContentContractMaterialV1>
}
ContentContractMaterialV1 {
  content_contract_id: ContentContractIdV1,
  manifest: ContentContractManifestV1
}
```

The internal typed form above is **not** serialized with Serde and does not
imply a JSON representation for `ContentContractManifestV1`. Replay V7's JSON
wire representation for the `content_contract` child is exactly:

```json
{
  "content_contract_id": "<64 lowercase hexadecimal characters>",
  "manifest_canonical_cbor_base64": "<canonical padded standard Base64>"
}
```

The Base64 value encodes the complete canonical-CBOR payload of
`ContentContractManifestV1` as specified by `content-contract-manifest.v1`;
it does not encode the outer digest-envelope framing. It uses the standard
Base64 alphabet (`A–Z`, `a–z`, `0–9`, `+`, `/`) with required `=` padding,
no whitespace, and canonical zero pad bits. The wire decoder bounds the
encoded string before allocation to the Base64 size corresponding to the
existing 64 MiB canonical-CBOR payload limit, decodes strictly, invokes the
closed `decode_content_manifest_v1` contract, re-encodes the typed manifest,
and requires byte-for-byte equality with the decoded payload. It recomputes
`ContentContractIdV1` from those canonical bytes and compares it with the
child ID and `SemanticContractManifestV1.content_contract_id`. The child
object has exactly the two fields shown; unknown or duplicate fields,
uppercase/non-hex ID characters, malformed or noncanonical Base64, invalid
CBOR, noncanonical manifest bytes, digest mismatch, and missing/extra child
all reject before replay execution or checkpoint restore admission.

When the semantic manifest's content ID is null, the wire value is exactly
`"content_contract": null`; when it is non-null, exactly one object of the
shape above is required. The V7 JSON Schema must express the closed object,
lowercase ID pattern `^[0-9a-f]{64}$`, Base64 alphabet/padding shape
`^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$`,
minimum encoded length 4, and maximum encoded length 89,478,488 characters
(`4 * ceil(67,108,864 / 3)`, for a 64 MiB decoded payload);
the typed decoder remains responsible for strict Base64 canonicality, CBOR
canonicality, child/parent digest equality, and content semantic validation.
This transport form introduces no generic JSON/CBOR field, Serde DTO for the
content manifest, or alternate content identity encoding.

For this executable Magic profile, `manifest.content_contract_id` is non-null and exactly equals `content_contract.content_contract_id`; absent ID requires absent child, and present ID requires exactly one child. The detached V7 verifier recomputes the ContentContractIdV1 from the child's exact canonical manifest bytes, validates the closed definition/profile structure, recomputes SemanticContractIdV1 from its manifest, recomputes RulesContractIdV1 from the rules manifest, checks all child IDs and all three ExecutionIdentityV1 occurrences, and checks rules-snapshot consistency under ADR 0055. Runtime restore additionally requires its verified catalog bytes/ID to equal this child material before making state executable. Provenance remains a separate audit artifact because it is excluded from ContentContractId; V7 verifies semantic content identity and does not claim provenance or rules correctness. When the semantic manifest's content ID is null, the child is absent as required by ADR 0055; V6 retains its historical non-null-content rejection.

`ReplayStepV7` retains the exact V6 fields: step index, actor, before CheckpointDigestV7, before revision, one DecisionResponseV2, accepted flag, after revision, after FullStateDigestV6, after status, after environment counters, and after CheckpointDigestV7. `AuthoritativeReplayV7` contains schema version, manifest, ordered steps, and final InitialEnvironmentIdentityV7. `ReplayRecorderV7` records these same typed identities and inputs. Initial and step records bind FullStateDigestV6 and CheckpointDigestV7. Existing Replay V6 control/provenance fields remain unchanged except successor schema identities and typed references. One explicit DecisionResponseV2 remains the replay input per step; no event-as-input or forced choice is added. The Recorder records only explicit response/control inputs and resulting identities, not projected events as a second authority.

For identical V7 initial identity, content/rules/execution identity, RNG contract/seed, initial state and external responses, direct execution, checkpoint restore, fork and full authoritative replay must produce identical semantic state, event/delta sequence, next Decision and final digests. Checkpoint/fork/replay preserve ManaState, history, counters, edges/timestamps, faces, ability registry and opaque ability mappings exactly. Incidental allocators, HashMap iteration, pointer/debug formatting and request-local CandidateId cannot affect semantics.

## 18. RED and conformance obligations

Implementation begins with failing RED evidence. Every rejected request verifies complete byte/semantic nonmutation: EngineState, all digests, StateRevision, global/perspective allocators, RNG, event/delta output, observations, visible sequences, next Decision, status and checkpoint identity.

### Definition/profile RED

- valid Mountain and Plains definitions have exact pinned Oracle provenance, correct basic subtype/type line and matching closed basic-land profile parameter;
- wrong profile ID, malformed body, unknown subtype, profile/type-line mismatch, wrong face/key identity and unknown future variant reject;
- profile ID or any executable body change changes ContentContractIdV1;
- semantic requirements are derived from profile/subtype and cannot be author-suppressed;
- names changed while type/profile semantics remain equal do not choose a different rules path.

### Mana and land RED

- create W, R and C units in typed pools; unrestricted/restricted bucket identity survives;
- creature-spell-only mana is not eligible for a land action, ability, or noncreature cast; do not implement a spending choice;
- pools empty after each applicable step/phase boundary, not before its end; zero clear is no-op;
- bucket overflow rejects without mutation;
- first legal land candidate and exact hand→battlefield new incarnation;
- second same-turn play rejects even if first land left battlefield;
- turn reset restores entitlement;
- wrong phase, nonactive player, no priority, nonempty stack, nonland, stale/fabricated opaque card and exhausted entitlement reject;
- all legal land and mana-ability candidates present, with no heuristic selection;
- mana ability taps its source and produces exact subtype mana immediately without a stack object;
- exact TransitionProduct proves before state, typed selected action, ordered authoritative events, StateDeltaV2, after state/revision and all perspective products.

### Turn-history RED

- total/noncreature cast counts, noncreature classification, and copies-not-cast behavior;
- life lost remains true after later gain; damage-caused life loss records actual event;
- permanent card to graveyard fact persists after it leaves graveyard; token does not count;
- noncombat red damage adds actual damage after replacement/prevention; combat excluded;
- Emberheart: opponent targets then controller targets; controller targets twice; retarget; spell-copy target; target control changes before a later target; same turn and reset boundary;
- stale incarnation target/use records pruned on zone transition; player histories retained;
- once-per-turn ability key cannot be constructed without validated AbilityAuthority record.

### Counter/attachment/face RED

- each allowed counter kind add/remove, zero removal, overflow, invalid zero/nonbattlefield/unknown kind;
- zone change clears, transform retains, +1/+1 and -1/-1 annihilate to smaller count, lore survives checkpoint/replay;
- attach/re-attach timestamp monotonicity; source/target departure removes/rejects edge before next decision; multiple ordinary Aura edges permitted; Role uniqueness retains newest timestamp and moves older Role to owner graveyard; simultaneous same-transition Role order rejects;
- front-face initial object; in-place transform preserves GameObjectId; zone transition creates new incarnation; Ojer return is new tapped back-face incarnation and emits no synthetic subsequent transform event.

### Ability-authority RED

- valid land intrinsic candidate resolves opaque ID → AbilityInstanceId → source/AbilityKey and validates definition/face/profile/content through the admitted ExecutionIdentity-selected catalog; it produces only its declared ability;
- stale incarnation, departed source, wrong controller, wrong face, wrong key, wrong definition/content, unknown profile and stale/fabricated opaque mapping reject atomically;
- a locked-pool graveyard ability identity can be represented without being executable in M4.2; exact candidate eligibility is profile/zone/rules-derived;
- global AbilityInstanceId renaming, hidden-zone allocator differences, and insertion/hash-map order preserve the same authorized public ability set and produce identical OpaqueAbilityIds/candidate bytes;
- opaque allocation is ordered by `(source OpaqueObjectId, profile-defined visible ability ordinal)` before allocating IDs; no first/random fallback;
- mana ability resolution is immediate and creates no StackObjectId;
- source departure retires visible identity and removes live authority row; return creates new incarnation/authority ID and never revives old opaque ID;
- checkpoint/restore/fork/replay preserve authority and perspective mappings and next legal request.

### Privacy, identity and persistence RED

- no trusted-ID/content/capability/continuation leakage in request, event, observation, information state, PlayerStep, or error;
- public pool/counter/attachment/face equality with authorized state and paired hidden-state noninterference;
- trusted-ID renaming with relationship preservation leaves player-visible bytes unchanged;
- V5, Checkpoint V6, Replay V6, Decision V2, ObservedEvent V2, PlayerStep V2 and unprofiled ContentContract V1 historical vectors remain exact;
- successor canonical known-answer vectors, mutation-of-every-new-field digest tests, map-insertion order invariance, noncanonical/duplicate/unknown/bounds negatives, full restore/fork/replay equivalence;
- full replay is executed and compared, not just detached schema validation.

## 19. Acceptance gates

The Spec is ready for independent Spec review when its independent semantic and compatibility decisions are accepted. The implementation is not accepted until all gates in current `docs/contracts/ACCEPTANCE_GATES.md` and current CI pass on exact reviewed heads. Separate status values remain:

```text
IMPLEMENTATION_PASS
HOSTED_CI_PASS
CODE_REVIEW_PASS
FINAL_ACCEPTANCE_PASS
```

Required implementation profiles include `just check-fast`, `just check`, `just check-all`, separate `just release-candidate`, `just archive-check`, `git diff --check`, exact required hosted CI (`PR Fast`, `PR Integration`, `manafold-pr-gate`, applicable platform smoke/CodeQL), independent final-diff review, and post-merge exact-master verification. These are future implementation-plan gates; none was run by authoring this Spec.

## 20. Explicit support boundary and stop conditions

State substrate is not card support:

```text
CounterState exists != Optimistic Scavenger supported
AttachmentState exists != Sheltered by Ghosts supported
FaceState exists != Ojer Axonil supported
Ability registry exists != arbitrary activated abilities supported
Mana restrictions exist != general mana payment supported
```

Stop and amend/re-review this Spec if implementation discovers: M4.1 ProfiledV1 cannot carry this closed body; a V5/V6 input or identity is being reinterpreted; current content catalog cannot validate ability authority; GameObject incarnation cannot own FaceKey/counter/attachment coherently; control/target history contradicts current Oracle/rulings; attachment timestamp/event order is ambiguous for locked scope; any state family cannot be canonically encoded; a new continuation/general stack is required; or PlayerObservation must expose trusted state. Do not solve such a contradiction in the Implementation Plan.
