# ContextApplicationV2 Slice 5 — Supersession and Currentness Implementation Plan

**Status:** provisional implementation plan; implementation not started

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement read-only ContextApplicationV2 supersession admission and deterministic application-wide revocation/currentness evaluation exactly as accepted by ADR 0043, without changing any persisted identity, schema, or production authority artifact.

**Architecture:** Extract one narrow internal V3 review-binding module from the existing Slice 4 application admission path. Keep Slice 3 application semantics and the Slice 4 public validator unchanged behind a compatibility adapter. Add one Slice 5 module that mechanically admits supersession records, groups accepted `cpsr.v2` revisions into semantic `cps.v2` edges, validates the immutable record-ID graph, and derives a frozen currentness read model grouped by `application_id`.

**Tech Stack:** Python 3.11–3.13, frozen dataclasses, `unittest`, existing `AuthoritySourceResolver`, `ContextApplicationV2Resolver`, canonical CBOR and digest-envelope identities, existing V2/V3 DTOs, repository verification scripts, and Rust structural parity checks. No new Rust Slice 5 policy implementation.

---

**Integration base (`BASE`):** `193493c610424bbf19bef30a25de14746c21dff5`

**Reviewed plan head before this amendment (`REVIEWED_PLAN_HEAD`):**
`9f2e6de07e66edb302cfdb1ba140e55f5857b085`

The latest accepted docs-only commit on
`chris/context-application-v2-slice5-implementation-plan` is the
`PLAN_HEAD` for implementation. The amendment commit containing this
correction becomes the next plan-head candidate and must be independently
reviewed before implementation authorization. Do not substitute `BASE` for
`PLAN_HEAD`.

**Implementation branch:** `chris/context-application-v2-slice5-supersession-currentness`

**Plan authorization:** `YES`

**Implementation authorization:** `NO` until this plan is independently reviewed and separately authorized.

## Approved inputs and non-negotiable invariants

Implementation starts only after plan approval from the exact accepted
`PLAN_HEAD`, with `origin/master` still at `BASE`. The implementation
branch is created from `PLAN_HEAD`; the plan and register commit therefore
remain in the final PR diff. The authoritative inputs are:

```text
docs/adr/0042-context-application-v2-reviewed-context-bridge.md
docs/adr/0043-context-application-v2-supersession-lineage-revocation-currentness.md
docs/superpowers/specs/2026-09-07-context-application-v2-slice5-supersession-currentness-design.md
docs/superpowers/specs/2026-09-07-context-application-v2-slice5-contract-gap-resolution-design.md
```

The implementation must preserve:

```text
cpa.v2, cpar.v2, cps.v2, cpsr.v2
asp.v3, ae.v3
AcceptanceSubjectPayloadV3
ReviewAcceptanceEventInputV3
ReviewAcceptanceEventLeafV3
ReviewEventRefV3
DigestReferenceV1
all existing JSON Schemas
all existing canonical-CBOR identity preimages
Slice 3 application semantic validation
Slice 4 public result/error behavior
existing V1 authority behavior
existing host-free V3 closure behavior
```

ADR 0043 fixes these Slice 5 semantics:

```text
graph node key                 = complete cpar.v2 record_id
non-null lineage edge          = superseded_record_id -> replacement_record_id
replacement application_id    = may differ from source application_id
semantic edge key              = complete cps.v2 supersession_id
same cps.v2 + distinct cpsr   = one edge, all cpsr IDs retained
distinct cps.v2 from one source, even same target
                                = MULTIPLE_SUCCESSORS
authority_revocation           = all records in source application_id group
cross-application revocation  = no propagation
currentness grouping key      = complete cpa.v2 application_id
eligible count 0               = valid no-current group
eligible count 1               = current record
eligible count >1              = CURRENTNESS_AMBIGUOUS
```

The code must not add a persisted `lineage_id`, `current`, `is_latest`,
`active_record`, or currentness index. A rejected admission or graph must
publish no partial result and must not mutate records, source files, caches, or
production artifacts.

## File map

| File | Operation | Responsibility |
|---|---|---|
| `scripts/context_application_v2_review_binding.py` | Create | One internal, typed V3 event/subject/closure/roster/evidence binding seam shared by application and supersession admission. |
| `scripts/context_application_v2_review_admission.py` | Modify | Delegation-only refactor preserving the existing public application admission interface, result fields, error codes, locations, causes, and role precedence. |
| `scripts/context_application_v2_supersession.py` | Create | Supersession-record mechanical admission, immutable semantic-edge grouping, graph validation, revocation, and currentness read-model derivation. |
| `python/tests/context_application_v2_test_support.py` | Create | Temporary-repository V3 subject/event fixture builders shared by existing Slice 4 and new Slice 5 tests; never writes under the production checkout. |
| `python/tests/test_context_application_v2_review_admission.py` | Modify | Reuse the shared fixture helper and add public-behavior equivalence assertions; retain every existing Slice 4 test. |
| `python/tests/test_context_application_v2_supersession.py` | Create | RED/GREEN tests for supersession admission, graph invariants, revocation, currentness, determinism, and mutation safety. |
| `scripts/run_python_tests.py` | Modify | Add the new supersession test module to the explicit smoke profile. |
| `python/tests/test_python_test_profiles.py` | Modify | Assert one smoke-profile entry for the new supersession module and preserve unfiltered full-profile discovery. |
| `docs/normative-document-register.v1.json` | Modify in the plan-only commit | Register this provisional implementation plan as `process / provisional / process-pr / maintainer`, following the existing Slice 3/4 plan convention. |

Do not modify:

```text
python/src/mtgml/authority.py
crates/mtgml-persistence/src/authority.rs
scripts/context_application_v2_resolver.py
scripts/reviewer_role_binding.py
schemas/context-application-authority.v2.schema.json
schemas/review-acceptance-event.v3.schema.json
identity fixtures
closure fixtures
production authority files
production V3 events
C artifacts
```

## Public and internal interfaces fixed by this plan

The shared seam is internal and receives the typed subject's own event
reference; callers cannot pass a second event reference or subject kind:

```python
def admit_v3_review_binding(
    subject: ContextApplicationV2Record
    | ContextApplicationV2SupersessionRecord,
    source_resolver: AuthoritySourceResolver,
    *,
    base_authority_binding: ContextAuthoritySourceBindingV2,
) -> ContextApplicationV2V3ReviewBindingResult:
    pass
```

The Slice 5 public record-admission interface is:

```python
class ContextApplicationV2SupersessionAdmissionValidator:
    def admit(
        self,
        record: ContextApplicationV2SupersessionRecord,
    ) -> ContextApplicationV2SupersessionAdmissionResult:
        pass
```

The currentness evaluator owns admission-before-currentness:

```python
class ContextApplicationV2CurrentnessEvaluator:
    def evaluate(
        self,
        application_records: Sequence[ContextApplicationV2Record],
        supersession_records: Sequence[ContextApplicationV2SupersessionRecord],
    ) -> ContextApplicationV2CurrentnessResult:
        pass
```

It accepts only typed sequences and performs Slice 4/Slice 5 admission itself.
There is no `already_validated`, `accepted`, `trusted`, or boolean bypass.

## Task 0: Verify the exact implementation base and plan head

**Files:** None modified.

- [ ] **Step 1: Resolve `BASE` and `PLAN_HEAD`, then create the implementation branch.**

Run:

```powershell
git fetch origin master
git fetch origin chris/context-application-v2-slice5-implementation-plan
$base = git rev-parse origin/master
$planHead = git rev-parse origin/chris/context-application-v2-slice5-implementation-plan
if ($base -ne "193493c610424bbf19bef30a25de14746c21dff5") { throw "BASE drift" }
git switch --create chris/context-application-v2-slice5-supersession-currentness $planHead
git rev-parse HEAD
git branch --show-current
git status --short --branch
git diff --name-only "$base..HEAD"
git diff --name-only HEAD
```

Expected:

```text
BASE = 193493c610424bbf19bef30a25de14746c21dff5
PLAN_HEAD = exact tip of origin/chris/context-application-v2-slice5-implementation-plan
HEAD = PLAN_HEAD
origin/master = BASE
branch = chris/context-application-v2-slice5-supersession-currentness
BASE..HEAD = the approved plan and register documents
working-tree diff from HEAD = empty
```

The known untracked
`docs/superpowers/plans/2026-08-26-m2-5-b2-terminal-card-classification-closure.md`
is preserved and is not staged.

If the resolved `PLAN_HEAD` is not the independently approved docs-only
plan/amendment tip, or if `origin/master` differs from `BASE`, stop and
record the exact discrepancy before touching implementation files.

- [ ] **Step 2: Re-read the accepted contract and implementation seams.**

Read ADR 0042 §§5–12, ADR 0043, both Slice 5 design documents, the current
Slice 4 admission module, the V3 resolver, the V1 authority validator, the
authority DTOs, and all existing ContextApplicationV2 tests. Confirm that the
V3 resolver already accepts a typed supersession subject and that no resolver,
schema, or identity extension is needed.

- [ ] **Step 3: Stop if the base, plan head, branch, or scope differs.**

Do not edit, stash, reset, clean, rebase, or delete anything when `BASE`,
`PLAN_HEAD`, the implementation branch, or unrelated untracked state differs.
Record the exact discrepancy for review.

- [ ] **Step 4: Commit no implementation work.**

This task is a baseline gate only. The plan remains the only authorized output
until plan review completes.

## Task 1: Add RED tests for the shared V3 seam and fixture reuse

**Files:**

- Create: `python/tests/context_application_v2_test_support.py`
- Modify: `python/tests/test_context_application_v2_review_admission.py`
- Test: `python/tests/test_context_application_v2_review_admission.py`

- [ ] **Step 1: Extract the existing temporary V3 fixture mechanics without changing behavior.**

Move the reusable portions of the existing `_record_with_v3_event`,
`_event_input_from_wire`, and `_write_rebound_event` helpers into a test-only
module. Preserve the current application helper as a thin delegate. Add these
typed helper interfaces:

```python
def build_application_with_v3_event(
    test_case: unittest.TestCase,
    case: Mapping[str, object],
    *,
    review_mode: ReviewMode = ReviewMode.MULTI_REVIEWER,
    reviewer_roles: tuple[str, ...] = DEFAULT_REVIEWER_ROLES,
) -> tuple[AuthoritySourceResolver, ContextApplicationV2Record, dict[str, object]]:
    pass

def build_supersession_with_v3_event(
    test_case: unittest.TestCase,
    case: Mapping[str, object],
    semantic_input: ContextApplicationV2SupersessionInputV2,
    *,
    review_mode: ReviewMode = ReviewMode.MULTI_REVIEWER,
    reviewer_roles: tuple[str, ...] = DEFAULT_REVIEWER_ROLES,
) -> tuple[
    AuthoritySourceResolver,
    ContextApplicationV2SupersessionRecord,
    dict[str, object],
]:
    pass
```

The supersession helper must construct a provisional record with a zero-valued
synthetic `ReviewEventRefV3`, compute its subject with
`acceptance_free_subject_payload()`, call the existing host-free
`expected_acceptance_source_closure_v3(...)`, write a temporary V3 event, then
construct the final record with the event's exact raw digest and event ID. This
keeps `cps.v2` independent of acceptance metadata while producing a valid
`cpsr.v2`.

- [ ] **Step 2: Add a RED test for the new shared seam.**

Add a test that passes an existing valid application subject and a valid
supersession subject to `admit_v3_review_binding`. Assert the future frozen
result fields and exact required-role tuple:

```python
def test_shared_v3_binding_accepts_both_subject_families(self) -> None:
    from context_application_v2_review_binding import admit_v3_review_binding

    case = self._synthetic_case()
    source_resolver, application, _ = build_application_with_v3_event(self, case)
    application_binding = admit_v3_review_binding(
        application,
        source_resolver,
        base_authority_binding=case["base_binding"],
    )
    self.assertEqual(
        application_binding.subject_kind,
        AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
    )

    supersession_input = ContextApplicationV2SupersessionInputV2(
        superseded_record_id_bytes=application.record_id.digest_bytes,
        replacement_record_id_bytes=None,
        replacement_record_kind=None,
        reason_code=SupersessionReason.AUTHORITY_REVOCATION,
        source_evidence_refs=(case["member"].member_evidence_refs[0],),
    )
    source_resolver, supersession, _ = build_supersession_with_v3_event(
        self, case, supersession_input
    )
    supersession_binding = admit_v3_review_binding(
        supersession,
        source_resolver,
        base_authority_binding=case["base_binding"],
    )
    self.assertEqual(
        supersession_binding.subject_kind,
        AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD,
    )
    self.assertEqual(
        supersession_binding.required_roles,
        (
            "architecture_maintainer",
            "rules_authority_maintainer",
            "conformance_maintainer",
            "information_safety_reviewer",
        ),
    )
```

The test must assert that the helper result exposes no raw artifact or mutable
JSON projection.

- [ ] **Step 3: Add a RED compatibility fingerprint for the existing Slice 4 public behavior.**

Capture the existing public error fingerprint for the current application
admission cases without changing their expected values:

```python
def admission_fingerprint(error: ContextApplicationV2ReviewAdmissionError) -> tuple[object, ...]:
    return (
        error.code,
        error.location,
        error.cause_code,
        error.missing_role,
        str(error),
    )
```

Use it for the existing wrong-subject, missing-role, duplicate-reviewer,
closure, stale-evidence, and read-only rejection cases. The post-extraction
adapter must produce the same tuples.

- [ ] **Step 4: Run the RED tests.**

Run:

```powershell
python -m unittest discover -s python/tests -p 'test_context_application_v2_review_admission.py' -v
```

Expected: the new direct seam test fails because
`context_application_v2_review_binding.py` does not yet exist; all failures
must be import/interface failures, not altered existing Slice 4 assertions.

- [ ] **Step 5: Commit the test-only RED change.**

```powershell
git add python/tests/context_application_v2_test_support.py python/tests/test_context_application_v2_review_admission.py
git diff --cached --check
git commit -m "test: characterize ContextApplicationV2 shared V3 binding"
```

## Task 2: Implement the narrow shared V3 review-binding seam

**Files:**

- Create: `scripts/context_application_v2_review_binding.py`
- Test: `python/tests/test_context_application_v2_review_admission.py`

- [ ] **Step 1: Define the frozen result and typed failure.**

Use the existing Slice 4 fields and role order; do not introduce a new review
policy:

```python
REQUIRED_V2_ROLES: Final = (
    "architecture_maintainer",
    "rules_authority_maintainer",
    "conformance_maintainer",
    "information_safety_reviewer",
)

@dataclass(frozen=True)
class ContextApplicationV2V3ReviewBindingResult:
    subject_kind: AcceptanceSubjectKindV3
    subject_digest_reference: DigestReferenceV1
    review_event_ref: ReviewEventRefV3
    event_id: str
    exact_event_closure: tuple[ContextAuthoritySourceBindingV2, ...]
    reviewer_roster_ref: ReviewerRosterRefV1
    required_roles: tuple[str, ...]
    review_mode: ReviewMode

class ContextApplicationV2V3ReviewBindingError(ValueError):
    def __init__(
        self,
        code: str,
        location: str,
        *,
        cause_code: str | None = None,
        missing_role: str | None = None,
    ) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        self.missing_role = missing_role
        super().__init__(f"{code} at {location}")
```

- [ ] **Step 2: Implement typed subject-kind and event-reference ownership.**

Derive the expected subject kind from the runtime type:

```python
if isinstance(subject, ContextApplicationV2Record):
    expected_kind = AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD
elif isinstance(subject, ContextApplicationV2SupersessionRecord):
    expected_kind = AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_SUPERSESSION_RECORD
else:
    raise ContextApplicationV2V3ReviewBindingError("APPLICATION_INPUT_INVALID", "subject")
reference = subject.review_event_ref_v3
```

Resolve `reference` exactly once through
`ContextApplicationV2Resolver.resolve_review_event_leaf_v3(...)`. Preserve the
existing explicit mapping: resolver V3 codes remain identical; delegated
`ResolutionError` at event loading becomes `V3_EVENT_SOURCE_INVALID` with the
source code as cause.

- [ ] **Step 3: Reconstruct the exact subject digest and compare all fields.**

Use the typed subject's existing acceptance-free method:

```python
subject_payload = AcceptanceSubjectPayloadV3(
    subject_kind=expected_kind,
    subject_payload=subject.acceptance_free_subject_payload(),
)
expected_reference = DigestReferenceV1.from_identity(subject_payload.identity())
if resolved_event.event.subject_kind is not expected_kind:
    raise ContextApplicationV2V3ReviewBindingError(
        "V3_SUBJECT_KIND_MISMATCH", "event.subject_kind"
    )
if expected_reference != resolved_event.event.subject_payload_digest_reference:
    raise ContextApplicationV2V3ReviewBindingError(
        "V3_SUBJECT_DIGEST_MISMATCH",
        "event.subject_payload_digest_reference",
    )
```

Dataclass equality must compare `envelope_id`, `algorithm_id`,
`semantic_domain`, `payload_codec_id`, `input_schema_id`, and `digest_bytes`.

- [ ] **Step 4: Reuse exact standalone closure and reviewer policy.**

Call only:

```python
expected_closure = resolver.expected_acceptance_source_closure_v3(
    subject,
    resolved_event.event.reviewer_roster_ref,
)
require_exact_source_set(
    resolved_event.event.source_binding_digests,
    expected_closure,
)
```

Do not pass HostBinding sources. Reuse `resolve_reviewer_roster` and
`validate_reviewer_binding_against_roster`; preserve duplicate reviewer
rejection, exact roster role tuples, missing-role precedence, non-empty review
evidence, and the existing review-mode enum. Do not parse raw V3 JSON or
reviewer rosters in the new module.

- [ ] **Step 5: Run the shared-seam GREEN tests and the unchanged Slice 4 file.**

```powershell
python -m unittest discover -s python/tests -p 'test_context_application_v2_review_admission.py' -v
```

Expected: the new direct shared-seam tests pass; existing tests remain green
because the public application adapter has not yet been changed.

- [ ] **Step 6: Commit the shared seam.**

```powershell
git add scripts/context_application_v2_review_binding.py
git diff --cached --check
git commit -m "refactor: extract ContextApplicationV2 V3 binding seam"
```

## Task 3: Refactor Slice 4 application admission through the seam

**Files:**

- Modify: `scripts/context_application_v2_review_admission.py`
- Modify: `python/tests/test_context_application_v2_review_admission.py`

- [ ] **Step 1: Preserve the public imports and result surface.**

Keep `REQUIRED_V2_ROLES`, `ContextApplicationV2ReviewAdmissionResult`,
`ContextApplicationV2ReviewAdmissionError`, and
`ContextApplicationV2ReviewAdmissionValidator.admit(record)` importable with
the same names. Re-export the shared constant without creating a second value:

```python
from context_application_v2_review_binding import (
    REQUIRED_V2_ROLES,
    ContextApplicationV2V3ReviewBindingError,
    admit_v3_review_binding,
)
```

- [ ] **Step 2: Keep Slice 3 semantic validation first.**

The adapter must retain the current sequence:

```python
if not isinstance(record, ContextApplicationV2Record):
    raise ContextApplicationV2ReviewAdmissionError("APPLICATION_INPUT_INVALID", "record")
self._semantic_validator.validate(record)
binding = admit_v3_review_binding(
    record,
    self._source_resolver,
    base_authority_binding=self._base_binding,
)
return ContextApplicationV2ReviewAdmissionResult(
    record_id=record.record_id,
    application_id=record.application_id,
    subject_digest_reference=binding.subject_digest_reference,
    review_event_ref=binding.review_event_ref,
    event_id=binding.event_id,
    exact_event_closure=binding.exact_event_closure,
    reviewer_roster_ref=binding.reviewer_roster_ref,
    required_roles=binding.required_roles,
    review_mode=binding.review_mode,
)
```

Map `ContextApplicationV2V3ReviewBindingError` fields directly into the
existing public error class. Keep the existing public locations and structured
cause values; do not match exception message text.

- [ ] **Step 3: Run the full existing Slice 4 regression matrix.**

```powershell
python -m unittest discover -s python/tests -p 'test_context_application_v2_review_admission.py' -v
python -m unittest discover -s python/tests -p 'test_review_admission_foundation.py' -v
```

Expected: all existing tests pass with identical error fingerprints and frozen
result assertions. Any changed Slice 4 code/location/cause/message is a
blocking regression and must be corrected before continuing.

- [ ] **Step 4: Commit the compatibility adapter.**

```powershell
git add scripts/context_application_v2_review_admission.py python/tests/test_context_application_v2_review_admission.py
git diff --cached --check
git commit -m "refactor: reuse shared V3 binding for Slice 4 admission"
```

## Task 4: Add RED tests for supersession-record mechanical admission

**Files:**

- Create: `python/tests/test_context_application_v2_supersession.py`
- Modify: `python/tests/context_application_v2_test_support.py`

- [ ] **Step 1: Add the valid revocation admission control.**

Build the semantic input from the admitted application record's exact
`record_id` and one existing member evidence reference. Use the support helper
to create the content-addressed V3 event and final `cpsr.v2` record. Assert the
new validator returns the frozen admission result with exact IDs:

```python
def test_valid_authority_revocation_is_mechanically_admitted(self) -> None:
    case = self._application_case()
    _, application, _ = build_application_with_v3_event(self, case)
    semantic_input = ContextApplicationV2SupersessionInputV2(
        superseded_record_id_bytes=application.record_id.digest_bytes,
        replacement_record_id_bytes=None,
        replacement_record_kind=None,
        reason_code=SupersessionReason.AUTHORITY_REVOCATION,
        source_evidence_refs=(case["member"].member_evidence_refs[0],),
    )
    source_resolver, supersession, _ = build_supersession_with_v3_event(
        self, case, semantic_input
    )

    result = ContextApplicationV2SupersessionAdmissionValidator(
        source_resolver,
        base_authority_binding=case["base_binding"],
    ).admit(supersession)

    self.assertEqual(result.record_id, supersession.record_id)
    self.assertEqual(result.supersession_id, supersession.supersession_id)
    self.assertIsNone(result.replacement_record_id)
    self.assertEqual(result.reason_code, SupersessionReason.AUTHORITY_REVOCATION)
    self.assertEqual(result.subject_digest_reference.semantic_domain,
                     "manafold.m2.5.c.acceptance-subject-payload.v3")
```

- [ ] **Step 2: Add a valid replacement admission control.**

Construct a second valid application record in the same temporary source
fixture with a changed semantic application input and a complete V3 event. Use
the exact replacement `cpar.v2` ID in
`ContextApplicationV2SupersessionInputV2`; assert that a different
`application_id` is accepted when both endpoint record kinds are correct.

- [ ] **Step 3: Add identity and structural RED tests.**

Use `object.__setattr__` only in negative tests to mutate a frozen DTO after
construction, then assert the future stable categories:

```text
wrong supersession_id       -> SUPERSESSION_IDENTITY_MISMATCH
wrong record_id             -> SUPERSESSION_RECORD_IDENTITY_MISMATCH
wrong reason/null pair      -> SUPERSESSION_REASON_INVALID
wrong replacement kind      -> SUPERSESSION_REPLACEMENT_INVALID
wrong record type            -> SUPERSESSION_INPUT_INVALID
```

The test must assert that the wrong-type rejection performs no source-resolver
call.

- [ ] **Step 4: Add V3 admission RED tests.**

Rebind the temporary event while changing one field at a time and assert
`SUPERSESSION_REVIEW_ADMISSION_FAILED` with structured cause data for:

```text
wrong subject_kind
wrong complete subject DigestReferenceV1
wrong ReviewEventRefV3/raw event bytes
missing or extra source binding
unauthorized HostBinding source
stale review evidence
wrong roster
reviewer not in roster
exact-role mismatch
missing architecture/rules/conformance/information-safety role
```

The fixture must use `ContextApplicationV2Resolver.expected_acceptance_source_closure_v3(...)`
to build valid controls. It must never write under `sources/m2_5/authorities`
in the real checkout.

- [ ] **Step 5: Add frozen-result and rejection-safety assertions.**

Assert the admission result is a frozen dataclass, contains no raw artifact or
JSON object, and that repeated rejected calls return the same
`(code, location, cause_code)` fingerprint while preserving the record's full
`to_cbor()` projection and all temporary-repository file digests.

- [ ] **Step 6: Run the supersession RED tests.**

```powershell
python -m unittest discover -s python/tests -p 'test_context_application_v2_supersession.py' -v
```

Expected: import failure because
`ContextApplicationV2SupersessionAdmissionValidator` does not yet exist.

- [ ] **Step 7: Commit the supersession admission RED tests.**

```powershell
git add python/tests/test_context_application_v2_supersession.py python/tests/context_application_v2_test_support.py
git diff --cached --check
git commit -m "test: characterize ContextApplicationV2 supersession admission"
```

## Task 5: Implement supersession-record mechanical admission

**Files:**

- Create: `scripts/context_application_v2_supersession.py`
- Test: `python/tests/test_context_application_v2_supersession.py`

- [ ] **Step 1: Define the stable admission result and error.**

Use frozen values only:

```python
@dataclass(frozen=True)
class ContextApplicationV2SupersessionAdmissionResult:
    record_id: AuthorityIdentityV1
    supersession_id: AuthorityIdentityV1
    superseded_record_id: AuthorityIdentityV1
    replacement_record_id: AuthorityIdentityV1 | None
    reason_code: SupersessionReason
    subject_digest_reference: DigestReferenceV1
    review_event_ref: ReviewEventRefV3
    event_id: str
    exact_event_closure: tuple[ContextAuthoritySourceBindingV2, ...]
    reviewer_roster_ref: ReviewerRosterRefV1
    required_roles: tuple[str, ...]
    review_mode: ReviewMode

class ContextApplicationV2SupersessionError(ValueError):
    def __init__(
        self,
        code: str,
        location: str,
        *,
        cause_code: str | None = None,
        record_id: AuthorityIdentityV1 | None = None,
        supersession_id: AuthorityIdentityV1 | None = None,
    ) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        self.record_id = record_id
        self.supersession_id = supersession_id
        super().__init__(f"{code} at {location}")
```

The stable code set is:

```text
SUPERSESSION_INPUT_INVALID
SUPERSESSION_IDENTITY_MISMATCH
SUPERSESSION_RECORD_IDENTITY_MISMATCH
SUPERSESSION_REVIEW_ADMISSION_FAILED
SUPERSESSION_REASON_INVALID
SUPERSESSION_REPLACEMENT_INVALID
```

- [ ] **Step 2: Implement the exact structural and identity order.**

The validator must execute these checks before graph evaluation:

```python
if not isinstance(record, ContextApplicationV2SupersessionRecord):
    raise ContextApplicationV2SupersessionError(
        "SUPERSESSION_INPUT_INVALID", "record"
    )

semantic_input = ContextApplicationV2SupersessionInputV2(
    superseded_record_id_bytes=record.superseded_record_id.digest_bytes,
    replacement_record_id_bytes=(
        None
        if record.replacement_record_id is None
        else record.replacement_record_id.digest_bytes
    ),
    replacement_record_kind=(
        None
        if record.replacement_record_id is None
        else "context_application_v2_record"
    ),
    reason_code=record.reason_code,
    source_evidence_refs=record.source_evidence_refs,
)
expected_supersession_id = semantic_input.identity()
if expected_supersession_id != record.supersession_id:
    raise ContextApplicationV2SupersessionError(
        "SUPERSESSION_IDENTITY_MISMATCH", "supersession_id",
        supersession_id=record.supersession_id,
    )

expected_record_id = ContextApplicationV2SupersessionRecordInputV1(
    record.supersession_id.digest_bytes,
    record.review_event_ref_v3,
).identity()
if expected_record_id != record.record_id:
    raise ContextApplicationV2SupersessionError(
        "SUPERSESSION_RECORD_IDENTITY_MISMATCH", "record_id",
        record_id=record.record_id,
    )
```

Before constructing the semantic input, explicitly verify the closed reason,
replacement/null, endpoint-kind, evidence-type, canonical-order, and
duplicate-free invariants. Map each failed invariant to its fixed category;
do not derive categories by parsing `AuthorityContractError` text.

- [ ] **Step 3: Call the shared V3 binding seam and map causes.**

After identity checks, call:

```python
binding = admit_v3_review_binding(
    record,
    self._source_resolver,
    base_authority_binding=self._base_binding,
)
```

Map `ContextApplicationV2V3ReviewBindingError` to
`SUPERSESSION_REVIEW_ADMISSION_FAILED`, retaining its exact V3 code as
`cause_code` and its stable location as the error location. Do not run graph
target existence, successor uniqueness, revocation, or currentness in this
method.

- [ ] **Step 4: Return only the frozen admission result.**

Populate every field from the typed record and shared result. Do not expose
`ResolvedReviewAcceptanceEventV3`, `ResolvedArtifact`, raw event JSON, roster
JSON, or source maps. Export only the public validator, result, error, and the
internal graph types needed by the evaluator.

- [ ] **Step 5: Run the supersession admission GREEN tests and Slice 4 regressions.**

```powershell
python -m unittest discover -s python/tests -p 'test_context_application_v2_supersession.py' -v
python -m unittest discover -s python/tests -p 'test_context_application_v2_review_admission.py' -v
python -m unittest discover -s python/tests -p 'test_review_admission_foundation.py' -v
```

Expected: all valid admission controls and negative categories pass, and all
pre-existing Slice 4 tests retain their exact diagnostics and frozen result
surface.

- [ ] **Step 6: Commit the per-record admission implementation.**

```powershell
git add scripts/context_application_v2_supersession.py
git diff --cached --check
git commit -m "feat: add ContextApplicationV2 supersession admission"
```

## Task 6: Add RED tests for graph validation, revocation, and currentness

**Files:**

- Modify: `python/tests/test_context_application_v2_supersession.py`
- Modify: `python/tests/context_application_v2_test_support.py`

- [ ] **Step 1: Add the result-surface assertion.**

The future public result must be frozen and contain exactly these derived
fields:

```python
@dataclass(frozen=True)
class ContextApplicationV2CurrentnessResult:
    current_record_ids: tuple[AuthorityIdentityV1, ...]
    superseded_record_ids: tuple[AuthorityIdentityV1, ...]
    revoked_record_ids: tuple[AuthorityIdentityV1, ...]
    successor_edges: tuple[ContextApplicationV2SupersessionEdge, ...]
```

Assert no `current`, `is_latest`, `active_record`, cache, or raw artifact is
present.

- [ ] **Step 2: Add positive graph/currentness controls.**

Build all inputs through the real public evaluator and temporary fixtures:

```text
P1   A only -> A current
P2   A -> B with equal application_id -> B current
P3   A -> B with different application_id -> valid B current
P4   A -> B -> C -> C current
P5   A -> null -> A application group revoked, zero current
P6   A -> B -> null -> A superseded, B group revoked, zero current
P7   two independent lineages with distinct application IDs -> both current
P8   linked historical records for one application -> one current
P9   two cpsr revisions for one cps -> one edge, both cpsr IDs retained
P10  input record/supersession permutations -> equal result values
P11  complete supersession V3 subject/closure/roster/role/evidence -> PASS
```

- [ ] **Step 3: Add cross-application revocation precedence controls.**

For a valid edge `A[x] -> B[y]`, add another accepted record in group `x`
with `X[x] -> null`. Assert:

```text
A and X are revoked and cannot be current
B and other records in y are unaffected
the historical x -> y edge remains in successor_edges
```

Repeat with a revocation anchor in group `y`; assert B/y is revoked while the
x group is not retroactively revoked. These tests must use no event timestamp
or insertion order.

- [ ] **Step 4: Add distinct-semantic-edge and duplicate-record controls.**

Create two valid supersession records with the same source and replacement but
different `source_evidence_refs`, so their recomputed `cps.v2` IDs differ.
Assert `MULTIPLE_SUCCESSORS`. Then construct the distinct semantic edges
`A --semantic_correction--> B` and `A --authority_revocation--> null` and
assert the same `MULTIPLE_SUCCESSORS` failure; the revocation edge must not be
excluded from successor validation. Create two different accepted V3 events
for one unchanged semantic `cps.v2`; assert one edge and a canonical tuple
containing both `cpsr.v2` IDs. Repeat the same `cpsr.v2` object; assert
`DUPLICATE_RECORD_ID`.

- [ ] **Step 5: Add currentness ambiguity and graph-negative controls.**

Cover these exact categories:

```text
unknown superseded record          -> SUPERSEDED_RECORD_UNKNOWN
unknown replacement record         -> REPLACEMENT_RECORD_UNKNOWN
self-supersession                  -> SELF_SUPERSESSION
two distinct successors             -> MULTIPLE_SUCCESSORS
A -> B -> A                         -> SUPERSESSION_CYCLE
A -> B -> C -> A                    -> SUPERSESSION_CYCLE
two eligible records in one cpa     -> CURRENTNESS_AMBIGUOUS
invalid admitted supersession       -> SUPERSESSION_ADMISSION_FAILED
```

The ambiguity case must contain two individually admitted records with the
same exact `application_id` and no effective edge removing either candidate.
For `CURRENTNESS_AMBIGUOUS`, set `application_id` to the group key and
`subject_record_ids` to the sorted candidate IDs; leave
`supersession_id` and `subject_supersession_ids` empty.
For `SUPERSESSION_ADMISSION_FAILED`, assert the stable outer code and
location, the inner `cause_code` and `cause_location`, and the exact
`record_id`/`supersession_id` fields without inspecting exception text.

- [ ] **Step 6: Add deterministic error and mutation tests.**

For every graph failure, permute application and supersession input sequences
and assert the same error fingerprint:

```python
(
    error.code,
    error.location,
    error.cause_code,
    error.cause_location,
    error.record_id,
    error.supersession_id,
    error.application_id,
    error.subject_record_ids,
    error.subject_supersession_ids,
    error.cycle_path,
)
```

Snapshot all typed records, event references, temporary source bytes, review
evidence, roster files, IDs, and history before each rejection. Assert no
partial result or currentness cache exists after rejection.

- [ ] **Step 7: Run the graph RED tests.**

```powershell
python -m unittest discover -s python/tests -p 'test_context_application_v2_supersession.py' -v
```

Expected: admission tests pass, while graph tests fail because
`ContextApplicationV2CurrentnessEvaluator` and the derived graph result do not
yet exist.

- [ ] **Step 8: Commit the graph RED tests.**

```powershell
git add python/tests/test_context_application_v2_supersession.py python/tests/context_application_v2_test_support.py
git diff --cached --check
git commit -m "test: characterize Slice 5 graph currentness"
```

## Task 7: Implement immutable graph validation and currentness

**Files:**

- Modify: `scripts/context_application_v2_supersession.py`
- Test: `python/tests/test_context_application_v2_supersession.py`

- [ ] **Step 1: Define the frozen edge, result, and graph error DTOs.**

Use `AuthorityIdentityV1` values rather than bare strings in public results:

```python
@dataclass(frozen=True)
class ContextApplicationV2SupersessionEdge:
    supersession_id: AuthorityIdentityV1
    accepted_record_ids: tuple[AuthorityIdentityV1, ...]
    superseded_record_id: AuthorityIdentityV1
    replacement_record_id: AuthorityIdentityV1 | None
    reason_code: SupersessionReason

@dataclass(frozen=True)
class ContextApplicationV2CurrentnessResult:
    current_record_ids: tuple[AuthorityIdentityV1, ...]
    superseded_record_ids: tuple[AuthorityIdentityV1, ...]
    revoked_record_ids: tuple[AuthorityIdentityV1, ...]
    successor_edges: tuple[ContextApplicationV2SupersessionEdge, ...]

class ContextApplicationV2CurrentnessError(ValueError):
    def __init__(
        self,
        code: str,
        location: str,
        *,
        cause_code: str | None = None,
        cause_location: str | None = None,
        record_id: AuthorityIdentityV1 | None = None,
        supersession_id: AuthorityIdentityV1 | None = None,
        application_id: AuthorityIdentityV1 | None = None,
        subject_record_ids: tuple[AuthorityIdentityV1, ...] = (),
        subject_supersession_ids: tuple[AuthorityIdentityV1, ...] = (),
        cycle_path: tuple[AuthorityIdentityV1, ...] = (),
    ) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        self.cause_location = cause_location
        self.record_id = record_id
        self.supersession_id = supersession_id
        self.application_id = application_id
        self.subject_record_ids = subject_record_ids
        self.subject_supersession_ids = subject_supersession_ids
        self.cycle_path = cycle_path
        super().__init__(f"{code} at {location}")
```

- [ ] **Step 2: Add canonical collection and record helpers.**

Reject non-sequences, strings, bytes, mappings, and wrong record types as
`CURRENTNESS_INPUT_INVALID`. After type validation, sort only by complete
canonical identity bytes:

```python
def identity_key(identity: AuthorityIdentityV1) -> bytes:
    return encode_canonical(identity.to_cbor())

def record_key(record: ContextApplicationV2Record) -> bytes:
    return identity_key(record.record_id)

def sorted_records(
    values: Sequence[ContextApplicationV2Record],
) -> tuple[ContextApplicationV2Record, ...]:
    return tuple(sorted(values, key=record_key))
```

Reject duplicate application or supersession accepted-record IDs before any
graph lookup. Use the same complete identity key for all public ordering and
diagnostic selection.

- [ ] **Step 3: Implement admission-before-currentness.**

In `evaluate`, first call the existing
`ContextApplicationV2ReviewAdmissionValidator.admit` for every application
record in canonical order. Wrap a public Slice 4 error as
`APPLICATION_REVIEW_ADMISSION_FAILED` with its code/location as structured
cause data.

Then call `ContextApplicationV2SupersessionAdmissionValidator.admit` for every
supersession record in canonical `cpsr.v2` record-ID order. Do not accept a
caller-provided admitted flag or prevalidated wrapper as the public input. Wrap
any `ContextApplicationV2SupersessionError` as one
`ContextApplicationV2CurrentnessError` with:

```python
raise ContextApplicationV2CurrentnessError(
    "SUPERSESSION_ADMISSION_FAILED",
    "supersession_record",
    cause_code=exc.code,
    cause_location=exc.location,
    record_id=record.record_id,
    supersession_id=record.supersession_id,
    subject_record_ids=(
        record.record_id,
        record.superseded_record_id,
    ),
) from exc
```

Preserve the inner code and location only in these structured fields; never
parse exception text. The outer code, location, and identity fields are stable
for every permutation of the same invalid input.

- [ ] **Step 4: Group equal `cps.v2` revisions and construct one edge.**

Group successful supersession admissions by complete `supersession_id` key.
For each group:

```python
edge = ContextApplicationV2SupersessionEdge(
    supersession_id=first.supersession_id,
    accepted_record_ids=tuple(
        sorted((item.record_id for item in group), key=identity_key)
    ),
    superseded_record_id=first.superseded_record_id,
    replacement_record_id=first.replacement_record_id,
    reason_code=first.reason_code,
)
```

Retain every distinct `cpsr.v2` ID. An exact repeated record ID is
`DUPLICATE_RECORD_ID`. Do not emit `DUPLICATE_SUPERSESSION_ID` for distinct
accepted revisions of one valid `cps.v2`.

- [ ] **Step 5: Validate targets, self-edges, successor uniqueness, and cycles.**

Use the admitted application record map keyed by complete `cpar.v2` identity.
For each canonical edge:

```python
if edge.superseded_record_id not in application_by_id:
    raise ContextApplicationV2CurrentnessError(
        "SUPERSEDED_RECORD_UNKNOWN",
        "supersession_edge.superseded_record_id",
        record_id=edge.superseded_record_id,
        supersession_id=edge.supersession_id,
        subject_record_ids=(edge.superseded_record_id,),
        subject_supersession_ids=(edge.supersession_id,),
    )
if edge.replacement_record_id is not None and (
    edge.replacement_record_id not in application_by_id
):
    raise ContextApplicationV2CurrentnessError(
        "REPLACEMENT_RECORD_UNKNOWN",
        "supersession_edge.replacement_record_id",
        record_id=edge.replacement_record_id,
        supersession_id=edge.supersession_id,
        subject_record_ids=(edge.replacement_record_id,),
        subject_supersession_ids=(edge.supersession_id,),
    )
if edge.replacement_record_id == edge.superseded_record_id:
    raise ContextApplicationV2CurrentnessError(
        "SELF_SUPERSESSION",
        "supersession_edge.replacement_record_id",
        record_id=edge.superseded_record_id,
        supersession_id=edge.supersession_id,
        subject_record_ids=(edge.superseded_record_id,),
        subject_supersession_ids=(edge.supersession_id,),
    )
```

Insert only one distinct semantic edge per source. If a source already maps to
a different `cps.v2`, raise `MULTIPLE_SUCCESSORS` with
`record_id=source_record_id`, `subject_record_ids=(source_record_id,)`, and
`subject_supersession_ids` containing the sorted complete semantic IDs. Set
`supersession_id` to the canonical smallest conflicting semantic ID. This
includes the case where replacement IDs are equal and the case where one edge
is a revocation edge. After this check, follow the canonical successor map.
Normalize each cycle by its smallest canonical rotation, report the smallest
normalized cycle as `SUPERSESSION_CYCLE`, set `cycle_path` and
`subject_record_ids` to that normalized path, and set
`subject_supersession_ids` to the sorted semantic IDs on the cycle.

- [ ] **Step 6: Derive application-wide revocation and currentness.**

Derive the sets only after the complete graph is valid:

```python
replacement_sources = {
    edge.superseded_record_id
    for edge in edges
    if edge.replacement_record_id is not None
}
revoked_application_ids = {
    application_by_id[edge.superseded_record_id].application_id
    for edge in edges
    if edge.reason_code is SupersessionReason.AUTHORITY_REVOCATION
}
revoked_record_ids = {
    record.record_id
    for record in application_records
    if record.application_id in revoked_application_ids
}
```

Group all admitted application records by exact `application_id`, in canonical
group order. For a revoked group, use no candidates. Otherwise remove only
`replacement_sources`. Return zero candidates as valid, one as current, and
raise `CURRENTNESS_AMBIGUOUS` for more than one. Revocation removes candidates
but never removes historical edges or records.

- [ ] **Step 7: Canonicalize and return the complete immutable result.**

Sort every ID tuple by `identity_key`. Sort edges by source identity,
replacement identity/null, reason value, and semantic `supersession_id`. Build
the complete `ContextApplicationV2CurrentnessResult` in local variables and
return it only after all checks pass. Do not store it in a module, resolver, or
authority cache.

- [ ] **Step 8: Run the graph GREEN suite.**

```powershell
python -m unittest discover -s python/tests -p 'test_context_application_v2_supersession.py' -v
```

Expected: all admission, graph, revocation, currentness, duplicate-revision,
permutation, cycle, error-precedence, and mutation-safety tests pass.

- [ ] **Step 9: Commit the graph implementation.**

```powershell
git add scripts/context_application_v2_supersession.py
git diff --cached --check
git commit -m "feat: derive ContextApplicationV2 currentness"
```

## Task 8: Wire the new tests into the explicit smoke profile

**Files:**

- Modify: `scripts/run_python_tests.py`
- Test: `python/tests/test_python_test_profiles.py`

- [ ] **Step 1: Add the Slice 5 module to `SMOKE_TESTS`.**

Insert the following new test-module name immediately beside the other
ContextApplicationV2 entries:

"test_context_application_v2_supersession",

Do not change any other `SMOKE_TESTS` entry or the full-profile
discovery behavior.

- [ ] **Step 2: Add the profile-regression assertion.**

Extend `test_python_test_profiles.py` to assert the new module appears exactly
once in `run_python_tests.SMOKE_TESTS` and that the full profile remains
unfiltered discovery.

- [ ] **Step 3: Run the profile regression.**

```powershell
python -m unittest discover -s python/tests -p 'test_python_test_profiles.py' -v
python scripts/run_python_tests.py --profile smoke
```

Expected: the profile test and the smoke suite pass, including the new Slice 5
module, with no duplicate module loading.

- [ ] **Step 4: Commit the smoke-profile wiring.**

```powershell
git add scripts/run_python_tests.py python/tests/test_python_test_profiles.py
git diff --cached --check
git commit -m "test: include ContextApplicationV2 currentness in smoke"
```

## Task 9: Run the complete implementation verification gates

**Files:** No source edits expected.

- [ ] **Step 1: Run the focused final Python tests.**

```powershell
python -m unittest discover -s python/tests -p 'test_context_application_v2_review_admission.py' -v
python -m unittest discover -s python/tests -p 'test_context_application_v2_supersession.py' -v
python -m unittest discover -s python/tests -p 'test_context_application_v2_resolver.py' -v
python -m unittest discover -s python/tests -p 'test_context_application_v2_validator.py' -v
```

Expected: all Slice 3, Slice 4, resolver, shared-seam, supersession, graph,
currentness, duplicate-revision, revocation, permutation, and mutation tests
pass.

- [ ] **Step 2: Run the smoke and full Python profiles.**

```powershell
python scripts/run_python_tests.py --profile smoke
python scripts/run_python_tests.py --profile full
```

Expected: exit code 0 for both profiles. Report any explicit skips separately;
do not promote skipped coverage or unavailable tools to `PASS`.

- [ ] **Step 3: Run Python formatting, lint, and typing.**

```powershell
ruff format --check python scripts
ruff check python scripts
mypy --config-file python/pyproject.toml
```

Expected: all three commands exit 0. If `ruff` or `mypy` is unavailable,
record `NOT_RUN` and do not claim a clean gate.

- [ ] **Step 4: Run Rust structural regression gates without adding Rust policy.**

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
```

Expected: all commands exit 0. The diff must contain no Rust source change;
these gates prove existing DTO/identity/codec parity remains intact, not that
Rust owns Slice 5 policy.

- [ ] **Step 5: Run contract, schema, documentation, and maintainer checks.**

```powershell
python scripts/generate_contracts.py --check
python scripts/validate_schemas.py
python scripts/check_documentation.py
python scripts/validate_maintainer_artifacts.py
```

Expected: all commands exit 0 on a clean checkout. The implementation must not
change schemas or generated vocabulary. A shared worktree containing the
preserved unrelated untracked plan may report the known unregistered-document
failure; repeat the documentation command in a clean temporary worktree and
report both statuses separately.

- [ ] **Step 6: Run repository profiles.**

```powershell
python scripts/run_checks.py fast
python scripts/run_checks.py integration
python scripts/run_checks.py certification
```

Expected: every applicable command exits 0. Missing tools are `NOT_RUN`, and
production authority absence remains `BLOCKED`; neither status is a Slice 5
semantic pass.

- [ ] **Step 7: Recheck the final source diff and forbidden artifacts.**

```powershell
git diff --check origin/master...HEAD
git diff --name-only origin/master...HEAD
git status --short --branch
```

Expected tracked implementation files are limited to:

```text
docs/normative-document-register.v1.json
docs/superpowers/plans/2026-09-07-context-application-v2-slice5-supersession-currentness-implementation.md
scripts/context_application_v2_review_binding.py
scripts/context_application_v2_review_admission.py
scripts/context_application_v2_supersession.py
python/tests/context_application_v2_test_support.py
python/tests/test_context_application_v2_review_admission.py
python/tests/test_context_application_v2_supersession.py
python/tests/test_python_test_profiles.py
scripts/run_python_tests.py
```

No production `ae.v3`, `cpar.v2`, `cps.v2`, `cpsr.v2`, current-record file,
authority container, HostBinding artifact, C artifact, or generated production
fixture may appear.

- [ ] **Step 8: Commit no generated or production artifacts.**

If any forbidden artifact appears, stop, remove no unrelated user file, and
report the exact path and command that created it. Do not weaken a gate to
continue.

## Task 10: Exact-head handoff after implementation authorization

**Files:** No additional source edits expected.

- [ ] **Step 1: Record the exact final implementation head.**

```powershell
git rev-parse HEAD
git rev-parse origin/master
git status --short --branch
git diff --name-only origin/master...HEAD
```

Record the final commit, exact base, all changed files, every executed command,
and every `PASS`, `FAIL`, `NOT_RUN`, `SKIPPED`, or `BLOCKED` status. The
changed-file list must include the approved plan and register documents carried
from `PLAN_HEAD` in addition to the Slice 5 implementation files. Do not
call the implementation complete while a required gate is unknown.

- [ ] **Step 2: Re-fetch master and inspect semantic drift before delivery.**

```powershell
git fetch origin master
git rev-parse origin/master
git merge-base --is-ancestor origin/master HEAD
```

If `origin/master` moved after implementation started, do not silently rebase.
Compare the exact changed contract files and obtain a new base decision before
delivery.

- [ ] **Step 3: Push only the Slice 5 implementation branch.**

```powershell
git push --set-upstream origin chris/context-application-v2-slice5-supersession-currentness
```

Push only after all applicable local gates pass. Do not push production
authority artifacts or the unrelated untracked plan.

- [ ] **Step 4: Open one implementation PR and stop for independent review.**

The PR must state:

```text
SLICE 5 ONLY
```

and list as not implemented:

```text
production acceptance
production V3 events or authority records
HostBindingV2 integration
Buckle-Up human review
C derivation or ClassProjection
Task 5 Slice 3B
M3
Magic rules
cards
```

Wait for exact-head hosted PR Fast and CodeQL. Verify the hosted PR head is the
exact pushed implementation commit. Do not merge the implementation PR, do not
create production authority, and do not authorize Slice 6, Task 5 Slice 3B, or
M3 from local tests alone.

## Plan self-review

### Spec coverage

| ADR 0043/design requirement | Plan task |
|---|---|
| exact typed supersession admission | Tasks 4–5 |
| `cps.v2` recomputation | Task 5 |
| `cpsr.v2` recomputation from complete `ReviewEventRefV3` | Task 5 |
| exact supersession `AcceptanceSubjectPayloadV3` | Tasks 2 and 5 |
| full `DigestReferenceV1` equality | Task 2 |
| standalone host-free event closure | Tasks 2 and 5 |
| Slice 4 reviewer/evidence reuse and behavior preservation | Tasks 1–3 |
| record-ID graph nodes and `cps.v2` edges | Tasks 6–7 |
| all `cpsr.v2` revisions retained | Tasks 6–7 |
| distinct same-source `cps.v2` edges fail, even same target | Tasks 6–7 |
| self-edge, unknown target, successor, and cycle failures | Tasks 6–7 |
| application-wide revocation and cross-application isolation | Tasks 6–7 |
| currentness 0/1/>1 cardinality | Tasks 6–7 |
| admission-before-currentness | Tasks 5–7 |
| immutable tuple result and stable errors | Tasks 5–7 |
| input-order determinism and normalized cycle diagnostics | Tasks 6–7 and 9 |
| mutation safety/no partial cache | Tasks 4, 6, 7, and 9 |
| no HostBinding/Slice 6 behavior | all tasks; explicit boundary in Task 9 |
| no Rust policy or identity/schema change | file map and Task 9 |
| temporary fixtures/no production artifacts | Tasks 1, 4, 6, and 9 |

### Completeness and ambiguity review

The plan names every file, public interface, stable error category, test
scenario, command, expected outcome, and commit boundary. It does not leave a
semantic choice for implementation: ADR 0043 is the authority for replacement
lineage, duplicate `cpsr.v2` handling, revocation scope, and currentness.

The only permitted implementation decisions are local decomposition details
that preserve the listed interfaces and invariants. Any need to change a
persisted identity/schema/preimage, add HostBinding semantics, or create a
production artifact is a stop condition.

### Type and name consistency

The plan uses these names consistently:

```text
ContextApplicationV2V3ReviewBindingResult
ContextApplicationV2V3ReviewBindingError
admit_v3_review_binding
ContextApplicationV2SupersessionAdmissionResult
ContextApplicationV2SupersessionError
ContextApplicationV2SupersessionAdmissionValidator
ContextApplicationV2SupersessionEdge
ContextApplicationV2CurrentnessResult
ContextApplicationV2CurrentnessError
ContextApplicationV2CurrentnessEvaluator
```

The shared seam owns only V3 mechanical binding. The supersession validator
owns one-record mechanical admission. The evaluator owns graph validation,
application-wide revocation, and currentness derivation. Slice 3 semantics,
V1 behavior, schemas, identities, and Rust DTO ownership remain in their
existing modules.

## Plan-only status

```text
BASE=193493c610424bbf19bef30a25de14746c21dff5
REVIEWED_PLAN_HEAD=9f2e6de07e66edb302cfdb1ba140e55f5857b085
PLAN_HEAD=latest independently accepted docs-only plan/amendment tip
PLAN_ONLY=YES
IMPLEMENTATION_PLAN_CREATED=YES
IMPLEMENTATION_STARTED=NO
PRODUCTION_CODE_CHANGED=NO
PRODUCTION_ARTIFACT_CREATED=NO
SCHEMA_CHANGED=NO
IDENTITY_CHANGED=NO
PREIMAGE_CHANGED=NO
SLICE_6_AUTHORIZED=NO
TASK_5_SLICE_3B=BLOCKED
M3=BLOCKED
```

After the plan-only commit, stop for independent plan review. Do not execute
Task 1 or any later implementation task from this turn.
