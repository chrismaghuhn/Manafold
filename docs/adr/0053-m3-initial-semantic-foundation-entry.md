# ADR 0053 — M3 Entry Decision: Initial Semantic Foundation

- **Status:** accepted
- **Date:** 2026-09-15
- **Owners:** architecture maintainers; rules maintainers; conformance maintainers
- **Resolves:** the M3 Entry Decision scope gate in Issue [#178](https://github.com/chrismaghuhn/Manafold/issues/178)
- **Supersedes:** Issue [#163](https://github.com/chrismaghuhn/Manafold/issues/163) after the separate post-merge authorization gate
- **Companion authority:** [`M3_INITIAL_SEMANTIC_FOUNDATION_V1.md`](../rules/M3_INITIAL_SEMANTIC_FOUNDATION_V1.md)
- **Implementation evidence:** `NOT_RUN`; this ADR changes no Magic behavior and authorizes no M3 implementation

This is the accepted M3 Entry Decision. PR #182 merged the reviewed entry scope
at `10f0387281529be087bcb658b779c19a70c2bf8c` from head
`c2669d17b5abece5549f0781eea20b73b9776d29`. Acceptance makes the scope durable
on `master`, but a separate exact-`master` authorization remains required and
M3 remains unauthorized.

## Context

The foundation closure and Pre-M3 Remediation Freeze are complete. PR #181
merged the remaining governance decisions at:

```text
50f71adfdcd0d0f6ad50506290617e045baf0c28
```

The repository still has no real Magic capability implementation, no card or
deck support claim, and no M3 authorization. Issue #178 requires an exact
capability closure and a versioned semantic-ownership graph before a maintainer
can make the separate authorization decision.

The preserved Issue #129 research and Standard census are useful design inputs,
but neither is runtime authority. The M3 decision must freeze the smallest
reusable semantic foundation that can be reviewed without turning a research
candidate list into an implementation queue.

## Decision

### Initial Semantic Foundation

The companion artifact freezes five direct roots and an exact 11-node closure:

```text
DIRECT_ROOTS =
  rules/turn-structure@0.1.0
  rules/basic-priority@0.1.0
  rules/draw-card@0.1.0
  rules/cleanup-reset@0.1.0
  rules/combat-damage@0.1.0

RESOLVED_CAPABILITY_COUNT = 11
```

The closure contains the temporal/action skeleton, explicit empty-stack
priority under an authoritative pass-only precondition, ordinary draw and
cleanup consequences, bounded combat declarations, simultaneous combat damage,
basic life/marked-damage results, the selected SBA fixed point, and the
selected zone/incarnation consequences. The companion
artifact is the sole detailed identity, dependency, ownership, interaction,
exclusion, and exit record; this ADR does not duplicate its tables.

Each capability is versioned `0.1.0`, has one primary semantic owner, has
explicit exclusions, and targets `covered` at the M3 bounded exit. Capability
identity remains separate from runtime dispatch and from the Capability Registry
V1. The registry remains empty.

### M3.T0

T0 is a thin internal `mtgml-conformance` facade over the real authoritative
Rust execution path. It supplies exact setup, decisions, accepted/rejected
products, events, deltas, state/digest, status, player products, nonmutation,
parity, first-divergence assertions, and trusted reproducible failure packets.
It does not calculate Magic legality,
does not silently complete choices, does not create a public RulesCase schema,
and introduces zero Magic capabilities or support claims.

### S1

The first real Magic capability is:

```text
S1_ID = rules/turn-structure
S1_VERSION = 0.1.0
S1_OWNER = turn
S1_AUTHORITY = wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f
S1_SCOPE = two-player, format-neutral temporal structure with deterministic
           ordinary untap of all untap-eligible permanents, fixed normal
           phase/step order, no-priority classification, and active-player
           switch after cleanup
S1_DECISION_SURFACE = none for ordinary untap; forced progress clears all
                      untap-eligible permanents simultaneously
```

S1 is selected from the zero-depth dependency frontier because it is the
smallest genuine Magic semantic boundary that directly establishes the
temporal context required by every later selected action. Zone incarnation and
basic damage/life are also frontier leaves, but they carry higher information
and interaction risk and do not establish the temporal/action skeleton.
Priority and combat are deliberately downstream of the selected temporal
context.

The companion artifact also names the internal synthetic rules-input profile
needed by the bounded combat cases without adding a Card IR, public schema, or
second state authority. It binds the current state facts for creature type,
power/toughness, marked damage, control age, tapped state, library order,
inert card/action surface, and execution-context exclusions. Priority is
offered only after the rules-owned `PASS_ONLY_PRIORITY_PRECONDITION` proves
that no non-pass action exists; an unproven action surface fails closed before
Decision creation. The initial conformance case admits at most eight relevant
attackers and at most one relevant eligible blocker; more than one eligible
blocker fails closed before a blocking Decision is generated.

### Format and M3/M4 boundary

Initial M3 is format-neutral with `FormatState::None`. Commander, multiplayer,
Card IR, concrete cards, decks, bundles, benchmark workloads, and certification
remain outside this decision. M3 owns reusable semantic capabilities and their
evidence; M4 owns content integration and certification. A Semantic Witness is
not a supported card.

### Authority and fail-closed behavior

Every selected node cites the exact Comprehensive Rules snapshot bound by ADR
0051. Unsupported reachable behavior—such as casting, stack objects, costs,
replacement/prevention, triggers, continuous effects, first/double strike,
multiple blockers, empty-library draw, Commander, or multiplayer—must fail
closed before an incorrect transition, projection, replay step, or support
claim is produced. A pass-only priority window is offered only after its
authoritative action-surface precondition validates; an unimplemented action
candidate is never silently treated as absent.

No domain may commit authoritative state or environment state, append replay,
advance environment counters, bypass Decision, own projection legality, retain
hidden semantic state, or obtain unrestricted mutable full-state authority.
Cross-domain processes use the single primary orchestrators recorded in the
companion artifact.

## Alternatives rejected

### Use the 92-node Standard census closure

Rejected. The census is a stress input for dependency and interaction design,
not M3 scope authority, and it would turn content research into an
implementation queue.

### Select a monolithic `rules/combat` capability

Rejected. Combat has independent declaration, damage, decision, information,
and consequence boundaries. The selected graph keeps those boundaries
reviewable and excludes unsupported assignments explicitly.

### Make T0 a second rules engine or public case protocol

Rejected. T0 must prove the existing authoritative path. Independent legality
or automatic choices in the facade would create a competing semantic authority.

### Select zone incarnation or damage/life as S1

Rejected for S1. Both are valid closure leaves, but their information and
cross-domain risk is higher and they do not provide the temporal context needed
to compose the foundation. They remain selected closure dependencies.

### Freeze generic format hooks or populate the registry

Rejected by ADR 0052 and ADR 0022. No selected M3 capability requires a generic
format callback, and entry-scope identity is not a registered support claim.

## Consequences

Positive consequences:

- a maintainer can trace every selected node to a reviewed root or exact dependency;
- the dependency graph is acyclic and the ownership graph has one owner and one orchestrator wherever required;
- S1 is small enough for exact red-first evidence while remaining reusable;
- the normal-turn integration is bounded without claiming arbitrary Magic;
- unsupported semantics, card scope, deck scope, and certification remain fail-closed;
- T0, semantic capability identity, runtime ownership, storage ownership, and environment commit authority remain distinct.

Costs and constraints:

- the Initial Foundation bounds each case to at most one relevant eligible blocker; one blocker uses a single `ChooseOne` assignment and more than one eligible blocker fails closed before Decision creation;
- multiple blockers per attacker and combat-damage assignment remain a separate semantic slice;
- the M3 exit requires interaction evidence in addition to isolated capability cases;
- semantic versions must change when meaning changes, even if the Rust file layout does not.

## Compatibility

This acceptance-only change alters no production Rust or Python semantics, wire or Decision
schemas, observation schemas, replay/checkpoint codecs, RNG, digests, Card IR,
cards, decks, capability registry entries, or public API. It records the merged
entry decision as accepted while preserving the separate M3 authorization gate.

## Authorization conditions

PR #182 is merged and this ADR is accepted. Before the separate post-merge
authorization comment exists, the current state is:

```text
ENTRY_PR_182_HEAD = c2669d17b5abece5549f0781eea20b73b9776d29
ENTRY_PR_182_MERGE_COMMIT = 10f0387281529be087bcb658b779c19a70c2bf8c
M3_ENTRY_DECISION = ACCEPTED
INITIAL_FOUNDATION = ACCEPTED
M3_STARTED = NO
M3_AUTHORIZED = NO
AUTHORIZATION_HEAD = NOT_SET
```

A separate invocation must then fetch the current remote `master`, verify
containment of the entry PR, re-review the companion artifact and this ADR,
confirm `0 BLOCKER / 0 MAJOR`, verify required hosted gates, update the final
Issue #178 Entry Gate items, and post:

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

Only that explicit post-merge comment authorizes the first M3 implementation
task. Issue #163 may be closed as superseded only after those conditions hold.

## Review state

```text
DEPENDENCY_DAG_REVIEWED = YES
DEPENDENCY_CYCLES = 0
SEMANTIC_OWNERSHIP_GRAPH_REVIEWED = YES
CAPABILITIES_WITHOUT_PRIMARY_OWNER = 0
CROSS_DOMAIN_PROCESSES_WITHOUT_ORCHESTRATOR = 0
ENTRY_INTERACTION_OBLIGATIONS = 16
SATISFIED_INTERACTION_EVIDENCE_AT_ENTRY = 0
CAPABILITY_REGISTRY_MODIFIED = NO
MAGIC_SEMANTICS_IMPLEMENTED = NO
M3_ENTRY_DECISION = ACCEPTED
M3_AUTHORIZED = NO
M3_STARTED = NO
AUTHORIZATION_HEAD = NOT_SET
```
