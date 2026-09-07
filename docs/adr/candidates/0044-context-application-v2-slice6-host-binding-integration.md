# ADR 0044 Candidate: ContextApplicationV2 Slice 6 Host-Binding Integration

- **Status:** proposed; unaccepted ADR candidate
- **Date:** 2026-09-07
- **Owners:** architecture-maintainer, rules-maintainer, conformance-maintainer, information-safety-reviewer
- **Supersedes:** none
- **Superseded by:** none
- **Depends on:** ADR 0042, ADR 0043
- **Reviewed design baseline:** `2ba1d36585015a06efe741713536801b86b24bfd`
- **Implementation evidence:** `NOT_RUN`
- **Acceptance status:** independent review and acceptance required

This candidate records the three normative clarifications required before
ContextApplicationV2 Slice 6 can be implemented. It is informative until an
explicit acceptance change assigns the next permanent ADR number. It does not
authorize an implementation plan, production authority artifacts, Buckle-Up
review, C changes, Task 5 Slice 3B, or M3.

## Context

The Slice-6 design composes two independently authoritative lifecycle domains:

```text
ContextApplication currentness: cpar.v2 / cps.v2 / cpsr.v2
HostBinding currentness:        hbc.v1 / hbcr.v1 / hbcs.v1
```

Slice 5, accepted by ADR 0043, owns ContextApplication record admission,
supersession, revocation, and currentness. Existing HostBinding authority
machinery owns member-atomic claim admission, claim supersession, correlated
source joins, and current hbc claim derivation. Slice 6 must only compose the
two derived read models.

The exact-head design review at commit
`2ba1d36585015a06efe741713536801b86b24bfd` confirmed three unresolved
contract decisions.

### G1: event closure versus container closure

ADR 0042 §10.1 describes optional HostBinding authority and claim-record
bindings as part of `ExpectedAcceptanceSourceClosureV3`. The accepted Slice-4
design and public resolver path are host-free: Slice-4 reconstructs the V3
event closure without HostBinding inputs and rejects an extra HostBinding
source. The current private container path can pass HostBinding sources back
into an application event closure even though the public event path does not.

This creates two incompatible meanings for the source closure of the same
`ae.v3` event.

### G2: required HostBinding member applicability

The provisional HostBinding checklist says every current V1 Relation, Domain,
and Context Application requires an application link. The executable
HostBinding V2 validator and tests require links only for verified
`cross_deck` + `directional_binary` candidate members. They explicitly allow
non-cross-deck applications without links and require only the applicable
members of a mixed application.

The checklist is registered as `process / provisional`; it cannot override an
executable contract. ADR 0042 defines the additive cpa.v2 link but does not
state the V2 required-member predicate. The missing invariant must be made
normative before implementation.

### G3: unused and historical HostBinding links

The existing container accepts a nullable
`host_binding_authority_v2_binding`, and the current resolver tolerates a
non-null binding when the link collection is empty. The accepted contracts do
not state whether the following are valid, historical-only, or invalid:

- HostBinding authority present with no link and no current required member;
- a link for a known cpa group with no current cpar record;
- a historical link whose hbc claim has since been superseded or revoked.

Slice 6 needs a closed policy that distinguishes retained history from current
authority eligibility.

## Decision

This candidate proposes the following three clarifications.

### 1. Keep `ae.v3` event closure host-free

The V3 acceptance-event closure remains:

```text
ae.v3 event closure
    = existing Slice-4 ContextApplication/Supersession closure
    - HostBinding authority and claim-record bindings
```

The public Slice-4 event admission interface remains unchanged. It continues
to call the host-free `expected_acceptance_source_closure_v3` path and rejects
HostBinding sources as extra bindings. HostBinding must not be injected into a
V3 review-event source list through a private container path.

The completed context container has a separate additive closure:

```text
ContextApplicationAuthorityV2 container closure
    = static container bindings
    + V3 event leaf bindings
    + host-free V3 event closures
    + HostBinding authority provenance
    + referenced HostBinding claim-record provenance
```

HostBinding provenance is therefore container-level evidence for the
composition result. It is not part of the acceptance event's source claim and
does not alter `ae.v3`, `asp.v3`, `cpar.v2`, `cps.v2`, or `cpsr.v2` identities.

The resolver must express these as two distinct operations. A container
closure helper may add HostBinding provenance after independently validating
each host-free event closure, but it must not pass HostBinding bindings into
the event closure reconstruction.

### 2. Require HostBinding only for verified cross-deck directional members

For each current ContextApplication member, the evaluator resolves the exact
Candidate/SourceInstance through the existing source resolver. A HostBinding
claim is required exactly when the verified candidate record contains:

```text
scope    == "cross_deck"
relation == "directional_binary"
```

The predicate uses verified candidate fields. It does not inspect capability
names, card names, Oracle prose, candidate ordering, file ordering, discovery
ordering, timestamps, or co-occurrence.

The exact cpa-level link rules are:

```text
required member set non-empty:
    exactly one ApplicationHostBindingV2 link
    exact member-key union

required member set empty:
    no ApplicationHostBindingV2 link
```

The non-applicable member set is every member outside the verified predicate.
An extra claim for a non-applicable current member fails exact closure. The
existing non-empty `host_binding_claim_ids` DTO rule remains unchanged; an
empty required set is represented by no link, not by an empty link.

This clarification preserves the existing executable HostBinding behavior and
does not add a persisted applicability field.

The same applicability predicate applies to historical-only links. A link's
claim/member union must equal the verified required subset of the exact
admitted cpa member set, not the full member set. Current versus historical-only
status controls whether the link may qualify authority; it does not change the
member closure that the link must satisfy.

### 3. Define unused and historical link policy

The following closed policy applies to `ApplicationHostBindingV2` and the
top-level HostBinding authority binding:

| Situation | Decision |
| --- | --- |
| A known cpa group has no current cpar record and has a mechanically valid link | retain as `historical_only`; it never qualifies current authority |
| A current cpa link references an unknown hbc claim | reject `HOST_CLAIM_UNKNOWN` |
| A current cpa link references a superseded or revoked hbc claim | reject `HOST_CLAIM_NOT_CURRENT` |
| A historical-only link references an unknown or unadmitted hbc claim | reject `HOST_CLAIM_UNKNOWN` |
| A historical-only link references a known superseded or revoked hbc claim | retain `historical_only`; preserve the exact historical hbc identity and status; do not qualify current authority |
| A historical-only link references a known current hbc claim | retain `historical_only`; do not qualify current authority |
| Any current or historical link exists | `host_binding_authority_v2_binding` is required |
| HostBinding authority is present, but no link and no current cpa has a required member | reject `HOST_AUTHORITY_BINDING_UNEXPECTED` |
| A current cpa has a non-empty required member set but no link | reject the application host-binding closure |
| A link targets a cpa absent from all mechanically admitted ContextApplication records | reject `APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION` |
| A link targets a revoked/no-current cpa | retain only as `historical_only` after all link and claim checks pass |

Historical links remain inspectable input. They do not enter
`qualified_current_application_record_ids` or `current_host_claim_ids`.
Every historical link still requires exact cpa identity, member-key union,
observed host relationship, and source/snapshot provenance. No historical link
or claim is deleted, rewritten, or revived as current authority.

The HostBinding admission seam therefore exposes a derived, non-persisted
read model containing:

```text
admitted_claims_by_id: hbc.v1 -> claim
current_claims_by_id: hbc.v1 -> claim
current_claims_by_member: member -> hbc.v1
claim_record_status_by_record_id: hbcr.v1 -> current | superseded | revoked
claim_record_ids_by_claim_id: hbc.v1 -> tuple[hbcr.v1, ...]
```

Current cpa links consult `current_claims_by_member`. Historical-only links
consult `admitted_claims_by_id` and retain the status of each exact referenced
claim record in the derived result. Status is never aggregated ambiguously
from multiple hbcr records onto the hbc semantic identity.

## Composition invariants

The clarifications compose with existing contracts as follows.

### Independent currentness

Slice 6 calls `ContextApplicationV2CurrentnessEvaluator` first. HostBinding
claims cannot change cpar/cps/cpsr currentness. A currentness failure prevents
HostBinding source evaluation and publishes no partial result.

The existing HostBinding authority seam remains the only owner of hbcr/hbcs
current-claim semantics. Slice 6 consumes its frozen current-claim read model;
it does not select claims by filename, timestamp, input order, or lexical
recency.

### cpa-level link identity

`ApplicationHostBindingV2` remains keyed to complete `cpa.v2`, not `cpar.v2`.
The same link remains valid across acceptance-record revisions of one exact
cpa, because cpar acceptance metadata is not part of cpa semantic identity or
HostBinding claim identity.

For a cross-application replacement `A[x] -> B[y]`:

- a link for `x` can remain historical-only;
- `y` requires an independent exact link when its verified required set is
  non-empty;
- no link or claim transfers automatically across the lineage edge;
- an hbc claim is reusable only when its complete member key is identical and
  its observed relationship satisfies each use.

### Host expectation

For every required member:

```text
claim.observed_host_relationship
    == member.context_binding_v1[3]
```

The member binding remains the sole reviewed host semantic expectation. A
historical SourceContext value does not override it. HostBinding evidence does
not rewrite ContextApplication semantics or establish an interaction
conclusion.

### Exact shared snapshots

When HostBinding participates, the context container and HostBinding authority
must bind the same complete immutable snapshots:

```text
context.base_authority_v1_binding
    == host source role base_authority_v1

context.candidate_universe_binding
    == host source role candidate_universe
```

Role, path, schema, and raw digest all participate in equality. Recency and
compatible rebasing are not permitted.

## Validation and diagnostic precedence

The future Slice-6 evaluator uses this order:

1. validate the exact typed container and canonical structural collections;
2. detect duplicate cpar/cpsr/link IDs in canonical identity order;
3. evaluate Slice-5 currentness;
4. resolve current members and derive the verified required-member sets;
5. classify link targets as current or historical-only;
6. require or reject the HostBinding authority binding according to G3;
7. admit the HostBinding authority and compare shared snapshots;
8. validate claim existence, currentness, exact member keys, and observed host
   relationships;
9. validate exact container-level HostBinding provenance while keeping every
   `ae.v3` event closure host-free;
10. publish the frozen derived result.

Errors carry stable codes and structured IDs. The minimum closed vocabulary is:

```text
HOST_INTEGRATION_INPUT_INVALID
APPLICATION_CURRENTNESS_FAILED
HOST_AUTHORITY_BINDING_REQUIRED
HOST_AUTHORITY_BINDING_UNEXPECTED
HOST_AUTHORITY_INVALID
HOST_AUTHORITY_CROSS_SNAPSHOT_MISMATCH
APPLICATION_HOST_BINDING_INVALID
APPLICATION_HOST_BINDING_DUPLICATE
APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION
HOST_CLAIM_UNKNOWN
HOST_CLAIM_NOT_CURRENT
HOST_MEMBER_SET_MISMATCH
HOST_RELATIONSHIP_MISMATCH
HOST_BINDING_AMBIGUOUS
HOST_SOURCE_CLOSURE_MISMATCH
```

No error category is produced by parsing exception messages. A currentness
failure always precedes HostBinding source evaluation. A rejected evaluation
mutates no authority input, source file, ID, or history.

## Compatibility and impact

If accepted, this clarification changes semantic composition only. It does
not change:

```text
cpa.v2, cpar.v2, cps.v2, cpsr.v2
asp.v3, ae.v3
DigestReferenceV1, ReviewEventRefV3
existing HostBinding identities
existing JSON Schemas
canonical-CBOR preimages
Rust DTOs
```

It adds no `current` bit, active index, lineage field, application record ID
to hbc claims, applicability column, or player-facing API.

The change has no effect on authoritative engine state, RNG, replay,
checkpoint, state hashing, Decision protocol, PlayerObservation,
PlayerInformationState, or ML trajectory schemas.

## Alternatives considered

### Permit HostBinding in `ae.v3` event closure

Rejected. It would change the host-free Slice-4 admission meaning and could
reinterpret already existing V3 event closure expectations.

### Require HostBinding for every ContextApplication member

Rejected. It conflicts with the executable HostBinding validator and tests,
which explicitly permit non-cross-deck applications without links and require
only applicable members in mixed applications.

### Add a persisted applicability field

Rejected. The verified candidate record already provides the required
predicate. A new field would widen schemas and identity preimages without
evidence of necessity.

### Reject all links for cpa groups without a current cpar

Rejected. Immutable authority history remains inspectable, and cpa-level links
are not cpar-level currentness records. The closed historical-only status
preserves auditability while preventing authority qualification.

### Discard or rewrite stale hbc claims on historical links

Rejected. A historical link must preserve the exact hbc identity that it
actually referenced. Discarding the link loses audit history; rewriting it to
a later hbc claim invents a historical relationship. The claim remains
historical-only and cannot qualify current authority.

## Evidence and follow-up

Verified evidence at the design baseline includes:

- ADR 0042 §10.1 and §11 for V3/container closure and shared snapshots;
- ADR 0043 for cpa-group currentness and revocation scope;
- `scripts/context_application_v2_review_binding.py` and
  `scripts/context_application_v2_review_admission.py` for the host-free
  public V3 path;
- `scripts/context_application_v2_resolver.py` for the current private
  container HostBinding path;
- `scripts/context_application_v2_supersession.py` for Slice-5 ownership;
- `scripts/authority_v2_validator.py` and
  `scripts/authority_host_binding.py` for existing HostBinding lifecycle and
  source ownership;
- `python/tests/test_authority_v2_validator.py` for the verified
  cross-deck/directional required-member behavior; and
- exact-head design review at `2ba1d36585015a06efe741713536801b86b24bfd`.

Acceptance of this candidate must be a separate change that:

1. assigns the then-current permanent ADR number;
2. records independent review evidence with G1, G2, and G3 closed;
3. updates the provisional HostBinding checklist wording;
4. preserves ADR 0042 identities and schemas; and
5. explicitly authorizes a later Slice-6 implementation plan only after
   acceptance.

Until then:

```text
ADR_0044_CANDIDATE                 = PROPOSED
NORMATIVE_AUTHORITY_ESTABLISHED   = NO
SLICE_6_IMPLEMENTATION_PLAN       = NOT_AUTHORIZED
SLICE_6_IMPLEMENTATION             = NOT_AUTHORIZED
BUCKLE_UP_CANARY                   = NOT_AUTHORIZED
TASK_5_SLICE_3B                    = BLOCKED
M3                                  = BLOCKED
```
