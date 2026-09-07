
# ContextApplicationV2 Slice 6 — ApplicationHostBindingV2 Integration Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Status:** provisional; plan-only; execution not authorized

**Goal:** Add a read-only, deterministic Slice-6 composition module that qualifies current ContextApplicationV2 records with exact ApplicationHostBindingV2 and HostBinding claim closure without changing persisted contracts.

**Architecture:** Compose the accepted Slice-5 ContextApplication currentness read model with one extracted HostBinding authority read model. Keep ae.v3 event closures host-free; add HostBinding authority/claim provenance only to the ContextApplicationAuthorityV2 container closure. Derive required HostBinding members from the verified candidate predicate scope == cross_deck and relation == directional_binary for both current and historical-only links; currentness controls qualification, not member closure.

**Tech Stack:** Python 3.11–3.14, frozen dataclasses, existing AuthoritySourceResolver, existing ContextApplication Slice-2/3/4/5 modules, existing HostBinding V2 validator/source resolver, unittest, canonical CBOR/digest-envelope primitives, temporary repositories, and repository verification scripts.

**Plan baseline:** 20fb8d92cdac27fc0dc61060223ffd7d7fa00cb5

**Plan review base:** d2033223f73e84c7b3d3d49b0984ffbf0888a5a5

**Implementation base rule:** the implementation branch starts from the exact
final plan commit reviewed after this amendment. It must not start from the
contract head or the pre-amendment plan review base.

**Master baseline:** d647e63d7687bd2c022393feaee5bac6736e84ca

**Contract authority:** ADR 0042, ADR 0043, accepted ADR 0044, and the final Slice-6 design document.

**Implementation authorization:** NO at plan authoring time. The user authorized this plan only. Implementation starts only after an independent exact-plan review and separate authorization.

---

## 1. Non-negotiable contract invariants

Every implementation task below must preserve these accepted decisions.

### G1: two closure layers

The public V3 event closure remains:

`
ae.v3 event closure
    = existing ContextApplication/Supersession closure
    - HostBinding authority and claim-record bindings
`

The container closure is additive:

`
ContextApplicationAuthorityV2 container closure
    = static bindings
    + acceptance-event leaf bindings
    + host-free event closures
    + HostBinding authority provenance
    + referenced HostBinding claim-record provenance
`

No implementation may pass HostBinding bindings into the public
ContextApplicationV2ReviewAdmissionValidator or change an existing ae.v3,
asp.v3, cpar.v2, cps.v2, or cpsr.v2 identity.

### G2: verified applicability subset

For every exact admitted ContextApplication member, resolve the candidate and
source instance through ContextApplicationV2Resolver.resolve_member_source_instance.
The required HostBinding subset is exactly:

`
candidate_record["scope"]    == "cross_deck"
candidate_record["relation"] == "directional_binary"
`

This predicate applies to current and historical-only cpa links. It is not
derived from capability names, card names, Oracle text, array order, file
order, discovery order, timestamps, or co-occurrence.

For a non-empty required subset, a cpa-level link must contain exactly that
member-key union. For an empty required subset, no cpa-level link is allowed.
Current versus historical-only status affects authority qualification only; it
does not change the required member subset.

### G3: current versus historical HostBinding claims

The HostBinding read model must keep these identity levels separate:

`
hbc.v1  = semantic claim identity
hbcr.v1 = accepted claim-record identity
hbcs.v1 = claim-record supersession identity
`

The internal frozen read model must contain at least:

`
admitted_claims_by_id: hbc.v1 -> claim
current_claims_by_id: hbc.v1 -> claim
current_claims_by_member: ApplicationMemberKeyV1 -> hbc.v1
claim_record_status_by_record_id: hbcr.v1 -> current | superseded | revoked
claim_record_ids_by_claim_id: hbc.v1 -> tuple[hbcr.v1, ...]
`

Current cpa links require current hbc authority. Historical-only links require
an exact known/admitted hbc claim and preserve the exact record-level status;
they never qualify current authority. Unknown or unadmitted historical claims
fail closed. Historical claim IDs are never rewritten to replacement claim IDs.

### Other frozen invariants

- Slice 5 remains the only cpar/cps/cpsr currentness authority.
- ApplicationHostBindingV2 remains cpa.v2-keyed, never cpar.v2-keyed.
- Currentness runs before HostBinding source evaluation.
- Current and historical links use exact candidate ID, complete candidate
  digest, source-instance ID, observed relationship, and shared snapshots.
- No new schema, identity preimage, Rust DTO, persisted current index, player
  API, production authority artifact, Buckle-Up artifact, C artifact, Task 5
  Slice 3B work, or M3 work is in scope.
- All tests use temporary repositories and synthetic typed inputs. No test may
  create a production ae.v3, cpar/cps record, hbc/hbcr record, or authority
  artifact.

## 2. File ownership and scope map

### Files to create

`
scripts/context_application_v2_host_binding.py
    Sole Slice-6 composition module, frozen result, status/error types, and
    deterministic link/member validation.

python/tests/test_context_application_v2_host_binding.py
    Slice-6 RED/GREEN matrix, permutations, error fingerprints, and mutation
    snapshots.

docs/superpowers/plans/2026-09-07-context-application-v2-slice6-host-binding-integration-implementation.md
    This plan only; no code is authorized by its presence.
`

### Files to modify later

`
scripts/authority_v2_validator.py
    Extract a frozen HostBinding V2 read model while preserving validate()'s
    existing public result and lifecycle semantics.

scripts/context_application_v2_resolver.py
    Separate host-free event closure reconstruction from container-level
    HostBinding provenance in expected_container_source_closure_v2().

python/tests/test_authority_v2_validator.py
    Prove read-model extraction preserves existing HostBinding behavior.

python/tests/test_context_application_v2_resolver.py
    Prove application and supersession ae.v3 closures remain host-free while
    container closure contains HostBinding provenance separately.

python/tests/test_context_application_v2_review_admission.py
    Add or retain explicit host-source rejection at the public Slice-4 seam.

scripts/run_python_tests.py
    Add the new Slice-6 integration test module to the intentional smoke
    allowlist.

python/tests/test_python_test_profiles.py
    Update the closed smoke-profile expectations for that allowlist addition.
`

### Files that must remain unchanged

`
python/src/mtgml/authority.py
python/src/mtgml/host_binding.py
schemas/context-application-authority.v2.schema.json
schemas/interaction-review-authority.v2.schema.json
crates/mtgml-persistence/src/authority.rs
docs/adr/0042-context-application-v2-reviewed-context-bridge.md
docs/adr/0043-context-application-v2-supersession-lineage-revocation-currentness.md
docs/adr/0044-context-application-v2-slice6-host-binding-integration.md
`

Any required change to an unchanged file is a STOP condition and must be
reported as a new contract gap before implementation continues.

## 3. Public and internal types to implement

The implementation must use frozen dataclasses and tuple-backed collections.
The exact public shape is:

`
class ApplicationHostBindingStatus(StrEnum):
    QUALIFIED_CURRENT = "qualified_current"
    HISTORICAL_ONLY = "historical_only"


class HostBindingClaimRecordStatus(StrEnum):
    CURRENT = "current"
    SUPERSEDED = "superseded"
    REVOKED = "revoked"


@dataclass(frozen=True)
class ApplicationHostBindingResult:
    application_id: AuthorityIdentityV1
    status: ApplicationHostBindingStatus
    host_binding_claim_ids: tuple[str, ...]


@dataclass(frozen=True)
class ContextApplicationV2HostBindingEvaluationResult:
    currentness: ContextApplicationV2CurrentnessResult
    qualified_current_application_record_ids: tuple[AuthorityIdentityV1, ...]
    application_host_binding_results: tuple[ApplicationHostBindingResult, ...]
    current_host_claim_ids: tuple[str, ...]
`

The internal HostBindingAuthorityV2ReadModel must additionally retain exact
record-level status and claim-to-record provenance:

`
@dataclass(frozen=True)
class HostBindingAuthorityV2ReadModel:
    base_authority_v1_binding: HostBindingSourceBindingV2
    candidate_universe_binding: HostBindingSourceBindingV2 | None
    admitted_claims_by_id: tuple[
        tuple[str, CrossDeckHostBindingClaimV1], ...
    ]
    current_claims_by_id: tuple[
        tuple[str, CrossDeckHostBindingClaimV1], ...
    ]
    current_claims_by_member: tuple[
        tuple[ApplicationMemberKeyV1, str], ...
    ]
    claim_record_status_by_record_id: tuple[
        tuple[str, HostBindingClaimRecordStatus], ...
    ]
    claim_record_ids_by_claim_id: tuple[
        tuple[str, tuple[str, ...]], ...
    ]
    used_source_bindings: tuple[HostBindingSourceBindingV2, ...]
`

The reusable validator seam returns both the existing compatibility result and
the frozen read model from one validation/source-resolution traversal:

`
@dataclass(frozen=True)
class AuthorityV2AdmissionResult:
    validation_result: AuthorityV2ValidationResult
    read_model: HostBindingAuthorityV2ReadModel


class AuthorityV2Validator:
    def admit(self, value: object) -> AuthorityV2AdmissionResult:
        return self._admit_document_with_read_model(value)

    def validate(self, value: object) -> AuthorityV2ValidationResult:
        return self.admit(value).validation_result
`

The public evaluator must be:

`
class ContextApplicationV2HostBindingEvaluator:
    def __init__(self, source_resolver: AuthoritySourceResolver) -> None:
        self._source_resolver = source_resolver

    def evaluate(
        self,
        container: ContextApplicationAuthorityV2,
    ) -> ContextApplicationV2HostBindingEvaluationResult:
        return self._evaluate_container(container)
`

The caller cannot provide a trusted/current boolean, an already-admitted
HostBinding map, a selected claim record, or a caller-selected event closure.

The public error must be frozen and structured:

`
class ContextApplicationV2HostBindingError(ValueError):
    code: str
    location: str
    cause_code: str | None
    application_id: AuthorityIdentityV1 | None
    record_id: AuthorityIdentityV1 | None
    claim_id: str | None
    member_key: ApplicationMemberKeyV1 | None
    subject_ids: tuple[str, ...]
`

Use the closed codes from ADR 0044. Do not classify errors by parsing
exception messages.

## 4. Task 0 — Baseline and authorization guard

**Files:** none.

- [ ] **Step 1: Verify exact source and remote heads.**

Run from the implementation worktree before any code or test change:

`
git fetch origin master
git rev-parse HEAD
git rev-parse origin/master
git status --short --branch
git ls-files --others --exclude-standard
`

Expected at plan review baseline:

`
CONTRACT_HEAD        = 20fb8d92cdac27fc0dc61060223ffd7d7fa00cb5
PLAN_REVIEW_BASE     = d2033223f73e84c7b3d3d49b0984ffbf0888a5a5
PLAN_HEAD            = final reviewed plan branch tip
HEAD                 = PLAN_HEAD
origin/master        = d647e63d7687bd2c022393feaee5bac6736e84ca
worktree             = clean except explicitly preserved user files
`

- [ ] **Step 2: Confirm the accepted contract inputs.**

Read these exact files again at the execution head:

`
docs/adr/0042-context-application-v2-reviewed-context-bridge.md
docs/adr/0043-context-application-v2-supersession-lineage-revocation-currentness.md
docs/adr/0044-context-application-v2-slice6-host-binding-integration.md
docs/superpowers/specs/2026-09-07-context-application-v2-slice6-host-binding-integration-design.md
`

Confirm that G1, G2, G3, the host-free event closure, the historical-only
policy, and the record-level status model are unchanged. If any accepted input
has drifted, stop and obtain a new exact-head review.

- [ ] **Step 3: Confirm no implementation authorization has been broadened.**

Record these execution guards before continuing:

`
SLICE_6_IMPLEMENTATION_PLAN = AUTHORIZED
SLICE_6_IMPLEMENTATION      = NOT_AUTHORIZED until this plan is independently reviewed
PRODUCTION_AUTHORITY        = NOT_AUTHORIZED
BUCKLE_UP_CANARY            = NOT_AUTHORIZED
TASK_5_SLICE_3B             = BLOCKED
M3                           = BLOCKED
`

No source, schema, fixture, or production-artifact edit occurs in Task 0.

## 5. Task 1 — RED tests for the G1 closure split

**Files:**
- Modify: scripts/context_application_v2_resolver.py
- Modify: python/tests/test_context_application_v2_resolver.py
- Modify: python/tests/test_context_application_v2_review_admission.py

- [ ] **Step 1: Add a RED application-event host-source regression.**

Use the existing temporary-repository fixture and a typed
ContextApplicationV2Record. Build an event whose source list contains a
valid host_binding_authority_v2 or host_binding_claim_record binding in
addition to the host-free application closure. Assert that public Slice-4
admission fails with the existing structured closure failure, not a new
success path:

`
with self.assertRaises(ContextApplicationV2ReviewAdmissionError) as error:
    validator.admit(record)

self.assertEqual(error.exception.code, "V3_SOURCE_CLOSURE_MISMATCH")
`

- [ ] **Step 2: Add a RED container/event separation regression.**

Build a synthetic container with one valid application record, one cpa-level
HostBinding link, the host authority binding, and its claim-record binding.
The expected assertions are:

`
expected_event = resolver.resolve_review_event_leaf_v3(record.review_event_ref_v3)
assert host_authority_binding not in expected_event.event.source_binding_digests
assert claim_binding not in expected_event.event.source_binding_digests

expected_container = resolver.expected_container_source_closure_v2(container)
assert host_authority_binding in expected_container
assert claim_binding in expected_container
`

The RED expectation is that the current private container path incorrectly
passes HostBinding bindings into the application event closure or cannot keep
the two sets distinct.

- [ ] **Step 3: Add the supersession-event host-free regression.**

Use the existing supersession fixture. Insert a valid HostBinding source into the
V3 supersession event's source list and assert that the host-free supersession
admission rejects it with the stable V3 source-closure category. This protects
the no-HostBinding rule for both V3 subject families.

- [ ] **Step 4: Run only the new focused tests and record RED evidence.**

Use the src-layout path configuration used by the repository runner:

`
$taskPythonPath = "python/src;python/tests"
$env:PYTHONPATH = $taskPythonPath
python -m unittest test_context_application_v2_resolver test_context_application_v2_review_admission -v
Remove-Item Env:PYTHONPATH
`

Expected: the new assertions fail against the current private container
behavior, while existing host-free Slice-4 tests continue to pass. Do not
modify production code in this task before the RED result is observed.

- [ ] **Step 5: Remove the ContextApplication-specific HostBinding event seam.**

Change ContextApplicationV2Resolver so that
_expected_acceptance_source_closure_v3 has no host_bindings parameter. Keep
the public expected_acceptance_source_closure_v3 and validate_event_source_closure_v3
host-free. Leave the generic reconstruct_event_source_closure helper's
host_bindings parameter untouched because the existing closure algebra and
cross-language parity tests cover that generic helper.

- [ ] **Step 6: Keep HostBinding provenance at container level only.**

In expected_container_source_closure_v2, reconstruct every event closure with
the host-free ContextApplication path, then pass HostBinding bindings only to
reconstruct_container_source_closure:

@@
expected_event = self._expected_acceptance_source_closure_v3(
    record,
    resolved_event.event.reviewer_roster_ref,
    base_authority_binding=container.base_authority_v1_binding,
)
require_exact_source_set(
    resolved_event.event.source_binding_digests,
    expected_event,
)

expected = reconstruct_container_source_closure(
    static_bindings=(
        container.base_authority_v1_binding,
        container.candidate_universe_binding,
    ),
    event_leaf_bindings=tuple(event_leaf_bindings),
    event_closures=tuple(event_closures),
    host_bindings=host_bindings,
)
@@
No ContextApplication-specific private path may inject HostBinding into an
ae.v3 source list after this task.

- [ ] **Step 7: Run the G1 GREEN tests immediately after the RED tests.**

@@
$env:PYTHONPATH = "python/src;python/tests"
python -m unittest test_context_application_v2_resolver test_context_application_v2_review_admission -v
Remove-Item Env:PYTHONPATH
@@

Expected: event source bindings remain host-free, container source bindings
include HostBinding provenance separately, and extra HostBinding event sources
fail closed.

- [ ] **Step 8: Commit the isolated G1 change.**

@@
git add scripts/context_application_v2_resolver.py python/tests/test_context_application_v2_resolver.py python/tests/test_context_application_v2_review_admission.py
git diff --cached --check
git commit -m "fix: keep Slice 6 HostBinding provenance out of ae.v3 closures"
@@

The commit must contain only the G1 resolver/test changes.

## 6. Task 2 — RED/GREEN extraction of the HostBinding read model

**Files:**
- Modify: scripts/authority_v2_validator.py
- Modify: python/tests/test_authority_v2_validator.py

- [ ] **Step 1: Add the frozen status and read-model types.**

Add the internal HostBindingClaimRecordStatus enum and frozen
HostBindingAuthorityV2ReadModel with exactly these fields:

`
admitted_claims_by_id
current_claims_by_id
current_claims_by_member
claim_record_status_by_record_id
claim_record_ids_by_claim_id
base_authority_v1_binding
candidate_universe_binding
used_source_bindings
`

Add AuthorityV2AdmissionResult with validation_result and read_model. The
public compatibility validate() method must remain a projection over admit(),
so Slice 6 consumes the same successful admission without a second source
resolution or lifecycle traversal.

All mappings are represented as canonical tuples. No mutable dictionaries are
returned and no new persisted DTO is added.

- [ ] **Step 2: Add RED tests for claim-record status.**

Extend the existing synthetic HostBinding V2 fixtures with:

`
H  = one hbc.v1 claim
R1 = hbcr.v1 record containing H
R2 = a second accepted record containing H
R1 -> R2 = accepted record supersession
R3 = a different accepted record containing H
R3 -> null = authority revocation
`

Assert the read model distinguishes exact record IDs and does not aggregate
ambiguous status onto hbc.v1:

`
claim_record_ids_by_claim_id[H] contains every admitted hbcr.v1 ID
claim_record_status_by_record_id[R1] is superseded by its exact edge
claim_record_status_by_record_id[R3] is revoked
current_claims_by_id contains only semantic claims with a current record
current_claims_by_member contains only current member authority
`

Use separate fixtures for same-claim acceptance revisions and a replacement
claim identity. Assert that the existing AuthorityV2Validator.validate()
result and counts remain unchanged.

- [ ] **Step 3: Run the RED read-model tests.**

`
$env:PYTHONPATH = "python/src;python/tests"
python -m unittest test_authority_v2_validator -v
Remove-Item Env:PYTHONPATH
`

Expected: the new read-model access fails because the extraction seam does not
yet exist. Existing validator behavior remains the compatibility baseline.

- [ ] **Step 4: Extract the existing lifecycle result without duplicating it.**

Refactor the existing AuthorityV2Validator implementation behind an internal
typed admission seam that returns AuthorityV2AdmissionResult after all
existing checks pass. Keep the public compatibility method:

`
def admit(self, value: object) -> AuthorityV2AdmissionResult:
    return self._admit_document_with_read_model(value)

def validate(self, value: object) -> AuthorityV2ValidationResult:
    return self.admit(value).validation_result
`

The extraction must continue to use the existing successor_by_record,
superseded_record_ids, current-record filtering, correlated source resolver,
reviewer/event checks, and exact source closure. Do not introduce a second
supersession traversal.

Derive statuses from exact hbcr.v1 record IDs:

`
record is source of authority_revocation edge
    -> revoked
record is source of non-revocation successor edge
    -> superseded
record remains eligible as the current record revision
    -> current
`

When multiple records carry one hbc semantic claim, retain every record ID in
claim_record_ids_by_claim_id; only the existing validator's current-record
rules determine current_claims_by_id.

- [ ] **Step 5: Add typed validator error causes before Slice-6 consumption.**

Extend AuthorityV2ValidationError with a stable code, location, and structured
subject fields while preserving existing human-readable message text. Use a
single internal error constructor/helper so existing validation branches do
not classify by message text. At minimum expose:

`
AUTHORITY_V2_INVALID
HOST_BINDING_AMBIGUOUS
HOST_CLAIM_RECORD_INVALID
HOST_SOURCE_INVALID
HOST_SOURCE_CLOSURE_MISMATCH
`

The existing multiple-current-claim/member path must raise
HOST_BINDING_AMBIGUOUS with the exact member key and claim IDs. General root,
source, schema, and closure failures must carry typed causes that Slice 6 can
map to HOST_AUTHORITY_INVALID or HOST_SOURCE_CLOSURE_MISMATCH without parsing
exception messages.

- [ ] **Step 6: Run the GREEN HostBinding suite.**

`
$env:PYTHONPATH = "python/src;python/tests"
python -m unittest test_authority_v2_validator -v
Remove-Item Env:PYTHONPATH
`

Expected: the HostBinding validator tests pass, including the new record-level
status tests. Do not claim Slice-6 behavior; this only proves the reusable
HostBinding seam. The repository smoke profile is not evidence for these new
read-model tests because test_authority_v2_validator is not in the smoke
allowlist; the focused validator suite is the Task-2 evidence. Broader smoke
and integration gates run after the Slice-6 test module is added.

## 7. Task 3 — Build the Slice-6 typed composition seam

**Files:**
- Create: scripts/context_application_v2_host_binding.py
- Test: python/tests/test_context_application_v2_host_binding.py
- Modify: scripts/run_python_tests.py
- Modify: python/tests/test_python_test_profiles.py

- [ ] **Step 1: Add RED structural-entrypoint tests.**

Test that the public evaluator rejects:

`
non-ContextApplicationAuthorityV2 input
duplicate cpar.v2/cpsr.v2/link identities
caller-provided trusted/current booleans (the interface has no such argument)
unknown application_host_binding target cpa
`

Test noncanonical values at the correct boundary:

`
noncanonical wire/constructor input
    -> AuthorityContractError before evaluator entry

test-only forged typed container bypassing construction
    -> HOST_INTEGRATION_INPUT_INVALID at evaluator entry
`

Do not claim that a normally constructed frozen container can contain a
noncanonical collection; its constructor already rejects that state.

Assert stable HOST_INTEGRATION_INPUT_INVALID,
APPLICATION_HOST_BINDING_DUPLICATE, or
APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION codes and frozen error fields.

- [ ] **Step 2: Implement the evaluator skeleton.**

The only public call is:

`
def evaluate(
    self,
    container: ContextApplicationAuthorityV2,
) -> ContextApplicationV2HostBindingEvaluationResult:
    return self._evaluate_container(container)
`

The implementation sequence is fixed:

`
typed/projection preflight
-> ContextApplicationV2CurrentnessEvaluator.evaluate(...)
-> derive cpa member applicability
-> classify current/historical links
-> resolve/admit HostBinding authority
-> validate links/claims
-> validate container closure
-> frozen result
`

Do not create a second ContextApplication or HostBinding validator.

- [ ] **Step 3: Add currentness-first tests.**

Build a real Slice-5-invalid graph, such as A -> B and B -> A, plus a
malformed/unknown HostBinding source. Assert that the Slice-5 cycle failure is
returned before HostBinding source access. Use a resolver spy or
temporary-repository file snapshot to prove the HostBinding authority and claim
files were not read.

- [ ] **Step 4: Run the focused structural suite.**

`
$env:PYTHONPATH = "python/src;python/tests"
python -m unittest test_context_application_v2_host_binding -v
Remove-Item Env:PYTHONPATH
`

Expected: GREEN for the structural and currentness-first tests.

- [ ] **Step 5: Add the new Slice-6 test module to the smoke allowlist.**

Add test_context_application_v2_host_binding to SMOKE_TESTS in
scripts/run_python_tests.py and update the explicit profile expectations in
python/tests/test_python_test_profiles.py. Do not add the new test module to
the allowlist before it exists.

- [ ] **Step 6: Run the smoke profile after the new module is present.**

`
python scripts/run_python_tests.py --profile smoke
`

Expected: the smoke profile includes test_context_application_v2_host_binding
and reports zero failures. The test profile remains an intentional closed
allowlist; adding this module is an explicit Slice-6 maintainer decision.

## 8. Task 4 — Implement verified applicability and member-key closure

**Files:**
- Modify: scripts/context_application_v2_host_binding.py
- Modify: python/tests/test_context_application_v2_host_binding.py

- [ ] **Step 1: Derive the exact member key.**

For each admitted member, call the existing source resolver and construct:

`
ApplicationMemberKeyV1(
    candidate_id=member.candidate_id,
    candidate_identity_digest=(
        member.candidate_identity_digest_reference.digest_bytes
    ),
    source_instance_id=member.source_instance_id,
)
`

The complete six-field DigestReferenceV1 must already have passed existing
candidate identity validation. Compare member keys by canonical CBOR bytes.

- [ ] **Step 2: Derive required members from verified candidate records.**

Use only:

`
candidate_record.get("scope") == "cross_deck" and candidate_record.get("relation") == "directional_binary"
`

For a cpa link, group all admitted cpar records by exact cpa ID. Because cpa
identity includes the complete member preimage, every record in one group has
the same semantic member set. Select the current cpar for current qualification;
select a deterministic canonical admitted record for a historical-only link.
The applicability subset must be identical for both selections.

- [ ] **Step 3: Add mixed-member RED/GREEN tests.**

Create one cpa with:

`
member A: cross_deck + directional_binary
member B: non-cross-deck candidate
`

Assert:

`
current link {A}                  -> PASS
historical-only link {A}         -> PASS, historical_only
current link {A, B}              -> HOST_MEMBER_SET_MISMATCH
historical-only link {A, B}      -> HOST_MEMBER_SET_MISMATCH
historical-only link {A} + H1 old -> PASS, old H1 status retained
`

The historical cases must not require a current hbc claim, but they must
require an admitted exact hbc claim and exact member-key/source provenance.

- [ ] **Step 4: Validate host relationship per required member.**

Compare:

`
claim.observed_host_relationship == member.context_binding_v1[3]
`

Reject not_applicable for a required member and reject any mismatch with
HOST_RELATIONSHIP_MISMATCH. Do not derive the expectation from historical
SourceContext, capability names, or claim realization order.

- [ ] **Step 5: Run the member closure matrix.**

`
$env:PYTHONPATH = "python/src;python/tests"
python -m unittest test_context_application_v2_host_binding -v
Remove-Item Env:PYTHONPATH
`

Expected: current and historical mixed-member tests pass with identical
required subsets and distinct authority qualification statuses.

## 9. Task 5 — Compose current/historical HostBinding claim status

**Files:**
- Modify: scripts/context_application_v2_host_binding.py
- Modify: python/tests/test_context_application_v2_host_binding.py

- [ ] **Step 1: Resolve HostBinding authority requirements.**

Apply the closed G3 policy:

`
any current or historical link exists
    -> host_binding_authority_v2_binding required

host authority present with no link and no current required member
    -> HOST_AUTHORITY_BINDING_UNEXPECTED

current cpa required subset non-empty without link
    -> application host-binding closure failure
`

Compare base-authority and candidate-universe bindings as complete typed tuples.
Use AuthorityV2Validator for HostBinding root admission; do not parse a
second HostBinding root in Slice 6.

- [ ] **Step 2: Validate current targets against current claim indexes.**

For a current cpa link:

`
claim ID must be in current_claims_by_id
claim member key must match exactly
claim member union must equal required subset
claim observed relationship must match member binding
`

Unknown, superseded, revoked, or ambiguous claims fail with the existing
structured Slice-6 categories.

- [ ] **Step 3: Validate historical-only targets against admitted records.**

For a historical-only cpa link:

`
claim ID must be in admitted_claims_by_id
claim record provenance must exist in claim_record_ids_by_claim_id
record status must be retained per hbcr.v1
claim member key and required subset must match exactly
claim observed relationship must match member binding
`

The status may be current, superseded, or revoked, but the link never adds its
cpa to qualified_current_application_record_ids.

- [ ] **Step 4: Add record-level and historical status tests.**

Cover at least:

`
H1 current claim referenced by current cpa              -> QUALIFIED_CURRENT
H1 superseded record referenced by historical cpa       -> HISTORICAL_ONLY
H1 revoked record referenced by historical cpa          -> HISTORICAL_ONLY
unknown claim referenced by historical cpa              -> HOST_CLAIM_UNKNOWN
current cpa referencing H1 after H1 becomes stale       -> HOST_CLAIM_NOT_CURRENT
two hbcr records for one HBC claim                      -> exact record map, no HBC status aggregation
`

- [ ] **Step 5: Materialize the frozen result.**

Return canonical tuples only:

`
ContextApplicationV2HostBindingEvaluationResult(
    currentness=slice5_result,
    qualified_current_application_record_ids=qualified_ids,
    application_host_binding_results=application_results,
    current_host_claim_ids=current_claim_ids,
)
`

Historical claim-record statuses remain available through the internal frozen
HostBinding read model and the historical application result path; no mutable
map or persisted status bit is added.

- [ ] **Step 6: Run current/historical composition tests.**

`
$env:PYTHONPATH = "python/src;python/tests"
python -m unittest test_context_application_v2_host_binding -v
Remove-Item Env:PYTHONPATH
`

Expected: all G3 current/historical tests pass, with no production artifact
creation.

## 10. Task 6 — Determinism, mutation safety, and full negative matrix

**Files:**
- Modify: python/tests/test_context_application_v2_host_binding.py
- Modify: python/tests/test_context_application_v2_resolver.py
- Modify: python/tests/test_authority_v2_validator.py

- [ ] **Step 1: Separate canonical-construction permutations from invalid input.**

For valid typed inputs, construct equivalent containers from different upstream
insertion orders and canonicalize them before constructing the frozen DTO.
Permute independently:

`
context_application_v2_records
context_application_v2_supersession_records
application_host_bindings_v2
HostBinding claim records
HostBinding supersession records
claim IDs within links when rebuilding the canonical ApplicationHostBindingV2
value
`

Assert byte/value-identical results. Sorting must use complete canonical CBOR
identity bytes.

Add a separate negative matrix for deliberately noncanonical wire values and
test-only forged typed objects. The frozen DTO constructors/parser must reject
noncanonical arrays; a forged object reaching the evaluator must fail with
HOST_INTEGRATION_INPUT_INVALID. Never sort a noncanonical input inside Slice 6
and accept it as if it had been canonical at the contract boundary.

- [ ] **Step 2: Add failure-atomicity snapshots.**

For every rejection class, snapshot before evaluation:

`
container.to_wire()
tuple(record.to_cbor() for record in container.context_application_v2_records)
tuple(record.to_cbor() for record in container.context_application_v2_supersession_records)
temporary_repository_file_digests()
`

Run the same rejected evaluation twice and assert all snapshots and structured
error fingerprints are unchanged. Assert no production path is written.

- [ ] **Step 3: Add the remaining negative matrix.**

Cover N1–N28 from the accepted Slice-6 design, with explicit cases for:

`
wrong application kind or malformed cpa
duplicate link/claim/member IDs
unknown application or claim
wrong candidate ID/digest/source instance
wrong host relationship
missing/unexpected authority binding
wrong shared snapshot
current stale claim
historical unknown claim
historical superseded/revoked claim retention
cross-application automatic transfer
HostBinding source in ae.v3 closure
Slice-5 graph failure before HostBinding I/O
`

- [ ] **Step 4: Run the focused full Slice-6 suite.**

`
$env:PYTHONPATH = "python/src;python/tests"
python -m unittest test_context_application_v2_host_binding test_context_application_v2_resolver test_context_application_v2_review_admission test_context_application_v2_supersession test_authority_v2_validator -v
Remove-Item Env:PYTHONPATH
`

Expected: zero failures and zero errors. This command is implementation
evidence only after the implementation tasks are authorized and executed.

## 11. Task 7 — Integration gates and stop boundary

**Files:** none beyond the files listed above.

- [ ] **Step 1: Run the repository Python smoke profile.**

`
python scripts/run_python_tests.py --profile smoke
`

Expected: OK; report the actual test count and failures.

- [ ] **Step 2: Run the mandatory fast and integration profiles.**

Run both repository profiles at the final implementation head:

`
just check-fast
just check
`

These profiles are mandatory evidence. The integration profile must include
the repository's Ruff format check, Ruff lint, Mypy, Cargo fmt, Cargo check,
Cargo clippy, and Cargo test sub-gates. If the just wrapper is unavailable,
run the exact equivalent profiles:

`
python scripts/run_checks.py fast
python scripts/run_checks.py integration
`

Record each sub-gate as PASS, FAIL, NOT_RUN, or BLOCKED; do not promote a
partial direct command to a just-profile PASS.

- [ ] **Step 3: Run documentation and maintainer checks.**

`
python scripts/check_documentation.py
python scripts/validate_maintainer_artifacts.py
git diff --check
`

Expected: all commands exit 0. Report each status separately.

- [ ] **Step 4: Run the applicable broader repository profile.**

Because Slice 6 changes Python authority tooling and closure semantics, run:

`
python scripts/run_python_tests.py
python scripts/validate_schemas.py
python scripts/verify_repository.py
`

The just integration profile already owns the required Rust, Ruff, and Mypy
checks. Run the certification profile only if the release/certification scope
is separately authorized; Slice 6 does not claim certification by default.

- [ ] **Step 5: Inspect final scope and forbidden surfaces.**

`
$planHead = $env:MANAFOLD_SLICE6_PLAN_HEAD
if ([string]::IsNullOrWhiteSpace($planHead)) { throw "MANAFOLD_SLICE6_PLAN_HEAD is required" }
if ((git cat-file -t "$planHead^{commit}").Trim() -ne "commit") { throw "PLAN_HEAD is not a commit" }
$contractHead = "20fb8d92cdac27fc0dc61060223ffd7d7fa00cb5"
if ((git rev-parse "$planHead^{commit}").Trim() -eq $contractHead) { throw "PLAN_HEAD must include the reviewed plan commit" }
git diff --name-only "$planHead..HEAD"
git diff --name-only "d647e63d7687bd2c022393feaee5bac6736e84ca..HEAD"
git ls-files --others --exclude-standard
git status --short --branch
`

MANAFOLD_SLICE6_PLAN_HEAD is the exact final plan commit recorded by the
independent plan review before implementation starts. The first range is the
implementation-only scope. The second range includes the accepted ADR,
reviewed plan, and implementation. The implementation diff may contain only
the planned Python modules/tests and resolver compatibility tests. It must not
contain:

`
schemas/
python/src/mtgml/authority.py
python/src/mtgml/host_binding.py
crates/
production authority/source artifacts
C artifacts
player APIs or trajectory schemas
`

- [ ] **Step 6: Stop for independent implementation review.**

Do not create production authority artifacts, run Buckle-Up, modify C, start
Task 5 Slice 3B, begin M3, or claim Slice-6 implementation acceptance until a
separate exact-head review approves the implementation.

## 12. Acceptance checklist for the future implementation

Before any future implementation claim, verify all of the following with fresh
evidence:

`
G1 ae.v3 event closure is host-free                       = PASS
G1 container closure contains separate HostBinding provenance = PASS
G2 current required set uses verified candidate predicate  = PASS
G2 historical required set uses the same subset            = PASS
G3 current links require current HBC authority             = PASS
G3 historical links retain admitted exact HBC identity     = PASS
G3 status is record-level hbcr.v1                          = PASS
Slice-5 currentness remains sole cpar/cps/cpsr owner       = PASS
unknown targets/claims fail closed                         = PASS
permutation fingerprints are identical                     = PASS
rejection is mutation-free                                 = PASS
schemas/identities/preimages/Rust unchanged                = PASS
production authority artifacts created                     = NO
Buckle-Up canary executed                                  = NO
`

This plan ends at the independent implementation-review checkpoint. It does
not authorize execution merely because the plan itself is committed.
