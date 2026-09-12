# ADR 0047: M2.5 Scope Restoration and M3 Entry Boundary

**Status:** accepted

**Date:** 2026-09-12

**Supersedes:** none

**Superseded by:** none

**Depends on:** accepted `docs/ROADMAP.md`, `docs/SCOPE.md`, `docs/contracts/ACCEPTANCE_GATES.md`, ADR 0022, ADR 0037, ADR 0041, ADR 0045, and ADR 0046

**Reviewed baseline:** `0b7061ff230980e6ce9fe285093bc393438d1f8f`

**Candidate SHA-256 before promotion:** `27ccc8923b8529870d59194f4e5da3fdd9e45dc8e0e35adb03dd97ce6cf80f39`

**Research bundle SHA-256:** `fa94ea14cdf8bd59f2731957d832de6d43487f753f05c78b4503bc217bca2cad`

**Review provenance:** independent full ADR-candidate review; `APPROVE`; 0 BLOCKER / 0 MAJOR / 0 MINOR / 0 NIT

**Implementation evidence:** `NOT_RUN`

**Decision owner:** architecture/governance with scope, rules, capability, and
release owners

**Review boundary:** this ADR changes milestone ownership and exit-gate
interpretation only. It does not implement code, modify C/B1/B2/Authority,
change a candidate, or authorize a particular M3 implementation.

This ADR accepts milestone-scope architecture only. It does not authorize implementation, C changes, Authority migration, Candidate-1 or Candidate-4 resolution, Production Acceptance, Domain Audit 55/55, ranking, deck locking, or M3.

## Context

The accepted roadmap names M2.5 **Exact V1 Deck Lock and Capability Census**.
Its purpose is to freeze the finite two-deck V1 scope and the reusable
capability/decision/information obligations needed to implement that scope.
M3 then implements required Magic primitives, and M4 produces card definitions
and a certified V1 bundle.

The current pre-research package contains six source snapshots and a broad
15,679-candidate interaction ledger. C correctly represents all those
candidates as unresolved because no accepted Pair/Relation Authority closes
their semantic relation. Candidate 1 subsequently demonstrated that a single
semantic review can be approved while Production Authority still exposes
cross-layer B2-v2 and B1.Final gaps.

Treating every broad census candidate as an immediate Production Authority and
Acceptance obligation has made that later authority system the critical path
for M2.5. The repository's accepted scope and certification documents do not
make that equivalence.

## Problem

The repository currently contains three different kinds of completeness:

1. **Census completeness:** identities, source rows, reachable objects,
   capabilities, decisions, information effects, interactions, and exclusions
   are enumerated for a declared scope.
2. **Semantic scope completeness:** the obligations and reusable primitive
   surfaces needed to implement the declared scope are known and frozen.
3. **Production semantic authority closure:** every candidate has an accepted
   terminal relation/domain/context result with immutable acceptance
   provenance.

The first two are M2.5 scope work. The third is a later C/authority and bundle
certification responsibility. The current documents do not state that boundary
with enough precision to safely end M2.5 or authorize M3.

## Decision

Adopt the following milestone interpretation, subject to independent review
and formal ADR acceptance:

* M2.5 freezes an exact two-deck V1 scope envelope and its conservative future
  obligations.
* M2.5 does not require Production Authority and Acceptance for every broad
  interaction candidate in the pre-research ledger.
* `DECLARED_INTERACTION_MODEL_CLOSURE` remains a strict C gate. Its rule is
  unchanged: `PASS` requires `unresolved = 0` and all resolved candidates to
  have valid terminal/domain/context evidence.
* A blocked C snapshot may be carried into the M2.5 freeze only as an explicit,
  immutable future-obligation ledger. It is not C acceptance, semantic support,
  or permission to infer any unresolved result.
* M3 becomes eligible only after the separate M2.5 scope freeze passes and a
  separately authorized M3 task identifies its first bounded capability
  vertical slice. C does not automatically promote M3, and M3 does not
  promote C.
* M4/V1 certification requires complete authority and runtime evidence for the
  reachable bundle scope. Any unresolved interaction or missing authority
  remains a certification blocker.

The intended distinctions are normative:

```text
C BLOCKED       != candidate absent
C BLOCKED       != safe to assume semantics
C BLOCKED       != support
deferred        != waived
unresolved      != ignored
scope frozen    != implementation complete
implemented     != certified
```

## M2.5 exit contract

M2.5 FINAL may be declared only when the following are exact and reviewed:

1. exactly two legal official Commander 1v1 deck manifests are selected and
   content-addressed;
2. rules, format policy, Oracle/source, rulings, legality, schema, toolchain,
   and source-package identities for the selected scope are pinned;
3. all reachable cards, faces, tokens, copies, named/generated objects and
   known edge surfaces for the selected pair are enumerated;
4. the selected-pair decision, ordering, payment, and information inventories
   are frozen, including explicit unsupported cases;
5. the selected-pair capability requirement census and recursive dependency
   closure are complete as a scope/readiness model, with semantic owners;
6. the interaction candidate census is complete and source-bound for the
   declared scope;
7. unresolved C candidates are retained with their exact identity, source,
   closed reason, future owner, and certification-blocker status;
8. B1.Final and B2 upstream prerequisite closures pass, and the ADR 0046 B2
   current root is exact and verified;
9. target hardware and numerical acceptance thresholds are frozen;
10. the scope-impact/change process and exact-head freeze evidence pass; and
11. no generated report or prose promotes C, ranking, deck lock, support, or
    M3 beyond the result actually evidenced.

The broad six-deck REV3 ledger may be retained as research provenance, but it
must not be silently presented as the final two-deck V1 scope.

## C correctness preservation

This decision makes no change to the C contract or its schemas. In particular:

* C `PASS` still requires `unresolved = 0`.
* An unresolved candidate still has `terminal_disposition = null` and no
  semantic class.
* An unresolved candidate cannot be treated as `required_interaction`,
  `not_an_interaction_with_proof`, or out of scope.
* C still fails closed when B1/B2/source/identity/domain/context evidence is
  missing or contradictory.
* The existing candidate universe and C V3 blocked snapshot remain the
  source-bound ledger; no second C truth is introduced.
* A future C authority change remains a separately reviewed, versioned
  upstream contract. Candidate 1's `b2_closure_v2` and B1.Final citation gaps
  are not repaired or waived by this ADR.

The C specification's statement that a blocked snapshot cannot promote a
downstream gate is preserved. This ADR does not make C promote M3. It defines a
separate scope-freeze gate under which a later M3 authorization can be
considered without claiming that C has passed.

## M3 entry contract

M3 may be separately authorized only after M2.5 FINAL and requires, for its
specific vertical slice:

* exact frozen scope/source identities;
* the relevant capability and recursive dependency obligations;
* a pinned rules/Oracle/ruling authority set for the slice;
* explicit state, event, decision, ordering, visibility, identity, replay and
  information-safety obligations;
* red conformance cases before implementation;
* an explicit unsupported/fail-closed boundary; and
* separate implementation/review authorization.

M3 may implement a reusable primitive needed by the locked scope even while
unresolved C candidates remain. It may not claim those candidates are resolved,
supported, or certified. A C-unresolved candidate is a future authority and
certification obligation, not a license for an approximate rules path.

## M4 and certification ownership

M4 owns card definitions and the certified V1 bundle. Before a support claim,
the relevant locked bundle must have:

* reviewed card definitions and generated/reference objects;
* complete recursive capability closure for the bundle;
* required Relation/Domain/Context semantic authority for the bundle's
  interaction cases;
* valid immutable Acceptance and supersession provenance;
* soundness, completeness, information, replay, fuzz/soak, and performance
  evidence; and
* clean exact-source reproduction and all certification gates.

For a bundle that includes the complete declared C universe, `C PASS` and the
corresponding full C authority closure are required. A narrower future bundle
may use an explicitly narrower, complete scope, but it may not call the broad
unresolved ledger certified.

## Deferred obligations

The M2.5 freeze references the existing C candidate universe, classification
shards, blocked closure, and verification summary. Each unresolved item remains
bound to:

```text
candidate identity
source-instance identity
candidate shape and participants
source/package identity
review state and closed unresolved reason
required future authority families
owning milestone
certification blocker flag
```

The future authority program scales through reusable theorems and exact finite
applications. Human review is required for new semantic theorem families,
outliers, and acceptance decisions; the system must not require 15,679 bespoke
one-off semantic sessions merely because 15,679 source candidates were
enumerated. Every candidate that is actually resolved must nevertheless carry
candidate-specific, source-grounded proof and all required domain/context
coverage.

## Candidate-1 and Candidate-4 consequence

Candidate 1 remains an approved semantic canary with Production Authority
blocked by post-census contract gaps. Its Authority is deferred after M2.5.
Candidate 4 remains a later role-divergent canary exercising ParticipantRole-
Bridge/RPA/Context-V3/HostBinding concerns. Neither is required to establish
the finite M2.5 scope envelope or the first M3 primitive list.

## Ranking and deck selection

The roadmap requires selecting two exact decks; it does not require an
authoritative global ranking to exist. Therefore:

* a maintainer may make an explicit, source-bound deck-selection decision
  without claiming that it is an authoritative ranking;
* `AUTHORITATIVE_RANKING_AVAILABLE` remains false until a ranking contract and
  evidence pass;
* `REV2_REUSE_RATIO_REPRODUCIBLE` and
  `RANKING_UNCERTAINTY_PROPAGATION` remain blocked/deferred unless the selected
  deck decision explicitly depends on them;
* if ranking is used as the selection authority, its reproducibility and
  uncertainty gates become prerequisites of the deck-lock task.

## Transition and migration

No existing artifact is rewritten by this decision. The transition is:

```text
current broad research census
    -> exact two-deck scope selection
    -> frozen scope/capability/obligation envelope
    -> M2.5 FINAL
    -> separately authorized M3 vertical slices
    -> later C authority and certification closure
```

The existing `b2_closure`/`b2_closure_v2` distinction remains unchanged.
Candidate-1's missing current Authority binding and B1 citation coverage are
post-M2.5 authority blockers, not reasons to mutate historical contracts.

## Rejected alternatives

### A. Finish all 15,679 candidates before M3

Rejected as a milestone boundary. It confuses a broad research census with
production semantic authority, creates an unbounded serial critical path, and
is not required by the accepted M2.5 roadmap. The full C contract remains
available for later closure and certification.

### B. Mark C PASS with unresolved candidates

Rejected. It directly violates the C contract and would turn incomplete
semantic knowledge into false authority.

### C. Delete or weaken C authority contracts

Rejected. C remains the strict source-grounded semantic closure gate. This ADR
changes ownership of the milestone exit, not C correctness.

### D. Finish only Candidate 1 and Candidate 4 and generalize

Rejected. Canary evidence cannot establish equivalence for other candidates;
applications remain exact and finite.

### E. Freeze scope plus conservative obligations and defer authority

Selected. It preserves exact source identity, fail-closed unsupported behavior,
and certification debt while allowing M3 to implement only explicitly bounded
reusable primitives.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Missing required rule primitive | Freeze reachable capability/decision/information obligations and high-risk/outlier list; require every M3 slice to close its own authority and red cases |
| Later semantic category invalidates a foundation contract | Keep unresolved ledger and scope-impact identity; require versioned ADR/schema evolution and never mutate historical records |
| Deferred C becomes unbounded certification debt | Use reusable theorem families, exact applications, and a finite locked bundle; track every unresolved item and owner |
| False support claim | C unresolved remains nonterminal; lifecycle and bundle certification rules reject unsupported semantics |
| Milestone relabeling | Require exact two-deck manifests, thresholds, scope-impact evidence and exact-head verification before M2.5 FINAL |
| Ambiguous blocked-C wording | Independently review this ADR and explicitly cross-reference the C specification; if the owner intends a blanket M3 prohibition, reject this interpretation rather than silently overriding it |

## Verification requirements

Before accepting this ADR candidate, an independent reviewer must verify from
the live repository:

1. the exact roadmap/scope/acceptance-gate ownership cited here;
2. that C still rejects a PASS closure containing unresolved candidates;
3. that the proposed M2.5 gates are complete for an exact two-deck scope;
4. that no candidate-level Authority or Acceptance is being treated as an
   M2.5 prerequisite without an owning normative source;
5. that M3 entry remains separately authorized and fail-closed; and
6. that M4 certification still requires complete bundle-specific evidence.

Acceptance of this candidate, if later granted, must not itself modify C,
create Authority, create Acceptance, run Domain Audit 55/55, run ranking, lock
a deck pair, or start M3. Those actions require their own exact tasks and
executed gates.

## Non-goals

This ADR does not:

* resolve Candidate 1 or Candidate 4;
* modify the C contract, C artifacts, B1, B2, or Authority contracts;
* add `b2_closure_v2` to Authority source-role registries;
* create Production Authority or Acceptance;
* certify cards, capabilities, interactions, or decks;
* run Domain Audit 55/55 or ranking;
* lock the deck pair;
* implement Magic rules, Card IR, or M3.

## Accepted decision status

```text
M2.5_SCOPE_RESTORATION_DECISION = ACCEPTED_SCOPE_BOUNDARY
M2.5_FINAL                     = NOT_YET
C_PASS                         = BLOCKED, unchanged
M3                            = NOT_AUTHORIZED
M4_CERTIFICATION              = LATER
```
