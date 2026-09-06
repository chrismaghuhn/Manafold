# ContextApplicationV2 Slice 3 Semantic Validator Design

**Status:** approved for implementation planning

**Baseline:** `9a42c3424e61e54565ff0cd17b3f2f2780f78562`

**Scope:** ContextApplicationV2 implementation Slice 3 only

## Goal

Validate that every ContextApplicationV2 member connects the exact historical
SourceInstanceV1 to the exact ContextProofRecordV1 through the claimed bridge
values. The validator will establish source binding, theorem equivalence,
precondition preservation, bridge relation correctness, and exact evidence
resolution. It will not establish human acceptance or production authority.

## Non-goals

This slice will not implement V3 reviewer or checklist semantics, production
acceptance events, production ContextApplicationV2 records, supersession
currentness, HostBindingV2 semantic integration, ClassProjection or C
derivation, Buckle-Up review, cards, Magic rules, Task 5 Slice 3B, or M3.

The implementation will not change the JSON Schema, identity registry,
canonical identity preimages, Slice 1 identity fixtures, Slice 2 closure
fixtures, ADR 0042, or production C outputs.

## Governing invariants

The validator keeps the historical and reviewed values independent:

```text
SourceInstanceV1.source_context
    == member.bridge.context[*].source_value

ContextProofV1.context_dimensions
    == member.bridge.context[*].reviewed_value

ContextProofV1.temporal_semantics
    == member.bridge.temporal[*].reviewed_value
```

For each context slot, the validator derives the relation from the two bridge
values and rejects a caller-supplied relation that disagrees:

```text
source_value == reviewed_value  -> exact_match
source_value != reviewed_value  -> reviewed_divergence
```

The V1 `source_context` precondition continues to compare its expected value
with the historical SourceInstanceV1 value. It never compares with the V2
reviewed value. V1 ContextApplication validation keeps its old theorem/source
context equality check, so V1 behavior remains unchanged while V2 uses the
bridge.

## Architecture

### Pure semantic core

Both languages will expose a small, filesystem-free validation core. Its input
contains the typed theorem subject shape, member context binding, historical
source context values, theorem context values, bridge values and relations,
theorem temporal values, member temporal values, and theorem/member
precondition vectors.

The core performs only exact ordered comparisons and relation derivation. It
does not read files, inspect archives, parse Oracle text, inspect capability
names, infer semantic meaning, or resolve candidates. Failures carry a stable
category and a slot or precondition location where applicable.

Python and Rust will consume the same additive semantic golden matrix. The
matrix will contain positive exact-match and reviewed-divergence cases and
negative cases for each pure comparison. Rust tests will execute the matrix
through the Rust core; Python tests will execute the same cases through the
Python core and compare the expected validity and error category.

### Python integration validator

`scripts/context_application_v2_validator.py` will own the filesystem-aware
composition. Its primary semantic-validation entry point will accept a
structurally valid in-memory `ContextApplicationV2Record`, a V2
base-authority binding, and the existing resolver dependencies. A separate pure
input entry point may accept `ContextApplicationV2InputV1`; no duck-typed or
informal third input shape will be supported. It will:

1. resolve the exact base authority through `ContextApplicationV2Resolver`;
2. validate the complete base document with `AuthorityValidator`;
3. retrieve the exact `cpr.v1/...` ContextProofRecordV1 and require its
   record kind and identity;
4. resolve every member through the existing candidate-universe and
   SourceInstance resolver;
5. recompute `ContextApplicationV2InputV1(theorem_record_id, members).identity()`
   and require exact equality with the supplied `cpa.v2` application identity;
6. for a `ContextApplicationV2Record`, recompute
   `ContextApplicationV2RecordInputV1(application_id, review_event_ref_v3).identity()`
   and require exact equality with its supplied `cpar.v2` record identity;
7. apply the shared V1 source/member and precondition contract;
8. run the pure bridge comparisons against the exact theorem and source
   values;
9. resolve member, precondition, context-slot, and temporal-slot evidence
   through the existing V2 evidence resolver; and
10. return success only after every member passes.

The record-identity recomputation uses `review_event_ref_v3` only as the
existing Slice 1 structural value in the `cpar.v2` preimage. It does not load
the event or evaluate reviewer, checklist, or acceptance semantics.

The validator will inspect only fields already present in the accepted V2
contract. It will not require a V3 review event, process acceptance, select a
current record, or write any artifact. A structurally valid synthetic record
may contain a synthetic V3 reference because Slice 3 does not resolve or
evaluate that reference.

Validation will be read-only. Rejection will not mutate authority files,
source artifacts, candidate data, IDs, events, history, review state, or C.

### Shared V1 seam

`AuthorityValidator` will gain the smallest reusable operation needed by both
V1 and V2. The operation will validate the common source/member contract:

- theorem subject shape versus member context binding;
- source arity, directionality, participant positions, roles, kinds, and
  semantic references;
- member evidence resolution;
- exact precondition count, order, IDs, and observed payloads;
- precondition evidence resolution.

The shared operation will preserve the existing behavior of each
`precondition_kind` independently. It will not generalize all preconditions
into SourceInstance checks:

- `candidate_relation_shape` keeps the exact source-shape comparison;
- `participant_binding` keeps the exact source-participant comparison;
- `source_context` keeps the exact historical
  `SourceInstanceV1.source_context` comparison;
- `b2_boundary` keeps the existing bound-B2 resolution and validation;
- `temporal_semantic` keeps the existing V1 theorem/attestation behavior and
  introduces no SourceInstance-derived temporal fact; and
- `class_projection` keeps the existing V1 fail-closed proof requirement.

The V2 member shape has no `member_proof_attestation`, so the V2 adapter will
not invent a class-projection proof shortcut or a new member field. A V2
class-projection precondition follows the existing missing-proof failure path.

The shared operation will accept a narrow injected evidence resolver so V1
keeps its existing resolver and V2 can use the Slice 2 V2 resolver without
copying filesystem logic. It will not perform the V1-only
theorem-context/source-context vector equality.

The existing V1 context path will call the shared operation, then retain its
historical context-vector comparison and V1 slot attestation checks. The V2
path will call the shared operation, then validate its independent source-side
and theorem-side bridge equations.

## Source and evidence safety

Candidate and SourceInstance resolution will use all four member identity
inputs: candidate ID, full CandidateIdentityV1 digest reference,
SourceInstance ID, and candidate-universe binding. The resolver must confirm
candidate ownership, source ownership, snapshot identity, and exact source
bytes.

Every evidence reference must resolve to its exact raw digest, admitted
snapshot, valid locator, and source artifact. Candidate- or SourceInstance-
specific substitution will be rejected only when the existing locator
contract mechanically encodes enough ownership information to prove it. Global
evidence remains valid when the contract does not make it candidate-local.

Successful evidence resolution proves binding and integrity only. It does not
establish that the evidence is substantively sufficient to justify the
reviewed semantic value; that remains part of the later V3 human-review and
checklist admission.

Before completion, the implementation will explicitly audit every evidence
form used by the fixtures. If an exercised form cannot prove the required
ownership mechanically, the validator will not guess, search text, use array
position, or add a schema field. It will report
`EVIDENCE_IDENTITY_CONTRACT_GAP=YES` and stop the slice as `BLOCKED`.

## Test design

The tests will add, without changing historical fixtures:

- `context_application_v2_semantic_golden_matrix.v1.json` for pure Rust/Python
  parity;
- a synthetic exact-match application whose ten context relations are all
  `exact_match` and whose fourteen evidence groups resolve;
- a synthetic reviewed-divergence application in which at least one source
  value differs from the reviewed theorem value and still passes;
- a mandatory divergence regression with a V1 `source_context` precondition
  bound to the historical value;
- a V1 regression proving that a V1 ContextApplication with a theorem/source
  context mismatch remains rejected;
- negative tests for theorem, candidate, SourceInstance, snapshot, shape,
  source context, reviewed context, temporal value, relation, precondition,
  stale evidence, unresolved evidence, and mechanically provable
  cross-candidate or cross-SourceInstance substitution; and
- deterministic repeatability checks that compare validity, error category,
  and pure golden output across two identical runs.

The positive integration test will compose Slice 1 DTO/identity construction,
Slice 2 exact source resolution, and the Slice 3 semantic validator without
V3 reviewer policy, supersession, HostBinding semantics, or production
acceptance.

## Verification and delivery

The implementation plan will require focused tests, the smoke and full Python
profiles, Ruff, mypy, Rust formatting/check/clippy/test, generated-contract
and schema checks, documentation and maintainer-artifact checks, diff checks,
and the fast, integration, and certification profiles. Each result will be
reported separately as `PASS`, `FAIL`, `NOT_RUN`, or `BLOCKED`.

The final change will be committed on the dedicated Slice 3 branch and may be
offered as a PR against `master`, but it will not be merged. The final report
will retain `CONTEXT_APPLICATION_V2_IMPLEMENTED=NO`,
`FROZEN_CONTRACT=NO`, `HUMAN_ACCEPTANCE=BLOCKED`,
`CONTEXT_APPLICATION_V2_SLICE_4_AUTHORIZED=NO`, `TASK_5_SLICE_3B=BLOCKED`,
and `M3=BLOCKED`.
