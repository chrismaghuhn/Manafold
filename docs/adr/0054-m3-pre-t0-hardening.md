# ADR 0054 — M3 Pre-T0 Hardening: Capability Lifecycle, Dependency Closure, State Identity Cut, and Reauthorization

- **Status:** candidate; pending hardening-PR merge and exact-master reauthorization
- **Date:** 2026-09-16
- **Owners:** architecture maintainers; state maintainers; rules maintainers; conformance maintainers
- **Historical basis:** accepted ADR 0053 and `M3_INITIAL_SEMANTIC_FOUNDATION_V1.md`
- **Current companion:** [`M3_INITIAL_SEMANTIC_FOUNDATION_V2.md`](../rules/M3_INITIAL_SEMANTIC_FOUNDATION_V2.md)
- **Implementation evidence:** `NOT_RUN`; this ADR changes no production semantic behavior

This ADR records the pre-T0 hardening decision for the accepted M3 Entry
scope. It is a candidate on the hardening branch until the change is merged
and a separate exact-`master` review reauthorizes M3. ADR 0053 and Foundation
V1 remain immutable historical evidence; they are not rewritten to conceal the
earlier acceptance or authorization record.

## 1. Context and current status authority

The verified remote `master` at the beginning of this audit was:

```text
MASTER_AT_START = 0f13b43680ea7d0b043c5baee59eb2ed3c364ecc
```

PRs #181, #182, and #183 are merged. Issue #178 is open and contains a
historical Stage-B authorization at the same master SHA. Issue #163 is closed
as superseded historical planning provenance. The repository-owned status
surfaces still described M3 as unauthorized, while Issue #178's historical
comment described it as authorized. This is a status-authority contradiction,
not a reason to delete history.

The current candidate state is therefore:

```text
M3_PRE_T0_HARDENING = IN_PROGRESS

PRIOR_AUTHORIZATION_HEAD =
0f13b43680ea7d0b043c5baee59eb2ed3c364ecc

PRIOR_AUTHORIZATION_RECORD = HISTORICAL
CURRENT_PLAN_REAUTHORIZATION_REQUIRED = YES

M3_AUTHORIZED = NO
M3_STARTED = NO
AUTHORIZATION_HEAD = NOT_SET

IMPLEMENTATION_START = BLOCKED
BLOCK_REASON = PRE_T0_PLAN_HARDENING
```

The earlier authorization happened on the then-reviewed master and remains a
historical fact. This hardening review supersedes its present authorization
effect because the plan's capability closure and persisted-state prerequisites
are being reviewed again before implementation starts.

## 2. Re-audit findings

| Finding | Decision | Reason |
| --- | --- | --- |
| Issue #178 and repository status disagree | Confirmed; reset current authorization | README and ROADMAP are current status entry points. The historical Issue #178 authorization is preserved, but the candidate plan requires a new exact-master authorization. |
| Initial Foundation expands authoritative state meaning | Confirmed | Phase/step, explicit no-priority, pass progression, combat participation, characteristics, marked damage, and control-age facts are not represented by the current V3 state. |
| Foundation says eleven capabilities are specified while registry is empty | Confirmed; choose lifecycle Model B | V1 independently records key, version, scope, authority, dependency, ownership, information risk, and exclusions for all eleven nodes. Registering `specified` does not claim implementation, coverage, certification, cards, or decks. |
| Capability dependency DAG may contain state-producer edges | Confirmed for two edges | Cleanup and SBA consume authoritative facts; they do not semantically require the behavior of the capability that produced those facts. The edges are removed after the edge-by-edge audit in Section 6. |
| T0 needs a proof path for no-choice forced progress | Confirmed | Ordinary untap and ordinary cleanup do not create player Decisions. T0 must drive a rules-owned forced-progress path without inventing a response or second scheduler. |
| `rules/state-based-actions-combat` naming is unstable | Rejected as a rename trigger | The `-combat` suffix accurately bounds the selected SBA subset to the combat-oriented Initial Foundation. The key remains unchanged and the bounded meaning is made explicit. |

## 3. Capability lifecycle authority

The hardening candidate selects **Model B**. The eleven identities in
Foundation V2 are registered with:

```text
lifecycle = specified
implementation_paths = []
conformance_cases = []
benchmark_scenarios = []
IMPLEMENTED_CAPABILITY_COUNT = 0
COVERED_CAPABILITY_COUNT = 0
CERTIFIED_CAPABILITY_COUNT = 0
```

`specified` means that authority, scope, state requirements, decisions,
visibility, ordering, exclusions, ownership, and dependencies have been
reviewed. It does not mean `implemented`, `covered`, `certified`, card
supported, deck supported, or playable. The registry entries use the shared
Foundation V2 document as `spec_path` because each identity is independently
locatable in its identity table and the document avoids eleven duplicated
specification files. The registry validator permits this and still requires
authority references and rejects later lifecycle claims without evidence.

No registry entry is added for M3.P0 or M3.T0. Both are infrastructure and
proof work, not Magic capabilities.

## 4. State-requirement census

The following census covers the exact eleven-node Foundation closure. “Missing”
means missing from the current V3 authoritative state, not missing from every
existing event or transient workspace. Persistence/checkpoint/digest/replay
columns describe the required V4 plan; no V4 type is implemented by this ADR.

| Capability | Semantic fact | Current V3 location | Missing | Authoritative vs derived | Persistence required | Checkpoint required | Digest required | Replay consequence | Observation consequence | Future generalization risk | Proposed owner | Proposed representation |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `rules/turn-structure@0.1.0` | Turn number; active player; closed phase/step position; explicit no-priority untap/cleanup boundary; simultaneous untap eligibility and tapped-state clearing. | `EngineState.core.turn_number`, `active_player`; `zones_v1.objects[].tapped`; no phase/step or no-priority value. | YES: phase/step, explicit priority absence, and untap eligibility contract. | Turn position, active player, and eligible/tapped facts are authoritative; boundary events are derived audit products. | YES | YES | YES | One accepted player response includes all deterministic forced consequences through the next decision/outcome; no untap replay response. | Authorized phase, step, active player, and priority availability may be visible; trusted IDs and hidden state remain absent. | Extra/skipped turns, extra phases, phasing, and no-untap effects must fail closed rather than fit a dynamic registry. | `turn` / `mtgml-state` physical storage | Closed `TurnPosition` plus `PriorityState`; typed untap eligibility in the bounded creature/permanent fact. |
| `rules/basic-priority@0.1.0` | Priority presence/absence; holder; zero/one consecutive pass state; pass-only action-surface precondition; empty-stack boundary. | `EngineState.core.priority_player`; `execution_v2.pending_authoritative_decision`; empty V3 stack/effect arrays. | YES: representable `None` and explicit pass-progress state. | Priority state and validated action surface are authoritative; “pass-only” is a validated semantic predicate, not a fixture flag. | YES | YES | YES | Each explicit pass is one player decision and one replay step; two successive passes advance through rules-owned forced progress in that same transaction. | Priority availability/authorized holder can be projected; action-surface diagnostics and trusted bindings cannot. | Spells, abilities, stack objects, triggers, replacements, and multiplayer priority remain fail-closed. | `priority` semantic owner; `mtgml-state` stores state | `PriorityState::None` or `PriorityState::HeldBy { player, consecutive_passes }`, with `consecutive_passes` restricted to `0..=1`. |
| `rules/draw-card@0.1.0` | Draw-step context; active player; top of a nonempty ordered library; library-to-hand zone move; physical-card continuity; authorized knowledge update. | `zones_v1.ordered_zones[]`, `ZonePosition::Top`; `zones_v1.objects/locations`; `knowledge_v2` and `perspective_identities_v2`; no phase/step. | YES: draw-step context only; existing order/incarnation substrate is present. | Library order, object identity, and knowledge are authoritative; draw legality is rules-derived from turn position and profile. | YES | YES | YES | V4 replay binds the after identity and replays the authoritative zone/knowledge transition; no RNG is consumed by the draw. | Drawing player may learn the card through existing authorized knowledge; opponent-safe bytes remain independent of hidden card identity. | Empty-library, replacement, multiple-draw, and game-start cases remain unsupported. | `turn` orchestrator with `zones_identity` and information substrate | Existing ordered-zone vector with typed draw-step action and selected `ZoneTransition`. |
| `rules/cleanup-reset@0.1.0` | Ordinary cleanup boundary; no discard requirement in the selected case; marked damage is removed exactly at cleanup and retained before it; no priority. | No marked-damage field; cleanup position absent; `zones_v1.objects[]` has tapped/face-down only. | YES: marked damage and cleanup temporal context. | Marked damage is authoritative; cleanup eligibility is derived from typed turn position and bounded profile. | YES | YES | YES | Cleanup reset is deterministic forced work inside the response transaction or initialization stabilization; it is not a player replay action. | Authorized temporal state and public tapped/consequence changes only; marks need not be exposed unless authorized by a later observation contract. | Cleanup discard, duration expiry, and cleanup-trigger exceptions remain unsupported. | `turn` semantic owner; `mtgml-state` stores marks | Bounded `marked_damage: u64` in the typed foundation creature/permanent fact; no dependency on damage producer behavior. |
| `rules/combat-damage@0.1.0` | Combat step; attacking objects; defending player target; blocker assignment; positive power; simultaneous assignment/dealing; post-damage SBA boundary. | No combat context in `EngineState`; current zones only carry object/location/tapped/face-down. | YES: combat participation, assignments, simple characteristics, and marked damage. | Assignments and combat context are authoritative after explicit Decisions; positive-power and recipient results are derived within the rules transaction. | YES | YES | YES | One response transaction includes assignment, simultaneous damage, SBA fixed point, and stop at the next decision/outcome. | Public combat participation and authorized consequences may be observed; trusted object IDs and hidden bindings remain absent. | First/double strike, trample, prevention/replacement, multiple blockers, and assignment choices remain excluded. | `combat` orchestrator; `damage` and `state_based_actions` semantic owners | Typed bounded `CombatState` plus typed damage result/delta; no free-form labels. |
| `rules/combat-phase@0.1.0` | Beginning/end combat, declaration, damage, and end-of-combat step order; fixed defending opponent; participation removal. | No phase/step or combat state. | YES | Phase/step and fixed opponent are authoritative; skip rules are derived from the bounded combat state. | YES | YES | YES | Forced phase/step transitions remain in the same replay step until a real declaration decision or outcome. | Current authorized temporal position and combat visibility are projected without privileged identities. | Additional combat phases and second damage steps fail closed. | `combat` with `turn-priority-progression` | Closed combat-step enum nested in `TurnPosition`; optional bounded `CombatState`. |
| `rules/declare-attackers@0.1.0` | Eligible creatures; untapped state; continuous control since turn start; attacking subset; fixed defending player; explicit empty subset. | `zones_v1.objects[].controller/tapped`; no `controlled_since_turn`, creature classification, or attackers set. | YES: control age, creature qualification, and attacking set. | Control age and eligibility facts are authoritative; the legal subset is derived and exposed through a Decision. | YES | YES | YES | Accepted attacker declaration is one replay step; no implicit empty attack or default subset. | Actor receives canonical opaque/public candidates only; opponent receives only authorized public combat information. | Haste, defender, restrictions, requirements, and non-player attack targets fail closed. | `combat` / `combat-declaration` | `CombatState.attackers: ordered Vec<GameObjectId>` plus typed `FoundationCreatureFact`. |
| `rules/declare-blockers@0.1.0` | Attacking set; defending player; zero/one relevant blocker; complete blocker-to-attacker assignment; no priority during declaration. | No attacking or blocker assignment state. | YES | Explicit assignment is authoritative after Decision; eligible blocker set is derived from the closed profile. | YES | YES | YES | One blocker Decision, if needed, is one replay step and commits the complete assignment atomically. | Defending actor sees only authorized canonical candidates; more than one eligible blocker fails before Decision creation. | Multiple blockers, evasion, restrictions, requirements, and assignment order fail closed. | `combat` / `combat-declaration` | `CombatState.blockers` as a canonical typed mapping from attacker to zero/one blocker. |
| `rules/damage-and-life@0.1.0` | Player life; player loss flag; simple positive power; marked creature damage; source/affected roles; simultaneous result. | `core_v1.players[].life/has_lost`; no power/toughness or marked damage. | YES: simple creature facts and marks; life/loss already exists. | Life/loss and marks are authoritative; damage assignment/result is derived from combat state and simple characteristics. | YES | YES | YES | Replay validates the V4 after identity and exact accepted transition; no separate replay action for a damage consequence. | Authorized life/loss and public consequence may be observed; source/affected trusted IDs remain restricted. | Layers, copy effects, negative/complex characteristics, lifelink, infect/wither/toxic, and replacement fail closed. | `damage` / `mtgml-state` storage | `FoundationCreatureFact { power, toughness, marked_damage, ... }` plus existing `PlayerState`. |
| `rules/state-based-actions-combat@0.1.0` | Life loss; zero toughness; lethal marked damage; simultaneous application; fixed point; selected zone moves and terminal status. | `core_v1.players[].life/has_lost`; no creature characteristics/marks; zone incarnation exists in `zones_v1` and `ZoneTransition` event product. | YES: predicates' creature inputs and explicit fixed-point execution boundary; life/loss and zone substrate exist. | Predicates and simultaneous action set are derived by the SBA interpreter; life/marks/identity are authoritative state facts. | YES for resulting state; NO for a transient loop cursor | YES | YES | Fixed-point consequences remain inside the triggering replay step; terminal status ends the step without a next Decision. | Public loss/life/zone consequences may be projected; hidden identity and diagnostics remain absent. | Indestructible, regeneration, tokens, legend/world/Aura/Equipment/Role/counter/battle actions, and triggers fail closed. | `state_based_actions` with `zone-transition-pipeline` | Typed SBA action set/fixed point over `FoundationCreatureFact`; selected moves use `ZoneTransition`. |
| `rules/zone-incarnation@0.1.0` | Ordered library; physical-card continuity; new `GameObjectId`; old-incarnation snapshot/LKI input; selected draw/death moves. | `zones_v1.objects/locations/ordered_zones`; `allocators_v3.next_object_id`; `ZoneTransition.last_known/new_snapshot` exists in event products. | NO for selected live identity/order substrate; LKI is not persistent post-state. | Live objects, locations, order, and physical identity are authoritative; LKI is a transition-bound snapshot, not a cache. | YES for live state and physical continuity; NO for obsolete LKI after the transaction | YES | YES | V4 replay must reproduce the same typed transition and after identity; LKI remains part of event/delta/conformance evidence. | Existing perspective identity/knowledge lifecycle applies; public projection uses opaque IDs and authorized locations only. | Randomization, copy continuity, attachments, tokens, exile/command, and general LKI-trigger semantics remain excluded. | `zones_identity` / `zone-transition-pipeline` | Existing object/location vectors plus typed old/new `ObjectSnapshot`; no second identity owner. |

The census intentionally does not add generic layers, dynamic phase registries,
card-definition semantics, or hidden caches. A simple power/toughness value is
an authoritative foundation input only for the explicitly bounded simple
profile. Any characteristic-changing machinery must reject before execution;
the value must not become a permanent substitute for future layer evaluation.

## 5. Capability dependency and identity audit

The audit question for every edge was: **does A semantically require B's
behavior for A's declared scope to be correct?** Shared state, a producer /
consumer relationship, a Rust import, or a reusable helper is not sufficient.

| Edge | Requires B semantics? | Shared state only? | Shared primitive? | Runtime orchestration? | Verdict | Rationale |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| `rules/basic-priority -> rules/turn-structure` | YES | NO | NO | YES | RETAIN | Priority requires the canonical active player and phase/step context. |
| `rules/basic-priority -> rules/state-based-actions-combat` | YES | NO | NO | YES | RETAIN | The selected priority boundary must run the selected SBA fixed point before offering priority. |
| `rules/draw-card -> rules/turn-structure` | YES | NO | NO | YES | RETAIN | The draw is legal only at the selected draw-step boundary. |
| `rules/draw-card -> rules/zone-incarnation` | YES | NO | NO | YES | RETAIN | The declared draw includes the selected library-to-hand move and new incarnation. |
| `rules/cleanup-reset -> rules/turn-structure` | YES | NO | NO | YES | RETAIN | Cleanup reset is legal only at the selected cleanup boundary and inherits its no-priority classification. |
| `rules/cleanup-reset -> rules/damage-and-life` | NO | YES | YES | YES | REMOVE | Cleanup clears the marked-damage fact regardless of which capability produced it; requiring the producer would incorrectly turn shared state into a capability dependency. |
| `rules/combat-phase -> rules/turn-structure` | YES | NO | NO | YES | RETAIN | Combat is a phase in the fixed turn structure and uses its active-player ownership. |
| `rules/combat-phase -> rules/basic-priority` | YES | NO | NO | YES | RETAIN | Selected combat boundaries require the explicit pass protocol. |
| `rules/declare-attackers -> rules/combat-phase` | YES | NO | NO | YES | RETAIN | Attacker legality and commitment require the selected combat context. |
| `rules/declare-blockers -> rules/declare-attackers` | YES | NO | NO | YES | RETAIN | Blockers are assigned only to the attacking set produced by the preceding declaration. |
| `rules/combat-damage -> rules/declare-blockers` | YES | NO | NO | YES | RETAIN | Blocked/unblocked recipients require the completed bounded blocker assignment. |
| `rules/combat-damage -> rules/damage-and-life` | YES | NO | NO | YES | RETAIN | The combat-damage capability declares the selected player-life and marked-creature-damage results as part of its scope. |
| `rules/combat-damage -> rules/state-based-actions-combat` | YES | NO | NO | YES | RETAIN | Its declared pipeline is incomplete until the selected post-damage SBA fixed point runs. |
| `rules/state-based-actions-combat -> rules/damage-and-life` | NO | YES | YES | YES | REMOVE | SBA evaluates authoritative life/marked-damage facts and does not require the damage producer's execution behavior. |
| `rules/state-based-actions-combat -> rules/zone-incarnation` | YES | NO | NO | YES | RETAIN | Selected creature death/toughness moves require the new-incarnation and physical-continuity behavior. |

The recomputed graph is:

```text
DIRECT_ROOTS =
  rules/turn-structure@0.1.0
  rules/basic-priority@0.1.0
  rules/draw-card@0.1.0
  rules/cleanup-reset@0.1.0
  rules/combat-damage@0.1.0

RESOLVED_CAPABILITY_COUNT = 11
DEPENDENCY_EDGE_COUNT_BEFORE = 15
DEPENDENCY_EDGE_COUNT_AFTER = 13
DEPENDENCY_EDGES_REMOVED = 2
DEPENDENCY_EDGES_ADDED = 0
DEPENDENCY_CYCLES = 0

LEAVES =
  rules/turn-structure@0.1.0
  rules/damage-and-life@0.1.0
  rules/zone-incarnation@0.1.0
```

The current key `rules/state-based-actions-combat` is retained. Its suffix
means “the bounded SBA subset required by the combat-oriented Initial
Foundation,” not “all combat-independent state-based actions.” No key or
capability version is renamed by this audit.

## 6. Foundation version decision

Foundation V2 is required because the resolved dependency graph has changed.
Foundation V1 remains the immutable accepted record of the graph reviewed
under PR #182. V2 retains all eleven key/version identities, direct roots,
declared scopes, explicit exclusions, S1 selection, and sixteen interaction
obligations, while making the dependency correction and pre-T0 structural
prerequisites current.

```text
FOUNDATION_INPUT_VERSION = m3.initial-semantic-foundation.v1
FOUNDATION_OUTPUT_VERSION = m3.initial-semantic-foundation.v2
FOUNDATION_V2_REQUIRED = YES
CAPABILITY_KEY_RENAMES = NONE
```

No Foundation V1 text is edited in place and no historical acceptance is
reclassified as if it had not occurred.

## 7. Coordinated state and persistence identity cut

The first M3 semantic slice changes the meaning of authoritative state: legal
temporal positions, no-priority boundaries, pass progression, and forced
untap become observable transition inputs and affect events, deltas, replay,
checkpoint restoration, and player products. Retaining V3 would reinterpret a
V3 identity against a different state meaning. A coordinated cut is therefore
required.

```text
M3_STATE_IDENTITY_CUT_REQUIRED = YES
STATE_CUT_SCOPE = FULL_INITIAL_FOUNDATION
STATE_CUT_TASK_ORDER = BEFORE_T0
```

### 7.1 Option comparison

| Option | Correctness | Migration count | Review/abstraction risk | Decision |
| --- | --- | --- | --- | --- |
| A — S1-only cut | Correct for S1 if followed by more cuts, but immediately makes combat/damage state a second migration target. | Repeated | Smaller first review, but guaranteed near-term digest/checkpoint/replay churn and repeated fixture migration. | Rejected |
| B — coordinated Initial-Foundation cut | Correct for the exact eleven-node closure because the bounded facts are enumerated in Section 4; does not implement their semantics. | One structural cut before semantic slices. | Larger review, but no speculative fields outside the exact closure; future layers/copy machinery remains fail-closed and can introduce a later explicit version if required. | **Selected** |

Option B is semantic-neutral structural foundation work. It is not a Magic
capability and it does not advance any lifecycle state. The exact P0 review
must verify that every field is one of the censused facts and that no derived
cache or Card IR substitute is introduced.

### 7.2 Required next identities

| Current surface | Next identity | Decision | Rationale |
| --- | --- | --- | --- |
| `FullStateDigestV3` / `full-state-digest-input.v3` | `FullStateDigestV4` / `full-state-digest-input.v4` / `mtgml.full-state-digest.v4` | YES | Phase/step, priority state, combat facts, characteristics, marks, and any V4-owned state alter the canonical authoritative meaning. |
| `EnvironmentCheckpointV3` / `environment-checkpoint.v3` | `EnvironmentCheckpointV4` / `environment-checkpoint.v4` | YES | The checkpoint embeds the complete EngineState and must restore the V4 state atomically. |
| `CheckpointDigestV3` / `environment-checkpoint-digest-input.v3` | `CheckpointDigestV4` / `environment-checkpoint-digest-input.v4` / `mtgml.checkpoint-digest.v4` | YES | The checkpoint digest binds the V4 full-state identity and V4 checkpoint identity. |
| `ReplayManifestV3` / `replay-manifest.v3` | `ReplayManifestV4` / `replay-manifest.v4` | YES | The manifest must bind V4 initial identity, V4 checkpoint codec, V4 replay schema, and the M3 observation payload codec. |
| `ReplayStepV3` / `replay-step.v3` | `ReplayStepV4` / `replay-step.v4` | YES | Each step must bind V4 before/after checkpoint/state identities while retaining one-decision-per-step semantics. |
| `AuthoritativeReplayV3` / `authoritative-replay.v3` | `AuthoritativeReplayV4` / `authoritative-replay.v4` | YES | The replay file identity and validation chain must not reinterpret V3 steps against V4 state. |
| `InformationStateDigestV2` | unchanged in this plan | NO | Its semantic input already contains `ObservationEnvelopeV1`, whose payload codec is independently identified. A later retained-knowledge meaning change requires its own reviewed version. |
| `ObservationEnvelopeV1` / `ObservationDigestV1` | unchanged envelope/digest; new payload codec | NO | The envelope already carries an independently versioned `payload_codec`; its digest continues to bind canonical payload bytes. |

### 7.3 Conceptual V4 state shape

P0 must add the following typed, closed conceptual state without implementing
Magic semantics:

```text
CoreRulesStateV4 =
  players
  active_player
  turn_number
  position: TurnPosition
  priority: PriorityState

TurnPosition =
  Beginning(BeginningStep)
  PrecombatMain
  Combat(CombatStep)
  PostcombatMain
  Ending(EndingStep)

BeginningStep = Untap | Upkeep | Draw
CombatStep = BeginningOfCombat | DeclareAttackers | DeclareBlockers |
             CombatDamage | EndOfCombat
EndingStep = EndStep | Cleanup

PriorityState = None
              | HeldBy { player: PlayerId, consecutive_passes: 0..=1 }

CombatState =
  defending_player: PlayerId
  attackers: canonical GameObjectId[]
  blockers: canonical attacker -> zero_or_one GameObjectId mapping

FoundationCreatureFact =
  creature_qualification
  simple_power
  simple_toughness
  marked_damage
  controlled_since_turn
  untap_eligibility
```

The closed `TurnPosition` prevents invalid phase/step pairs without a free-form
string or dynamic registry. `PriorityState::None` is valid at untap and
ordinary cleanup; no sentinel `PlayerId(0)`, stale holder, or hidden
convention represents absence. The pass counter is bounded because the
selected protocol needs only zero or one consecutive pass before the second
pass advances the boundary. Future extra turns, skips, additional combat
steps, layers, copy effects, or format callbacks are not represented here and
must fail closed.

## 8. Historical V3 compatibility

V3 meaning remains immutable. No automatic V3-to-V4 state migration is
required. After P0 is implemented, the support matrix is:

| V3 artifact | Historical classification after V4 | Meaning |
| --- | --- | --- |
| `FullStateDigestV3` | `READABLE_VERIFIABLE_ONLY` | Can be parsed and verified against preserved V3 canonical bytes; it is not produced for current V4 state. |
| `EnvironmentCheckpointV3` | `ARCHIVED_ENGINE_REQUIRED` | Its embedded runtime state cannot be restored by the V4 current engine without an archived V3 engine. |
| `CheckpointDigestV3` | `READABLE_VERIFIABLE_ONLY` | The digest identity can be checked against preserved V3 checkpoint inputs; it is not a current checkpoint identity. |
| `ReplayManifestV3` | `READABLE_VERIFIABLE_ONLY` | Detached schema/identity-chain validation remains possible without executing V4 semantics. |
| `ReplayStepV3` | `READABLE_VERIFIABLE_ONLY` | Detached step identity can be checked under V3 rules; no V4 replay step is inferred. |
| `AuthoritativeReplayV3` | `ARCHIVED_ENGINE_REQUIRED` for semantic execution; structurally readable | Re-execution and projection parity require the archived V3 engine; no V3 step is reinterpreted as V4. |

## 9. Observation payload versioning

S1/M3 temporal data requires a new payload identity:

```text
OBSERVATION_ENVELOPE = observation-envelope.v1
OBSERVATION_DIGEST = mtgml.observation-digest.v1
OBSERVATION_PAYLOAD_NEXT_ID = synthetic-m3-observation.v1
OBSERVATION_PAYLOAD_CURRENT_AFTER_P0 = synthetic-m3-observation.v1
OBSERVATION_PAYLOAD_M2_CLASSIFICATION_AFTER_P0 = READABLE_VERIFIABLE_ONLY
```

The new payload is a canonical compact UTF-8 JSON object with sorted keys,
canonical decimal-string IDs, no omitted optional priority value, and exactly
these semantic fields in addition to the envelope-bound perspective and
revision context:

The canonical payload has this closed object shape (the `priority` value is
exactly one of the two shown alternatives):

```json
{
  "schema_version": "synthetic-m3-observation.v1",
  "active_player": "<PlayerId>",
  "turn_number": "<u64>",
  "phase": "<closed phase id>",
  "step": "<closed step id>",
  "priority": {"kind": "none"}
}
```

or:

```json
{
  "schema_version": "synthetic-m3-observation.v1",
  "active_player": "<PlayerId>",
  "turn_number": "<u64>",
  "phase": "<closed phase id>",
  "step": "<closed step id>",
  "priority": {"kind": "held_by", "player": "<PlayerId>"}
}
```

The admitted phase IDs are `beginning`, `precombat_main`, `combat`,
`postcombat_main`, and `ending`. The admitted step IDs are `untap`, `upkeep`,
`draw`, `precombat_main`, `beginning_of_combat`, `declare_attackers`,
`declare_blockers`, `combat_damage`, `end_of_combat`, `postcombat_main`,
`end_step`, and `cleanup`.

The payload schema must reject phase/step combinations not admitted by the
closed `TurnPosition`, and `priority.kind = none` is explicit rather than a
missing field. Its bytes are validated canonically before the existing V1
observation digest is calculated. `synthetic-m2-observation.v1` is never
rewritten to carry temporal fields. `ReplayManifestV4` must bind the selected
payload codec explicitly so replay reprojection cannot silently select an
older payload meaning. Existing `InformationStateDigestV2` remains valid in
this plan because its current observation field carries the codec identity and
canonical payload bytes.

The payload is player-visible only through the existing read-only projection
boundary. Paired states that differ only in unauthorized hidden identities,
RNG state, trusted IDs, or global allocation history must retain equal
player-safe bytes and equal error classes.

## 10. Authoritative events and StateDelta

The implementation needs typed trusted audit meaning for temporal state
changes, but this ADR does not add Rust enum variants or a public event
hierarchy. The P0/S1 design must provide typed internal event/delta families
for:

| Semantic change | Trusted audit requirement | Player/public consequence |
| --- | --- | --- |
| Turn/phase/step entry and exit | Typed temporal boundary event and exact `StateDelta` mutation; no free-form label. | Authorized phase/step appears in observation; a one-to-one public event is not required. |
| Active-player switch | Typed `ActivePlayerChanged` audit operation with before/after player. | Authorized active player appears; trusted IDs remain absent. |
| Ordinary untap | Typed `UntapCompleted` audit operation listing the canonical affected set and typed tapped-state deltas. | Public tapped consequences may appear through existing authorized products; no untap Decision or implicit pass. |
| Priority-bearing boundary | Typed `PriorityRequiredBoundary` audit marker tied to validated `PriorityState::HeldBy`; pass-only validation is recorded in trusted diagnostics. | Authorized priority availability/holder appears in observation. |
| Combat participation/assignment | Typed combat-state delta operations; explicit declaration Decision remains the only player choice. | Authorized actor sees only safe candidate/projection values. |
| Damage/marks/life | Existing typed `LifeChanged`/`ObjectTapped` style families plus new typed mark/damage operations where required. | Authorized public consequences only. |
| Zone/LKI transition | Existing typed `ZoneTransition` with old/new snapshots remains the LKI boundary. | Existing observation/knowledge lifecycle controls redaction and opaque identity. |

StateDelta remains a complete, reapplicable trusted product. Not every state
field mutation becomes a player-facing event. Trusted events, StateDelta,
after-state, digest, next Decision, and status must agree before the
environment commits.

## 11. Forced progress and replay semantics

The accepted transaction model remains:

```text
explicit player response
→ rules-owned transition
→ mandatory deterministic forced progress
→ stop at next Decision, outcome, or unsupported boundary
→ one atomic environment commit
```

Production requires one internal rules-owned forced-progress primitive shared
by the normal response transaction, reset/initial stabilization where
applicable, and the T0 forced-progress proof. It is an internal Rust path; it
is not a player action, public API, replay action by default, second scheduler,
or second rules engine. It must consume a validated state, execute the same
authoritative rule ownership path, and fail closed at unsupported mandatory
work.

After a player response, all deterministic consequences through the next
decision/outcome remain inside the same `ReplayStepV4`. The replay counter
therefore continues to mean one meaningful player-controlled decision per
step. At reset/episode initialization, the controller must stabilize to the
first real Decision before constructing the initial checkpoint. No fabricated
response, implicit pass, `decisions_submitted` increment, or initialization
replay step is emitted for no-choice forced progress.

T0 adds the following proof class:

```text
FORCED_PROGRESS_CONFORMANCE
```

It must assert no fabricated response, exact before/after state, typed
authoritative events, complete StateDelta, digest, status, next Decision,
player products, deterministic stop condition, checkpoint/restore parity,
fork parity, replay parity, and fail-closed unsupported forced paths.

## 12. T0, S1, and execution order

The exact order is:

```text
M3.P0_STATE_IDENTITY_CUT
  semantic-neutral state/digest/checkpoint/replay/observation structure
  capability count added = 0
  Magic semantics implemented = NO

→ M3.T0
  thin private conformance facade over the real Rust kernel
  forced-progress proof included
  capability count added = 0
  Magic semantics implemented = NO

→ M3.S1 = rules/turn-structure@0.1.0
  first real bounded Magic capability
```

P0 precedes T0 because the current T0 proof products are coupled to V3 state,
checkpoint, and replay identities. T0 then precedes S1 because it proves the
real-kernel conformance path, including no-choice forced progress, without
becoming a semantic implementation. S1 remains unchanged in scope: fixed
two-player temporal order, ordinary untap, no-priority classification, and
active-player switch after cleanup; draw, cleanup damage removal, priority,
combat declarations, stack, triggers, layers, extra turns, multiplayer, and
Commander remain excluded from S1.

## 13. Interaction obligations and M3/M4 boundary

All sixteen `M3-ENTRY-001` through `M3-ENTRY-016` obligations are retained.
Their capability identities are updated from Foundation V1 to V2 only where
the document reference changes; the two removed DAG edges do not remove any
obligation. The status is:

```text
INTERACTION_OBLIGATIONS_REVIEWED = 16
INTERACTION_OBLIGATIONS_RETAINED = 16
INTERACTION_OBLIGATIONS_UPDATED = 0 identity changes; V2 authority reference only
SATISFIED_INTERACTION_EVIDENCE = 0
```

No interaction row is marked satisfied by this design review. Standard census
data remains downstream stress input, OD-003 remains downstream, and no card,
deck, format, bundle, or certification claim is created.

## 14. Candidate change boundary

This hardening candidate may change documentation, the versioned Foundation
authority, the capability registry lifecycle metadata, the normative-document
register, and current-status tests. It must not change production behavior:

```text
PRODUCTION_RUST_CHANGED = NO
PRODUCTION_PYTHON_CHANGED = NO
MAGIC_SEMANTICS_CHANGED = NO
RULES_KERNEL_BEHAVIOR_CHANGED = NO
V4_IMPLEMENTED = NO
T0_IMPLEMENTED = NO
S1_IMPLEMENTED = NO
CARDS_ADDED = 0
DECKS_ADDED = 0
```

No V4 type, forced-progress API, T0 facade, semantic event implementation,
Card IR definition, or public RulesCase protocol is introduced here.

## 15. Verification and reauthorization

The hardening branch must pass repository-owned documentation, repository,
schema, registry, focused-status, fast, integration, and diff checks that are
actually executable in the environment. The Windows `just` wrapper is a
separate result and must remain `BLOCKED` if `/bin/bash` is unavailable. A
fresh exact-head self-review must find:

```text
BLOCKER = 0
MAJOR = 0
```

Hosted PR checks are reported separately and are not inferred from local
results. The PR must remain unmerged. After merge, a separate invocation must
fetch exact remote `master`, verify merge-tree parity and hosted evidence,
review ADR 0054, Foundation V2, the registry, README/ROADMAP, and Issue #178,
then post a new authorization record. Until that happens:

```text
M3_PRE_T0_HARDENING = CANDIDATE
M3_AUTHORIZED = NO
M3_STARTED = NO
AUTHORIZATION_HEAD = NOT_SET
REAUTHORIZATION_REQUIRED_AFTER_MERGE = YES
```

The future record must preserve:

```text
PRIOR_AUTHORIZATION_HEAD =
0f13b43680ea7d0b043c5baee59eb2ed3c364ecc

M3_PRE_T0_HARDENING_REVIEW = APPROVE
AUTHORIZATION_HEAD = <new exact master SHA>
M3_PLAN_STATUS = FROZEN
INITIAL_FOUNDATION = FROZEN
CAPABILITY_CLOSURE = FROZEN
DEPENDENCY_DAG = FROZEN
STATE_IDENTITY_MIGRATION_PLAN = FROZEN
T0_FORCED_PROGRESS_CONTRACT = FROZEN
S1 = rules/turn-structure@0.1.0
M3_AUTHORIZED = YES
M3_STARTED = NO
AUTHORIZED_NEXT_TASK = M3.P0_STATE_IDENTITY_CUT
```

This ADR never performs that future authorization.
