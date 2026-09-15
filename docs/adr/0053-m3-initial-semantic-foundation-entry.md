# ADR 0053 — M3 Entry Decision: Initial Semantic Foundation

- **Status:** accepted candidate; effective on merge of the entry PR
- **Date:** 2026-09-15
- **Owners:** architecture maintainers; rules maintainers; conformance maintainers
- **Resolves:** the M3 Entry Decision scope gate in Issue [#178](https://github.com/chrismaghuhn/Manafold/issues/178)
- **Supersedes:** Issue [#163](https://github.com/chrismaghuhn/Manafold/issues/163) only after merge, independent approval, and explicit authorization
- **Companion authority:** [`M3_INITIAL_SEMANTIC_FOUNDATION_V1.md`](../rules/M3_INITIAL_SEMANTIC_FOUNDATION_V1.md)
- **Implementation evidence:** `NOT_RUN`; this ADR changes no Magic behavior and authorizes no M3 implementation

This is the candidate M3 Entry Decision. Before the entry PR is merged, the
record is not present on `master` and cannot authorize M3. After merge it makes
the scope durable, but a separate exact-`master` authorization remains
required.

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
priority, ordinary draw and cleanup consequences, bounded combat declarations,
simultaneous combat damage, basic life/marked-damage results, the selected SBA
fixed point, and the selected zone/incarnation consequences. The companion
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
parity, and first-divergence assertions. It does not calculate Magic legality,
does not silently complete choices, does not create a public RulesCase schema,
and introduces zero Magic capabilities or support claims.

### S1

The first real Magic capability is:

```text
S1_ID = rules/turn-structure
S1_VERSION = 0.1.0
S1_OWNER = turn
S1_AUTHORITY = wotc-cr-2026-08-07-txt-20260819-sha256-4381ad1b39ab2c05f7d03633a20f711ed37277074d3266dcba5f38cbb527423f
S1_SCOPE = two-player, format-neutral temporal structure with ordinary untap,
           fixed normal phase/step order, no-priority classification, and
           active-player switch after cleanup
```

S1 is selected from the zero-depth dependency frontier because it is the
smallest genuine Magic semantic boundary that directly establishes the
temporal context required by every later selected action. Zone incarnation and
basic damage/life are also frontier leaves, but they carry higher information
and interaction risk and do not establish the temporal/action skeleton.
Priority and combat are deliberately downstream of the selected temporal
context.

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
claim is produced.

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

- the blocker assignment uses an internal typed staged Decision representation because the current M2 decision union has no pair-valued public candidate; the representation remains one atomic rules action and does not freeze a public schema;
- multiple blockers per attacker and combat-damage assignment remain a separate semantic slice;
- the M3 exit requires interaction evidence in addition to isolated capability cases;
- semantic versions must change when meaning changes, even if the Rust file layout does not.

## Compatibility

This candidate changes no production Rust or Python semantics, wire or Decision
schemas, observation schemas, replay/checkpoint codecs, RNG, digests, Card IR,
cards, decks, capability registry entries, or public API. It adds only the
companion governance artifact, this ADR, the normative-document registration,
and the roadmap/index references required to make the candidate discoverable.

## Authorization conditions

The entry PR must remain unmerged until reviewed. While it is unmerged:

```text
M3_ENTRY_DECISION = CANDIDATE
INITIAL_FOUNDATION = CANDIDATE
M3_STARTED = NO
M3_AUTHORIZED = NO
AUTHORIZATION_HEAD = NOT_SET
```
After merge, a separate invocation must fetch the current remote `master`,
verify containment of the entry PR, re-review the companion artifact and this
ADR, confirm `0 BLOCKER / 0 MAJOR`, verify required hosted gates, update the
final Issue #178 Entry Gate items, and post:

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
M3_ENTRY_DECISION = CANDIDATE
M3_AUTHORIZED = NO
M3_STARTED = NO
AUTHORIZATION_HEAD = NOT_SET
```
