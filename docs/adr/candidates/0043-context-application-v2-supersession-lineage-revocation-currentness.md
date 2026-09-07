# ADR 0043 (Candidate): ContextApplicationV2 Supersession Lineage, Revocation, and Currentness

- **Status:** candidate
- **Date:** 2026-09-07
- **Proposed permanent number:** 0043
- **Supersedes:** none
- **Superseded by:** none
- **Depends on:** ADR 0042
- **Reviewed baseline:** `2cdf4116ac2c7f490507e1253e18b66fd6d5fb2d`
- **Implementation evidence:** `NOT_RUN`

This is a narrow candidate amendment to ADR 0042. Under
`docs/adr/README.md`, it is informative until an independent review and a
separate acceptance change assign it permanent ADR status. It does not edit
ADR 0042, authorize Slice 5 implementation, or authorize an implementation
plan.

## Context

ADR 0042 freezes the ContextApplicationV2 semantic and accepted-record
identities, the four supersession reasons, same-kind replacement, revocation
nullability, immutable records, and the V3 acceptance subject/event contracts.
It does not fully specify the graph interpretation needed to derive a current
record.

The missing decisions are:

- whether a replacement must share the superseded record's `application_id`;
- whether multiple accepted `cpsr.v2` records may represent one semantic
  `cps.v2` edge; and
- the scope of `authority_revocation` and the exact currentness calculation.

Without these decisions, an implementation could make a different authority
choice based on file order, timestamps, or a convenient graph traversal.

## Decision

### 1. Authority lineage and replacement

The authority-lineage graph contains accepted
`ContextApplicationV2Record` nodes keyed by their complete `cpar.v2`
`record_id`.

A non-null supersession is the directed edge:

```text
superseded_record_id -> replacement_record_id
```

Both endpoints must be existing complete `cpar.v2` record identities and both
endpoint kinds must be exactly `context_application_v2_record`. The source and
replacement must not be the same record.

The edge does **not** require:

```text
superseded.application_id == replacement.application_id
```

The three non-revocation reasons may therefore replace a record with a
different `cpa.v2` semantic application when all existing identity, review,
evidence, and graph checks pass. Same-kind record compatibility is the complete
endpoint relation.

The authority lineage is the directed transitive graph of these exact record-ID
edges. No `lineage_id`, predecessor field, timestamp, filename, or mutable
lineage index is persisted.

The exact currentness grouping key is the complete `cpa.v2` `application_id`
carried by each accepted `cpar.v2` record. This is a currentness grouping key,
not a replacement constraint and not a second persisted identity.

### 2. Semantic supersession edges and accepted revisions

The semantic key of a supersession edge is the complete `cps.v2`
`supersession_id`. Its existing preimage remains authoritative and includes the
exact source/replacement IDs, reason, and canonical source evidence while
excluding acceptance metadata.

Multiple accepted `ContextApplicationV2SupersessionRecord` values are valid
when they bind the same `cps.v2` semantic identity and have distinct complete
`cpsr.v2` accepted-record identities. They are immutable acceptance revisions
of one semantic supersession edge. Each revision must independently pass the
complete V3 event, reviewer, evidence, and standalone source-closure
admission.

The graph materializes exactly one semantic edge for that `cps.v2` and retains
all of its accepted `cpsr.v2` record IDs in canonical complete-CBOR order:

```text
ContextApplicationV2SupersessionEdge = {
    supersession_id: cps.v2,
    accepted_record_ids: tuple[cpsr.v2, ...],
    superseded_record_id: cpar.v2,
    replacement_record_id: cpar.v2 | null,
    reason_code: ContextApplicationV2SupersessionReasonCode,
}
```

No accepted revision is selected by recency or silently discarded. An exact
repeat of a `cpsr.v2` record identity is `DUPLICATE_RECORD_ID`.

`source_evidence_refs` are part of the `cps.v2` preimage. Consequently, two
assertions with the same source and effective replacement but different
semantic evidence are different `cps.v2` edges. After equal-`cps.v2`
acceptance revisions have been grouped, more than one distinct `cps.v2` edge
from the same source record is `MULTIPLE_SUCCESSORS`, even when the distinct
edges name the same replacement record.

The semantic edge key is therefore `cps.v2`, while `cpsr.v2` is retained
acceptance provenance. No timestamp, file order, lexical naming, or newest
selection participates.

### 3. Revocation scope and precedence

`authority_revocation` is **semantic-application-wide** over the exact
`cpa.v2` group of the revocation source record.

For every accepted revocation edge whose source is `A`, define:

```text
revoked_application_ids += { A.application_id }
revoked_record_ids = {
    R.record_id
    for every accepted record R
    where R.application_id is in revoked_application_ids
}
```

This includes every immutable `cpar.v2` revision of the exact semantic
application, including revisions that are not direct graph descendants of the
revocation source. The supersession edge and all historical records remain
readable and immutable; revocation only removes records from currentness
eligibility.

Revocation does not propagate to a different `application_id`, even when that
application is connected by a non-revocation lineage edge. In particular, for
`A[x] -> B[y]`:

- a revocation of any record in group `x` revokes all records in group `x`,
  including `A`, but does not revoke `B` or other records in group `y`;
- a revocation of any record in group `y` revokes all records in group `y`,
  including `B`, but does not retroactively revoke group `x`;
- the historical `x -> y` edge is not deleted in either case.

This is neither record-local nor whole-connected-lineage-wide. It prevents an
unlinked accepted revision of the same semantic application from bypassing
revocation without silently revoking a corrected application with a different
`cpa.v2` identity.

### 4. Deterministic currentness

Let:

```text
accepted_records = all mechanically admitted ContextApplicationV2Record values
replacement_sources = sources of distinct non-revocation cps.v2 edges
```

For each exact `application_id` group:

```text
if application_id is in revoked_application_ids:
    current_records = ∅
else:
    current_records = {
        record in accepted_records with this application_id
        where record.record_id is not in replacement_sources
    }
```

The cardinality rule is closed:

```text
|current_records| = 0  -> valid no-current state
|current_records| = 1  -> that record is current
|current_records| > 1  -> CURRENTNESS_AMBIGUOUS
```

Multiple current records may coexist across distinct `application_id` groups.
There is no current-record bit or persisted current-record index.

An application record or supersession record that fails mechanical admission
never enters `accepted_records` or the graph. A structurally valid but
graph-invalid collection fails as a whole and publishes no currentness result.

## Deterministic examples

`A`, `B`, and `C` denote accepted `cpar.v2` record IDs.

| Input | Result |
|---|---|
| `A` only | `A` is current |
| `A -> B` | `A` is superseded; `B` is current if its group has one eligible record |
| `A -> B -> C` | `A` and `B` are superseded; `C` is current if its group has one eligible record |
| `A -> null` | the exact `A.application_id` group is revoked; it has zero current records |
| `A -> B -> null` | `A` is superseded; `B`'s application group is revoked; the simple chain has zero current records |
| two independent lineages with distinct application IDs | one current record per eligible group may coexist |
| two independent eligible lineages with the same application ID | `CURRENTNESS_AMBIGUOUS` |
| linked historical records for one application | only the final eligible record is current |
| unlinked historical records for one application | `CURRENTNESS_AMBIGUOUS`; no recency choice |
| `A -> null` plus another revision `B` with `B.application_id == A.application_id` | both are revoked; zero current records in that group |
| `A -> null` plus `B` with a different application ID | only A's group is revoked; B may be current |
| two `cpsr.v2` revisions for one `cps.v2` | one semantic edge; both accepted record IDs remain provenance |
| two distinct `cps.v2` edges from A to the same B | `MULTIPLE_SUCCESSORS` |

The `A -> B` rule remains valid whether or not the two records have equal
`application_id` values. A changed `cpa.v2` replacement creates a new
currentness group while the exact record-ID edge preserves the historical
lineage.

## Compatibility and ownership

This candidate preserves, byte-for-byte and semantically, wherever existing
contracts apply:

```text
cpa.v2
cpar.v2
cps.v2
cpsr.v2
asp.v3
ae.v3
all existing JSON Schemas
all existing canonical-CBOR identity preimages
DigestReferenceV1
ReviewEventRefV3
ContextApplicationV2SupersessionInputV2
ContextApplicationV2SupersessionRecordInputV1
```

It adds no `lineage_id`, `current`, `is_latest`, `active_record`, or persisted
currentness index. Currentness is a derived read model over immutable accepted
records and immutable semantic edges.

The candidate does not alter:

- Slice 3 semantic application validation;
- Slice 4 V3 application review admission or its public diagnostics;
- the existing host-free V3 closure path;
- HostBinding V1/V2 contracts or Slice 6 integration;
- Rust DTO/identity ownership;
- C, Task 5 Slice 3B, M3, Magic rules, cards, or production authority.

After this candidate is accepted, implementation may reuse one narrow internal
V3 review-binding seam for application and supersession subjects. The graph
validator remains a separate Slice 5 module. That later work must use
temporary fixtures and must not create production authority artifacts.

## Consequences

Positive consequences:

- semantic supersession and acceptance provenance remain separate;
- corrected applications may cross from one `cpa.v2` to another without
  weakening same-kind record validation;
- duplicate acceptance revisions remain auditable instead of being dropped;
- revocation cannot be bypassed by another accepted revision of the same
  semantic application;
- currentness is deterministic, immutable, and reproducible without a cache;
- no new identity, schema, Rust, or production-artifact surface is needed.

The derived result may contain zero current records for a group. Multiple
eligible records in one group are intentionally an error rather than a
timestamp-based choice. Distinct application groups may produce multiple
current records in one overall result.

## Acceptance boundary

This candidate is not itself normative authority. Candidate acceptance must:

1. independently review the exact lineage, duplicate-`cps.v2`, revocation, and
   currentness clauses above;
2. assign permanent ADR 0043 status through an explicit acceptance change;
3. preserve ADR history by leaving ADR 0042 immutable; and
4. only then permit a separately authorized Slice 5 implementation plan.

No implementation plan or implementation may begin from this candidate alone.

## Status

```text
SLICE5_CONTRACT_RESOLUTION_CONTENT      = PASS

LINEAGE_KEY_RESOLVED                    = YES
DUPLICATE_CPS_ACCEPTANCE_RULE_RESOLVED  = YES
REVOCATION_SCOPE_RESOLVED               = YES
CURRENTNESS_SEMANTICS_RESOLVED          = YES

IDENTITY_CHANGE_REQUIRED                = NO
SCHEMA_CHANGE_REQUIRED                  = NO
PREIMAGE_CHANGE_REQUIRED                = NO

NORMATIVE_AUTHORITY_ESTABLISHED         = NO
DESIGN_APPROVED_AS_ADR_CANDIDATE        = YES
CONTRACT_GAP_SEMANTIC                   = NO
CONTRACT_GAP_NORMATIVE                  = YES

IMPLEMENTATION_PLAN_AUTHORIZED          = NO
IMPLEMENTATION_STARTED                  = NO
```
