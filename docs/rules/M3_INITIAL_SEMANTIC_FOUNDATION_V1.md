# M3 Initial Semantic Foundation V1

**Status:** candidate entry decision; acceptance requires merge of the entry PR
**Stability:** provisional, versioned M3 scope
**Artifact version:** `m3.initial-semantic-foundation.v1`
**Candidate baseline:** `50f71adfdcd0d0f6ad50506290617e045baf0c28`
**M3 state:** `M3_STARTED = NO`, `M3_AUTHORIZED = NO`

This document is the exact operational scope record for the candidate M3 Entry
Decision. It becomes a durable accepted scope record only when the entry PR is
merged. It never authorizes M3 by itself. The separate post-merge authorization
protocol in this document remains mandatory.

## 1. Decision boundary and authority

This artifact is governed by:

- the normative hierarchy in [`docs/NORMATIVE_HIERARCHY.md`](../NORMATIVE_HIERARCHY.md);
- accepted ADR 0022 for capability identity and lifecycle;
- accepted ADR 0041 for semantic domains and explicit ownership;
- accepted ADR 0051 for the Comprehensive Rules snapshot;
- accepted ADR 0052 for the format-neutral initial M3 boundary;
- [`docs/ROADMAP.md`](../ROADMAP.md) for milestone ordering;
- [`docs/OPEN_DECISIONS.md`](../OPEN_DECISIONS.md) for the decision register;
- Issue [#178](https://github.com/chrismaghuhn/Manafold/issues/178) as the live M3 tracker;
- Issue [#163](https://github.com/chrismaghuhn/Manafold/issues/163) as planning provenance;
- the preserved Issue #129 research input as a non-normative conformance and interaction input.

The exact Comprehensive Rules authority for every node that cites Magic rules
is:

```text
snapshot_id = wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f
effective_date = 2026-08-07
artifact = MagicCompRules 20260819.txt
sha256 = 4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f
```

The artifact was independently acquired during this decision from the exact
official URL recorded by ADR 0051. Its measured length was `977822` bytes and
its SHA-256 matched the identity above. No newer rules release is substituted.

The snapshot identity is authority metadata, not evidence that any rule or
capability is implemented, covered, or certified.

## 2. M3 entry state

The candidate is based on the verified current `origin/master`:

```text
MASTER_AT_START = 50f71adfdcd0d0f6ad50506290617e045baf0c28
PR_181_MERGE_HEAD = 50f71adfdcd0d0f6ad50506290617e045baf0c28

FINAL_FOUNDATION_CLOSURE = PASS
CANONICAL_FINDINGS_DISPOSITIONED = 53/53
OPEN_P0_BLOCKERS = 0
OPEN_P1_BLOCKERS = 0
REQUIRED_P2_BLOCKERS = 0
PRE_M3_REMEDIATION_FREEZE = PASS
FOUNDATION_READY_FOR_M3 = YES
CORE_MODULARIZATION = COMPLETE
PRE_M3_GOVERNANCE_CLEANUP = COMPLETE

OD_003 = OPEN / M4 / NOT_M3_BLOCKING
OD_004 = RESOLVED
OD_019 = RESOLVED
UNRESOLVED_OPEN_DECISION_M3_BLOCKERS = 0

FND-026B = BLOCKED_CONTRACT_AMBIGUITY / NONBLOCKING
FND-026C = DEFERRED_P2
HRD-006 = DEFERRED_P2

M3_ENTRY_DECISION = CANDIDATE
INITIAL_FOUNDATION = CANDIDATE
M3_STARTED = NO
M3_AUTHORIZED = NO
AUTHORIZATION_HEAD = NOT_SET
```

The retained findings remain at their accepted dispositions. This decision
does not restart the foundation audit and does not convert a deferred or
blocked-contract item into `PASS`.

## 3. M3.T0 proof infrastructure freeze

`M3.T0` is proof infrastructure, not a Magic capability and not a registry
entry.

```text
M3.T0 = thin internal conformance facade over real authoritative execution
M3.T0 != first Magic capability
T0_OWNER = crates/mtgml-conformance
T0_VISIBILITY = repository-internal / experimental / non-public
T0_WIRE_STATUS = non-wire
T0_PUBLIC_SCHEMA_FREEZE = NO
NEW_MAGIC_CAPABILITIES_FROM_T0 = 0
NEW_SUPPORT_CLAIMS_FROM_T0 = 0
```

### 3.1 T0 included scope

The facade may provide one typed internal case path containing:

| T0 input or product | Frozen requirement |
| --- | --- |
| Case identity | Stable internal case identity and authority metadata; not a public protocol identity. |
| Setup | Complete validated setup supplied explicitly. Setup distinguishes structural validity, case validity, and historical reachability. |
| Driver | Real trusted controller, player endpoint, and Rust kernel path. |
| Decision | Exact current authoritative and perspective-visible decision assertions. |
| Response | Explicit submitted response; no driver-generated target, mode, payment, order, combat choice, or pass. |
| Acceptance | Exact accepted or rejected result. |
| Events | Exact ordered authoritative event product. |
| Delta | Exact complete `StateDelta` product and full reapplication check. |
| Result state | Independently authored complete expected state where the case needs it. |
| Digest | Exact resulting state digest. |
| Continuation | Exact next decision and typed continuation state where present. |
| Status | Exact `EpisodeStatus`. |
| Player products | Exact per-player observation, information state, visible decision, observed events, and `PlayerStep`. |
| Rejection | Complete rejection/nonmutation fingerprint. |
| Parity hooks | Checkpoint/restore, fork, replay, and reprojection parity hooks. |
| Diagnostics | Deterministic first-divergence packet with trusted details kept outside player products. |
| Failure packet | Trusted reproducible failure-packet integration with engine/build, authority, input, expected/actual identity, invariant detail, and deterministic rerun command. |

T0 uses the existing authoritative proof machinery. It does not define a second
rules program, a generic rules DSL, a public `RulesCaseV1` schema, a public wire
contract, or a separate interaction registry.

### 3.2 T0 prohibited scope

The facade must not independently calculate or repair any Magic meaning. It must
not calculate targets, costs, payments, triggers, replacements, prevention,
state-based actions, layers, copy semantics, combat legality, or Magic
outcomes. It must not invent rule-relevant history, choose a first/default/
random candidate, or pass priority implicitly. An invalid or unsupported case
fails before an incorrect semantic product is emitted.

### 3.3 T0 exit evidence frozen for later implementation

The later T0 implementation must provide all of the following on the real Rust
path:

1. one existing accepted synthetic case;
2. one existing rejected synthetic case with a complete nonmutation fingerprint;
3. one explicit multi-step synthetic case;
4. perspective-safe assertion for every required player product;
5. at least one exercised checkpoint, fork, or replay hook;
6. a deliberate mismatch with deterministic first-divergence output;
7. independently authored expectations for semantic values, distinct from harness comparator self-tests;
8. trusted reproducible failure-packet integration where a case fails;
9. focused and repository gates with executed evidence.

Every item is `NOT_SATISFIED_AT_ENTRY`. T0 implementation begins only after
post-merge explicit M3 authorization.

## 4. Initial Semantic Foundation scope

### 4.1 Boundary

The Initial Semantic Foundation is the smallest reviewed closure that provides:

- a trustworthy two-player temporal/action skeleton;
- basic empty-stack priority progression;
- a bounded normal combat path;
- the draw and cleanup consequences required by the selected turn path;
- combat damage, life, marked damage, bounded state-based actions, and the zone/incarnation consequences required by that path;
- explicit decisions, perspective-safe products, atomic rejection, deterministic events/deltas, and parity evidence through the existing substrate.

The foundation is format-neutral:

```text
INITIAL_M3_FORMAT = FORMAT_NEUTRAL
INITIAL_M3_FORMAT_STATE = FormatState::None
COMMANDER_SEMANTICS = OUT_OF_INITIAL_FOUNDATION
```

Foundational engine contracts remain separate substrates rather than Magic
capability nodes. They include the Decision protocol, state transaction,
observation projection, information-state tracking, replay, checkpoint/fork,
RNG, state hashing, and event/delta auditing. Their existing ownership and
version identities remain binding.

### 4.2 Direct roots

The direct roots are the reviewed semantic goals of the entry scope:

```text
DIRECT_ROOTS =
  rules/turn-structure@0.1.0
  rules/basic-priority@0.1.0
  rules/draw-card@0.1.0
  rules/cleanup-reset@0.1.0
  rules/combat-damage@0.1.0
```

`rules/draw-card` and `rules/cleanup-reset` are direct roots because they are
required turn-action consequences of the selected integration path. They are
not hidden implementation details.

### 4.3 Transitive dependency set and resolved closure

```text
TRANSITIVE_DEPENDENCIES =
  rules/combat-phase@0.1.0
  rules/declare-attackers@0.1.0
  rules/declare-blockers@0.1.0
  rules/damage-and-life@0.1.0
  rules/state-based-actions-combat@0.1.0
  rules/zone-incarnation@0.1.0

RESOLVED_CLOSURE_COUNT = 11
```

The resolved closure is exactly:

```text
1.  rules/turn-structure@0.1.0
2.  rules/basic-priority@0.1.0
3.  rules/draw-card@0.1.0
4.  rules/cleanup-reset@0.1.0
5.  rules/combat-damage@0.1.0
6.  rules/combat-phase@0.1.0
7.  rules/declare-attackers@0.1.0
8.  rules/declare-blockers@0.1.0
9.  rules/damage-and-life@0.1.0
10. rules/state-based-actions-combat@0.1.0
11. rules/zone-incarnation@0.1.0
```

### 4.4 Capability identity table

The following table is the single authoritative identity record for this
candidate. A capability key is a support identity, not a runtime dispatch key.
`M3_target_lifecycle` names the required lifecycle at the bounded M3 exit; it
does not claim that the capability is currently implemented or covered.

| `key` | `version` | `role` | `summary` | `included_scope` | `explicit_exclusions` | `authority` | `dependencies` | `primary_semantic_owner` | `primary_orchestrator` | `physical_state_owner` | `decision_surface` | `information_risk` | `M3_target_lifecycle` | `entry_reason` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `rules/turn-structure` | `0.1.0` | `root` | Deterministic two-player turn, phase, and step structure. | Active-player ownership; turn numbering; beginning, precombat main, combat, postcombat main, and ending phase order; beginning/untap/upkeep/draw and ending/end/cleanup step order; no-priority classification for untap and ordinary cleanup; untap of all active-player permanents in the selected simple state; boundary entry/exit; active-player switch after cleanup. | Game-start procedure and first-turn draw skip; phasing; no-untap effects; extra or skipped turns/phases/steps; additional combat phases; effects that alter durations; phase-specific choices outside the selected roots; draw and damage removal execution, which belong to their named capabilities. | Pinned snapshot; CR `500.1`, `500.3`, `500.12`, `501.1`, `502.3-502.4`, `505.1-505.2`, `512.1`, `513.1`, `514.3`. | none | `turn` | `turn-priority-progression` | `mtgml-state::EngineState` | none in the isolated capability; integration receives explicit priority products | low | `covered` | Direct temporal/action root. |
| `rules/basic-priority` | `0.1.0` | `root` | Empty-stack priority windows and explicit pass progression. | Active player receives priority at applicable step/phase boundaries; only `pass_priority` is offered in the selected empty-stack scope; each explicit pass transfers priority; two successive passes end the current priority-bearing step/phase; state-based-action check precedes priority; no hidden automatic pass. | Non-pass actions; spells; activated abilities; stack objects; mana abilities; triggered abilities; replacement/prevention; priority during unsupported cleanup conditions; multiplayer priority. | Pinned snapshot; CR `117.1`, `117.2c-117.2d`, `117.3a`, `117.3d`, `117.4-117.5`, `500.2`. | `rules/turn-structure@0.1.0`, `rules/state-based-actions-combat@0.1.0` | `priority` | `turn-priority-progression` | `mtgml-state::EngineState` | Explicit `pass_priority` response for each offered window | medium | `covered` | Direct priority root required to close temporal windows without guessing; its pre-priority SBA check is part of the selected closure. |
| `rules/draw-card` | `0.1.0` | `root` | One ordinary draw-step card draw. | During the selected draw step, the active player puts the top card of a nonempty library into that player’s hand; the move is one authoritative zone/incarnation transition; the active player receives authorized knowledge and the opponent does not receive the card identity; no RNG is consumed by the draw itself. | Empty-library attempted draw; draw replacement; multiple draws; effects that reveal, modify, or replace a draw; drawing outside the draw step; mulligan and game-start draws; card-specific draw triggers. | Pinned snapshot; CR `121.1-121.2`, `401.2`, `402.1`; CR `121.4` is the fail-closed boundary for an empty library. | `rules/turn-structure@0.1.0`, `rules/zone-incarnation@0.1.0` | `turn` | `turn-step-action` | `mtgml-state::EngineState` | none | high | `covered` | Direct turn-action consequence required by the selected normal-turn integration. |
| `rules/cleanup-reset` | `0.1.0` | `root` | Bounded cleanup reset for combat damage. | During an ordinary cleanup step with no discard requirement, remove damage marked on permanents and complete the no-priority cleanup path; preserve all earlier marked damage until this boundary. | Hand-size discard; cleanup-trigger exception; duration expiry for unsupported continuous effects; mana-pool semantics; additional cleanup steps; effects that modify cleanup. | Pinned snapshot; CR `120.6`, `514.1-514.3`. | `rules/turn-structure@0.1.0`, `rules/damage-and-life@0.1.0` | `turn` | `cleanup-damage-reset` | `mtgml-state::EngineState` | none for the selected hand-size-safe case | medium | `covered` | Direct consequence required to make marked combat damage and the next turn semantically coherent. |
| `rules/combat-damage` | `0.1.0` | `root` | One bounded normal combat-damage step. | One combat-damage step; each attacking/blocking creature with positive power assigns its power; unblocked attackers damage the defending player; a blocked attacker and its single blocker damage each other; all assigned damage is dealt simultaneously; the selected post-damage SBA pipeline runs before the next priority window. | First strike/double strike; trample; deathtouch; lifelink; infect/wither/toxic; planeswalker/battle damage; multiple blockers per attacker; damage assignment choices; damage replacement/prevention; combat effects; creatures entering combat by effects. | Pinned snapshot; CR `510.1a-510.3`, `120.2a`, `120.3a`, `120.3e`, `120.4b-120.4d`, `120.5-120.6`. | `rules/declare-blockers@0.1.0`, `rules/damage-and-life@0.1.0`, `rules/state-based-actions-combat@0.1.0` | `combat` | `combat-damage-pipeline` | `mtgml-state::EngineState` | none in the one-blocker, no-assignment-choice scope | high | `covered` | Direct bounded combat root. |
| `rules/combat-phase` | `0.1.0` | `dependency` | Combat phase context and step progression. | Beginning of combat, declare attackers, declare blockers, combat damage, and end of combat order; fixed defending opponent in a two-player game; skip of declare blockers and combat damage when no attackers exist; priority at beginning and end boundaries; removal from combat when the phase ends. | Multiplayer defending-player selection; extra/skipped combat phases; first/double-strike second damage step; effects putting attackers/blockers onto the battlefield; combat effects that alter participation. | Pinned snapshot; CR `506.1-506.3`, `507.1-507.2`, `508.8`, `511.1-511.3`. | `rules/turn-structure@0.1.0`, `rules/basic-priority@0.1.0` | `combat` | `combat-priority-progression` | `mtgml-state::EngineState` | none; declaration decisions belong to declaration capabilities | medium | `covered` | Required dependency for all selected combat steps. |
| `rules/declare-attackers` | `0.1.0` | `dependency` | Bounded attacker declaration and legality. | Active player chooses an unordered subset of eligible, untapped creatures controlled continuously since the turn began; the fixed opponent is the attack target; selected creatures tap and become attacking; empty attack is representable; all choices are explicit and canonical. | Haste; defender; attack restrictions or requirements; attack costs; banding; attacking planeswalkers/battles; effects putting creatures onto the battlefield attacking; multiplayer target selection; card-specific attack abilities. | Pinned snapshot; CR `506.2-506.3`, `508.1a-508.1d`, `508.1f`, `508.1k`, `508.2`, `508.8`. | `rules/combat-phase@0.1.0` | `combat` | `combat-declaration` | `mtgml-state::EngineState` | `ChooseMany` over the complete eligible attacker set, minimum `0`, maximum eligible count | high | `covered` | Required dependency for a bounded combat path and for legal attacker soundness/completeness. |
| `rules/declare-blockers` | `0.1.0` | `dependency` | Bounded blocker assignment with at most one blocker per attacker. | Defending player explicitly assigns untapped controlled creatures to attacking creatures; every blocker blocks at most one attacker and every attacker has at most one blocker in this scope; the empty assignment is representable; the assignment commits atomically after the internal typed continuation completes; no priority occurs between assignment stages. | Multiple blockers per attacker; evasion; blocking restrictions or requirements; block costs; banding; effects putting creatures onto the battlefield blocking; damage assignment ordering; multiplayer. | Pinned snapshot; CR `509.1a-509.1h`, `509.2`; CR `510.1c-510.1d` defines the excluded multiple-blocker assignment boundary. | `rules/declare-attackers@0.1.0` | `combat` | `combat-declaration` | `mtgml-state::EngineState` | Internal typed continuation of `ChooseOne`: for each canonically ordered blocker, choose a legal attacker or an explicit `confirm` no-assignment candidate; each stage is actor-bound and nonmutating on rejection | high | `covered` | Required dependency for a complete bounded block assignment and unambiguous damage recipients. |
| `rules/damage-and-life` | `0.1.0` | `dependency` | Basic combat-damage results for life and marked creature damage. | Apply simultaneous unmodified combat damage; damage to a player causes that player to lose the amount; damage to a creature without infect/wither marks that amount; retain source, affected-object, and event roles; retain marks until cleanup. | Poison, counters from infect/wither, lifelink, toxic, prevention, replacement, damage redirection, noncombat damage, negative/unsupported characteristic calculations, life-payment semantics. | Pinned snapshot; CR `119.2-119.3`, `120.1`, `120.2a`, `120.3a`, `120.3e`, `120.4b-120.4d`, `120.6`. | none | `damage` | `combat-damage-pipeline` | `mtgml-state::EngineState` | none | high | `covered` | Required consequence of the selected combat-damage root and cleanup root. |
| `rules/state-based-actions-combat` | `0.1.0` | `dependency` | Bounded state-based action fixed point after selected damage. | Before a player receives priority, repeatedly check and perform applicable actions simultaneously: a player at `0` or less life loses; a creature with toughness `0` or less is put into its owner’s graveyard; a creature with lethal marked damage is destroyed; repeat until stable; simultaneous player loss produces the accepted terminal outcome. | Poison; empty-library loss; tokens; deathtouch; indestructible/regeneration; legend/world/Aura/Equipment/Role/counter/Saga/battle actions; replacement effects; triggered abilities; any unsupported condition. | Pinned snapshot; CR `104.3a-104.4a`, `119.6`, `704.1-704.3`, `704.5a`, `704.5f-704.5g`, `704.8`, `701.8`. | `rules/damage-and-life@0.1.0`, `rules/zone-incarnation@0.1.0` | `state_based_actions` | `combat-damage-pipeline` | `mtgml-state::EngineState` | none | high | `covered` | Required post-damage consequence and terminal boundary for the selected combat path. |
| `rules/zone-incarnation` | `0.1.0` | `dependency` | Selected zone moves with object incarnation and identity continuity. | Battlefield-to-owner-graveyard moves caused by selected SBAs and library-to-owner-hand moves caused by selected draws; every zone change creates a new `GameObjectId`; `PhysicalCardId` continuity is preserved; old-incarnation snapshot is available for the authoritative event/audit boundary; ordered library top is consumed; perspective identity/knowledge is updated through existing projection contracts. | Generic zone moves; shuffle/randomization; copy continuity; attachments; tokens; exile/command zone; cast/resolution exceptions; general LKI-dependent triggers; hidden randomization identity retirement beyond existing substrate behavior. | Pinned snapshot; CR `400.1-400.7`, `400.7j`, `401.1-401.2`, `402.1`, `700.4`; existing identity and information contracts remain binding. | none | `zones_identity` | `zone-transition-pipeline` | `mtgml-state::EngineState` | none | high | `covered` | Required identity consequence of draw and bounded SBA paths. |

### 4.5 Version policy

Every node uses `0.1.0` because this is the first frozen semantic meaning for
that node. A later change that can alter legal actions, state, visibility,
events, outcomes, replay, or dataset meaning creates a new semantic version.
Evidence-only additions retain the version when the declared meaning does not
change. File layout, Rust function names, and card names never define a
capability version.

## 5. Dependency DAG

### 5.1 Edge meaning

For every edge below:

```text
A -> B
```

means that capability `A` semantically requires capability `B`. It does not
describe Rust imports, file placement, helper use, or runtime dispatch.

| Edge | Semantic rationale |
| --- | --- |
| `rules/basic-priority -> rules/turn-structure` | Priority needs a canonical current phase/step and active-player context. |
| `rules/basic-priority -> rules/state-based-actions-combat` | CR `117.5` requires the selected state-based-action check before a player receives priority, including after a damage event. |
| `rules/draw-card -> rules/turn-structure` | A draw is legal only as the selected draw-step turn-based action. |
| `rules/draw-card -> rules/zone-incarnation` | Moving the library top card into hand changes zone and game-object incarnation while preserving physical-card continuity. |
| `rules/cleanup-reset -> rules/turn-structure` | Cleanup reset is legal only at the selected cleanup boundary and inherits its no-priority classification. |
| `rules/cleanup-reset -> rules/damage-and-life` | Cleanup removes the marked-damage state produced by the selected damage capability. |
| `rules/combat-phase -> rules/turn-structure` | Combat is one phase in the fixed turn structure and uses its active-player ownership. |
| `rules/combat-phase -> rules/basic-priority` | Beginning/end and other priority-bearing combat boundaries require the empty-stack pass protocol. |
| `rules/declare-attackers -> rules/combat-phase` | Attacker legality and commitment require the selected beginning-of-combat and declare-attackers context. |
| `rules/declare-blockers -> rules/declare-attackers` | Blockers can only be assigned to the attacking set produced by the preceding declaration. |
| `rules/combat-damage -> rules/declare-blockers` | Damage recipients and blocked/unblocked status require the completed bounded blocker assignment. |
| `rules/combat-damage -> rules/damage-and-life` | Combat damage requires the selected player-life and marked-creature-damage results. |
| `rules/combat-damage -> rules/state-based-actions-combat` | The selected combat path is not complete until its post-damage state-based-action fixed point runs before priority. |
| `rules/state-based-actions-combat -> rules/damage-and-life` | Lethal damage and life-loss conditions are derived from the selected damage results. |
| `rules/state-based-actions-combat -> rules/zone-incarnation` | Creature death and toughness-based graveyard moves create new object incarnations. |
| `rules/turn-structure`, `rules/damage-and-life`, `rules/zone-incarnation` | These are the acyclic semantic leaves of the selected closure; no lower Magic capability is hidden beneath them. |

### 5.2 Readable graph

```text
rules/turn-structure@0.1.0

rules/basic-priority@0.1.0
├── rules/turn-structure@0.1.0
└── rules/state-based-actions-combat@0.1.0
    ├── rules/damage-and-life@0.1.0
    └── rules/zone-incarnation@0.1.0

rules/draw-card@0.1.0
├── rules/turn-structure@0.1.0
└── rules/zone-incarnation@0.1.0

rules/cleanup-reset@0.1.0
├── rules/turn-structure@0.1.0
└── rules/damage-and-life@0.1.0

rules/combat-damage@0.1.0
├── rules/declare-blockers@0.1.0
│   └── rules/declare-attackers@0.1.0
│       └── rules/combat-phase@0.1.0
│           ├── rules/turn-structure@0.1.0
│           └── rules/basic-priority@0.1.0
│               ├── rules/turn-structure@0.1.0
│               └── rules/state-based-actions-combat@0.1.0
│                   ├── rules/damage-and-life@0.1.0
│                   └── rules/zone-incarnation@0.1.0
├── rules/damage-and-life@0.1.0
└── rules/state-based-actions-combat@0.1.0
    ├── rules/damage-and-life@0.1.0
    └── rules/zone-incarnation@0.1.0
```

### 5.3 Acyclicity and frontier

```text
DEPENDENCY_DAG_REVIEWED = YES
DEPENDENCY_CYCLES = 0
LEAVES =
  rules/turn-structure@0.1.0
  rules/damage-and-life@0.1.0
  rules/zone-incarnation@0.1.0
```

The implementable semantic frontier before any selected node is implemented is
exactly those three leaves. `rules/turn-structure` is selected as S1. The
priority, draw, cleanup, combat, and SBA nodes remain downstream of explicit
dependencies. Runtime interaction between turn and priority is owned by one
orchestrated process; it is not represented as a reverse dependency edge.

## 6. Semantic ownership graph

### 6.1 Domain inventory

The domain names below are conceptual ownership boundaries, not a frozen Rust
crate tree.

| Domain | Owns | Does not own |
| --- | --- | --- |
| `turn` | Turn ownership, phase/step meaning, selected turn-based actions, temporal boundaries | Priority legality, environment commit, projection, replay append |
| `priority` | Empty-stack priority window and explicit pass meaning | Turn state storage, stack/spell semantics, environment scheduling |
| `combat` | Combat context, declaration legality, combat participation, combat integration order | Damage result calculation, state storage, projection, replay append |
| `damage` | Selected combat-damage event results, player life loss from damage, marked creature damage | SBAs, zone commitment, projection |
| `state_based_actions` | Selected SBA predicates, simultaneous application, fixed-point order, terminal consequence | General triggers, replacement, state storage, environment commit |
| `zones_identity` | Selected zone transitions, game-object incarnation, physical-card continuity, required old-incarnation snapshot | Generic information projection, arbitrary zone mechanics, replay interpretation |
| `conformance` | T0 case vocabulary, independent expectations, exact comparison, diagnostic evidence | Magic legality, semantic outcome calculation, support registration |

The following existing planes remain authorities without becoming semantic
owners of a selected Magic capability:

| Plane | Fixed authority |
| --- | --- |
| `mtgml-state` | Sole physical owner of complete `EngineState`, identity, knowledge, and format state. |
| `mtgml-rules` | Trusted transition execution and candidate state/event/delta production. |
| `mtgml-environment` | Precommit environment product, projection validation, counters, replay/checkpoint commit or discard. |
| `mtgml-observation` | Read-only redaction and perspective-safe products. |
| `mtgml-replay` | Replay identity and validation; it does not reinterpret rules. |
| `mtgml-conformance` | Internal proof facade; it does not calculate Magic legality. |

No domain may commit `EngineState`, commit environment state, append replay,
advance environment counters, bypass Decision, own projection legality, retain
hidden mutable semantic state, or obtain unrestricted mutable full-state access.

### 6.2 Cross-domain process orchestrators

Each process that touches more than one semantic owner has exactly one primary
orchestrator. The orchestrator owns integration order and delegates local
legality; it does not duplicate it.

| Process | Touched owners | Primary orchestrator | Integration order |
| --- | --- | --- | --- |
| Turn/step progression with priority | `turn`, `priority` | `turn-priority-progression` owned by `turn` | Enter boundary; perform selected turn-based action; run required SBA check; offer explicit priority; advance only after the required empty-stack pass sequence. |
| Turn-based draw | `turn`, `zones_identity`, information substrate | `turn-step-action` owned by `turn` | Validate draw-step context; consume the authoritative library top; commit zone/incarnation and authorized knowledge; then offer priority. |
| Cleanup damage reset | `turn`, `damage` | `cleanup-damage-reset` owned by `turn` | Validate ordinary cleanup; remove marked damage simultaneously; perform the no-priority cleanup exit. |
| Combat phase with priority | `combat`, `turn`, `priority` | `combat-priority-progression` owned by `combat` | Enter combat; run beginning-of-combat priority; invoke declarations in order; invoke damage; run end-of-combat priority; remove combat participation. |
| Attacker declaration | `combat`, Decision substrate | `combat-declaration` owned by `combat` | Read the complete eligible set; accept one explicit canonical set; validate; tap and mark attackers atomically; offer the next priority window. |
| Blocker declaration | `combat`, Decision substrate | `combat-declaration` owned by `combat` | Build the typed staged request; accept explicit blocker assignments; commit the complete bounded assignment atomically; offer the next priority window. |
| Combat damage pipeline | `combat`, `damage`, `state_based_actions`, `zones_identity` | `combat-damage-pipeline` owned by `combat` | Freeze assignments; assign and deal simultaneously; apply damage results; run SBA fixed point; commit terminal/zone consequences; only then return to priority. |
| Zone transition and identity | `zones_identity`, information substrate | `zone-transition-pipeline` owned by `zones_identity` | Capture old incarnation; apply selected move; allocate new authoritative incarnation; preserve or update physical/opaque identity according to existing contracts; validate projections before commit. |

The ownership graph is reviewed separately from the capability dependency DAG.
The only delegated semantic paths are:

```text
turn-priority-progression -> priority
combat-priority-progression -> priority
combat-damage-pipeline -> damage -> state_based_actions -> zones_identity
turn-step-action -> zones_identity
cleanup-damage-reset -> damage
```

There is no return edge from `priority`, `damage`, `state_based_actions`, or
`zones_identity` to a local owner. The apparent turn/priority mutual runtime
conversation is one named orchestrated process, not a semantic ownership cycle.

```text
SEMANTIC_OWNERSHIP_GRAPH_VERSION = m3.semantic-ownership.v1
SEMANTIC_OWNERSHIP_GRAPH_REVIEWED = YES
CAPABILITIES_WITHOUT_PRIMARY_OWNER = 0
CROSS_DOMAIN_PROCESSES_WITHOUT_ORCHESTRATOR = 0
OWNERSHIP_CYCLES = 0
```

## 7. M3.S1 selection

### 7.1 Candidate comparison

The comparison uses `L` = low, `M` = medium, and `H` = high. `Depth` is the
longest capability-dependency path to the candidate. The table compares
plausible first slices after the closure was derived; it does not select a
capability by file size or coding convenience.

| Candidate | Depth | Reuse | Scope boundedness | Conformance clarity | Decision complexity | Information risk | Cross-domain complexity | Replay/checkpoint relevance | Interaction burden | Fail-closed clarity | Premature abstraction risk |
| --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `rules/turn-structure@0.1.0` | 0 | H | H | H | L | L | M | H | M | H | L |
| `rules/zone-incarnation@0.1.0` | 0 | H | M | M | L | H | H | H | H | M | M |
| `rules/damage-and-life@0.1.0` | 0 | H | M | H | L | H | M | H | H | H | L |
| `rules/basic-priority@0.1.0` | 1 | H | M | H | M | M | H | H | H | H | M |
| `rules/combat-phase@0.1.0` | 2 | H | M | M | M | M | H | H | H | M | M |
| `rules/declare-attackers@0.1.0` | 3 | H | M | M | H | H | H | H | H | M | M |
| `rules/combat-damage@0.1.0` | 5 | H | M | M | L | H | H | H | H | M | M |

### 7.2 Exact S1 definition

```text
S1_ID = rules/turn-structure
S1_VERSION = 0.1.0
S1_SCOPE =
  two-player, format-neutral temporal structure with active-player ownership,
  fixed phase/step order, ordinary untap action, explicit no-priority
  classification, deterministic phase/step boundaries, and active-player
  switch after cleanup
S1_EXCLUSIONS =
  draw execution, cleanup damage removal, priority/pass semantics, combat
  declarations, spells, abilities, stack objects, triggers, replacements,
  continuous effects, game-start procedure, extra/skipped turns/phases/steps,
  additional combat phases, multiplayer, Commander
S1_DEPENDENCIES = none (Magic capability dependencies)
S1_PRIMARY_SEMANTIC_OWNER = turn
S1_PRIMARY_ORCHESTRATOR = turn-priority-progression for integrated use;
  none for the isolated turn-only proof path
S1_INFORMATION_RISK = LOW
S1_DECISION_SURFACE = NONE in the isolated scope; no auto-pass is permitted
S1_AUTHORITY_SNAPSHOT =
  wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f
S1_ADDITIONAL_OFFICIAL_AUTHORITY = NONE
S1_CR_SECTIONS =
  500.1, 500.3, 500.12, 501.1, 502.3-502.4, 505.1-505.2, 512.1, 513.1,
  514.3
S1_REQUIRED_EVENTS =
  turn_boundary_entered, phase_started, step_started, untap_completed,
  step_ended, phase_ended, active_player_changed, priority_required_boundary
S1_REQUIRED_STATE_DELTA =
  exact current turn/phase/step context, active-player identity, turn number,
  selected active-player permanent tapped-state changes from untap, and no
  mutation to life, zones, RNG, knowledge, replay, or environment counters
S1_REQUIRED_REJECTION_BEHAVIOR =
  reject unsupported priority-required advancement, game-start/extra/skip
  context, invalid temporal state, and any hidden choice without mutating the
  complete semantic/environment/player/replay fingerprint
S1_REQUIRED_PROJECTION_EVIDENCE =
  phase/step and active-player values are projected only through authorized
  player products; trusted IDs, hidden definitions, RNG, and diagnostics stay
  absent; paired unauthorized hidden-state variants produce identical bytes
S1_REQUIRED_REPLAY_CHECKPOINT_FORK_EVIDENCE =
  one accepted boundary parity case, checkpoint/restore future-boundary parity,
  equal-input fork parity, controlled divergent-input parity, and replay
  reprojection for the same boundary
S1_INTERACTION_OBLIGATIONS =
  M3-ENTRY-001, M3-ENTRY-002, M3-ENTRY-015, M3-ENTRY-016
S1_EXIT_CRITERIA =
  exact independent conformance expectations, deterministic boundary events and
  delta, rejection nonmutation, perspective-safe products, and applicable
  checkpoint/fork/replay parity all pass on the exact implementation head
```

The names in `S1_REQUIRED_EVENTS` are conformance expectation labels only. They
do not freeze Rust enum variants, public event names, wire fields, or a new
event hierarchy.

The minimal witnesses are synthetic and carry no card-support meaning:

```text
S1-W-UNTAP = validated turn-2 untap state with one tapped active-player permanent
S1-W-BOUNDARY = validated state at the end of an ordinary step with no pending stack/effect/trigger
```

No card, deck, Oracle distribution, bundle, or certification work is part of
S1.

## 8. Requirement to capability mapping

The following mappings are reviewed foundation requirements. They are not
implementation tasks and do not register support.

| Requirement ID | Authority | Meaning | Capability |
| --- | --- | --- | --- |
| `M3-REQ-TURN-001` | CR `500.1`, `501.1`, `502.3-502.4`, `505.1-505.2`, `512.1`, `513.1`, `514.3` | Fixed two-player phase/step order, ordinary untap, and no-priority boundaries. | `rules/turn-structure@0.1.0` |
| `M3-REQ-PRIORITY-001` | CR `117.3a`, `117.3d`, `117.4-117.5`, `500.2` | Explicit empty-stack priority and pass progression. | `rules/basic-priority@0.1.0` |
| `M3-REQ-DRAW-001` | CR `121.1`, `401.2`, `402.1` | One draw-step top-card move into the active player’s hand. | `rules/draw-card@0.1.0` |
| `M3-REQ-CLEANUP-001` | CR `120.6`, `514.1-514.3` | Remove marked damage at ordinary cleanup while retaining it before cleanup. | `rules/cleanup-reset@0.1.0` |
| `M3-REQ-COMBAT-001` | CR `506.1-506.3`, `507.1-507.2`, `508.8`, `511.1-511.3` | Fixed two-player combat context and step order. | `rules/combat-phase@0.1.0` |
| `M3-REQ-ATTACK-001` | CR `508.1a-508.1d`, `508.1f`, `508.1k`, `508.2` | Complete bounded attacker set decision and commitment. | `rules/declare-attackers@0.1.0` |
| `M3-REQ-BLOCK-001` | CR `509.1a-509.1h`, `509.2` | Complete bounded blocker assignment with explicit actor choices. | `rules/declare-blockers@0.1.0` |
| `M3-REQ-DAMAGE-001` | CR `510.1a-510.3`, `120.2a`, `120.3a`, `120.3e`, `120.4b-120.4d` | Simultaneous selected combat damage assignment and dealing. | `rules/combat-damage@0.1.0` |
| `M3-REQ-LIFE-MARKS-001` | CR `119.2-119.3`, `120.3a`, `120.3e`, `120.6` | Life loss and marked creature damage results. | `rules/damage-and-life@0.1.0` |
| `M3-REQ-SBA-001` | CR `104.3b`, `104.4a`, `704.1-704.3`, `704.5a`, `704.5f-704.5g`, `704.8` | Bounded simultaneous SBA fixed point and terminal consequence. | `rules/state-based-actions-combat@0.1.0` |
| `M3-REQ-INCARNATION-001` | CR `400.6-400.7`, `400.7j`, `700.4` | New game-object incarnation with physical-card continuity for selected moves. | `rules/zone-incarnation@0.1.0` |
| `M3-REQ-ARCH-001` | ADR `0041`; current Decision, Information, Replay, State, and Execution contracts | Keep explicit ownership, decision, information, atomicity, determinism, and parity boundaries. | Existing foundational substrates; not a Magic capability node |

## 9. Interaction obligations

The foundation uses the reviewed lifecycle:

```text
POTENTIAL_SEAM -> REVIEWED_OBLIGATION -> SATISFIED_EVIDENCE
```

The following seams are applicable to the exact closure and are promoted to
Entry-level obligations. Every row is reviewed at Entry, and every row has
`SATISFIED_EVIDENCE = NO` until later implementation produces the required
evidence.

| Obligation ID | Reviewed seam | Closure identities | Required evidence at M3 exit | Risk | Entry state |
| --- | --- | --- | --- | --- | --- |
| `M3-ENTRY-001` | turn progression × priority | `turn-structure`, `basic-priority` | Exact boundary/pass sequence; no priority at untap/ordinary cleanup; no implicit pass; state/event/delta/status parity. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-002` | step progression × forced progress | `turn-structure`, `draw-card`, `cleanup-reset` | Untap and cleanup automatic actions finish before priority; draw occurs exactly once; unsupported mandatory action fails closed. | medium | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-003` | draw × zone/incarnation × information | `draw-card`, `zone-incarnation` | Nonempty-library draw moves the correct top card, creates the correct new incarnation, updates only authorized knowledge, and preserves paired hidden-state bytes. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-004` | combat phase × priority | `combat-phase`, `basic-priority` | Beginning/end combat priority sequence and empty-stack advancement are exact; skipped combat steps occur only when no attackers exist. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-005` | declare attackers × Decision | `declare-attackers`, Decision substrate | Every bounded legal attacker subset is representable exactly once; every offered subset is legal; empty attack is explicit. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-006` | attack legality × control/status | `declare-attackers`, `turn-structure` | Continuous-control and untapped predicates are correct; haste, restrictions, requirements, and unsupported status are rejected before mutation. | medium | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-007` | declare blockers × Decision continuation | `declare-blockers`, Decision substrate | All bounded partial injective assignments, including no blocks, are explicit and complete; each stage rejects atomically; no stage silently defaults. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-008` | block assignment × combat damage | `declare-blockers`, `combat-damage` | Blocked/unblocked status and every source/recipient pair match the committed assignment; no stale or trusted-ID binding crosses the player boundary. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-009` | combat damage × life/marked damage | `combat-damage`, `damage-and-life` | Positive-power damage is simultaneous and exact; player life and creature marks match source and affected-object roles. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-010` | damage × SBA fixed point | `damage-and-life`, `state-based-actions-combat` | SBA checks occur before priority, all applicable selected actions are simultaneous, and the check repeats until stable. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-011` | SBA × zone/incarnation/LKI boundary | `state-based-actions-combat`, `zone-incarnation` | Lethal creature moves to its owner’s graveyard with a new `GameObjectId`, physical continuity, old-incarnation event/audit input, and exact delta. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-012` | zone × information/identity projection | `zone-incarnation`, observation substrate | Public move and hidden draw produce only authorized observations, knowledge, opaque IDs, event envelopes, and sequence values. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-013` | cleanup × marked damage | `cleanup-reset`, `damage-and-life` | Marked damage persists through combat/end step and is removed exactly at ordinary cleanup. | medium | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-014` | combat × checkpoint/fork/replay | all reachable combat nodes, parity substrates | Nonempty combat history restores, forks, and replays with equivalent state, digest, events, delta, next Decision, products, and status. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-015` | unsupported adjacent semantics × fail-closed boundary | all selected nodes | First/double strike, multiple blockers, restrictions, stack objects, effects, empty-library draw, unsupported keywords, and hidden alternatives reject before mutation. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |
| `M3-ENTRY-016` | accepted/rejected cross-cutting product | all selected nodes, T0 substrate | Accepted state/event/delta/decision/status/projection products agree; rejected work preserves the complete fingerprint and emits no false support claim. | high | `REVIEWED_OBLIGATION / SATISFIED_EVIDENCE = NO` |

### 9.1 Risk-scaled evidence classes

| Risk | Required evidence classes for applicable obligations |
| --- | --- |
| low | Isolated exact case; relevant pairwise case; rejection/nonmutation where the capability exposes rejection; ordinary deterministic replay evidence. |
| medium | Isolated and pairwise cases; selected ordering or three-way case; decision and information evidence where applicable; bounded/property evidence when it is the simplest independent proof. |
| high | Explicit cross-domain pipeline; ordering-sensitive cases; decision soundness and completeness; information noninterference; checkpoint/restore/fork/replay parity; bounded exploration, deterministic campaign, or targeted mutant when the reviewed obligation needs it. |

Line coverage is not semantic evidence. A surviving targeted mutant is an
`EVIDENCE_GAP_CANDIDATE`, not an automatic production bug.

## 10. Decision and information obligations

### 10.1 Decision soundness and completeness

The selected decision surfaces require both properties:

```text
SOUNDNESS = every offered complete choice is legal in the declared scope
COMPLETENESS = every legal complete choice in the declared scope is representable
```

This applies to attacker subsets and blocker assignments. It also applies to
every explicit priority pass and every typed blocker-continuation stage. The
driver never fills a missing choice with a first, default, random, or implicit
pass value.

The blocker continuation is an internal representation of one atomic
declare-blockers action. No spell, ability, trigger, SBA, priority window, or
environment commit occurs between its stages. Rejection at any stage restores
the complete pre-submission fingerprint.

### 10.2 Information-safety matrix

| Surface | Trusted expectation | Player-safe expectation | Paired hidden-state requirement | Redaction boundary |
| --- | --- | --- | --- | --- |
| Turn/priority | Full temporal state and trusted bindings remain available to the kernel. | Only authorized phase/step, priority availability, and public decision values. | Opponent hidden identities, seeds, global allocators, and hidden cursors do not alter bytes. | `mtgml-observation` and endpoint projection; no trusted IDs or diagnostics. |
| Draw | Kernel sees exact library top and physical/game-object identities. | Drawing player may retain authorized knowledge; opponent sees no card identity. | Different hidden card identities produce equal opponent-safe bytes when all authorized public facts match. | Perspective-local knowledge/opaque identity contracts. |
| Attack/block decisions | Kernel sees authoritative bindings and complete legal set. | Actor sees only its authorized opaque/public candidates and request-local IDs. | Trusted-ID renaming and insertion order do not alter visible order or IDs. | `mtgml-decision` binding boundary and `mtgml-observation` projection. |
| Damage/SBA/zone | Kernel sees source, affected object, old incarnation, and full event/delta. | Public consequences are visible through authorized observed events; trusted IDs and hidden diagnostics are absent. | Unauthorized hidden state does not alter player bytes, event sequence, or error class. | Read-only projection before environment commit. |

## 11. Scenario construction policy

The foundation preserves:

```text
structurally valid != case-valid != historically reachable
```

Structural setup is permitted for irrelevant background facts. A fact that can
change legality, object identity, visibility, knowledge, ordering, RNG,
history-sensitive behavior, LKI, or expected output must be established by a
real authoritative transition or by an explicitly authoritative validated
initial state. No helper may silently claim that an object was cast, attacked,
revealed, known, randomized, or damaged in an earlier turn.

The bounded integration case therefore uses an authoritative prelude for the
control-age and library-order facts that affect legality or draw identity. If
the real kernel cannot establish that prelude, the case fails closed and does
not inject the missing history.

## 12. Explicit exclusions and fail-closed boundary

Every excluded family has an owner and a precise boundary. An excluded family
is not reachable as silently approximated behavior inside the selected scope.

| Category | Status | Owning workflow | Entry boundary | Required behavior if encountered |
| --- | --- | --- | --- | --- |
| Full spell casting | `DEFERRED_TO_LATER_M3` | rules capability workflow | No cast action, casting legality, or casting continuation. | Reject before mutation with trusted unsupported/invariant failure. |
| Full stack | `DEFERRED_TO_LATER_M3` | rules capability workflow | Stack is empty for all selected priority paths. | An offered or supplied stack object fails closed. |
| Mana system | `DEFERRED_TO_LATER_M3` | rules capability workflow | No mana pool or mana production semantics. | Reject any path requiring mana. |
| General cost/payment | `DEFERRED_TO_LATER_M3` | rules capability workflow | No cost, payment, alternative cost, or additional cost. | Reject before a payment decision can be offered. |
| General targeting | `DEFERRED_TO_LATER_M3` | rules capability workflow | Only fixed two-player attack target and combat participant bindings. | Reject any target-selection requirement. |
| General resolution | `DEFERRED_TO_LATER_M3` | rules capability workflow | No spell/ability resolution. | Reject any resolution continuation. |
| Triggered abilities | `DEFERRED_TO_LATER_M3` | rules capability workflow | No trigger detection, ordering, stack placement, or resolution. | Reject any nonempty/waiting trigger path. |
| Replacement/prevention | `DEFERRED_TO_LATER_M3` | rules capability workflow | Combat damage is unmodified; no replacement/prevention effects. | Reject before applying the event. |
| Continuous-effect layers | `DEFERRED_TO_LATER_M3` | rules capability workflow | Creature power/toughness and legality use the declared simple state only. | Reject a characteristic dependency or layer requirement. |
| Copy effects | `DEFERRED_TO_LATER_M3` | rules capability workflow | No copied values or copy effects. | Reject before object characteristics are inferred. |
| Attachments | `DEFERRED_TO_LATER_M3` | rules capability workflow | No attachment relationships. | Reject Aura, Equipment, Fortification, or attachment legality. |
| Aura | `DEFERRED_TO_LATER_M3` | rules capability workflow | No Aura objects or Aura SBAs. | Reject before any Aura state is accepted. |
| Equipment | `DEFERRED_TO_LATER_M3` | rules capability workflow | No Equipment objects or Equipment SBAs. | Reject before any Equipment state is accepted. |
| Evergreen keywords | `DEFERRED_TO_LATER_M3` | rules capability workflow | No haste, defender, flying/reach, menace, first/double strike, trample, deathtouch, lifelink, infect, wither, or toxic. | Reject a keyword-dependent legality or result. |
| Multiple blockers and damage assignment | `DEFERRED_TO_LATER_M3` | combat capability workflow | At most one blocker per attacker; no damage-assignment choice. | Reject a second blocker or assignment requirement. |
| Empty-library draw | `DEFERRED_TO_LATER_M3` | draw/SBA capability workflow | `rules/draw-card` requires a nonempty library. | Reject before mutating library, status, or replay. |
| First-turn game-start procedure | `OUT_OF_INITIAL_FOUNDATION` | later game-start scope review | The integration starts from a validated turn-2 state. | Reject an unclassified game-start/first-turn-skip context. |
| Extra or skipped turns/phases/steps | `DEFERRED_TO_LATER_M3` | turn capability workflow | Fixed normal turn only. | Reject before advancing. |
| Commander | `M4` | format capability and bundle workflow | `FormatState::None`; no Commander rules or registration. | Reject Commander configuration/semantics. |
| Loops and shortcuts | `LATER_MILESTONE` | explicit loop policy decision | No loop-capable mandatory path. | Apply no heuristic shortcut; fail closed at the unsupported boundary. |
| Multiplayer | `LATER_MILESTONE` | multiplayer format workflow | Exactly two players and one fixed opponent. | Reject a topology other than two-player. |
| Search/determinization | `M5` | ML/search workflow | No full-state search forks or determinization. | No policy access to those products. |
| ML training | `M5` | ML environment workflow | No dataset, reward, training, or trajectory claim. | Do not create training output. |
| Parallel rollout optimization | `ISSUE_176_AFTER_CERTIFIED_BUNDLE` | benchmark/performance workflow | Reference semantic backend only; no optimized backend. | Do not add performance state or alternate semantics. |
| Deck support | `M4` | Card IR/bundle workflow | No deck manifest or format support claim. | Do not load or certify a deck. |
| Card certification | `M4` | capability/bundle certification workflow | No Card IR, card definition, bundle, or certification claim. | Do not register or certify a witness. |

The fail-closed boundary is part of every selected capability’s included scope.
An engine that silently skips, defaults, approximates, or chooses an excluded
meaning has not implemented this foundation.

## 13. M3/S1 and M3/M4 boundaries

M3 owns reusable semantics, capability identity and evidence, semantic
ownership, conformance, reviewed interaction obligations, decision/information
products, exact state/event/delta/parity evidence, and fail-closed unsupported
behavior.

M4 owns reviewed Card IR definitions, content-specific requirements, recursive
capability closure, exact bundles/decks, content evidence, and certification.
The foundation may use synthetic witnesses only. A witness is not a card
definition, deck, bundle, support claim, or certification result.

The Standard Mono-Red/Mono-White research remains a dependency and interaction
stress input. Its `92` advisory capability candidates, `40` potential seams,
and `29` high-priority interaction candidates do not enter this closure and do
not freeze a benchmark or support claim. OD-003 remains open and downstream.

## 14. Bounded M3 integration milestone

The exact integration milestone is a synthetic, two-player, format-neutral
normal-turn path through the real Rust kernel:

```text
turn 2, active player P1, FormatState::None
→ Untap
→ Upkeep
→ Draw one nonempty-library card
→ Precombat Main
→ Beginning of Combat
→ Declare Attackers
→ Declare Blockers
→ Combat Damage
→ End of Combat
→ Postcombat Main
→ End Step
→ Cleanup
→ turn 3, active player P2
→ Untap
→ Upkeep
→ Draw one nonempty-library card
→ next priority window
```

The validated setup has exactly two players, ordinary life totals, nonempty
libraries, no stack objects, no effects, no triggers, no replacements, no
continuous effects, no unsupported keywords, no attack/block restrictions, and
hand sizes that do not require cleanup discard. P1 has two eligible untapped
creatures: a `3/3` attacker `A` and a `2/2` attacker `C`. P2 has one eligible
untapped `2/2` blocker `B`. P1 explicitly attacks with `{A, C}`; P2 explicitly
assigns `B` to `A`. Damage is simultaneous: `A` deals `3` to `B`, `B` deals `2`
to `A`, and `C` deals `2` to P2. The selected SBA fixed point moves lethal `B`
to its owner’s graveyard as a new game-object incarnation, and the marked
damage on surviving `A` remains until cleanup. P2’s life becomes `18`.

Every priority-bearing boundary uses explicit pass responses. The next turn
proves active-player switch and a second draw/projection path. The case
asserts exact state, event order, delta, digest, next Decision, status, all
player products, rejection nonmutation, checkpoint/restore, fork, replay, and
unsupported-boundary behavior.

The integration milestone is a bounded composition proof. It does not claim
arbitrary Magic, card support, deck support, Standard, Commander, or universal
combat semantics.

## 15. Bounded M3 exit criteria

M3 may reach its declared exit only when all of the following are true:

1. T0 has the exact exit evidence in Section 3.3.
2. Every selected capability has current `SPECIFIED`, `IMPLEMENTED`, and `COVERED` evidence for its declared scope.
3. Every dependency is present, version-correct, acyclic, and covered where required.
4. Every capability has one current pinned authority, one semantic owner, explicit exclusions, and a fail-closed boundary.
5. Every applicable obligation in Section 9 has current `SATISFIED_EVIDENCE`.
6. Decision soundness and completeness pass for attacker and blocker choices.
7. Information-safety and paired-state noninterference pass for all high-risk surfaces.
8. Rejected responses preserve the complete semantic/environment/player/replay fingerprint.
9. State/event/delta/after-state parity passes for every selected transition family.
10. Representative checkpoint/restore, fork, and replay parity passes on nonempty selected history.
11. The exact integration milestone in Section 14 passes with the real Rust kernel.
12. Required fast/integration/certification repository gates and exact-head hosted checks pass.
13. Unsupported reachable behavior outside the declared scope fails closed.
14. Independent final M3 review has `0 BLOCKER / 0 MAJOR`.
15. No card/deck/format/certification or benchmark claim exceeds the declared boundary.

The required exit artifact uses these states:

| Capability | SPECIFIED at Entry | IMPLEMENTED at Entry | COVERED at Entry | M3 exit requirement |
| --- | --- | --- | --- | --- |
| All 11 closure capabilities | `YES` | `NO` | `NO` | `YES` for all three states |

At candidate creation:

```text
ENTRY_REVIEWED_INTERACTION_OBLIGATIONS = 16
ENTRY_SATISFIED_INTERACTION_EVIDENCE = 0
M3_INITIAL_SEMANTIC_FOUNDATION = NOT_COMPLETE
M3 = NOT_COMPLETE
M4 = BLOCKED_BY_M3_EXIT
```

`covered`, not `certified`, is the target lifecycle for each selected semantic
capability. Certification remains an exact content/bundle claim under ADR
0022.

## 16. Explicit authorization protocol

### 16.1 Candidate stage

While the entry PR is unmerged:

```text
M3_ENTRY_DECISION = CANDIDATE
INITIAL_FOUNDATION = CANDIDATE
M3_STARTED = NO
M3_AUTHORIZED = NO
AUTHORIZATION_HEAD = NOT_SET
```

The candidate branch head, green CI, Issue #178 checkbox, research report,
proposed ADR, or this document cannot substitute for post-merge authorization.

### 16.2 Post-merge authorization

After the complete entry PR is merged, a separate reviewed invocation must:

1. fetch the current remote `master`;
2. verify that the exact entry PR is contained in it;
3. record the exact merge commit and required hosted checks;
4. independently re-review this artifact, the ADR, the DAG, ownership graph, exclusions, and S1;
5. confirm `BLOCKER = 0` and `MAJOR = 0`;
6. update the relevant final Entry Gate checkboxes in Issue #178;
7. post one explicit authorization comment containing the exact merged `master` SHA.

The authorization comment must contain:

```text
M3_ENTRY_REVIEW = APPROVE
AUTHORIZATION_HEAD = <exact current merged master SHA>

INITIAL_FOUNDATION = FROZEN
CAPABILITY_CLOSURE = FROZEN
DEPENDENCY_DAG = FROZEN
SEMANTIC_OWNERSHIP_GRAPH = FROZEN
T0_SCOPE = FROZEN
S1 = FROZEN

M3_AUTHORIZED = YES
M3_STARTED = NO
AUTHORIZED_NEXT_TASK = M3.T0
```

Only after that comment exists may T0 implementation begin. The first actual
M3 implementation task may then set `M3_STARTED = YES`; authorization and
execution start are distinct states.

### 16.3 Authorization failure

If the post-merge review finds a blocker or major:

```text
M3_ENTRY_REVIEW = FAIL
M3_AUTHORIZED = NO
AUTHORIZATION_HEAD = NOT_SET
```

The reviewed merged head may be recorded separately as `REVIEWED_HEAD` without
turning it into an authorization head.

## 17. Candidate change boundary

This candidate changes no production Rust semantics, production Python
semantics, wire or Decision schemas, observation schemas, replay/checkpoint
codecs, RNG, digests, Card IR, cards, decks, or capability registry support
entries. It adds no public RulesCase protocol, no dynamic format hook, no
benchmark, and no training machinery.

```text
CAPABILITY_REGISTRY_MODIFIED = NO
CAPABILITIES_REGISTERED = 0
PRODUCTION_RUST_CHANGED = NO
PRODUCTION_PYTHON_CHANGED = NO
MAGIC_SEMANTICS_IMPLEMENTED = NO
CARDS_ADDED = 0
DECKS_ADDED = 0
```

The only durable candidate authority artifacts are this operational foundation
record and the companion M3 Entry ADR. Issue #178 remains open as the master
execution tracker. Issue #163 remains open as planning provenance until the
entry decision is merged, independently approved, and explicitly authorized.
