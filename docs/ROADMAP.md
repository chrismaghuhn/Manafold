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
GOVERNANCE_CLEANUP = COMPLETE_FOR_CANDIDATE
M3_STARTED = NO
M3_AUTHORIZED = NO
AUTHORIZATION_HEAD = NOT_SET
M3_ENTRY_DECISION = NEXT
```

`M2.5 = NOT_CLAIMED / NOT_FROZEN`. Its abandoned census and research
machinery was removed from the active repository; the merged history remains
historical evidence only. Final foundation closure and the Pre-M3 Remediation
Freeze are complete, and Issue #105 is closed as historical audit work. Issue
#162 core modularization is also complete and merged by PR #179. The current
active maintainer work area is M3 Entry governance under Issue #178; this is
not a Magic-semantics milestone and does not authorize M3. The pre-M3
governance cleanup is complete for this candidate; the separate M3 Entry
Decision remains next.

`M3 = NOT_STARTED / NOT_AUTHORIZED` for the Manafold engine, with
`AUTHORIZATION_HEAD = NOT_SET`. Census-driven scope research is outside this authoritative engine repository, and external census M3 authorization must not
be treated as engine-semantic authorization. The next gate is the separate
reviewed M3 Entry Decision and Initial Semantic Foundation selection; no
semantic implementation, S1 selection, or capability registration is implied
by the current governance work.

## M3 — Bounded Semantic Coverage

**Status:** `NOT_AUTHORIZED`; this section describes a future execution model
only. It does not authorize M3, select its first capability, or select a card.

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
No first M3 card, witness, or capability is frozen by this roadmap.

### M3 slice model

The future roadmap is organized around bounded slices, each with its own
declared exit:

```text
M3.T0  thin private conformance facade over the real Rust kernel
M3.S1  first separately reviewed bounded semantic capability slice
M3.S2  next independently justified capability slice
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

The exact number of slices and the Initial Semantic Foundation are not frozen
here.

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
exact capabilities, witnesses, interactions, and count require a separate
reviewed M3 entry/scope decision.

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
