# Proposed ADR 0042 Amendment

## ContextApplicationV2 Slice 5 Contract Gap Resolution

**Status:** proposed contract clarification; design-only; pending independent review

**Repository baseline:** `2cdf4116ac2c7f490507e1253e18b66fd6d5fb2d`

**Branch:** `chris/context-application-v2-slice5-supersession-currentness`

**Parent design commit:** `952c95d5c91f594b763c922b443380b6b402c371`

**Implementation authorization:** `NO`

This document resolves the three connected Slice 5 contract questions without
changing the existing serialized identities or schemas. It is a proposed
amendment to ADR 0042, not an edit to the accepted ADR itself. It authorizes no
Slice 5 implementation, implementation plan, production artifact, HostBinding
integration, C change, Task 5 Slice 3B work, or M3 work.

## 1. Scope and decision summary

The amendment closes exactly these gaps:

1. the relation between a superseded and replacement application record;
2. the meaning of multiple accepted `cpsr.v2` records for one `cps.v2`;
3. revocation scope and deterministic current-record derivation.

The decisions are:

```text
Authority lineage relation:
    exact cpar.v2 record-ID edge; same application_id is not required

Semantic supersession edge key:
    cps.v2

Multiple cpsr.v2 records for one cps.v2:
    allowed immutable acceptance revisions; one semantic edge, all record IDs retained

Currentness grouping key:
    exact cpa.v2 application_id

authority_revocation scope:
    semantic-application-wide over the exact cpa.v2 group

Currentness:
    the sole eligible accepted cpar.v2 record in each application_id group
```

The graph edge relation and currentness grouping are deliberately different
keys. The edge connects immutable accepted record revisions; the grouping key
prevents an unlinked accepted revision of the same semantic application from
being selected by recency or file order.

## 2. Repository evidence

The relevant accepted and executable surfaces were inspected at the requested
baseline:

- ADR 0042 §§5.1–5.3 defines `cpa.v2`, `cpar.v2`, `cps.v2`, and `cpsr.v2`;
  the four reasons; same-kind endpoints; null replacement for revocation;
  self-edge/cycle prohibitions; and immutable records.
- ADR 0042 §8.2 defines the exact supersession
  `AcceptanceSubjectPayloadV3` without acceptance metadata.
- ADR 0042 §§9–10 define the V2 review policy and standalone V3 closure.
- `python/src/mtgml/authority.py` and
  `crates/mtgml-persistence/src/authority.rs` implement the same structural
  identity/preimage split. Neither carries a persisted lineage or current bit.
- `scripts/context_application_v2_review_admission.py` is application-record
  specific and already has the accepted Slice 4 role, evidence, event, and
  closure behavior.
- `scripts/context_application_v2_resolver.py` already accepts a typed
  application or supersession subject for
  `expected_acceptance_source_closure_v3(...)` and rejects host/container
  cycle sources on the standalone path.
- `scripts/authority_validator.py` provides historical V1 behavior: one
  successor per source, cycle rejection, and current application facts grouped
  by semantic application ID. This is a V1 precedent, not a V2 authority.
- `scripts/authority_v2_validator.py` provides HostBinding-specific graph
  behavior only. It is not used to decide ContextApplicationV2 semantics.
- The V2 authority fixture has no accepted supersession graph, so it cannot
  resolve the missing lifecycle rules by example.

ADR 0042's `CONTRACT_GAPS_REMAINING=0` status describes the contract-plumbing
decisions made by that ADR. Its §18 leaves the supersession validator as a
later slice. This document adds the missing evaluation semantics required by
that later slice; it does not contradict or silently reinterpret ADR 0042.

## 3. Proposed normative addition to ADR 0042

If independently reviewed and accepted, add the following section after ADR
0042 §5.3. The wording below is the contract resolution; it is not an
implementation plan.

### 5.4 Slice 5 lineage, accepted supersession revisions, and currentness

#### 5.4.1 Application-record nodes and lineage edges

The graph node for ContextApplicationV2 currentness is an accepted
`ContextApplicationV2Record`, keyed by its complete `cpar.v2` `record_id`.
The node retains its exact `cpa.v2` `application_id` for currentness grouping.

A non-null supersession is a directed edge:

```text
superseded_record_id -> replacement_record_id
```

Both endpoints are complete `cpar.v2` record identities and both endpoint
kinds are exactly `context_application_v2_record`. The edge does not require
the two records to have equal `application_id` values.

The authority lineage is the directed transitive graph of these exact record-ID
edges. No `lineage_id`, predecessor field, timestamp, or mutable lineage index
is persisted. A replacement with a different `cpa.v2` is valid when all other
record, identity, review, evidence, and graph invariants pass.

This preserves the existing distinction:

```text
cpa.v2   = exact semantic application
cpar.v2  = accepted immutable record revision
```

The three non-revocation reasons may therefore replace a record whose exact
semantic application preimage changed. Same-kind compatibility is the complete
endpoint relation; same-application equality is not an additional invariant.

#### 5.4.2 Semantic edge identity and accepted record revisions

The semantic key of a supersession edge is the complete `cps.v2`
`supersession_id`. It is recomputed from the existing
`ContextApplicationV2SupersessionInputV2` preimage and excludes acceptance
metadata.

Multiple `ContextApplicationV2SupersessionRecord` values are allowed when they
have the same `cps.v2` semantic identity and distinct `cpsr.v2` accepted-record
identities. Such values are immutable acceptance revisions of one semantic
supersession edge, not competing successors. Every revision must independently
pass the complete Slice 5 mechanical admission, including its own exact
`ReviewEventRefV3`, V3 event, reviewer policy, evidence, and source closure.

The graph contains one semantic edge for that `cps.v2` key and retains all
accepted `cpsr.v2` record IDs in canonical order as provenance:

```text
ContextApplicationV2SupersessionEdge = {
    supersession_id: cps.v2,
    accepted_record_ids: tuple[cpsr.v2, ...],
    superseded_record_id: cpar.v2,
    replacement_record_id: cpar.v2 | null,
    reason_code: ContextApplicationV2SupersessionReasonCode,
}
```

No accepted revision is silently discarded, selected by recency, or
deduplicated away. An exact repeat of the same `cpsr.v2` record identity is a
duplicate accepted record and fails. Two different semantic `cps.v2` edges
from the same superseded `cpar.v2` source are competing successors and fail,
even when they happen to name the same replacement record.

#### 5.4.3 Successor uniqueness and cycles

After equal-`cps.v2` accepted revisions have been grouped as one semantic edge,
each superseded `cpar.v2` record may have at most one distinct semantic
successor edge. The following are invalid:

```text
A -> B and A -> C
A -> B and A -> B with a different cps.v2 semantic payload
```

The following is valid only as multiple accepted revisions of the same edge:

```text
cpsr/1: cps.v2 = X, A -> B
cpsr/2: cps.v2 = X, A -> B
```

Every directed cycle is rejected. Cycle detection operates on the canonical
successor map keyed by complete `cpar.v2` record identities. A diagnostic path,
if exposed, is normalized to its smallest canonical rotation, and the smallest
normalized cycle is reported.

#### 5.4.4 Currentness grouping key

The exact currentness group key is the complete `cpa.v2` `application_id` on an
accepted application record. This is the semantic application identity, not an
event ID, accepted record ID, filename, timestamp, or inferred lineage label.

The graph edge remains record-to-record even when a replacement has a different
`application_id`. This permits a semantic correction or source/model revision
to produce a new semantic application while preserving an exact historical
edge from the old accepted record.

For each application-ID group, currentness is evaluated only after every
application record and every candidate supersession record has been
mechanically admitted and the complete graph has passed structural validation.

#### 5.4.5 Revocation scope

`authority_revocation` is semantic-application-wide over the exact
`cpa.v2` application-ID group of its `superseded_record_id`.

If an accepted revocation edge names record `A`, then every accepted
`ContextApplicationV2Record` whose `application_id == A.application_id` is
revoked for currentness, including other immutable `cpar.v2` revisions that
are not direct graph descendants of `A`.

Revocation does not propagate to records with another `application_id`, even if
those records are connected by a non-revocation supersession edge. A changed
replacement application is governed by its own group and any revocation edge
that names that group.

This scope prevents an unlinked accepted revision of the same semantic
application from bypassing an authority revocation. It also avoids making a
revocation of one semantic application silently revoke a corrected semantic
application with a different `cpa.v2` identity.

#### 5.4.6 Derived currentness

Define:

```text
replacement_sources = sources of semantic edges with non-null replacement
revocation_anchors   = sources of semantic edges with reason authority_revocation
revoked_application_ids = {
    record.application_id
    for record in revocation_anchors
}
revoked_record_ids = {
    record.record_id
    for record in all accepted application records
    if record.application_id in revoked_application_ids
}
```

An accepted application record is eligible only when:

```text
record.record_id not in replacement_sources
record.record_id not in revoked_record_ids
```

For each `application_id` group:

```text
one eligible record  -> current
zero eligible records -> valid group with no current record
more than one         -> CURRENTNESS_AMBIGUOUS
```

`superseded_record_ids` in the derived read model is the set of
`replacement_sources`. `revoked_record_ids` is the semantic-application-wide
revocation set above. These sets may overlap when a record is both the source
of a non-null replacement and belongs to an application group revoked by a
separate accepted revocation edge; currentness excludes either status.

The result is derived afresh from immutable inputs. No persisted `current`,
`is_latest`, `active_record`, or currentness index exists.

#### 5.4.7 Determinism

The graph and currentness result do not depend on input order, file order,
filesystem enumeration, timestamps, mtime, commit time, username, absolute
path, randomness, or network access. All public tuples and all diagnostic
details use complete canonical CBOR ordering. A rejected candidate or rejected
graph contributes nothing to the returned read model.

### 5.5 Amendment effect

This addition changes only the semantic interpretation of the already existing
in-memory graph evaluation. It does not change:

```text
cpa.v2
cpar.v2
cps.v2
cpsr.v2
asp.v3
ae.v3
ContextApplicationV2SupersessionInputV2
ContextApplicationV2SupersessionRecordInputV1
AcceptanceSubjectPayloadV3
ReviewAcceptanceEventInputV3
ReviewAcceptanceEventLeafV3
ReviewEventRefV3
DigestReferenceV1
```

No JSON Schema, canonical-CBOR preimage, digest envelope, Rust DTO, or
production authority artifact changes are required.

## 4. Exact answers to the three resolved questions

### 4.1 Authority lineage

For `A -> B`, the required relation is:

```text
A.record_id is an accepted cpar.v2 record ID
B.record_id is an accepted cpar.v2 record ID
the cps.v2 edge names exactly A.record_id and B.record_id
both endpoint kinds are context_application_v2_record
A.record_id != B.record_id
```

There is no requirement that:

```text
A.application_id == B.application_id
```

The exact graph endpoint key is `record_id`. The exact currentness grouping key
is `application_id`. A graph path is the authority lineage; no additional
persisted lineage identity is introduced.

### 4.2 Duplicate `cps.v2` acceptance

Multiple accepted `cpsr.v2` records for one `cps.v2` are allowed only as
immutable accepted-record revisions of the same semantic edge. They must have
distinct complete `cpsr.v2` IDs and each must pass mechanical admission.

The graph edge is keyed by `cps.v2`, not `cpsr.v2`. All `cpsr.v2` record IDs are
retained in one canonical provenance tuple. The graph does not pick one,
deduplicate away one, or use an event timestamp. An exact duplicate
`cpsr.v2` record ID is rejected as `DUPLICATE_RECORD_ID`. Distinct `cps.v2`
edges from one source are rejected as `MULTIPLE_SUCCESSORS`.

### 4.3 Revocation and currentness

Revocation is semantic-application-wide, keyed by the exact `cpa.v2`
`application_id` of the revocation source. It is neither record-local nor
whole-connected-lineage-wide.

Currentness is the sole eligible accepted `cpar.v2` record in each
`application_id` group after non-null successor sources and revoked groups are
excluded. Zero is valid. More than one is a deterministic ambiguity error.
Multiple current records may coexist across distinct application-ID groups.

## 5. Deterministic scenarios

`A`, `B`, and `C` below denote accepted `cpar.v2` record IDs. The table assumes
all records and supersession records have already passed mechanical admission.

| Scenario | Derived result |
|---|---|
| `A` only, no supersession | `current=(A)` |
| `A -> B`, same or different `application_id` | `A` is superseded; `B` is current if its group has no other eligible record |
| `A -> B -> C` | `A` and `B` are superseded; `C` is current if its group has no other eligible record |
| `A -> null` | the exact `A.application_id` group is revoked; `current=()` for that group |
| `A -> B`, `B -> null` | `A` is superseded; `B`'s application group is revoked; no current record in the simple chain |
| two independent lineages with distinct application IDs | one current record per group may coexist |
| two independent lineages with the same application ID | more than one eligible record; `CURRENTNESS_AMBIGUOUS` |
| multiple historical records with one application ID and one valid replacement chain | only the final eligible record is current |
| multiple unlinked historical records with one application ID | `CURRENTNESS_AMBIGUOUS`; no recency choice |
| `A -> null` plus another accepted revision `B` with `B.application_id == A.application_id` | both are in `revoked_record_ids`; zero current records for that group |
| `A -> null` plus `B` with a different application ID | only A's group is revoked; B may be current |
| two `cpsr.v2` revisions for one `cps.v2` | one successor edge; both accepted `cpsr.v2` IDs retained |

A replacement with a changed `cpa.v2` therefore starts a new semantic
application group for currentness, while the record-ID edge preserves the
historical lineage relation. This is explicit and does not depend on event
metadata or ordering.

## 6. Admission and implementation boundary after amendment

The amendment does not authorize implementation yet. When separately
authorized after independent review, the implementation must preserve the
following shape.

### 6.1 Slice 4 reuse

Extract one narrow internal V3 mechanical-review-binding seam from the existing
application admission module or into an equally narrow internal module. It
must own only:

- typed `ReviewEventRefV3` resolution;
- exact subject-kind equality;
- complete six-field `DigestReferenceV1` equality;
- standalone V3 source-closure reconstruction and exact-set comparison;
- reviewer-roster resolution;
- exact roster-role equality and duplicate reviewer rejection;
- the ordered mandatory V2 role policy;
- review-mode closure and review-evidence integrity;
- stable V3 cause mapping.

The existing public
`ContextApplicationV2ReviewAdmissionValidator.admit(...)` interface, result,
diagnostics, and validation order remain unchanged. Application semantic
validation remains outside the shared seam. Supersession graph validation
remains outside the shared seam.

### 6.2 Supersession admission

The new Slice 5 admission path must:

1. require the exact typed `ContextApplicationV2SupersessionRecord`;
2. recompute `cps.v2` with
   `ContextApplicationV2SupersessionInputV2.identity()`;
3. recompute `cpsr.v2` with
   `ContextApplicationV2SupersessionRecordInputV1.identity()` using the exact
   `ReviewEventRefV3`;
4. enforce the closed reason/replacement and exact endpoint-kind rules;
5. reconstruct the exact supersession `AcceptanceSubjectPayloadV3`;
6. compare every field of the complete `DigestReferenceV1`;
7. call the existing host-free
   `expected_acceptance_source_closure_v3(...)` path;
8. reuse the existing V3 reviewer/evidence policy; and
9. return no success result until every mechanical check passes.

The currentness evaluator must receive typed raw application and supersession
records and perform admission itself. It must not accept an unchecked
`already_validated=True`, `accepted=True`, or `trusted=True` argument. Private
frozen admission wrappers are permitted for in-process proof flow; no second
persisted authority type is introduced.

### 6.3 Deterministic validation order

The eventual implementation uses this stable order:

1. validate input collection types;
2. validate application-record types and duplicate `cpar.v2` record IDs;
3. admit all application records in canonical record-ID order;
4. validate supersession-record types and closed fields;
5. recompute all `cps.v2` identities;
6. recompute all `cpsr.v2` identities;
7. admit all supersession V3 bindings in canonical `cpsr.v2` order;
8. group equal `cps.v2` revisions and reject exact duplicate `cpsr.v2` IDs;
9. verify superseded-record existence;
10. verify replacement existence and exact same-kind endpoint values;
11. reject self-supersession;
12. reject multiple distinct semantic successors per source;
13. reject cycles;
14. derive replacement-source and semantic-application-wide revocation sets;
15. derive current candidates by exact `application_id` group;
16. reject multiple eligible candidates in one group;
17. canonicalize the immutable result.

Within each stage, canonical complete-CBOR identity ordering determines the
single reported failure. No error category is obtained by parsing exception
text.

## 7. Derived read model and stable error surface

The eventual public read model is frozen and contains only derived immutable
values:

```text
ContextApplicationV2CurrentnessResult {
    current_record_ids: tuple[AuthorityIdentityV1, ...],
    superseded_record_ids: tuple[AuthorityIdentityV1, ...],
    revoked_record_ids: tuple[AuthorityIdentityV1, ...],
    successor_edges: tuple[ContextApplicationV2SupersessionEdge, ...],
}
```

Canonical ordering is:

- every record-ID tuple: complete canonical CBOR encoding of the identity;
- `accepted_record_ids` inside an edge: complete canonical CBOR encoding;
- `successor_edges`: source record ID, replacement record ID/null, reason, then
  semantic `cps.v2` identity by complete canonical CBOR encoding.

The result has no raw artifacts, review event JSON, mutable dictionaries,
current bit, latest bit, or persisted index.

The stable error categories are:

```text
CURRENTNESS_INPUT_INVALID
DUPLICATE_RECORD_ID
APPLICATION_REVIEW_ADMISSION_FAILED

SUPERSESSION_INPUT_INVALID
SUPERSESSION_IDENTITY_MISMATCH
SUPERSESSION_RECORD_IDENTITY_MISMATCH
SUPERSESSION_REVIEW_ADMISSION_FAILED
SUPERSESSION_REASON_INVALID
SUPERSESSION_REPLACEMENT_INVALID

SUPERSEDED_RECORD_UNKNOWN
REPLACEMENT_RECORD_UNKNOWN
SELF_SUPERSESSION
MULTIPLE_SUCCESSORS
SUPERSESSION_CYCLE
CURRENTNESS_AMBIGUOUS
```

There is no `DUPLICATE_SUPERSESSION_ID` failure for distinct accepted
`cpsr.v2` revisions of one valid `cps.v2`; that case is explicitly allowed.
Exact repeated accepted record identities remain `DUPLICATE_RECORD_ID`.

Each error carries stable code/location and structured IDs or inner cause data
where relevant. It never claims substantive human-review correctness,
production authority, C acceptance, or semantic Magic correctness.

## 8. Mutation safety, information boundaries, and Rust boundary

All future admission and graph evaluation is read-only. It builds local
candidate maps and tuples and publishes no partial result. A rejected
application or supersession record cannot alter currentness, and a rejected
graph cannot update a cache.

Rejection must preserve:

```text
application records
supersession records
review event refs
source artifacts
reviewer rosters
review evidence
candidate universe
base authority
IDs
history
C
filesystem artifacts
```

Slice 5 does not validate or consume `ApplicationHostBindingV2`, `hbc.v1`
claims, `interaction_review_authority.v2`, or HostBinding currentness. That
work remains Slice 6.

No Rust policy implementation is required. Rust structural DTO and identity
parity already exists; this amendment adds no wire contract that would require
Rust currentness code. Adding Rust merely for symmetry is out of scope.

## 9. Later acceptance test design

No tests are written or run in this contract-resolution stage. After the
amendment is independently accepted and implementation is separately
authorized, temporary-fixture tests must include:

### Positive cases

```text
P1  A only -> A current
P2  A -> B with equal application_id -> B current
P3  A -> B with different application_id -> valid B current
P4  A -> B -> C -> C current
P5  A -> null -> valid semantic-application revocation
P6  A -> B -> null -> no current in the simple chain
P7  two independent lineages with distinct application IDs -> both current
P8  linked historical revisions for one application -> one current
P9  two cpsr revisions for one cps -> one edge and both cpsr IDs retained
P10 input permutations -> byte/value-equivalent result
P11 accepted supersession with exact subject, full digest, closure, roles, evidence
```

### Negative cases

```text
N1  unknown superseded record
N2  unknown replacement record
N3  self-supersession
N4  competing successors from one source
N5  two-node cycle
N6  longer cycle
N7  authority_revocation with replacement
N8  non-revocation with null replacement
N9  wrong endpoint kind
N10 cps.v2 identity mismatch
N11 cpsr.v2 identity mismatch
N12 wrong V3 subject kind
N13 wrong complete asp.v3 digest reference
N14 stale ReviewEventRefV3
N15 stale review evidence
N16 wrong roster or reviewer role tuple
N17 missing mandatory V2 role
N18 missing source-closure binding
N19 extra source-closure binding
N20 unauthorized HostBinding source
N21 exact duplicate cpar.v2 record ID
N22 exact duplicate cpsr.v2 record ID
N23 two unlinked eligible cpar.v2 revisions sharing one application_id
N24 rejected candidate does not affect any derived result
N25 graph result and diagnostics are input-order independent
```

The semantic-application-wide revocation tests must include both:

```text
A -> null with B.application_id == A.application_id
A -> null with B.application_id != A.application_id
```

to prove that the scope is neither record-local nor whole-lineage-wide.

Mutation tests snapshot typed projections and temporary-repository file
digests before and after repeated rejected admission/evaluation. No production
authority, acceptance event, supersession record, or current-record file is a
test fixture.

## 10. Self-review of the amendment

| Axis | Result | Reason |
|---|---|---|
| ADR 0042 identities/preimages | `PASS` | all existing identity families and preimage inputs remain unchanged |
| ADR 0042 reason/replacement rules | `PASS` | the four reasons and null/non-null relationship are preserved |
| Slice 4 reuse | `PASS` | the amendment requires one shared mechanical V3 seam and preserves the public application validator |
| V1 historical behavior | `PASS` as precedent | record-ID successors, cycles, and application-ID current grouping are retained as clarified V2 semantics without copying V1 identity rules |
| HostBinding boundary | `PASS` | HostBinding cannot determine lineage, revocation, or currentness |
| determinism | `PASS` | canonical IDs, canonical edges, no recency, and normalized cycle diagnostics are explicit |
| fail-closed currentness | `PASS` | rejected records never enter the graph; zero is explicit and multiple eligible candidates fail |
| revocation safety | `PASS` | an authority revocation covers every revision of the exact semantic application and no changed application ID |
| mutation safety | `PASS` | all evaluation is local/read-only; no persisted index or partial cache update |
| production-artifact prohibition | `PASS` | no production bytes are created or required |
| Rust boundary | `PASS` | no new cross-language policy contract is introduced |

The amendment is minimal because it adds only graph interpretation and derived
read-model semantics. It does not add a persisted field, a new identity, a new
schema, or a second authority.

## 11. Contract-resolution status and stop

The proposed amendment resolves the previously reported design ambiguity. It
is not yet an accepted ADR change; independent review is required before any
implementation planning.

```text
LINEAGE_KEY_RESOLVED=YES
DUPLICATE_CPS_ACCEPTANCE_RULE_RESOLVED=YES
REVOCATION_SCOPE_RESOLVED=YES
CURRENTNESS_SEMANTICS_RESOLVED=YES

IDENTITY_CHANGE_REQUIRED=NO
SCHEMA_CHANGE_REQUIRED=NO
CONTRACT_GAP_REMAINING=NO

IMPLEMENTATION_PLAN_AUTHORIZED=NO
IMPLEMENTATION_STARTED=NO
```

The mandatory next state is:

```text
STOP
wait for independent review of this contract-resolution document
do not write an implementation plan
do not implement Slice 5
```
