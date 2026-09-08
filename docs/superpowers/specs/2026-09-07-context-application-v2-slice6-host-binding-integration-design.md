# ContextApplicationV2 Slice 6 — ApplicationHostBindingV2 Semantic Integration

**Status:** design-only; provisional; implementation blocked by the contract gaps in §27

**Design baseline:** `d647e63d7687bd2c022393feaee5bac6736e84ca`

**Remote baseline:** `origin/master = d647e63d7687bd2c022393feaee5bac6736e84ca`

**Design branch:** `chris/context-application-v2-slice6-host-binding-design`

**Implementation authorization:** `NO`

**Production authority/artifact creation:** `NOT AUTHORIZED`

**Buckle-Up review/canary:** `NOT AUTHORIZED`

This document is the implementation-ready design preparation for the final
ContextApplicationV2 infrastructure slice. It changes no production code,
schema, identity, canonical-CBOR preimage, Rust DTO, authority artifact, C
artifact, card, rule, or player-facing surface.

## 1. Executive conclusion

Slice 6 should be one deep composition module with one public interface:

```text
ContextApplicationV2HostBindingEvaluator.evaluate(
    container: ContextApplicationAuthorityV2,
    source_resolver: AuthoritySourceResolver,
) -> ContextApplicationV2HostBindingEvaluationResult
```

The caller supplies one immutable typed container and an injected source
resolver. The evaluator performs all admission itself. It does not accept
`accepted`, `current`, `trusted`, `already_validated`, or precomputed current
HostBinding claims. It returns only frozen derived identities, statuses, and
claim/member projections; it never returns raw JSON, source artifacts, file
handles, resolver internals, or mutable indexes.

The composition order is:

```text
typed container/projection preflight
    -> Slice-5 ContextApplicationV2 currentness
    -> verified current-member host applicability
    -> existing HostBinding V2 authority/current-claim admission seam
    -> exact ApplicationHostBindingV2 link closure
    -> host semantic comparison
    -> separate container/source-closure verification
    -> frozen qualified read model
```

The two lifecycle systems remain independent:

```text
ContextApplication currentness: cpar.v2 / cps.v2 / cpsr.v2
HostBinding currentness:        hbc.v1 / hbcr.v1 / hbcs.v1
```

Slice 5 remains the sole owner of ContextApplication currentness. The existing
HostBinding authority machinery remains the sole owner of hbc/hbcr/hbcs
admission and current-claim derivation. Slice 6 only composes their typed
read models.

The design is intentionally blocked before implementation. Repository
inspection found three connected normative gaps:

1. ADR 0042 §10.1 permits optional HostBinding sources in an
   `ExpectedAcceptanceSourceClosureV3`, while the accepted Slice-4 design and
   its public admission path require V3 review-event closure to remain
   host-free.
2. The provisional HostBinding checklist says every current V1 application
   requires a link, while the executable HostBinding validator and tests
   require links only for verified `cross_deck` + `directional_binary`
   members. ADR 0042 does not define the V2 required-member rule.
3. The accepted contracts do not define the policy for unused HostBinding
   authority or links targeting known cpa groups without a current cpar record,
   including links whose hbc claim later became superseded or revoked.

The recommended resolution is an ADR candidate that preserves the existing
host-free V3 admission surface, defines HostBinding applicability from the
verified candidate record, and makes container-level HostBinding provenance a
separate closure layer. No implementation plan is authorized until that
clarification is independently accepted.

## 2. Verified repository baseline

The baseline gate was executed in an isolated linked worktree:

```text
git fetch origin master                         PASS
git rev-parse HEAD                              d647e63d7687bd2c022393feaee5bac6736e84ca
git rev-parse origin/master                     d647e63d7687bd2c022393feaee5bac6736e84ca
git status --short --branch                     clean on design branch
```

The original checkout remains on local `master`; its unrelated untracked
file, `docs/superpowers/plans/2026-08-26-m2-5-b2-terminal-card-classification-closure.md`,
was not modified or copied into the design worktree.

The current merged baseline contains:

| Surface | Verified owner | Slice-6 implication |
| --- | --- | --- |
| `cpa.v2`, `cpar.v2`, `cps.v2`, `cpsr.v2`, `asp.v3`, `ae.v3` | `python/src/mtgml/authority.py`, Rust persistence DTOs, ADR 0042/0043 | frozen; no identity or preimage change |
| bridge/member semantic validation | `scripts/context_application_v2_validator.py` | reuse; no HostBinding inference |
| V3 review admission | `scripts/context_application_v2_review_binding.py`, `scripts/context_application_v2_review_admission.py` | reuse; remains host-free at its public seam |
| source and closure algebra | `scripts/context_application_v2_resolver.py` | reuse exact source verification; resolve the event/container split before implementation |
| ContextApplication currentness | `scripts/context_application_v2_supersession.py` | sole currentness owner; call it before HostBinding evaluation |
| HostBinding DTOs and identities | `python/src/mtgml/host_binding.py` | reuse `hbc.v1`, `hbcr.v1`, `hbcs.v1`, and `ApplicationMemberKeyV1` |
| HostBinding source joins | `scripts/authority_host_binding.py` | reuse `resolve_claim_for_member` and evidence resolution |
| HostBinding authority graph | `scripts/authority_v2_validator.py` | extract a frozen current-claim read model without duplicating lifecycle rules |
| V2 context container | `ContextApplicationAuthorityV2` in Python/Rust plus `context-application-authority.v2.schema.json` | typed input already exists; no field addition |

The repository smoke profile was executed from the repository runner:

```text
python scripts/run_python_tests.py --profile smoke             PASS
144 tests, 0 failures
```

The direct `python -m unittest ...` invocation without the repository runner
was not used as acceptance evidence because the src-layout import path was
not configured; the repository runner is the valid evidence for this result.

## 3. Existing semantic ownership map

### 3.1 ContextApplication ownership

`python/src/mtgml/authority.py` and
`crates/mtgml-persistence/src/authority.rs` own the frozen structural DTOs,
closed identity kinds, complete `DigestReferenceV1`, and canonical-CBOR
preimages. `ApplicationHostBindingV2` is already additive, cpa-keyed, and
member-claim-ID based. It must not gain a record ID or HostBinding metadata in
its identity.

`ContextApplicationV2SemanticValidator` owns Slice-3 theorem/source/member
equivalence. Its `context_binding_v1` is required to match the theorem subject
shape and the resolved source-instance participant shape. Its reviewed bridge
values do not rewrite historical `SourceContextV1` values.

`ContextApplicationV2ReviewAdmissionValidator` owns Slice-4 application
admission. It composes the semantic validator with the shared V3 binding seam;
the public V3 event closure is reconstructed without caller-selected
HostBinding sources.

`ContextApplicationV2CurrentnessEvaluator` owns all Slice-5 graph admission and
currentness. It groups by complete `cpa.v2` application ID, permits a
record-ID replacement across different cpa IDs, applies application-wide
revocation to the exact cpa group, and rejects ambiguous current candidates.

### 3.2 HostBinding ownership

`python/src/mtgml/host_binding.py` owns:

- member-atomic `hbc.v1` claim identity;
- accepted claim-record `hbcr.v1` identity;
- claim supersession `hbcs.v1` identity;
- exact `ApplicationMemberKeyV1` shape;
- correlated discovery/realization witness shape; and
- the existing V1 application-link contract.

`scripts/authority_host_binding.py` is the only source-bound host realization
join owner. It obtains bytes through `AuthoritySourceResolver`, checks exact
Candidate/SourceInstance identity, and verifies discovery, deck-row, OSI, and
B2 correlations. It does not infer interaction meaning.

`scripts/authority_v2_validator.py` owns the existing HostBinding V2 envelope,
claim-record/supersession admission, current-record rejection, source closure,
and V1 application-link closure. Slice 6 must reuse this lifecycle machinery
through a narrow frozen read-model seam; it must not create a second hbc
supersession or currentness algorithm.

### 3.3 Container/closure ownership

`ContextApplicationV2Resolver` owns repository-relative source-role validation,
raw-byte digest verification, V3 event parsing, event-closure reconstruction,
and container-closure reconstruction. The public
`expected_acceptance_source_closure_v3` path deliberately does not accept a
caller-selected HostBinding collection. This is the correct Slice-4 trust
boundary and must remain so.

The existing private container helper can discover HostBinding source bindings
from cpa-level links. Its use for application event closures is the subject of
the contract gap in §27; implementation must not silently choose between the
two incompatible interpretations.

## 4. Slice-6 scope and non-scope

### In scope

- one typed, read-only Slice-6 composition interface;
- exact composition of Slice-5 current cpa records with cpa-level links;
- reuse of existing HostBinding claim admission/currentness/source joins;
- exact member-key mapping and required-member closure;
- exact reviewed host-expectation comparison;
- explicit current, historical-only, no-required-members, revoked, and
  unknown-application outcomes;
- deterministic ordering, stable structured diagnostics, and failure
  atomicity;
- a frozen derived read model;
- adversarial, permutation, and mutation-safety test design;
- an ADR-candidate recommendation for the three identified contract gaps.

### Out of scope

- production Python/Rust implementation;
- production `ae.v3`, `cpar.v2`, `cps.v2`, `cpsr.v2`, `hbc.v1`, `hbcr.v1`, or
  HostBinding authority artifacts;
- real Buckle-Up human review or candidate classification;
- edits to `C`;
- `ClassProjection`;
- Task 5 Slice 3B;
- M3, Magic rules, cards, or ML behavior;
- any player-facing API, observation, trajectory, replay, checkpoint, or RNG
  change;
- changes to accepted identities, schemas, canonical-CBOR preimages, or Rust
  DTOs.

## 5. Trust boundary and selected interface

### 5.1 Alternatives

| Option | Interface | Assessment |
| --- | --- | --- |
| A | Separate typed application records, supersession records, links, and a pre-admitted HostBinding snapshot | Too easy to mix source projections and bypass one admission stage; duplicates container consistency checks at callers. |
| B | One typed `ContextApplicationAuthorityV2` container plus an injected `AuthoritySourceResolver` | Recommended. The container is already the repository-native composition value; the evaluator owns all admission and closure checks behind one deep interface. |
| C | Raw container JSON plus raw HostBinding authority JSON | Rejected. It would expose two parsers and permit caller-selected or stale authority data. |

### 5.2 Recommended public seam

The future module should expose only the following public operation and frozen
result/error types:

```text
ContextApplicationV2HostBindingEvaluator.evaluate(
    container: ContextApplicationAuthorityV2,
    source_resolver: AuthoritySourceResolver,
) -> ContextApplicationV2HostBindingEvaluationResult
```

The evaluator must:

1. require the exact typed container;
2. validate canonical collection order and duplicate IDs before source walks;
3. derive the base/candidate/host source projections from the container rather
   than accepting parallel caller arguments;
4. invoke `ContextApplicationV2CurrentnessEvaluator` itself;
5. obtain current HostBinding claims only from the existing HostBinding
   authority admission seam;
6. compare complete source-binding tuples, not only path or raw digest;
7. publish no partial result on any failure.

The injected `AuthoritySourceResolver` is a byte-verification dependency, not
a trusted semantic decision. The caller cannot supply its own current flags,
claim map, or raw privileged resolver state through the interface.

## 6. Currentness composition

### 6.1 Independent lifecycle equations

Slice 6 must evaluate these domains independently:

```text
current_context_records = Slice5Currentness(
    admitted cpar.v2 records,
    admitted cps.v2/cpsr.v2 records,
)

current_host_claims = ExistingHostBindingCurrentness(
    admitted hbcr.v1 records,
    admitted hbcs.v1 records,
)

qualified_current_context = Compose(
    current_context_records,
    ApplicationHostBindingV2 links,
    current_host_claims,
)
```

No HostBinding value may remove, replace, or reinterpret a cpar/cps/cpsr
record. No cpa/cpar value may select a HostBinding claim revision. A failed
HostBinding composition rejects the Slice-6 evaluation but does not mutate or
rewrite either historical lifecycle input.

### 6.2 Current cpa records

The evaluator first calls the existing Slice-5 evaluator with the container's
two record arrays. A cpa group with one current cpar record is current; a
revoked group may have no current record; an ambiguous group fails before any
HostBinding source evaluation. A current cpa record is identified by its
complete `cpar.v2` ID, while its cpa-level link is keyed by complete `cpa.v2`.

### 6.3 Same-cpa acceptance revisions

The same `ApplicationHostBindingV2` remains valid across cpar acceptance
revisions of the same exact cpa ID. The cpa identity includes the theorem and
member preimages, while the cpar identity adds only acceptance-event
metadata. The link is deliberately cpa-level and contains no review-event
reference or record ID. The current cpar record supplies the member set used
for qualification; because the cpa ID is unchanged, that set is identical
across acceptance revisions.

No rebinding is required merely because the current cpar event reference
changes. A record-specific rebinding would contradict the existing additive
`ApplicationHostBindingV2` shape and would introduce acceptance metadata into
HostBinding semantics.

### 6.4 Cross-application replacement

For `A[x] -> B[y]` with `x != y`:

- a link for `x` remains historical input if `x` is an admitted cpa group;
  it does not qualify any current record after Slice 5 leaves `x` without a
  current cpar;
- `y` must independently satisfy its own required-member closure and possess
  its own link when HostBinding is applicable;
- no link or hbc claim is transferred from `x` to `y` by lineage proximity;
- an hbc claim may be referenced by both links only when its complete member
  key is exactly the same and its current observed relationship satisfies both
  reviewed expectations;
- a claim with a different candidate ID, complete candidate digest, or source
  instance is not reusable merely because the cpa lineage is connected.

This preserves the distinction between historical record lineage and the
member-atomic hbc identity domain.

### 6.5 Revoked or no-current cpa groups

An admitted link targeting a known cpa group with no current cpar is retained
as a `historical_only` link result after mechanical validation. It is never
included in `qualified_current_application_record_ids` and cannot make a
revoked group current. Unknown cpa targets are rejected; they cannot become
historical by being placed in the container.

Historical links still must be structurally exact and may reference only
admitted hbc claim IDs. A superseded or revoked hbc claim remains historical
with its exact status; it is not made current merely because its cpa link is
historical. No historical input is deleted or rewritten.

## 7. HostBinding current-claim reuse

### 7.1 Required extraction seam

The existing `AuthorityV2Validator` currently returns only counts. Later
implementation should add a narrow internal/frozen admission result behind
the same validator, conceptually:

```text
HostBindingAuthorityV2ReadModel {
    base_authority_v1_binding: HostBindingSourceBindingV2
    candidate_universe_binding: HostBindingSourceBindingV2 | None
    admitted_claims_by_id: tuple[(str, CrossDeckHostBindingClaimV1), ...]
    current_claims_by_id: tuple[(str, CrossDeckHostBindingClaimV1), ...]
    current_claims_by_member: tuple[
        (ApplicationMemberKeyV1, HostBindingIdentityV1), ...
    ]
    claim_record_status_by_record_id: tuple[
        (str, "current" | "superseded" | "revoked"), ...
    ]
    claim_record_ids_by_claim_id: tuple[(str, tuple[str, ...]), ...]
    used_source_bindings: tuple[HostBindingSourceBindingV2, ...]
}
```

The adapter must run the existing V2 validator's structural parsing, source
resolution, hbcr/hbcs graph checks, current-record checks, correlated witness
validation, and V2 source closure once. The existing `validate(...)` method
may remain a compatibility adapter that discards the read model and returns
the current count result. Slice 6 consumes only the frozen read model.

The seam must preserve:

- duplicate claim-record rejection;
- unknown/superseded/revoked claim rejection;
- multiple current records for one semantic hbc claim rejection;
- one current hbc claim per exact member key;
- source/evidence and correlated witness verification;
- candidate/pair/B2 snapshot consistency;
- existing V1 application-link behavior.

It must not expose raw authority JSON or the validator's mutable working maps.

### 7.2 No second HostBinding lifecycle

Slice 6 must not derive hbc currentness by looking at claim IDs in links,
sorting filenames, choosing the newest hbcr, or reconstructing hbcs graph rules.
The adapter is a seam, not a new authority implementation.

## 8. Member-key mapping

For every ContextApplication member, the exact HostBinding key is:

```text
ApplicationMemberKeyV1(
    candidate_id = member.candidate_id,
    candidate_identity_digest =
        member.candidate_identity_digest_reference.digest_bytes,
    source_instance_id = member.source_instance_id,
)
```

The complete six-field `DigestReferenceV1` remains validated before the digest
bytes are used. `ContextApplicationV2Resolver.resolve_member_source_instance`
must verify the candidate ID, complete candidate identity reference,
candidate-universe binding, and source-instance ID through the existing source
boundary. The HostBinding adapter then requires exact equality of all three
member-key fields.

No truncation, alternate digest domain, candidate name normalization, source
instance alias, internal object ID, or player-visible identity is permitted.
The canonical member-key CBOR bytes are the equality key for duplicate and
coverage checks.

## 9. Required member set and applicability

### 9.1 Recommended executable rule

For a current ContextApplication member, resolve the exact candidate/source
instance through the existing resolver. The member requires a HostBinding
claim exactly when the verified candidate record has:

```text
scope    == "cross_deck"
relation == "directional_binary"
```

This is the narrow applicability rule already implemented by
`AuthorityV2Validator._host_binding_application_members` and exercised by
the existing HostBinding tests. It is based on verified candidate fields, not
capability names, card names, lexical text, ordering, or discovery behavior.

The non-applicable member set is every member outside that exact pair. It
requires no hbc claim and cannot be covered by an extra hbc claim for a
current cpa link. A current application with an empty required set therefore
passes with no ApplicationHostBindingV2 link, provided the normative gap in
§27 is resolved in favor of this existing executable rule.

### 9.2 Exact closure rules

For each current cpa group:

| Required member set | Link rule | Result |
| --- | --- | --- |
| non-empty | exactly one link for the cpa, exact claim/member-key union | qualifies only after all claim checks pass |
| empty | no link permitted | current application qualifies without HostBinding |
| any | duplicate cpa link | reject |
| any | unknown claim, duplicate member, missing member, or extra member | reject |

The same verified applicability predicate applies to historical-only links. A
historical link's claim/member union equals the required subset derived from
the exact admitted cpa member set; it does not expand to all cpa members.
Current versus historical-only status controls whether the link may qualify
authority, not which member closure the link must satisfy.

For a known cpa group with no current cpar, a link is `historical_only` and
does not satisfy or create a current application. Its hbc IDs are still
checked against the admitted/current HostBinding read model appropriate to its
status, and its member union is
checked against the same applicability-derived required subset.

## 10. Reviewed host semantic expectation

The sole Slice-6 semantic expectation is the ContextApplication member's
`context_binding_v1[3]` host relationship. Slice 3 proves that this binding
equals the theorem subject shape and the resolved source-instance shape. The
reviewed bridge's ten context-slot values do not authorize HostBinding to
rewrite it, and historical `SourceContextV1` values do not override it.

For every required member:

```text
claim.observed_host_relationship
    == member.context_binding_v1[3]
```

The expected value must be `same_host` or `cross_host`; a required member with
`not_applicable` fails closed as `HOST_RELATIONSHIP_MISMATCH`. The
HostBinding claim remains a positive observed host fact only. It does not
establish source/affected direction, relation truth, domain applicability,
interaction, separation, exclusivity, or C disposition.

## 11. Host authority binding requirements

The following is the recommended semantic rule, subject to the contract
clarification in §27:

| Situation | `host_binding_authority_v2_binding` | Link/result behavior |
| --- | --- | --- |
| no current required members, no links | `null` | allowed; no HostBinding sources are needed |
| current required members exist, link missing | non-null does not repair the missing link | reject the missing application closure |
| any current or historical link exists | required | resolve and validate the exact V2 HostBinding authority |
| link exists but binding is absent | required error | `HOST_AUTHORITY_BINDING_REQUIRED` |
| binding is non-null but no link/current requirement exists | forbidden | `HOST_AUTHORITY_BINDING_UNEXPECTED` |
| binding role/path/schema/digest is stale or mismatched | present but invalid | `HOST_AUTHORITY_INVALID` |

The container projection and cross-snapshot rules are exact:

```text
context.base_authority_v1_binding
    == host_authority.source_bindings[artifact_role=base_authority_v1]

context.candidate_universe_binding
    == host_authority.source_bindings[artifact_role=candidate_universe]
```

Equality is the complete typed tuple: role, path, schema, and raw digest.
There is no recency, normalization, rebasing, or compatible-snapshot path.

The host authority must be resolved through `AuthoritySourceResolver` and
admitted by `AuthorityV2Validator`; Slice 6 must not parse a second V2 root.

## 12. ApplicationHostBindingV2 validation

Every link is validated mechanically and semantically in this order:

1. exact `ApplicationHostBindingV2` type;
2. exact `application_kind == "context_application"`;
3. complete `cpa.v2` application ID;
4. canonical, duplicate-free `hbc.v1` claim IDs;
5. duplicate cpa-link prepass;
6. target cpa exists in the admitted cpar set;
7. target currentness status is derived from Slice 5, never caller-supplied;
8. every claim ID exists in the HostBinding read model;
9. every claim has the exact expected member key;
10. a current target uses a currently valid hbc claim, while a historical-only
    target uses a known admitted claim and preserves its derived historical
    status;
11. claim-member union equals the required member set for current targets;
12. historical-only targets use the same applicability-derived required subset
    from the admitted cpa member shape but never qualify;
13. every observed relationship equals the member's reviewed host expectation;
14. no superseded/revoked/ambiguous claim qualifies a current cpa; an admitted
    superseded or revoked claim may be retained only for historical-only
    linkage;
15. no automatic x-to-y transfer is performed.

Unknown target cpa IDs, duplicate links, noncanonical IDs, unknown claims,
wrong candidate IDs, wrong complete candidate digests, wrong source instances,
missing members, extra members, and wrong host relationships all fail closed.

The existing DTO's non-empty `host_binding_claim_ids` rule is preserved. A
zero-required current cpa has no link rather than an empty link object.

## 13. V3 review admission and container closure

Slice-4 review admission remains host-free:

```text
ContextApplicationV2ReviewAdmissionValidator.admit(record)
    -> expected_acceptance_source_closure_v3(record, roster)
    -> no HostBinding source bindings
```

Slice 6 must not pass caller-selected HostBinding sources to that public path,
must not add host authority to `ae.v3` source closure, and must not change the
V3 event subject or event identity. HostBinding is composed after independent
application/supersession admission and currentness.

At the container level, host authority and claim-record source bindings may be
included in the context container's exact closure only as an additive
container-level provenance set. They must not be treated as V3 review-event
source bindings. The existing private resolver behavior that passes host
bindings into an application event closure must be split or otherwise
clarified before implementation; see G1 in §27.

No HostBinding source is permitted in a supersession event closure. A host
binding source inserted into a Slice-4 V3 event remains an exact-closure
failure, not a later Slice-6 upgrade path.

## 14. Deterministic validation order

The future evaluator must use this closed order:

1. require the exact typed container and collection types;
2. validate container source projections and canonical duplicate-free arrays;
3. detect duplicate cpar/cpsr/link IDs by canonical identity order;
4. run `ContextApplicationV2CurrentnessEvaluator` before HostBinding source
   evaluation;
5. resolve current-record members and derive required member keys in canonical
   `(cpa.v2, cpar.v2, member-key)` order;
6. validate link target existence, current/historical status, and duplicate
   cpa-link closure;
7. derive whether HostBinding authority is required, optional, or unexpected;
8. resolve/admit the exact HostBinding authority and compare shared snapshots;
9. validate links and claim IDs/status in canonical link/claim order, using
   current claim indexes for current targets and admitted claim records for
   historical-only targets;
10. verify exact claim/member union and reviewed host relationship;
11. verify the separate exact container source closure;
12. materialize and canonicalize the frozen result.

Within a stage, input order is irrelevant. The first diagnostic is selected by
complete canonical-CBOR identity order. No filesystem order, dictionary order,
mtime, timestamp, filename recency, network, RNG, or lexical “latest” rule is
allowed.

If Slice-5 currentness fails, no HostBinding claim or authority file is read.
This gives the graph/currentness failure deterministic precedence and prevents
a malformed HostBinding source from masking an earlier ContextApplication
authority failure.

## 15. Stable error taxonomy

The future module should expose a closed typed error with stable fields:

```text
code
location
cause_code
application_id
record_id
claim_id
member_key
subject_ids
```

Recommended codes are:

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
HOST_MEMBER_APPLICABILITY_INVALID
HOST_SOURCE_CLOSURE_MISMATCH
```

`HOST_CLAIM_NOT_CURRENT` applies to a current cpa target. A historical-only
target may reference an admitted superseded or revoked claim; the evaluator
retains that claim's status and does not qualify the cpa. `HOST_CLAIM_UNKNOWN`
still applies when the historical link names no admitted claim.

`APPLICATION_HOST_BINDING_NONCURRENT_APPLICATION` is not emitted for a known
historical-only cpa link under the recommended policy; that status is a valid
derived result, not an error. It remains an implementation-time negative case
only if the independent contract decision chooses to reject historical links.

`APPLICATION_CURRENTNESS_FAILED` carries the nested Slice-5 code and location;
it does not reinterpret it. Existing HostBinding errors remain structured
cause data. No error classification may parse exception messages.

## 16. Result/read-model design

The minimal frozen derived result is:

```text
ApplicationHostBindingStatus =
    qualified_current
  | historical_only

ApplicationHostBindingResult {
    application_id: AuthorityIdentityV1       # cpa.v2
    status: ApplicationHostBindingStatus
    host_binding_claim_ids: tuple[str, ...]
}

ContextApplicationV2HostBindingEvaluationResult {
    currentness: ContextApplicationV2CurrentnessResult
    qualified_current_application_record_ids: tuple[AuthorityIdentityV1, ...]
    application_host_binding_results: tuple[ApplicationHostBindingResult, ...]
    current_host_claim_ids: tuple[str, ...]
}
```

All tuples are canonical and duplicate-free. `currentness` is the existing
Slice-5 immutable read model. `qualified_current_application_record_ids`
contains current cpar IDs that either have no required HostBinding members or
have passed their exact link/claim closure. `historical_only` entries make
retained x-links/revoked links auditable without allowing them to qualify
authority. `current_host_claim_ids` is the canonical union used by qualified
current links only.

The result contains no raw event/source JSON, mutable dictionaries, `current`
bits, active indexes, file handles, resolver references, or privileged state.
No result is published until all stages pass.

## 17. Failure atomicity and mutation safety

The evaluator is read-only. Every call constructs local tuples and immutable
maps, and it returns only after all checks pass. Rejection must leave unchanged:

```text
ContextApplication records and supersession records
ApplicationHostBindingV2 values
HostBinding claim/supersession inputs
review-event references and source files
currentness inputs and IDs
C artifacts and production directories
```

Later tests must snapshot typed `to_cbor()`/`to_wire()` values, all referenced
raw-file digests, and the complete temporary repository tree before and after
repeated rejection calls. An error fingerprint must remain identical under
input permutation and repeated evaluation.

## 18. Positive test matrix

The later implementation suite must include at least:

| ID | Scenario | Required result |
| --- | --- | --- |
| P1 | one current cpa with non-empty required host set and exact complete link | `PASS`; one qualified cpar |
| P2 | same cpa with historical cpar revisions and one current record | `PASS`; one cpa-level link remains valid |
| P3 | `A[x] -> B[y]`, independent complete link for current `y` | `PASS`; x historical-only or absent, never transferred |
| P4 | one current hbc claim referenced by two links with exactly identical member key | `PASS` only when both reviewed host expectations match |
| P5 | multiple independent current cpa groups with complete links | `PASS`; canonical qualified tuple for each |
| P6 | current app with empty required set and no link | `PASS` if the accepted applicability rule is the existing cross-deck/directional rule |
| P7 | valid hbcr/hbcs lifecycle leaves exactly one current hbc claim per member | `PASS` through existing HostBinding seam |
| P8 | permutation of all top-level arrays and claim arrays | byte/value-identical result |
| P9 | reviewed-divergence context whose member host expectation is the reviewed theorem binding | `PASS`; historical SourceContext is not used as override |
| P10 | host-free V3 review admission followed by container-level HostBinding composition | V3 admission unchanged; Slice 6 result qualifies |
| P11 | known revoked/no-current cpa with a mechanically valid historical link | `PASS`; historical-only result, zero qualified record |
| P12 | historical-only cpa link references a known superseded or revoked hbc claim | `PASS`; exact historical identity/status retained, zero qualification |

## 19. Negative test matrix

At minimum:

| ID | Mutation | Expected class |
| --- | --- | --- |
| N1 | wrong `application_kind` | `APPLICATION_HOST_BINDING_INVALID` |
| N2 | malformed cpa ID | typed input failure |
| N3 | unknown cpa | `APPLICATION_HOST_BINDING_UNKNOWN_APPLICATION` |
| N4 | duplicate cpa link | `APPLICATION_HOST_BINDING_DUPLICATE` |
| N5 | noncanonical claim IDs | typed input failure |
| N6 | duplicate claim IDs | typed input failure |
| N7 | unknown hbc ID | `HOST_CLAIM_UNKNOWN` |
| N8 | current cpa link references a superseded hbc claim | `HOST_CLAIM_NOT_CURRENT` |
| N9 | current cpa link references a revoked hbc claim | `HOST_CLAIM_NOT_CURRENT` |
| N10 | two current hbc claims for one member | `HOST_BINDING_AMBIGUOUS` |
| N11 | missing required member claim | `HOST_MEMBER_SET_MISMATCH` |
| N12 | extra non-required member claim | `HOST_MEMBER_SET_MISMATCH` |
| N13 | wrong candidate ID | `HOST_MEMBER_SET_MISMATCH` |
| N14 | wrong complete candidate digest | `HOST_MEMBER_SET_MISMATCH` or `HOST_CLAIM_NOT_CURRENT` from source rebind |
| N15 | wrong source instance | `HOST_MEMBER_SET_MISMATCH` |
| N16 | wrong observed host relationship | `HOST_RELATIONSHIP_MISMATCH` |
| N17 | required link/host authority binding absent | `HOST_AUTHORITY_BINDING_REQUIRED` or application closure error by precedence |
| N18 | wrong host authority path/schema/digest | `HOST_AUTHORITY_INVALID` |
| N19 | host authority present with no link or requirement | `HOST_AUTHORITY_BINDING_UNEXPECTED` |
| N20 | link for revoked cpa with an invalid/unknown target | unknown-target failure; no current qualification |
| N21 | historical-only link for a known no-current cpa references an unknown or unadmitted claim | `HOST_CLAIM_UNKNOWN`; no silent acceptance |
| N22 | stale x link after cross-app x -> y | historical-only x; no automatic current qualification |
| N23 | automatic hbc transfer x -> y | no transfer; missing y closure or claim-key failure |
| N24 | HostBinding source inserted into V3 event closure | Slice-4 exact-closure failure |
| N25 | duplicate/malformed IDs under permutations | identical structured diagnostic |
| N26 | adversarial mutation of typed DTO internals | `HOST_INTEGRATION_INPUT_INVALID` or typed child error |
| N27 | graph/currentness failure with otherwise malformed HostBinding | `APPLICATION_CURRENTNESS_FAILED`; HostBinding is not evaluated |
| N28 | HostBinding failure after a valid currentness result | no partial qualified result and unchanged snapshots |

The tests must also cover wrong source snapshot, wrong claim path/raw digest,
wrong candidate-universe snapshot, extra source bindings, missing source
bindings, duplicate source role/path pairs, and a different hbc claim with the
same candidate name but a different complete member key.

## 20. Information-safety impact

Slice 6 introduces no player-facing API and no new player-visible identifier.
The composition module remains maintainer-side authority tooling under
`scripts/`. It must not be imported by `PlayerEndpoint`, `PlayerObservation`,
`PlayerInformationState`, ML action construction, or trajectory generation.

HostBinding source facts remain source/evidence facts. They cannot be copied
into observations, retained knowledge, actions, rewards, trajectories, or
player-visible IDs. If a downstream player-facing dependency is discovered in
implementation, Slice 6 is blocked and the dependency requires a separate
information-safety review.

## 21. Replay and determinism impact

The intended impact is:

```text
authoritative engine state       NO CHANGE
RNG                              NO CHANGE
replay schema                    NO CHANGE
checkpoint schema                NO CHANGE
state hashing                    NO CHANGE
Decision protocol                NO CHANGE
PlayerObservation                NO CHANGE
PlayerInformationState           NO CHANGE
ML trajectory schemas            NO CHANGE
```

Slice 6 evaluates immutable authority inputs and produces a derived read model;
it does not alter runtime state, replay bytes, checkpoints, state digests, or
decision legality. A discovered runtime import or persisted-current index is
an architectural blocker, not an implementation detail.

## 22. Schema, identity, canonical preimage, and Rust impact

The recommended implementation requires:

```text
identity changes                 NO
schema changes                   NO
canonical-CBOR preimage changes  NO
Rust DTO changes                 NO
persisted current indexes        NO
```

`ApplicationHostBindingV2`, `ContextApplicationAuthorityV2`, the HostBinding
identities, and source roles already exist. The future work belongs in Python
composition/read-model adapters and tests. If implementation appears to need
an empty `ApplicationHostBindingV2`, a record ID in a claim, a new source role,
or a Rust/schema field, it must stop and report a new `CONTRACT_GAP`.

The event/container closure ambiguity is a semantic/ADR issue, not permission
to change a schema or identity preimage.

## 23. Reuse versus duplication audit

The design intentionally reuses:

- `ContextApplicationV2CurrentnessEvaluator` for all cpar/cps/cpsr semantics;
- `ContextApplicationV2ReviewAdmissionValidator` and the shared V3 binding seam
  for application/supersession admission;
- `ContextApplicationV2Resolver.resolve_member_source_instance` for exact
  candidate/source-instance binding;
- `HostBindingSourceResolver.resolve_claim_for_member` for source-bound hbc
  claim verification;
- `AuthorityV2Validator` for the HostBinding V2 root, hbcr/hbcs lifecycle,
  source closure, reviewer/event closure, and current-claim checks;
- `ContextApplicationV2Resolver` for context source-role and container closure
  algebra after the G1 event/container rule is resolved.

The design forbids:

- a second supersession engine;
- a second hbc currentness graph;
- a second source/evidence parser;
- a second reviewer policy;
- host inference from capability/card/file/discovery ordering;
- a giant validator that owns both ContextApplication semantics and HostBinding
  source resolution.

## 24. Minimal future file ownership

The later implementation should be limited to the smallest coherent set:

```text
scripts/context_application_v2_host_binding.py
    Slice-6 composition, link validation, result, and stable diagnostics

scripts/authority_v2_validator.py
    narrow frozen HostBinding read-model seam; preserve validate() behavior

scripts/context_application_v2_resolver.py
    only the G1-approved separation of host container provenance from V3 event
    closure, if the existing helper cannot express it without ambiguity

python/tests/test_context_application_v2_host_binding.py
    Slice-6 positive, negative, permutation, and mutation matrix

python/tests/test_authority_v2_validator.py
    extraction-compatibility tests; existing behavior must remain unchanged

python/tests/test_context_application_v2_resolver.py
    exact host-free event/container closure regression tests
```

The following remain frozen unless a new contract gap is proved:

```text
python/src/mtgml/authority.py
python/src/mtgml/host_binding.py
schemas/context-application-authority.v2.schema.json
schemas/interaction-review-authority.v2.schema.json
crates/mtgml-persistence/src/authority.rs
all accepted identity/preimage definitions
production authority/source artifacts
```

## 25. Later implementation decomposition (recommendation only)

This is a design decomposition, not an implementation plan and not an
authorization to start implementation.

### 6.1 Typed composition/trust-boundary seam

- **Owned files:** new `scripts/context_application_v2_host_binding.py` and
  its new focused test module.
- **RED tests:** exact typed container requirement; no trusted/current flags;
  frozen result surface; duplicate top-level IDs; currentness-first failure.
- **Semantic responsibility:** establish the sole public Slice-6 seam and
  compose Slice 5 without HostBinding input.
- **Explicit non-responsibility:** no hbc parsing, no source artifact creation,
  no event-closure policy change.
- **Acceptance gates:** focused tests, smoke profile, diff check.
- **STOP condition:** any need to alter DTO/schema/identity or to accept a
  caller-selected HostBinding snapshot.

### 6.2 HostBinding current-claim read-model adapter

- **Owned files:** `scripts/authority_v2_validator.py` plus compatibility tests.
- **RED tests:** exact frozen current-claim map, duplicate-member ambiguity,
  stale hbcr/hbcs rejection, V1 validator result parity.
- **Semantic responsibility:** expose the existing HostBinding lifecycle once
  as a typed read model.
- **Explicit non-responsibility:** no ContextApplication or cpa linkage.
- **Acceptance gates:** all existing HostBinding tests, full Python profile,
  source-closure checks.
- **STOP condition:** extraction changes existing error precedence or current
  claim semantics.

### 6.3 Exact ApplicationHostBindingV2 closure

- **Owned files:** Slice-6 module/tests; resolver tests only if G1 requires it.
- **RED tests:** kind/ID/canonicality, duplicate links, unknown applications,
  exact claim/member union, wrong key fields, stale claim, cross-snapshot
  mismatch.
- **Semantic responsibility:** validate each cpa-level link and historical-only
  status.
- **Explicit non-responsibility:** no automatic transfer and no cpa currentness.
- **Acceptance gates:** focused matrix, permutation fingerprints, source
  closure, mutation snapshots.
- **STOP condition:** member applicability cannot be resolved by the accepted
  contract clarification.

### 6.4 Current ContextApplication × HostBinding composition

- **Owned files:** Slice-6 module/tests.
- **RED tests:** same-cpa revisions, x->y replacement, revocation, multiple
  independent current groups, empty required set, reviewed-divergence host
  expectation.
- **Semantic responsibility:** derive qualified current cpar IDs and current
  claim union from two independent read models.
- **Explicit non-responsibility:** no new lifecycle algorithm and no semantic
  interaction conclusion.
- **Acceptance gates:** Slice-5 smoke tests, HostBinding tests, Slice-6 suite,
  deterministic result equality.
- **STOP condition:** HostBinding changes a Slice-5 result or a historical link
  is treated as current by absence of a cpa record.

### 6.5 Adversarial, determinism, and mutation matrix

- **Owned files:** Slice-6 tests and, only where necessary, resolver tests.
- **RED tests:** all N1–N28, top-level permutations, malformed typed DTOs,
  repeated rejection snapshots, source-byte tampering.
- **Semantic responsibility:** prove fail-closed behavior and stable error
  fingerprints.
- **Explicit non-responsibility:** no production canary or authority artifact.
- **Acceptance gates:** full Python profile, documentation/maintainer checks,
  relevant Rust structural checks only if touched.
- **STOP condition:** any test requires production data or a real Buckle-Up
  acceptance.

### 6.6 Final integration gates

- **Owned files:** no new ownership; review package only.
- **RED tests:** exact-source closure, information-safety scan, no schema/Rust
  drift, scope gate including untracked files.
- **Semantic responsibility:** demonstrate the implementation remains within
  the accepted Slice-6 boundary.
- **Explicit non-responsibility:** no implementation plan, production records,
  Buckle-Up canary, Task 5 Slice 3B, or M3.
- **Acceptance gates:** repository-defined docs, Python, schema, Rust, and
  conformance gates applicable to the actual touched files; all evidence
  separately labeled.
- **STOP condition:** any required contract gate is `FAIL`, `BLOCKED`, or
  `NOT_RUN`.

## 26. Acceptance gates for this design-only task

The design branch changes documentation and the document register only. The
required gates are:

```text
git diff --check                         required
python scripts/check_documentation.py    required
python scripts/validate_maintainer_artifacts.py
                                          required
```

The following are not design-task acceptance gates and must not be promoted to
Slice-6 implementation evidence:

```text
production authority preflight            NOT_RUN / not applicable
real Buckle-Up canary                     NOT_RUN / not authorized
Slice-6 implementation tests              NOT_RUN
Rust Slice-6 semantic implementation      NOT_RUN
```

The already executed smoke profile is baseline evidence only; it does not
claim Slice-6 behavior.

## 27. Contract gaps and ADR requirement

### G1 — V3 event closure versus container HostBinding closure

**Verified conflict:**

- ADR 0042 §10.1 includes optional `host_binding_authority_v2` and
  `host_binding_claim_record` bindings in `ExpectedAcceptanceSourceClosureV3`.
- The accepted Slice-4 design §13 and its public implementation call
  `expected_acceptance_source_closure_v3(record, roster)` without HostBinding
  arguments and reject an extra HostBinding source as a V3 closure mismatch.
- The current resolver has a private host-binding parameter and a container
  helper that can pass host bindings into an application event closure, while
  the public event resolver remains host-free.

**Why implementation cannot safely choose:**

Choosing event-level HostBinding sources would change the V3 acceptance-event
source contract and Slice-4 behavior. Excluding them from all closure
calculations would contradict the explicit ADR 0042 container-participation
wording. Both choices affect what an existing `ae.v3` event is required to
bind.

**Minimal options:**

1. amend ADR 0042 and Slice 4 to permit explicitly declared HostBinding sources
   in an application event closure;
2. preserve host-free V3 event closure and put HostBinding authority/claim
   bindings only in the context-container closure union;
3. create a second event version, which is unnecessary and would widen frozen
   identity/schema surface.

**Recommendation:** option 2. It preserves the already implemented Slice-4
host-free admission and keeps HostBinding as additive container provenance.
Clarify ADR 0042 §10.1 and the resolver's private helper in a new reviewed
ADR-0044 candidate. No identity, schema, or Rust change should be needed.

### G2 — Required HostBinding member-set authority

**Verified conflict/insufficiency:**

- `CROSS_DECK_HOST_BINDING_REVIEW_CHECKLIST.md` says every current V1 Relation,
  Domain, and Context Application requires a host-binding link.
- `AuthorityV2Validator._host_binding_application_members` and its tests define
  required coverage only for verified `cross_deck` + `directional_binary`
  candidate members; non-cross-deck applications explicitly pass without a
  link.
- ADR 0042 defines the additive cpa.v2 link but does not state the V2
  required-member predicate.

**Why implementation cannot safely choose:**

Requiring all ContextApplication members would make a different semantic
closure from the existing HostBinding validator and would make the existing
empty-required-set behavior invalid. Reusing the executable predicate without
normative clarification would promote provisional implementation behavior over
conflicting process text.

**Minimal options:**

1. require every V2 member;
2. require only verified `cross_deck` + `directional_binary` members;
3. add a new persisted applicability field, which would require schema and
   identity changes.

**Recommendation:** option 2, codified as an additive semantic clarification.
It preserves the existing source-bound HostBinding authority and requires no
identity/schema/Rust change. The checklist wording should be amended in the
same ADR/process review so it no longer contradicts executable behavior.

### G3 — Presence of unused HostBinding binding and historical links

ADR 0042 makes the top-level HostBinding binding nullable, while the current
resolver tolerates a non-null authority binding with no links. It does not
state whether a known cpa link with no current cpar is forbidden, ignored, or
historical-only. Slice 6 needs this distinction for exact closure and audit.

**Recommendation:** allow a known no-current cpa link only as a mechanically
validated `historical_only` result; require HostBinding authority whenever any
link exists; require current hbc claims for current cpa links but only an
admitted exact hbc identity plus its derived status for historical-only links;
reject a non-null authority binding when no current requirement and no link
exists. Codify these statuses in the same ADR candidate. Do not delete
historical links, rewrite their claim IDs, or silently accept unknown targets.

### Gap status

```text
CONTRACT_GAP_FOUND       = YES
ADR_REQUIRED             = YES
IDENTITY_CHANGE_REQUIRED = NO
SCHEMA_CHANGE_REQUIRED   = NO
PREIMAGE_CHANGE_REQUIRED = NO
RUST_CHANGE_REQUIRED     = NO
IMPLEMENTATION_ALLOWED   = NO
```

## 28. Buckle-Up boundary

Slice 6 design prepares only the infrastructure boundary. It must not:

- accept or classify Buckle-Up;
- write Buckle-Up production records or events;
- create hbc/cpar/cps/cpsr production artifacts;
- modify C;
- claim interaction closure;
- start Task 5 Slice 3B; or
- begin M3.

The eventual sequence remains:

```text
accepted Slice-6 implementation
    -> independent implementation review
    -> real Buckle-Up candidate review
    -> accepted/current cpar.v2
    -> exact ApplicationHostBindingV2
    -> exact current hbc.v1 claims
    -> qualified Slice-6 authority
    -> E2E canary evaluation
```

No step after the design is self-authorized here.

## 29. Design-stage status

```text
TASK                                      = SLICE_6_DESIGN_ONLY
BASE                                     = d647e63d7687bd2c022393feaee5bac6736e84ca
REMOTE_HEAD                              = d647e63d7687bd2c022393feaee5bac6736e84ca
PRODUCTION_CODE_CHANGED                  = NO
PRODUCTION_AUTHORITY_CREATED             = NO
SCHEMA_CHANGED                           = NO
IDENTITY_CHANGED                         = NO
PREIMAGE_CHANGED                         = NO
RUST_CHANGED                             = NO
SLICE_6_DESIGN_CREATED                   = YES
SLICE_6_IMPLEMENTATION_PLAN_AUTHORIZED   = NO
SLICE_6_IMPLEMENTATION_AUTHORIZED        = NO
BUCKLE_UP_CANARY_AUTHORIZED              = NO
TASK_5_SLICE_3B                           = BLOCKED
M3                                        = BLOCKED
CONTRACT_GAP                              = YES
ADR_REQUIRED                              = YES
```

The branch must stop after this design commit and independent design review.
