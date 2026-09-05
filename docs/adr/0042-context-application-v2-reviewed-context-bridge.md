# ADR 0042: ContextApplicationV2 Reviewed-Context Bridge

- **Status:** accepted
- **Date:** 2026-09-05
- **Supersedes:** none
- **Superseded by:** none
- **Review provenance:** independently reviewed ContextApplicationV2 freeze candidate, candidate SHA-256 `7771008d06d3d120a8880454ed7fdf89b4c0b545ec7ca9ba858b6c1a93f76637`, accepted after final adversarial review found `0 BLOCKER / 0 MAJOR`; permanent number allocated at acceptance according to `docs/adr/README.md`
- **Implementation evidence:** `NOT_RUN`; this ADR freezes the implementation direction only and does not establish an executable Frozen Contract
- **Reviewed baseline:** `faf63a2711a3a8f682b277195f99dbb204be3f64`

This document is a design and contract decision only. It does not authorize
implementation, human acceptance, production Authority records, Acceptance
Events, C changes, Task 5 Slice 3B, or M3.

This revision uses the attached `Manafold_ContextApplicationV2_ADR_Candidate.md`
as its source document and closes the remaining contract-plumbing findings.
All new semantic identities use the accepted `mtgml.digest-envelope.v1`
contract; raw artifact bindings continue to use exact raw-file SHA-256 values.

## 1. Decision

The accepted architecture is preserved:

- `CandidateIdentityV1` is unchanged.
- `SourceInstanceV1` and `SourceContextV1` are unchanged.
- `ContextProofV1` and `ContextProofRecordV1` are unchanged.
- `ContextApplicationV1` remains historically frozen.
- `ClassProjectionV1` is unchanged.
- `ContextProofV2` is not introduced.
- `ContextApplicationV2` is an additive, explicitly versioned bridge from the
  immutable historical SourceContext claim to a separately reviewed semantic
  context.

The bridge never rewrites, normalizes, or reinterprets `SourceContextV1`.
Every context-slot mismatch is explicit as `reviewed_divergence`; it is not a
record supersession operation.

The V2 bridge is valid only for an exact finite member set and must remain
non-authoritative until its own V2 record and V3 review event have been
accepted under the closed lifecycle below.

## 2. Problem and ownership boundary

The historical SourceInstance ledger may contain `not_applicable` for one or
more of its ten context dimensions. Existing V1 validation correctly requires
every V1 `source_context` precondition to equal that historical value.

Reviewed semantic context may, after independent positive review, describe a
different value for an application. That fact belongs only to the V2 bridge:

~~~text
SourceContextV1 (historical source fact)
        |
        | exact source binding plus explicit comparison
        v
ContextApplicationV2 bridge attestation
        |
        | independent reviewed evidence
        v
reviewed semantic context
~~~

`ContextProofV1` remains the reusable theorem contract. Its `source_context`
preconditions continue to bind to the historical `SourceInstanceV1` value,
never to `reviewed_value`.

The bridge establishes only structural/evidential context binding. It does not
decide relation truth, domain applicability, candidate classification, ranking,
deck selection, Magic rules, or terminal C disposition.

## 3. Unchanged V1 precondition semantics

For every V1 `source_context` precondition:

~~~text
ContextProofV1 precondition
        == exact historical SourceInstanceV1.source_context value
~~~

The following are invalid:

- comparing a V1 precondition to `ContextApplicationV2.reviewed_value`;
- replacing `not_applicable` with a reviewed value in a V1 member;
- treating reviewed divergence as a SourceInstance correction;
- silently normalizing or falling back between the two values.

V2 may carry the same V1 precondition attestations required by the existing
authority contract, but the V1 validator must retain its historical equality
check unchanged.

## 4. ContextApplicationV2 semantic model

### 4.1 Closed enums

`ContextBridgeRelationV2` is the closed enum:

~~~text
exact_match
reviewed_divergence
~~~

`unresolved` is not a materialized value. A missing or undecided value is a
missing bridge attestation and fails validation.

### 4.2 Context slot attestation

There are exactly ten context-slot attestations, in the canonical order of the
existing `ContextBindingV1` vocabulary:

~~~text
ContextSlotBridgeAttestationV2 = [
    slot_name,
    source_value,
    reviewed_value,
    relation,
    evidence_refs_sorted,
    rationale
]
~~~

`slot_name` and the allowed value vocabulary are closed by the existing V1
contract. No new slot is introduced here.

Required invariants:

- each of the ten slots occurs exactly once;
- slot order is the accepted canonical order;
- `source_value` equals the exact value in the bound
  `SourceInstanceV1.source_context`;
- `source_value == reviewed_value` implies `relation=exact_match`;
- `source_value != reviewed_value` implies
  `relation=reviewed_divergence`;
- `evidence_refs_sorted` is non-empty for every context slot, including
  `exact_match` slots whose value is `not_applicable`;
- every context slot requires positive, source-bound evidence for its reviewed
  semantic value; equality alone is not semantic review evidence;
- `reviewed_divergence` additionally requires explicit positive evidence for
  the reviewed value and an exact binding of the historical source value;
- `exact_match` does not permit a source/review mismatch and does not waive
  the positive reviewed-value evidence requirement;
- no evidence absence, lexical match, normalization, or fallback can satisfy
  a slot;
- `rationale` explains the reviewed bridge fact but is not semantic authority
  by itself.

### 4.3 Temporal slot attestation

There are exactly four temporal attestations, in the accepted
`temporal_value_vocabulary` order:

~~~text
TemporalSlotAttestationV2 = [
    slot_name,
    reviewed_value,
    evidence_refs_sorted,
    rationale
]
~~~

Temporal values are reviewed values, not a rewrite of the ten historical
SourceContext slots. Each slot must be present exactly once, have a supported
non-`unresolved` value, a non-empty `evidence_refs_sorted`, and positive
source-bound evidence.

### 4.4 Member bridge attestation

~~~text
ContextMemberBridgeAttestationV2 = [
    context_slot_attestations_10,
    temporal_slot_attestations_4
]
~~~

### 4.5 Theorem/application equivalence

For every V2 member, the bound `ContextProofRecordV1` remains the authority for
the reviewed theorem values. The bridge changes only the old theorem-to-source
equality seam; it does not permit an application to drift away from its
theorem:

~~~text
member.context_binding_v1
    == exact ContextProofV1.subject_shape

member.context_binding_v1
    == the admitted SourceInstance/candidate shape wherever the unchanged
       V1 source/member binding rules require that comparison

for i in 0..9:
    member.bridge.context[i].reviewed_value
        == exact ContextProofV1.context_dimensions[i].value

for i in 0..3:
    member.bridge.temporal[i].reviewed_value
        == exact ContextProofV1.temporal_semantics[i].value

member.precondition_attestations_v1
    == the existing exact V1 theorem-precondition set and order
~~~

The corresponding source-side bridge facts remain independent:

~~~text
for i in 0..9:
    member.bridge.context[i].source_value
        == exact SourceInstanceV1.source_context[i].value

member.bridge.context[i].source_value
    != member.bridge.context[i].reviewed_value
    ↔ member.bridge.context[i].relation == reviewed_divergence
~~~

The V2 bridge therefore means exactly:

~~~text
historical SourceContextV1 value
    -> explicit source-side binding
reviewed ContextProofV1 theorem value
    -> exact theorem-side binding
~~~

It never validates a V1 `source_context` precondition against the reviewed
value. A mismatch with the theorem value, subject shape, temporal semantics,
or V1 precondition payload fails closed.

### 4.6 Exact member preimage

The member preimage is the following fixed canonical CBOR array. Text values
are UTF-8 strings; digest values are byte strings; arrays marked `_sorted` are
sorted by their complete canonical CBOR encoding and are duplicate-free.

~~~text
ContextApplicationMemberV2 = [
    candidate_id_utf8,
    candidate_identity_digest_reference_v1,
    source_instance_id_utf8,
    candidate_universe_binding,
    context_binding_v1,
    precondition_attestations_v1,
    member_evidence_refs_sorted,
    context_member_bridge_attestation_v2
]
~~~

`candidate_identity_digest_reference_v1` is the complete existing
`DigestReferenceV1`, not only its 32 digest bytes:

~~~text
DigestReferenceV1 = [
    envelope_id_utf8,
    algorithm_id_utf8,
    semantic_domain_utf8,
    payload_codec_id_utf8,
    input_schema_id_utf8,
    digest_bytes_32
]
~~~

Its JSON projection has the corresponding six closed named fields
`envelope_id`, `algorithm_id`, `semantic_domain`, `payload_codec_id`,
`input_schema_id`, and `digest_hex`. Only the extracted final digest bytes are
used for the member sort key; the complete reference remains part of the
member preimage and is validated against CandidateIdentityV1.

The member set is finite, non-empty, duplicate-free, and canonically ordered
by the existing V1 application member rule:

~~~text
[candidate_identity_digest_reference_v1.digest_bytes_32,
 UTF8(source_instance_id)]
~~~

The redundant `candidate_id` field is not used to replace that ordering rule.

## 5. V2 semantic and record identities

### 5.0 Common digest-envelope rule

Every new V2/V3 semantic identity is computed through the accepted envelope;
no new identity uses a naked hash of a canonical payload:

~~~text
canonical_payload = canonical_cbor(payload)

digest_envelope =
    ASCII("mtgml.digest-envelope.v1") || 0x00 ||
    frame(ASCII("sha-256")) ||
    frame(UTF8(semantic_domain)) ||
    frame(ASCII("mtgml.canonical-cbor.v1")) ||
    frame(UTF8(input_schema_id)) ||
    frame(canonical_payload)

digest_bytes = SHA256(digest_envelope)

DigestReferenceV1 = [
    "mtgml.digest-envelope.v1",
    "sha-256",
    semantic_domain,
    "mtgml.canonical-cbor.v1",
    input_schema_id,
    digest_bytes
]
~~~

`frame(x)` is an unsigned 64-bit big-endian byte length followed by the exact
bytes of `x`. The semantic-domain and input-schema strings are distinct fields;
the payload's first schema marker is the exact input schema ID where the
underlying contract specifies one.

For every identity below, the `<64 lowercase hex>` component is
`DigestReferenceV1.digest_bytes_32.hex()`, and the corresponding full digest
reference is recomputed and checked. `payload_codec_id` is always
`mtgml.canonical-cbor.v1`, `algorithm_id` is always `sha-256`, and `envelope_id`
is always `mtgml.digest-envelope.v1`.

### 5.1 Application identity

~~~text
ContextApplicationV2InputV1 = [
    "manafold.m2.5.c.context-application-input.v2",
    theorem_record_id_bytes,
    members_v2_sorted
]

context_application_id =
    "cpa.v2/" + hex(
        SHA256(digest_envelope(
            "manafold.m2.5.c.context-application.v2",
            "manafold.m2.5.c.context-application-input.v2",
            canonical_cbor(ContextApplicationV2InputV1)
        ))
    )
~~~

The semantic identity excludes acceptance metadata and the accepted record
identity. It includes the exact theorem record and every exact member bridge.

### 5.2 Accepted record identity

~~~text
ContextApplicationV2RecordInputV1 = [
    "manafold.m2.5.c.context-application-record-input.v2",
    context_application_v2_id_bytes,
    review_event_ref_v3_cbor
]

context_application_record_id =
    "cpar.v2/" + hex(
        SHA256(digest_envelope(
            "manafold.m2.5.c.context-application-record.v2",
            "manafold.m2.5.c.context-application-record-input.v2",
            canonical_cbor(ContextApplicationV2RecordInputV1)
        ))
    )
~~~

### 5.3 Supersession identity

`ContextApplicationV2SupersessionReasonCode` is a closed reuse of the accepted
V1 vocabulary:

~~~text
semantic_correction
source_revision
model_revision
authority_revocation
~~~

The replacement relationship is closed as follows:

~~~text
authority_revocation:
    replacement_record_id   = null
    replacement_record_kind = null

semantic_correction | source_revision | model_revision:
    replacement_record_id   != null
    replacement_record_kind = context_application_v2_record
~~~

No free-form reason string or unsupported reason value is admitted.

~~~text
ContextApplicationV2SupersessionInputV2 = [
    "manafold.m2.5.c.context-application-v2-supersession-input.v2",
    superseded_record_id_bytes,
    replacement_record_id_bytes_or_null,
    "context_application_v2_record",
    replacement_record_kind_or_null,
    reason_code,
    source_evidence_refs_sorted
]

context_application_supersession_id =
    "cps.v2/" + hex(
        SHA256(digest_envelope(
            "manafold.m2.5.c.context-application-supersession.v2",
            "manafold.m2.5.c.context-application-v2-supersession-input.v2",
            canonical_cbor(ContextApplicationV2SupersessionInputV2)
        ))
    )
~~~

The accepted supersession record has its own identity so that acceptance
provenance is immutable without reintroducing a semantic/event cycle:

~~~text
ContextApplicationV2SupersessionRecordInputV1 = [
    "manafold.m2.5.c.context-application-supersession-record-input.v2",
    supersession_id_bytes,
    review_event_ref_v3_cbor
]

supersession_record_id =
    "cpsr.v2/" + hex(
        SHA256(digest_envelope(
            "manafold.m2.5.c.context-application-supersession-record.v2",
            "manafold.m2.5.c.context-application-supersession-record-input.v2",
            canonical_cbor(ContextApplicationV2SupersessionRecordInputV1)
        ))
    )
~~~

The acceptance reference is not used to construct the semantic application or
supersession identity. It is record/admission metadata only. In particular,
`review_event_ref_v3` is excluded from the `cps.v2` preimage. The V3 event may
refer to the already computed supersession identity, and the supersession
record may refer to the resulting event, without an identity cycle.

Replacement rules:

- replacement is either `null` for revocation or an exact V2 record ID;
- non-null replacement must have kind
  `context_application_v2_record`;
- superseded and replacement records must be the same semantic kind;
- a record cannot supersede itself;
- a supersession cycle is rejected;
- multiple immutable historical records for one semantic application are
  allowed; the current record is selected only after valid same-kind
  supersession evaluation;
- historical records and supersession leaves are never mutated.

## 6. Persisted V2 container and wire shapes

### 6.1 Container

The dedicated top-level artifact is:

~~~text
path:
sources/m2_5/authorities/context_application_authority/v2/context_application_authority.v2.json

schema:
manafold.m2.5.c.context-application-authority.v2
~~~

Its closed JSON object fields, in canonical field order, are:

~~~text
{
  "schema": "manafold.m2.5.c.context-application-authority.v2",
  "base_authority_v1_binding": ContextAuthoritySourceBindingV2,
  "host_binding_authority_v2_binding": ContextAuthoritySourceBindingV2 | null,
  "candidate_universe_binding": ContextAuthoritySourceBindingV2,
  "source_bindings": ContextAuthoritySourceBindingV2[],
  "context_application_v2_records": ContextApplicationV2Record[],
  "context_application_v2_supersession_records":
      ContextApplicationV2SupersessionRecord[],
  "application_host_bindings_v2": ApplicationHostBindingV2[]
}
~~~

No `context_application_authority_v2` source-binding role is permitted: the
container must not bind itself.

The top-level `base_authority_v1_binding`, `candidate_universe_binding`, and
optional `host_binding_authority_v2_binding` are exact projections of the
corresponding unique entries in `source_bindings`; they are not additional
occurrences. A source-binding list must contain each represented role/path
tuple once.

Container arrays are canonically ordered by the applicable complete canonical
CBOR encoding and are duplicate-free. The container source set is exact; extra
or missing bindings fail closed.

### 6.2 Application record

~~~text
ContextApplicationV2Record = {
  "record_id": "cpar.v2/<64 lowercase hex>",
  "application_id": "cpa.v2/<64 lowercase hex>",
  "theorem_record_id": "cpr.v1/<64 lowercase hex>",
  "members": ContextApplicationMemberV2[],
  "acceptance": {
    "decision": "human_accepted",
    "review_event_ref": ReviewEventRefV3
  }
}
~~~

The JSON object is closed. `decision` is the sole accepted decision for this
record shape; no generated proposal or structural qualification may emit it.
The record identity may include its `review_event_ref_v3` through
`ContextApplicationV2RecordInputV1`, but the event subject payload is the
acceptance-free semantic record payload defined in §8.2; therefore the event
does not depend on `cpar.v2`.

### 6.3 Supersession record

~~~text
ContextApplicationV2SupersessionRecord = {
  "record_id": "cpsr.v2/<64 lowercase hex>",
  "supersession_id": "cps.v2/<64 lowercase hex>",
  "superseded_record_id": "cpar.v2/<64 lowercase hex>",
  "replacement_record_id": "cpar.v2/<64 lowercase hex>" | null,
  "superseded_record_kind": "context_application_v2_record",
  "replacement_record_kind": "context_application_v2_record" | null,
  "reason_code":
      "semantic_correction"
      | "source_revision"
      | "model_revision"
      | "authority_revocation",
  "source_evidence_refs": EvidenceRef[],
  "acceptance": {
    "decision": "human_accepted",
    "review_event_ref": ReviewEventRefV3
  }
}
~~~

The `replacement_record_kind` is null exactly when
`replacement_record_id` is null. `source_evidence_refs` is canonically sorted,
duplicate-free, and included in the supersession identity. The acceptance
reference is record metadata and is not included in `cps.v2`.

### 6.4 ApplicationHostBindingV2

This is additive and does not change `ApplicationHostBindingV1`.

JSON/wire form:

~~~json
{
  "application_kind": "context_application",
  "application_semantic_id": "cpa.v2/<64 lowercase hex>",
  "host_binding_claim_ids": ["hbc.v1/<64 lowercase hex>", "..."]
}
~~~

Canonical CBOR form:

~~~text
[
    application_kind,
    application_semantic_id,
    host_binding_claim_ids_sorted
]
~~~

The host-claim IDs are sorted by complete canonical CBOR encoding and are
duplicate-free. V2 accepts only `cpa.v2` for this application kind. Existing
V1 host bindings remain limited to `rpa.v1`, `dpa.v1`, and `cpa.v1`.

## 7. ContextAuthoritySourceBindingV2

This new binding type is distinct from both `SourceBindingDigestV1` and
`HostBindingSourceBindingV2`. Neither existing type is extended.

### 7.1 Wire and CBOR forms

JSON/wire form:

~~~json
{
  "artifact_role": "<closed role>",
  "path": "<repository-relative POSIX path>",
  "schema": "<closed schema id>" | null,
  "raw_sha256": "<64 lowercase hex>"
}
~~~

Canonical CBOR form:

~~~text
[
    artifact_role_utf8,
    path_utf8,
    schema_utf8_or_null,
    raw_sha256_bytes_32
]
~~~

The identity of a binding is the complete four-field tuple. In addition, the
pair `(artifact_role, path)` is a unique key within each source-binding list:
any duplicate role/path pair fails, even when schema or raw digest differs.
The role/path grammar maps every path to exactly one role, so a path cannot be
reintroduced under a different role. Content-addressed leaf or claim roles may
repeat only with distinct content-addressed paths; static snapshot roles occur
at most once per closure. Exact duplicate tuples always fail.

### 7.2 Closed role/path/schema registry

The following table is the complete role vocabulary for V2 context authority
and V3 acceptance-event bindings:

| role | exact path grammar | exact schema |
|---|---|---|
| `base_authority_v1` | `sources/m2_5/authorities/interaction_review_authority.v1.json` | `manafold.m2.5.c.interaction-review-authority.v1` |
| `declared_model` | `sources/m2_5/closures/C/declared_interaction_model.v2.json` | `manafold.m2.5.c.declared-interaction-model.v2` |
| `candidate_universe` | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `manafold.m2.5.c.interaction-candidate-universe.v2` |
| `rev3_candidate_census` | `derived/Pair_Interaction_Census_REV3.csv` | null |
| `rev3_pair_aggregates` | `derived/Pair_Requirement_Aggregates_REV3.json` | null |
| `rev3_card_requirement_map` | `derived/Card_Requirement_Map_REV3.csv` | null |
| `rev3_deck_row_source_resolution` | `inputs/deck_row_source_resolution_REV3.csv` | null |
| `rev3_osi_source_records` | `source/raw/oracle_cards_selected_REV3.jsonl` | null |
| `rev3_source_index` | `source/raw/source_record_index_REV3.csv` | null |
| `b2_catalog` | `sources/m2_5/closures/B2/requirement_family_catalog.v1.json` | `manafold.m2.5.b2.requirement-family-catalog.v1` |
| `b2_classifications` | `sources/m2_5/closures/B2/card_semantic_classifications.v1.json` | `manafold.m2.5.b2.card-semantic-classifications.v1` |
| `b2_closure` | `sources/m2_5/closures/B2/classification_closure.v1.json` | `manafold.m2.5.b2.classification-closure.v1` |
| `b1_final_citations` | `sources/m2_5/closures/B1/official_authority_citations.v3.json` | `manafold.m2.5.b1.official-authority-citations.v3` |
| `b1_final_closure` | `sources/m2_5/closures/B1/official_authority_citation_closure.v2.json` | `manafold.m2.5.b1.official-authority-citation-closure.v2` |
| `reviewer_roster_leaf` | `sources/m2_5/authorities/reviewer_rosters/v1/<64 lowercase hex>.json` | `manafold.m2.5.c.reviewer-roster.v1` |
| `acceptance_event_leaf_v3` | `sources/m2_5/authorities/review_acceptance_events/v3/<64 lowercase hex>.json` | `manafold.m2.5.c.review-acceptance-event.v3` |
| `host_binding_authority_v2` | `sources/m2_5/authorities/interaction_review_authority.v2.json` | `manafold.m2.5.c.interaction-review-authority.v2` |
| `host_binding_claim_record` | `sources/m2_5/authorities/cross_deck_host_binding_claims/v1/<64 lowercase hex>.json` | `manafold.m2.5.c.cross-deck-host-binding-claim-record.v1` |
| `context_application_authority_v2` | not permitted | not permitted |
| `acceptance_event_leaf_v2` | not permitted in this V3 closure | not permitted in this V3 closure |
~~~

The path is repository-relative and uses `/`; absolute paths, URLs, path
aliases, prefix stripping, and schema substitution are rejected. The basename
of every content-addressed leaf must be lowercase hex and must equal the
required digest component for that role.

Canonical binding ordering is by complete canonical CBOR encoding of the
four-element binding tuple. The role table is closed: an unknown role, a
known role at a different path/schema, a duplicate role/path pair, a duplicate
static role, an exact duplicate tuple, or a content-addressed repeat using the
same path fails closed.

## 8. ReviewAcceptanceEventV3 and ReviewEventRefV3

### 8.1 Event family

~~~text
ReviewAcceptanceEventV3:
  schema = manafold.m2.5.c.review-acceptance-event.v3
  event_id = ae.v3/<64 lowercase hex>
  path = sources/m2_5/authorities/review_acceptance_events/v3/<64 lowercase hex>.json
  checklist_id = interaction-authority-review-checklist.v2
~~~

Its closed JSON/wire object is:

~~~text
{
  "event_id": "ae.v3/<64 lowercase hex>",
  "schema": "manafold.m2.5.c.review-acceptance-event.v3",
  "subject_kind": "context_application_v2_record"
      | "context_application_v2_supersession_record",
  "subject_payload_digest": DigestReferenceV1,
  "decision": "human_accepted",
  "reviewer_roster_ref": ReviewerRosterRefV1,
  "reviewer_role_bindings": ReviewerRoleBinding[],
  "review_mode": <closed review-mode enum>,
  "checklist_id": "interaction-authority-review-checklist.v2",
  "source_binding_digests": ContextAuthoritySourceBindingV2[],
  "review_evidence_refs": EvidenceRef[]
}
~~~

The event subject payload is computed from the exact subject-specific payload
before acceptance metadata. The event must not include its own leaf or the
future context container in `source_binding_digests`.

The event semantic input is:

~~~text
ReviewAcceptanceEventV3InputV1 = [
    "manafold.m2.5.c.review-acceptance-event-input.v3",
    subject_kind,
    subject_payload_digest_reference_v1,
    "human_accepted",
    reviewer_roster_ref_cbor,
    reviewer_role_bindings_sorted,
    review_mode,
    "interaction-authority-review-checklist.v2",
    source_binding_digests_sorted,
    review_evidence_refs_sorted
]

event_id = "ae.v3/" + hex(
    SHA256(digest_envelope(
        "manafold.m2.5.c.review-acceptance-event.v3",
        "manafold.m2.5.c.review-acceptance-event-input.v3",
        canonical_cbor(ReviewAcceptanceEventV3InputV1)
    ))
)
~~~

### 8.2 AcceptanceSubjectPayloadV3

This version is new and does not reuse the Host-Binding V2
`manafold.m2.5.c.acceptance-subject-payload.v2` contract.

~~~text
AcceptanceSubjectPayloadV3InputV1 = [
    "manafold.m2.5.c.acceptance-subject-payload-input.v3",
    subject_kind,
    subject_wire_payload_without_acceptance_metadata
]

subject_payload_digest_reference_v1 = DigestReferenceV1(
    semantic_domain="manafold.m2.5.c.acceptance-subject-payload.v3",
    input_schema_id="manafold.m2.5.c.acceptance-subject-payload-input.v3",
    payload=canonical_cbor(AcceptanceSubjectPayloadV3InputV1)
)

asp.v3 subject digest ID =
    "asp.v3/" + subject_payload_digest_reference_v1.digest_bytes_32.hex()
~~~

For a record subject, the acceptance-free payload is exactly:

~~~text
ContextApplicationV2RecordSubjectPayloadV1 = [
    "context_application_v2_record",
    application_id_bytes,
    theorem_record_id_bytes,
    members_v2_sorted
]
~~~

For a supersession subject, it is exactly the supersession semantic payload
without its acceptance object:

~~~text
ContextApplicationV2SupersessionSubjectPayloadV1 = [
    "context_application_v2_supersession_record",
    supersession_id_bytes,
    superseded_record_id_bytes,
    replacement_record_id_bytes_or_null,
    superseded_record_kind,
    replacement_record_kind_or_null,
    reason_code,
    source_evidence_refs_sorted
]
~~~

`subject_kind` is closed to the two V2 subject kinds above. The event semantic
identity is recomputed from the exact accepted V3 event fields and the
precomputed subject payload digest; no event ID is caller-selected.
The input-schema marker is
`manafold.m2.5.c.acceptance-subject-payload-input.v3`; it is not the semantic
domain string. The `asp.v3` digest is the envelope digest, not a naked payload
hash.

### 8.3 ReviewEventRefV3

JSON/wire form:

~~~json
{
  "event_id": "ae.v3/<64 lowercase hex>",
  "path": "sources/m2_5/authorities/review_acceptance_events/v3/<64 lowercase hex>.json",
  "raw_sha256": "<64 lowercase hex>"
}
~~~

Canonical CBOR form:

~~~text
[
    path_utf8,
    raw_sha256_bytes_32,
    ["event_id", event_id_utf8]
]
~~~

Validation is exact and independent for the semantic and raw digests:

~~~text
basename_hex(path)
    == digest component of event_id

parsed leaf.event_id
    == referenced event_id

recomputed event semantic identity
    == referenced event_id

SHA256(exact raw event-file bytes)
    == ReviewEventRefV3.raw_sha256
~~~

The event semantic digest is not required to equal the raw-file digest.

## 9. Review checklist and roles

The existing `interaction-authority-review-checklist.v1` remains immutable and
unchanged. The V2 bridge uses a new immutable definition:

~~~text
interaction-authority-review-checklist.v2
~~~

In addition to all V1 obligations, V2 requires explicit review of:

- exact historical `source_value` for all ten context slots;
- exact reviewed value for all ten context slots and all four temporal slots;
- mechanically derived `exact_match` versus `reviewed_divergence`;
- non-empty, independent positive evidence for every reviewed context value,
  including `exact_match` and `not_applicable`, and every reviewed temporal
  value;
- explicit historical source binding and inequality proof for every
  `reviewed_divergence`;
- no normalization, absence-of-evidence inference, or SourceContext rewrite;
- source-binding and snapshot closure;
- V2 record supersession and revocation.

The semantic obligations of checklist V2 are immutable after admission. A
material change requires a new checklist ID/version; V2 is never overwritten
or repurposed.

Every accepted V2 record requires:

~~~text
architecture_maintainer
rules_authority_maintainer
~~~

`information_safety_reviewer` is additionally mandatory whenever the reviewed
context or evidence implicates information or visibility semantics.
`conformance_maintainer` may be required by the checklist. Solo review does
not waive any role or evidence requirement and still requires a separate full
review pass.

## 10. Acceptance source-closure functions

Event and container closure are separate contracts.

### 10.1 ExpectedAcceptanceSourceClosureV3

For one V3 acceptance event:

~~~text
ExpectedAcceptanceSourceClosureV3(subject) =
  {base_authority_v1, declared_model, reviewer_roster_leaf}
  union subject theorem/application/member evidence
  union candidate-universe and SourceInstance bindings
  union all B1/B2 evidence and required complete B2 closure
  union all ten context-slot evidence refs
  union all four temporal-slot evidence refs
  union optional host_binding_authority_v2 and host_binding_claim_record refs
  minus {acceptance_event_leaf_v3, context_application_authority_v2}
~~~

The exact role/path/schema/raw-digest tuples are reconstructed from the
subject, not accepted from a caller-selected superset. Extras and missing
bindings fail closed. A V3 event source list may not contain its own leaf or
the context container.

### 10.2 ExpectedContainerSourceClosureV2

For the completed context container:

~~~text
ExpectedContainerSourceClosureV2(container) =
  {container base/static bindings}
  union every referenced ae.v3 leaf
  union each referenced event's exact event closure sources
  union every required host authority/claim artifact
  minus {context_application_authority_v2}
~~~

The container must bind every referenced event leaf. The two exact source sets
are compared independently; no event/container cycle is permitted.

## 11. Host-binding cross-snapshot rules

If a V2 context container participates in Host-Binding V2, the shared immutable
snapshots must be exactly equal:

~~~text
context.base_authority_v1_binding
    == host_binding_v2.base_authority_v1_binding

context.candidate_universe_binding
    == unique host_binding_v2.source_bindings[
        artifact_role == "candidate_universe"
    ]
~~~

The Host-Binding V2 object has no dedicated top-level
`candidate_universe_binding` field. The unique role-selected binding above is
the authoritative value. For every other shared immutable snapshot, equality
is the complete `ContextAuthoritySourceBindingV2` tuple: role, path, schema,
and raw digest.

If the Host-Binding V2 container does not carry a required shared binding, V2
participation fails closed. There is no compatibility-by-recency, rebasing,
normalization, or “newer snapshot” exception.

## 12. Validation algorithm

The implementation must validate in this order:

1. Validate closed JSON/wire shapes, version prefixes, path grammar, enum
   vocabulary, UTF-8, and canonical ordering.
2. Validate all V2 identities from their exact canonical CBOR preimages.
3. Resolve the exact `ContextProofRecordV1`; validate its V1 identity and
   preserve all V1 source-context precondition checks.
4. Resolve the exact CandidateIdentityV1, SourceInstanceV1, candidate-universe
   binding, and member set. Reject duplicate, missing, substituted, or
   cross-source-instance members.
5. Validate all ten historical source values against the bound SourceInstance;
   validate all four temporal slots against the closed temporal vocabulary.
6. Validate theorem/application equivalence for subject shape, all ten context
   values, all four temporal values, and the exact V1 precondition set. Then
   validate `exact_match`/`reviewed_divergence` mechanically and require
   independent positive evidence for every reviewed context value and every
   reviewed temporal value. `reviewed_divergence` additionally requires the
   exact historical source value and a mechanically proven mismatch.
7. Resolve every evidence locator, raw digest, schema, archive member, and
   source binding. Reject wrong snapshots, stale bytes, cross-candidate
   substitution, or evidence from a different SourceInstance.
8. Reconstruct `ExpectedAcceptanceSourceClosureV3` or
   `ExpectedContainerSourceClosureV2` as appropriate and require exact set
   equality.
9. If Host-Binding V2 is present, enforce exact shared-snapshot equality and
   validate `ApplicationHostBindingV2` separately from V1 bindings.
10. Validate reviewer roster, mandatory roles, checklist V2, separate-review
    mode, and the exact V3 event reference.
11. Validate same-kind supersession, replacement/revocation rules, and reject
    cycles before selecting a current record.
12. On any rejection, mutate nothing and do not create or promote an accepted
    record.

No stage may infer semantics from lexical text, capability names,
co-occurrence, absence of evidence, or array position outside the explicitly
specified canonical ordering.

## 13. Required negative matrix

The implementation and conformance suite must reject at least:

- missing any of the ten context slots;
- missing any of the four temporal slots;
- duplicate slot;
- wrong slot order;
- wrong CandidateIdentity;
- wrong SourceInstance ID;
- wrong candidate-universe binding;
- source value different from bound SourceContextV1;
- reviewed context value different from the corresponding ContextProofV1
  context dimension;
- reviewed temporal value different from the corresponding ContextProofV1
  temporal semantic;
- `context_binding_v1` different from the ContextProofV1 subject shape;
- a V1 precondition attestation different from the bound theorem precondition;
- `exact_match` where source differs from reviewed value;
- `reviewed_divergence` where source equals reviewed value;
- missing positive evidence for divergence;
- `exact_match` with empty evidence refs or without positive reviewed-value
  evidence;
- evidence from the wrong snapshot;
- stale raw digest;
- cross-candidate evidence substitution;
- cross-SourceInstance evidence substitution;
- unsupported enum;
- materialized `unresolved` value;
- blanket all-`not_applicable` without positive semantic justification;
- V2 application bound to the wrong ContextProof record;
- duplicate members;
- noncanonical member order;
- unknown SourceBinding role;
- role/path/schema mismatch;
- duplicate static binding or exact duplicate tuple;
- duplicate `(artifact_role, path)` with a different raw digest or schema;
- repeated content-addressed leaf/claim using the same path;
- event source list containing its own leaf;
- event source list containing the context container;
- container missing a referenced event leaf;
- extra or missing event/container closure source;
- host-binding base-authority digest mismatch;
- host-binding candidate-universe digest mismatch;
- `ApplicationHostBindingV1` carrying `cpa.v2`;
- malformed V2 JSON object or malformed V2 CBOR array;
- wrong V3 checklist ID;
- missing architecture or rules-authority role;
- information-sensitive review without information-safety reviewer;
- solo review that omits a role or evidence requirement;
- event path basename/event-ID mismatch;
- event semantic-ID mismatch;
- event raw digest mismatch;
- supersession replacement of a different record kind;
- unsupported or free-form supersession reason code;
- `authority_revocation` with a replacement record;
- a correction/revision reason without a replacement record;
- self-supersession or supersession cycle;
- supersession acceptance/source-closure cycle.

## 14. Rust/Python/schema ownership

The authoritative contract vocabulary and version registry must be defined in
the repository's existing contract source, then generated or checked for:

- Rust DTOs and canonical-CBOR identity producers;
- Python DTOs, resolver, and validator;
- JSON Schemas for wire objects;
- canonical/negative fixtures and conformance tests;
- documentation and acceptance-gate checks.

The JSON object shape and canonical CBOR array shape are separate test
surfaces. Rust and Python must produce identical identity bytes, ordering,
closed-enum behavior, source-set reconstruction, and failure behavior.

Existing V1 fixtures must be decoded and validated byte-for-byte as before.
No V1 producer may emit a V2 prefix, and no V2 consumer may reinterpret a V1
record as a V2 record.

## 15. Cross-layer impact

### RelationApplication

Relation V1 remains unchanged. A context V2 bridge may be referenced by a
future cross-layer authority container only through an explicitly versioned
V2 link; V1 RelationApplication cannot reference `cpa.v2` implicitly.

### DomainProof / DomainApplication

Domain V1 remains unchanged. A V2 context bridge can provide reviewed context
facts only through an explicitly versioned consumer contract. It does not
convert those facts into domain applicability or bypass DomainProof
preconditions.

### Host Binding

`ApplicationHostBindingV1` and the accepted Host-Binding V2 meaning are
unchanged. `ApplicationHostBindingV2` is the additive successor for `cpa.v2`
and reuses member-atomic `hbc.v1` claims without changing their identity.

### ClassProjectionV1

`ClassProjectionV1` remains unchanged. A context bridge does not create a
class projection and cannot make a non-class-producing theorem class-producing.

### Future C derivation

No current C V3/V4 artifact changes. A future C derivation must explicitly
select a versioned consumer contract and bind the exact V2 record/event/source
closure. It must not treat the existence of a V2 bridge as a classification or
terminal disposition.

## 16. Historical compatibility and migration

There is no in-place migration of V1 artifacts. Existing V1 identities,
records, acceptance events, schemas, fixtures, and replay behavior remain
valid under their original contracts.

Implementation is additive:

1. admit the closed V2 contract and registry without changing V1;
2. implement independent V2 decode/identity/validation paths;
3. add V3 acceptance-event and V2 checklist support;
4. validate synthetic and real source-bound fixtures;
5. materialize no production V2 record until human review is separately
   authorized;
6. integrate downstream consumers only through explicit versioned links.

Changing historical SourceContext values, reusing an existing prefix, or
rebasing a V2 record onto a different source snapshot is forbidden.

## 17. Determinism and scaling

All identities and source closures use exact canonical bytes, sorted complete
CBOR encodings, closed vocabularies, and raw digest checks. Timestamps,
environment identity, local paths, URLs, current usernames, and network lookups
are excluded from deterministic identity.

The member bridge is finite and independently verifiable per member. The
implementation should stream or index source resolution rather than build a
second semantic engine. For the current 15,679-candidate universe, unchanged
source snapshots may be shared by digest while each V2 member retains exact
CandidateIdentity/SourceInstance and evidence closure. Batch partitioning must
not alter semantic identity: member order is canonical and application
membership is explicit.

## 18. Smallest safe implementation slices

Implementation remains unauthorized by this ADR. If later authorized, use
these bounded slices:

1. contract registry, schemas, Rust/Python DTOs, canonical identity vectors;
2. V2 source-binding registry and event/container closure resolver;
3. V2 bridge/member validator with preserved V1 precondition behavior;
4. V3 event/reference and V2 checklist/role validation;
5. supersession validator and immutable record fixtures;
6. cross-layer HostBindingV2 integration;
7. real Buckle-Up review only after all gates pass.

No slice creates `human_accepted`, an Acceptance Event, a production Authority
record, a ClassProjection, a C change, Slice 3B work, or M3 work without a
separate authorization.

## 19. Acceptance gates before implementation or canary resumption

Before implementation is accepted, independently run and report:

- contract-registry and schema-drift checks;
- Rust/Python canonical-CBOR identity parity;
- V1 historical fixture decode/validation regression;
- positive exact-match bridge fixture;
- positive reviewed-divergence bridge fixture with evidence;
- all negative cases in §13;
- event-vs-container closure cycle tests;
- HostBinding V1/V2 cross-snapshot tests;
- reviewer-role and checklist-V2 tests;
- deterministic repeatability and raw-byte hash checks;
- `git diff --check`;
- repository fast, integration, certification, Rust, Python, archive, and
  reproducibility gates;
- hosted exact-head CI where applicable.

Before the Buckle-Up canary receives any human acceptance consideration,
`timing`, `decision_actor_relation`, and `control_ownership_relation` must be
separately positively reviewed. This ADR intentionally does not decide those
values.

## 20. Collision-free contract registry

The following is the single registry for every new semantic identity in this
ADR. The existing V1 and Host-Binding V2 rows are preserved by reference and
are not redefined here.

| identity family | prefix | semantic_domain | input_schema_id | payload codec | algorithm | envelope |
|---|---|---|---|---|---|---|
| Context application V2 | `cpa.v2/` | `manafold.m2.5.c.context-application.v2` | `manafold.m2.5.c.context-application-input.v2` | `mtgml.canonical-cbor.v1` | `sha-256` | `mtgml.digest-envelope.v1` |
| Context application record V2 | `cpar.v2/` | `manafold.m2.5.c.context-application-record.v2` | `manafold.m2.5.c.context-application-record-input.v2` | `mtgml.canonical-cbor.v1` | `sha-256` | `mtgml.digest-envelope.v1` |
| Context application supersession V2 | `cps.v2/` | `manafold.m2.5.c.context-application-supersession.v2` | `manafold.m2.5.c.context-application-v2-supersession-input.v2` | `mtgml.canonical-cbor.v1` | `sha-256` | `mtgml.digest-envelope.v1` |
| Context application supersession record V2 | `cpsr.v2/` | `manafold.m2.5.c.context-application-supersession-record.v2` | `manafold.m2.5.c.context-application-supersession-record-input.v2` | `mtgml.canonical-cbor.v1` | `sha-256` | `mtgml.digest-envelope.v1` |
| Context acceptance subject V3 | `asp.v3/` | `manafold.m2.5.c.acceptance-subject-payload.v3` | `manafold.m2.5.c.acceptance-subject-payload-input.v3` | `mtgml.canonical-cbor.v1` | `sha-256` | `mtgml.digest-envelope.v1` |
| Context acceptance event V3 | `ae.v3/` | `manafold.m2.5.c.review-acceptance-event.v3` | `manafold.m2.5.c.review-acceptance-event-input.v3` | `mtgml.canonical-cbor.v1` | `sha-256` | `mtgml.digest-envelope.v1` |

Preserved, already-owned registry families:

| family | prefix/domain | rule |
|---|---|---|
| Context proof, record, application | existing `cp.v1/`, `cpr.v1/`, `cpa.v1/` | unchanged V1 registry entries and meanings |
| Host-Binding claim and acceptance | existing `hbc.v1/`, `asp.v2/`, `ae.v2/` | unchanged Host-Binding V2 registry entries and meanings |
| Review checklist V1 | `interaction-authority-review-checklist.v1` | immutable, unchanged |

The V2 checklist identifier
`interaction-authority-review-checklist.v2` and the container schema
`manafold.m2.5.c.context-application-authority.v2` are versioned contract
identifiers, not digest-prefixed semantic IDs. The new SourceBinding type is
`ContextAuthoritySourceBindingV2`; it does not reuse or extend either existing
binding type. `b1_final_citations` and `b1_final_closure` are part of that
closed role registry in §7.2.

## Final status

~~~text
CONTEXT_APPLICATION_V2_ADR       = ACCEPTED
CONTRACT_GAPS_REMAINING          = 0
IMPLEMENTATION_DIRECTION         = FROZEN
FROZEN_CONTRACT                  = NO
IMPLEMENTATION_AUTHORIZED        = NO
HUMAN_ACCEPTANCE                  = BLOCKED
TASK_5_SLICE_3B                  = BLOCKED
M3                               = BLOCKED
~~~

The Buckle-Up semantic values remain undecided. No Acceptance Event, Authority
record, ClassProjection, C change, or historical V1 reinterpretation is
created by this specification.
