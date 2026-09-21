# M3.S1 `rules/turn-structure@0.1.0` Specification

**Status:** planning specification; ready for independent review
**Lifecycle at this artifact:** `specified`
**Production implementation authorized by this document:** NO
**Date:** 2026-09-21
**Capability:** `rules/turn-structure@0.1.0`

This document is the Phase A output for M3.S1. It defines the bounded semantic
contract that a later implementation plan may implement. It does not implement
Rust behavior, change a capability lifecycle, create a card, or authorize a
production PR.

## 1. Status and exact repository baseline

The actual remote was checked before making repository-specific claims:

```text
REMOTE_REPOSITORY = https://github.com/chrismaghuhn/Manafold
REMOTE_MASTER = 3bb404e750d93b82afd15e1a7e2c0cc6c0e4e635
REMOTE_MASTER_REF = refs/heads/master
REMOTE_MASTER_RESULT = PASS (git ls-remote, then git fetch origin master)
```

The working source tree inspected for this specification was commit
`03e14f3`, an ancestor of the fetched remote merge commit. PR #200 contributes
the V5 merge commit without a source-tree delta relative to that ancestor.
The specification baseline is therefore the exact current remote-master tree
at `3bb404e750d93b82afd15e1a7e2c0cc6c0e4e635`.

Current status at the baseline is:

```text
M3.P0 = COMPLETE / FROZEN
M3.T0 = COMPLETE / FROZEN
V5_EXECUTION_IDENTITY = CLOSED
CHECKPOINT_CURRENT = V5
REPLAY_CURRENT = V5
EXECUTION_IDENTITY = V1
M3.S1 = AUTHORIZED / NOT_STARTED
rules/turn-structure@0.1.0 = specified
implemented = 0
covered = 0
certified = 0
```

V5 is consumed by S1. It is not reopened, migrated, or versioned again by this
specification. `PRODUCTION_CODE_CHANGED = NO` for this planning task.

## 2. Normative authority and hierarchy

The binding order is the repository's accepted normative hierarchy:

1. the pinned Comprehensive Rules authority;
2. accepted ADRs and accepted normative contracts;
3. executable state/event/delta, schema, fixture, and conformance contracts;
4. process and informative documents.

The primary reviewed inputs are:

- `project-sources/01_AGENTS.md` and `AGENTS.md`;
- `project-sources/03_NORMATIVE_HIERARCHY.md`;
- `project-sources/06_ARCHITECTURE.md`, `07_DOMAIN_MODEL.md`,
  `08_EXECUTION_MODEL.md`, `09_RULES_SEMANTICS.md`, `10_DECISION_PROTOCOL.md`,
  `11_INFORMATION_MODEL.md`, `12_REPLAY_AND_DETERMINISM.md`,
  `13_STATE_HASHING.md`, `16_TESTING_AND_CONFORMANCE.md`;
- `project-sources/20_CAPABILITY_MODEL.md`, `21_CERTIFICATION.md`,
  `22_ADDING_RULES_AND_MECHANICS.md`, `23_AUTHORITY_POLICY.md`,
  `24_SEMANTIC_CONTRACT.md`, `26_WIRE_CONTRACT.md`,
  `27_ENGINE_STATE_CLOSURE.md`, `28_ACCEPTANCE_GATES.md`,
  `30_API_LIFECYCLE.md`, `31_SCHEMA_EVOLUTION.md`, and `34_ADR_BUNDLE.md`;
- the current `docs/` versions of those contracts;
- accepted ADR 0054, `docs/adr/0054-m3-pre-t0-hardening.md`;
- accepted ADR 0055, `docs/adr/0055-v5-execution-identity.md`;
- `docs/rules/M3_INITIAL_SEMANTIC_FOUNDATION_V2.md`;
- current Issue #178 tracker state and its explicit S1 authorization comment;
- the current capability registry and semantic-contract catalog.

The pinned rules authority is verified in Foundation V2 and ADR 0051:

```text
wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f
```

The S1 authority references the closed temporal and ordinary-untap scope in
Comprehensive Rules `500.1`, `500.3`, `500.12`, `501.1`, `502.2–502.4`,
`505.1–505.2`, `512.1`, `513.1`, and `514.3`, as enumerated by Foundation V2.
The rules snapshot is authority metadata; it is not evidence that the
capability is implemented or covered.

Foundation V2 and ADR 0054 remain authoritative for S1 semantic scope. Their
references to a coordinated V4 state/persistence cut are historical planning
provenance superseded for current persistence by ADR 0055 and the merged V5
implementation. This specification preserves that history and uses the
current V5 checkpoint/replay surfaces.

## 3. Capability identity and lifecycle state

```text
S1_KEY = rules/turn-structure
S1_VERSION = 0.1.0
S1_ID = rules/turn-structure@0.1.0
S1_PRIMARY_SEMANTIC_OWNER = turn
S1_CAPABILITY_DEPENDENCIES = NONE
S1_AUTHORITY = wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f
S1_AUTHORIZATION_HEAD = 587016574e4e8f9f797a713877f8caf1c5143cfb
S1_LIFECYCLE = specified
```

The current registry entry remains `specified` with empty implementation and
conformance evidence. This specification defines the evidence required for
later `implemented` and `covered` transitions; it does not perform either
transition. `certified` is outside S1.

The first Magic semantic contract may reference S1's key and version in its
content-derived capability closure. That closure identifies semantic meaning;
it does not promote registry lifecycle or create a card/deck/format claim.

## 4. Goals

S1 has these bounded goals:

- define the closed normal two-player temporal order;
- own `turn_number`, `active_player`, and the existing `TurnPosition`;
- execute ordinary untap as deterministic rules-owned forced progress;
- execute a quiescent ordinary-cleanup-to-next-turn temporal switch;
- preserve exact state/event/delta, checkpoint, fork, replay, observation, and
  deterministic-rerun contracts;
- make every accepted S1 mutation auditable and every unsupported boundary
  fail closed;
- make S1 executable only through `ExecutionProgramV1::MagicRules` bound to
  the exact S1 semantic contract;
- keep ordinary untap decisionless and keep Python rules-free.

S1 is not a playable-game milestone. It is the first bounded real Magic
semantic contract, not a card, deck, format, or priority milestone.

## 5. Explicit non-goals

S1 does not implement or authorize:

- game start, mulligans, opening hands, or the first-turn draw skip;
- priority, `pass_priority`, priority holders, consecutive passes, priority
  transfer, two-pass advancement, stack-empty priority, or SBA-before-priority;
- draw execution, library-to-hand movement, empty-library draw handling, or
  draw replacement;
- cleanup damage removal, cleanup discard, duration expiry, cleanup trigger
  exceptions, or cleanup-reset semantics;
- attacker declarations, blocker declarations, combat state, combat skipping,
  or combat damage;
- spells, activated abilities, mana abilities, stack objects, triggered
  abilities, state-based actions, replacement, or prevention;
- extra turns, skipped turns, extra phases, skipped phases, extra steps,
  skipped steps, additional combat phases, or any dynamic turn registry;
- phasing or day/night;
- “doesn't untap” effects, untap choices, untap restrictions, or effect-created
  untap modifiers;
- zone transitions, zone-location mutation, object reincarnation, or physical
  card identity mutation;
- Card IR, cards, decks, formats, Commander, or multiplayer beyond two
  players;
- performance optimization, parallel simulation, MCTS/search, or ML training.

If an included temporal transition requires one of these semantics, S1 stops
at the unsupported boundary and fails closed. It never guesses, defaults,
passes, skips, repairs, or fabricates a player response.

## 6. Existing authoritative substrate reused

S1 reuses the following existing owners; it does not create duplicate state or
execution authorities:

| Existing owner | S1 use |
| --- | --- |
| `crates/mtgml-state/src/core.rs` | `CoreRulesState`, `TurnPosition`, closed step enums, `PriorityState`, `active_player`, and `turn_number`. |
| `crates/mtgml-state/src/engine.rs` | closed `EngineState`, `EngineStateParts`, and V4 full-state identity. |
| `crates/mtgml-state/src/zones.rs` | authoritative objects, locations, controller, tapped state, and physical/game-object identity families. |
| `crates/mtgml-state/src/validation.rs` | generic cross-component state validation before and after every candidate product. |
| `crates/mtgml-state/src/delta.rs` | complete replacement delta plus typed semantic audit. |
| `crates/mtgml-rules/src/program_kernel.rs` | program-owned kernel construction, response entry point, and forced-progress entry point. |
| `crates/mtgml-rules/src/product.rs` | atomic accepted-product construction and transition-contract validation. |
| `crates/mtgml-rules/src/semantic_cursor.rs` | sequential event cursor and final-state parity. |
| `crates/mtgml-environment/src/synthetic/commit.rs` | existing forced-progress and accepted-transaction commit discipline to be generalized/reused, not duplicated. |
| `crates/mtgml-conformance/src/facade.rs` | `ForcedProgress` conformance path with no response field. |
| `crates/mtgml-observation/src/m3.rs` | existing closed temporal observation payload and explicit priority value. |
| `crates/mtgml-environment/src/semantic_catalog.rs` | V5 immutable catalog, exact admission order, and runtime support predicate. |
| V5 checkpoint/replay types | current `EnvironmentCheckpointV5`, `ReplayManifestV5`, `ReplayStepV5`, and `ExecutionIdentityV1`; no V6 surface. |

`EngineState` remains the only semantic input. Caches are forbidden from
affecting legality, transitions, event order, digest, replay, checkpoint, or
projection.

## 7. Supported-state predicate

The S1/MagicRules support predicate is a program-aware admission predicate
above generic `validate_engine_state`. It is evaluated before execution and
before a checkpoint can be restored into the S1 backend.

The first and non-negotiable condition is executable and testable:

```text
players.len() == 2
```

The predicate then requires all of the following:

1. the two player IDs are distinct and both are declared in `core.players`;
2. `active_player` is one of those IDs; the other player is derived as the
   unique set difference, never selected by a default or map-order accident;
3. `turn_number >= 1`; `u64::MAX` is valid only until an increment would be
   required, at which point progression rejects with overflow and mutates
   nothing;
4. `position` is the existing closed `TurnPosition`; no string or dynamic
   position is admitted;
5. S1 entry has no pending authoritative decision, no continuation, no stack
   object, and no effect, waiting-trigger, or delayed-effect state. A state
   that contains unsupported effect/trigger machinery is rejected before
   untap derivation;
6. `format == FormatState::None`, and no combat state is active for an S1
   executable operation;
7. the current `PriorityState` is `None` for the S1 no-priority path. A held
   priority state is not interpreted, advanced, or passed by S1;
8. every relevant live object/location and every source fact passes existing
   structural validation; no unsupported modifier can be represented by an
   unvalidated or hidden cache;
9. all generic state/profile conditions pass. Perspective-local opaque-ID
   resolution is deliberately not part of Rules Admission. Rules semantics
   must not depend on whether a player projector can currently materialize an
   `OpaqueObjectId`.

The predicate does not require a particular number of objects. It admits
already-untapped objects, objects controlled by either player, and empty
affected sets, subject to the closed profile above. It does not persist
`untap_eligible`, `can_untap`, effective characteristics, or any other answer
cache.

The executable S1 operation set is intentionally narrower than the closed
temporal vocabulary:

```text
Beginning(Untap)       -> ordinary untap, then the Upkeep boundary
Ending(Cleanup)        -> quiescent cleanup boundary, then next player's Untap boundary
all other positions    -> stop/fail closed at the named downstream capability
```

The closed positions remain valid state vocabulary and observation vocabulary;
their presence does not authorize downstream semantics.

## 8. Exact temporal state machine

S1 owns this canonical normal-turn skeleton:

```text
Beginning(Untap)
  -> Beginning(Upkeep)
  -> Beginning(Draw)
  -> PrecombatMain
  -> Combat(BeginningOfCombat)
  -> Combat(DeclareAttackers)
  -> Combat(DeclareBlockers)
  -> Combat(CombatDamage)
  -> Combat(EndOfCombat)
  -> PostcombatMain
  -> Ending(EndStep)
  -> Ending(Cleanup)
  -> next turn: Beginning(Untap), active player switched
```

`TurnPosition::canonical_rank()` is the existing structural ordering witness.
S1 adds one reviewed successor relation over the existing closed enum; it does
not add a second cursor.

There are two deliberately distinct meanings:

- The **temporal successor relation** owns that `Upkeep` is followed by
  `Draw`, and owns every other row in the skeleton.
- The **executable forced-progress path** may commit only work whose rules
  semantics are in S1. It may enter the next temporal boundary and stop at a
  boundary whose next mandatory work requires an unsupported capability. It
  must not cross a priority, draw, combat, or cleanup-reset boundary by
  silently skipping that work.

Therefore, the positive `Upkeep -> Draw` evidence is a successor-relation
assertion plus an unsupported-boundary assertion; it is not permission to skip
priority or draw. A runtime attempt to execute from `Upkeep` fails closed at
`rules/basic-priority@0.1.0`. Similar downstream stops are explicit:

| Current position | First unsupported mandatory meaning |
| --- | --- |
| `Beginning(Upkeep)` | `rules/basic-priority@0.1.0` |
| `Beginning(Draw)` | `rules/draw-card@0.1.0` |
| `PrecombatMain` | `rules/basic-priority@0.1.0` |
| any combat position | the named combat/priority capability for that boundary |
| `PostcombatMain` | `rules/basic-priority@0.1.0` |
| `Ending(EndStep)` | `rules/basic-priority@0.1.0` and unsupported trigger work |
| `Ending(Cleanup)` with required reset/discard/exception work | `rules/cleanup-reset@0.1.0` or the exact unsupported cleanup meaning |

The failure is not a terminal game result, a draw, a truncation heuristic, an
implicit pass, or an accepted no-op. The supported work before the stop is
committed only when the existing forced-progress transaction defines that
boundary as its deterministic stop; the unsupported attempt itself never
mutates state, RNG, IDs, knowledge, events, replay, counters, or episode
status.

Game start and first-turn draw behavior are not represented by this state
machine. S1 cases begin from a complete validated state supplied by trusted
setup.

## 9. Ordinary untap semantics

Ordinary untap is S1-owned deterministic forced progress:

```text
enter Beginning(Untap)
  -> validate the S1 supported profile
  -> derive the complete affected set from authoritative source facts
  -> canonicalize the set by ascending GameObjectId
  -> clear tapped=true to false for every member simultaneously
  -> emit exact typed semantic evidence
  -> advance to Beginning(Upkeep)
  -> create no Decision
```

For the bounded simple profile, an object is eligible exactly when all of the
following are true in the before-state:

```text
live object
AND its authoritative location is Battlefield
AND object.controller == active_player
AND object.tapped == true
AND the validated state contains no represented unsupported untap modifier
```

Objects controlled by the nonactive player are not eligible under the pinned
ordinary-untap rule and remain unchanged. Already-untapped objects remain
unchanged. No player chooses an object and no optional untap assignment exists
in S1.

The affected set is computed completely before any field is changed. No
intermediate state is observable, and no affected object is processed through a
second scheduler. An empty affected set is still represented by the typed
`UntapCompleted` semantic event so the boundary and complete derived set are
auditable; it produces no object-tapped observation occurrences.

S1's only permitted zone/object mutation is:

```text
GameObject.tapped: true -> false
```

The untap path must prove equality of every other zone/object/identity field.
It never changes:

```text
S1_ZONE_TRANSITIONS_AUTHORIZED = NO
S1_ZONE_LOCATION_MUTATION_AUTHORIZED = NO
S1_OBJECT_INCARNATION_CHANGE_AUTHORIZED = NO
S1_PHYSICAL_CARD_IDENTITY_CHANGE_AUTHORIZED = NO
```

## 10. Forced-progress semantics

Ordinary untap and the quiescent cleanup turn switch use the existing
rules-owned forced-progress architecture:

```text
rules-owned forced progress
!= player action
!= DecisionResponse
!= implicit pass
!= replay step invented for no-choice work
!= second scheduler
!= second rules engine
```

There is one authoritative primitive. The program-owned MagicRules kernel,
the environment's existing `execute_forced_progress` commit path, and the T0
`ForcedProgress` case all call that same primitive. T0 supplies inputs and
assertions; it does not derive turn legality or untap eligibility.

For MagicRules S1:

- `apply(response)` cannot complete S1 work. A fabricated response is rejected
  or fails closed without mutation; it is never treated as pass or untap
  choice.
- `advance_forced_progress()` accepts no response and performs only the
  currently supported deterministic boundary work.
- no S1 transition creates, clears, or replaces a player decision;
  `Decision count = 0` and `fabricated response = 0` for ordinary untap;
- forced progress does not increment the environment's submitted-decision or
  accepted-player-transition counters and does not append a `ReplayStepV5`;
  emitted rule events and the resulting checkpoint identity still advance
  through the existing trusted commit contract;
- an unsupported mandatory downstream operation returns a typed fail-closed
  error and leaves the borrowed input/backend product unchanged.

## 11. Cleanup boundary and next-turn transition

S1 owns the temporal transition after an **ordinary quiescent Cleanup**:

```text
active_player = the unique other declared player
turn_number = turn_number.checked_add(1)
position = Beginning(Untap)
priority = None
```

There is no wraparound. `turn_number == u64::MAX` rejects before any mutation
with a typed overflow error. A cleanup state that requires damage removal,
discard, duration expiry, a cleanup trigger exception, or another excluded
meaning fails closed before the active-player or turn-number change. A clean
bounded state may therefore prove `Cleanup -> next player's Untap` without
claiming the cleanup-reset capability.

The next player's untap is a separate S1 ordinary-untap boundary operation;
the cleanup transition does not silently perform it in the same product. This
keeps the boundary identity and the affected set auditable.

## 12. State mutation ownership

The following ownership is binding:

| State/value | Authority | S1 mutation |
| --- | --- | --- |
| `core.position` | `mtgml-state::CoreRulesState` | S1 temporal transition only |
| `core.active_player` | `mtgml-state::CoreRulesState` | S1 cleanup turn switch only |
| `core.turn_number` | `mtgml-state::CoreRulesState` | S1 cleanup turn switch only, checked arithmetic |
| `core.priority` | existing state representation; priority capability owns behavior | S1 preserves `None`; never creates or transfers priority |
| `zones.objects[*].tapped` | existing authoritative `GameObject` | S1 ordinary untap only, active-player controlled battlefield set |
| object/location/physical identity | `mtgml-state::ZoneState` and existing identity model | never changed by S1 |
| RNG, allocators, knowledge, opaque mappings | existing closed `EngineState` owners | no S1 semantic mutation except event allocator/revision and the existing visible occurrence cursor when public tap events are projected |
| pending decision/continuations/effects/triggers | existing `ExecutionState` | no S1 creation or completion |
| format state | existing `FormatState` | must remain `None` |

No controller, backend field, projector cache, callback, thread, or test helper
may retain a second temporal cursor or untap eligibility cache.

## 13. Event and `StateDelta` contract

The existing typed event/audit architecture is extended only with S1-required
families. No generic event bus and no speculative Priority, Draw, Combat, or
future event types are added.

The minimum typed authoritative vocabulary is:

```text
AuthoritativeRuleEventKind::TurnPositionChanged {
    from: TurnPosition,
    to: TurnPosition,
}

AuthoritativeRuleEventKind::UntapCompleted {
    affected_objects: Vec<GameObjectId>,
}

AuthoritativeRuleEventKind::ActivePlayerChanged {
    from: PlayerId,
    to: PlayerId,
}

AuthoritativeRuleEventKind::TurnNumberChanged {
    from: u64,
    to: u64,
}
```

Each family has the same named `SemanticDeltaOperation` representation. The
existing `PerspectiveOccurrence` family remains the one authority for
perspective-visible sequence/opaque mapping lifecycle; its observation policy
may gain the narrowly required existing-schema `ObjectTapped` projection
meaning. It does not duplicate the `UntapCompleted` state mutation.

Canonical event order is part of the S1 contract:

1. `UntapCompleted`;
2. one `PerspectiveOccurrence` per affected public object and perspective,
   ordered by perspective ID then GameObjectId, when the existing mapping can
   safely project the public tapped consequence;
3. `TurnPositionChanged` for Untap -> Upkeep;
4. for Cleanup -> next Untap: `TurnNumberChanged`,
   `ActivePlayerChanged`, then `TurnPositionChanged`.

All `RuleEventId` values are contiguous, all events carry the committed after
revision, and `StateDelta.audit` is exactly the event-to-audit mapping in this
order. The complete replacement in `StateDelta` remains the reconstruction
authority; the audit is the semantic explanation and cursor proof, not a
second state patch.

The sequential semantic cursor must add and validate:

- current position and its legal successor;
- current active player and the exact two-player other-player relation;
- current turn number and checked increment;
- every `UntapCompleted` affected object, including strict canonical order,
  tapped-before, battlefield location, active-player control, and tapped-after
  equality;
- final parity for all four fields and all object snapshots.

An event with a missing eligible object, a noneligible object, duplicate or
noncanonical affected IDs, a wrong position edge, wrong player switch, or
overflow is a transition-contract failure. It never becomes a legal result.

## 14. Observation and information-safety contract

No new player-facing schema is required. S1 reuses:

```text
ObservationEnvelopeV1
ObservationDigestV1
payload_codec = synthetic-m3-observation.v1
SyntheticM3Observation
```

The existing payload exposes only the already-authorized bounded fields:

```text
active_player
turn_number
turn_position
priority
```

`priority = none` is emitted at S1-authorized no-priority boundaries. S1 does
not expose a priority holder, pass count, execution identity, semantic
contract ID, rules snapshot, checkpoint digest, trusted ID, physical card ID,
hidden zone identity, seed, or RNG cursor.

The existing `ObservedEventKindV2::ObjectTapped` wire meaning is sufficient to
show public tapped-state consequences. The S1 projection path may emit it for
each affected object through the existing perspective-local lifecycle and
opaque-ID mapping. If an authorized public object cannot be resolved to an
opaque ID, projection fails closed; it never substitutes `GameObjectId`.
No new public event variant or observation payload field is introduced.

`PlayerObservation`, `PlayerInformationState`, and `EngineState` remain
distinct. Observation and information state are pure projections of explicit
state and retained knowledge. Paired-state tests must remain equal for
unauthorized hidden IDs, RNG, allocator history, checkpoints, execution
identity, and catalog contents.

The ownership boundary is explicit:

```text
rules/profile admission
    != perspective/opaque mapping validation

rules kernel
    -> produces a semantically valid candidate product

environment pre-commit projection
    -> resolves authorized opaque mappings
    -> failure (for example AuthorizedObjectUnresolvable)
       rejects the complete candidate atomically
```

`lifecycle_projection::project_occurrence_envelopes()` is the existing owner
of that pre-commit check. S1 does not move it into `mtgml-rules` and does not
make untap legality depend on projection success.

## 15. V5 ExecutionIdentity and semantic-contract integration

S1 is the first production `MagicRules` contract. The exact content-addressed
contract is:

```text
program_kind = magic_rules
rules_authority = comprehensive_rules {
    snapshot_id = wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f
}
capability_closure = [
    { key = rules/turn-structure, version = 0.1.0 }
]
format_contract_id = null
content_contract_id = null
```

The closure is exact: it contains S1 and no other Foundation capability. In
particular it does not authorize `basic-priority`, `draw-card`,
`cleanup-reset`, `combat-phase`, `declare-attackers`, `declare-blockers`,
`combat-damage`, `damage-and-life`, `state-based-actions-combat`, or
`zone-incarnation`.

The current V5 source chain is preserved:

```text
contracts/catalog/semantic-contracts.v1.json
  -> scripts/generate_semantic_contract_catalog.py
  -> crates/mtgml-environment/src/semantic_catalog_generated.rs
  -> RuntimeSemanticCatalog
  -> V5 admission
  -> program-owned MagicRules kernel
```

The production catalog becomes a closed two-entry catalog: the existing
synthetic legacy entry and the exact S1 comprehensive-rules entry. The
generator, generated Rust, KAT, and catalog tests must all agree. Runtime
support is exact `(program_kind, semantic_contract_id)` admission, not
“MagicRules means every comprehensive contract.”

V5 restore admission remains in its current order: checkpoint validation,
identity/digest recomputation, catalog resolution, child-manifest checks,
program/authority pairing, exact runtime support, then program-aware state
admission. A mismatched or unknown S1 contract fails closed before backend
mutation.

`ExecutionIdentityV1`, `EnvironmentCheckpointV5`, `CheckpointDigestV5`,
`ReplayManifestV5`, and `ReplayStepV5` remain current. The V5 replay manifest's
existing provenance fields are populated with deterministic S1 scenario
metadata as required by its current schema; this is not a deck, Card IR, or
card-support claim. No V6 checkpoint/replay identity is introduced.

## 16. Fail-closed unsupported boundaries

The trusted failure taxonomy must distinguish at least:

- unsupported player count;
- unsupported S1 state/profile or invalid temporal admission;
- unsupported priority-bearing downstream boundary;
- unsupported draw boundary;
- unsupported cleanup-reset work;
- unsupported combat boundary;
- unsupported fabricated/player response to a no-choice S1 path;
- exact two-player other-player derivation failure;
- turn-number overflow;
- untap affected-set or narrow-mutation invariant failure;
- semantic-contract unknown, digest mismatch, authority mismatch, or runtime
  unsupported admission;
- generic state, transition, projection, digest, or replay invariant failure.

These are typed trusted diagnostics. Player endpoints expose only the existing
closed service/rejection surface. Unsupported S1 work is never converted to a
terminal win/loss/draw, technical truncation, implicit pass, random action,
default action, or “first candidate.”

## 17. Determinism and atomicity requirements

For accepted S1 work:

- the same complete state and execution identity produce the same affected set,
  event order, delta, digest, observations, and status;
- `BTreeMap`/explicit numeric ordering, not hash iteration or allocation
  history, determines every exposed order;
- no RNG stream is read or advanced;
- only revision and the rule-event allocator advance as required by the
  existing accepted-product contract, plus existing perspective visible
  sequence state when an observed tap event is emitted;
- all state, event, audit, delta, next-decision, status, and projections are
  validated before commit.

For rejected or unsupported work, the complete nonmutation fingerprint is
unchanged:

```text
state, RNG, trusted IDs, perspective IDs, knowledge, history, events,
replay, checkpoint identity, environment counters, episode status, and
player-visible bytes
```

The only exception is a sanitized error returned outside the semantic state.

## 18. Replay/checkpoint/fork consequences

S1 changes authoritative state meaning only through the already-current V4
`EngineState`/`FullStateDigestV4` surfaces introduced by P0. V5 binds the
execution program and exact S1 contract without changing `EngineState` or
introducing a new persistence version.

Required consequences:

- an S1 checkpoint stores the same complete `EnvironmentCheckpointV5` plus
  `ExecutionIdentityV1` already required by V5;
- restore rejects unknown, mismatched, or unsupported S1 identity before
  backend mutation;
- fork begins from byte/equality-identical V5 checkpoint identity and produces
  identical forced progress;
- ordinary forced progress does not invent a replay response or replay step;
  the existing V5 recorder baseline/empty-segment behavior is reused;
- an empty S1 replay from an exact checkpoint remains valid and reprojects to
  the same final identity; any player-response replay remains impossible until
  a downstream decision capability is present;
- state digest, checkpoint digest, replay identity, and observation digest
  parity are checked from the committed products, never from hidden runtime
  state.

If the current V5 envelope cannot represent an S1 scenario without changing
the meaning of a V5 field, implementation must stop and report
`SCOPE_DEPENDENCY_DISCOVERED`; it must not create V6 or silently reinterpret
V5. The accepted design assumes the existing V5 provenance fields can carry
deterministic non-card S1 scenario identity without a support claim.

## 19. Conformance matrix

Every case is an exact trusted-kernel/conformance case, not a shape-only unit
test. Expected state, event list, complete delta, digest, status, observations,
information states, and rejection fingerprint are independently authored.

### Positive temporal cases

| Case | Required assertion |
| --- | --- |
| `s1.temporal.successor-table` | Exact closed successor relation for every row, including `Untap -> Upkeep`, `Upkeep -> Draw`, normal phase/step order, and `Cleanup -> next Untap`. |
| `s1.untap.to-upkeep` | Accepted forced progress from Untap; exact untap event/affected set, position change, no decision, and exact products. |
| `s1.cleanup.next-turn` | Accepted quiescent Cleanup transition; exact active-player switch, checked turn increment, `Cleanup -> next player's Untap`, and no decision. |
| `s1.deterministic-rerun` | Two identical complete inputs produce byte/equality-identical products. |

### Ordinary untap cases

| Case | Required assertion |
| --- | --- |
| `s1.untap.one-active-tapped` | One active-player battlefield object is cleared. |
| `s1.untap.multiple-active-tapped` | Complete set is derived before mutation and emitted in canonical order. |
| `s1.untap.already-untapped` | Untapped objects are unchanged; empty affected set is exact. |
| `s1.untap.nonactive-control` | Tapped objects controlled by the other player remain tapped under the pinned ordinary rule. |
| `s1.untap.narrow-mutation` | Only `tapped` changes; no zone, location, object, physical identity, owner, controller, face-down, allocator, RNG, knowledge, or format mutation is possible. |

### Negative/support-boundary cases

| Case | Required assertion |
| --- | --- |
| `s1.admission.not-two-players` | 1 and 3 player valid-shaped states reject before execution; no guessed “other” player. |
| `s1.admission.priority-held` | Held priority fails closed; S1 does not pass or transfer it. |
| `s1.admission.unsupported-profile` | Unsupported execution/modifier profile rejects without mutation. |
| `s1.downstream.upkeep-priority` | Progression stops/fails closed at the unsupported priority boundary; no implicit pass. |
| `s1.downstream.draw` | Draw boundary is unsupported; no library-to-hand movement. |
| `s1.downstream.combat` | Combat boundaries are unsupported; no combat state or declaration is guessed. |
| `s1.cleanup.reset-required` | Required cleanup reset/discard/exception work fails closed; no turn switch is guessed. |
| `s1.turn-number-overflow` | `u64::MAX` cleanup transition rejects atomically with no wraparound. |
| `s1.invalid-temporal-state` | Invalid or unsupported temporal/profile state is not repaired. |
| `s1.fabricated-response` | No-choice S1 work cannot be completed by a response, pass, or default. |

### V5 and information cases

| Case | Required assertion |
| --- | --- |
| `s1.catalog.exact-closure` | Generated S1 semantic ID recomputes from the exact one-capability closure. |
| `s1.catalog.wrong-closure` | A different closure/ID is not admitted as S1. |
| `s1.catalog.program-pairing` | SyntheticRulesCompat cannot run the CR contract and MagicRules cannot run synthetic legacy. |
| `s1.observation.temporal-fields` | Active player, turn number, closed position, and authorized `priority=none` are exact; privileged V5 fields are absent. |
| `s1.observation.public-untap` | Public tapped consequences use existing opaque `ObjectTapped` observation meaning; no trusted ID crosses the boundary. |

### Parity cases

Every representative positive and negative case must include applicable:

```text
checkpoint -> restore parity
fork parity
replay parity (including no fabricated S1 replay step)
FullStateDigestV4 parity
CheckpointDigestV5 parity
observation/information parity
deterministic rerun parity
```

## 20. Interaction obligations

S1 has no capability dependency edges. The reviewed obligations are therefore
boundary obligations, not downstream implementation work:

- the turn owner must not implement priority, draw, cleanup reset, or combat;
- forced progress must stop before unsupported mandatory work;
- untap must not invoke zone-incarnation or card semantics;
- public untap observation must use the existing information/opaque-ID path;
- V5 catalog admission must not use registry lifecycle or mutable runtime
  lookup as semantic authority;
- T0 must drive the real S1 kernel and must not calculate eligibility or turn
  order independently.

No Foundation V2 interaction row involving `basic-priority`, `draw-card`,
`cleanup-reset`, combat, damage, SBA, or zone-incarnation is marked satisfied
by S1. Those capabilities remain specified-only and unauthorized for S1
execution.

### Carried S1 review findings

The three carried findings from Issue #178 are explicitly dispositioned here:

```text
S1_MINOR_1 = CLOSED_FOR_CURRENT_SCOPE / HISTORICAL_V4_WORDING_RETAINED
S1_MINOR_2 = CLOSED_BY_EXECUTABLE_SPEC / EXACT_TWO_PLAYER_PREDICATE_REQUIRED
S1_MINOR_3 = CLOSED_BY_EXECUTABLE_SPEC / TAPPED_ONLY_UNTAP_MUTATION
```

`S1_MINOR_1` does not rewrite Foundation V2 or ADR 0054. Their V4 wording is
preserved as historical provenance; this specification names V5 as current
checkpoint/replay architecture. `S1_MINOR_2` is the explicit executable
`players.len() == 2` admission condition and has positive/negative evidence
in Section 19. `S1_MINOR_3` is the complete affected-set and narrow mutation
proof in Sections 9, 12, 13, and 19; any unrelated zone/object/identity
change is a transition-contract failure.

## 21. Capability lifecycle evidence

The later implementation may advance the registry only with current evidence:

### `specified -> implemented`

Requires:

- a reviewed reference implementation path in Rust;
- exact S1 support predicate and MagicRules/V5 admission path;
- exact ordinary untap and cleanup-turn-switch products;
- typed event/delta/cursor closure;
- no production Python rules logic;
- focused Rust and catalog/generator tests passing;
- no unsupported downstream capability promoted by accident.

### `implemented -> covered`

Requires every applicable case in Section 19, independent expected values,
complete rejected-work nonmutation, exact event/delta/digest/state parity,
information/noninterference evidence, checkpoint/restore/fork/replay parity,
deterministic rerun, and the repository's applicable fast/integration gates.

`certified` remains out of scope. No card, deck, format, Commander, or
playability claim follows from either lifecycle transition.

## 22. Documentation and status impacts

This planning task adds this specification and its derived plan only, plus the
repository workflow's local issue-tracker configuration. It does not rewrite
Foundation V2, ADR 0054, ADR 0055, README, ROADMAP, or the current registry.

The later implementation change must, in one coherent documentation closure:

- register the specification and plan if required by the normative-document
  register;
- update the capability registry implementation/evidence paths only when the
  lifecycle evidence exists;
- update the semantic-contract catalog source and generated artifact together;
- preserve historical V4 wording while making current status explicitly V5;
- update README/ROADMAP/current-status tests only to narrow S1 evidence, never
  to claim broad Magic, cards, decks, formats, or certification;
- record local and hosted gate results separately using `PASS`, `FAIL`,
  `NOT_RUN`, or `BLOCKED`.

## 23. Acceptance criteria

The S1 specification is accepted for planning only when:

- the exact two-player predicate is executable and testable;
- the current `TurnPosition` is the only temporal cursor;
- the exact normal-turn order and executable stop boundaries are explicit;
- ordinary untap derives a complete active-controller affected set from source
  facts and mutates only `tapped` simultaneously;
- Cleanup-to-next-Untap switch has checked overflow and no wraparound;
- no S1 player decision, pass, priority, draw, combat, cleanup reset, zone
  transition, or identity mutation is implied;
- typed event/delta/cursor closure covers position, untap set, active player,
  and turn number;
- existing observation and observed-event schemas are reused without exposing
  privileged identity or V5 metadata;
- exact V5 semantic-contract admission binds only S1 and remains fail-closed;
- every unsupported boundary and every rejection is atomic;
- Section 19 evidence is sufficient to distinguish specified, implemented,
  and covered;
- the independent specification review has zero BLOCKER and zero MAJOR
  findings.

## 24. Explicit scope firewall

```text
S1_ONLY = YES
PRIORITY_IMPLEMENTATION = NO
DRAW_IMPLEMENTATION = NO
CLEANUP_RESET_IMPLEMENTATION = NO
COMBAT_IMPLEMENTATION = NO
ZONE_TRANSITION_IMPLEMENTATION = NO
CARD_IMPLEMENTATION = NO
V5_REDESIGN = NO
CHECKPOINT_V6 = NO
REPLAY_V6 = NO
```

If implementation reveals a genuine V5 structural insufficiency, the work
stops with `SCOPE_DEPENDENCY_DISCOVERED` and reports the exact field and
correctness reason. It does not silently widen S1 or invent a new identity.

## 25. Open questions

None. The frozen scope answers the design choices required for this artifact.
Any implementation-time discovery that would require a downstream capability,
a V5 redesign, a new public schema, a hidden semantic field, or a different
authority is a blocker/scope dependency, not an open design question to be
resolved by guessing.

## Specification quality gate

This review was performed after drafting the complete specification and before
deriving the implementation plan.

```text
SPEC_REVIEW_BASE = 3bb404e750d93b82afd15e1a7e2c0cc6c0e4e635
SPEC_BLOCKER = 0
SPEC_MAJOR = 0
SPEC_MINOR = 0
SPEC_NIT = 0

SCOPE_EXPANSION_FOUND = NO
PRIORITY_LEAK_FOUND = NO
DRAW_LEAK_FOUND = NO
COMBAT_LEAK_FOUND = NO
ZONE_IDENTITY_LEAK_FOUND = NO
PLAYER_INFORMATION_LEAK_FOUND = NO
V5_REDESIGN_FOUND = NO
DUPLICATE_AUTHORITY_FOUND = NO

SPEC_PLAN_GATE = PASS
```

The specification is therefore suitable for Phase B derivation. This does not
mean S1 implementation, coverage, or certification has passed.
