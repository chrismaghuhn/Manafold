# ContextApplicationV2 Slice 4 — V3 Review Admission Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Implement read-only, fail-closed V3 review admission for a semantically valid ContextApplicationV2Record without changing V1/V2/V3 schema or identity contracts and without creating production authority artifacts.

**Architecture:** Add a repository-aware ContextApplicationV2ReviewAdmissionValidator that composes the existing Slice-3 semantic validator and Slice-2 resolver. Extend only the V3 resolver path with additive machine-readable diagnostics, reuse the existing V1 roster and exact-role machinery through a narrow typed seam, and return a frozen DTO containing only identities, references, canonical closure bindings, roles, and review mode.

**Tech Stack:** Python 3.13, unittest, frozen dataclasses, existing AuthoritySourceResolver, canonical CBOR/identity DTOs, JSON Schema/documentation validators, and Rust identity regression tests. No new Rust admission-policy implementation.

---

## Approved inputs and invariants

Implementation starts only from:

    docs/superpowers/specs/2026-09-06-context-application-v2-slice4-v3-review-admission-design.md

Approved design head before this plan:

    65997ea9a47cba38a671f867138f0c037fc5c1b6

Implementation-plan head before code execution:

    the exact final plan-only HEAD reported at this plan-review handoff

The implementation must preserve:

    V1 checklist, V1 event identity, V1 event schema, and accepted V1 records unchanged
    asp.v3, ae.v3, cpa.v2, and cpar.v2 unchanged
    ReviewAcceptanceEventInputV3, ReviewAcceptanceEventLeafV3, ReviewEventRefV3 unchanged
    AcceptanceSubjectPayloadV3 unchanged
    Slice-2 source closure as the only event/container closure walker
    Slice-3 semantic validation as the only V2 semantic validator
    no host bindings in Slice-4 event closure
    no production event, record, authority artifact, or human acceptance
    no supersession currentness or HostBinding semantic result

The required-role order is fixed and never selected by set iteration:

    (
        "architecture_maintainer",
        "rules_authority_maintainer",
        "conformance_maintainer",
        "information_safety_reviewer",
    )

The first missing role wins. Missing information safety returns INFORMATION_SAFETY_REVIEWER_REQUIRED; missing any earlier role returns REVIEWER_ROLE_MISSING.

## File map

| File | Responsibility |
|---|---|
| scripts/context_application_v2_resolver.py | Additive V3 resolver diagnostic codes; retain event, checklist, mode, and evidence ownership. |
| scripts/context_application_v2_validator.py | Preserve Slice-3 fallback semantic codes and expose the pure typed information-sensitivity inventory seam. |
| scripts/authority_source_resolver.py | Existing owner of raw roster bytes, digest/schema/path checks, closed JSON shape, and source-level ReviewerV1/ReviewerRosterV1 validation; no new implementation is planned here. |
| scripts/reviewer_role_binding.py | Calls the existing roster-leaf resolver exactly once, materializes typed values from its already-verified artifact projection, and owns only exact reviewer existence/complete-role equality; no raw parsing or ordering policy. |
| scripts/authority_validator.py | Delegate the existing V1 role-binding comparison through the typed seam without changing V1 diagnostics. |
| scripts/context_application_v2_review_admission.py | New admission module, stable errors, frozen result, semantic composition, event binding, closure, roster, and role policy. |
| docs/maintenance/INTERACTION_AUTHORITY_REVIEW_CHECKLIST_V2.md | New immutable V2 checklist definition. |
| docs/normative-document-register.v1.json | Checklist and process-artifact registration. |
| python/tests/test_context_application_v2_resolver.py | Resolver diagnostic and legacy compatibility tests. |
| python/tests/test_context_application_v2_validator.py | Slice-3 resolver-code fallback regression tests. |
| python/tests/test_authority_validator.py | V1 roster/role and shared-seam regression tests, including V1 reviewer-ID ordering. |
| python/tests/test_context_application_v2_contract.py | V3 full-CBOR reviewer-binding ordering regression. |
| python/tests/test_context_application_v2_review_admission.py | New positive, negative, precedence, mutation, determinism, and result-surface tests. |
| python/tests/test_review_admission_foundation.py | V1/V2 checklist registration and immutability assertions. |
| scripts/run_python_tests.py | Add the new admission module to the explicit smoke profile. |

No schema, Rust authority DTO, production fixture, production event, or production application record is added.

---

### Task 0: Verify the exact implementation baseline

**Files:** None modified.

- [ ] Step 1: Confirm branch, plan head, design/base identity, tracked state, and unrelated untracked state.

Run:

~~~powershell
git status --short --branch
git rev-parse HEAD
git rev-parse origin/master
git diff --name-only origin/master..HEAD
~~~

Expected:

    PLAN_HEAD = the exact final plan-only HEAD reported at this plan-review handoff
    origin/master = 0dfd646fc6b8b7e09fef69a9721eba8487425a46
    origin/master..HEAD contains the approved design file and implementation plan
    docs/superpowers/plans/2026-08-26-m2-5-b2-terminal-card-classification-closure.md remains untracked and unstaged

- [ ] Step 2: Re-read the approved design, ADR 0042, the Slice-2 resolver, the Slice-3 validator, and the V1 AuthorityValidator before editing.

- [ ] Step 3: Commit no code during baseline verification. Stop if the base or scope differs.

---

### Task 1: Add RED tests for the V3 resolver diagnostic seam

**Files:**
- Modify: python/tests/test_context_application_v2_resolver.py
- Modify: python/tests/test_context_application_v2_validator.py

- [ ] Step 1: Extend the existing temporary V3 event fixture with a two-mode helper that rewrites event JSON and writes only inside TemporaryDirectory.

The helper must expose these modes:

    IDENTITY_MISMATCH:
        mutate the wire, recompute raw SHA-256, keep the old event_id/path
        and construct a ReviewEventRefV3 with that old identity
        -> reaches V3_EVENT_IDENTITY_INVALID

    SELF_CONSISTENT_MUTATION:
        mutate the typed ReviewAcceptanceEventInputV3 fields
        recompute the new ae.v3 identity
        update event_id and content-addressed path
        serialize the new leaf, recompute raw SHA-256, and build a new ref
        -> reaches downstream roster/source/evidence/closure checks

The helper must never reuse an old event_id for a self-consistent mutation.

- [ ] Step 2: Add resolver code assertions for these mutations:

| Mutation | Expected code |
|---|---|
| top-level event shape or schema | V3_EVENT_SCHEMA_INVALID |
| decision other than human_accepted | V3_EVENT_DECISION_INVALID |
| checklist marker mismatch | CHECKLIST_V2_MISMATCH |
| invalid closed review mode | REVIEW_MODE_INVALID |
| JSON event ID differs from the reference identity | V3_EVENT_IDENTITY_INVALID |
| missing roster binding or self-leaf source | V3_EVENT_SOURCE_INVALID |
| malformed or empty review evidence | REVIEW_EVIDENCE_INVALID or REVIEW_EVIDENCE_MISSING |
| wrong reference type | V3_EVENT_REFERENCE_INVALID |

Each test must assert the structured code and the current human-readable message. No test may parse exception text to derive a category.

- [ ] Step 3: Add a legacy compatibility assertion for a non-V3 source/closure failure. It must retain the existing exception type/message and expose no V3 code. Add a Slice-3 regression proving an existing semantic error code remains unchanged when a legacy resolver failure is caught.

- [ ] Step 4: Run the RED tests:

~~~powershell
python -m unittest discover -s python/tests -p test_context_application_v2_resolver.py -v
python -m unittest discover -s python/tests -p test_context_application_v2_validator.py -v
~~~

Expected: the new code assertions fail because the resolver currently exposes only message text; existing unrelated tests retain their baseline status.

---

### Task 2: Implement the additive V3 resolver diagnostic seam

**Files:**
- Modify: scripts/context_application_v2_resolver.py
- Modify: scripts/context_application_v2_validator.py
- Test: python/tests/test_context_application_v2_resolver.py
- Test: python/tests/test_context_application_v2_validator.py

- [ ] Step 1: Add a closed V3 code set and an optional code field to ContextApplicationV2ResolutionError. Legacy calls to _fail pass no code; V3 event/evidence checks pass the new code. In the Slice-3 caller, use:

~~~python
resolver_code = getattr(exc, "code", None)
raise ContextApplicationV2SemanticValidationError(
    resolver_code or "MEMBER_SOURCE_BINDING_MISMATCH",
    label,
) from exc
~~~

Apply the same fallback pattern at the existing evidence-resolution catch.

- [ ] Step 2: Keep every existing resolver message unchanged and add code only at the owning V3 checks. Use this mapping:

    wrong reference type -> V3_EVENT_REFERENCE_INVALID
    wrong event shape/schema -> V3_EVENT_SCHEMA_INVALID
    wrong decision -> V3_EVENT_DECISION_INVALID
    wrong checklist -> CHECKLIST_V2_MISMATCH
    invalid ReviewMode -> REVIEW_MODE_INVALID
    event identity failure -> V3_EVENT_IDENTITY_INVALID
    source-list, roster-binding, or self-leaf failure -> V3_EVENT_SOURCE_INVALID
    empty review evidence -> REVIEW_EVIDENCE_MISSING
    malformed/unresolvable review evidence owned by this resolver -> REVIEW_EVIDENCE_INVALID

- [ ] Step 3: Do not wrap delegated ResolutionError instances from AuthoritySourceResolver into a new exception type. They already carry stable status/code/message. The admission layer maps them by call stage and keeps the original code as cause metadata.

- [ ] Step 4: Run focused GREEN tests:

~~~powershell
python -m unittest discover -s python/tests -p test_context_application_v2_resolver.py -v
python -m unittest discover -s python/tests -p test_context_application_v2_validator.py -v
~~~

Expected: PASS, including new codes, unchanged messages, and unchanged Slice-3 categories.

- [ ] Step 5: Commit:

~~~powershell
git add scripts/context_application_v2_resolver.py scripts/context_application_v2_validator.py python/tests/test_context_application_v2_resolver.py python/tests/test_context_application_v2_validator.py
git commit -m "refactor: add coded V3 resolver diagnostics"
~~~

---

### Task 3: Add the shared exact-role seam and preserve V1 behavior

**Files:**
- Create: scripts/reviewer_role_binding.py
- Modify: scripts/authority_validator.py
- Modify: python/tests/test_authority_validator.py
- Test: python/tests/test_context_application_v2_contract.py

- [ ] Step 1: Add RED tests for the single-owner roster parser, exact reviewer existence, complete roster-role equality, and the separate V1/V3 ordering rules. Spy on resolve_reviewer_roster_leaf and assert resolve_reviewer_roster calls it exactly once and consumes the verified artifact projection without a second raw-byte JSON parse. Add V1 regression cases preserving:

    unknown reviewer or role mismatch -> REVIEWER_ROLE_BINDING_MISMATCH
    empty bindings -> REVIEWER_ROLE_BINDING_INVALID
    duplicate or unsorted V1 IDs -> NONCANONICAL_REVIEWERS

Keep the existing V3 regression that proves full canonical-CBOR ordering of complete ReviewerRoleBinding tuples. The new shared helper must not sort, normalize, or reject based on either V1 ID ordering or V3 full-CBOR ordering.

- [ ] Step 2: Create a typed helper with this interface:

~~~python
@dataclass(frozen=True)
class ReviewerRoleBindingValidationError(ValueError):
    reason: Literal[
        "not_in_roster",
        "role_mismatch",
    ]
    reviewer_id: str | None = None


def resolve_reviewer_roster(
    source_resolver: AuthoritySourceResolver,
    reference: ReviewerRosterRefV1,
) -> ReviewerRosterV1:
    """Call resolve_reviewer_roster_leaf once and materialize its verified value."""


def validate_reviewer_binding_against_roster(
    binding: ReviewerRoleBindingV1,
    roster: ReviewerRosterV1,
) -> None:
    """Raise ReviewerRoleBindingValidationError unless the binding is exact."""
~~~

The implementation must call source_resolver.resolve_reviewer_roster_leaf(reference) exactly once, use its already-verified artifact.json_value projection, and never call json.loads(artifact.raw_bytes). It must not duplicate raw digest/schema/path/closed-shape validation. It may materialize ReviewerV1 and ReviewerRosterV1 from the verified projection. The exact-role helper owns only reviewer existence and complete role-tuple equality. It must not add role vocabulary, subset semantics, role escalation, reviewer-count rules, project-owner requirements, or ordering.

- [ ] Step 3: Replace AuthorityValidator._parse_roster raw JSON parsing with a delegation to resolve_reviewer_roster and preserve its current V1 error codes/messages. The raw source owner remains AuthoritySourceResolver.resolve_reviewer_roster_leaf; neither AuthorityValidator nor reviewer_role_binding.py calls json.loads on raw roster bytes. Keep AuthorityValidator._event_role_bindings as the V1 owner of reviewer-ID ordering and existing diagnostics. Slice 4 calls resolve_reviewer_roster and validate_reviewer_binding_against_roster, then explicitly rejects duplicate reviewer IDs while leaving V3 full-CBOR ordering to ReviewAcceptanceEventInputV3.

- [ ] Step 4: Run:

~~~powershell
python -m unittest discover -s python/tests -p test_authority_validator.py -v
python -m unittest discover -s python/tests -p test_context_application_v2_contract.py -v
~~~

Expected: PASS with existing V1 role and conditional information-safety behavior unchanged.

- [ ] Step 5: Commit:

~~~powershell
git add scripts/reviewer_role_binding.py scripts/authority_validator.py python/tests/test_authority_validator.py
git commit -m "refactor: share exact reviewer role binding checks"
~~~

---

### Task 4: Write RED admission-contract tests and fixture builders

**Files:**
- Create: python/tests/test_context_application_v2_review_admission.py

- [ ] Step 1: Build a temporary-repository fixture using the existing Slice-3 synthetic source pattern and AuthoritySourceResolverTests helpers. Return a source resolver, base binding, semantically valid ContextApplicationV2Record, embedded V3 event reference, event wire, and raw-file digest snapshot. Write all artifacts only under TemporaryDirectory.

- [ ] Step 2: Construct the V3 event from ReviewAcceptanceEventInputV3 and ReviewAcceptanceEventLeafV3, write its exact content-addressed path, and set the record review_event_ref_v3 to the actual raw digest and event ID. The default fixture must pass Slice-3 semantic validation and exact Slice-2 event closure. All downstream negative tests must use the SELF_CONSISTENT_MUTATION mode when they need to reach roster, source, evidence, or closure validation.

- [ ] Step 3: Add RED tests for the four positive controls:

    ordinary exact-match application -> PASS
    one valid reviewed-divergence member -> PASS
    typed information-sensitive values with the mandatory information role -> PASS
    solo_separate_self_review with all mechanical fields -> PASS without claiming temporal proof

- [ ] Step 4: Add the frozen result-surface assertion:

~~~python
result = validator.admit(record)
self.assertEqual(result.record_id, record.record_id)
self.assertEqual(result.application_id, record.application_id)
self.assertEqual(result.review_event_ref, record.review_event_ref_v3)
self.assertIsInstance(result.exact_event_closure, tuple)
self.assertFalse(hasattr(result, "artifact"))
self.assertFalse(hasattr(result, "resolved_event"))
with self.assertRaises(dataclasses.FrozenInstanceError):
    result.review_mode = ReviewMode.MULTI_REVIEWER
~~~

- [ ] Step 5: Run RED:

~~~powershell
python -m unittest discover -s python/tests -p test_context_application_v2_review_admission.py -v
~~~

Expected: module import failure because the admission module does not exist.

---

### Task 5: Implement the frozen admission module

**Files:**
- Create: scripts/context_application_v2_review_admission.py
- Test: python/tests/test_context_application_v2_review_admission.py

- [ ] Step 1: Define the frozen result and stable admission error types:

~~~python
REQUIRED_V2_ROLES: Final = (
    "architecture_maintainer",
    "rules_authority_maintainer",
    "conformance_maintainer",
    "information_safety_reviewer",
)


@dataclass(frozen=True)
class ContextApplicationV2ReviewAdmissionResult:
    record_id: AuthorityIdentityV1
    application_id: AuthorityIdentityV1
    subject_digest_reference: DigestReferenceV1
    review_event_ref: ReviewEventRefV3
    event_id: str
    exact_event_closure: tuple[ContextAuthoritySourceBindingV2, ...]
    reviewer_roster_ref: ReviewerRosterRefV1
    required_roles: tuple[str, ...]
    review_mode: ReviewMode
~~~

Define ContextApplicationV2ReviewAdmissionError with this structured surface:

~~~python
class ContextApplicationV2ReviewAdmissionError(ValueError):
    code: str
    location: str
    cause_code: str | None
    missing_role: str | None
    message: str

    def __init__(self, code: str, location: str, *, cause_code: str | None = None, missing_role: str | None = None) -> None:
        self.code = code
        self.location = location
        self.cause_code = cause_code
        self.missing_role = missing_role
        self.message = f"{code} at {location}"
        super().__init__(self.message)
~~~

The public code APPLICATION_INPUT_INVALID has highest precedence for a non-record input and must occur before Slice-3 or filesystem access. Do not expose ResolvedReviewAcceptanceEventV3, ResolvedArtifact, raw JSON, or mutable mappings in the result.

Add the diagnostic-only typed inventory in scripts/context_application_v2_validator.py:

~~~python
@dataclass(frozen=True)
class ContextApplicationV2InformationSensitivityInventory:
    fact_paths: tuple[str, ...]


def collect_information_sensitivity_facts(
    *,
    historical_source_values: Sequence[str],
    bridge_source_values: Sequence[str],
    bridge_reviewed_values: Sequence[str],
    theorem_context_values: Sequence[str],
    theorem_preconditions: Sequence[ContextPreconditionValueV1],
) -> ContextApplicationV2InformationSensitivityInventory:
    """Record only typed visibility/information facts in canonical order."""
~~~

The helper consumes only ContextPreconditionValueV1.precondition_id and the existing typed value shape. It recognizes the fixed source_context and class_projection payload forms already produced by Slice 3 and emits no fact for unknown shapes. It never reads card/capability names, theorem JSON mappings, rationale, evidence prose, filenames, or natural-language values. The unconditional information-safety role remains unchanged.

- [ ] Step 2: Implement the constructor with source resolver and base authority binding. Instantiate ContextApplicationV2Resolver and ContextApplicationV2SemanticValidator with the same base binding. Reject non-record input before filesystem access.

- [ ] Step 3: Implement admit in this exact order:

    1. validate record type
    2. run ContextApplicationV2SemanticValidator.validate
    3. resolve record.review_event_ref_v3 through resolve_review_event_leaf_v3
    4. require event.subject_kind == context_application_v2_record
    5. recompute AcceptanceSubjectPayloadV3 for context_application_v2_record
    6. compare the complete DigestReferenceV1
    7. reconstruct expected_acceptance_source_closure_v3 with no host bindings
    8. require exact source-set equality
    9. resolve the exact reviewer roster through resolve_reviewer_roster
    10. validate typed binding existence and complete roles
    11. explicitly reject duplicate reviewer IDs; leave V3 full-CBOR ordering to the V3 DTO
    12. enforce REQUIRED_V2_ROLES in tuple order
    13. consume resolver-verified mode/evidence postconditions
    14. return the frozen result

The roster step must call resolve_reviewer_roster, not a private AuthorityValidator method. The shared helper owns raw path/schema/digest/JSON parsing; AuthorityValidator delegates to it and maps its failures back to the existing V1 diagnostics.

After Slice-3 validation succeeds, collect a frozen diagnostic-only typed information-sensitivity inventory from the already typed V2 facts. The inventory must record deterministic fact paths for non-not_applicable visibility and information_relation values in historical source values, reviewed bridge source/reviewed values, theorem context values, source_context preconditions, and class_projection precondition context vectors. It must not change the unconditional role policy, inspect names/rationale/evidence prose, or perform a second semantic validation.

- [ ] Step 4: Map errors through an explicit table and never match message text:

    non-record public input -> APPLICATION_INPUT_INVALID
    Slice-3 semantic error -> SEMANTIC_VALIDATION_FAILED
    resolver-owned V3 code -> identical public code
    delegated event-leaf ResolutionError -> V3_EVENT_SOURCE_INVALID with cause_code
    delegated closure ResolutionError -> V3_SOURCE_CLOSURE_MISMATCH with cause_code
    complete subject-reference mismatch -> V3_SUBJECT_DIGEST_MISMATCH
    exact closure mismatch -> V3_SOURCE_CLOSURE_MISMATCH
    unknown roster or mismatched roster bytes -> REVIEWER_ROSTER_INVALID
    unknown reviewer -> REVIEWER_BINDING_NOT_IN_ROSTER
    role tuple mismatch -> REVIEWER_BINDING_INVALID
    duplicate reviewer ID -> REVIEWER_DUPLICATE
    first missing mandatory role other than information safety -> REVIEWER_ROLE_MISSING
    missing information role -> INFORMATION_SAFETY_REVIEWER_REQUIRED

- [ ] Step 5: Enforce the subject-kind binding before any digest comparison:

~~~python
if (
    resolved_event.event.subject_kind
    is not AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD
):
    raise ContextApplicationV2ReviewAdmissionError(
        "V3_SUBJECT_KIND_MISMATCH",
        "event.subject_kind",
    )
~~~

The test for this case must create a semantically self-consistent new ae.v3 whose subject kind is context_application_v2_supersession_record, recompute its event identity/path/raw digest, build a new ReviewEventRefV3, rebuild the ContextApplicationV2Record with ContextApplicationV2Record.from_parts using that new ref, recompute cpar.v2 record_id, and then pass the rebound record through the application admission path. Otherwise Slice 3 would correctly stop at RECORD_IDENTITY_MISMATCH before the SubjectKind check.

- [ ] Step 6: Compare the full subject reference:

~~~python
subject = AcceptanceSubjectPayloadV3(
    subject_kind=AcceptanceSubjectKindV3.CONTEXT_APPLICATION_V2_RECORD,
    subject_payload=record.acceptance_free_subject_payload(),
)
expected_subject_reference = DigestReferenceV1.from_identity(subject.identity())
if expected_subject_reference != resolved_event.event.subject_payload_digest_reference:
    raise ContextApplicationV2ReviewAdmissionError(
        "V3_SUBJECT_DIGEST_MISMATCH",
        "event.subject_payload_digest_reference",
    )
~~~

- [ ] Step 7: Reuse closure ownership exactly:

~~~python
expected_closure = self._resolver.expected_acceptance_source_closure_v3(
    record,
    resolved_event.event.reviewer_roster_ref,
)
require_exact_source_set(
    resolved_event.event.source_binding_digests,
    expected_closure,
)
~~~

Do not pass caller-selected host bindings and do not reimplement event/container closure walking.

- [ ] Step 8: Enforce the fixed role precedence:

~~~python
role_union = frozenset(
    role
    for binding in validated_bindings
    for role in binding.roles
)
for role in REQUIRED_V2_ROLES:
    if role not in role_union:
        code = (
            "INFORMATION_SAFETY_REVIEWER_REQUIRED"
            if role == "information_safety_reviewer"
            else "REVIEWER_ROLE_MISSING"
        )
        raise ContextApplicationV2ReviewAdmissionError(
            code,
            "reviewer_role_bindings." + role,
            missing_role=role,
        )
~~~

- [ ] Step 9: Return only typed/frozen references and tuples:

~~~python
return ContextApplicationV2ReviewAdmissionResult(
    record_id=record.record_id,
    application_id=record.application_id,
    subject_digest_reference=expected_subject_reference,
    review_event_ref=record.review_event_ref_v3,
    event_id=resolved_event.event_id,
    exact_event_closure=expected_closure,
    reviewer_roster_ref=resolved_event.event.reviewer_roster_ref,
    required_roles=REQUIRED_V2_ROLES,
    review_mode=resolved_event.event.review_mode,
)
~~~

- [ ] Step 10: Run the positive tests:

~~~powershell
python -m unittest discover -s python/tests -p test_context_application_v2_review_admission.py -v
~~~

Expected: all positive controls and result-surface assertions PASS.

- [ ] Step 11: Commit:

~~~powershell
git add scripts/context_application_v2_review_admission.py python/tests/test_context_application_v2_review_admission.py
git commit -m "feat: add ContextApplicationV2 V3 review admission"
~~~

---

### Task 6: Add the immutable Checklist V2 and document registration

**Files:**
- Create: docs/maintenance/INTERACTION_AUTHORITY_REVIEW_CHECKLIST_V2.md
- Modify: docs/normative-document-register.v1.json
- Modify: python/tests/test_review_admission_foundation.py

- [ ] Step 1: Add RED foundation assertions for the V2 path, checklist ID, registration metadata, and V1 file/content preservation.

- [ ] Step 2: Create the V2 definition with:

    Status: accepted V2 checklist definition
    Stability: accepted
    Checklist ID: interaction-authority-review-checklist.v2

State explicitly that V1 is inherited by reference and never rewritten. Add all ten historical source values, all ten reviewed context values, all four temporal values, derived exact_match/reviewed_divergence, positive evidence including not_applicable, divergence inequality, no normalization/lexical/capability/co-occurrence/absence inference, exact Candidate/SourceInstance/V1 preconditions, exact V3 closure, roster/roles, immutable provenance, and future supersession/revocation obligations.

- [ ] Step 3: Register the checklist exactly as:

~~~json
{
  "change_process": "governance-pr",
  "owner_role": "project-governance",
  "path": "docs/maintenance/INTERACTION_AUTHORITY_REVIEW_CHECKLIST_V2.md",
  "role": "process",
  "stability": "accepted"
}
~~~

Add process-pr / maintainer / provisional rows for the approved design and this plan using the existing register convention. Leave the V1 row unchanged.

- [ ] Step 4: Run:

~~~powershell
python -m unittest python.tests.test_review_admission_foundation -v
python scripts/check_documentation.py
python scripts/validate_maintainer_artifacts.py
~~~

Expected: PASS.

- [ ] Step 5: Commit:

~~~powershell
git add docs/maintenance/INTERACTION_AUTHORITY_REVIEW_CHECKLIST_V2.md docs/normative-document-register.v1.json python/tests/test_review_admission_foundation.py
git commit -m "docs: add interaction authority review checklist V2"
~~~

---

### Task 7: Add the complete negative matrix, precedence, mutation safety, and determinism tests

**Files:**
- Modify: python/tests/test_context_application_v2_review_admission.py
- Modify: python/tests/test_context_application_v2_resolver.py
- Modify: python/tests/test_authority_validator.py

- [ ] Step 1: Add event/reference mutations using the two fixture modes and assert:

    wrong ReviewEventRefV3 -> V3_EVENT_REFERENCE_INVALID
    wrong event raw digest -> delegated SOURCE_DIGEST_MISMATCH cause
    raw event JSON event_id differs from a valid reference -> V3_EVENT_IDENTITY_INVALID
    malformed ReviewEventRefV3 path/basename -> structural DTO/input rejection before admission
    wrong schema -> V3_EVENT_SCHEMA_INVALID
    wrong decision -> V3_EVENT_DECISION_INVALID
    wrong checklist -> CHECKLIST_V2_MISMATCH
    invalid mode -> REVIEW_MODE_INVALID
    SELF_CONSISTENT_MUTATION empty/stale/malformed/unresolved evidence -> REVIEW_EVIDENCE_MISSING/INVALID

- [ ] Step 2: Add subject mutations with actual validation-precedence coverage and assert:

    self-consistent supersession subject kind in an ae.v3 -> V3_SUBJECT_KIND_MISMATCH at event.subject_kind
    malformed envelope/algorithm/codec/domain/input schema -> resolver-owned V3 structural category before Slice-4 digest comparison
    semantically invalid application/theorem/member mutation -> SEMANTIC_VALIDATION_FAILED
    structurally valid asp.v3 metadata with digest bytes for another valid subject -> V3_SUBJECT_DIGEST_MISMATCH
    second semantically valid V2 record with correctly recomputed cpa.v2/cpar.v2 but the first record's event -> V3_SUBJECT_DIGEST_MISMATCH

Do not expect application_id mutation to reach the subject comparison; Slice-3 identity validation has precedence.

- [ ] Step 3: Add closure mutations and assert:

    missing/extra/stale binding -> V3_SOURCE_CLOSURE_MISMATCH
    wrong digest/schema/role/path -> V3_SOURCE_CLOSURE_MISMATCH
    duplicate role/path -> V3_SOURCE_CLOSURE_MISMATCH
    self-leaf/context-container source -> V3_SOURCE_CLOSURE_MISMATCH or V3_EVENT_SOURCE_INVALID
    unauthorized host-binding source -> V3_SOURCE_CLOSURE_MISMATCH

- [ ] Step 4: Add roster and role mutations:

    unknown/tampered/digest-mismatched roster -> REVIEWER_ROSTER_INVALID
    reviewer absent from roster -> REVIEWER_BINDING_NOT_IN_ROSTER
    role subset/superset/escalation -> REVIEWER_BINDING_INVALID
    duplicate reviewer ID with different role tuple -> REVIEWER_DUPLICATE
    missing architecture/rules/conformance -> REVIEWER_ROLE_MISSING
    missing information safety -> INFORMATION_SAFETY_REVIEWER_REQUIRED

Use a table-driven precedence test and require architecture, then rules, then conformance, then information safety precedence. Do not derive the expected error by iterating a set.

- [ ] Step 5: Add mode-policy tests:

    multi_reviewer with one selected reviewer passes when all required roles are present
    solo_separate_self_review passes with complete mechanical fields
    solo mode cannot waive a role or evidence
    no result claims temporal separation, reviewer independence, or prose sufficiency

- [ ] Step 6: Add typed information-sensitivity inventory tests. Assert that visibility and information_relation facts are recorded for historical source values, bridge source/reviewed values, theorem context values, source_context preconditions, and class_projection context vectors. Mutating card names, capability names, rationale, filenames, or evidence prose must not add or remove inventory facts. The test must also assert that the inventory does not change the always-required information-safety role policy.

- [ ] Step 7: Add mutation-safety snapshots for the record, members, event reference, resolver DTOs, source files, candidate universe, base authority, event files, evidence, and C. Every rejected admission must preserve the snapshots and create no accepted record.

- [ ] Step 8: Add deterministic repeatability tests. Identical inputs must produce identical result values or identical code/location/cause_code/missing_role/message tuples. No clock, username, network, absolute path, random value, or filesystem enumeration may influence the result.

- [ ] Step 9: Run:

~~~powershell
python -m unittest discover -s python/tests -p test_context_application_v2_review_admission.py -v
python -m unittest discover -s python/tests -p test_context_application_v2_resolver.py -v
python -m unittest discover -s python/tests -p test_authority_validator.py -v
~~~

Expected: PASS for all positive, negative, precedence, mutation, and deterministic cases.

- [ ] Step 10: Commit:

~~~powershell
git add python/tests/test_context_application_v2_review_admission.py python/tests/test_context_application_v2_resolver.py python/tests/test_authority_validator.py
git commit -m "test: close ContextApplicationV2 Slice 4 admission matrix"
~~~

---

### Task 8: Register the smoke profile and preserve existing golden coverage

**Files:**
- Modify: scripts/run_python_tests.py
- Test: python/tests/test_context_application_v2_contract.py
- Test: python/tests/test_context_application_v2_validator.py
- Test: python/tests/test_context_application_v2_resolver.py

- [ ] Step 1: Add test_context_application_v2_review_admission to SMOKE_TESTS next to the existing ContextApplicationV2 contract, validator, and resolver modules.

- [ ] Step 2: Run:

~~~powershell
python scripts/run_python_tests.py --profile smoke
python scripts/run_python_tests.py --profile full
cargo test -p mtgml-persistence --all-features --locked
~~~

Expected: PASS. Existing Slice-2 closure, Slice-3 semantic, identity, schema, V1 authority, and review-foundation tests remain green.

- [ ] Step 3: Commit:

~~~powershell
git add scripts/run_python_tests.py
git commit -m "test: include Slice 4 admission in Python smoke profile"
~~~

---

### Task 9: Run repository-wide implementation gates

**Files:** No source edits expected.

- [ ] Step 1: Run formatting, lint, type, schema, documentation, and maintainer checks:

~~~powershell
ruff format --check python scripts
ruff check python scripts
mypy --config-file python/pyproject.toml
cargo fmt --all -- --check
python scripts/generate_contracts.py --check
python scripts/validate_schemas.py
python scripts/check_documentation.py
python scripts/validate_maintainer_artifacts.py
git diff --check
~~~

Expected: PASS for every executed command. Missing tools are NOT_RUN, never PASS.

- [ ] Step 2: Run repository profiles:

~~~powershell
python scripts/run_checks.py fast
python scripts/run_checks.py integration
python scripts/run_checks.py certification
~~~

Required:

    FAST=PASS
    INTEGRATION=PASS
    CERTIFICATION=PASS

- [ ] Step 3: Run full Rust gates:

~~~powershell
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
~~~

Expected: PASS. No Rust admission policy or identity implementation is added.

- [ ] Step 4: Inspect final scope:

~~~powershell
git status --short --branch
git diff --name-only origin/master..HEAD
git diff --check origin/master..HEAD
git ls-files --others --exclude-standard
~~~

Tracked implementation scope must contain only the planned resolver, compatibility, reviewer seam, admission module, checklist/register, smoke-profile, and test files. The pre-existing unrelated untracked plan remains untracked and unstaged.

- [ ] Step 5: Commit no generated production event or authority record. Preserve the approved design commits; do not amend or rebase them.

---

### Task 10: Exact-head handoff and independent implementation review

**Files:** No additional source files.

- [ ] Step 1: Record exact final source identity:

~~~powershell
git rev-parse HEAD
git log -1 --oneline
git status --short --branch
~~~

Report every executed gate with PASS, FAIL, NOT_RUN, or BLOCKED and identify the exact head used.

- [ ] Step 2: After all local implementation gates PASS, audit current master/base drift and push the final exact commit:

    chris/context-application-v2-slice4-v3-review-admission

Run git fetch origin master, inspect git diff origin/master..HEAD, and record the actual current origin/master before creating the PR. Do not silently rebase over semantic changes.

Create the PR against current master with title:

    M2.5.C: implement ContextApplicationV2 review admission slice 4

The PR body begins with SLICE 4 ONLY, lists implemented behavior, and lists production acceptance, production events/records, supersession currentness, HostBinding semantics, C, Slice 3B, and M3 as not implemented. Do not merge.

- [ ] Step 3: Wait for PR Fast and CodeQL, verify that the PR head equals the exact local commit, and report workflow run IDs and job conclusions. Then stop for independent ChatGPT implementation review. Do not merge and do not authorize Slice 5, Slice 6, Task 5 Slice 3B, M3, production acceptance, or contract freeze from local tests alone.

## Plan self-review checklist

- [ ] Every approved design boundary maps to a task.
- [ ] V3 resolver parsing, checklist, mode, and evidence ownership remains in the existing resolver.
- [ ] Slice-3 semantic validation is composed, not duplicated.
- [ ] Full DigestReferenceV1 equality is tested.
- [ ] Event subject_kind is checked before subject digest comparison.
- [ ] Role failure precedence is an ordered tuple.
- [ ] V1 reviewer-ID ordering and V3 full-CBOR ordering remain separate owners.
- [ ] The result contains no ResolvedArtifact or JSON graph.
- [ ] V1 roster, role, checklist, and conditional information-safety behavior has regression coverage.
- [ ] Exact-match, reviewed-divergence, information-sensitive, and solo positive controls exist.
- [ ] The required negative matrix, mutation safety, and deterministic repeatability exist.
- [ ] Identity-mismatch and self-consistent event mutation fixtures are distinct.
- [ ] Subject-error tests respect resolver and Slice-3 validation precedence.
- [ ] The typed information-sensitivity inventory is exercised without lexical/prose inputs.
- [ ] Checklist V2 registration metadata is exact.
- [ ] PR creation precedes hosted Fast/CodeQL verification and independent implementation review.
- [ ] No production authority/event/record creation or Slice-5/6/M3 work is included.
- [ ] No incomplete or underspecified step remains in this plan.
