# M2.5.C — Declared Interaction Model Closure

Status: approved design specification; implementation not authorized in this phase.

Date: 2026-08-28

Branch: `chris/m2-5-c-interaction-model-closure`

## 1. Purpose

M2.5.C closes the declared interaction model as an additive, source-grounded
snapshot. It reconciles every candidate in the immutable REV3 interaction
input, deduplicates reusable terminal semantic classes, preserves concrete
source-instance and context bindings, and emits one fail-closed closure
artifact.

The closure is a declaration and provenance boundary. It is not a Magic rules
engine, a card executor, a ranking input, a deck-lock decision, or an M3
conformance result.

The C gate may make exactly one transition:

```text
DECLARED_INTERACTION_MODEL_CLOSURE: BLOCKED -> PASS
```

It may not promote any later gate. Ranking, reuse, deck lock, and M3 remain
blocked after a successful C closure.

## 2. Scope and non-goals

### 2.1 In scope

The implementation authorized by this specification will:

1. declare the accepted interaction-model vocabulary;
2. materialize the complete current candidate universe from the pinned REV3
   candidate source, with explicitly accounted additions and removals;
3. maintain a deduplicated authority of reusable semantic interaction classes;
4. classify every current candidate with one terminal disposition;
5. bind every required class and instance to exact B2 semantic boundaries and
   the B1.Final official-citation graph;
6. validate the resulting closure with a dedicated fail-closed checker;
7. provide a 32-case negative-test matrix with stable failure reasons; and
8. record execution evidence through the H_exec to H_evidence protocol.

### 2.2 Out of scope

C will not:

- modify REV3, B1, B1.Final, B2, or their historical verification artifacts;
- add or change Magic rules, Card IR, card executors, capabilities, or
  semantic authorities;
- infer interaction truth from capability names, keywords, regular
  expressions, lexical co-occurrence, model scores, or unreviewed heuristics;
- fetch or invent a missing official authority;
- decide automatic controller choices, payment, targets, ordering, or hidden
  information;
- create ranking features or reuse classifications;
- unlock deck lock or M3;
- claim arbitrary unbounded N-way interaction completeness;
- rewrite the dirty main checkout; or
- treat a passing Python-only check as Rust-workspace or hosted-CI evidence.

## 3. Normative sources and authority order

The implementation MUST read and obey the current versions of:

- `AGENTS.md`;
- `README.md`;
- `docs/NORMATIVE_HIERARCHY.md`;
- `docs/DOMAIN_MODEL.md`;
- `docs/INFORMATION_MODEL.md`;
- `docs/DECISION_PROTOCOL.md`;
- `docs/DECISION_INVENTORY.md`;
- `docs/RULES_SEMANTICS.md`;
- `docs/TESTING_AND_CONFORMANCE.md`;
- `docs/testing/NONINTERFERENCE_TESTING.md`;
- `docs/contracts/ACCEPTANCE_GATES.md`;
- `docs/contracts/ENGINE_STATE_CLOSURE.md`;
- `docs/contracts/WIRE_CONTRACT.md`;
- `docs/contracts/ML_CONTRACT.md`;
- `docs/STATE_HASHING.md`;
- `docs/REPLAY_AND_DETERMINISM.md`;
- `docs/cards/CAPABILITY_MODEL.md`;
- `docs/cards/CERTIFICATION.md`; and
- the accepted M2.5/B1/B2 ADRs and closure documents that govern the input
  artifacts.

Where a source contradicts another source, the implementation MUST stop with
`BLOCKED` and identify the contradiction. It must not choose a convenient
interpretation.

The authority order for C data is:

```text
accepted normative documents and ADRs
    > immutable B1.Final and B2 artifacts
    > pinned REV3 source rows and evidence
    > review decisions recorded in C
    > derived C closure, report, and verification evidence
```

C review decisions can classify candidates only within the declared model;
they cannot create a new rules authority.

## 4. Immutable prerequisites and exact input identity

The implementation MUST resolve the live repository state before generating
any C source. It MUST fetch/remap the relevant remote state, verify that the
M2.5 prerequisite PRs are merged where required, record the exact
`origin/master` SHA, and inspect descendants for drift. The SHA observed while
authoring this specification is:

```text
df3d760de2c6b22403764725e0ef707161bbce13
```

That value is a recorded design baseline, not permission to skip the live
check at implementation time.

The following prerequisite gates MUST execute successfully before C can be
`PASS`:

```text
python scripts/check_m2_5_master_drift.py
python scripts/check_m2_5_master_drift.py --negative-self-test
python scripts/check_m2_5_master_drift.py --verify-archive
python scripts/check_m2_5_b1_authority_citations.py
python scripts/check_m2_5_b1_authority_citations.py --negative-self-test
python scripts/check_m2_5_b2_classifications.py
python scripts/check_m2_5_b2_classifications.py --negative-self-test
python scripts/check_m2_5_b1_final_authority_citations.py
python scripts/check_m2_5_b1_final_authority_citations.py --negative-self-test
```

The archive root is supplied through `MANAFOLD_SOURCE_ARCHIVE`; the checker
MUST resolve the archive as a child of that root, not as an arbitrary user
path. The pinned private REV3 archive is:

```text
relative path: m2_5/Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip
sha256: 99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90
```

The C checker MUST verify the archive bytes and the required member identities
before consuming a member. A missing archive, an alternate archive, or a
digest mismatch is `BLOCKED`, not a reason to continue with a substitute.

The semantic REV3 input member is:

```text
inputs/interaction_model_v1.json
sha256: f7a069df5040e9337719aadf0c1c4bde09a4b5dad0bb6489eada49d369a9bc8f
```

Its accepted model identity is `interaction-model.v1`. Its declared coverage
is `PAIRWISE_PLUS_REVIEW_OUTLIERS`, covering:

- unary/card-specific declared outliers;
- unordered binary family relations;
- directional binary relations; and
- explicitly reviewed higher-order interactions.

It does not claim arbitrary unbounded N-way Magic interaction completeness.
The inherited REV3 terminal labels are historical input labels only. Every REV3
candidate currently carries `AMBIGUOUS_REQUIRES_REVIEW`; C MUST not copy that
label as a terminal result.

The exact current REV3 candidate source is:

```text
derived/Pair_Interaction_Census_REV3.csv
```

The checker MUST preserve the exact source row identity, including all source
fields and row-level digest, for every inherited candidate.

## 5. Accepted prerequisite facts

The following facts are expected from the current accepted B2 and B1.Final
inputs and MUST be checked rather than assumed:

| Input fact | Required value at C execution |
| --- | ---: |
| B2 requirement families | 216 |
| B2 active families | 210 |
| B2 active-unassigned families | 6 |
| B2 terminal card classifications | 402 |
| B2 terminal requirement assignments | 1883 |
| B2 assignment rows | 441 |
| B1.Final terminal authorities | 7 |
| B1.Final required active families | 210 |
| REV3 candidates | 15679 |

If the live input produces different values, the checker MUST report the
identity/count mismatch and stop. It may not silently update the specification
or broaden the accepted input.

B2 `ACTIVE_UNASSIGNED` families are valid catalog rows but are not valid
card-derived semantic evidence. They may be used only for an independently
reviewed global obligation explicitly permitted by the B2 contract. A C class
that relies on one as a card-derived proof is rejected.

## 6. Artifact inventory and authority graph

C owns exactly these source artifacts:

```text
sources/m2_5/closures/C/
  declared_interaction_model.v1.json
  interaction_candidate_universe.v1.json
  interaction_semantic_classes.v1.json
  interaction_classifications.v1.json
  interaction_closure.v1.json
  INTERACTION_MODEL_REPORT.md
  verification/
    c_negative_test_matrix.v1.json
    c_verification_summary.v1.json
```

The dedicated checker is:

```text
scripts/check_m2_5_c_interactions.py
```

The C source authority graph is:

```text
declared_interaction_model
            |
            v
interaction_candidate_universe
            |
            +------------------------------+
            v                              v
interaction_semantic_classes       interaction_classifications
            \                              /
             \                            /
              +----------> interaction_closure
                                   |
                                   v
                          report / verification evidence
```

The semantic direction is equivalently:

```text
REV3 + B2 + B1.Final
        |
        v
Candidate Universe
        |
        +----------> Semantic Classes
        |                    |
        +----------> Candidate Dispositions
                             |
                             v
                          Closure
```

`interaction_semantic_classes.v1.json` is the sole C authority for reusable
class definitions. `interaction_classifications.v1.json` is the sole C
authority for candidate-level terminal dispositions and concrete instance
bindings. A class definition MUST NOT be copied into candidate records.

The digest graph is deliberately acyclic:

```text
model
candidate universe
semantic classes
classifications
      |
      v
interaction_closure
      |
      v
report / verification evidence
```

`interaction_closure.v1.json` binds only the semantic C inputs:

- `declared_interaction_model.v1.json`;
- `interaction_candidate_universe.v1.json`;
- `interaction_semantic_classes.v1.json`; and
- `interaction_classifications.v1.json`.

The closure also records the exact identities of its external B1.Final, B2,
and REV3 prerequisites. It does not bind its own bytes, the report, the
negative-test matrix, or the verification summary. The report may reproduce
closure results and the closure digest; the summary may record all digests.
Neither is therefore a closure input.

## 7. File contracts

All JSON artifacts MUST be UTF-8, canonicalized with the repository's existing
canonical JSON/CBOR identity procedure, and emitted with deterministic key and
array ordering. Unknown top-level keys are rejected. Unless a field is
explicitly nullable below, it is required and non-null.

### 7.1 `declared_interaction_model.v1.json`

This file declares the vocabulary and the boundaries of C. It contains:

```text
schema
model_id
model_version
coverage_scope
accepted_rev3_model
accepted_rev3_candidate_source
included_shapes
excluded_claims
terminal_dispositions
context_dimensions
authority_policy
```

Required values include:

```text
model_id = "declared-interaction-model.v1"
coverage_scope = "PAIRWISE_PLUS_REVIEW_OUTLIERS"
terminal_dispositions includes:
  REQUIRED_INTERACTION
  NOT_AN_INTERACTION_WITH_PROOF
  OUT_OF_DECLARED_SCOPE_WITH_REASON
```

`OUT_OF_DECLARED_SCOPE_WITH_REASON` is permitted only where the candidate is
provably outside the declared model boundary and the record states the exact
boundary and evidence. It is not a substitute for unresolved review. Every
current candidate still requires a terminal disposition.

The following are forbidden as terminal dispositions or status values:

```text
AMBIGUOUS_REQUIRES_REVIEW
REQUIRES_REVIEW
UNKNOWN
PROVISIONAL
PENDING
UNRESOLVED
```

The model MUST explicitly preserve directionality, participant role, host
relationship, zone/visibility, timing and temporal ordering, source versus
affected object, controller versus owner, replacement/layer dependency,
trigger/LKI context, information dependency, decision actor, and higher-order
arity.

### 7.2 `interaction_candidate_universe.v1.json`

This file is the mechanically complete candidate ledger. It contains:

```text
schema
model_id
input_bindings
candidate_count
candidate_reconciliation_counts
candidates
```

Each `candidates[]` object contains exactly:

```text
candidate_id
candidate_identity
source_origin
scope
relation
participant_refs
supporting_requirement_ids
rev3_source_row
rev3_source_row_sha256
reconciliation_status
reconciliation_reason
```

`candidate_id`, `scope`, `relation`, `participant_refs`, and source fields are
preserved from REV3. `candidate_identity` is a digest over the canonical
source-row identity and is not a replacement for the original ID.

`source_origin` is one of:

```text
REV3
B2_DERIVED
TARGETED_HIGHER_ORDER_REVIEW
```

Every inherited REV3 candidate MUST appear exactly once with its original
`candidate_id`. A newly derived candidate MUST have a deterministic new ID,
an explicit source origin, and exact B2/source evidence. A candidate MUST NOT
disappear because it is hard to review.

`reconciliation_status` is an accounting field, not a terminal semantic
disposition. The allowed values are:

```text
UNCHANGED
STALE_REV3_CANDIDATE
REMOVED_NOT_INTERACTION
MERGED_SEMANTIC_DUPLICATE
NEW_B2_DERIVED_CANDIDATE
NEW_TARGETED_HIGHER_ORDER_CANDIDATE
```

The status is accompanied by a source-grounded reason. A stale, removed, or
merged row still receives a record in `interaction_classifications` and a
terminal disposition; accounting status never permits silent omission.

The candidate universe MUST preserve the three REV3 relation shapes:

```text
UNORDERED_BINARY
DIRECTIONAL_BINARY
DECLARED_CARD_TRIGGER
```

It MUST also be able to represent explicitly reviewed higher-order candidates
with an ordered participant list and arity greater than two.

### 7.3 `interaction_semantic_classes.v1.json`

This file contains one canonical definition for each reusable terminal
semantic class. It contains:

```text
schema
model_id
input_bindings
class_count
classes
```

Each `classes[]` object contains:

```text
interaction_class_id
class_identity
arity
directionality
participant_roles
host_relationship
context_dimensions
temporal_semantics
b2_family_refs
b2_boundary_refs
b1_final_citation_refs
semantic_rationale
source_evidence_refs
```

Allowed `arity` values are `UNARY`, `BINARY`, and `HIGHER_ORDER`. A
`HIGHER_ORDER` class MUST state the exact finite participant count and ordered
participant roles; it is not an unbounded N-way claim.

Allowed `directionality` values are:

```text
NONE
SYMMETRIC
DIRECTED
```

`DIRECTED` classes MUST identify source and affected roles and preserve the
edge direction. Reversing the edge, collapsing it to an unordered pair, or
removing a role is a validation failure.

`participant_roles[]` is ordered and each entry contains:

```text
position
role
participant_kind
semantic_ref
```

Roles MUST state whether a participant is a source, affected object, target,
controller, owner, replacement actor, trigger source, decision actor, or an
other explicitly named role required by the evidence. A role may not be
implied only by its position.

`host_relationship` is one of:

```text
SAME_HOST
CROSS_HOST
NOT_APPLICABLE
```

The value is semantic: it is not a display label derived from deck names.
Context dimensions MUST state the relevant zone, visibility, timing/phase or
event order, information identity, control/ownership, replacement/layer, and
decision context. If a dimension is not relevant, the class says
`NOT_APPLICABLE`; it must not omit the field.

`b2_family_refs[]` and `b2_boundary_refs[]` identify exact current B2 families
and boundary digests. Each required family reference states its lifecycle and
assignment role. A card-derived class may reference only a valid terminal
assignment to an `ACTIVE` family. An `ACTIVE_UNASSIGNED` family is rejected in
that position.

`b1_final_citation_refs[]` identify nodes in the accepted B1.Final citation
graph. Every normative rule claim supporting a required class MUST resolve to
one of these nodes. C cannot create a citation node or replace a missing
official domain with a URL, prose, or live search result.

`class_identity` is computed from the canonical class meaning, including
arity, directionality, roles, host relationship, context, temporal semantics,
B2 boundary references, B1.Final citation references, and rationale evidence.
Source instances are not copied into the class definition; they are bound by
candidate classification records.

### 7.4 `interaction_classifications.v1.json`

This file contains one record for every candidate in the candidate universe.
It contains:

```text
schema
model_id
candidate_universe_sha256
semantic_classes_sha256
classification_count
candidate_classifications
```

Each `candidate_classifications[]` object contains only candidate-level
semantic facts and provenance:

```text
candidate_id
terminal_disposition
interaction_class_id
source_instance_context_mappings
reconciliation
review_rationale
evidence_refs
```

`interaction_class_id` is required for `REQUIRED_INTERACTION` and must be null
for `NOT_AN_INTERACTION_WITH_PROOF` and
`OUT_OF_DECLARED_SCOPE_WITH_REASON`.

`source_instance_context_mappings[]` binds the candidate to concrete source
instances and exact participant/context values. Each mapping contains:

```text
source_instance_id
participant_bindings
context_binding
b2_assignment_refs
b1_final_citation_refs
```

The mapping is required even for a non-interaction disposition when source
instances or boundary evidence exist; a non-interaction result must show what
was reviewed and why the interaction relation is disproved. A mapping may not
invent an instance not present in the source ledger.

`reconciliation` records the candidate-universe status, original REV3 ID when
applicable, and any merged/new/stale/removal linkage. `review_rationale` is a
specific source-grounded explanation, not a keyword or co-occurrence claim.
`evidence_refs` resolve only to pinned REV3, B2, B1.Final, or C review records.

The file MUST NOT contain copied class definitions. It may contain only the
class ID and the concrete binding to that class.

### 7.5 `interaction_closure.v1.json`

This is the sole semantic C closure artifact. It contains:

```text
schema
closure_id
model_id
bound_semantic_inputs
external_prerequisite_identities
candidate_reconciliation
semantic_class_metrics
terminal_disposition_metrics
source_instance_metrics
gate_status
downstream_status
flags
```

`bound_semantic_inputs` contains exactly four entries, one for each semantic C
input named in §6. Each entry records path, schema, raw SHA-256, canonical
identity digest, and record count. The closure does not include a self-digest.

`external_prerequisite_identities` records the exact REV3 archive/member,
B2 catalog/classification/boundary/assignment closure, and B1.Final authority
citation graph identities used to validate the four C inputs. These are
identity bindings, not copies of those artifacts.

`candidate_reconciliation` MUST report at least:

```text
rev3_total
rev3_unchanged
rev3_stale
rev3_removed_not_interaction
rev3_merged_semantic_duplicate
new_b2_derived
new_targeted_higher_order
current_total
```

`terminal_disposition_metrics` MUST report:

```text
required_interaction
not_an_interaction_with_proof
out_of_declared_scope_with_reason
unresolved
```

`unresolved` MUST equal zero for a possible `PASS`. All counts are recomputed
by the checker from the source artifacts; hand-entered aggregates cannot make
the gate pass.

`downstream_status` MUST preserve:

```text
RANKING_REUSE: BLOCKED
DECK_LOCK: BLOCKED
M3_CONFORMANCE: BLOCKED
```

The closure MUST carry no flag that promotes these states.

### 7.6 `INTERACTION_MODEL_REPORT.md`

The report is a human-readable projection of the C artifacts. It MUST include:

- the exact source/master/archive identities;
- the authority graph and acyclic digest policy;
- candidate totals and all reconciliation deltas;
- semantic class count and class-shape totals;
- terminal disposition totals and unresolved count;
- high-risk review-set coverage;
- B2 and B1.Final binding summary;
- closure status and closure digest, when available;
- downstream blocked statuses; and
- exact commands and their actual statuses.

The report MUST NOT be a closure input. If it repeats closure results or
digests, the checker treats it as derived documentation.

### 7.7 `c_negative_test_matrix.v1.json`

This file is a fixed verification contract, not a semantic input. It contains
exactly 32 cases, each with:

```text
case_id
mutation
expected_status
expected_reason_code
target_artifact
```

It is checked for exact inventory and stable target reason codes. It is not
bound into the closure digest.

### 7.8 `c_verification_summary.v1.json`

This file is an evidence record and remains fully outside the closure. It may
be provisional at H_exec with commands marked `NOT_RUN`; it becomes the
post-execution summary only in Phase C.

The final summary MUST record:

```text
schema
execution_commit
source_commit
source_tree_sha256
prerequisite_results
c_result
negative_test_result
repository_gate_results
artifact_digests
report_digest
negative_matrix_digest
closure_digest
evidence_export
```

`artifact_digests` records the digests of model, candidate universe, semantic
classes, classifications, closure, report, and negative matrix. The summary
does not record or bind its own digest. The final summary must never claim
`PASS` for an unexecuted command.

## 8. Candidate generation and reconciliation

### 8.1 Complete inherited universe

The implementation MUST read the exact REV3 pair census and produce a
candidate record for all 15,679 source rows, unless live prerequisite identity
checking proves that the pinned input changed; in that event C is blocked.

The following source shapes are preserved:

```text
INTRA_DECK + UNORDERED_BINARY
CROSS_DECK + DIRECTIONAL_BINARY
UNARY_OR_HIGHER_ORDER + DECLARED_CARD_TRIGGER
```

The 18 unary/card-specific records remain individually identifiable by their
source OSI identity. Pair IDs, family IDs, relation, source scope, and
supporting requirement IDs remain traceable to their exact source row.

### 8.2 Current B2 alignment

For every candidate that relies on a B2 card classification, C MUST resolve:

1. the exact OSI/card semantic identity;
2. the B2 classification record and review status;
3. every referenced requirement assignment;
4. the current family ID and lifecycle;
5. the exact B2 semantic boundary digest; and
6. the assignment provenance and review evidence.

An unknown OSI, unknown family, missing assignment, invalid assignment, stale
boundary, or altered assignment provenance blocks the candidate and therefore
blocks the closure. No fallback to a family name or capability name is
allowed.

### 8.3 Reconciliation accounting

The closure MUST make each delta explicit. The categories are:

```text
unchanged REV3 candidate
stale REV3 candidate
removed because review proves no interaction
merged semantic duplicate
new B2-derived candidate
new targeted higher-order candidate
```

The categories are mutually exclusive per candidate lineage. A merged
duplicate is not deleted: its original candidate record remains, its
classification points to the canonical class, and the merge relationship is
recorded. A removed/non-interaction candidate remains in the ledger and gets
`NOT_AN_INTERACTION_WITH_PROOF`; “removed” is accounting language, not silent
disappearance.

The count equation MUST hold:

```text
current_total
  = rev3_total
  + new_b2_derived
  + new_targeted_higher_order
```

with the REV3 delta categories partitioning the inherited rows. The checker
also recomputes the partition from candidate lineage instead of trusting the
equation alone.

## 9. Semantic review protocol

### 9.1 Terminal truth

Each candidate is reviewed to exactly one of:

```text
REQUIRED_INTERACTION
NOT_AN_INTERACTION_WITH_PROOF
OUT_OF_DECLARED_SCOPE_WITH_REASON
```

No candidate may remain ambiguous, provisional, unresolved, or pending in a
closure that claims `PASS`. `UNRESOLVED = 0` is a hard gate.

`REQUIRED_INTERACTION` means the cited source and exact semantic boundaries
demonstrate a reusable interaction relation within the declared model.

`NOT_AN_INTERACTION_WITH_PROOF` means the reviewed source and boundaries
demonstrate that the candidate is only co-occurrence, independently composable,
or otherwise lacks the declared interaction relation. The rationale must name
the reviewed participants and the boundary distinction.

`OUT_OF_DECLARED_SCOPE_WITH_REASON` means the candidate is explicitly outside
the finite declared model boundary. It requires a precise model-boundary
reference and cannot conceal missing review.

### 9.2 Class deduplication

Two candidates may share an `interaction_class_id` only when all class identity
fields are equal after canonicalization:

- arity;
- directionality and edge orientation;
- ordered participant roles;
- host relationship;
- zone and visibility context;
- timing and temporal semantics;
- source/affected/controller/owner/replacement/trigger/decision roles;
- B2 boundary references; and
- B1.Final citation references.

If any of these differ, the candidates require different classes or a
candidate-level non-required disposition. Class deduplication must never erase
the concrete source instance, source row, or context mapping.

### 9.3 Required review domains

The implementation MUST maintain explicit reviewed sets for at least:

- triggers and last-known-information behavior;
- replacement effects and layer/dependency ordering;
- copy effects and token creation;
- target legality, protection, and targeting identity;
- control and ownership changes;
- Commander/format-specific behavior;
- hidden information and visibility boundaries;
- ordering and temporal dependencies;
- source versus affected object identity;
- controller/owner/decision-actor identity; and
- explicitly reviewed higher-order interactions.

Membership in a review set is evidence of review coverage, not an inference
that the candidate is required. The resulting terminal disposition and
rationale remain candidate-specific.

### 9.4 Source evidence requirements

Every required class and candidate mapping MUST reference source-grounded
evidence from the pinned REV3 archive and exact B2/B1.Final records. The
reviewer may use an accepted repository record only through its immutable
identity. A capability name, card name, keyword, pair co-occurrence, model
label, or generated family label is not semantic proof by itself.

If a required interaction depends on an official domain absent from B1.Final,
the implementation MUST stop with `BLOCKED`. It must not invent a citation,
fetch live rules during generation, or silently downgrade the requirement.

## 10. Dedicated verifier contract

`python scripts/check_m2_5_c_interactions.py` is the authoritative C structural
and semantic-boundary verifier. It MUST be deterministic and fail closed.

The default invocation MUST verify the current C source and prerequisite
identities. `--negative-self-test` MUST run the exact 32 mutations from the
negative matrix and require every mutation to be rejected with its expected
reason code.

The checker MUST perform all of the following checks:

1. Resolve the exact repository root, C paths, archive root, and archive
   member identities.
2. Validate the C JSON schemas, closed vocabularies, deterministic ordering,
   and exact file inventory.
3. Execute or consume the current B1, B1.Final, B2, and master-drift gate
   results, rejecting any prerequisite that is not `PASS`.
4. Verify the exact REV3 archive and candidate source digests.
5. Verify the declared model scope and its finite higher-order boundary.
6. Recompute the complete candidate universe and reject a missing, duplicate,
   renamed, or extra inherited candidate.
7. Verify every current candidate has exactly one classification record.
8. Verify candidate IDs, class IDs, source-instance IDs, and mapping IDs are
   unique where required.
9. Verify every classification's reconciliation lineage and source row.
10. Verify every required mapping has exact OSI, family, assignment, boundary,
    lifecycle, and provenance bindings.
11. Reject an unknown OSI, family, assignment, or citation reference.
12. Reject a card-derived use of `ACTIVE_UNASSIGNED`.
13. Verify arity, participant count, role names, direction, edge orientation,
    host relationship, zones, timing, information, ordering, and temporal
    semantics.
14. Reject orphan source instances and duplicate or unbound mappings.
15. Verify every required class resolves to B1.Final citation graph nodes.
16. Recompute all candidate, class, reconciliation, and terminal counts.
17. Verify exact bound-input digests and closure identity fields.
18. Verify that report and verification evidence are not closure inputs.
19. Verify downstream statuses remain blocked and no later gate is promoted.
20. Verify the negative-test matrix inventory and expected reason codes.
21. Validate the staging/evidence state against the H_exec/H_evidence protocol.

The checker MUST contain no semantic rule that maps a keyword or capability
name directly to an interaction disposition.

The checker MUST use stable reason codes. A failure may include detail, but the
primary reason code for each negative case is part of the test contract.

Recommended exit semantics are:

```text
0: PASS
1: FAIL
2: BLOCKED
```

An unavailable required prerequisite is `BLOCKED`; malformed or contradictory
C data is `FAIL`. Neither status may be reported as `PASS`.

## 11. Negative-test matrix

The matrix contains exactly the following independent mutations. Each mutation
must target one condition and must be rejected for the stated primary reason.

| Case | Mutation | Expected reason code |
| --- | --- | --- |
| C-001 | Make a required prerequisite gate non-terminal | `PREREQUISITE_NOT_PASS` |
| C-002 | Change the B2 catalog identity | `B2_CATALOG_DIGEST_MISMATCH` |
| C-003 | Change the B2 classification identity | `B2_CLASSIFICATIONS_DIGEST_MISMATCH` |
| C-004 | Change a B2 boundary binding | `B2_BOUNDARY_BINDING_MISMATCH` |
| C-005 | Change the B1.Final citation-graph identity | `B1_FINAL_GRAPH_DIGEST_MISMATCH` |
| C-006 | Change the pinned REV3 archive digest | `REV3_ARCHIVE_DIGEST_MISMATCH` |
| C-007 | Remove one inherited REV3 candidate | `REV3_CANDIDATE_UNACCOUNTED` |
| C-008 | Leave a candidate unresolved while claiming closure PASS | `UNRESOLVED_CANDIDATE_ON_PASS` |
| C-009 | Reference an unknown OSI | `OSI_UNKNOWN` |
| C-010 | Reference an unknown B2 family | `FAMILY_UNKNOWN` |
| C-011 | Reference an invalid assignment | `ASSIGNMENT_BINDING_INVALID` |
| C-012 | Use `ACTIVE_UNASSIGNED` as card-derived proof | `ACTIVE_UNASSIGNED_CARD_DERIVED` |
| C-013 | Duplicate an interaction class ID with different meaning | `DUPLICATE_CLASS_ID` |
| C-014 | Duplicate a candidate/source-instance mapping | `DUPLICATE_SOURCE_INSTANCE_MAPPING` |
| C-015 | Add a source instance with no candidate owner | `ORPHAN_SOURCE_INSTANCE` |
| C-016 | Reverse a directed relation | `DIRECTION_REVERSED` |
| C-017 | Remove the direction from a directed relation | `DIRECTIONALITY_LOST` |
| C-018 | Remove a required participant role | `PARTICIPANT_ROLE_MISSING` |
| C-019 | Remove one participant from a higher-order class | `HIGHER_ORDER_PARTICIPANT_MISSING` |
| C-020 | Rewrite same-host context as cross-host | `HOST_RELATIONSHIP_MISMATCH` |
| C-021 | Rewrite cross-host context as same-host | `HOST_RELATIONSHIP_MISMATCH` |
| C-022 | Remove a required context dimension | `CONTEXT_DIMENSION_MISSING` |
| C-023 | Remove a required B1.Final citation reference | `B1_CITATION_UNRESOLVED` |
| C-024 | Add an authority not present in accepted inputs | `UNAPPROVED_AUTHORITY` |
| C-025 | Bind C to a stale but internally self-consistent prerequisite | `PREREQUISITE_IDENTITY_STALE` |
| C-026 | Tamper with an aggregate count | `AGGREGATE_COUNT_MISMATCH` |
| C-027 | Use a non-terminal disposition in a PASS closure | `NONTERMINAL_DISPOSITION_ON_PASS` |
| C-028 | Promote ranking/reuse status | `DOWNSTREAM_STATUS_PROMOTED` |
| C-029 | Promote deck-lock status | `DOWNSTREAM_STATUS_PROMOTED` |
| C-030 | Promote M3 status | `DOWNSTREAM_STATUS_PROMOTED` |
| C-031 | Change a source artifact after H_exec | `SOURCE_CHANGED_AFTER_H_EXEC` |
| C-032 | Change the evidence summary's recorded artifact digest | `EVIDENCE_DIGEST_BINDING_MISMATCH` |

The matrix MUST also include, in each case's mutation detail, the exact target
path and the expected status (`FAIL` or `BLOCKED`). Cases C-028 through C-030
must prove that a downstream promotion is rejected even when all semantic C
inputs remain otherwise valid. Case C-031 proves the direct-child evidence
boundary. Case C-032 proves that the summary is outside the closure but still
must accurately report evidence identities.

## 12. Master-drift allowlist integration

The existing master-drift verifier MUST remain narrow. C may extend its exact
allowlist only with:

```text
scripts/check_m2_5_c_interactions.py
sources/m2_5/closures/C/
```

It must not allow the broad `scripts/` directory or broad `sources/m2_5/`
prefix. Near-miss paths such as a backup copy of the C checker must be
rejected. C source is additive and must not modify historical B1/B2/REV3
artifacts.

## 13. H_exec and H_evidence protocol

### Phase A — H_exec source snapshot

All C source artifacts, the checker, the exact negative matrix, the report, and
a provisional verification summary with command statuses `NOT_RUN` are created
in the isolated worktree. The source commit is H_exec. Before recording H_exec:

- all C files must be present;
- the working tree must be clean except for the commit being created;
- the master-drift allowlist must recognize only the intended C paths;
- no generated verification output may be mixed into reproducible source; and
- no C implementation beyond the specified artifacts and checker may be
  introduced in the spec-review phase.

The provisional summary records `execution_commit = null` and does not claim
the execution gates passed.

### Phase B — execute against exact H_exec

The required prerequisite, C, repository, language, and integration commands
run against the exact H_exec tree. Any source edit, generated source rewrite,
or semantic artifact correction after H_exec invalidates the evidence cycle and
requires a new H_exec.

At minimum, the C execution set is:

```text
python scripts/check_m2_5_c_interactions.py
python scripts/check_m2_5_c_interactions.py --negative-self-test
python scripts/verify_repository.py
python scripts/run_checks.py integration
cargo +1.85.1 fmt --all -- --check
cargo +1.85.1 check --workspace --all-targets --all-features --locked
```

The repository's applicable Ruff, Mypy, Clippy, Rust test, Python test,
schema, conformance, information-safety, replay, and reproducibility gates
must be included according to `run_checks.py integration` and the acceptance
gate documents. Each command is recorded with its actual status.

### Phase C — H_evidence summary-only child

After Phase B, the only permitted source change is:

```text
sources/m2_5/closures/C/verification/c_verification_summary.v1.json
```

That change records H_exec, the exact execution commit, source-tree identity,
all command results, and all seven requested artifact digests:

```text
model
candidate universe
semantic classes
classifications
closure
report
negative matrix
```

The Phase C commit MUST be a direct child of H_exec. Its diff MUST contain
exactly the verification summary. Any other diff is a failed evidence cycle,
not a minor warning.

Raw logs, the independent semantic-review export, and other generated
verification output remain outside the reproducible source archive.

## 14. Independent review export

After H_exec and before implementation acceptance, the agent MUST provide an
independent review export outside the repository. It must contain:

- the exact H_exec SHA and source tree identity;
- the model summary and its digest;
- the complete candidate reconciliation ledger or a lossless export of it;
- every semantic class with all required fields;
- every candidate classification and concrete source-instance/context mapping;
- the high-risk review-set memberships;
- B2 family/boundary and B1.Final citation bindings;
- the report path and digest; and
- explicit missing/blocked evidence.

The export is for review and is not a new authority. It must not be committed
to the repository or fed back into the closure digest.

## 15. Acceptance criteria

C is accepted only when all of the following are true:

1. The live exact-head and archive preflight are verified.
2. All B1, B1.Final, B2, and master-drift prerequisites are `PASS`.
3. The REV3 candidate universe is complete and every lineage is reconciled.
4. The model scope is finite and matches `PAIRWISE_PLUS_REVIEW_OUTLIERS`.
5. Semantic classes are separately authoritative and deduplicated.
6. Candidate classifications contain no copied class definitions.
7. Every candidate has exactly one terminal disposition.
8. `UNRESOLVED = 0`.
9. Every required class has exact B2 boundary and B1.Final citation bindings.
10. All required context, direction, role, host, timing, ordering, and arity
    information is explicit.
11. The closure binds only its four semantic C inputs and is acyclic with
    respect to report and verification evidence.
12. The 32 negative tests pass with their exact reason codes.
13. H_exec and H_evidence satisfy the direct-child summary-only rule.
14. Required repository and language gates execute successfully.
15. Ranking/reuse, deck lock, and M3 remain `BLOCKED`.
16. The independent review export is available for human review.
17. The implementation branch and PR, if later requested, remain unmerged
    until the semantic C spec and resulting evidence have been independently
    reviewed.

If any criterion is unknown, unexecuted, contradictory, or unavailable, the
result is `BLOCKED` or `NOT_RUN` as appropriate. It is not a C PASS.

## 16. Spec-review boundary

This commit defines the artifact contracts, authority graph, review protocol,
negative cases, verifier obligations, and evidence protocol only. It does not
create the C JSON artifacts, checker, review classifications, or implementation
logic. Those changes require a separate explicit implementation authorization
after independent review of this specification.
