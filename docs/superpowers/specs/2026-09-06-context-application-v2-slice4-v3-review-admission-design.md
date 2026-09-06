# ContextApplicationV2 Slice 4 — V3 Review Admission Design

**Status:** design-only proposal; implementation not started

**Date:** 2026-09-06

**Reviewed base:** `0dfd646fc6b8b7e09fef69a9721eba8487425a46` (`origin/master`)

**Branch:** `chris/context-application-v2-slice4-v3-review-admission`

## 1. Slice-4 ownership boundary

Slice 4 admits one `ContextApplicationV2Record` when its exact V3 acceptance
event mechanically binds the record, the source closure, the reviewer roster,
the required roles, the V2 checklist marker, the review mode, and every review
evidence locator. The result means only:

```text
the persisted V3 envelope is structurally and mechanically bound to this
typed ContextApplicationV2Record
```

It does not mean that the human review was substantively correct or that the
record is current. The validator is read-only. It must not create, rewrite,
or promote an application record, an acceptance event, a source artifact, or
a current-record index.

The validator composes the existing Slice-3 semantic validator. It accepts a
record only after Slice-3 validation succeeds, then validates the embedded
`ReviewEventRefV3` and its V3 admission contract. One invalid member, event,
source, reviewer, or evidence reference rejects the whole record.

## 2. Checklist V2 and lifecycle

The new immutable definition is:

```text
docs/maintenance/INTERACTION_AUTHORITY_REVIEW_CHECKLIST_V2.md
Checklist ID: interaction-authority-review-checklist.v2
```

Checklist V1 remains unchanged and retains its historical meaning. Checklist
V2 inherits every V1 obligation by reference and adds explicit review of:

- the exact historical `source_value` for all ten context slots;
- the exact `reviewed_value` for all ten context slots and all four temporal
  slots;
- the mechanically derived `exact_match` and `reviewed_divergence` relation;
- positive evidence for every reviewed context and temporal value, including
  `exact_match` and `not_applicable`;
- the exact historical source binding and actual inequality for every
  `reviewed_divergence`;
- no SourceContext rewrite, normalization, lexical inference,
  capability-name inference, co-occurrence inference, or absence-of-evidence
  inference;
- exact Candidate, SourceInstance, and V1 precondition preservation;
- exact V3 source closure, reviewer roster, role coverage, and immutable
  accepted provenance; and
- the future supersession and revocation obligations.

The checklist records the supersession/revocation obligation, but Slice 4 does
not validate a supersession graph or select a current record.

The proposed normative-document registration for the checklist is:

| field | value |
|---|---|
| `path` | `docs/maintenance/INTERACTION_AUTHORITY_REVIEW_CHECKLIST_V2.md` |
| `role` | `process` |
| `owner_role` | `project-governance` |
| `stability` | `accepted` |
| `change_process` | `governance-pr` |

This design commit records the proposal only. It does not create the checklist
file or change the document register.

## 3. Mandatory reviewer-role policy

Every V2 application admission requires the union of these roles:

```text
architecture_maintainer
rules_authority_maintainer
conformance_maintainer
information_safety_reviewer
```

The first two roles are required by ADR 0042. `conformance_maintainer` is
always required because every Slice-4 admission is a final cross-artifact
acceptance of public V3 identity, schema, source-closure, and negative-contract
behavior. The earlier review-authority design assigns this role to final
evidence and negative-contract acceptance; every V2 admission covers that
surface.

`information_safety_reviewer` is always required under the conservative policy
in Section 5. A single reviewer may hold several roles. Role coverage is
computed from the selected reviewer IDs, not from an event-local appointment
field.

The existing exact-roster-role rule remains authoritative: each event binding
must name a reviewer from the bound roster and must reproduce that reviewer's
complete canonical role tuple. An event may select a subset of roster members,
but that subset must cover every mandatory role.

## 4. `conformance_maintainer` rule

`conformance_maintainer` is unconditional for `context_application_v2_record`
admission. It is not inferred from the presence of a particular evidence file,
review mode, or natural-language rationale. This removes an otherwise
ambiguous condition from the V3 admission contract and matches the accepted
review-authority policy for final evidence and negative-contract acceptance.

Missing coverage returns `REVIEWER_ROLE_MISSING` with the missing role named in
the diagnostic. The event cannot waive the role in solo mode.

## 5. Information-safety policy and typed inventory

The selected policy is conservative mandatory review. Every V2 application
requires `information_safety_reviewer` because the subject always carries
reviewed context and member evidence, while V3 acceptance evidence has no
machine-readable sensitivity classification. Requiring the role for every V2
record avoids a false negative without inspecting prose.

The implementation must still inventory these typed facts because they define
the exact information-sensitive surface and preserve a future narrowing seam:

- the `visibility` and `information_relation` positions in the ten historical
  source values;
- the same two positions in every reviewed bridge value;
- the same two positions in every theorem context vector;
- `source_context` preconditions for those dimensions; and
- `class_projection` preconditions whose exposed context vector contains a
  non-`not_applicable` value for either dimension.

These facts are suitable for deterministic diagnostics and future policy
versioning. They do not relax the unconditional V2 role requirement. The
validator must not inspect card names, capability names, Oracle prose,
evidence prose, file names, or rationale text to infer sensitivity. Opaque
evidence references resolve for integrity and locator correctness only.

V1 keeps its current conditional information-safety behavior. The V2 policy
must not broaden or weaken V1 acceptance.

## 6. `project_owner` rule

`project_owner` is not mandatory for V2 admission. It is valid only when the
bound roster grants it. If a selected reviewer has that role, the event must
include it because the event binding must equal the reviewer's complete roster
role tuple. The event cannot add, remove, or substitute the role locally.

The current production roster is content-addressed and contains one reviewer
with all five closed roles. Slice-4 tests may use temporary synthetic rosters;
they must preserve the same closed vocabulary and exact role-binding semantics.

## 7. `multi_reviewer` mechanical meaning

`multi_reviewer` is a closed declaration of the persisted review mode. Slice 4
validates that the value belongs to the V3 enum, that the event has non-empty
canonical reviewer bindings, that the roster bindings are exact, and that all
mandatory roles are covered.

The contract does not contain a reviewer-count rule. Slice 4 therefore must
not infer `reviewer_count >= 2` from the mode name.

## 8. `solo_separate_self_review` mechanical meaning

`solo_separate_self_review` is also a closed declaration. It preserves every
V1 separate-review obligation:

- a separate pass after proposal or artifact generation;
- review of frozen exact bytes, identities, and source bindings;
- no semantic edits during the acceptance pass;
- a fresh pass after any required edit;
- a complete checklist rerun; and
- portable review evidence.

The persisted V3 fields contain no timestamp, authoring-pass identity, or
pass-link identity. Slice 4 can therefore validate only the closed mode,
exact roster bindings, complete mandatory roles, and non-empty resolvable
evidence. It must not claim to prove temporal separation, reviewer
independence, or substantive evidence sufficiency. Solo mode never waives a
role or evidence requirement.

## 9. What mechanical admission cannot prove

Admission proves raw-byte integrity, schema and identity consistency,
subject/event/source binding, roster authorization, role coverage, review-mode
closure, and locator resolvability. It does not prove:

- that a human actually performed the declared review process;
- that a separate solo pass occurred at a different time;
- that the reviewers were independent people;
- that review prose was substantively correct or sufficient; or
- that the accepted record is current after supersession or revocation.

Those claims remain outside the persisted V3 contract or belong to Slice 5 and
later real-human-review authorization.

## 10. Reviewer-roster binding semantics

The implementation must reuse the existing V1 roster semantics through one
small shared mechanical seam. The seam may extract the current
`AuthorityValidator._parse_roster(...)` and
`AuthorityValidator._event_role_bindings(...)` behavior into a narrow helper,
but the V1 validator must delegate without a semantic change.

Roster admission requires:

- exact repository-relative content-addressed path;
- exact `manafold.m2.5.c.reviewer-roster.v1` schema;
- exact raw SHA-256 bytes and filename binding;
- closed reviewer IDs and role vocabulary;
- canonical reviewer ordering;
- canonical, duplicate-free role ordering;
- duplicate-free reviewer IDs; and
- exact equality between each event binding's role tuple and the bound
  reviewer's complete role tuple.

V3's structural DTO currently canonicalizes complete role-binding tuples. The
semantic helper must additionally reject the same `reviewer_id` more than
once, even when the duplicate entries carry different role subsets.

## 11. Acceptance-subject digest binding

For a `ContextApplicationV2Record`, recompute the exact subject payload:

```text
AcceptanceSubjectPayloadV3(
    subject_kind = context_application_v2_record,
    subject_payload = record.acceptance_free_subject_payload(),
)
```

Derive the expected reference with:

```text
DigestReferenceV1.from_identity(subject.identity())
```

The comparison with `event.subject_payload_digest_reference` is full structural
equality across:

```text
envelope_id
algorithm_id
semantic_domain
payload_codec_id
input_schema_id
digest_bytes
```

The validator must reject a matching digest with a wrong domain, schema,
envelope, codec, algorithm, subject kind, application ID, theorem ID, or
member set. It must not compare only the 32 digest bytes.

## 12. V3 event/reference ownership

The validator uses only `record.review_event_ref_v3`. It does not accept a
caller-selected event reference. It calls
`ContextApplicationV2Resolver.resolve_review_event_leaf_v3(...)`, which remains
the sole owner of:

- V3 path grammar and basename-to-event-ID binding;
- raw event digest and JSON shape;
- V3 schema and typed event reconstruction;
- `human_accepted` and V2 checklist markers;
- semantic event-ID recomputation;
- self-leaf rejection; and
- exact review-evidence locator resolution.

Slice 4 must not create a second event parser or raw-file resolver. Structural
resolver failures map into stable Slice-4 error categories without changing
the resolver's ownership. Slice 4 may make one narrowly additive diagnostic
change to `ContextApplicationV2ResolutionError` and the V3 event-resolution
path: the exception exposes a closed machine-readable `code` alongside its
existing human-readable message. Existing message text, exception type, and
resolver behavior remain compatible for current callers. The code field is
populated only for resolver-owned V3 event/evidence failures; legacy V2
source/closure failures retain their current observable error shape. Existing
Slice-3 callers retain their current fallback semantic categories when they
handle legacy resolver failures. The Slice-4 validator maps these codes, never
exception-message text. No checklist, review-mode, or evidence validation
moves out of the resolver.

## 13. Exact event source closure

After resolving the embedded event, the validator calls:

```text
ContextApplicationV2Resolver.expected_acceptance_source_closure_v3(
    record,
    event.reviewer_roster_ref,
)
```

It passes no host bindings. The event source list must equal the independently
reconstructed canonical closure exactly. The closure includes the base and
declared model bindings, exact roster binding, member/candidate/source-instance
provenance, theorem dependencies, B1/B2 dependencies, and all member evidence
needed by the existing resolver. It excludes the event leaf and the V2
container.

Missing, extra, stale, substituted, duplicate, wrong-role, wrong-schema, or
wrong-digest bindings fail closed. Any host-binding source fails exact closure
in Slice 4 because HostBinding integration belongs to Slice 6. Existing
event/container cycle rejection remains the resolver's responsibility.

## 14. Review-evidence resolution

V3 review evidence must be non-empty. Every reference must resolve through the
existing `ContextApplicationV2Resolver.resolve_acceptance_evidence(...)` path,
which verifies the repository-relative locator, raw digest, JSON pointer or
archive member where applicable, and exact artifact bytes.

The admission validator checks resolution and integrity only. It does not parse
review prose or add fields such as `approved`, `sufficient`, or `review_score`.
Resolution proves that evidence is bound and locatable; it does not prove that
the human review was substantively adequate.

## 15. Application-specific and reusable admission interfaces

The public Slice-4 module is proposed as:

```text
scripts/context_application_v2_review_admission.py
```

with a narrow interface:

```text
ContextApplicationV2ReviewAdmissionValidator.admit(
    record: ContextApplicationV2Record,
) -> ContextApplicationV2ReviewAdmissionResult
```

The result is a frozen mechanical-admission DTO with exactly these fields:

```text
record_id: AuthorityIdentityV1
application_id: AuthorityIdentityV1
subject_digest_reference: DigestReferenceV1
review_event_ref: ReviewEventRefV3
event_id: str
exact_event_closure: tuple[ContextAuthoritySourceBindingV2, ...]
reviewer_roster_ref: ReviewerRosterRefV1
required_roles: tuple[str, ...]
review_mode: ReviewMode
```

`reviewer_roster_ref` is the resolved content-addressed roster reference; it
is not a separate `AuthorityIdentityV1` family. The result does not expose the
resolver's `ResolvedReviewAcceptanceEventV3` or its `ResolvedArtifact`, JSON
projection, or raw artifact graph. The typed frozen
`ReviewAcceptanceEventInputV3` may remain an internal value; if a later caller
needs it, it may be added only as a frozen DTO, never as a resolved artifact
wrapper. The result contains no currentness or human-sufficiency claim.

Internally, the module uses one reusable V3 binding seam that accepts a typed
subject kind, acceptance-free subject payload, and the subject's embedded
event reference. That seam may later serve a supersession subject, but its
result means only “the V3 envelope binds this typed subject.” The only
Slice-4 admission entry point accepts an application record.

The validation order is fixed:

1. validate the input record type;
2. run `ContextApplicationV2SemanticValidator.validate(...)`;
3. resolve the embedded V3 event leaf;
4. recompute the exact `AcceptanceSubjectPayloadV3`;
5. compare the complete digest reference;
6. reconstruct and compare the exact event source closure with no host path;
7. resolve the exact reviewer roster;
8. validate reviewer existence, exact role tuples, and duplicate identities;
9. compute and enforce the required V2 role set using the fixed precedence in
   Section 17;
10. validate the closed review mode without inventing count or timing rules;
11. confirm non-empty evidence resolution; and
12. return the immutable mechanical-admission result.

No step mutates the record, its members, resolved artifacts, source files,
candidate universe, base authority, event files, review evidence, or `C`.

## 16. Supersession boundary

Slice 4 does not validate supersession edges, replacement existence, graph
cycles, revocation, or current-record selection. It must not expose a result
named or documented as “supersession accepted,” “current,” or “replacement
authoritative.”

The internal V3 binding seam may accept
`context_application_v2_supersession_record` only to validate its subject/event
binding in a future caller. That reusable result remains agnostic to graph
validity. Supersession validation belongs to Slice 5.

## 17. Stable error model and precedence

The new module exposes a typed fail-closed error carrying a stable category,
location, and optional wrapped source/semantic diagnostic. It preserves the
existing validators' detailed causes while giving callers deterministic
categories. The resolver's additive V3 diagnostic seam has this closed code
vocabulary:

```text
V3_EVENT_REFERENCE_INVALID
V3_EVENT_SCHEMA_INVALID
V3_EVENT_DECISION_INVALID
CHECKLIST_V2_MISMATCH
REVIEW_MODE_INVALID
REVIEW_EVIDENCE_MISSING
REVIEW_EVIDENCE_INVALID
V3_EVENT_IDENTITY_INVALID
V3_EVENT_SOURCE_INVALID
```

The resolver assigns these codes at the owning checks. It preserves the
existing message text and does not require callers to parse it. Delegated raw
source failures retain their existing exception type, message, and
machine-readable source-resolver code; the V3 boundary maps them through an
explicit code table, never by matching message text. The Slice-4 validator
maps resolver codes to its public admission categories through an explicit
table. A resolver-owned specific code is never collapsed into
`V3_EVENT_REFERENCE_INVALID` merely because it is convenient.

The public admission category set is:

```text
SEMANTIC_VALIDATION_FAILED
V3_EVENT_REFERENCE_INVALID
V3_EVENT_SCHEMA_INVALID
V3_EVENT_DECISION_INVALID
V3_EVENT_IDENTITY_INVALID
V3_EVENT_SOURCE_INVALID
V3_SUBJECT_KIND_MISMATCH
V3_SUBJECT_DIGEST_MISMATCH
V3_SOURCE_CLOSURE_MISMATCH
REVIEWER_ROSTER_INVALID
REVIEWER_BINDING_INVALID
REVIEWER_BINDING_NOT_IN_ROSTER
REVIEWER_DUPLICATE
REVIEWER_ROLE_MISSING
INFORMATION_SAFETY_REVIEWER_REQUIRED
REVIEW_MODE_INVALID
REVIEW_EVIDENCE_MISSING
REVIEW_EVIDENCE_INVALID
CHECKLIST_V2_MISMATCH
```

The mapping at the Slice-4 seam is explicit:

| failure raised by | admission category |
|---|---|
| resolver-owned V3 code | the identical named category |
| delegated `ResolutionError` while loading the event leaf | `V3_EVENT_SOURCE_INVALID` |
| delegated `ResolutionError` while resolving review evidence | `REVIEW_EVIDENCE_INVALID` |
| Slice-3 semantic validator | `SEMANTIC_VALIDATION_FAILED` |
| full subject digest-reference comparison | `V3_SUBJECT_DIGEST_MISMATCH` |
| exact closure comparison | `V3_SOURCE_CLOSURE_MISMATCH` |

The original delegated resolver code remains available as structured cause
metadata. No mapping branch examines exception text.

The stable category is part of deterministic admission behavior. A rejection
must never collapse into a bare `False` or a message-only exception.

After roster parsing and exact role-binding validation succeed, required-role
selection uses this ordered tuple, never a set iteration:

```text
(
    architecture_maintainer,
    rules_authority_maintainer,
    conformance_maintainer,
    information_safety_reviewer,
)
```

The first missing role determines the result. Missing
`information_safety_reviewer` returns `INFORMATION_SAFETY_REVIEWER_REQUIRED`.
Missing any of the other three roles returns `REVIEWER_ROLE_MISSING`. Thus a
record missing several roles has one stable, process-independent diagnostic.
Earlier validation failures, such as an invalid roster or duplicate reviewer
ID, take precedence over role coverage because role coverage is not evaluated
until the roster binding is valid.

## 18. Mutation safety and determinism

The implementation takes immutable DTOs as input and returns immutable result
or error values. Temporary repositories may be read during tests, but the
admission path has no write operation. Rejected admission must leave unchanged:

```text
record, members, event reference, resolved event, roster, source files,
candidate universe, base authority, event files, review evidence, and C
```

Tests will snapshot DTO projections and raw file digests before and after both
successful and rejected calls. A rejected call must not materialize an
accepted record.

Identical subject bytes, event bytes, roster bytes, closure, and evidence must
produce identical status, error category, required role set, and result bytes.
The implementation must not depend on a clock, username, filesystem
enumeration order, map iteration, network access, absolute checkout paths, or
randomness. Canonical CBOR ordering and existing canonical source-set helpers
remain the ordering authorities.

## 19. Test matrix

The eventual implementation adds temporary-repository synthetic tests for at
least these positive controls:

| control | expected result |
|---|---|
| exact-match application with complete closure and roles | `PASS` |
| one valid reviewed-divergence member | `PASS` |
| information-sensitive typed facts with the approved role policy | `PASS` |
| `solo_separate_self_review` with all mechanically representable obligations | `PASS`, without claiming temporal proof |

The negative matrix must cover:

- wrong event reference, raw digest, path basename, event ID, schema,
  decision, or checklist;
- wrong subject kind, complete digest-reference field, digest bytes,
  application ID, member set, or semantic ID;
- missing, extra, stale, duplicate, substituted, wrong-role, wrong-schema,
  wrong-digest, self-leaf, container, or unauthorized host source bindings;
- unknown, tampered, or digest-mismatched rosters;
- roster-missing reviewers, exact-role mismatch, event-local role escalation,
  duplicate reviewer IDs, and missing mandatory roles;
- missing information-safety coverage under the approved unconditional policy;
- empty, stale, malformed, or unresolved review evidence;
- review modes outside the closed enum;
- solo-mode attempts to waive a role or evidence requirement; and
- any mutation of the record, resolver inputs, or source files during
  rejection.

Focused tests must also prove V1 behavior is unchanged, including the V1
roster parser, exact role-binding semantics, V1 event fixtures, and existing
conditional information-safety checks. Existing Slice-2 closure and Slice-3
semantic golden matrices remain unchanged and must still pass. Rust V3
identity/contract regressions run, but no Rust admission-policy implementation
is planned. `ReviewAcceptanceEventInputV1`, the V1 event schema and identity,
and existing accepted V1 records remain byte-for-byte semantically unchanged.

## 20. Expected files and cross-layer impact

The design-only commit contains only this file. After separate approval of an
implementation plan, the expected implementation change set is:

```text
docs/maintenance/INTERACTION_AUTHORITY_REVIEW_CHECKLIST_V2.md
docs/normative-document-register.v1.json
scripts/context_application_v2_review_admission.py
scripts/context_application_v2_resolver.py
```

If extraction is required to preserve V1 reuse without coupling the new module
to private validator internals, the plan may add one narrow shared reviewer
helper and a delegation-only edit to `scripts/authority_validator.py`.
The likely tests are:

```text
python/tests/test_context_application_v2_review_admission.py
python/tests/test_context_application_v2_resolver.py
python/tests/test_authority_validator.py
python/tests/test_review_admission_foundation.py
```

The resolver test additions must prove the new diagnostic codes are stable,
that existing exception messages remain unchanged, and that no caller parses
message text.

The implementation must not change:

```text
asp.v3, ae.v3, cpa.v2, cpar.v2,
ReviewAcceptanceEventInputV3, ReviewAcceptanceEventLeafV3,
ReviewEventRefV3, AcceptanceSubjectPayloadV3,
schemas/review-acceptance-event.v3.schema.json,
schemas/context-application-authority.v2.schema.json,
or the Rust admission-policy surface.
```

No production event, production ContextApplicationV2 record, or production
authority record is created. Synthetic fixtures remain temporary test inputs.

## 21. Explicit non-goals and acceptance boundary

This Slice-4 design does not authorize:

- production human acceptance or real Buckle-Up review;
- production V3 event or ContextApplicationV2 creation;
- supersession graph validation or currentness;
- HostBindingV2 semantic integration;
- ApplicationHostBindingV2 validation;
- real `C` derivation or ClassProjection;
- Task 5 Slice 3B;
- card support, Magic rules, M3, ranking, or datasets; or
- a schema, identity, canonical-CBOR preimage, or Rust/Python contract change.

The final implementation may claim only mechanical V3 review admission after
the implementation, independent review, and required gates pass. It may not
claim that ContextApplicationV2 as a whole is implemented or frozen.

## Design-stage status

```text
BASE=0dfd646fc6b8b7e09fef69a9721eba8487425a46
BRANCH=chris/context-application-v2-slice4-v3-review-admission

ROLE_POLICY_PROPOSAL=architecture + rules + conformance + information-safety;
                     project-owner optional and roster-bound
INFORMATION_SAFETY_POLICY=conservative mandatory for every V2 application
CONFORMANCE_ROLE_POLICY=mandatory for every V2 application admission
MULTI_REVIEWER_POLICY=closed enum only; no inferred reviewer count
SOLO_REVIEW_POLICY=closed enum plus checklist/evidence obligations; no timing proof

SCHEMA_CHANGE_REQUIRED=NO
IDENTITY_CHANGE_REQUIRED=NO
CONTRACT_GAP_FOUND=NO

PRODUCTION_CODE_CHANGED=NO
PRODUCTION_EVENT_CREATED=NO
PRODUCTION_AUTHORITY_RECORD_CREATED=NO
PRODUCTION_CHECKLIST_V2_CREATED=NO

SLICE_4_IMPLEMENTATION_STARTED=NO
SLICE_5_AUTHORIZED=NO
SLICE_6_AUTHORIZED=NO
TASK_5_SLICE_3B=BLOCKED
M3=BLOCKED
```
