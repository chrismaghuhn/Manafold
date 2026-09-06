# Manafold — ContextApplicationV2 Slice 5

## Supersession, Revocation, Lineage, and Deterministic Currentness Design

**Status:** design-only; blocked before implementation planning by an accepted-contract gap

**Design baseline:** `2cdf4116ac2c7f490507e1253e18b66fd6d5fb2d`

**Design branch:** `chris/context-application-v2-slice5-supersession-currentness`

**Implementation authorization:** `NO`

**Production acceptance:** `NOT AUTHORIZED`

This document is the Stage B design artifact for ContextApplicationV2 Slice 5.
It does not change a persisted contract, schema, identity preimage, authority
record, Acceptance Event, checklist, C artifact, HostBinding contract, or Rust
policy. It must not be converted into an implementation plan until the
contract gap recorded in Section 7 is resolved by an accepted contract change
or an explicit accepted clarification.

## 1. Scope and non-goals

Slice 5 has two separate responsibilities:

1. mechanically admit one immutable `ContextApplicationV2SupersessionRecord`;
2. evaluate the graph of admitted application records and supersession records
   as a deterministic, immutable currentness read model.

The slice does not make human review substantively correct, does not establish
production authority, and does not modify `C`. It does not implement
`ApplicationHostBindingV2` integration, Buckle-Up review, ClassProjection,
Task 5 Slice 3B, M3, Magic rules, cards, or ML behavior.

No production V3 event, `ContextApplicationV2` record, supersession record, or
current-record file may be created. Later implementation tests must use
synthetic temporary repositories only.

## 2. Stage A repository evidence

The exact repository inspection ran after fetching `origin/master`:

```text
git rev-parse HEAD                 = 9a42c3424e61e54565ff0cd17b3f2f2780f78562
git rev-parse origin/master        = 2cdf4116ac2c7f490507e1253e18b66fd6d5fb2d
git status --short --branch        = master...origin/master [behind 34]
```

The working tree contained one unrelated untracked file:

```text
docs/superpowers/plans/2026-08-26-m2-5-b2-terminal-card-classification-closure.md
```

It was preserved and is not part of this design. The design branch was created
from the exact `origin/master` baseline; no source changes existed on the
branch before this document.

The inspection covered, at minimum:

- ADR 0042, especially §§5–12 and §§18–20;
- the accepted V2 review checklist;
- `context_application_v2_resolver.py`;
- `context_application_v2_validator.py`;
- `context_application_v2_review_admission.py`;
- `reviewer_role_binding.py`;
- the V1 `AuthorityValidator` and HostBinding V2 validator;
- the Python and Rust authority DTO/identity implementations;
- Slice 3 and Slice 4 specifications and plans;
- the current V2 identity, closure, semantic, and review-admission tests;
- the V2 authority and acceptance-event schemas and fixtures.

The repository has no Slice 5 currentness module or Slice 5 graph test. The
V2 authority fixture has an empty
`context_application_v2_supersession_records` array. The production base
authority artifact is absent from this checkout, so production preflight is
not a valid Slice 5 test input.

## 3. Frozen facts that this design preserves

ADR 0042 and the existing DTOs freeze the following facts:

- `cpa.v2` is the semantic identity of an exact finite application.
- `cpar.v2` is the accepted application-record identity and includes the exact
  `ReviewEventRefV3`.
- `cps.v2` is the semantic supersession identity. Its preimage contains the
  superseded record ID, optional replacement record ID, closed kind fields,
  reason, and canonical source evidence; it excludes acceptance metadata.
- `cpsr.v2` is the accepted supersession-record identity. Its preimage contains
  the exact `cps.v2` digest and the exact `ReviewEventRefV3`.
- replacement is `null` exactly for `authority_revocation`;
  `semantic_correction`, `source_revision`, and `model_revision` require a
  non-null `context_application_v2_record` replacement.
- both endpoint kinds are fixed to `context_application_v2_record`.
- historical application records and supersession records are immutable.
- `AcceptanceSubjectPayloadV3` for a supersession is the acceptance-free
  payload specified by ADR 0042 §8.2.
- the V3 subject kind is exactly
  `context_application_v2_supersession_record`.
- a standalone V3 closure is reconstructed by
  `ContextApplicationV2Resolver.expected_acceptance_source_closure_v3(...)`.
  The caller supplies no HostBinding sources.

The V2 DTOs in `python/src/mtgml/authority.py` and
`crates/mtgml-persistence/src/authority.rs` already encode these structural
facts. Slice 5 must not alter them.

## 4. Design alternatives and recommendation

### Alternative A — widen the public Slice 4 validator

Change `ContextApplicationV2ReviewAdmissionValidator.admit(...)` to accept a
union of application and supersession subjects.

This would put graph-admission concerns into the existing application-facing
interface, widen a stable public surface, and make it harder to preserve the
current application diagnostics exactly. It is rejected.

### Alternative B — add one narrow internal V3 review-binding seam

Extract the mechanics common to both subject families into a new private
module, tentatively:

```text
scripts/context_application_v2_review_binding.py
```

Keep the current application validator as a compatibility adapter and add a
supersession adapter. This is the recommended seam because it is deep: callers
provide one typed subject and its existing dependencies, while event parsing,
full digest comparison, closure reconstruction, roster checks, and stable V3
diagnostics remain local to one implementation.

### Alternative C — reuse `AuthorityValidator` or HostBinding V2 graph code

The V1 validator and HostBinding V2 validator are useful historical and
architectural references. Neither is authoritative for ContextApplicationV2
Slice 5. V1 identity/lifecycle semantics differ, and HostBinding is explicitly
owned by Slice 6. This alternative is rejected.

## 5. Proposed module interfaces, pending contract resolution

The following is the smallest implementation shape if Section 7 is resolved.
It is not an implementation authorization.

### 5.1 Shared V3 review-binding module

The internal seam should expose one frozen result and one typed failure. Its
conceptual interface is:

```text
admit_v3_review_binding(
    subject: ContextApplicationV2Record
             | ContextApplicationV2SupersessionRecord,
    source_resolver: AuthoritySourceResolver,
    base_authority_binding: ContextAuthoritySourceBindingV2,
) -> V3ReviewBindingResult
```

The subject kind and event reference are derived from the exact typed subject;
the caller cannot select a different event reference or subject kind through a
parallel argument. The frozen result contains only:

```text
subject_kind
subject_digest_reference: DigestReferenceV1
review_event_ref: ReviewEventRefV3
event_id
exact_event_closure: tuple[ContextAuthoritySourceBindingV2, ...]
reviewer_roster_ref: ReviewerRosterRefV1
required_roles: tuple[str, ...]
review_mode: ReviewMode
```

It does not expose `ResolvedArtifact`, raw event JSON, or a mutable source
graph. It does not run application semantic validation and does not validate
supersession graph relationships.

The shared implementation reuses, without duplication:

- `ContextApplicationV2Resolver.resolve_review_event_leaf_v3(...)`;
- `ContextApplicationV2Resolver.expected_acceptance_source_closure_v3(...)`;
- `require_exact_source_set(...)`;
- `resolve_reviewer_roster(...)`;
- `validate_reviewer_binding_against_roster(...)`;
- the exact ordered `REQUIRED_V2_ROLES` policy;
- resolver-owned V3 diagnostic categories and structured cause codes.

The required role order remains:

```text
architecture_maintainer
rules_authority_maintainer
conformance_maintainer
information_safety_reviewer
```

The first missing role determines the diagnostic. `project_owner` remains
optional; if present in the bound roster, the complete roster role tuple must
be preserved. No new reviewer policy, minimum reviewer count, timing claim, or
human-sufficiency claim is added.

### 5.2 Slice 4 compatibility adapter

`ContextApplicationV2ReviewAdmissionValidator.admit(...)` remains the public
application-specific interface and returns the existing
`ContextApplicationV2ReviewAdmissionResult` with the same fields.

Its order remains:

1. exact application-record type check;
2. existing Slice 3 semantic validation;
3. shared V3 event resolution;
4. exact application subject reconstruction;
5. complete six-field `DigestReferenceV1` comparison;
6. exact standalone V3 closure comparison;
7. roster and role validation;
8. non-empty review evidence confirmation;
9. the existing frozen result.

The adapter maps the shared failure to the existing
`ContextApplicationV2ReviewAdmissionError` without parsing exception text.
Existing tests and the diagnostic tuple
`(code, location, cause_code, missing_role)` must remain unchanged. The
existing `REQUIRED_V2_ROLES` import/export remains available from the current
module for compatibility.

The extraction is proven behavior-preserving by running the existing Slice 4
test file unchanged and adding a focused diagnostic-equivalence test that
compares the pre-extraction matrix with the adapter matrix. No Slice 4
semantic or public-result field moves into the generic seam.

### 5.3 Supersession admission module

The new module, tentatively:

```text
scripts/context_application_v2_supersession.py
```

owns:

- one-record mechanical supersession admission;
- graph construction from already admitted records;
- graph invariant validation;
- deterministic currentness derivation.

Its one-record interface is conceptually:

```text
ContextApplicationV2SupersessionAdmissionValidator.admit(
    record: ContextApplicationV2SupersessionRecord,
) -> ContextApplicationV2SupersessionAdmissionResult
```

The result is frozen and contains the semantic IDs and shared V3 admission
metadata needed by the graph layer. It does not contain raw source artifacts.

The graph evaluator receives typed raw records through one safe public
entrypoint and performs admission itself:

```text
ContextApplicationV2CurrentnessEvaluator.evaluate(
    application_records: Sequence[ContextApplicationV2Record],
    supersession_records: Sequence[ContextApplicationV2SupersessionRecord],
) -> ContextApplicationV2CurrentnessResult
```

It does not accept `already_validated`, `accepted`, `trusted`, or similar
unchecked booleans. Internally it may use private frozen wrappers around the
successful Slice 4 and Slice 5 admission results; those wrappers are not
persisted authority types.

## 6. Supersession-record mechanical admission

The per-record pipeline is read-only and has the following obligations.

### 6.1 Exact typed and structural input

Reject before any source access unless the input is exactly a
`ContextApplicationV2SupersessionRecord`. Revalidate the closed structural
facts instead of relying only on a caller having used the DTO constructor:

- `record_id` has kind `context_application_v2_supersession_record`;
- `supersession_id` has kind `context_supersession_v2`;
- `superseded_record_id` has kind `context_application_record_v2`;
- a non-null replacement has the same application-record kind;
- the superseded kind is exactly
  `context_application_v2_record`;
- replacement kind and replacement ID are both null or both non-null;
- reason is one of the four `SupersessionReason` values;
- source evidence is a non-empty, canonical, duplicate-free tuple of
  `EvidenceRefV1` values.

### 6.2 Identity recomputation

Construct the existing
`ContextApplicationV2SupersessionInputV2` from the record fields, including
the exact canonical `source_evidence_refs`, and recompute:

```text
expected_supersession_id = input.identity()       # cps.v2
```

Compare the complete `AuthorityIdentityV1` with `record.supersession_id`.

Then construct the existing
`ContextApplicationV2SupersessionRecordInputV1` from:

```text
record.supersession_id.digest_bytes
record.review_event_ref_v3
```

and recompute:

```text
expected_record_id = record_input.identity()      # cpsr.v2
```

Compare the complete identity with `record.record_id`. The event reference is
never reduced to only `event_id` or only its raw digest.

### 6.3 V3 subject and closure

Reconstruct exactly:

```text
AcceptanceSubjectPayloadV3(
    subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD,
    subject_payload=record.acceptance_free_subject_payload(),
)
```

The acceptance-free payload is exactly:

```text
[
    "context_application_v2_supersession_record",
    supersession_id.digest_bytes,
    superseded_record_id.digest_bytes,
    replacement_record_id.digest_bytes_or_null,
    "context_application_v2_record",
    "context_application_v2_record" | null,
    reason_code,
    [source_evidence_ref.to_cbor() ...],
]
```

Compute `DigestReferenceV1.from_identity(subject.identity())` and compare all
six fields with the event's `subject_payload_digest_reference`:

```text
envelope_id
algorithm_id
semantic_domain
payload_codec_id
input_schema_id
digest_bytes
```

The shared helper first requires the event's exact subject kind, then performs
this complete reference comparison.

For closure, call only:

```text
resolver.expected_acceptance_source_closure_v3(
    record,
    resolved_event.event.reviewer_roster_ref,
)
```

and compare the event's source list with `require_exact_source_set(...)`.
This path expands the supersession `source_evidence_refs`, required B2/B1
dependencies, base authority, model, and reviewer roster through the existing
resolver. It supplies no caller-selected HostBinding binding. An unexpected
HostBinding binding fails as an extra source through the existing exact-set
check.

The shared helper also reuses the existing V3 event parser, event-ID
recomputation, event raw-digest validation, review-evidence resolution,
reviewer-roster resolution, exact roster-role equality, duplicate reviewer
rejection, mandatory V2 role policy, review-mode closure, and non-empty review
evidence rule.

## 7. Contract analysis and blocking gap

### 7.1 Replacement relation: `application_id` equality is not frozen

The accepted contract does not require:

```text
replacement.application_id == superseded.application_id
```

The endpoint relation is already exact at the accepted-record level: the
supersession names `cpar.v2` record IDs and both endpoint kinds are
`context_application_v2_record`. Requiring equal `cpa.v2` IDs would be an
additional rule not present in the accepted preimage or record shape. It would
also make at least some semantic corrections, source revisions, or model
revisions unable to replace an application whose exact theorem/member
preimage changed.

Therefore the design must not add same-`application_id` equality to admission.
The graph endpoint key is the exact `cpar.v2` `record_id`. This part is
resolved:

```text
REPLACEMENT_APPLICATION_ID_EQUALITY_REQUIRED = NO
```

### 7.2 What the accepted artifacts do not decide

The following facts are separately present but do not close the lifecycle
question:

| Artifact | What it proves | What it does not prove |
|---|---|---|
| ADR 0042 `cpa.v2`/`cpar.v2` identities | application semantics and accepted-record provenance are separate | how multiple accepted revisions are grouped for currentness |
| ADR 0042 `cps.v2`/`cpsr.v2` identities | supersession semantics exclude acceptance metadata; accepted records include it | whether multiple `cpsr.v2` records with one `cps.v2` are history-valid |
| ADR 0042 §5.3 | same-kind endpoints, no self-edge/cycle, immutable records | whether revocation propagates across same-`cpa.v2` revisions |
| V2 container DTO/schema | full `cpsr.v2` records may be represented in a canonical array | graph uniqueness/currentness semantics |
| V1 `AuthorityValidator` | V1 rejects duplicate semantic supersession IDs and skips superseded sources for current application facts | V2 duplicate policy; V1 is a different contract |
| HostBinding V2 validator/checklist | one current claim revision in a HostBinding-specific graph | ContextApplicationV2 authority; HostBinding is Slice 6 precedent only |
| V2 fixtures/tests | structural and closure shapes, including a revocation subject | no accepted V2 supersession graph or duplicate-revision case |

### 7.3 Duplicate `cps.v2` semantic acceptances

The accepted V2 contract is silent on this case:

```text
cpsr-1: cps.v2 = X, record_id = cpsr.v2/R1, event metadata = E1
cpsr-2: cps.v2 = X, record_id = cpsr.v2/R2, event metadata = E2
```

The separate `cpsr.v2` identity makes both records representable and protects
immutable acceptance provenance. That is evidence for a possible
historical-revision interpretation, but it is not an explicit permission to
admit two records for one semantic edge. Conversely, V1's duplicate semantic
ID rejection cannot be copied into V2 without resolving the changed identity
layer first.

There are three materially different semantics:

1. **Accepted revisions:** admit both, retain both `cpsr` IDs, and expose one
   semantic edge keyed by `cps.v2` with all accepted record IDs in canonical
   order.
2. **One semantic acceptance:** reject the second `cpsr` for the same `cps` with
   a stable duplicate-semantic error.
3. **Record-keyed edges:** retain both as separate graph edges, which requires
   an explicit rule explaining why equal semantic edges do not duplicate or
   conflict in currentness.

No accepted contract selects one of these. The design therefore must not
silently deduplicate, reject, choose, or recency-select either record.

```text
DUPLICATE_CPS_ACCEPTANCE_RULE_RESOLVED = NO
```

### 7.4 Currentness and lineage gap

The accepted text says that multiple immutable historical records for one
semantic application are allowed and that the current record is selected after
same-kind supersession evaluation. It does not fully specify:

- whether current-candidate uniqueness is grouped by `application_id`, a
  connected supersession lineage, or another key;
- whether a replacement with a different `application_id` starts a new current
  group or continues the source group;
- whether `authority_revocation` revokes only its exact source `cpar.v2` record
  or all accepted records sharing its `cpa.v2` application identity;
- whether two unsuperseded records with the same `cpa.v2` are always ambiguous;
- whether the duplicate `cpsr` policy in §7.3 changes the edge and candidate
  counts.

The V1 current-application fact helper groups surviving records by the
semantic application ID and rejects multiple surviving revisions, but that is
historical V1 behavior and is not an explicit V2 contract. Applying it to V2
would be a new semantic rule unless accepted as a V2 clarification.

Thus currentness cannot be implemented without choosing an unstated lineage
and duplicate-acceptance rule:

```text
LINEAGE_KEY_RESOLVED                    = NO
CURRENTNESS_SEMANTICS_RESOLVED          = NO
CONTRACT_GAP_FOUND                      = YES
```

This is not a claim that ADR 0042 was incorrectly accepted. Its
`CONTRACT_GAPS_REMAINING=0` status closes the contract-plumbing decisions made
by that ADR, while ADR 0042 §18 explicitly leaves the supersession validator as
a later implementation slice. The missing rules above are the additional
evaluation semantics that Slice 5 must not invent.

The required resolution is an accepted clarification or ADR amendment that
states, at minimum:

1. the current-candidate grouping key;
2. whether equal-`cps.v2` `cpsr.v2` records are valid revisions;
3. whether equal-`cps.v2` revisions collapse to one semantic edge while
   retaining all accepted record IDs;
4. the scope of an authority revocation; and
5. the exact result for multiple unsuperseded records in one group.

Until then, no implementation plan may be written.

## 8. Conditional graph and read-model shape

This section records the smallest candidate shape for review after the gap is
resolved. It is intentionally conditional and is not a frozen contract.

### 8.1 Private nodes and edges

The private graph node would be an admitted application record keyed by its
complete `cpar.v2` identity:

```text
ApplicationRecordNode {
    record_id: AuthorityIdentityV1
    application_id: AuthorityIdentityV1
    admitted_record: private admitted wrapper
}
```

The candidate semantic edge would be:

```text
ContextApplicationV2SupersessionEdge {
    supersession_id: AuthorityIdentityV1          # cps.v2
    accepted_record_ids: tuple[AuthorityIdentityV1, ...]  # cpsr.v2
    superseded_record_id: AuthorityIdentityV1     # cpar.v2
    replacement_record_id: AuthorityIdentityV1 | None
    reason_code: SupersessionReason
}
```

Under the accepted-revisions option in §7.3, `supersession_id` is the semantic
edge key and `accepted_record_ids` preserves every admitted acceptance
revision. No revision is silently discarded. Under another accepted policy,
the edge type and duplicate error may differ; that is why this shape is not
implemented now.

### 8.2 Candidate public read model

The minimal frozen result would be:

```text
ContextApplicationV2CurrentnessResult {
    current_record_ids: tuple[AuthorityIdentityV1, ...]
    superseded_record_ids: tuple[AuthorityIdentityV1, ...]
    revoked_terminal_record_ids: tuple[AuthorityIdentityV1, ...]
    successor_edges: tuple[ContextApplicationV2SupersessionEdge, ...]
}
```

Field meanings:

- `current_record_ids` contains only accepted application-record IDs that are
  current under the resolved grouping rule.
- `superseded_record_ids` contains sources of non-null replacement edges.
- `revoked_terminal_record_ids` contains sources of `authority_revocation`
  edges. It is separate from `superseded_record_ids` because a revocation has
  no replacement.
- `successor_edges` is the complete immutable semantic edge view, including
  all accepted `cpsr.v2` provenance IDs required by the resolved duplicate
  policy.

No mutable mapping, `current` bit, `is_latest` bit, active-record field, or
persisted current-record index is added. The result is a pure read model
constructed locally after all checks succeed.

### 8.3 Conditional currentness equations

If the pending contract clarification adopts the V1-compatible rule “one
current accepted record per `cpa.v2` group,” the candidate algorithm is:

```text
replaced_sources = sources of edges with non-null replacement
revoked_sources  = sources of edges with null replacement
survivors        = accepted application records not in either source set
groups           = survivors grouped by application_id

one survivor in a group  -> that record is current
more than one            -> CURRENTNESS_AMBIGUOUS
zero survivors           -> valid; the group has no current record
```

This rule does not require a replacement to share `application_id`; an edge
connects exact record IDs, while current-candidate uniqueness is evaluated on
the clarified group key. If the accepted clarification instead defines
lineage groups or revocation propagation, these equations must not be used.

Under the conditional rule, the required scenario results are:

| Input | Conditional result |
|---|---|
| `A` only | `current=(A)`, no edges |
| `A → B` | `current=(B)`, `superseded=(A)` |
| `A → B → C` | `current=(C)`, `superseded=(A,B)` |
| `A → null` | `current=()`, `revoked_terminal=(A)` |
| `A → B`, `B → null` | `current=()`, `superseded=(A)`, `revoked_terminal=(B)` |
| two independent lineages with distinct clarified groups | one current per group |
| multiple historical records for one `cpa.v2` | valid only when graph/group evaluation leaves one survivor |
| two unsuperseded candidates in one clarified group | deterministic `CURRENTNESS_AMBIGUOUS` |

The `A → null` row is not enough to resolve the unresolved case where another
accepted record shares `A.application_id`. That exact case must be fixed by
the accepted clarification rather than inferred from timestamps, record names,
or event metadata.

## 9. Graph validation order and stable errors

The eventual implementation should use this order, with the duplicate and
currentness steps revised only by the accepted clarification:

1. validate the two input collection types and materialize them once;
2. validate every application record type and reject duplicate `cpar.v2`
   record IDs;
3. sort application records by complete canonical identity bytes and run Slice 4
   admission for every record;
4. validate every supersession record type and its closed structural fields;
5. recompute and compare every `cps.v2` identity;
6. recompute and compare every `cpsr.v2` identity;
7. sort supersession records by complete canonical record identity bytes and run
   shared V3 mechanical review admission for every record;
8. apply the accepted duplicate-`cps` lifecycle rule;
9. verify the superseded record exists and has the exact application-record kind;
10. verify replacement/null and reason semantics;
11. verify a non-null replacement exists and has the exact application-record
    kind;
12. reject self-supersession;
13. enforce at most one distinct semantic successor per source;
14. detect cycles;
15. derive the three status sets and the semantic edge tuple;
16. enforce the clarified current-candidate uniqueness rule;
17. canonicalize and return the frozen result.

The closed error surface should distinguish independently testable classes:

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

`DUPLICATE_SUPERSESSION_ID` remains conditional: it is valid only if the
accepted clarification rejects equal-`cps.v2` accepted revisions. If accepted
revisions are valid, it must not be emitted for the §7.3 case; an exact
duplicate `cpsr.v2` record is instead `DUPLICATE_RECORD_ID`.

Every error is a typed value carrying a stable code, stable location, and
structured cause/subject IDs where useful. No category is derived by parsing
exception text. Inner Slice 4/V3 codes remain structured cause data.

The deterministic precedence is the numbered validation order above. Within a
stage, the first diagnostic is selected from canonical complete-CBOR ordering,
never from input order or unordered-set iteration. A cycle path, if exposed,
is normalized by rotating each cycle to its smallest canonical node and then
choosing the smallest normalized cycle.

## 10. Admission trust boundary

Currentness never receives a caller assertion that a record was already
validated. Its public raw-typed entrypoint performs:

```text
typed application records
    -> Slice 4 semantic + V3 admission
typed supersession records
    -> Slice 5 structural + V3 admission
successful private wrappers
    -> graph validation
    -> immutable read model
```

Rejected application or supersession records are absent from the graph and can
never influence currentness. The private wrappers are an in-process proof
boundary, not a second persisted authority contract.

## 11. Mutation safety and determinism

Every admission and evaluation path is read-only. It constructs local tuples,
sets, and lookup maps and returns only after all invariants pass. It does not
update a cache or partially publish a current result.

Rejection must preserve the exact:

```text
application records
supersession records
review event references
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

Later tests must snapshot typed `to_cbor()`/`to_wire()` projections and all
temporary-repository file digests before and after repeated rejection calls.

Equivalent input permutations must produce byte/value-equivalent result DTOs
or the same error code, location, cause, and canonical subject details. The
implementation may depend only on explicit typed values and exact source
bytes. It must not use file order, input order, filesystem enumeration,
timestamps, mtime, commit time, username, absolute paths, network, randomness,
or lexical “latest” naming.

## 12. HostBinding and Rust boundaries

Slice 5 does not:

- validate `ApplicationHostBindingV2`;
- require HostBinding claims;
- consume `hbc.v1` claims to establish currentness;
- load `interaction_review_authority.v2` as a semantic requirement;
- let HostBinding determine lineage;
- modify HostBinding V1/V2 contracts.

The only V3 source path is the existing host-free closure function. Container
closure integration remains separately owned by the existing resolver and
Slice 6's cross-layer work.

No Rust Slice 5 policy implementation is proposed. Rust already owns the
structural DTO and identity contract, and the Python authority tooling owns
source resolution, review admission, graph validation, and currentness. Rust
parity would require a separately accepted cross-language policy contract; it
must not be added for symmetry.

## 13. Expected later implementation files

Only the design document is allowed in this Stage D commit. If the contract
gap is resolved and a later implementation is separately authorized, the
smallest expected change set is:

```text
scripts/context_application_v2_review_binding.py       new internal seam
scripts/context_application_v2_review_admission.py      delegation-only edit
scripts/context_application_v2_supersession.py         new Slice 5 module

python/tests/test_context_application_v2_review_admission.py
python/tests/test_context_application_v2_supersession.py new graph/admission tests
python/tests/test_context_application_v2_resolver.py    only if an existing
                                                          resolver regression needs it
```

The following are not expected to change for Slice 5:

```text
python/src/mtgml/authority.py
crates/mtgml-persistence/src/authority.rs
schemas/context-application-authority.v2.schema.json
schemas/review-acceptance-event.v3.schema.json
identity and closure golden fixtures
docs/maintenance/INTERACTION_AUTHORITY_REVIEW_CHECKLIST_V2.md
HostBinding contracts
C artifacts
production authority artifacts
```

If implementation requires any of those frozen surfaces, stop and report a
new exact contract gap.

## 14. Later acceptance test design

No Slice 5 tests are written or run in this design-only stage. After the gap is
resolved, temporary-fixture tests must include:

### Positive controls

```text
P1   one admitted application record -> deterministic current result
P2   A -> B semantic_correction -> B current
P3   A -> B source_revision -> valid
P4   A -> B model_revision -> valid
P5   A -> null authority_revocation -> valid revoked terminal
P6   A -> B -> C -> deterministic terminal result
P7   A -> B -> null -> valid fully revoked lineage
P8   application input permutation -> equivalent result
P9   supersession input permutation -> equivalent result
P10  valid supersession V3 admission: exact subject, full digest reference,
     exact closure, roster, roles, and evidence
```

Independent-lineage positives and equal-`cps.v2` revision positives may be
added only after the accepted clarification selects their semantics.

### Negative controls

```text
N1   unknown superseded record
N2   unknown non-null replacement record
N3   self-supersession
N4   A -> B and A -> C
N5   A -> B -> A
N6   A -> B -> C -> A
N7   authority_revocation with non-null replacement
N8   non-revocation reason with null replacement
N9   wrong replacement record kind
N10  wrong superseded record kind
N11  cps.v2 identity mismatch
N12  cpsr.v2 record identity mismatch
N13  wrong V3 supersession subject kind
N14  wrong full asp.v3 subject digest reference
N15  wrong or stale ReviewEventRefV3
N16  stale review evidence
N17  wrong reviewer roster
N18  reviewer not in roster
N19  exact-role mismatch
N20  missing mandatory V2 role
N21  missing source-closure binding
N22  extra source-closure binding
N23  unauthorized HostBinding source
N24  rejected supersession does not alter a separately derived result
N25  graph result and stable diagnostics are input-order independent
```

The accepted duplicate policy must add either its exact rejection case or its
positive revision case. The accepted currentness policy must add an explicit
ambiguous-currentness case if that state is invalid.

## 15. Stage C self-review

| Review axis | Outcome | Evidence/constraint |
|---|---|---|
| ADR 0042 | `PASS` for preserved surfaces; `BLOCKED` for unresolved lifecycle semantics | identities, subject payload, same-kind/null rules, and no-HostBinding boundary are preserved; no new rule is silently chosen |
| Slice 4 behavior | `PASS` as a design constraint | public entrypoint, result, role order, diagnostics, and semantic validator remain unchanged through an adapter |
| V1 historical behavior | `PASS` as precedent only | V1 immutable records, source-keyed successor rule, cycle rejection, and one-current application fact are recorded without being copied as an unaccepted V2 rule |
| HostBinding boundary | `PASS` | no HostBinding claim, authority, or lineage semantics enter Slice 5 |
| determinism | `PASS` as an implementation obligation | complete canonical-CBOR ordering and normalized cycle diagnostics are specified |
| fail-closed currentness | `BLOCKED` pending contract clarification | currentness cannot be made exact while duplicate revision and grouping/revocation semantics are silent |
| mutation safety | `PASS` as an implementation obligation | all result construction is local and read-only; no cache/index is authorized |
| production-artifact prohibition | `PASS` | no production artifact or acceptance event is created; later tests use temporary fixtures only |
| Rust boundary | `PASS` | existing structural Rust parity is preserved; no policy code is added |

This self-review found no reason to change a frozen schema or identity. It did
find the lifecycle gap in Section 7. The correct outcome is to stop before an
implementation plan rather than to select a convenient interpretation.

## 16. Required design questions — explicit answers

1. **What module owns supersession-record mechanical admission?**
   `scripts/context_application_v2_supersession.py` is the proposed owner;
   implementation is blocked pending Section 7.

2. **What Slice 4 code is reused?**
   The V3 event resolver, exact closure reconstruction, exact source-set
   comparator, reviewer-roster helper, exact role helper, required-role tuple,
   review-mode handling, evidence resolution, and stable V3 cause mapping.

3. **Is a generic internal V3 seam extracted?**
   Yes. Alternative B, a new narrow internal review-binding module, is
   recommended.

4. **How is application admission behavior preserved?**
   Keep the current public validator and result unchanged; make it a delegation
   adapter; run existing tests unchanged plus a diagnostic-equivalence matrix.

5. **What establishes `cps.v2`?**
   Recompute `ContextApplicationV2SupersessionInputV2.identity()` from all
   semantic fields and compare the complete identity.

6. **What establishes `cpsr.v2`?**
   Recompute `ContextApplicationV2SupersessionRecordInputV1.identity()` from
   the exact `cps.v2` digest and complete `ReviewEventRefV3`.

7. **What exact `AcceptanceSubjectPayloadV3` is used?**
   The eight-field acceptance-free supersession payload in ADR 0042 §8.2 and
   Section 6.3 above.

8. **How is full `DigestReferenceV1` equality enforced?**
   Compare all six fields, not only `digest_bytes`.

9. **How is standalone event closure reconstructed?**
   Through `ContextApplicationV2Resolver.expected_acceptance_source_closure_v3(...)`
   with no HostBinding argument, followed by `require_exact_source_set(...)`.

10. **How are roster, roles, checklist, and evidence reused?**
    Through the shared seam and existing `reviewer_role_binding.py`; no parser,
    roster policy, checklist policy, or evidence parser is copied.

11. **What graph node type is used?**
    A private admitted `ContextApplicationV2Record` node keyed by `cpar.v2`
    `record_id`.

12. **What graph edge type is used?**
    A frozen semantic supersession edge containing `cps.v2`, source record,
    replacement/null, reason, and all accepted `cpsr.v2` IDs.

13. **What is the semantic edge key?**
    The frozen semantic identity is `cps.v2`; the graph's treatment of
    multiple accepted records for that key is unresolved.

14. **Are duplicate `cps.v2` edges through multiple `cpsr.v2` records allowed?**
    Not provable from the accepted contracts. This is a contract gap, not an
    implementation choice.

15. **What constitutes multiple successors?**
    After the duplicate policy is fixed, more than one distinct semantic edge
    from one superseded record is a conflict, even if two edges name the same
    target but differ semantically. Equal accepted revisions count as one edge
    only if the accepted policy says so.

16. **How is self-supersession detected?**
    Compare exact source and non-null replacement `cpar.v2` identities.

17. **How are cycles detected deterministically?**
    Build a canonical successor map after the at-most-one-successor check,
    traverse canonical node order, and normalize any reported cycle path.

18. **How is revocation represented?**
    As an edge with `replacement_record_id=None` and
    `reason_code=AUTHORITY_REVOCATION`; its source is in the separate
    `revoked_terminal_record_ids` result field.

19. **What exactly is current?**
    Not yet frozen. Under the candidate V1-compatible policy, it is the sole
    surviving accepted record in its clarified `application_id` group.

20. **Can zero current records be valid because of revocation?**
    Yes for a fully revoked group under the candidate policy. Whether a
    revocation propagates to other same-`cpa.v2` records is unresolved.

21. **When may multiple current records coexist?**
    Under the candidate policy, only across distinct clarified groups or
    independent lineages allowed by that policy; multiple candidates in one
    group are ambiguous. The accepted contract must confirm this.

22. **What is the exact lineage/grouping key?**
    Graph edges are keyed by `cpar.v2` record IDs; candidate grouping is
    tentatively `cpa.v2` `application_id`. The accepted contract does not make
    the latter currentness rule explicit, so the overall lineage key is
    unresolved.

23. **Must replacement share `application_id`?**
    No. Only exact same-kind record IDs are frozen.

24. **If not specified, is that a contract gap?**
    No same-application equality is required by the accepted contract, so its
    absence is not itself a gap. The gap is the missing grouping, revocation,
    and duplicate-acceptance semantics that currentness needs.

25. **Does currentness admit records itself?**
    Yes. The public evaluator accepts typed raw records and performs both
    admissions; no unchecked trusted flag is accepted.

26. **What public result DTO is returned?**
    The frozen four-field `ContextApplicationV2CurrentnessResult` in §8.2,
    pending accepted duplicate/currentness semantics.

27. **What is canonical ordering?**
    All IDs, accepted revision tuples, and edges are ordered by complete
    canonical CBOR identity/edge encodings; no input order is observable.

28. **What stable error model is exposed?**
    The closed categories in §9 with stable location and structured cause data;
    duplicate-`cps` naming remains conditional on Section 7.

29. **What is deterministic error precedence?**
    The numbered pipeline in §9; within each stage, canonical identity order.

30. **How is mutation safety demonstrated?**
    Snapshot typed projections and temporary-repository raw file digests before
    and after repeated successful/rejected calls; assert no cache or artifact
    is written.

31. **What remains Slice 6?**
    All ApplicationHostBindingV2 validation and cross-snapshot/HostBinding
    integration, including any downstream use of currentness.

32. **What files are expected to change?**
    Later: one shared internal seam, a delegation-only Slice 4 edit, one Slice
    5 module, and focused tests listed in §13. This Stage D commit contains
    only this design file.

33. **Why is no persisted current-record index needed?**
    The inputs and accepted edges are immutable and the result is a pure
    deterministic read model. A persisted index would create a second mutable
    authority and introduce invalidation/replay obligations not in the frozen
    contract.

34. **Why is no Rust implementation needed?**
    Rust already owns structural DTO/identity parity. Slice 5 policy belongs
    to repository-aware authority tooling; no accepted wire contract requires
    a Rust currentness implementation.

35. **What tests establish Slice 5 acceptance?**
    The temporary-fixture P1–P10 and N1–N25 matrices in §14, the unchanged
    Slice 4 regression matrix, Rust/Python identity regression, closure and
    source-boundary negatives, deterministic permutation checks, mutation
    snapshots, and later repository gates. They are not run in this design
    stage.

## 17. Design-stage status and hard stop

```text
MASTER_BASE=2cdf4116ac2c7f490507e1253e18b66fd6d5fb2d
DESIGN_ONLY=YES
PRODUCTION_CODE_CHANGED=NO
TESTS_CHANGED=NO
SCHEMA_CHANGED=NO
IDENTITY_CHANGED=NO
PRODUCTION_ARTIFACT_CREATED=NO

SLICE_5_IMPLEMENTATION_STARTED=NO
SLICE_5_IMPLEMENTED=NO
SLICE_6_AUTHORIZED=NO
TASK_5_SLICE_3B=BLOCKED
M3=BLOCKED

LINEAGE_KEY_RESOLVED=NO
DUPLICATE_CPS_ACCEPTANCE_RULE_RESOLVED=NO
CURRENTNESS_SEMANTICS_RESOLVED=NO
CONTRACT_GAP_FOUND=YES
```

The mandatory result is:

```text
STOP
do not create an implementation plan
do not implement Slice 5
```

This design commit is therefore the end of Stage D. Independent design review
and an accepted lifecycle clarification are required before Stage E.
