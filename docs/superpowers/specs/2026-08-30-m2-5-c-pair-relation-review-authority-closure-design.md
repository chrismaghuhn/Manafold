# M2.5.C Task 1 — Pair/Relation and Review-Authority Closure Specification

**Status:** proposed implementation-ready specification; Task 1 only

**Date:** 2026-08-30

**Reviewed base:** origin/master at
37afeb8fceca728ab727b2df76defab9722890c0

**Primary output of this task:** this document only. It is a non-authoritative
design artifact. It is not a C closure input, semantic authority, production
implementation, checker implementation, or classification snapshot.

## 0. Scope and decision boundary

This document specifies the smallest trustworthy authority architecture that a
later implementation could use to resolve the current M2.5.C candidate
snapshot. It does not resolve any candidate and does not change the current C
snapshot or any upstream source.

The current C result remains the correct fail-closed state:

~~~
candidate_count       = 15,679
semantic_class_count  = 0
unresolved            = 15,679
DECLARED_INTERACTION_MODEL_CLOSURE = BLOCKED
~~~

The existing source and identity boundaries remain authoritative:

~~~
REV3 source identity      -> candidate and source-instance provenance
B2 terminal authority     -> card-to-family assignments and family boundaries
B1.Final authority        -> official rule/citation graph
C review authority        -> future accepted semantic review records
C classification          -> deterministic derivative, never source authority
~~~

The proposed authority must be added in a future, separately reviewed change.
That change must amend the C input graph and its checker in a new versioned C
contract. It must not add a file to the closed current
sources/m2_5/closures/C/ inventory or mutate the current V4 snapshot.

## 1. Executive conclusion

Pair/Relation Authority alone is not sufficient. A candidate becomes
terminally classifiable only when the candidate-specific relation conclusion,
all required review-domain conclusions, and any disposition-specific context
and scope evidence form one accepted, source-grounded closure.

The minimum complete authority set is:

| Authority | Owns | Does not own |
| --- | --- | --- |
| Existing source identity authority | exact REV3 rows, B2 records, B1.Final nodes, raw digests, and locators | interaction truth merely because a source is present |
| RelationProof theorem | a reusable semantic relation, separation, or model-boundary proposition | which candidates or source instances satisfy its preconditions |
| Exact relation application | the exact candidate/source-instance members to which one theorem applies, with a proof of every member precondition | a new relation theorem or a wildcard population |
| DomainProof theorem and exact domain application | one of the eleven C-domain applicability conclusions and its member preconditions | the candidate terminal disposition |
| ContextProof theorem and exact context application | class context and temporal values, with positive evidence for every value | relation truth or domain applicability |
| Acceptance and supersession authority | human acceptance provenance, immutable revisions, and explicit replacement/revocation | semantic meaning in the absence of the accepted proof payload |
| Current C candidate/source ledger | candidate identity, source-instance identity, and exact source binding | proof of interaction, non-interaction, or scope |

The theorem/application split is mandatory. A theorem is a semantic statement;
an application is an exact binding of that statement to a finite member set.
Every application member carries structured evidence that its preconditions
match the theorem. An enumerated member list without that per-member proof is
not equivalence evidence and is rejected.

The recommended later implementation is therefore:

~~~
accepted source artifacts
        +
immutable accepted proof theorems
        +
exact per-member applications with precondition attestations
        +
disposition-specific closure validation
        ↓
deterministic C classifications and semantic classes
~~~

## 2. Current C authority-gap audit

The live accepted base was checked before authoring this document. The exact
accepted upstream identities include:

~~~
REV3 package                         99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90
REV3 candidate member                82f9312113bb1007ad6562d454c515f85dbc1e0d7a471f7b1c6793725aea45d4
B2 family catalog                    a9dc94b86a2efdb6885081191e53380cf5b3723a58487600b6372bcb789abb92
B2 classifications                   40cd5b9c37e26157a6df0449a75040f8a5879d825e3946dd500d666a502201d5
B2 closure                           ed6a0bf4b0eb83c85027fdcc61eaf32bfa7bb06d4de78c77d0946d87212e7d43
B1.Final citations                   aaf684335be10255843f4b5debd6fed71835043eef9e585c5fa024109248a25a
B1.Final closure                     b6980cffcb71bf73acba6ef698a418ad83cac21bbd0121a4ca9becfe6d630dea
~~~

The upstream accepted counts are 216 B2 families, of which 210 are ACTIVE
and six are ACTIVE_UNASSIGNED; 402 terminal B2 classifications; 1,883
terminal assignments; 441 B2 projection rows; and seven B1.Final authority
nodes. These are prerequisite facts, not interaction proof.

### 2.1 Candidate and current-artifact audit

The current C candidate ledger contains exactly:

| Candidate shape | Count | Current meaning |
| --- | ---: | --- |
| intra_deck + unordered_binary | 8,131 | pair candidate with canonical unordered participant order |
| cross_deck + directional_binary | 7,530 | ordered candidate; REV3 left-to-right order is preserved but does not itself prove source/affected roles |
| unary_or_higher_order + declared_card_trigger | 18 | exact joined card/OSI candidate; not grandfathered as a trigger |
| **Total** | **15,679** | every candidate has one source instance |

The current C artifacts establish these facts:

* interaction_candidate_universe.v2.json preserves every REV3 row, candidate
  identity, source binding, and source instance.
* interaction_review_additions.v2.json has zero accepted targeted-review
  records. It is an input for candidate proposals, not relation authority.
* interaction_semantic_classes.v2.json has zero classes.
* The 16 classification shards contain one record for every candidate. All
  records are unresolved.
* The 15,661 binary records use
  insufficient_pair_relation_authority.
* The 18 card-trigger records use missing_required_review_evidence.
* Every classification carries eleven unresolved domain assessments. The
  source evidence attached to those assessments is retained provenance, not
  positive domain applicability proof.

### 2.2 Production-validator audit

The current scripts/check_m2_5_c_interactions.py confirms the authority gap
explicitly:

* PRODUCTION_EVIDENCE_AUTHORITIES contains only rev3, b2, and b1_final.
* c_review is accepted only for test-only fixtures under test-only/.
* PAIR_RELATION_AUTHORITY_PATHS is empty.
* DOMAIN_REVIEW_AUTHORITY_PATHS is empty.
* CLASS_CONTEXT_AUTHORITY_PATHS is empty.
* validate_noninteraction_proof() rejects a production non-interaction
  because no admitted Pair/Relation authority exists.
* validate_review_domain_assessments() requires positive evidence for
  applicable and not_applicable, but the current production path has no
  admitted domain-review authority path. unresolved is the correct current
  value.
* validate_class_context_evidence() requires a positive locator for every
  context and temporal slot, but no admitted class-context authority exists.
  The check is exercised by test fixtures because the current class set is
  empty.
* validate_classes() checks B2 boundary references, B1.Final citation
  references, context slots, and the existing class digest. It does not and
  cannot currently verify an accepted candidate-specific relation theorem.
* validate_classifications() has no field for a relation-authority
  application and requires current classification evidence to match the
  source-derived REV3/B2 evidence set. A future authority cannot be inserted
  into this V3 record by adding an unrecognized reference.
* validate_closure() binds exactly five current C semantic inputs. It has no
  input slot for a review-authority artifact.

### 2.3 Missing versus already-satisfied authority

The following are genuine prerequisites for future C resolution:

| Requirement | Current state | Needed for |
| --- | --- | --- |
| Exact candidate identity and source-instance binding | present in the C ledger | every disposition |
| B2 family lifecycle, assignment, and precise boundary | present upstream | every card/family-bound candidate |
| B1.Final official citation graph | present upstream | every rule claim that a proof uses |
| Candidate-specific positive relation proof or positive separation proof | absent | every terminal relation disposition |
| Exact theorem-to-member precondition proof | absent | every reused theorem application |
| Candidate/domain-specific applicability or non-applicability proof | absent | every resolved candidate under the current eleven-domain C contract |
| Context and temporal proof | absent | every emitted required-interaction class and every proof that depends on context |
| Explicit model-bound scope proof | absent | every out-of-scope disposition |
| Human acceptance and immutable supersession lineage | absent | every production authority record |

The following are not missing C prerequisites and must remain outside this
task: ranking, reuse-ratio reconstruction, deck-pair selection, deck lock,
M3, runtime Magic semantics, Card IR, and certification.

## 3. Exact answer: is Pair/Relation Authority alone sufficient?

No.

Pair/Relation Authority can establish only the relation-level proposition. The
current C contract separately requires eleven domain assessments for every
resolved candidate. A required interaction also requires context and temporal
values sufficient to construct a valid semantic class. An out-of-scope result
requires a positive proof of the declared model boundary. A non-interaction
requires positive separation, not the absence of a positive relation theorem.

The minimum additional authorities beyond a relation theorem/application are:

1. **Domain applicability authority.** One accepted, exact application for
   each of the eleven review domains, with applicable or not_applicable
   proven from candidate/domain-specific evidence. Missing evidence remains
   unresolved.
2. **Class-context authority.** A source-grounded context and temporal proof
   for every emitted required-interaction class, with a separately proven
   application to each member. It is not needed for a non-interaction or
   scope disposition unless that proof explicitly depends on context.
3. **Scope authority.** A model-bound proof for
   out_of_declared_scope_with_reason. This is a proof variant of the
   relation authority, not a convenient fourth terminal bucket.
4. **Acceptance/supersession authority.** A valid semantic record is not
   production authority merely because it is syntactically present. The
   record needs human-acceptance provenance and immutable revision handling.

B2 and B1.Final remain required inputs, but they are not replaced by these
new records and they do not become pair/relation authority by being cited.

## 4. Authority taxonomy and ownership

### 4.1 Existing authorities

* **REV3 source authority** owns immutable acquisition, archive-member, row,
  and source-instance provenance. Its historical
  AMBIGUOUS_REQUIRES_REVIEW value is not a terminal C disposition.
* **B2 authority** owns reviewed card-to-family assignment edges and the
  exact B2_SEMANTIC_BOUNDARY_V1 value for each family. ACTIVE_UNASSIGNED
  is never valid card-derived proof.
* **B1.Final authority** owns the accepted official citation graph and its
  citation-to-boundary dependency review. A citation node proves the cited
  official rule domain exists in the graph; it does not prove that two
  candidates interact.
* **C candidate ledger** owns the finite current candidate set and concrete
  source-instance records. It does not own semantic truth.

### 4.2 New authority types

The future review-authority artifact contains three theorem families and three
application families:

| Type | Theorem identity excludes | Application identity includes |
| --- | --- | --- |
| RelationProofV1 | candidate ID and source-instance ID | exact candidate identity, source instance, participant bindings, and member precondition attestations |
| DomainProofV1 | candidate ID and source-instance ID | exact candidate/source-instance member and domain precondition attestations |
| ContextProofV1 | candidate ID and source-instance ID | exact candidate/source-instance member and context precondition attestations |

RelationProofV1.proof_kind has exactly three semantic variants:

~~~
positive_interaction
positive_separation
model_bound_scope
~~~

These map respectively to:

~~~
required_interaction
not_an_interaction_with_proof
out_of_declared_scope_with_reason
~~~

Unary, binary, directional, symmetric, and finite higher-order relations are
subject variants. They are not alternate authorities.

### 4.3 Acceptance is a cross-cutting authority

The authority artifact may contain only accepted proof/application records and
explicit historical supersession records. A proposal is not accepted merely
because it appears in a file. Acceptance requires:

~~~
proposal
  -> structural and binding validation
  -> human semantic review
  -> acceptance event bound to exact source identities
  -> immutable accepted record
  -> optional later supersession or revocation record
~~~

Acceptance provenance is not semantic proof. It establishes that the semantic
proof was reviewed and accepted under a declared process. The acceptance
process is itself closed: the event schema, event identity, authorized roles,
record payload binding, source bindings, and portable evidence are defined in
§12.2 and §13.3. A generator cannot create human_accepted merely by writing
that enum value.

## 5. Positive interaction proof contract

positive_interaction proves that the exact reviewed semantic participants
have a causal relation inside the declared C model. It must state the
participating boundaries, the relation shape, and the mechanism that connects
the participants. Candidate co-occurrence, shared deck membership, shared
capability names, lexical similarity, or a B2 family assignment is never the
mechanism.

### 5.1 Required positive proof content

Every positive relation theorem must establish all applicable obligations:

1. **Participant identity.** Each participant is an exact semantic reference
   from the admitted source ledger. A family participant resolves to its B2
   family ID and precise boundary; a card participant resolves to its exact
   OSI/source identity.
2. **Arity and shape.** The theorem declares unary, binary, or finite
   higher_order arity and the exact relation shape.
3. **Direction.** An unordered relation uses canonical participant ordering.
   A directional relation preserves explicit ordered positions. The theorem
   must not infer source and affected from REV3 left and right labels.
4. **Causal mechanism.** A finite, ordered causal chain identifies the
   reviewed operation, event/effect boundary, and affected participant. Each
   edge is backed by a B2 boundary and, where a rule claim is made, a
   B1.Final citation.
5. **Applicable context dependencies.** The theorem lists context
   preconditions it actually requires. It does not invent values for
   irrelevant dimensions; those values are handled by the context authority.
6. **Information and actor behavior.** If the relation depends on visibility,
   identity, controller, owner, or decision actor, the dependency is explicit.
7. **Temporal behavior.** If the relation depends on event order, duration,
   trigger timing, replacement order, or LKI, the dependency is explicit.
8. **Boundary completeness.** The proof identifies the exact B2 boundary
   fields used and demonstrates that adjacent concepts excluded by those
   boundaries are not silently substituted.

### 5.2 Causal-chain representation

The semantic proof payload is a closed structured value, not a prose-only
claim. Its normalized form is the fixed CausalChainEdgeV1 array:

~~~
causal_chain = [
  CausalChainEdgeV1, ...
]
~~~

CausalChainEdgeV1 is exactly:

~~~
[ordinal_u32, from_role_position_u32, operation_enum,
 through_boundary_refs_sorted, event_or_effect_role_position_or_null,
 to_role_position_or_null, b1_final_citation_refs_sorted]
~~~

ordinal_u32 is the complete edge identity in V1. The chain must contain
exactly 0, 1, ..., n-1 in order. through_boundary_refs_sorted contains
B2BoundaryRefV1 values sorted by canonical CBOR bytes, and
b1_final_citation_refs_sorted contains B1FinalCitationRefV1 values in their
canonical order. V1 defines no second edge identity. Applications bind this
chain through the exact causal_chain_ordinals value defined in §12.4 and
§12.10.

operation_enum is a closed enum owned by this future authority schema:

~~~
reads
changes_characteristic
changes_eligibility
changes_target_legality
changes_controller
changes_ownership
changes_zone
creates_object
copies_value
replaces_event
triggers_ability
orders_event
supplies_choice
~~~

The exact relation-channel vocabulary is:

~~~
participant_boundary
event_or_effect_causality
target_or_choice
zone_or_object_identity
control_or_ownership
replacement_or_layer
trigger_or_lki
information_or_visibility
ordering_or_temporal
decision_actor
format_and_declared_scope
~~~

RequiredRelationChannelsV1 is a duplicate-free array of these enum values in
the order shown. Its empty form is allowed when the causal chain has no
additional relation-channel obligation; it is still a present field.

If a required operation is not representable, the authority cannot be
accepted. It does not add an other escape hatch; a versioned authority-model
amendment is required.

The chain is a reviewed semantic proof record. It is not a runtime evaluator,
rules interpreter, or automatic legality engine.

### 5.3 Positive proof and disposition

The theorem's proof_kind = positive_interaction maps to
required_interaction only when its exact application is accepted and the
disposition-specific closure matrix in §14 passes. A valid relation theorem by
itself does not emit a class or resolve a candidate.

## 6. Non-interaction proof contract

not_an_interaction_with_proof requires positive separation evidence. It means
that the reviewed participants are co-occurring or independently meaningful,
but the declared relation does not exist under the exact C model and reviewed
context. It never means that the reviewer failed to find an interaction.

### 6.1 Separation theorem

The separation payload has:

~~~
separation_kind =
  boundary_disjointness
  | closed_channel_exclusion
  | independent_effect_separation

separation_obligations = [
  SeparationObligationV1, ...
]
~~~

The closed channel vocabulary is:

~~~
participant_boundary
event_or_effect_causality
target_or_choice
zone_or_object_identity
control_or_ownership
replacement_or_layer
trigger_or_lki
information_or_visibility
ordering_or_temporal
decision_actor
format_and_declared_scope
~~~

The theorem must contain exactly one SeparationObligationV1 for every channel
in the vocabulary, in that order. The obligation fixes whether each member
must prove separated or not_relevant. The relation application records and
proves that exact conclusion for every member. An omitted, unknown, or
unresolved channel is not separation proof.

not_applicable in a domain assessment does not itself prove separation. It
only proves that the domain is irrelevant to the separate domain review.

### 6.2 Forbidden non-interaction shortcuts

The following are rejected even if they are repeated for every candidate:

~~~
B2 family records without a separation theorem
keyword or lexical disjointness
absence of a positive interaction record
absence of a matching card or rule
capability-name comparison
deck or family co-occurrence alone
model/LLM confidence or heuristic score
one unresolved review domain
incomplete participant coverage
~~~

The source evidence must positively establish the separation conclusion. If a
single relevant channel cannot be separated, the candidate remains unresolved.

## 7. Out-of-declared-scope proof contract

out_of_declared_scope_with_reason is valid only when a candidate is
provably outside the finite boundary of the declared model. It is not a
storage location for difficult, expensive, or currently unsupported review.

### 7.1 Closed scope reasons

The first implementation may use only these reason codes:

~~~
unbounded_n_way_not_representable
undeclared_participant_kind
undeclared_relation_shape
undeclared_outcome_surface
~~~

Each code must resolve to an exact field in the admitted model
declared-interaction-model.v2, its coverage_scope, or its excluded_claims. The
proof contains:

~~~
model_id
model_version
model_boundary_locator
reason_code
observed_candidate_shape
positive_boundary_evidence_refs
~~~

missing_b1_final_citation, missing_relation_review, missing domain evidence,
unknown card semantics, and reviewer workload are not scope reasons. They
produce BLOCKED or unresolved according to the validation precedence.

### 7.2 Model expansion

An expanded model is a new model identity. It may review previously
out-of-scope candidates in a new additive authority artifact, but it never
silently reinterprets the old scope proof. The old record remains historical
evidence.

## 8. Review-domain applicability authority

The current C model requires exactly these eleven domains, in this order:

~~~
triggers_and_lki
replacement_layers_and_dependency
copy_and_token_creation
target_legality_protection_and_identity
control_and_ownership
commander_and_format
hidden_information_and_visibility
ordering_and_temporal_dependencies
source_versus_affected_identity
controller_owner_and_decision_actor
higher_order_interactions
~~~

### 8.1 Domain theorem

DomainProofV1 is a reusable proposition for one domain and one terminal
applicability value:

~~~
review_domain = one exact domain above
applicability = applicable | not_applicable
criterion = closed structured semantic criterion
preconditions = exact source/boundary facts
source_evidence_refs = exact resolved source facts
b2_boundary_refs = exact B2 boundaries used, if any
b1_final_citation_refs = exact B1.Final nodes used, if any
~~~

unresolved is not an accepted domain theorem. It is the deterministic derived
result when no valid accepted application covers a required candidate and
domain.

applicable proves that the domain can affect the relation review or its
required class/context. not_applicable proves that the domain is irrelevant
for this exact candidate relation. Neither value is inferred from the
candidate relation enum, participant names, capability names, or source
co-occurrence.

### 8.2 Domain application

Each DomainApplicationV1 is an exact finite member set for one
DomainProofV1. Every member contains:

~~~
candidate_id
candidate_identity
source_instance_id
candidate_universe_binding
precondition_attestations[]
member_evidence_refs[]
~~~

The application validator requires one attestation for every theorem
precondition, exact candidate/source-instance resolution, and positive
candidate/domain evidence. A batch of 10,000 members is valid only when all
10,000 members carry those exact checks. The batch itself does not establish
equivalence.

### 8.3 Empty domain membership

An empty derived review set is valid only when every current candidate has an
accepted not_applicable application for that domain. An empty list of
applicable records is not evidence that the domain is irrelevant.

## 9. Class-context authority

Context belongs to a reusable context theorem and its exact applications. It
does not belong in the relation theorem identity merely because a source
instance happened to use one context.

### 9.1 Context dimensions

The context theorem carries all ten current C dimensions:

~~~
zone
visibility
timing
temporal_order
source_affected_relation
control_ownership_relation
replacement_layer_relation
trigger_lki_relation
information_relation
decision_actor_relation
~~~

It also carries all four temporal values:

~~~
trigger_order
dependency_order
duration
replacement_order
~~~

Every slot is present. not_applicable is a semantic value, not omission.
Each not_applicable slot needs a positive source-grounded explanation.

### 9.2 Context theorem and application

ContextProofV1 identifies the exact arity, directionality, participant-role
shape, host relationship, ten context values, four temporal values, and any
required B2/B1.Final references. It excludes CandidateId and SourceInstanceId
from its semantic theorem identity.

ContextApplicationV1 binds that theorem to an exact finite member set. Every
member proves:

1. the candidate identity resolves to the current candidate ledger;
2. the source instance resolves to that candidate;
3. the participant bindings and relation direction match the theorem;
4. every context and temporal slot is supported by the member's evidence; and
5. the member is semantically equivalent to the theorem on all theorem
   preconditions.

If a source instance has a different zone, visibility, timing, temporal order,
control, owner, LKI, target, information, or decision meaning, it requires a
different context application and normally a different context theorem. A
source-instance exception is not an implicit override.

### 9.3 Class identity compatibility

The existing InteractionClassIdentityV1 contract remains unchanged in this
task. A future class may reuse that identity only when the relation proof's
normalized semantic claim is completely represented by the existing nine
class-identity positions:

~~~
arity
directionality
participant_roles
host_relationship
context_dimensions
temporal_semantics
b2_family_refs
b2_boundary_refs
b1_final_citation_refs
~~~

The future authority validator must require an explicit structured
ClassProjectionEquivalenceV1 proof, not a boolean. V1 class sharing is
strict: the relation theorem semantic ID must be identical for every member
of one class. The proof must also contain identical values for all nine class
positions and an accepted explanation that no non-provenance semantic field
remains unrepresented. Different theorem semantic IDs, including different
relation mechanisms, cannot share a V1 class even when their nine projected
positions happen to compare equal. Cross-theorem class equivalence requires a
new versioned class-identity contract.
If that proof cannot be made, the candidate remains unresolved until a
versioned amendment defines a new class identity preimage. No relation
mechanism may be silently dropped from a class identity for deduplication.

## 10. Unary and declared-card-trigger candidates

The 18 declared_card_trigger rows are ordinary candidates with a unary
relation shape. They are not grandfathered by the relation label.

A valid unary positive theorem must bind:

~~~
exact OSI/card participant
exact REV3 source row and joined source record
terminal B2 assignments and exact family boundaries
the trigger condition and event relation
LKI/intervening-if behavior where applicable
the required B1.Final citation nodes
all disposition-specific domain/context obligations
~~~

A B2 cap.trigger assignment, a DECLARED_CARD_TRIGGER source cell, or a
card name is not enough. A unary non-interaction requires a positive
separation theorem for the declared unary relation. A unary scope result
requires a model-bound scope proof. Otherwise the record remains unresolved.

## 11. Higher-order review

Higher-order support is finite and explicitly reviewed. It does not claim
arbitrary N-way Magic completeness.

Every higher-order theorem/application must include:

~~~
arity = higher_order
relation = reviewed_higher_order
finite participant count = len(participant_roles) > 2
ordered participant roles
exact ParticipantSourceRefV1 for every participant
exact review-additions record identity, when the candidate is proposed there
source evidence for every participant and causal edge
exact candidate/source-instance applications
~~~

Participant source references must use the unchanged current V1 source-kind
vocabulary: rev3_row, b2_assignment, or b2_classification. They must resolve
to the pinned REV3 or B2 source records. B1.Final is not a participant source;
its official rule nodes belong in B1FinalCitationRefV1. Unknown, orphan,
duplicated, or reordered references fail validation.
Duplicate semantic participants are rejected unless a future model version
introduces an explicit multiplicity contract. The theorem cannot use an
unbounded participant pattern.

The existing empty interaction_review_additions.v2.json does not block the
architecture; it simply yields no targeted higher-order candidates. A future
addition is a proposal input and still requires an accepted relation theorem,
exact application, domain closure, and context closure where applicable.

## 12. Exact proposed artifact and record model

### 12.1 Future artifact boundary

The future implementation should add one upstream, non-C-inventory artifact:

~~~
sources/m2_5/authorities/interaction_review_authority.v1.json
~~~

This path is proposed, not created by Task 1. It must be added to a new
versioned C input contract before it can affect C derivation.

The top-level object has exactly these fields:

~~~
schema
model_binding
source_bindings
relation_proofs
relation_applications
domain_proofs
domain_applications
context_proofs
context_applications
supersession_records
~~~

The top-level schema is:

~~~
manafold.m2.5.c.interaction-review-authority.v1
~~~

The artifact has no self-digest field. Its raw SHA-256 is bound by a future C
closure input record. It cannot cite itself as semantic evidence.

source_bindings is the exact complete set of ArtifactBindingV1 values needed
by every theorem, application, supersession, acceptance event, and model in
this artifact. It is sorted by the canonical CBOR bytes of the complete
binding tuple, duplicate-free, and contains no unused binding. It includes
acceptance-event leaves and reviewer-roster leaves for resolution, but this
root-level set is not part of any theorem, application, or supersession
identity. Adding a later record may change this root set without changing
existing leaf-event bytes or existing accepted record IDs.

### 12.2 Common binding types

ArtifactBindingV1 has exactly:

~~~
authority_kind
path
schema_or_null
raw_sha256
artifact_role
~~~

authority_kind is exactly model, rev3, b2, b1_final, c_candidate,
reviewer_roster, or acceptance_event. schema_or_null is null only for one of
these exact raw REV3 members:

~~~
derived/Pair_Interaction_Census_REV3.csv
inputs/deck_row_source_resolution_REV3.csv
source/raw/source_record_index_REV3.csv
source/raw/oracle_cards_selected_REV3.jsonl
~~~

For every other admitted path, schema_or_null is the exact logical schema
identifier listed below; null is not a substitute for a missing schema:

| Artifact role | Exact schema_or_null rule |
| --- | --- |
| declared_model | manafold.m2.5.c.declared-interaction-model.v2 |
| rev3_source with inputs/interaction_model_v1.json | interaction-model.v1 |
| b2_catalog | manafold.m2.5.b2.requirement-family-catalog.v1 |
| b2_classifications | manafold.m2.5.b2.card-semantic-classifications.v1 |
| b2_closure | manafold.m2.5.b2.classification-closure.v1 |
| b1_final_citations | manafold.m2.5.b1.official-authority-citations.v3 |
| b1_final_closure | manafold.m2.5.b1.official-authority-citation-closure.v2 |
| candidate_universe | manafold.m2.5.c.interaction-candidate-universe.v2 |
| acceptance_event_leaf | manafold.m2.5.c.review-acceptance-event.v1 |
| reviewer_roster_leaf | manafold.m2.5.c.reviewer-roster.v1 |

An unlisted path, role, or schema combination fails source-binding validation.
The raw REV3 member list is closed; a future raw member requires a versioned
authority amendment rather than a null by extension.

artifact_role is one of:

~~~
declared_model
rev3_source
b2_catalog
b2_classifications
b2_closure
b1_final_citations
b1_final_closure
candidate_universe
acceptance_event_leaf
reviewer_roster_leaf
~~~

model_binding has exactly:

~~~
path
raw_sha256
model_id
model_version
~~~

All bindings resolve to the exact current accepted artifact. A path or digest
without actual locator resolution is not sufficient.

AcceptanceV1 has exactly:

~~~
decision
review_event_ref
~~~

decision is exactly human_accepted. review_event_ref resolves to one immutable
event leaf. The event record has exactly:

~~~
event_id
schema
subject_kind
subject_payload_digest
decision
reviewer_roster_ref
reviewer_role_bindings
review_mode
checklist_id
source_binding_digests
review_evidence_refs
~~~

review_event_ref has exactly:

~~~
path = sources/m2_5/authorities/review_acceptance_events/v1/<event_digest_hex>.json
raw_sha256 = exact raw digest of this immutable leaf artifact
locator = ["event_id", event_id]
~~~

Its canonical form is ReviewEventRefV1 = [path, raw_sha256_bytes,
["event_id", event_id]]. The artifact binding and locator must both resolve
before the acceptance record is trusted.

The event schema is exactly:

~~~
manafold.m2.5.c.review-acceptance-event.v1
~~~

The immutable event leaf has no events array and no appendable aggregate
container. Its top-level fields are exactly the event fields listed above. A
new event creates a new leaf path derived from its own event digest; it cannot
change the bytes or raw digest of an existing leaf. A separate manifest may
enumerate event leaves for discovery, but no accepted record identity may
depend on that mutable manifest root.

subject_kind is exactly one of:

~~~
relation_theorem_record
domain_theorem_record
context_theorem_record
relation_application_record
domain_application_record
context_application_record
supersession_record
~~~

decision is exactly human_accepted. review_mode is exactly
multi_reviewer or solo_separate_self_review. checklist_id is exactly
interaction-authority-review-checklist.v1. reviewer_role_bindings is sorted
by reviewer_id and has exactly:

~~~
[reviewer_id, roles_sorted]
~~~

Each reviewer_id must resolve in reviewer_roster_ref. The listed roles must
equal the complete role array for that exact reviewer in the bound roster.
The union of all roles in reviewer_role_bindings is therefore deterministically
the union of roles held by the selected reviewer IDs. The event must contain the role set required for the
referenced subject kind; the role requirements are the ones stated below and
in the ownership model.

The event identity uses the exact envelope with:

~~~
semantic_domain = manafold.m2.5.c.review-acceptance-event.v1
input_schema_id = manafold.m2.5.c.review-acceptance-event-input.v1
~~~

and the fixed payload:

~~~
[schema_id, subject_kind, subject_payload_digest_bytes, decision,
reviewer_roster_ref, reviewer_role_bindings_sorted, review_mode, checklist_id,
source_binding_digests_sorted, review_evidence_refs]
~~~

SourceBindingDigestV1 has exactly:

~~~
[artifact_role, path, schema_or_null, raw_sha256_bytes]
~~~

ReviewerRoleBindingV1 has exactly [reviewer_id, roles_sorted], and
ReviewerRosterRefV1 has exactly [path, schema, raw_sha256_bytes].

artifact_role is one of declared_model, rev3_source, b2_catalog,
b2_classifications, b2_closure, b1_final_citations, b1_final_closure,
candidate_universe, or reviewer_roster_leaf. The event's
source_binding_digests is the exact sorted union defined by this closed rule:

~~~
subject source set =
  { declared model binding }
  + { every raw artifact named by a source_evidence_ref,
      B2 boundary ref, B1.Final citation ref, candidate-universe binding,
      or member_evidence_ref in the subject record }
  + { reviewer roster binding }
~~~

The subject source set is de-duplicated by the complete
SourceBindingDigestV1 tuple and sorted by its canonical CBOR bytes. A theorem
subject therefore includes the model, its proof-evidence artifacts, its B2
and B1.Final artifacts, and the roster when those are present. An application
subject additionally includes the candidate-universe artifact and every
member-evidence artifact. A supersession subject includes the model and every
source artifact named by its evidence. The acceptance-event leaf itself, the
review-authority aggregate, and any mutable discovery manifest are always
excluded. This inclusion set is recomputed and is not caller-selected.

The evidence-kind to artifact-role mapping is fixed:

| Referenced fact | Required source-binding roles |
| --- | --- |
| declared model | declared_model |
| REV3 row, archive member, or source record | rev3_source |
| B2 family boundary | b2_catalog, b2_closure |
| B2 assignment or classification | b2_classifications, b2_catalog, b2_closure |
| B1.Final citation | b1_final_citations, b1_final_closure |
| candidate or source instance | candidate_universe |
| reviewer identity/role | reviewer_roster_leaf |

The resolver adds exactly the listed roles for each referenced fact and then
de-duplicates them. It never adds a role merely because an artifact happens to
exist. AcceptanceEvidenceRefV1 values are bound separately in the event and
are not semantic source bindings.

AcceptanceEvidenceRefV1 has exactly:

~~~
[path, raw_sha256_bytes, locator]
~~~

Its path is a committed repository path or a normalized portable external
review-export locator. It is not semantic proof and cannot be a creator-local
path. Acceptance evidence references are sorted by their canonical CBOR bytes
and are part of event identity.

The accepted role vocabulary is exactly:

~~~
project_owner
architecture_maintainer
rules_authority_maintainer
information_safety_reviewer
conformance_maintainer
~~~

A relation proof, relation application, or supersession requires the
rules_authority_maintainer role. A schema or cross-artifact acceptance also
requires architecture_maintainer. A proof that asserts hidden-information or
player-visible consequences requires information_safety_reviewer. A final
evidence/negative-contract acceptance requires conformance_maintainer. The
project_owner is valid only when the bound roster assigns that role. No
event-local role override or appointment field exists. In solo mode,
review_mode = solo_separate_self_review requires the complete separate
self-review checklist; it does not waive any role or evidence requirement.

reviewer_roster_ref has exactly [path, schema, raw_sha256_bytes] and resolves
to an immutable roster leaf at:

~~~
sources/m2_5/authorities/reviewer_rosters/v1/<roster_digest_hex>.json
~~~

The roster schema is exactly manafold.m2.5.c.reviewer-roster.v1. The leaf has
exactly schema and reviewers. Each reviewer entry has exactly reviewer_id and
roles, with reviewers sorted by reviewer_id and roles sorted and
duplicate-free. The path basename is the raw SHA-256 of the exact roster leaf
bytes. The current repository has no named public roster, so a future production
acceptance is BLOCKED until a portable roster/review identity is admitted.
The event's subject_payload_digest is the digest of the exact record payload
before acceptance metadata. The accepted theorem/application record identity
includes the event_id and event raw digest, but the event never contains the
final record identity. This one-way binding prevents a digest cycle while
binding the human decision to the exact reviewed bytes.

The acceptance-event leaf is a committed, versioned source input or a portable
review export whose exact bytes are included in the accepted evidence package.
A creator-local path, mutable branch, live GitHub state, or bare
human_accepted flag is not a trust anchor.

### 12.3 RelationProofV1

Each relation theorem record has exactly:

~~~
theorem_id
record_id
proof_kind
subject
preconditions
proof_payload
b2_boundary_refs
b1_final_citation_refs
source_evidence_refs
semantic_rationale
acceptance
~~~

subject has exactly:

~~~
arity
relation
directionality
participant_roles
host_relationship
~~~

participant_roles is an ordered array of:

~~~
position
role
participant_kind
semantic_ref
~~~

It contains exact semantic participant identities, not candidate IDs or source
instance IDs. For unordered_binary, participant references use canonical order.
For directional_binary, role position and edge direction are preserved. A role
cannot be inferred from its position.

proof_payload is exactly one RelationProofPayloadV1 variant:

~~~
RelationProofPayloadV1 =
  ["positive_interaction",
   [CausalChainV1, RequiredRelationChannelsV1,
    class_projection_template_or_null]]
  | ["positive_separation",
     [separation_kind, SeparationObligationsV1]]
  | ["model_bound_scope",
     [ModelBoundaryRefV1, reason_code, observed_candidate_shape,
      positive_boundary_evidence_refs]]
~~~

preconditions is an ordered, unique array. It is a tagged union, not a
general-purpose predicate language. The JSON object has exactly the following
fields, and its canonical CBOR form is the two-element array shown below:

~~~
precondition_id
precondition_kind
payload

[precondition_id, [precondition_kind, payload]]
~~~

The allowed precondition_kind variants and their fixed payloads are:

~~~
candidate_relation_shape = [scope, relation, directionality, host_relationship]
participant_binding = [position, role, participant_kind, semantic_ref]
b2_boundary = [family_id, lifecycle, assignment_role,
               precise_semantic_definition]
source_context = [dimension, expected_value]
temporal_semantic = [dimension, expected_value]
class_projection = [arity, directionality, participant_roles,
                     host_relationship, context_dimensions,
                     temporal_semantics, b2_family_refs,
                     b2_boundary_refs, b1_final_citation_refs]
~~~

source_context.dimension is one of the ten declared context dimensions, and
temporal_semantic.dimension is one of the four declared temporal dimensions.
Their expected_value types are fixed by that dimension's existing C
vocabulary. Every variant uses exact structural equality; none supports a
runtime expression, an open field path, a regular expression, a range,
negation, or an unbounded quantifier. A theorem that needs a different
comparison requires a new versioned precondition variant.

### 12.4 RelationApplicationV1

Each relation application record has exactly:

~~~
application_id
record_id
theorem_record_id
terminal_disposition
members
acceptance
~~~

The theorem's proof_kind and the application's terminal_disposition must match
exactly. members is a finite exact membership set sorted by:

~~~
[candidate_identity.digest_bytes, UTF8(source_instance_id)]
~~~

Each member has exactly:

~~~
candidate_id
candidate_identity
source_instance_id
candidate_universe_binding
relation_binding
precondition_attestations
member_evidence_refs
member_proof_attestation
~~~

relation_binding repeats the exact candidate relation shape and ordered
participant bindings so the validator can detect direction reversal,
participant substitution, and stale candidate reuse. The validator resolves
the candidate_id and source_instance_id against the bound candidate universe;
the application is not a second source-instance ledger.

precondition_attestations has exactly one entry per theorem precondition:

~~~
precondition_id
observed_value
evidence_refs
equivalence_rationale
~~~

The validator compares observed_value to the theorem expectation using the
exact variant-specific equality defined in §12.9. Every member must pass every
precondition. A member cannot supply an exception, wildcard, prefix, or hidden
fallback. A different semantic fact requires a different theorem/application.

member_proof_attestation is required and is selected by the theorem proof kind:

~~~
["positive_interaction", [causal_chain_ordinals,
                           class_projection_equivalence_or_null]]
["positive_separation", [channel_coverages]]
["model_bound_scope", [scope_boundary_attestation]]
~~~

For positive_separation, channel_coverages contains exactly one entry for every
channel in §6.1, in the fixed channel order. Each entry must match the
theorem's required_conclusion and has exactly:

~~~
channel
coverage = separated | not_relevant
positive_boundary_facts
source_evidence_refs
b1_final_citation_refs
rationale
~~~

An application member cannot inherit channel coverage from the batch or from
another member. For positive_interaction, causal_chain_ordinals must equal
the theorem's complete ordered ordinal sequence. If a class is emitted, the
theorem must carry a non-null class_projection_template and the member's
class_projection_equivalence must be the structured value defined in §12.11.
Null is allowed only for a non-class disposition.

### 12.5 DomainProofV1 and DomainApplicationV1

DomainProofV1 has exactly:

~~~
theorem_id
record_id
review_domain
applicability
criterion
preconditions
b2_boundary_refs
b1_final_citation_refs
source_evidence_refs
semantic_rationale
acceptance
~~~

review_domain and applicability use the exact current C vocabularies.
criterion is a closed structured value that says what positive fact makes
the domain applicable or not applicable. It cannot be a keyword predicate.

DomainApplicationV1 has exactly:

~~~
application_id
record_id
theorem_record_id
review_domain
applicability
members
acceptance
~~~

Its members use the same exact subject and precondition-attestation shape as
relation applications and add exactly one DomainMemberAttestationV1 per
member. Each member has exactly:

~~~
candidate_id
candidate_identity
source_instance_id
candidate_universe_binding
domain_binding
precondition_attestations
member_evidence_refs
domain_member_attestation
~~~

domain_binding repeats review_domain and applicability. The application must
not mix domains or applicability values. A domain member without 1:1
criterion attestations in domain_member_attestation is not an application.

### 12.6 ContextProofV1 and ContextApplicationV1

ContextProofV1 has exactly:

~~~
theorem_id
record_id
subject_shape
context_dimensions
temporal_semantics
preconditions
b2_boundary_refs
b1_final_citation_refs
source_evidence_refs
semantic_rationale
acceptance
~~~

subject_shape includes the exact arity, directionality, participant roles, and
host relationship. context_dimensions and temporal_semantics use the fixed
ten- and four-slot objects already declared by C.

ContextApplicationV1 has exactly:

~~~
application_id
record_id
theorem_record_id
members
acceptance
~~~

Every member has exactly:

~~~
candidate_id
candidate_identity
source_instance_id
candidate_universe_binding
context_binding
precondition_attestations
member_evidence_refs
context_member_attestation
~~~

context_binding repeats the exact theorem subject shape. Every member has
exactly one ContextMemberAttestationV1 covering every context and temporal
slot. The list is not sufficient by itself.

### 12.7 Supersession records

SupersessionRecordV1 has exactly:

~~~
supersession_id
superseded_record_id
replacement_record_id
superseded_record_kind
replacement_record_kind
reason_code
source_evidence_refs
acceptance
~~~

The record-kind vocabulary is exactly relation_theorem_record,
relation_application_record, domain_theorem_record, domain_application_record,
context_theorem_record, or context_application_record. replacement_record_id
and replacement_record_kind are both null only for a revocation. Otherwise
both are required, and replacement_record_kind must equal
superseded_record_kind. The validator resolves both IDs and independently
checks that their prefixes and record payload kinds agree with that field. A
RelationProof record cannot be replaced by a DomainApplication record.

The allowed reason
codes are:

~~~
semantic_correction
source_revision
model_revision
authority_revocation
~~~

The old record is never edited or deleted. Current derivation selects only an
accepted record that is not the source of a valid supersession record in the
selected model/version. Historical validation keeps the entire lineage.

### 12.8 Field-level contract

The following rules apply to every authority record. They are part of the
proposed V1 schema, not implementation advice.

| Field or group | Owner | Requiredness and representation | Ordering and validation | Reuse and failure behavior |
| --- | --- | --- | --- | --- |
| schema and model binding | authority artifact | required, non-null exact ASCII identifiers and model binding | fixed top-level position in the CBOR input; exact equality to the admitted model | a mismatch blocks the authority; no synonym or migration is implicit |
| theorem_id | theorem record | required, non-null namespaced digest reference | digest derived from the theorem semantic preimage; no self-reference | same semantic theorem may be used by many applications; mutation of semantic fields fails identity validation |
| record_id | accepted record | required, non-null namespaced digest reference | digest derived from theorem identity, evidence, and acceptance data; no predecessor field is present | a changed accepted record creates a new immutable revision; in-place mutation fails |
| application_id | application set | required, non-null namespaced digest reference | digest covers theorem record, disposition/domain, and the complete member set | adding, removing, or changing a member requires a new application; no implicit expansion |
| proof_kind, disposition, domain, applicability, reason codes | semantic owner of the typed record | required exact closed enum; no null | enum variant ID is exact lowercase ASCII and is encoded as a CBOR enum pair | unknown, noncanonical, or mismatched variants fail; values are never inferred from names |
| subject shape and participant roles | relation/context theorem | required; roles are a non-empty fixed ordered array | positions are contiguous from zero; unordered participants use canonical CBOR order; directional roles preserve order | a changed participant, role, arity, or direction creates a new theorem; reverse reuse fails |
| preconditions | theorem | required ordered array; empty only for a theorem whose subject has no additional precondition | unique precondition IDs in canonical byte order; each ID appears once in each application | every application member must attest every precondition; omissions fail |
| proof payload | theorem | required exact tagged union for the proof kind | fixed variant and nested array order; no free-form predicate or unknown operation | unsupported semantics block acceptance; payload mutation changes theorem identity |
| B2 boundary refs | theorem/application evidence | required when a participant or proof claim uses a B2 family; otherwise an explicit empty array | sorted by canonical B2 boundary key, duplicate-free; exact lifecycle and boundary text | reused only when exact boundary refs match; stale or ACTIVE_UNASSIGNED card-derived refs fail |
| B1.Final citation refs | theorem/application evidence | required when the proof makes an official-rule claim; otherwise an explicit empty array | sorted by canonical citation-pair bytes, duplicate-free | reused only under the exact citation graph identity; missing required citations fail |
| source evidence refs | theorem/application provenance | required for every accepted record; non-empty for every semantic conclusion | sorted by canonical EvidenceRefV1 bytes, duplicate-free | source evidence changes create a new record ID; a valid-looking but unresolved locator fails |
| member subject binding | application | required, non-null candidate ID, CandidateIdentityV1, SourceInstanceId, and candidate-universe binding | members sorted by candidate digest bytes then SourceInstanceId bytes; no duplicates | every member is independently checked; enumerating a member without proof is invalid |
| member precondition attestations | application member | required exactly once for each theorem precondition | ordered by theorem precondition order; evidence refs canonical and duplicate-free | an unequal observed value requires another theorem/application; no exception field exists |
| member proof attestation | application member | required tagged union selected by proof kind; separation carries all channels, interaction carries the complete causal-chain binding, and scope carries the model-boundary attestation | fixed variant and canonical channel/edge order; no inherited batch-only proof | omission or cross-member reuse fails; a boolean or prose-only claim is not an attestation |
| semantic rationale and equivalence rationale | accepted record | required non-empty human-readable prose; excluded from theorem meaning | prose is not used to resolve identifiers or predicates | editing prose creates a new accepted record but cannot repair missing structured proof |
| acceptance | every accepted record | required; exactly human_accepted plus immutable review-event reference | review-event reference is provenance only and is included in record identity | absent or unresolvable acceptance means EXPERIMENTAL, never production authority |
| supersession | lineage | required only in a SupersessionRecordV1; replacement may be explicit null for revocation | one current replacement or one revocation; no cycles or competing successors | old records remain readable; current derivation rejects superseded records |

Nullability is closed. Optional semantic values use explicit null only where
this document says so: replacement_record_id and replacement_record_kind in a
revocation, optional context-dependent B2/B1 reference arrays represented as
empty arrays, and no other record field. Missing and null are not equivalent.

### 12.9 Closed precondition and evidence shapes

The future schema does not expose a generic predicate language. Every
precondition is one of these tagged, fixed-payload variants:

~~~
["candidate_relation_shape",
 [scope, relation, directionality, host_relationship]]
["participant_binding",
 [position, role, participant_kind, semantic_ref]]
["b2_boundary",
 [family_id, lifecycle, assignment_role, precise_semantic_definition]]
["source_context", [dimension, expected_value]]
["temporal_semantic", [dimension, expected_value]]
["class_projection",
 [arity, directionality, participant_roles, host_relationship,
  context_dimensions, temporal_semantics, b2_family_refs,
  b2_boundary_refs, b1_final_citation_refs]]
~~~

The tag determines the payload type, allowed fields, allowed values, and
comparison. Candidate relation shape, participant binding, B2 boundary, and
class projection use exact structural equality. Context and temporal variants
use exact equality for the named closed dimension. No variant supports an open
field path, regular expression, range, negation, evaluator, unbounded
quantifier, or language-native callback. A needed comparison requires a new
versioned tagged variant.

The source/value mapping for the two dimension variants is closed:

| Precondition tag | Allowed dimension | Exact value type | Comparison | Minimum evidence kind |
| --- | --- | --- | --- | --- |
| source_context | zone | C zone vocabulary | exact enum equality | candidate/source-instance binding plus source evidence for the slot |
| source_context | visibility | C visibility vocabulary | exact enum equality | candidate/source-instance binding plus source evidence for the slot |
| source_context | timing | C timing vocabulary | exact enum equality | candidate/source-instance binding plus source evidence for the slot |
| source_context | temporal_order | C temporal-order vocabulary | exact enum equality | candidate/source-instance binding plus source evidence for the slot |
| source_context | source_affected_relation | C source/affected vocabulary | exact enum equality | candidate/source-instance binding plus source evidence for the slot |
| source_context | control_ownership_relation | C control/ownership vocabulary | exact enum equality | candidate/source-instance binding plus source evidence for the slot |
| source_context | replacement_layer_relation | C replacement/layer vocabulary | exact enum equality | candidate/source-instance binding plus source evidence for the slot |
| source_context | trigger_lki_relation | C trigger/LKI vocabulary | exact enum equality | candidate/source-instance binding plus source evidence for the slot |
| source_context | information_relation | C information vocabulary | exact enum equality | candidate/source-instance binding plus source evidence for the slot |
| source_context | decision_actor_relation | C decision-actor vocabulary | exact enum equality | candidate/source-instance binding plus source evidence for the slot |
| temporal_semantic | trigger_order | C trigger-order vocabulary | exact enum equality | source evidence for the temporal slot and any cited rule |
| temporal_semantic | dependency_order | C dependency-order vocabulary | exact enum equality | source evidence for the temporal slot and any cited rule |
| temporal_semantic | duration | C duration vocabulary | exact enum equality | source evidence for the temporal slot and any cited rule |
| temporal_semantic | replacement_order | C replacement-order vocabulary | exact enum equality | source evidence for the temporal slot and any cited rule |

candidate_relation_shape has the fixed four-enum payload and requires the
candidate-universe binding plus its exact REV3 row. participant_binding uses
the exact role/participant tuple and requires the corresponding candidate,
REV3, or B2 locator. b2_boundary requires the exact B2 catalog boundary and,
for a card-derived assignment, the exact B2 assignment/classification
locator. class_projection requires the complete ClassProjectionV1 value and
the structured proof in §12.11. These evidence-kind rules are selected by the
tag; a record cannot override them with an open required_evidence_ref_kinds
list.

DomainProofV1.criterion is an ordered array of the following closed variants:

~~~
["channel_implicated", [channel, positive_boundary_fact]]
["channel_excluded", [channel, positive_boundary_fact]]
["rule_domain_required", [authority_id, citation_id, covered_boundary_fields]]
["rule_domain_excluded", [excluded_domain_id, positive_boundary_fact]]
~~~

The channel vocabulary is exactly:

~~~
participant_boundary
event_or_effect_causality
target_or_choice
zone_or_object_identity
control_or_ownership
replacement_or_layer
trigger_or_lki
information_or_visibility
ordering_or_temporal
decision_actor
format_and_declared_scope
~~~

Each channel appears at most once in a criterion. A domain proof must provide
positive evidence for the criterion variant; the criterion is not a selector
and never derives applicability from a candidate shape.

The following evidence and attestation shapes are fixed:

~~~
BoundaryEvidenceV1 = [b2_family_id, b2_lifecycle, precise_semantic_definition]
CitationEvidenceV1 = [authority_id, citation_id]
MemberEvidenceV1 = [authority_kind, path, locator, raw_sha256_bytes]
PreconditionAttestationV1 = [precondition_id, observed_value,
                             member_evidence_refs, equivalence_rationale]
~~~

The JSON artifact may use named fields for readability, but the validator
converts them to these fixed semantic arrays before identity or ordering. An
unrecognized clause, evidence shape, channel, or comparison is a schema
failure, not a reason to accept free text.

### 12.10 Closed nested semantic types

The nested values included in the application preimages use these exact
arrays:

~~~
CandidateUniverseBindingV1 = [path, schema, raw_sha256_bytes]

ParticipantBindingV1 = [position_u32, role_enum,
                         participant_kind_enum, semantic_ref_utf8]

RelationBindingV1 = [scope_enum, relation_enum, directionality_enum,
                     host_relationship_enum, participant_bindings]

DomainBindingV1 = [review_domain_enum, applicability_enum]

ContextBindingV1 = [arity_enum, directionality_enum, participant_roles,
                    host_relationship_enum]

PreconditionAttestationV1 = [precondition_id, observed_payload,
                             member_evidence_refs_sorted,
                             equivalence_rationale]

RelationApplicationMemberV1 = [
  candidate_id_utf8, candidate_identity_digest_reference,
  source_instance_id_utf8, candidate_universe_binding, relation_binding,
  precondition_attestations, member_evidence_refs_sorted,
  member_proof_attestation
]

DomainApplicationMemberV1 = [
  candidate_id_utf8, candidate_identity_digest_reference,
  source_instance_id_utf8, candidate_universe_binding, domain_binding,
  precondition_attestations, member_evidence_refs_sorted,
  domain_member_attestation
]

ContextApplicationMemberV1 = [
  candidate_id_utf8, candidate_identity_digest_reference,
  source_instance_id_utf8, candidate_universe_binding, context_binding,
  precondition_attestations, member_evidence_refs_sorted,
  context_member_attestation
]
~~~

candidate_identity_digest_reference is the existing six-position
DigestReferenceV1, not its JSON object projection. CandidateUniverseBindingV1
must resolve to the exact current candidate-universe path, schema, and raw
digest. ParticipantBindingV1 preserves position and role; relation bindings
use canonical participant order for unordered relations and source order for
directional relations. DomainBindingV1 is the exact domain/applicability pair.
ContextBindingV1 is the exact theorem subject shape.

The three member types are not interchangeable. Their member arrays are
sorted by the unsigned canonical-CBOR bytes of
[candidate_identity_digest_bytes, source_instance_id_utf8] and reject
duplicates. The application preimages in §13.2 use the corresponding member
type, so every nested attestation and binding is hashed after this exact
conversion.

PositiveBoundaryFactV1 is the closed union:

~~~
["b2_boundary", [family_id, lifecycle, assignment_role,
                  precise_semantic_definition]]
["rev3_locator", [path, raw_sha256_bytes, locator]]
["b2_locator", [path, raw_sha256_bytes, locator]]
["b1_citation", [authority_id, citation_id]]
["context_slot", [slot_kind, slot_name, observed_value]]
["model_boundary", [model_id, model_version, model_boundary_locator]]
~~~

PositiveBoundaryFactV1 arrays are non-empty, duplicate-free, and sorted by
canonical CBOR bytes. The tag fixes the payload type and the allowed source
authority. A fact cannot be represented by an untyped object or a rationale
string.

RelationChannelV1 is the exact enum vocabulary in §6.1.
RequiredRelationChannelsV1 is a duplicate-free array of RelationChannelV1 in
that vocabulary order. SeparationObligationV1 is exactly:

~~~
[relation_channel_enum, required_conclusion_enum]
~~~

where required_conclusion_enum is separated or not_relevant. The
SeparationObligationsV1 array contains exactly one obligation for every
RelationChannelV1, in channel order.

ModelBoundaryRefV1 is exactly:

~~~
[path, schema, raw_sha256_bytes, locator]
~~~

It must resolve to the declared interaction-model.v2 artifact and its exact
model boundary field. ScopeBoundaryAttestationV1 uses this same type.

### 12.11 Structured class-projection and separation coverage

Class projection is a structured proof. It is not a boolean field. Its exact
payload is:

~~~
ClassProjectionV1 = [
  arity, directionality, participant_roles, host_relationship,
  context_dimensions, temporal_semantics, b2_family_refs,
  b2_boundary_refs, b1_final_citation_refs
]

ClassProjectionEquivalenceV1 = [
  theorem_projection,
  member_projection,
  equal_positions,
  semantic_claim_relation,
  evidence_refs,
  rationale
]
~~~

theorem_projection and member_projection must each contain the exact nine
positions above. equal_positions must be the fixed ordered list of all nine
position names. semantic_claim_relation is exactly
same_theorem_semantic_id plus the theorem semantic digest bytes. A member
bound to a different theorem semantic ID cannot share the class unless a
future version adds a separately specified cross-theorem equivalence proof.
The evidence refs must positively establish the equality; equal decoded
values without evidence are insufficient.

For a positive-separation relation application, every member has exactly one
ChannelCoverageV1 entry for every channel in §6.1, in the declared channel
order. Each entry must match the theorem's required_conclusion and is:

~~~
ChannelCoverageV1 = [
  channel,
  coverage = separated | not_relevant,
  positive_boundary_facts,
  source_evidence_refs,
  b1_final_citation_refs,
  rationale
]
~~~

Both separated and not_relevant are positive conclusions. A not_relevant
entry proves why the channel cannot apply to this exact member; it is not an
absence marker. No channel may be omitted, duplicated, or marked unresolved.
This per-member structure is the equivalence proof for a reused separation
theorem; the membership list alone has no authority.

The remaining member attestation types are:

~~~
CausalChainOrdinalAttestationV1 = [causal_chain_ordinals]

ScopeBoundaryAttestationV1 = [
  model_id, model_version, model_boundary_ref, reason_code,
  observed_candidate_shape, positive_boundary_evidence_refs
]

CriterionAttestationV1 = [
  criterion_index, observed_criterion, evidence_refs, equivalence_rationale
]

DomainMemberAttestationV1 = [criterion_attestations]

ContextSlotAttestationV1 = [
  slot_kind, slot_name, observed_value, evidence_refs, equivalence_rationale
]

ContextMemberAttestationV1 = [slot_attestations]
~~~

causal_chain_ordinals must equal the exact array 0, 1, ..., n-1 from the
theorem's causal chain. model_boundary_ref is
[path, schema, raw_sha256_bytes, locator] and must resolve to the exact
declared model. The scope reason and observed shape must equal the theorem's
scope payload.

criterion_attestations must contain exactly one entry for every criterion
clause, in clause order, with criterion_index equal to its zero-based index.
observed_criterion must equal the corresponding tagged DomainCriterionV1
clause after canonicalization, and its evidence must prove that clause for
the exact candidate/member.

slot_attestations must contain exactly these fourteen entries, in this order:

~~~
context_dimension: zone, visibility, timing, temporal_order,
                   source_affected_relation, control_ownership_relation,
                   replacement_layer_relation, trigger_lki_relation,
                   information_relation, decision_actor_relation
temporal_semantic: trigger_order, dependency_order, duration, replacement_order
~~~

slot_name selects the exact closed value vocabulary for observed_value.
ContextSlotAttestationV1 compares observed_value to the corresponding theorem
slot by exact equality and requires positive evidence for every slot,
including not_applicable. Missing, duplicated, reordered, or cross-member
attestations fail validation.

## 13. Persistent identity and digest contract

The review authority requires stable semantic identities and separate accepted
record identities.

### 13.1 Identity layers

* theorem_id identifies semantic theorem meaning. It excludes candidate and
  source-instance identity so the same relation theorem can be applied to
  multiple exact source instances.
* record_id identifies the accepted immutable record. It binds the theorem
  identity, exact source-evidence refs, and acceptance provenance. A changed
  evidence set or acceptance event therefore creates a new record ID even when
  the semantic theorem ID remains unchanged. Lineage is not stored as a
  predecessor field; SupersessionRecordV1 is the sole lineage authority.
* application_id identifies an exact finite application set. It binds the
  theorem record, terminal disposition, every candidate identity, every source
  instance, every observed precondition value, and every member evidence set.

Changing any of the following must change the appropriate identity:

~~~
participant identity or role
relation direction or arity
proof kind or proof payload
model boundary
B2 boundary reference or lifecycle
B1.Final citation reference
candidate identity
source-instance identity
member precondition value
domain or applicability
context or temporal value
terminal disposition
member evidence binding or acceptance provenance
~~~

Changing only editorial rationale or the order of already canonicalized
evidence references does not change the theorem semantic ID, but it still
changes the accepted record_id if the exact accepted record bytes or
provenance change. The old record remains immutable.

### 13.2 Canonical codec

All theorem, record, application, and supersession identities reuse the
accepted ADR-0038 digest envelope:

~~~
algorithm_id    = sha-256
payload_codec   = mtgml.canonical-cbor.v1
envelope_id     = mtgml.digest-envelope.v1
~~~

No identity hashes arbitrary JSON, Serde output, a schema-library object, or a
language-native serialization.

The CBOR payload uses fixed-position arrays. Its exact rules are:

* schema identity is the first payload position;
* optional values are explicit null;
* enums are [variant_id, payload];
* semantic sequences preserve order;
* unordered references are sorted by the canonical CBOR bytes of their
  declared semantic key;
* duplicate unordered entries are rejected;
* digest references carry 32 digest bytes, not hex text;
* the reader re-encodes and requires byte equality.

The fixed-position preimages are:

~~~
RelationProofSemanticInputV1 = [
  schema_id, model_id, proof_kind, arity, relation, directionality,
  host_relationship, participant_roles, preconditions, proof_payload,
  b2_boundary_refs, b1_final_citation_refs
]

RelationProofRecordInputV1 = [
  schema_id, theorem_id_bytes, source_evidence_refs,
  review_event_ref, semantic_rationale
]

RelationApplicationInputV1 = [
  schema_id, theorem_record_id_bytes, terminal_disposition, members
]

RelationApplicationRecordInputV1 = [
  schema_id, application_id_bytes, review_event_ref
]

DomainProofSemanticInputV1 = [
  schema_id, model_id, review_domain, applicability, criterion,
  preconditions, b2_boundary_refs, b1_final_citation_refs
]

DomainProofRecordInputV1 = [
  schema_id, theorem_id_bytes, source_evidence_refs,
  review_event_ref, semantic_rationale
]

DomainApplicationInputV1 = [
  schema_id, theorem_record_id_bytes, review_domain, applicability, members
]

DomainApplicationRecordInputV1 = [
  schema_id, application_id_bytes, review_event_ref
]

ContextProofSemanticInputV1 = [
  schema_id, model_id, subject_shape, context_dimensions,
  temporal_semantics, preconditions, b2_boundary_refs,
  b1_final_citation_refs
]

ContextProofRecordInputV1 = [
  schema_id, theorem_id_bytes, source_evidence_refs,
  review_event_ref, semantic_rationale
]

ContextApplicationInputV1 = [
  schema_id, theorem_record_id_bytes, members
]

ContextApplicationRecordInputV1 = [
  schema_id, application_id_bytes, review_event_ref
]

SupersessionRecordInputV1 = [
  schema_id, superseded_record_id_bytes, replacement_record_id_bytes_or_null,
  superseded_record_kind, replacement_record_kind_or_null, reason_code,
  source_evidence_refs, review_event_ref
]
~~~

For every theorem/application preimage, nested arrays use the field order
defined in §§12.3–12.11. The semantic theorem or application ID is computed
first. The accepted theorem/application record ID then hashes that semantic
ID, the exact accepted source evidence, the complete immutable ReviewEventRefV1
(leaf path, event identity, locator, and raw digest), and required rationale
where the record has it. No accepted record
preimage contains a predecessor field. The application ID hashes its exact
member set and observed values. This ordering prevents self-reference and
makes source-instance identity an application property rather than a theorem
property. SupersessionRecordV1 alone links old and replacement record IDs.

No second canonical codec or ad hoc authority digest is permitted.

### 13.3 Closed identity registry

The following registry fixes every persisted identity introduced by this
architecture. The value in the prefix column is the exact lowercase ASCII
namespace followed by the complete 64-character digest; truncation and
alternate spellings are forbidden.

| Identity | Prefix | semantic_domain | input_schema_id | Fixed payload |
| --- | --- | --- | --- | --- |
| relation theorem semantic ID | rp.v1/ | manafold.m2.5.c.relation-proof.v1 | manafold.m2.5.c.relation-proof-input.v1 | RelationProofSemanticInputV1 |
| relation theorem accepted record ID | rpr.v1/ | manafold.m2.5.c.relation-proof-record.v1 | manafold.m2.5.c.relation-proof-record-input.v1 | RelationProofRecordInputV1 |
| relation application semantic ID | rpa.v1/ | manafold.m2.5.c.relation-application.v1 | manafold.m2.5.c.relation-application-input.v1 | RelationApplicationInputV1 |
| relation application accepted record ID | rpar.v1/ | manafold.m2.5.c.relation-application-record.v1 | manafold.m2.5.c.relation-application-record-input.v1 | RelationApplicationRecordInputV1 |
| relation supersession ID | rps.v1/ | manafold.m2.5.c.relation-supersession.v1 | manafold.m2.5.c.relation-supersession-input.v1 | SupersessionRecordInputV1 |
| domain theorem semantic ID | dp.v1/ | manafold.m2.5.c.domain-proof.v1 | manafold.m2.5.c.domain-proof-input.v1 | DomainProofSemanticInputV1 |
| domain theorem accepted record ID | dpr.v1/ | manafold.m2.5.c.domain-proof-record.v1 | manafold.m2.5.c.domain-proof-record-input.v1 | DomainProofRecordInputV1 |
| domain application semantic ID | dpa.v1/ | manafold.m2.5.c.domain-application.v1 | manafold.m2.5.c.domain-application-input.v1 | DomainApplicationInputV1 |
| domain application accepted record ID | dpar.v1/ | manafold.m2.5.c.domain-application-record.v1 | manafold.m2.5.c.domain-application-record-input.v1 | DomainApplicationRecordInputV1 |
| domain supersession ID | dps.v1/ | manafold.m2.5.c.domain-supersession.v1 | manafold.m2.5.c.domain-supersession-input.v1 | SupersessionRecordInputV1 |
| context theorem semantic ID | cp.v1/ | manafold.m2.5.c.context-proof.v1 | manafold.m2.5.c.context-proof-input.v1 | ContextProofSemanticInputV1 |
| context theorem accepted record ID | cpr.v1/ | manafold.m2.5.c.context-proof-record.v1 | manafold.m2.5.c.context-proof-record-input.v1 | ContextProofRecordInputV1 |
| context application semantic ID | cpa.v1/ | manafold.m2.5.c.context-application.v1 | manafold.m2.5.c.context-application-input.v1 | ContextApplicationInputV1 |
| context application accepted record ID | cpar.v1/ | manafold.m2.5.c.context-application-record.v1 | manafold.m2.5.c.context-application-record-input.v1 | ContextApplicationRecordInputV1 |
| context supersession ID | cps.v1/ | manafold.m2.5.c.context-supersession.v1 | manafold.m2.5.c.context-supersession-input.v1 | SupersessionRecordInputV1 |
| acceptance subject payload ID | asp.v1/ | manafold.m2.5.c.acceptance-subject-payload.v1 | manafold.m2.5.c.acceptance-subject-payload-input.v1 | AcceptanceSubjectPayloadV1 |
| review acceptance event ID | ae.v1/ | manafold.m2.5.c.review-acceptance-event.v1 | manafold.m2.5.c.review-acceptance-event-input.v1 | AcceptanceEventInputV1 |

The three theorem/application families therefore have separate semantic and
accepted-record domains. The supersession prefix is selected by the record
kind; its payload is otherwise shared and its schema/domain remain distinct.
Every JSON identity projection uses exactly the existing
DigestReferenceJsonV1 fields:

~~~
{
  envelope_id: "mtgml.digest-envelope.v1",
  algorithm_id: "sha-256",
  semantic_domain: <registry value>,
  payload_codec_id: "mtgml.canonical-cbor.v1",
  input_schema_id: <registry value>,
  digest_hex: <64 lowercase hexadecimal characters>
}
~~~

The CBOR form is the existing six-position DigestReferenceV1 with the digest
as 32 bytes. A registry row may not be changed in place. A new semantic
meaning or a changed preimage requires a new registry row and version.

### 13.4 Acceptance-event trust anchor

Acceptance events are stored in the future, separately bound artifact:

~~~
sources/m2_5/authorities/review_acceptance_events/v1/<event_digest_hex>.json
~~~

Each leaf's top-level object has schema
manafold.m2.5.c.review-acceptance-event.v1 and exactly the event fields listed
in §12.2. There is no appendable aggregate review_acceptance_events.v1.json
artifact. The event ID is computed from
AcceptanceEventInputV1:

~~~
[schema_id, subject_kind, subject_payload_digest_bytes, decision,
reviewer_roster_ref, reviewer_role_bindings_sorted, review_mode, checklist_id,
source_binding_digests_sorted, review_evidence_refs]
~~~

The leaf path basename is exactly the event digest rendered in lowercase
hexadecimal. Adding a second event creates a second leaf and leaves every
previous leaf path, byte sequence, raw digest, and accepted record binding
unchanged. A manifest or current-event index is discovery metadata only and
is never included in ReviewEventRefV1 or any accepted record identity.

subject_payload_digest is the AcceptanceSubjectPayloadV1 identity of the
record content before acceptance metadata. Its exact wrapper is

~~~
AcceptanceSubjectPayloadV1 = [subject_kind, subject_payload]
~~~

Its closed subject_payload variants are:

~~~
relation_theorem_record = [relation_theorem_id_bytes,
                           source_evidence_refs, semantic_rationale]
domain_theorem_record = [domain_theorem_id_bytes,
                         source_evidence_refs, semantic_rationale]
context_theorem_record = [context_theorem_id_bytes,
                          source_evidence_refs, semantic_rationale]
relation_application_record = [relation_application_id_bytes]
domain_application_record = [domain_application_id_bytes]
context_application_record = [context_application_id_bytes]
supersession_record = [superseded_record_id_bytes,
                       replacement_record_id_bytes_or_null,
                       superseded_record_kind, replacement_record_kind_or_null,
                       reason_code, source_evidence_refs]
~~~

The accepted theorem record ID includes the event ID and raw event digest;
the accepted application record ID does the same. The event never contains
the final accepted record ID, so the binding is acyclic. Validation recomputes
the subject payload from the accepted record and requires equality with the
event's subject payload digest.

An acceptance event is authorized only when every reviewer ID resolves to an
accepted maintainer-roster snapshot and every required role is present. The
minimum required role is rules_authority_maintainer for all semantic proofs.
Architecture, information-safety, and conformance roles are additionally
required when the event covers their respective surfaces, as specified in
§12.2. A project_owner may be listed only when the bound roster assigns that
role; the event cannot override the roster. The
current repository has no named public roster; therefore future production
acceptance is BLOCKED until that portable identity binding exists.

The acceptance event binds the exact semantic content, source bindings, and
review checklist. A Git commit or digest proves byte identity, not reviewer
authorization by itself. A creator-local path, mutable review URL, or a
human_accepted flag without the event and roster resolution is not an
acceptance trust anchor.

## 14. Disposition-specific authority closure matrix

The following matrix is normative for future C derivation:

| Terminal disposition | Required accepted authority | Not required unless proof depends on it | Class allowed |
| --- | --- | --- | --- |
| required_interaction | positive relation application; all eleven terminal domain applications; required B2 boundary refs; required B1.Final citations; context application with positive evidence for every emitted slot; class-projection completeness | no unrelated context theorem or irrelevant citation | yes, exactly one valid InteractionClassIdentityV1 |
| not_an_interaction_with_proof | positive separation application; all eleven terminal domain applications; complete participant and relation coverage; every channel has positively evidenced separated or not_relevant matching the theorem obligation | context application only when the separation theorem has context preconditions; B1 refs only where the separation proof makes rule claims | no |
| out_of_declared_scope_with_reason | model-bound scope application; all eleven terminal domain applications required by the existing C contract; exact model boundary and positive scope evidence | context only when scope proof depends on it; B2/B1 refs only where used by the boundary proof | no |
| unresolved | no terminal authority closure | any retained source evidence may remain as provenance | no; class and terminal disposition are null |

The current C V3 contract requires eleven terminal domain assessments for every
resolved candidate. This matrix preserves that rule. It does not require a
semantic class or class context for non-interaction or out-of-scope results.

The current C reason precedence remains:

~~~
1. insufficient_pair_relation_authority
2. insufficient_boundary_relation_evidence
3. missing_required_review_evidence
~~~

The future validator applies it after all source and authority checks. A
missing relation application takes precedence over missing domain/context
evidence. A relation theorem with an invalid concrete member binding uses the
boundary-relation reason. A fully applicable relation with an incomplete
domain/context/citation chain uses missing-review-evidence.

## 15. Source and evidence resolution rules

Every production reference resolves through this sequence:

~~~
exact artifact path
  + exact artifact schema
  + exact raw SHA-256
  + exact locator
  + actual locator resolution
~~~

### 15.1 REV3

An inherited candidate application resolves:

~~~
REV3 archive package SHA-256
archive member path
archive member SHA-256
row ordinal
all preserved source cells
candidate identity
~~~

The checker re-reads the row and rejects a valid-looking locator from another
row or archive revision.

### 15.2 B2

A B2 reference resolves the exact current catalog/classification/assignment
record. A family boundary reference includes:

~~~
family_id
lifecycle
assignment_role
precise_semantic_definition
~~~

The current catalog record must have the exact raw digest and matching
B2_SEMANTIC_BOUNDARY_V1 value. ACTIVE_UNASSIGNED cannot support a
card-derived proof. A B2 family name without its exact boundary is not a
boundary reference.

### 15.3 B1.Final

A B1.Final reference resolves the exact (authority_id, citation_id) node in
the accepted citation graph and verifies the graph artifact SHA-256. The
citation's official artifact locator must resolve to the cited rule/section
under the B1.Final contract. A URL, rule number, or live search result alone
is not accepted.

### 15.4 Candidate and source-instance binding

Every application member binds to the exact future C candidate-universe raw
digest and resolves candidate_id, CandidateIdentityV1, and SourceInstanceId
together. The validator compares the application relation binding with the
ledger's participant bindings and source context. A copied source-instance ID
from another candidate fails even if its text is valid.

### 15.5 Review acceptance references

Acceptance references use a separate closed acceptance-evidence type. They may
prove that a human accepted a record, but they cannot establish a Magic rule,
domain applicability, context value, relation, or scope conclusion. A
c_review locator remains test-only and cannot enter a production accepted
record.

## 16. Reuse and deduplication model

Reuse has two independent stages:

~~~
semantic theorem deduplication
        ↓
exact finite application membership
~~~

### 16.1 Theorem reuse

Two theorem records share one theorem ID only when their canonical semantic
preimages are identical. For an unordered relation, participant references
are canonically ordered while preserving multiplicity. For a directional
relation, ordered positions and edge direction are part of the preimage.

Theorem reuse is allowed across source instances when all theorem
preconditions are identical and each application member proves them. Theorem
reuse is not allowed across a changed zone, visibility, timing, controller,
owner, LKI, target, decision actor, or other semantic precondition.

### 16.2 Exact membership

Every application set has a complete finite member list. Its list is sorted,
unique, and bound to one theorem record. Every member has one exact
precondition attestation per theorem precondition. The validator rejects:

~~~
wildcards
prefixes
capability-name selectors
family-name selectors without exact identities
implicit "all current candidates" membership
membership without per-member precondition evidence
~~~

Adding a candidate after an application was accepted requires a new
application record or a new immutable revision. It must never silently enter
an old set because it happens to match a selector.

### 16.3 Semantic class deduplication

Class deduplication remains the current C equality relation. A class is shared
only when all nine existing class identity positions are equal after
canonicalization and the authority records contain an accepted
ClassProjectionEquivalenceV1 proof. Concrete source-instance
mappings remain in candidate classifications; they are not copied into class
definitions.

## 17. Human-review workflow

The practical solo-maintainer workflow is:

~~~
1. Freeze the exact source/head identities.
2. Materialize a review proposal from the complete candidate ledger.
3. Group proposals only by exact semantic theorem candidates.
4. Review the theorem's participant boundaries, mechanism/separation, and citations.
5. Validate the theorem before accepting it.
6. Materialize an exact finite application set.
7. Prove every member's theorem precondition with exact evidence.
8. Review and accept domain theorems/applications for all eleven domains.
9. Review and accept context theorems/applications where the matrix requires them.
10. Record the immutable acceptance event.
11. Derive C classifications and classes deterministically.
12. Run the positive controls and every adversarial mutation.
13. Produce the source/evidence snapshot and stop at its declared gate.
~~~

Proposal tooling may search, group, or display candidates. It may not write an
accepted semantic result, invoke an LLM as authority, or turn a model score
into a proof. Human review must inspect the exact semantic boundaries and the
per-member precondition evidence.

## 18. Deterministic derivation into C

The future C pipeline must use this order:

~~~
accepted upstream source identities
        ↓
accepted theorem/application authority
        ↓
candidate/source-instance join
        ↓
disposition-specific closure matrix
        ↓
candidate classification
        ↓
semantic-class deduplication
        ↓
closure metrics and fail-closed gate
~~~

For each candidate in the existing deterministic C candidate order, the
deriver:

1. resolves the exact candidate identity and source instance;
2. finds one and only one applicable accepted relation application, or records
   the relation authority gap;
3. resolves all eleven domain applications and records one terminal result per
   domain, or records the domain gap;
4. resolves context only when required by the matrix;
5. checks B2 boundary and B1.Final citation closure;
6. derives the terminal disposition, class ID, or unresolved state;
7. recomputes all aggregate metrics from the resulting records.

The checker verifies this derivation. It does not invent relation facts,
domain values, context values, citations, or candidate membership.

### 18.1 Future C versioning requirement

The current C V3 classification and closure schemas cannot consume this
authority without a versioned amendment because:

* the current C input graph binds exactly five semantic inputs;
* the current classification grammar has no authority-application reference;
* the current production evidence equality rule accepts only the existing
  source-derived evidence set; and
* the current checker intentionally admits no Pair/Relation, domain, or
  class-context authority path.

The future amendment must add, in a new C version, an exact raw binding for the
review-authority artifact and an exact candidate-level reference to the
relation/domain/context application records used for derivation. It must not
reinterpret C V3. CandidateIdentityV1 and InteractionClassIdentityV1 may
remain unchanged when §9.3's class-projection condition passes. The new C
classification identity/version must be used if the record grammar changes.

## 19. Validation precedence and status

The future authority validator uses this deterministic precedence:

1. **Transport and shape:** UTF-8, duplicate keys, closed objects, canonical
   JSON, required fields, scalar bounds, and array shape.
2. **Version and model identity:** exact authority schema, model ID/version,
   and supported semantic vocabulary.
3. **Raw source identity:** exact artifact path, schema, raw digest, and
   prerequisite package identity.
4. **Locator resolution:** actual REV3, B2, B1.Final, candidate, source-
   instance, and acceptance locator resolution.
5. **Acceptance lifecycle:** human-acceptance provenance, no unaccepted
   production records, valid supersession graph, and no current use of a
   superseded/revoked record.
6. **Theorem identity:** canonical semantic preimage and theorem digest.
7. **Application subject binding:** exact candidate identity, source instance,
   candidate-universe binding, and finite member uniqueness.
8. **Relation binding:** arity, participant count, role positions, canonical
   unordered order, directional order, host relation, and edge orientation.
9. **Evidence completeness:** B2 boundaries, B1.Final citations, proof
   evidence, and per-member precondition evidence.
10. **Domain authority:** exact eleven-domain coverage, applicability values,
    positive evidence, and no domain reuse across a non-equivalent member.
11. **Context authority:** all context/temporal slots, positive
    not_applicable evidence, source-instance context comparison, and class
    projection completeness.
12. **Proof/disposition consistency:** proof kind maps to the declared
    terminal disposition; non-interaction has positive separation; scope has a
    valid model-bound reason; required interaction has a complete causal proof.
13. **Identity recomputation:** application, record, theorem, and any derived
    class identity.
14. **Coverage and aggregates:** every candidate exactly once, every required
    application member accounted for, no orphan or duplicate mappings, and
    metrics partition the full candidate set.
15. **C integration:** exact C input bindings, gate vocabulary, historical
    lineage, negative-contract inventory, and downstream blocked flags.

The status meanings are:

| Status | Meaning | Terminal C result? |
| --- | --- | --- |
| PASS | The checked artifact or positive control satisfied its contract | only as an executed gate result |
| FAIL | Present data is malformed, contradictory, noncanonical, or incorrectly bound | no |
| BLOCKED | A required external authority, source, tool, or approved version is unavailable | no |
| NOT_RUN | The required check was not executed | no |
| EXPERIMENTAL | Proposal or test-only material not admitted to production authority | no |
| UNRESOLVED | Candidate-level semantic conclusion is not defensible yet | no; C remains blocked |
| SUPERSEDED | Historical record has an accepted replacement or revocation | no for current derivation |

An unavailable source authority is BLOCKED; an authority file containing a bad
binding is FAIL; a validly represented candidate lacking an accepted proof is
UNRESOLVED. A mutation that changes a present raw digest, path, or locator is
therefore FAIL, even when the resulting value is syntactically valid.
BLOCKED is reserved for missing or unavailable external bytes, authorities, or
approved versions. These statuses must not be collapsed.

## 20. Complete adversarial mutation matrix

The future implementation must use valid controls. Each negative test first
passes the same production validator used for the unmutated control, then
applies exactly one mutation. The harness must not manufacture the expected
error, skip a missing natural precondition, or accept an earlier unrelated
failure.

| ID | Isolated mutation | Expected status | Primary reason code |
| --- | --- | --- | --- |
| IRA-001 | Wrong candidate ID in a relation application member | FAIL | CANDIDATE_BINDING_MISMATCH |
| IRA-002 | Wrong CandidateIdentityV1 digest for an otherwise valid candidate | FAIL | CANDIDATE_IDENTITY_MISMATCH |
| IRA-003 | Wrong source-instance ID | FAIL | SOURCE_INSTANCE_BINDING_MISMATCH |
| IRA-004 | Relation application points to a different candidate-universe raw digest | FAIL | SOURCE_ARTIFACT_DIGEST_MISMATCH |
| IRA-005 | Reverse a directional relation | FAIL | DIRECTION_REVERSED |
| IRA-006 | Remove direction from a directional relation | FAIL | DIRECTIONALITY_LOST |
| IRA-007 | Reorder an unordered relation away from canonical order | FAIL | NONCANONICAL_ORDER |
| IRA-008 | Substitute one participant | FAIL | PARTICIPANT_BINDING_MISMATCH |
| IRA-009 | Remove a participant role | FAIL | PARTICIPANT_ROLE_MISSING |
| IRA-010 | Bind the wrong B2 family | FAIL | FAMILY_UNKNOWN |
| IRA-011 | Bind the wrong B2 assignment ordinal or boundary | FAIL | ASSIGNMENT_BINDING_INVALID |
| IRA-012 | Use ACTIVE_UNASSIGNED as card-derived evidence | FAIL | ACTIVE_UNASSIGNED_CARD_DERIVED |
| IRA-013 | Bind a different REV3 row with a valid-looking ordinal | FAIL | REV3_SOURCE_BINDING_MISMATCH |
| IRA-014 | Change a source artifact SHA while retaining a valid path | FAIL | SOURCE_ARTIFACT_DIGEST_MISMATCH |
| IRA-015 | Use a valid-looking nonexistent locator | FAIL | LOCATOR_UNRESOLVED |
| IRA-016 | Bind an unknown B1.Final citation | FAIL | B1_CITATION_UNRESOLVED |
| IRA-017 | Bind an accepted citation from the wrong B1.Final graph revision | FAIL | B1_FINAL_GRAPH_DIGEST_MISMATCH |
| IRA-018 | Emit required interaction without a positive causal proof | FAIL | POSITIVE_RELATION_PROOF_MISSING |
| IRA-019 | Omit one causal-chain edge | FAIL | POSITIVE_RELATION_PROOF_INCOMPLETE |
| IRA-020 | Use a mechanism operation outside the closed vocabulary | FAIL | PROOF_OPERATION_UNKNOWN |
| IRA-021 | Omit a required B1.Final citation from a causal edge | FAIL | B1_CITATION_UNRESOLVED |
| IRA-022 | Mark an applicable domain without positive domain evidence | FAIL | DOMAIN_EVIDENCE_MISSING |
| IRA-023 | Mark a not-applicable domain without positive separation evidence | FAIL | DOMAIN_EVIDENCE_MISSING |
| IRA-024 | Omit one of the eleven domain applications | FAIL | DOMAIN_COVERAGE_INCOMPLETE |
| IRA-025 | Apply a domain theorem to another candidate without member attestation | FAIL | DOMAIN_MEMBER_PRECONDITION_MISSING |
| IRA-026 | Use one unresolved domain in a resolved candidate | FAIL | DOMAIN_STATE_MISMATCH |
| IRA-027 | Derive non-interaction only from generic B2 family evidence | FAIL | NONINTERACTION_PROOF_MISSING |
| IRA-028 | Derive non-interaction only from lexical disjointness | FAIL | NONINTERACTION_PROOF_MISSING |
| IRA-029 | Derive non-interaction from absence of interaction evidence | FAIL | NONINTERACTION_PROOF_MISSING |
| IRA-030 | Leave one separation channel unresolved | FAIL | NONINTERACTION_PROOF_INCOMPLETE |
| IRA-031 | Omit one participant from separation coverage | FAIL | NONINTERACTION_PARTICIPANT_COVERAGE_INCOMPLETE |
| IRA-032 | Emit out-of-scope without a model-bound reason | FAIL | SCOPE_PROOF_MISSING |
| IRA-033 | Use a model version different from the declared C model | FAIL | MODEL_IDENTITY_MISMATCH |
| IRA-034 | Use a scope reason that does not apply to the candidate shape | FAIL | SCOPE_REASON_INAPPLICABLE |
| IRA-035 | Reuse a theorem for a non-equivalent candidate member | FAIL | THEOREM_PRECONDITION_MISMATCH |
| IRA-036 | Replace exact membership with a wildcard selector | FAIL | WILDCARD_MEMBERSHIP_FORBIDDEN |
| IRA-037 | Replace exact membership with a prefix selector | FAIL | PREFIX_MEMBERSHIP_FORBIDDEN |
| IRA-038 | Add a candidate to an accepted application without a new revision | FAIL | APPLICATION_MEMBERSHIP_MUTATED |
| IRA-039 | Reuse a directional theorem in the reverse direction | FAIL | DIRECTION_REVERSED |
| IRA-040 | Copy context from an unrelated class/member | FAIL | CONTEXT_MEMBER_BINDING_MISMATCH |
| IRA-041 | Emit an all-not_applicable context vector without slot evidence | FAIL | CLASS_CONTEXT_EVIDENCE_MISSING |
| IRA-042 | Change one context slot to a structurally valid but unsupported value | FAIL | CLASS_CONTEXT_EVIDENCE_MISSING |
| IRA-043 | Use a blanket not_applicable domain vector | FAIL | DOMAIN_EVIDENCE_MISSING |
| IRA-044 | Use generic candidate evidence as domain-specific proof | FAIL | DOMAIN_MEMBER_PRECONDITION_MISSING |
| IRA-045 | Mutate a semantic theorem field without changing its theorem digest | FAIL | THEOREM_IDENTITY_MISMATCH |
| IRA-046 | Mutate an application member or precondition without changing its application digest | FAIL | APPLICATION_IDENTITY_MISMATCH |
| IRA-047 | Use noncanonical participant/evidence ordering | FAIL | NONCANONICAL_ORDER |
| IRA-048 | Duplicate an evidence reference | FAIL | DUPLICATE_EVIDENCE_REF |
| IRA-049 | Duplicate a participant in a finite higher-order set | FAIL | DUPLICATE_PARTICIPANT |
| IRA-050 | Mark a proposal accepted without human acceptance provenance | FAIL | ACCEPTANCE_PROVENANCE_MISSING |
| IRA-051 | Mutate an accepted record in place | FAIL | IMMUTABLE_RECORD_MUTATED |
| IRA-052 | Create a supersession edge to an unknown record | FAIL | SUPERSESSION_TARGET_UNKNOWN |
| IRA-053 | Use a superseded record for current derivation | FAIL | SUPERSEDED_AUTHORITY_USED |
| IRA-054 | Create two current replacements for one record | FAIL | SUPERSESSION_GRAPH_AMBIGUOUS |
| IRA-055 | Emit a higher-order record with no finite participant set | FAIL | HIGHER_ORDER_PARTICIPANT_MISSING |
| IRA-056 | Orphan a ParticipantSourceRefV1 | FAIL | PARTICIPANT_SOURCE_UNRESOLVED |
| IRA-057 | Promote a card-trigger row solely from its relation label | FAIL | POSITIVE_RELATION_PROOF_MISSING |
| IRA-058 | Add a new C authority reference without a versioned C schema/input binding | FAIL | C_AUTHORITY_INPUT_UNBOUND |
| IRA-059 | Remove the externally required acceptance-event leaf | BLOCKED | ACCEPTANCE_ARTIFACT_UNAVAILABLE |
| IRA-060 | Replace an immutable acceptance-event leaf while retaining its event ID | FAIL | ACCEPTANCE_LEAF_MUTATED |
| IRA-061 | Add an appendable aggregate acceptance-event container | FAIL | ACCEPTANCE_CONTAINER_FORBIDDEN |
| IRA-062 | Bind a reviewer to a role not held in the bound roster | FAIL | ACCEPTANCE_REVIEWER_ROLE_MISMATCH |
| IRA-063 | Make the reviewer-role union differ from the exact roster-derived union | FAIL | ACCEPTANCE_REVIEWER_ROLE_MISMATCH |
| IRA-064 | Omit or add one required SourceBindingDigestV1 entry | FAIL | ACCEPTANCE_SOURCE_BINDING_SET_INVALID |
| IRA-065 | Include the acceptance-event leaf in its own source-binding set | FAIL | ACCEPTANCE_SOURCE_BINDING_SELF_REFERENCE |
| IRA-066 | Change the acceptance subject payload without changing its event digest | FAIL | ACCEPTANCE_SUBJECT_DIGEST_MISMATCH |

Positive controls are also mandatory:

~~~
IRA-POS-001  one accepted relation theorem with one exact application member
IRA-POS-002  one theorem reused by multiple exact members, each with attestations
IRA-POS-003  one fully separated non-interaction with all required channels
IRA-POS-004  one scope proof matching an exact declared model boundary
IRA-POS-005  all-eleven-domain not_applicable membership with positive evidence
IRA-POS-006  one required-interaction context profile with evidence for every slot
IRA-POS-007  finite higher-order application with exact ordered participants
IRA-POS-008  unary card-trigger application with exact OSI/source binding
IRA-POS-009  immutable supersession from one accepted record to one replacement
IRA-POS-010  two independent acceptance-event leaves with the first record binding unchanged
~~~

Every positive control must pass before its single mutation is applied. A
positive control is test input only and never a committed semantic authority.

## 21. Historical, versioning, and supersession policy

* Authority schema versions are immutable. A semantic change requires a new
  schema/model or an explicitly versioned field contract.
* Accepted theorem and application bytes are immutable. Corrections append a
  new record and a supersession record.
* A superseded record remains readable and verifiable as historical evidence;
  it is excluded from current derivation through the explicit supersession
  graph.
* A source-artifact change creates a new authority record or blocks the
  current one. It never updates a raw SHA in place.
* A B2 or B1.Final revision invalidates any application whose exact upstream
  binding no longer resolves. A future review must create new applications
  against the new upstream identity.
* A model expansion does not reinterpret old out-of-scope records.
* C V3 and its accepted blocked evidence remain immutable. A future authority
  implementation is an additive descendant with a new C integration version.
* The authority artifact must not include the private corrected-V2 migration
  source or the forbidden historical V2 monolith as a new semantic authority.
* Raw reports, LLM transcripts, reviewer notes, and proposals remain
  provenance or review material unless an accepted typed record binds them;
  none can override the proof schema.

## 22. Maintainer ergonomics and expected review scaling

The current 15,679 candidates make repeated prose impractical. The design
reduces repetition without weakening proof:

* One theorem can cover a repeated semantic relation across many source
  instances.
* Every member still carries a compact structured precondition attestation and
  exact evidence references.
* Context proofs are split from relation proofs, so a changed source context
  does not force a new relation theorem.
* Domain proofs are reusable, but their applications remain exact and
  candidate/domain-specific.
* A theorem can be reviewed once; member applicability is reviewed as a finite
  auditable set rather than as an implicit selector.
* Derived reports can show batches and summaries, but the authority artifact
  remains the source of membership.

The theoretical maximum for current C domain coverage is 172,469
candidate/domain pairs (15,679 × 11). The design does not require 172,469
independent paragraphs, but it does require 172,469 exact terminal member
conclusions before a C PASS. A missing member is a real coverage gap.

Review planning may prioritize high-risk domains or candidate groups, but
priority does not change closure requirements and does not authorize selecting
only a convenient subset.

## 23. Security and information safety

The authority system is offline review metadata, not runtime player state. It
must still preserve Manafold's information-safety boundaries:

* Production authority records may reference trusted source identities but
  must not expose runtime GameObjectId, RNG state, hidden order, or private
  player knowledge to player endpoints.
* Review tooling may read a complete trusted source archive only within its
  authorized maintainer boundary. It must not export hidden game state into ML
  observations or public trajectories.
* Candidate/source-instance IDs are review identities, not player-visible
  action labels.
* LLM or heuristic output is proposal material and must not cross the
  acceptance boundary without a human-reviewed structured proof.
* Exact digests prevent silent source substitution but do not provide
  authenticity; repository and review-process integrity remain separate
  controls.
* Rejected or unresolved authority records must not be used to generate
  training labels, ranking features, or support claims.
* A malicious member list cannot broaden authority because every member is
  resolved against the exact candidate/source ledger and every theorem
  precondition is checked.
* Canonical decoding must enforce resource limits before allocating large
  arrays or strings, reusing the accepted persistence-codec limits where the
  authority implementation uses canonical CBOR.

## 24. Impact analysis

### C artifacts

No current C artifact changes in Task 1. A future C version must bind the
authority artifact and application references explicitly. The current blocked
snapshot remains a valid historical fail-closed result.

### C checker

The future checker verifies the authority graph and derives classifications. It
must not contain relation rules, domain heuristics, class defaults, or hidden
membership logic. The current empty authority-path constants remain correct
until a new implementation contract is accepted.

### B1 and B1.Final

Unchanged. B1.Final citations remain the official rule graph. The new authority
may reference accepted nodes but cannot create, replace, or reinterpret them.

### B2

Unchanged. B2 family boundaries and assignments remain card-side authority.
They support relation proofs but do not become relation truth. ACTIVE_UNASSIGNED
remains unusable for card-derived proof.

### REV3

Unchanged and immutable. Candidate/source identity, row values, and historical
labels remain preserved. The new authority binds them; it does not rewrite
them.

### Capability model

The review authority is not a capability registry, runtime opcode, semantic
domain dispatch key, or implementation path. Capability identities and
lifecycles remain governed by docs/cards/CAPABILITY_MODEL.md.

### Certification

An accepted review proof is not capability implementation, coverage, or bundle
certification. C closure may remain a prerequisite for later work, but it
cannot certify a card, rule, or bundle by itself.

### Future M3

The authority records semantic review evidence and does not implement Magic
rules. M3 must still implement typed reusable semantics, exact transitions,
information safety, and conformance. A review theorem cannot be imported as a
runtime legality shortcut.

### Future ML datasets

Only terminal, provenance-complete, accepted C outputs may enter a declared
dataset pipeline. Unresolved candidates, proposals, review notes, and
authority gaps remain explicitly excluded or quarantined. Review IDs are not
player action labels, and the authority artifact does not cross the player
information boundary.

### Downstream gates

The following remain unchanged and blocked:

~~~
REV2_REUSE_RATIO_REPRODUCIBLE = BLOCKED
RANKING_UNCERTAINTY_PROPAGATION = BLOCKED
DECK_PAIR_LOCKED = false
AUTHORITATIVE_RANKING_AVAILABLE = false
M3_STARTED = false
~~~

## 25. Minimal future implementation decomposition

The later implementation should be split into these independently reviewable
steps:

1. **Authority contract amendment.** Register the new authority schema, exact
   CBOR preimages, acceptance evidence type, source bindings, and immutable
   supersession rules.
2. **C integration amendment.** Add the authority raw binding and exact
   application references to a new C classification/closure version. Preserve
   current C V3 meaning and identity contracts.
3. **Source/evidence resolver.** Implement exact REV3, B2, B1.Final, candidate,
   source-instance, and acceptance locator resolution.
4. **Theorem validator.** Validate semantic theorem shape, proof payload,
   boundaries, citations, canonical identity, and acceptance.
5. **Application validator.** Validate exact finite membership and every
   member precondition, with no wildcard or implicit expansion.
6. **Domain/context validator.** Validate eleven-domain coverage and
   disposition-specific context closure.
7. **Deterministic C derivation.** Produce classifications/classes from the
   accepted authority graph without semantic logic in the checker.
8. **Adversarial suite.** Implement the matrix in §20 with valid controls and
   isolated mutations.
9. **Historical/evidence closure.** Add exact-head, raw-digest, deterministic
   rerun, review export, and supersession-lineage evidence.

No step may begin until the preceding contract and review gate is accepted.
This decomposition does not authorize any of those implementation steps in
Task 1.

## 26. Exact acceptance gates for a future implementation

A future implementation may claim a C PASS only when all of these gates have
actually executed successfully on one exact source identity:

1. **Exact source/head gate:** approved origin/master base, REV3 archive and
   member digests, B2/B1.Final identities, and current candidate ledger all
   resolve exactly.
2. **Authority-schema gate:** canonical shape, closed vocabularies, CBOR
   preimages, digest recomputation, duplicate/order checks, and resource
   bounds pass.
3. **Acceptance gate:** every used theorem/application has an immutable
   acceptance-event leaf; its event identity, subject payload, leaf path/raw
   digest, reviewer-roster binding, reviewer-to-role mapping, required role
   union, and exact SourceBindingDigestV1 set recompute successfully.
   Supersession graph is unambiguous; no proposal, mutable aggregate event
   container, or superseded record is used.
4. **Relation coverage gate:** every current candidate has exactly one valid
   relation application or remains explicitly unresolved; all resolved
   relation applications pass participant, direction, and per-member
   precondition checks.
5. **Domain gate:** every candidate has exactly eleven domain assessments;
   every resolved assessment is positively evidenced; no unresolved domain is
   paired with a resolved candidate.
6. **Context gate:** every required-interaction class has complete positive
   context/temporal evidence and a valid exact member application; no blanket
   not_applicable vector is accepted.
7. **Disposition gate:** the matrix in §14 passes exactly; non-interaction is
   positive separation, scope is model-bound, and required interaction has a
   complete causal proof.
8. **Class identity gate:** all class IDs recompute under the unchanged
   InteractionClassIdentityV1 contract, or a separately accepted versioned
   class-identity amendment is used. No semantic mechanism is omitted.
9. **Candidate coverage gate:** all 15,679 candidates are present exactly
   once; every source instance and every authority application member is
   accounted for; aggregate metrics recompute.
10. **Adversarial gate:** all fixed C mutations plus the authority matrix in
    §20 execute against valid controls with their exact expected statuses and
    reason codes.
11. **Determinism gate:** two clean derivations produce byte-identical
    authority-derived artifacts, identities, classifications, class order,
    and closure metrics.
12. **Historical gate:** accepted records are immutable; supersession and
    source revisions are validated without reinterpreting historical bytes.
13. **C integration gate:** the new C version binds exactly the authority
    artifact and application records, preserves current downstream flags, and
    does not promote ranking, deck lock, or M3.
14. **Repository gate:** applicable documentation, schema, Python, Rust,
    conformance, and reproducibility checks execute on the exact head. Any
    unavailable check is NOT_RUN or BLOCKED, never PASS.
15. **Evidence gate:** H_exec/H_evidence and external review evidence follow
    the accepted C evidence protocol for the new version. The current V4
    blocked evidence is not rewritten.

The C PASS result additionally requires:

~~~
unresolved = 0
resolved_required_interaction
 + resolved_not_an_interaction_with_proof
 + resolved_out_of_declared_scope_with_reason
 = 15,679
~~~

If any authority, member, domain, context, source, or acceptance gate is
unknown, unavailable, contradictory, or unexecuted, the result is not C PASS.

## 27. Explicit non-goals

This specification does not:

* implement production code or a checker;
* create or modify the new authority artifact;
* modify current C, B1, B1.Final, B2, or REV3 artifacts;
* classify any of the 15,679 candidates;
* emit interaction classes;
* promote DECLARED_INTERACTION_MODEL_CLOSURE;
* select a relation by capability name, keyword, score, or co-occurrence;
* reconstruct or replace the REV2 ranking formula;
* compute ranking, reuse ratio, or deck lock;
* begin M3, add Magic rules, add Card IR, or add cards;
* claim arbitrary unbounded N-way completeness;
* use an LLM, heuristic, random choice, or native executor as semantic
  authority;
* make an enumerated membership list a substitute for per-member proof;
* expose trusted source/review metadata through player endpoints or ML
  observations; or
* alter the existing untracked plan in the working tree.

## 28. Open questions and bounded future decisions

The architecture is implementable without changing the conclusions above.
The following bounded prerequisites remain before implementation:

1. A portable accepted maintainer-roster snapshot and review identity must be
   admitted. The acceptance-event path, schema, identity, role vocabulary, and
   acyclic subject binding are fixed in §§12.2 and 13.3; the current repository
   has no named roster, so production acceptance remains BLOCKED until one is
   supplied.
2. The first accepted relation theorem must demonstrate whether the existing
   nine-position InteractionClassIdentityV1 projection is complete. If not,
   the C/class identity amendment must be designed before any class is emitted.
3. If a legitimate future higher-order proof needs repeated semantic
   participants, the model must add an explicit multiplicity contract before
   accepting such a record.
4. The future C amendment must decide whether authority application references
   are added directly to a new classification record or to a separate derived
   sidecar. Either option must bind them exactly and must not alter C V3.

None of these questions authorizes a heuristic default. An unresolved answer
blocks the affected implementation surface.

## 29. Recommendation for a later implementation task

~~~
GO_WITH_CHANGES
~~~

The architecture is ready for a separately reviewed implementation cycle after
the four required changes are captured here. The implementation must first
land the versioned authority schema and C integration amendment, including
fixed digest preimages, acceptance-event provenance, and the explicit class-
projection completeness gate. It must not implement directly against the
current C V3 contract, and it must not begin candidate resolution until those
amendments are independently reviewed.

## Appendix A — Current authority conclusion in one table

| Question | Answer |
| --- | --- |
| Does REV3 candidate existence prove interaction? | No |
| Does B2 family co-occurrence prove interaction? | No |
| Does B2 family-boundary disjointness alone prove non-interaction? | No |
| Does B1.Final citation presence prove pair relation? | No |
| Does Pair/Relation Authority alone close C? | No |
| Can an accepted theorem be reused? | Yes, only through exact member applications with per-member precondition proof |
| Must SourceInstanceId be in a reusable theorem identity? | No; it belongs in the application identity |
| Must required interaction have class context? | Yes, when emitting a class; every value needs positive evidence |
| Must non-interaction have a class? | No |
| May unresolved remain? | Yes; it is the correct fail-closed result |
| Is the current C snapshot changed by this spec? | No |
