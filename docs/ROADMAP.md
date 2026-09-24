# Roadmap

**Status:** accepted milestone ordering; dates intentionally uncommitted

## M0.2 — Specification and Maintainer Readiness

- normative hierarchy and document register;
- precise domain, execution, error, format, digest, concurrency, and trajectory contracts;
- rules/card capability and certification lifecycle;
- maintainer schemas, scaffolding, census, preflight, and document validation;
- remove duplicate/orphan state ownership;
- all M0.2 specification/tooling gates green; executable contract freeze continues in V0.2.1.

**Exit:** specification baseline complete; V0.2.1 executable contract closure begins. No playable claim.

## V0.2.1 — Executable Contract Closure

- canonical domain-separated full-state digest input;
- complete environment checkpoints with status and limit counters;
- sequential/compositional semantic-event validation;
- exact conformance inputs;
- typed knowledge provenance/invalidation;
- native-executor definition closure;
- full pinned-toolchain gates.

**Exit:** contract frozen; M1 unblocked. No playable claim.

## V0.2.2 — Executable Freeze and Maintainer Ergonomics

- preserve V0.2.1 semantics;
- generate mechanical cross-language vocabulary from one catalog;
- provide fast/integration/certification maintainer profiles;
- split PR/integration/nightly CI;
- provide a tested synthetic golden path;
- commit `Cargo.lock` and pass every freeze gate.

**Exit:** `CONTRACT_FROZEN`; M1 unblocked.

## M1 — Closed Deterministic Kernel Shell

**Status:** COMPLETE by the M1 closure evidence merged in PR #47.

- construct/reset synthetic complete `EngineState`;
- accepted and rejected synthetic decision paths;
- exact state/event/delta/status product;
- checkpoint, restore, fork, replay, and digest parity;
- deterministic ID and RNG streams;
- two bound synthetic player endpoints;
- no real Magic cards.

**Exit:** all 10 M1 gates PASS; M2 unblocked. No playable/real-Magic claim.

## M2 — Decision Machinery and Synthetic Information Safety

**Status:** `COMPLETE` by accepted ADR 0041, which records the exact M2.Final
closure evidence head `352cd80c2ef58a406c30bf7db1cb792109fafc3f`.

- representative closed decision families and serializable continuations;
- separate trusted and player-visible request identity;
- perspective-bound observation, retained information state, observed events, safe errors, and PlayerStep;
- perspective-local opaque/protocol identity lifecycle;
- candidate soundness/completeness harness;
- paired-state byte noninterference and multi-endpoint isolation;
- checkpoint/fork/replay parity for all newly authoritative information state;
- initial rules-free Python semantic integration without choosing production transport.

### M2 implementation slices

```text
M2.A      contract/version architecture freeze (accepted; M2.B local structural gate PASS)
M2.B      one V3 state/digest/checkpoint/replay structural cut
M2.C      closed decisions and typed continuations
M2.D      player projection, PlayerStep, and errors
M2.E      knowledge, opaque identity, and observed events
M2.F      legal-choice soundness/completeness harness
M2.G      paired-state noninterference and endpoint closure
M2.H      temporary rules-free Python semantic adapter
M2.Final  exact-head executable closure
```

M2.A architecture acceptance did not by itself mark an M2 executable behavior
gate `PASS`; the later M2.Final evidence and accepted ADR 0041 closed the M2
foundation. M2.B remains the structural implementation slice within that
history.

## Post-M2 scope boundary

The current project status is:

```text
FINAL_FOUNDATION_CLOSURE = PASS
PRE_M3_REMEDIATION_FREEZE = PASS
FOUNDATION_READY_FOR_M3 = YES
CORE_MODULARIZATION = COMPLETE
GOVERNANCE_CLEANUP = COMPLETE
M3_PRE_T0_HARDENING = COMPLETE / ACCEPTED
ADR_0054 = ACCEPTED
FOUNDATION_V2 = ACCEPTED
M3_PLAN_STATUS = ACCEPTED
PRIOR_AUTHORIZATION_HEAD = 0f13b43680ea7d0b043c5baee59eb2ed3c364ecc
M3_AUTHORIZED = YES
AUTHORIZATION_HEAD = ea668c47ef1361b3d989fd32b8f3cfd4751b1e79
AUTHORIZED_TASK_AT_AUTHORIZATION_HEAD = M3.P0_STATE_IDENTITY_CUT
M3_STARTED = YES
P0_REVIEW_HEAD = a7e641a7e6145610c9533187cf6340712f460e44
P0_MERGE_COMMIT = 20dac927027776ef5f0a5b389a27d4a05eefb180
P0_EXACT_HEAD_REVIEW = APPROVE
P0_COMPLETE = YES
P0_FROZEN = YES
T0_AUTHORIZED = YES
T0_STARTED = YES
T0_01 = COMPLETE / MERGED (PR #189, merge acdde953b133dae94a4641c429f4f3d352b34838)
T0_02A = COMPLETE / MERGED (PR #190, merge 299827175c285cb081b62d67e1668370185b704b)
T0_02B = COMPLETE / MERGED (PR #191, merge b403edefcabf7b304c0fa5f6816d22ac8aca477b)
T0_CLOSURE_REVIEW_HEAD = b403edefcabf7b304c0fa5f6816d22ac8aca477b
T0_STATUS_SYNC_REVIEW_HEAD =
56d0343b5e500f8460660991c15a8164437dbd50
T0_STATUS_SYNC_PR = #193
T0_STATUS_SYNC_MERGE =
b9c5f2be97b8fc1f31d648d58f890de78f0a035c
T0_COMPLETE = YES
T0_FROZEN = YES
T0_FREEZE_EXECUTED = YES
M3_S1 = rules/turn-structure@0.1.0
M3_S1_AUTHORIZATION_REVIEW = APPROVE
S1_AUTHORIZATION_HEAD =
587016574e4e8f9f797a713877f8caf1c5143cfb
S1_AUTHORIZATION_ELIGIBLE = YES
S1_AUTHORIZED = YES
S1_IMPLEMENTATION_AUTHORIZED = YES
S1_STARTED = YES
S1_IMPLEMENTATION_STARTED = YES
S1_AUTHORIZED_TASK_AT_S1_HEAD = M3.S1
S1_DEPENDENCIES = NONE
S1_PRIMARY_SEMANTIC_OWNER = turn
S1_TURN_STRUCTURE_LIFECYCLE = covered
S1_COVERAGE_STATUS = covered / certification not claimed
S1_SPECIFIED_CAPABILITY_COUNT = 10
S1_IMPLEMENTED_CAPABILITY_COUNT = 0
S1_COVERED_CAPABILITY_COUNT = 1
S1_CERTIFIED_CAPABILITY_COUNT = 0
CURRENT_RESUMABLE_EXECUTION_CONTRACT = V6
V4_V5_RESUMABLE_CONTRACT_STATUS = HISTORICAL_ONLY / V5_TO_V6_MIGRATION_NONE
TASK_13_DOCUMENTATION_STATUS_CLOSURE = COMPLETE
TASK_14_EXACT_HEAD_VERIFICATION = COMPLETE
S1_EXACT_HEAD_VERIFICATION = PASS
M3_S1_STATUS = COMPLETE / COVERED / NOT CERTIFIED
M3_S2 = rules/zone-incarnation@0.1.0
PR_208 = MERGED
S2_EXACT_HEAD_VERIFICATION = PASS
M3_S2_STATUS = COMPLETE / IMPLEMENTED / NOT COVERED / NOT CERTIFIED
M3_S2_AUTHORIZED = YES
S2_IMPLEMENTATION_AUTHORIZED = YES
S2_STARTED = YES
S2_IMPLEMENTATION_STARTED = YES
S2_ZONE_INCARNATION_LIFECYCLE = implemented
S2_COVERED = NO
S2_CERTIFIED = NO
S2_COVERAGE_STATUS = implemented / covered blocked by authoritative replay
S2_SPECIFIED_CAPABILITY_COUNT = 9
S2_IMPLEMENTED_CAPABILITY_COUNT = 1
S2_COVERED_CAPABILITY_COUNT = 1
S2_CERTIFIED_CAPABILITY_COUNT = 0
S2_AUTHORITATIVE_REPLAY = DEFERRED_REQUIRED / BLOCKED_FOR_COVERED
S2_STATE_BASED_ACTIONS_INTERACTION = UNSATISFIED
S2_DRAW_CARD_INTERACTION = UNSATISFIED
TASK_7_LIFECYCLE_PROMOTION = COMPLETE
TASK_8_EXACT_HEAD_VERIFICATION = COMPLETE
S1_SUPPORT_PREDICATE_REQUIRES_EXACTLY_TWO_PLAYERS = YES
S1_ZONE_TRANSITIONS_AUTHORIZED = NO
S1_ZONE_LOCATION_MUTATION_AUTHORIZED = NO
S1_OBJECT_INCARNATION_CHANGE_AUTHORIZED = NO
S1_PHYSICAL_CARD_IDENTITY_CHANGE_AUTHORIZED = NO
S1_ALLOWED_ZONES_OWNED_MUTATION = bounded untap tapped-field mutation only
S1_REVIEW_MINORS = 3 CARRIED
S3_P0_IMPLEMENTATION = COMPLETE
S3_P0_COMPLETE = YES
S3_P0_FROZEN = YES
S3_P0_REVIEW_HEAD = 0cd24d1f2cb4183c19fb04ce0c3c827148313a3b
S3_P0_PR = #210
S3_P0_MERGE_COMMIT = ffc433985f41e6e2980df23a103b5e2358527ea3
S3_0_AUTHORIZED = YES
S3_0_IMPLEMENTATION_AUTHORIZED = YES
S3_0_STARTED = YES
S3_0_COMPLETE = YES
S3_0_FROZEN = YES
S3_0_REVIEW_HEAD = aa28f9753225dca0e58e33b0d0356a1cc560aae3
S3_0_PR = #211
S3_0_MERGE_COMMIT = 66f3b713787cad89674257f6e0b6448b9fd568f9
S3_A_AUTHORIZED = YES
S3_A_IMPLEMENTATION_AUTHORIZED = YES
S3_A_STARTED = YES
S3_A_TASK_5 = COMPLETE / REVIEWED
S3_A_TASK_5_REVIEW_HEAD = 0e56a805136f8ccec7e41d19067ed21df010488d
S3_A_TASK_6 = COMPLETE / REVIEWED
S3_A_TASK_6_REVIEW_HEAD = 5711886772e431a80f81bcd099cb5546d327a362
S3_A_TASK_7 = COMPLETE / REVIEWED
S3_A_TASK_7_REVIEW_HEAD = eb9c9a92d60bf82bccc0ceeb174ad58e1f8c1d48
S3_A_TASK_8 = COMPLETE / REVIEWED
S3_A_TASK_8_REVIEW_HEAD = 8a531c33ced9e532bd90b3870ac088b06924c348
S3_A_TASK_9A = COMPLETE / REVIEWED
S3_A_TASK_9A_REVIEW_HEAD = 28ffe32b8acb72c0ec3cca98bcfbd90499827f5b
S3_A_TASK_9B0 = COMPLETE / REVIEWED
S3_A_TASK_9B0_REVIEW_HEAD = 8cafb91b121cc2e29f3834ddb1a959a429a20ec4
S3_A_TASK_9 = COMPLETE / REVIEWED
S3_A_BLOCK_1 = COMPLETE / REVIEWED
S3_A_IMPLEMENTATION = COMPLETE / REVIEWED
S3_A_EXACT_HEAD_REVIEW = PASS
M3_BLOCK_1 = COMPLETE / REVIEWED
M3_BLOCK_1_REVIEW_HEAD = 6fc3ff9694aa9d61975929a2d4a1c8006df28014
S3_A_PRODUCTION_SEMANTIC_CONTRACT_REQUIRED = YES
S3_A_PRODUCTION_SEMANTIC_CONTRACT_ALLOCATED = YES
S3_A_PRODUCTION_SEMANTIC_CONTRACT_ID = 51efc0307d9ef8fc4fca46f8ea6e4ea5d5293cb8301c2a7917a590982079020e
S3_A_LIFECYCLE = specified
NONTERMINAL_FINAL_ORDER_REFERENCE_PATH = PASS
S3_B_AUTHORIZED = YES
S3_C_AUTHORIZED = NO
S3_B_IMPLEMENTATION_AUTHORIZED = YES
S3_C_IMPLEMENTATION_AUTHORIZED = NO
MAGIC_RULES_FLOW_INVENTORY_V1 = REVIEWED / FROZEN_PLANNING_INPUT
MAGIC_RULES_FLOW_INVENTORY_REVIEW_HEAD = 8a440645735fcb17be9470c7a53969860b1d4fad
M3_MAJOR_SEMANTIC_BLOCKS = 8
S3_B_STARTED = YES
S3_B_IMPLEMENTATION = COMPLETE_CANDIDATE
S3_B_PRODUCTION_SEMANTIC_CONTRACT_REQUIRED = YES
S3_B_PRODUCTION_SEMANTIC_CONTRACT_ALLOCATED = YES
S3_B_PRODUCTION_SEMANTIC_CONTRACT_ID = c480cbae69bf0496aff83bb973a859721bfa0f969351b33b3f0b09ee3f7c5498
BASIC_PRIORITY_LIFECYCLE = specified
M3_BLOCK_2 = COMPLETE_CANDIDATE
BLOCK_3_STARTED = NO
CURRENT_M3_BLOCK = BASIC_PRIORITY_AND_REFERENCE_RESPONSE_INTEGRATION
NEXT_GATE = M3_BLOCK_2_EXACT_HEAD_REVIEW
```

`M2.5 = NOT_CLAIMED / NOT_FROZEN`. Its abandoned census and research
machinery was removed from the active repository; the merged history remains
historical evidence only. Final foundation closure and the Pre-M3 Remediation
Freeze are complete, and Issue #105 is closed as historical audit work. Issue
#162 core modularization is also complete and merged by PR #179. The accepted M3 authorization at
`ea668c47ef1361b3d989fd32b8f3cfd4751b1e79` authorized the task
`M3.P0_STATE_IDENTITY_CUT`; that semantic-neutral P0 infrastructure slice is
now merged after an approved cumulative exact-head review, with reviewed head
`a7e641a7e6145610c9533187cf6340712f460e44` and merge commit
`20dac927027776ef5f0a5b389a27d4a05eefb180`, so P0 is complete and frozen. The
pre-M3 governance cleanup and M3 Entry Decision remain historically accepted,
and PR #184 has accepted the hardened ADR 0054 and Foundation V2 plan. T0 was
reauthorized under Issue #178 and implemented by merged PRs #189 (T0-01),
#190 (T0-02A), and #191 (T0-02B); the exact-head closure review found the
frozen T0 exit evidence complete, the repository status sync was exact-head
reviewed, CI-green, and merged as PR #193, and Issue #178 finalized T0 as
COMPLETE / FROZEN. Issue #178 recorded an exact-master S1 authorization
review APPROVE at `587016574e4e8f9f797a713877f8caf1c5143cfb`:
At that historical authorization point, `rules/turn-structure@0.1.0` was
authorized for implementation and implementation had not started; the current
registry now records the bounded capability as `covered`, with certification
still unclaimed. At S1 authorization, the other ten Foundation capabilities
were still specified and downstream work had not been authorized.

PR #208 merged S2 at `b67cfdcc0a8e623da52a889ef2ae138a3e4256ac`; exact-head
verification passed. M3.S2 is complete as an implementation slice, but is not
covered or certified. S2 authoritative replay remains deferred and required
before covered. M3.S3 selection/design is complete. S3.P0 is complete and
frozen after PR #210 merged at
`ffc433985f41e6e2980df23a103b5e2358527ea3`, following exact-head review of
`0cd24d1f2cb4183c19fb04ce0c3c827148313a3b`. S3.0 is complete and frozen.
PR #211's reviewed source head `aa28f9753225dca0e58e33b0d0356a1cc560aae3`
merged at `66f3b713787cad89674257f6e0b6448b9fd568f9`. S3.A is authorized and
started. Block 1 is complete / reviewed. S3.B Basic Priority is implemented
as a candidate under its distinct generated semantic identity; its capability
lifecycle remains specified pending exact-head review. Block 3 and S3.C remain
unstarted / unauthorized.

M3 has started through semantic-neutral P0 infrastructure, T0
conformance/proof infrastructure, the covered S1 turn-structure capability,
and the implemented bounded S2 zone-incarnation capability. P0 is
complete/frozen; T0 is COMPLETE / FROZEN; S1 is covered and S2 is implemented,
neither certified. Nine Foundation capabilities remain `specified`; one is
implemented and one is covered. S2 authoritative replay remains deferred and
required before covered. The state-based-actions-combat and draw-card
interactions with S2 are unsatisfied. Census-driven scope research is outside
this authoritative engine repository, and external census M3 authorization
must not be treated as engine-semantic authorization. No broad Magic, card,
deck, format, Commander, or playability claim follows.

The historical accepted operational scope record for that decision is
[`docs/rules/M3_INITIAL_SEMANTIC_FOUNDATION_V1.md`](rules/M3_INITIAL_SEMANTIC_FOUNDATION_V1.md).
The accepted hardening plan is recorded in
[`docs/rules/M3_INITIAL_SEMANTIC_FOUNDATION_V2.md`](rules/M3_INITIAL_SEMANTIC_FOUNDATION_V2.md)
and [`docs/adr/0054-m3-pre-t0-hardening.md`](adr/0054-m3-pre-t0-hardening.md).
Neither artifact itself performs execution authorization; the P0 authorization
was recorded at `ea668c47ef1361b3d989fd32b8f3cfd4751b1e79`, and T0
reauthorization was recorded separately under Issue #178.

## M3 — Bounded Semantic Coverage

**Status:** `STARTED — P0 COMPLETE / FROZEN — T0 COMPLETE / FROZEN — S1 COMPLETE / COVERED / NOT CERTIFIED`;
the sections below describe the
execution model for the remaining slices. M3.P0 infrastructure is merged and
frozen; M3.T0 is finalized COMPLETE / FROZEN (Issue #178); M3.S1 is
`COMPLETE / COVERED / NOT CERTIFIED` for the bounded turn-structure
capability. Task 13 documentation/status/generated-contract closure and Task
14 exact-head verification are complete. M3.S2 was merged by PR #208 and
passed exact-head verification; it remains not covered while authoritative
replay is deferred. S2 is complete / implemented / not covered / not certified.

The planned execution order is:

```text
M3.P0 semantic-neutral state/persistence identity cut
→ M3.T0 thin private conformance facade
→ M3.S1 rules/turn-structure@0.1.0 covered
→ Task 13 documentation/status/generated-contract closure complete
→ Task 14 exact-head verification COMPLETE
→ M3.S2 rules/zone-incarnation@0.1.0 implemented / not covered
→ Task 7 lifecycle promotion COMPLETE
→ Task 8 exact-head verification PASS (PR #208 merged)
→ M3.S3 selection and design
```

P0 added no Magic capability and advanced no capability lifecycle; T0 added no
Magic capability and advanced no capability lifecycle.

### Purpose and progress authority

When separately authorized, M3 builds reusable Magic semantics as bounded,
independently reviewable capability slices. Its primary progress unit is a
covered semantic capability slice and its applicable reviewed interaction
obligations.

M3 progress is not measured by:

- card count or deck completion;
- raw parsed, generated, or compiled content;
- usage frequency by itself; or
- playability of a game or matchup.

The architecture driver is semantic depth followed by interaction evidence,
then reusable capability coverage, then content breadth. A concrete deck or
deck pair may provide witnesses, but it must not determine M3 semantics.

### Semantic witnesses

A **Semantic Witness** is a concrete card, rules example, ruling example, or
minimal scenario used to expose and prove a reusable semantic capability. A
witness is evidence input, not a Manafold card definition or support claim:

```text
witness != supported
implemented != covered
covered != certified
```

Any named card used while planning a slice is illustrative unless a later
review explicitly promotes it through the Card IR and certification workflows.
The S1 capability identity is retained from accepted ADR 0053 and the accepted
hardening plan; no card or witness support claim is frozen by this roadmap.

### M3 slice model

The future roadmap is organized around bounded slices, each with its own
declared exit:

```text
M3.T0  thin private conformance facade over the real Rust kernel
M3.S1  rules/turn-structure@0.1.0 covered; certification not claimed
M3.S2  rules/zone-incarnation@0.1.0 implemented; not covered pending authoritative replay
M3.S3  first meaningful multi-capability interaction closure
M3.Sn  additional reviewed slices as evidence and scope justify them
```

M3.T0 may provide complete setup, explicit player responses, exact transition
products, rejection nonmutation, state/event/delta assertions,
perspective-safe assertions, structured diagnostics, and applicable
replay/checkpoint/fork assertions. It remains a thin facade over the real
authoritative kernel and does not require a real card.

A slice normally declares its capability identity and bounded scope, pinned
authority, explicit exclusions, minimal witnesses, RED conformance cases,
implementation, applicable decision and information evidence, state/event/
delta evidence, replay/checkpoint/fork parity where relevant, reviewed
interaction obligations, fail-closed unsupported cases, and the evidence
needed for lifecycle advancement. An evidence class may be marked not
applicable only with a scope-specific reason.

In compact form, a bounded coverage claim requires:

```text
capability
    + included scope and explicit exclusions
    + pinned authority and implementation
    + isolated conformance evidence
    + applicable interaction obligations and evidence
    + applicable decision, information, state/event/delta, and parity evidence
    = bounded semantic coverage claim
```

The claim never silently extends beyond its declared scope.

The durable scope rule is:

> Complete one semantic capability slice before expanding to unrelated
> semantic breadth.

The exact number of later slices is not frozen here. The Initial Semantic
Foundation scope is historically accepted by ADR 0053, refined by the accepted
ADR 0054/Foundation V2 hardening plan, and remains gated by the separate
exact-`master` reauthorization procedure.

### Interaction closure and risk map

Issue #129's interaction philosophy remains in force:

```text
POTENTIAL_SEAM
    -> REVIEWED_OBLIGATION
    -> SATISFIED_EVIDENCE
```

The obligations are explicit, reviewed, scope-aware, and risk-based. M3 must
review applicable seams, but it must not require a Cartesian product of every
capability, and it must not allow zero interaction review. Isolated capability
passes do not prove their composition.

The following domains remain an evidence-priority risk map, not a global M3
exit checklist:

```text
zones / object incarnation       LKI
replacement / prevention         triggers
state-based actions              priority / stack
hidden information / knowledge   continuous effects / layers
copy effects                     loops / forced progress
costs / payment                  targeting / resolution legality
combat                           control / ownership
Commander-specific state
```

A domain may therefore contain one narrow covered slice alongside explicitly
unsupported cases. A narrow continuous-effects slice, for example, does not
claim that all layer or dependency cases are supported.

### Selection signals and external research

Usage and census data may help prioritize reusable capabilities. They do not
establish semantic authority, support, or correctness, and this roadmap freezes
no scoring formula, weights, or percentages. The separate `manafold-census`
project remains advisory:

```text
census signal
    -> human review
    -> explicit Manafold scope decision
    -> capability specification
    -> implementation and conformance
```

A future reviewed selection may consider foundational importance, reuse across
cards, usage-informed relevance, interaction centrality, ML decision value,
information-safety value, and a reasonable bounded implementation cost. No one
signal dominates automatically; highest usage is not automatically first.

`Coverage gain` is another advisory planning concept: implementing one
reusable capability may move many content items closer to recursive closure.
Future census work may estimate cards requiring a capability,
usage-weighted demand, sole blockers, or recursive closure gain. None of these
estimates is a support count, a certification result, or an engine decision.

```text
ML/census planning signal != Rust rules authority
```

External engines and their failure registers remain research and
differential-testing references only. Generalized lessons motivating this
boundary include the externally recorded classes `EXT-MTG-004`, `EXT-MTG-006`,
and relevant 1v1 portions of `EXT-MTG-018`: replacement/prevention/trigger/
SBA/layer and repeated-instance composition can fail; parsed, generated, or
compiled shape is not semantic support; actor, ordering, and simultaneous-
choice distinctions matter. Therefore:

```text
parsed != semantically supported
compiled != semantically supported
many implemented cards != proven composition
isolated capability correctness != interaction correctness
playable game != conformance evidence
later gameplay discovery != a substitute for explicit interaction evidence
```

These are generalized architectural lessons, not claims that another engine
is Manafold's authority or that any finding belongs exclusively to a named
engine. Do not assign an `EXT-MTG-*` finding to a specific external engine
unless the finalized external-failure register provides that provenance.

### Initial Semantic Foundation and M3 exit

The **Initial Semantic Foundation** is a small, explicitly reviewed set of
foundational capability slices, their applicable reviewed interaction
obligations, and the conformance infrastructure needed to prove them. Its
historical exact capabilities, witnesses, interactions, and count are accepted
by ADR 0053; the current dependency/lifecycle/state hardening is recorded by
accepted ADR 0054 and Foundation V2. Neither status claims implementation,
coverage, certification, or M3 authorization.

M3 may advance to its declared bounded exit only when that foundation and its
required gates have current `PASS` evidence. This is not a claim that all
Magic mechanics, all Issue #129 risk domains, a complete Commander rules
engine, arbitrary Commander or deck support, a representative metagame, or
global card coverage are complete. These are not requirements for the bounded
M3 exit.

### M3 and M4 progress are separate

| M3 semantic progress | M4 content and integration progress |
| --- | --- |
| specified, implemented, and covered capabilities | reviewed Card IR definitions |
| semantic witnesses with current evidence | definitions covered by their requirements |
| satisfied reviewed interaction obligations | recursive capability closure |
| applicable decision, information, state/event/delta, and parity evidence | exact deck or bundle manifests |
| bounded exclusions and fail-closed unsupported cases | bundle certification and end-to-end integration |

These dimensions must not be collapsed into one percentage.

## M4 — Card Definitions, Bundle Integration, and First Certification

M4 may begin after the separately reviewed Initial Semantic Foundation reaches
its declared exit. M3 is the Initial Semantic Foundation milestone; once that
exit is reached, M3 may be marked `COMPLETE` and M4 may begin. M4 does not wait
for universal Magic semantic closure. Later semantic capabilities continue
through the same capability workflow during M4 or later; they do not reopen M3
or block M4 merely because unrelated Magic semantics remain unsupported.

M4 owns reviewed Card IR definitions, content-specific conformance, recursive
capability closure, bundle integration, exact deck manifests when required,
and the first legitimate locked-bundle support claim. For each card or
content item:

```text
card definition
    -> derive recursive capability requirements
    -> all required capabilities covered?
         YES -> continue content and bundle evidence
         NO  -> return the missing general semantics to the capability workflow
```

Missing semantics must be implemented as reusable capabilities, never as
card-name-specific shortcuts. M3 semantic coverage does not automatically
support a card, deck, format, or bundle.

Certification remains bundle-specific:

```text
locked bundle
    + exact capability closure
    + exact content and snapshots
    + required runtime and conformance evidence
    = certified support claim
```

A small, deeply proven initial bundle may therefore be certified while broader
Magic semantics remain unsupported, provided every semantic capability
reachable and required by that bundle has complete required closure and
current evidence. Exclusions bound the certified support claim; they do not
waive missing requirements inside the bundle's reachable scope. Unsupported
semantics outside that scope may remain excluded, but an unsupported reachable
requirement blocks certification. Later bundles may require additional
capabilities and evidence; M3 semantic coverage alone never certifies a card,
deck, format, or bundle.

## M3/M4 boundary summary

```text
M3: bounded reusable semantics and applicable interaction obligations
    -> M4: reviewed Card IR, content integration, bundles, decks,
           and certification
```

M4 content breadth is downstream of semantic evidence. A deck manifest and
bundle are certification artifacts, not the architecture driver for M3.

## M5 — ML Environment and Baselines

- stable Python/native transport;
- versioned trajectory schema and semantic action keys;
- scripted/random/heuristic baselines;
- replay-buffer/export tooling;
- batched environment/inference boundary;
- benchmark and leak evidence.

## Later

- more certified bundles and deck generalization;
- four-player Commander and vector utilities;
- information-set/search APIs;
- optimized reversible rollout backend after profiling and differential parity;
- broader source-assisted card implementation.
