# ADR 0045: Reviewed Participant-Role Bridge

- **Status:** accepted
- **Date:** 2026-09-09
- **Supersedes:** none
- **Superseded by:** none
- **Depends on:** ADR 0041, ADR 0042, ADR 0043, ADR 0044
- **Reviewed baseline:** `b130a0eb6cae9b089660a2d2d20270b3e817f8e8`
- **Candidate SHA-256 before promotion:** `1fa7292fd9505a57df6480fcdeab7bb3cda5f3ea1f2439149c7d4fffb662905f`
- **Review provenance:** final independent adversarial contract review; `0 BLOCKER / 0 MAJOR / 0 MINOR`
- **Implementation evidence:** `NOT_RUN`

This is an accepted architecture decision only. It does not authorize
implementation, production authority records, C changes, or M3.

## Final contract review result

```text
SOURCE_INSTANCE_V1_IMMUTABLE        = PASS
RELATION_PROOF_V1_UNCHANGED         = PASS
PARTICIPANT_ROLE_BRIDGE_V1          = PASS
RPA_V1_EXACT_ROLE_PATH              = PASS
RPA_V2_DIVERGENT_ROLE_PATH          = PASS
RPA_V2_PRECONDITION_RESOLVER        = PASS
RPA_V2_CURRENTNESS                  = PASS
RPA_V2_AGGREGATE                    = PASS
CONTEXT_APPLICATION_V2_FROZEN       = PASS
CONTEXT_APPLICATION_V3              = PASS
CONTEXT_V3_PRECONDITION_RESOLVER    = PASS
CONTEXT_V3_CURRENTNESS              = PASS
CONTEXT_RPA_SNAPSHOT_COHERENCE      = PASS
APPLICATION_HOST_BINDING_V3         = PASS
HOST_BINDING_SEPARATION             = PASS
HBC_SEMANTICS_UNCHANGED             = PASS
SHARED_DIGEST_ENVELOPE              = PASS
V2_V3_MEMBER_WIRE_SHAPES            = PASS
SUPERSESSION_CONTRACT               = PASS
V3_BINDING_PROJECTIONS              = PASS
V3_AGGREGATE_COLLECTIONS            = PASS
AE_V4_SUBJECTS                      = PASS
AE_V4_REVIEWER_POLICY               = PASS
AE_V4_REVIEW_EVIDENCE               = PASS
AE_V4_SOURCE_BINDING_TYPE           = PASS
AE_V4_REV3_REGISTRY                 = PASS
V1_V4_SOURCE_PROJECTION             = PASS
TRANSITIVE_EVENT_CLOSURES           = PASS
SUPERSESSION_ENDPOINT_CLOSURES      = PASS
MUTABLE_AGGREGATES_EXCLUDED         = PASS
CANDIDATE_4_ARCHITECTURE            = PASS
NEW_MAGIC_SEMANTIC_DEFECT           = 0

BLOCKER                             = 0
MAJOR                               = 0
MINOR                               = 0
ARCHITECTURAL_REDIRECTION_REQUIRED  = NO
ADR_READY                           = PASS
ADR_ACCEPTANCE_RECOMMENDED          = YES
PERMANENT_ADR_PROMOTION             = AUTHORIZED / RECOMMENDED
IMPLEMENTATION                      = NOT_AUTHORIZED
PRODUCTION_AUTHORITY_RECORDS        = NOT_AUTHORIZED
C_CHANGE                            = STOP
DOMAIN_AUDIT_55_55                  = NOT_RUN
M3                                  = NOT_AUTHORIZED
```

## Verified baseline

The live remote `origin/master` was read at:

```text
b130a0eb6cae9b089660a2d2d20270b3e817f8e8
```

The local checkout was `master` at `d647e63d7687bd2c022393feaee5bac6736e84ca`,
behind the live remote. The worktree contained one unrelated untracked plan;
this candidate does not modify or remove it.

The current C status is not “no Pair/Relation Authority exists.” The committed
authority contains accepted Buckle-Up canary review authority. C remains
blocked because complete candidate/domain/context authority closure is absent:

```text
C = BLOCKED
accepted review authority exists for the Buckle-Up canary
complete candidate/domain/context authority closure = absent
M3_STARTED = NO
```

The five-candidate pilot is identified by the user-supplied exact pilot bytes.
It is not silently replaced by a similarly named candidate from the larger
current C inventory.

## Problem

Historical source provenance and reviewed relation semantics are different
facts:

```text
historical source role != reviewed semantic role
```

For the pilot's Candidate 4:

```text
candidate_id:
  CROSS_DECK|P3|cap.mass_destruction|cap.death_trigger|DIRECTIONAL_BINARY

candidate_identity:
  af33dd4f0b65103102828bfec8ebd23196b1685282ee6ef9f0dc4690e3a6420b

REV3 row_ordinal:
  6463
```

The historical source contains two `ordered_participant` roles, while the
reviewed theorem requires `source` and `affected`. Historical array position
does not establish causality.

## Decision direction

The architecture remains:

```text
SourceInstanceV1
    unchanged historical provenance

RelationProofV1
    unchanged reviewed relation theorem

ParticipantRoleBridgeV1
    new embedded source-to-reviewed role binding

RelationApplicationV2
    new owner of the reviewed role application

ContextApplicationV2
    unchanged; no alternate validator profile

ContextApplicationV3
    new versioned semantic application for role-divergent context members

HostBinding
    unchanged and independent
```

## Semantic ownership

For every participant position `i`:

```text
SourceInstanceV1.participant_bindings[i]
    = historical role, participant kind, semantic reference

RelationProofV1.subject.participant_roles[i]
    = reviewed role, participant kind, semantic reference

ParticipantRoleBridgeV1[i]
    = position
    + participant kind
    + semantic reference
    + historical source role
    + reviewed role
```

The bridge proves identity across representations. It does not prove causal
truth. Causal truth remains owned by `RelationProofV1`.

`HostBinding` proves host/deck realization only. It cannot create or infer
`source`, `affected`, or any other relation role.

## ParticipantRoleBridgeV1

The bridge is a typed value, not an independent authority family. It has no
standalone acceptance event, supersession graph, or digest prefix. It is part
of the `RelationApplicationV2` semantic preimage.

Canonical CBOR:

```text
ParticipantRoleBridgeV1 = [
    ParticipantRoleBridgeEntryV1, ...
]

ParticipantRoleBridgeEntryV1 = [
    position_u32,
    participant_kind_utf8,
    semantic_ref_utf8,
    historical_source_role_utf8,
    reviewed_role_utf8
]
```

Wire form:

```json
{
  "participant_role_bridge": [
    {
      "position": 0,
      "participant_kind": "requirement_family",
      "semantic_ref": "cap.mass_destruction",
      "historical_source_role": "ordered_participant",
      "reviewed_role": "source"
    }
  ]
}
```

Required invariants:

- exactly one entry for every participant;
- positions are exactly `0..n-1` and canonically ordered;
- participant count equals the exact SourceInstance and theorem count;
- participant kind and semantic reference equal both source and theorem;
- `historical_source_role` equals the exact SourceInstance role;
- `reviewed_role` equals the exact RelationProof role;
- only the two role fields may differ;
- no role is inferred from position, left/right ordering, host, card name, or
  capability name;
- no `null`, `unknown`, or `unresolved` role is admitted.

## Shared digest-envelope contract

Every identity family introduced by this ADR uses the existing
`mtgml.digest-envelope.v1` contract unchanged:

```text
envelope_id      = mtgml.digest-envelope.v1
algorithm_id     = sha-256
payload_codec_id = mtgml.canonical-cbor.v1
```

For the exact semantic input of each identity:

```text
canonical_payload = canonical_cbor(exact semantic input)

digest_envelope =
    ASCII("mtgml.digest-envelope.v1") || 0x00 ||
    frame(ASCII("sha-256")) ||
    frame(UTF8(semantic_domain)) ||
    frame(ASCII("mtgml.canonical-cbor.v1")) ||
    frame(UTF8(input_schema_id)) ||
    frame(canonical_payload)

digest_bytes = SHA256(digest_envelope)
```

`frame(x)` is an unsigned 64-bit big-endian byte length followed by the exact
bytes of `x`. No new identity uses a naked payload hash, a JSON hash, a raw
UTF-8 ID hash, or a different codec. `DigestReferenceV1` remains the complete
six-field envelope reference used wherever a full reference is required.

This rule applies to every new `rpa.v2`, `rpar.v2`, `rps.v2`, `rpsr.v2`,
`cpa.v3`, `cpar.v3`, `cps.v3`, `cpsr.v3`, `asp.v4`, and `ae.v4` identity.

## RelationApplicationV2

`RelationProofV1`, `RelationApplicationV1`, and all V1 validators remain
unchanged.

Canonical V2 member shape:

```text
RelationApplicationMemberV2 = [
    candidate_id_utf8,
    candidate_identity_digest_reference_v1,
    source_instance_id_utf8,
    candidate_universe_binding_v1,
    reviewed_relation_binding_v1,
    participant_role_bridge_v1,
    precondition_attestations_v1,
    member_evidence_refs_sorted,
    member_proof_attestation_v1
]
```

`reviewed_relation_binding_v1` retains the existing five-field shape:

```text
[
    scope,
    relation,
    directionality,
    host_relationship,
    reviewed_participant_roles
]
```

The member ordering remains:

```text
[candidate_identity_digest_bytes_32, UTF8(source_instance_id)]
```

The V2 application identity is:

```text
RelationApplicationV2InputV1 = [
    "manafold.m2.5.c.relation-application-input.v2",
    theorem_record_id_bytes,
    terminal_disposition,
    members_v2_sorted
]
```

Identity:

```text
rpa.v2/<digest>
semantic_domain = manafold.m2.5.c.relation-application.v2
input_schema_id = manafold.m2.5.c.relation-application-input.v2
```

The bridge is included in `members_v2_sorted`, so changing a role mapping
changes the `rpa.v2` identity.

### Strict V1/V2 eligibility

`RelationApplicationV2` is exclusively the divergent-role path. Every V2
member must satisfy:

```text
exists position i:
    historical_source_role[i] != reviewed_role[i]
```

An exact-role member is not valid in RPA V2 and must remain on the V1 path:

```text
exact-role member
    -> RelationApplicationV1 only

role-divergent member
    -> RelationApplicationV2 only
```

An RPA V2 application may not mix exact-role and divergent-role members. If a
reusable theorem is applied to both populations, the application member sets
are split into separate V1 and V2 applications. There is no cross-version
"latest" selection.

The accepted-record identity is:

```text
RelationApplicationV2RecordInputV1 = [
    "manafold.m2.5.c.relation-application-record-input.v2",
    relation_application_id_bytes,
    review_event_ref_v4_cbor
]
```

Identity:

```text
rpar.v2/<digest>
semantic_domain = manafold.m2.5.c.relation-application-record.v2
input_schema_id = manafold.m2.5.c.relation-application-record-input.v2
```

The semantic domain and input schema identifiers are frozen together with the
V4 acceptance contract in this candidate.

### RelationApplicationV2 precondition resolver

`RelationApplicationV2` retains `precondition_attestations_v1` and the six
closed V1 precondition kinds. V2 does not reinterpret their source meaning.
Its exact resolver is:

```text
candidate_relation_shape
    -> existing exact V1 source/candidate shape
    -> relation_binding host relationship is the reviewed theorem value

participant_binding
    -> exact historical SourceInstanceV1 comparison:
       [position, historical role, participant_kind, semantic_ref]
    -> ParticipantRoleBridgeV1 separately proves historical role -> reviewed
       role
    -> never reinterpret the V1 precondition as the reviewed role

b2_boundary
    -> existing exact V1 B2 theorem-side semantics

source_context
    -> existing exact historical SourceInstanceV1 semantics

temporal_semantic
    -> existing exact theorem/member attestation semantics

class_projection
    -> existing RelationApplicationV1 member-proof semantics
    -> reviewed projection remains owned by RelationProof/member proof
```

The RPA V2 resolver is a versioned seam only because it adds the bridge
validation alongside the unchanged V1 precondition facts. It does not weaken,
skip, or reinterpret any V1 precondition. A mismatch in the historical
`participant_binding` fact fails closed even when the bridge contains a
plausible reviewed role.

### RelationApplicationMemberV2 wire shape

The JSON/Wire object is closed (`additionalProperties = false`) and has exactly
these fields:

```json
{
  "candidate_id": "...",
  "candidate_identity": {},
  "source_instance_id": "...",
  "candidate_universe_binding": {
    "path": "...",
    "schema": "...",
    "raw_sha256": "<64 lowercase hex>"
  },
  "relation_binding": {
    "scope": "...",
    "relation": "...",
    "directionality": "...",
    "host_relationship": "...",
    "participant_bindings": [
      {
        "position": 0,
        "role": "...",
        "participant_kind": "...",
        "semantic_ref": "..."
      }
    ]
  },
  "participant_role_bridge": [
    {
      "position": 0,
      "participant_kind": "...",
      "semantic_ref": "...",
      "historical_source_role": "...",
      "reviewed_role": "..."
    }
  ],
  "precondition_attestations": [
    {
      "precondition_id": "...",
      "observed_value": {},
      "evidence_refs": [],
      "equivalence_rationale": "..."
    }
  ],
  "member_evidence_refs": [],
  "member_proof_attestation": {}
}
```

The nested objects use the existing closed `DigestReferenceV1`,
`EvidenceRefV1`, and V1 relation member-proof wire contracts. The CBOR mapping
is exact:

```text
relation_binding
    -> [scope, relation, directionality, host_relationship,
        participant_bindings]

participant_role_bridge
    -> ParticipantRoleBridgeV1

precondition_attestations
    -> precondition_attestations_v1
member_evidence_refs
    -> member_evidence_refs_sorted
member_proof_attestation
    -> member_proof_attestation_v1
```

## Source-binding design

The mechanical tuple is shared only as an internal codec/helper contract:

```text
ReviewAuthoritySourceBindingShape = [
    artifact_role_utf8,
    path_utf8,
    schema_utf8_or_null,
    raw_sha256_bytes_32
]
```

Its JSON projection is:

```json
{
  "artifact_role": "<closed role>",
  "path": "<repository-relative POSIX path>",
  "schema": "<closed schema id>",
  "raw_sha256": "<64 lowercase hex>"
}
```

Existing public contracts are not renamed or reinterpreted:

```text
ContextAuthoritySourceBindingV2 = unchanged public contract
HostBindingSourceBindingV2      = unchanged public contract
RelationAuthoritySourceBindingV2 = new public contract
ContextAuthoritySourceBindingV3 = new public contract for the V3 registry
```

Internal code may share encoding, path, digest, and canonical-order helpers.
Each public authority retains its own versioned type and closed role/path/schema
registry:

```text
RelationAuthority role registry
ContextAuthority role registry
HostBinding role registry
```

The RPA registry must bind the exact model, candidate universe, REV3, B2,
B1.Final, reviewer-roster, and V4 acceptance-event artifacts. It must reject
unknown roles, wrong paths, wrong schemas, duplicate role/path tuples, stale
raw digests, and self-binding of the aggregate authority.

SourceInstance data is resolved through the exact candidate-universe artifact
and source-instance locator; it is never copied into a second ledger.

### RelationAuthoritySourceBindingV2 registry

The complete RPA V2 role registry is:

| role | path grammar | schema |
|---|---|---|
| `base_authority_v1` | `sources/m2_5/authorities/interaction_review_authority.v1.json` | `manafold.m2.5.c.interaction-review-authority.v1` |
| `declared_model` | `sources/m2_5/closures/C/declared_interaction_model.v2.json` | `manafold.m2.5.c.declared-interaction-model.v2` |
| `candidate_universe` | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `manafold.m2.5.c.interaction-candidate-universe.v2` |
| `rev3_candidate_census` | `derived/Pair_Interaction_Census_REV3.csv` | `null` |
| `rev3_pair_aggregates` | `derived/Pair_Requirement_Aggregates_REV3.json` | `null` |
| `rev3_card_requirement_map` | `derived/Card_Requirement_Map_REV3.csv` | `null` |
| `rev3_deck_row_source_resolution` | `inputs/deck_row_source_resolution_REV3.csv` | `null` |
| `rev3_osi_source_records` | `source/raw/oracle_cards_selected_REV3.jsonl` | `null` |
| `rev3_source_index` | `source/raw/source_record_index_REV3.csv` | `null` |
| `b2_catalog` | `sources/m2_5/closures/B2/requirement_family_catalog.v1.json` | `manafold.m2.5.b2.requirement-family-catalog.v1` |
| `b2_classifications` | `sources/m2_5/closures/B2/card_semantic_classifications.v1.json` | `manafold.m2.5.b2.card-semantic-classifications.v1` |
| `b2_closure` | `sources/m2_5/closures/B2/classification_closure.v1.json` | `manafold.m2.5.b2.classification-closure.v1` |
| `b1_final_citations` | `sources/m2_5/closures/B1/official_authority_citations.v3.json` | `manafold.m2.5.b1.official-authority-citations.v3` |
| `b1_final_closure` | `sources/m2_5/closures/B1/official_authority_citation_closure.v2.json` | `manafold.m2.5.b1.official-authority-citation-closure.v2` |
| `reviewer_roster_leaf` | `sources/m2_5/authorities/reviewer_rosters/v1/<64 lowercase hex>.json` | `manafold.m2.5.c.reviewer-roster.v1` |
| `acceptance_event_leaf_v1` | `sources/m2_5/authorities/review_acceptance_events/v1/<64 lowercase hex>.json` | `manafold.m2.5.c.review-acceptance-event.v1` |
| `acceptance_event_leaf_v4` | `sources/m2_5/authorities/review_acceptance_events/v4/<64 lowercase hex>.json` | `manafold.m2.5.c.review-acceptance-event.v4` |

`relation_application_authority_v2` is forbidden in its own registry. Static
roles occur once; content-addressed roles use exact lowercase-hex path
grammar, and every role/path pair is unique.

### ContextAuthoritySourceBindingV3 registry

The complete V3 container registry is the union of the context-side immutable
roles and these composition roles:

| role | path grammar | schema |
|---|---|---|
| `base_authority_v1` | `sources/m2_5/authorities/interaction_review_authority.v1.json` | `manafold.m2.5.c.interaction-review-authority.v1` |
| `declared_model` | `sources/m2_5/closures/C/declared_interaction_model.v2.json` | `manafold.m2.5.c.declared-interaction-model.v2` |
| `candidate_universe` | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `manafold.m2.5.c.interaction-candidate-universe.v2` |
| `rev3_candidate_census` | `derived/Pair_Interaction_Census_REV3.csv` | `null` |
| `rev3_pair_aggregates` | `derived/Pair_Requirement_Aggregates_REV3.json` | `null` |
| `rev3_card_requirement_map` | `derived/Card_Requirement_Map_REV3.csv` | `null` |
| `rev3_deck_row_source_resolution` | `inputs/deck_row_source_resolution_REV3.csv` | `null` |
| `rev3_osi_source_records` | `source/raw/oracle_cards_selected_REV3.jsonl` | `null` |
| `rev3_source_index` | `source/raw/source_record_index_REV3.csv` | `null` |
| `b2_catalog` | `sources/m2_5/closures/B2/requirement_family_catalog.v1.json` | `manafold.m2.5.b2.requirement-family-catalog.v1` |
| `b2_classifications` | `sources/m2_5/closures/B2/card_semantic_classifications.v1.json` | `manafold.m2.5.b2.card-semantic-classifications.v1` |
| `b2_closure` | `sources/m2_5/closures/B2/classification_closure.v1.json` | `manafold.m2.5.b2.classification-closure.v1` |
| `b1_final_citations` | `sources/m2_5/closures/B1/official_authority_citations.v3.json` | `manafold.m2.5.b1.official-authority-citations.v3` |
| `b1_final_closure` | `sources/m2_5/closures/B1/official_authority_citation_closure.v2.json` | `manafold.m2.5.b1.official-authority-citation-closure.v2` |
| `reviewer_roster_leaf` | `sources/m2_5/authorities/reviewer_rosters/v1/<64 lowercase hex>.json` | `manafold.m2.5.c.reviewer-roster.v1` |
| `acceptance_event_leaf_v1` | `sources/m2_5/authorities/review_acceptance_events/v1/<64 lowercase hex>.json` | `manafold.m2.5.c.review-acceptance-event.v1` |
| `acceptance_event_leaf_v2` | `sources/m2_5/authorities/review_acceptance_events/v2/<64 lowercase hex>.json` | `manafold.m2.5.c.review-acceptance-event.v2` |
| `acceptance_event_leaf_v4` | `sources/m2_5/authorities/review_acceptance_events/v4/<64 lowercase hex>.json` | `manafold.m2.5.c.review-acceptance-event.v4` |
| `relation_authority_v2` | `sources/m2_5/authorities/relation_application_authority/v2/relation_application_authority.v2.json` | `manafold.m2.5.c.relation-application-authority.v2` |
| `host_binding_authority_v2` | `sources/m2_5/authorities/interaction_review_authority.v2.json` | `manafold.m2.5.c.interaction-review-authority.v2` |
| `host_binding_claim_record` | `sources/m2_5/authorities/cross_deck_host_binding_claims/v1/<64 lowercase hex>.json` | `manafold.m2.5.c.cross-deck-host-binding-claim-record.v1` |

The raw REV3 roles from the RPA registry are also admitted with null schema
where the V3 source closure requires them. `context_application_authority_v3`
is forbidden as a self-binding role. The V3 container source arrays use the
exact registry appropriate to each public binding type; internal codec reuse
does not merge their public meanings.

## RPA V2 currentness and supersession

RPA V1 and RPA V2 graphs are separate:

```text
rpar.v1 / rps.v1
rpar.v2 / rps.v2 / rpsr.v2
```

A V2 supersession cannot supersede a V1 record, and a V1 supersession cannot
supersede a V2 record.

The V2 supersession preimage is:

```text
RelationApplicationV2SupersessionInputV1 = [
    "manafold.m2.5.c.relation-application-v2-supersession-input.v2",
    superseded_record_id_bytes,
    replacement_record_id_bytes_or_null,
    "relation_application_v2_record",
    replacement_record_kind_or_null,
    reason_code,
    source_evidence_refs_sorted
]
```

The accepted supersession-record preimage is:

```text
RelationApplicationV2SupersessionRecordInputV1 = [
    "manafold.m2.5.c.relation-application-supersession-record-input.v2",
    supersession_id_bytes,
    review_event_ref_v4_cbor
]
```

The exact domains and input schemas are:

```text
rps.v2:
  semantic_domain = manafold.m2.5.c.relation-application-supersession.v2
  input_schema_id = manafold.m2.5.c.relation-application-v2-supersession-input.v2

rpsr.v2:
  semantic_domain = manafold.m2.5.c.relation-application-supersession-record.v2
  input_schema_id = manafold.m2.5.c.relation-application-supersession-record-input.v2
```

### Shared V2/V3 supersession contract

RPA V2 and Context V3 reuse the existing closed `SupersessionReason` vocabulary
without reinterpretation:

```text
semantic_correction
source_revision
model_revision
authority_revocation
```

The replacement rule is exact:

```text
authority_revocation:
    replacement_record_id   = null
    replacement_record_kind = null

semantic_correction | source_revision | model_revision:
    replacement_record_id   != null
    replacement_record_kind = exact same application-record kind
```

`source_evidence_refs` is a non-empty `EvidenceRefV1[]`, sorted by complete
canonical CBOR encoding and duplicate-free. Every reference resolves to the
exact path, locator, schema, and raw digest. Self-edges, unknown endpoints,
competing successors, and supersession cycles fail closed.

The same reason, replacement, evidence, self-edge, and cycle rules apply to
`rps.v2/rpsr.v2` and `cps.v3/cpsr.v3`.

Currentness follows ADR 0043:

- application IDs are currentness groups;
- non-revocation supersession sources are excluded;
- `authority_revocation` revokes the complete semantic application group;
- zero current records is a valid historical/no-current result;
- one current record is current;
- more than one is `CURRENTNESS_AMBIGUOUS`;
- no timestamp, filename, insertion order, or persisted `current` flag is used.

Before RPA V2 live eligibility, the referenced `theorem_record_id` must also
resolve to a current accepted V1 `RelationProofV1` theorem record under the
existing V1 theorem currentness graph. A superseded or revoked `rpr.v1` makes
the application historical/auditable but ineligible for live authority and
returns `SUPERSEDED_AUTHORITY_USED`.

This theorem-currentness check applies both at RPA V2 admission and whenever
the current RPA read model is recomputed. Historical RPA records remain
readable and verifiable.

## RelationApplicationAuthorityV2 aggregate

The RPA V2 authority artifact referenced by Context V3 is itself a closed
normative container:

```json
{
  "schema": "manafold.m2.5.c.relation-application-authority.v2",
  "base_authority_v1_binding": {},
  "candidate_universe_binding": {},
  "source_bindings": [],
  "relation_application_v2_records": [],
  "relation_application_v2_supersession_records": []
}
```

The fields have these exact meanings:

```text
base_authority_v1_binding:
    exactly one RelationAuthoritySourceBindingV2 whose role is
    base_authority_v1

candidate_universe_binding:
    exactly one RelationAuthoritySourceBindingV2 whose role is
    candidate_universe

source_bindings:
    the complete, exact source set used by all contained V2 records,
    supersession records, theorem references, member evidence, and V4 events

relation_application_v2_records:
    immutable accepted rpar.v2 records

relation_application_v2_supersession_records:
    immutable accepted rpsr.v2 records
```

The two top-level binding fields are exact projections of unique entries in
`source_bindings`; they are not second occurrences:

```text
base_authority_v1_binding
    = exact source_bindings entry where artifact_role = base_authority_v1

candidate_universe_binding
    = exact source_bindings entry where artifact_role = candidate_universe
```

Projection equality is exact across:

```text
artifact_role
path
schema
raw_sha256
```

Missing projections, duplicate role/path entries, or a projection with a
different field or digest fail closed.

The aggregate has no semantic self-digest and may not bind its own path in
`source_bindings`. Its raw bytes are bound by a V3 container or later C source
closure through the exact `relation_authority_v2` artifact binding.

Canonical collection rules:

- `source_bindings` are sorted by complete canonical CBOR encoding and are
  duplicate-free by both complete tuple and `(artifact_role, path)`;
- application records are sorted by complete canonical CBOR encoding and have
  unique `rpar.v2` record IDs;
- supersession records are sorted by complete canonical CBOR encoding and have
  unique `rpsr.v2` record IDs;
- every record identity is recomputed before it enters currentness;
- every referenced theorem resolves to the exact bound V1 authority;
- every referenced candidate/source instance resolves to the exact candidate
  universe binding;
- no ContextAuthority or HostBinding role is admitted into this container's
  source registry;
- missing, extra, stale, or substituted source bindings fail closed.

The exact container source closure is:

```text
ExpectedRelationApplicationAuthorityV2SourceClosure(authority) =
    {base_authority_v1, candidate_universe, declared_model,
     reviewer_roster_leaf}
    union theorem/application/member evidence sources
    union every referenced V4 acceptance-event leaf
    union every referenced V4 event's exact immutable source closure
    union all required B1/B2/REV3 sources
    minus {relation_application_authority_v2}
```

The reconstructed set must equal `source_bindings` exactly. The container
does not select current records by file order or filename; currentness is the
separate ADR 0043-style derived graph described above.

## RelationApplicationV1 compatibility

Existing V1 behavior remains unchanged:

- V1 readers and writers do not gain a bridge field;
- V1 validators retain full participant-binding equality;
- V1 diagnostics retain their existing meanings;
- no V1 identity is rewritten or upgraded in place;
- no V2 record supersedes a V1 record;
- exact-role members remain exclusively on the V1 compatibility path;
- any role-divergent member requires a current accepted RPA V2 application;
- there is no uniform exact-role V1-to-V2 migration path;
- an exact-role member cannot be represented by both current V1 and current V2
  application authority.

## ContextApplicationV2 and ContextApplicationV3

`ContextApplicationV2` is completely frozen:

```text
cpa.v2 identity                  unchanged
cpar.v2 identity                 unchanged
cps.v2 / cpsr.v2                 unchanged
ContextApplicationMemberV2       unchanged
ae.v3 acceptance contract        unchanged
existing V1 source validator     unchanged
```

There is no `ContextApplicationV2RoleQualifiedProfile`. No alternate validator
profile may make an otherwise invalid `cpa.v2` valid.

Role-divergent reviewed context semantics use a new semantic application:

```text
ContextApplicationV3
    new versioned owner of reviewed context application
    explicitly binds current RelationApplicationV2 authority
```

### ContextApplicationMemberV3

The V3 member is intentionally close to the V2 member, but the RPA binding is
part of the new versioned contract:

```text
ContextApplicationMemberV3 = [
    candidate_id_utf8,
    candidate_identity_digest_reference_v1,
    source_instance_id_utf8,
    candidate_universe_binding_v1,
    reviewed_context_binding_v1,
    relation_application_v2_id_bytes_32,
    precondition_attestations_v1,
    member_evidence_refs_sorted,
    context_member_bridge_attestation_v2
]
```

Every V3 member is role-divergent and therefore carries a non-null exact
current `rpa.v2` ID. Exact-role members are not valid V3 members:

```text
exact-role member
    -> ContextApplicationV2 only

role-divergent member
    -> ContextApplicationV3 only
```

`ContextApplicationV3` may not mix exact-role and divergent-role members. If a
reusable ContextProof applies to both populations, its application member sets
are split into separate V2 and V3 applications.

The wire projection may render the field as:

```json
{
  "relation_application_v2_id": "rpa.v2/<64 lowercase hex>"
}
```

Wire decoding is exact:

```text
"rpa.v2/<64 lowercase hex>"
    -> AuthorityIdentityKind::RelationApplicationV2
    -> digest_bytes_32
```

The canonical CBOR preimage contains only `relation_application_v2_id_bytes_32`.
For the required RPA binding, the validator requires:

```text
position
participant_kind
semantic_ref
    == SourceInstanceV1 and candidate

reviewed role
    == ContextProofV1 subject role
    == current RelationApplicationV2 reviewed role

historical role
    == SourceInstanceV1 role
    == RelationApplicationV2 bridge historical role
```

### V3 precondition resolver

`ContextApplicationMemberV3` retains the closed V1 precondition vocabulary but
does not reuse the V1/V2 source validator as a whole. V3 uses this exact
versioned resolver:

```text
candidate_relation_shape
    -> exact candidate scope, relation, arity, and directionality
    -> exact V3 reviewed host relationship

participant_binding
    -> exact theorem expectation equals
       [position, historical SourceInstanceV1 role,
        participant_kind, semantic_ref]
    -> the existing V1 exact source-participant comparison is preserved
    -> ParticipantRoleBridgeV1 additionally proves historical role ->
       reviewed role for the V3 semantic binding
    -> a theorem expectation containing reviewed source/affected roles
       against an ordered historical SourceInstance fails closed; it is not
       repaired by V3 and requires a future versioned theorem contract

b2_boundary
    -> existing exact theorem-side B2 boundary resolution

source_context
    -> exact historical SourceInstanceV1 source_context value
    -> reviewed divergence is owned by ContextMemberBridgeAttestationV2
    -> no direct historical-value == reviewed-value requirement

temporal_semantic
    -> exact theorem/member attestation behavior from the existing V1/V2
       temporal contract

class_projection
    -> ContextProofV1 expected projection
       == linked current RPA V2 member's reviewed class projection/member
          projection
    -> no ContextApplicationV3 member_proof_attestation is required
```

All six V1 precondition kinds remain closed and exact. V3 does not add a
wildcard, exception, ignored-role, or fallback variant. In particular, V3 does
not reinterpret the V1 `participant_binding` meaning. The linked RPA V2
member's own relation proof/member-proof validation remains authoritative for
the reviewed relation projection; Context V3 consumes that result rather than
changing the V1 precondition fact.

The bridge is not copied into V3. V3 consumes the exact current RPA V2
application identity that owns it.

### ContextApplicationMemberV3 wire shape

The JSON/Wire object is closed (`additionalProperties = false`) and has exactly
these fields:

```json
{
  "candidate_id": "...",
  "candidate_identity": {},
  "source_instance_id": "...",
  "candidate_universe_binding": {
    "path": "...",
    "schema": "...",
    "raw_sha256": "<64 lowercase hex>"
  },
  "context_binding": {
    "arity": "...",
    "directionality": "...",
    "participant_roles": [
      {
        "position": 0,
        "role": "...",
        "participant_kind": "...",
        "semantic_ref": "..."
      }
    ],
    "host_relationship": "..."
  },
  "relation_application_v2_id": "rpa.v2/<64 lowercase hex>",
  "precondition_attestations": [
    {
      "precondition_id": "...",
      "observed_value": {},
      "evidence_refs": [],
      "equivalence_rationale": "..."
    }
  ],
  "member_evidence_refs": [],
  "context_member_attestation": {
    "context_slot_attestations": [],
    "temporal_slot_attestations": []
  }
}
```

The nested candidate, evidence, precondition, and context-attestation objects
use their existing closed V1/V2 wire contracts. The CBOR mapping is exact:

```text
context_binding
    -> reviewed_context_binding_v1

relation_application_v2_id
    -> relation_application_v2_id_bytes_32

precondition_attestations
    -> precondition_attestations_v1
member_evidence_refs
    -> member_evidence_refs_sorted
context_member_attestation
    -> context_member_bridge_attestation_v2
```

### ContextApplicationV3 identity

```text
ContextApplicationV3InputV1 = [
    "manafold.m2.5.c.context-application-input.v3",
    theorem_record_id_bytes,
    members_v3_sorted
]
```

`members_v3_sorted` is finite, non-empty, duplicate-free, and canonically
ordered by exactly:

```text
[candidate_identity_digest_reference_v1.digest_bytes_32,
 UTF8(source_instance_id)]
```

The same key is used for duplicate detection. Member insertion order, JSON
object order, theorem order, and source-file order do not affect `cpa.v3`.

Identity:

```text
cpa.v3/<digest>
semantic_domain = manafold.m2.5.c.context-application.v3
input_schema_id = manafold.m2.5.c.context-application-input.v3
```

The identity kind is the closed proposed
`AuthorityIdentityKind::ContextApplicationV3`; its digest bytes are the only
semantic ID payload reused by `ApplicationHostBindingV3`.

The accepted record is:

```text
ContextApplicationV3RecordInputV1 = [
    "manafold.m2.5.c.context-application-record-input.v3",
    context_application_v3_id_bytes,
    review_event_ref_v4_cbor
]
```

Accepted-record identity:

```text
cpar.v3/<digest>
semantic_domain = manafold.m2.5.c.context-application-record.v3
input_schema_id = manafold.m2.5.c.context-application-record-input.v3
```

Supersession uses separate V3 identities:

```text
cps.v3/<digest>
cpsr.v3/<digest>
```

Their exact semantic preimage is:

```text
ContextApplicationV3SupersessionInputV1 = [
    "manafold.m2.5.c.context-application-v3-supersession-input.v3",
    superseded_record_id_bytes,
    replacement_record_id_bytes_or_null,
    "context_application_v3_record",
    replacement_record_kind_or_null,
    reason_code,
    source_evidence_refs_sorted
]
```

The accepted supersession-record preimage is:

```text
ContextApplicationV3SupersessionRecordInputV1 = [
    "manafold.m2.5.c.context-application-v3-supersession-record-input.v3",
    supersession_id_bytes,
    review_event_ref_v4_cbor
]
```

The exact domains and input schemas are:

```text
cps.v3:
  semantic_domain = manafold.m2.5.c.context-application-supersession.v3
  input_schema_id = manafold.m2.5.c.context-application-v3-supersession-input.v3

cpsr.v3:
  semantic_domain = manafold.m2.5.c.context-application-supersession-record.v3
  input_schema_id = manafold.m2.5.c.context-application-v3-supersession-record-input.v3
```

They follow ADR 0043's record-ID lineage and application-ID currentness model.
V3 supersession records cannot supersede V2 records, and V2 supersession
records cannot supersede V3 records. Revocation is application-wide within the
V3 `cpa.v3` application-ID group.

### ContextApplicationAuthorityV3

The normative V3 authority carries V3 records and is required whenever any
V3 record, V3 supersession record, or V3 HostBinding composition exists. It is
not an optional cache or read-model convenience:

```json
{
  "schema": "manafold.m2.5.c.context-application-authority.v3",
  "base_authority_v1_binding": {},
  "candidate_universe_binding": {},
  "source_bindings": [],
  "relation_application_authority_v2_binding": {},
  "relation_source_bindings": [],
  "host_binding_authority_v2_binding": null,
  "host_binding_source_bindings": [],
  "context_application_v3_records": [],
  "context_application_v3_supersession_records": [],
  "application_host_bindings_v3": []
}
```

The top-level binding fields are typed and are exact projections of unique
entries in `source_bindings`; they are not second occurrences:

```text
base_authority_v1_binding
    = ContextAuthoritySourceBindingV3(role = base_authority_v1)

candidate_universe_binding
    = ContextAuthoritySourceBindingV3(role = candidate_universe)

relation_application_authority_v2_binding
    = ContextAuthoritySourceBindingV3(role = relation_authority_v2)

host_binding_authority_v2_binding
    = ContextAuthoritySourceBindingV3(role = host_binding_authority_v2)
      | null
```

The projected tuple is equal in all four fields:

```text
artifact_role
path
schema
raw_sha256
```

Missing projections, duplicate role/path entries, or a projection that differs
from its unique source-binding entry fail closed.

The normative artifact path and schema are:

```text
path:
  sources/m2_5/authorities/context_application_authority/v3/
    context_application_authority.v3.json

schema:
  manafold.m2.5.c.context-application-authority.v3
```

`host_binding_authority_v2_binding` is a `ContextAuthoritySourceBindingV3`
projection of the exact ADR 0044 HostBinding authority artifact. Its
nullability is closed:

```text
any current or historical V3 HostBinding link exists
    -> host_binding_authority_v2_binding is required

no V3 HostBinding link and no required V3 member exists
    -> host_binding_authority_v2_binding must be absent
```

An authority binding with no applicable link is rejected. A link without the
authority binding is rejected. The binding is container-level provenance; it
does not enter the V4 acceptance-event closure and does not change HBC
semantics. When the binding is absent, `host_binding_source_bindings` must be
an empty array.

It does not contain or reinterpret the V2 authority aggregate. Existing V2
records remain historical V2 records. V3 has its own currentness graph and
does not share record IDs with V2.

`source_bindings` is the complete context-side V3 source set and uses the
public `ContextAuthoritySourceBindingV3` type. `relation_source_bindings` is
the exact source set required to resolve the referenced RPA V2 authority and
its current application/event closure and uses the public
`RelationAuthoritySourceBindingV2` type. `host_binding_source_bindings` is a
`HostBindingSourceBindingV2[]` using the existing public HostBinding
provenance contract; no Context-to-Host source-binding projection is used.
All three arrays are sorted by complete canonical CBOR encoding,
duplicate-free, and compared against reconstructed closure sets. The V3
aggregate cannot bind itself, and no HostBinding claim is accepted as
relation-role evidence.

The remaining V3 authority arrays are canonical collections:

```text
context_application_v3_records:
    sorted by complete canonical CBOR encoding
    duplicate-free by cpar.v3 record ID

context_application_v3_supersession_records:
    sorted by complete canonical CBOR encoding
    duplicate-free by cpsr.v3 record ID

application_host_bindings_v3:
    sorted by complete canonical CBOR encoding
    duplicate-free by complete link identity
```

Every contained identity is recomputed before collection ordering and
currentness evaluation. Input/file order never selects a record or link.

The complete normative container closure is:

```text
ExpectedContextApplicationAuthorityV3SourceClosure(authority) =
    {base_authority_v1, candidate_universe, declared_model,
     required REV3, B1, B2, reviewer_roster_leaf}
    union every referenced V4 acceptance-event leaf
    union every referenced V4 event's exact immutable source closure
    union {relation_authority_v2}
    union exact relation_source_bindings
    union, when any V3 HostBinding link exists,
        {host_binding_authority_v2}
        union exact host_binding_source_bindings
        union every referenced host-binding claim-record/source closure
    minus {context_application_authority_v3}
```

`source_bindings`, `relation_source_bindings`, and
`host_binding_source_bindings` must each equal their independently
reconstructed expected set. In particular, every referenced
`host_binding_claim_record` is present in `host_binding_source_bindings`; a
claim ID alone is not provenance. HostBinding sources are container-level and
are never added to a V4 acceptance-event closure.

### Context/RPA shared-snapshot invariant

Before any V3 member or HostBinding composition is evaluated, the V3 and RPA
authority bindings must agree on every shared immutable snapshot:

```text
ContextApplicationAuthorityV3.base_authority_v1_binding
    == RelationApplicationAuthorityV2.base_authority_v1_binding

ContextApplicationAuthorityV3.candidate_universe_binding
    == RelationApplicationAuthorityV2.candidate_universe_binding
```

For any model, REV3, B1, or B2 artifact required by both authorities, the
complete binding tuple must also be equal:

```text
artifact_role
path
schema
raw_sha256
```

There is no compatible rebase, recency substitution, semantic-equivalence
substitution, or filename-based repair. A mismatch fails before V3 member,
RPA, or HostBinding evaluation.

The closed eligibility resolver is:

```text
historical roles == reviewed roles
    -> exactly one current ContextApplicationV2 path
    -> no current ContextApplicationV3 path

historical roles != reviewed roles
    -> exactly one current ContextApplicationV3 path
    -> current RelationApplicationV2 binding required
    -> no current ContextApplicationV2 path
```

No timestamp, version preference, or "latest wins" rule resolves a collision.
An ambiguous or duplicated cross-version result fails closed.

Before Context V3 live eligibility, the referenced `theorem_record_id` must
resolve to a current accepted V1 `ContextProofV1` theorem record under the
existing V1 theorem currentness graph. A superseded or revoked `cpr.v1` makes
the Context V3 record historical/auditable but ineligible for live authority
and returns `SUPERSEDED_AUTHORITY_USED`. This check applies at V3 admission and
on every later eligibility recomputation.

### ApplicationHostBindingV3

The existing HostBinding claims and currentness remain unchanged:

```text
hbc.v1 / hbcr.v1 / hbcs.v1 = unchanged
ApplicationHostBindingV2   = cpa.v2 composition, unchanged
```

V3 adds only a new composition link:

```text
ApplicationHostBindingV3 = [
    "context_application_v3",
    context_application_v3_id_bytes_32,
    host_binding_claim_ids_sorted
]
```

Wire form:

```json
{
  "application_kind": "context_application_v3",
  "application_semantic_id": "cpa.v3/<64 lowercase hex>",
  "host_binding_claim_ids": ["hbc.v1/<64 lowercase hex>"]
}
```

The wire string is not the CBOR identity representation. Decode is exact:

```text
"cpa.v3/<64 lowercase hex>"
    -> AuthorityIdentityKind::ContextApplicationV3
    -> digest_bytes_32
```

The closed application kind and the reconstructed identity kind must agree.
The canonical preimage contains only the 32 digest bytes, never a free-form
UTF-8 identity string.

The V3 link reuses existing current `hbc.v1` claims only when every exact
member key and observed host relationship matches. It creates no new host
semantics, host claim identity, or host currentness rule. ADR 0044's required
member predicate, source closure, snapshot equality, and historical-link
policy apply unchanged.

The existing ADR 0044 HostBinding diagnostic taxonomy is reused with the V3
application kind; a second host-semantic error vocabulary is not introduced.

HostBinding composition remains a separate read model and source closure. A
HostBinding claim cannot satisfy the RPA binding, and an RPA binding cannot
satisfy a HostBinding requirement.

## Candidate 4

Using the exact pilot bytes:

```text
SourceInstanceV1:
  0 ordered_participant / requirement_family / cap.mass_destruction
  1 ordered_participant / requirement_family / cap.death_trigger

RelationProofV1:
  0 source   / requirement_family / cap.mass_destruction
  1 affected / requirement_family / cap.death_trigger

RelationApplicationV2:
  0 ordered_participant -> source
  1 ordered_participant -> affected

ContextApplicationV3:
  exact reviewed source/affected context binding
  exact current rpa.v2 application ID for the divergent member
  historical roles checked through the RPA V2 bridge

HostBinding:
  ApplicationHostBindingV3 composition to existing current HBC claims,
  if required by ADR 0044
```

`ContextApplicationV2` remains historical and is not used as an invalid
intermediate object. The pilot identity is not substituted with the separate
`cap.linked_death_trigger` candidate family.

## Frozen V4 acceptance contract

`ae.v3` is not extended. Its subject vocabulary remains closed to the ADR 0042
ContextApplicationV2 subjects.

V4 is frozen for both new versioned authority families:

```text
AcceptanceSubjectKindV4 =
    relation_application_v2_record
    relation_application_v2_supersession_record
    context_application_v3_record
    context_application_v3_supersession_record
```

The exact contract identifiers are:

```text
acceptance subject schema:
  manafold.m2.5.c.acceptance-subject-payload.v4

acceptance subject input schema:
  manafold.m2.5.c.acceptance-subject-payload-input.v4

acceptance event schema:
  manafold.m2.5.c.review-acceptance-event.v4

acceptance event input schema:
  manafold.m2.5.c.review-acceptance-event-input.v4

checklist:
  interaction-authority-review-checklist.v3
```

### Acceptance subject payload

```text
AcceptanceSubjectPayloadV4InputV1 = [
    "manafold.m2.5.c.acceptance-subject-payload-input.v4",
    subject_kind,
    subject_payload_without_acceptance_metadata
]
```

The subject payload for an RPA V2 record is:

```text
RelationApplicationV2RecordSubjectPayloadV1 = [
    "relation_application_v2_record",
    application_id_bytes,
    theorem_record_id_bytes,
    terminal_disposition,
    members_v2_sorted
]
```

The subject payload for a ContextApplicationV3 record is:

```text
ContextApplicationV3RecordSubjectPayloadV1 = [
    "context_application_v3_record",
    application_id_bytes,
    theorem_record_id_bytes,
    members_v3_sorted
]
```

The subject payload for either supersession kind is:

```text
SupersessionRecordSubjectPayloadV4 = [
    subject_kind,
    supersession_id_bytes,
    superseded_record_id_bytes,
    replacement_record_id_bytes_or_null,
    superseded_record_kind,
    replacement_record_kind_or_null,
    reason_code,
    source_evidence_refs_sorted
]
```

The subject identity is:

```text
asp.v4/<digest>
semantic_domain = manafold.m2.5.c.acceptance-subject-payload.v4
input_schema_id = manafold.m2.5.c.acceptance-subject-payload-input.v4
```

### ReviewAcceptanceEventV4

The exact event preimage is:

```text
ReviewAcceptanceEventInputV4V1 = [
    "manafold.m2.5.c.review-acceptance-event-input.v4",
    subject_kind,
    subject_payload_digest_reference_v1,
    "human_accepted",
    reviewer_roster_ref_v1,
    reviewer_role_bindings_sorted,
    review_mode,
    "interaction-authority-review-checklist.v3",
    source_binding_digests_sorted: ReviewAuthoritySourceBindingV4[],
    review_evidence_refs_sorted: AcceptanceEvidenceRefV1[]
]
```

`reviewer_role_bindings_sorted` is an existing
`ReviewerRoleBindingV1[]` without semantic reinterpretation:

```text
ReviewerRoleBindingV1 = [reviewer_id_utf8, roles_sorted]
```

Each binding uses the existing V1 contract: roles are canonical-sorted and
duplicate-free. The outer collection is sorted lexicographically by
`reviewer_id` and is duplicate-free by reviewer ID.

The event identity is:

```text
ae.v4/<digest>
semantic_domain = manafold.m2.5.c.review-acceptance-event.v4
input_schema_id = manafold.m2.5.c.review-acceptance-event-input.v4
```

The persisted V4 event leaf is a closed JSON object with exactly these fields:

```json
{
  "event_id": "ae.v4/<64 lowercase hex>",
  "schema": "manafold.m2.5.c.review-acceptance-event.v4",
  "subject_kind": "<AcceptanceSubjectKindV4>",
  "subject_payload_digest": {},
  "decision": "human_accepted",
  "reviewer_roster_ref": {},
  "reviewer_role_bindings": [],
  "review_mode": "multi_reviewer",
  "checklist_id": "interaction-authority-review-checklist.v3",
  "source_binding_digests": [],
  "review_evidence_refs": []
}
```

Unknown fields, missing fields, wrong schema, wrong checklist, duplicate
reviewer IDs, noncanonical collections, and any decision other than
`human_accepted` fail before semantic admission. `subject_payload_digest` is
the complete `DigestReferenceV1` for the exact V4 subject preimage.

`review_evidence_refs_sorted` is non-empty and uses the existing
`AcceptanceEvidenceRefV1` contract without reinterpretation:

```text
AcceptanceEvidenceRefV1 = [
    path_utf8,
    raw_sha256_bytes_32,
    locator_v1
]

locator_v1 =
    ["whole_artifact", null]
    | ["json_pointer", json_pointer_utf8]
    | ["archive_member", member_path_utf8]
```

The wire projection is the existing closed object with exactly `path`,
`raw_sha256`, and `locator`. The collection is sorted by complete canonical
CBOR encoding and duplicate-free. Every locator, path, and raw digest is
resolved exactly; evidence absence, an empty collection, or an unresolved
locator fails closed.

The event leaf path is derived only from the event digest:

```text
sources/m2_5/authorities/review_acceptance_events/v4/<event_digest_hex>.json
```

The exact `ReviewEventRefV4` shape is:

```text
ReviewEventRefV4 = [
    path_utf8,
    raw_sha256_bytes_32,
    ["event_id", event_id_utf8]
]
```

Its wire form is:

```json
{
  "event_id": "ae.v4/<64 lowercase hex>",
  "path": "sources/m2_5/authorities/review_acceptance_events/v4/<64 lowercase hex>.json",
  "raw_sha256": "<64 lowercase hex>"
}
```

The event semantic digest and raw file digest are independently validated.
The path basename must equal the event digest component; the parsed leaf event
ID and recomputed event identity must match the reference.

### V4 review rules

The V4 checklist requires:

```text
architecture_maintainer
rules_authority_maintainer
conformance_maintainer
information_safety_reviewer
```

All four roles are mandatory for every V4 subject. This is a closed rule; it
does not depend on a diagnostic information-sensitivity predicate. Reviewer
IDs and roles must resolve exactly through the immutable reviewer roster.
`review_mode` is closed to `multi_reviewer` and
`solo_separate_self_review`; solo mode does not waive any required role.

The checklist requires explicit review of:

- every historical and reviewed participant role;
- every `ParticipantRoleBridgeV1` entry;
- exact source/candidate/member identity;
- the V1/V2/V3 eligibility partition;
- RPA V2 and Context V3 currentness dependencies;
- exact supersession/revocation rules;
- exact source closure and raw digests;
- no HostBinding-derived role inference.

### V4 source closure

`ReviewAcceptanceEventV4.source_binding_digests` uses its own closed public
type. It does not use the broader RelationAuthoritySourceBindingV2 or
ContextAuthoritySourceBindingV3 public types:

```text
ReviewAuthoritySourceBindingV4 = [
    artifact_role_utf8,
    path_utf8,
    schema_utf8_or_null,
    raw_sha256_bytes_32
]
```

Its JSON projection is the exact four-field object already used by the V4
event schema. The event source-binding tuple uses this mechanical shape; only
the V4 role registry is new. The closed registry contains the existing
model, candidate, REV3, B2, B1.Final, and reviewer-roster roles plus:

```text
acceptance_event_leaf_v1
acceptance_event_leaf_v4
```

The exact static role/path/schema entries are:

| role | path | schema |
|---|---|---|
| `declared_model` | `sources/m2_5/closures/C/declared_interaction_model.v2.json` | `manafold.m2.5.c.declared-interaction-model.v2` |
| `candidate_universe` | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `manafold.m2.5.c.interaction-candidate-universe.v2` |
| `rev3_candidate_census` | `derived/Pair_Interaction_Census_REV3.csv` | `null` |
| `rev3_deck_row_source_resolution` | `inputs/deck_row_source_resolution_REV3.csv` | `null` |
| `rev3_osi_source_records` | `source/raw/oracle_cards_selected_REV3.jsonl` | `null` |
| `rev3_source_index` | `source/raw/source_record_index_REV3.csv` | `null` |
| `b2_catalog` | `sources/m2_5/closures/B2/requirement_family_catalog.v1.json` | `manafold.m2.5.b2.requirement-family-catalog.v1` |
| `b2_classifications` | `sources/m2_5/closures/B2/card_semantic_classifications.v1.json` | `manafold.m2.5.b2.card-semantic-classifications.v1` |
| `b2_closure` | `sources/m2_5/closures/B2/classification_closure.v1.json` | `manafold.m2.5.b2.classification-closure.v1` |
| `b1_final_citations` | `sources/m2_5/closures/B1/official_authority_citations.v3.json` | `manafold.m2.5.b1.official-authority-citations.v3` |
| `b1_final_closure` | `sources/m2_5/closures/B1/official_authority_citation_closure.v2.json` | `manafold.m2.5.b1.official-authority-citation-closure.v2` |
| `reviewer_roster_leaf` | `sources/m2_5/authorities/reviewer_rosters/v1/<64 lowercase hex>.json` | `manafold.m2.5.c.reviewer-roster.v1` |
| `acceptance_event_leaf_v1` | `sources/m2_5/authorities/review_acceptance_events/v1/<64 lowercase hex>.json` | `manafold.m2.5.c.review-acceptance-event.v1` |
| `acceptance_event_leaf_v4` | `sources/m2_5/authorities/review_acceptance_events/v4/<64 lowercase hex>.json` | `manafold.m2.5.c.review-acceptance-event.v4` |

The four raw REV3 roles above are the complete V4 raw-REV3 vocabulary. Each
has `schema = null`; no other raw REV3 role or path is admitted. The mutable
`relation_application_authority_v2` aggregate is not an event source binding;
its current composition belongs to the V3 container.

### V1 dependency source projection

An immutable V1 event closure is projected into the V4 event-binding type by a
closed total function:

```text
V1DependencySourceBindingToV4(SourceBindingDigestV1 binding):

if binding.artifact_role == "rev3_source":
    derived/Pair_Interaction_Census_REV3.csv
        -> rev3_candidate_census
    inputs/deck_row_source_resolution_REV3.csv
        -> rev3_deck_row_source_resolution
    source/raw/oracle_cards_selected_REV3.jsonl
        -> rev3_osi_source_records
    source/raw/source_record_index_REV3.csv
        -> rev3_source_index

otherwise:
    exact same artifact_role, path, schema, and raw_sha256
    when that tuple exists in the closed V4 Event Registry

otherwise:
    fail closed
```

The projected V4 tuple retains the exact path, schema/nullability, and raw
digest. This is a representation projection only; it does not modify the V1
event, V1 evidence, or V1 authority meaning. A V1 dependency with no exact V4
projection cannot enter an `ae.v4` source closure.

The following are forbidden in every V4 event closure:

```text
the event's own leaf
context_application_authority_v3
host_binding_authority_v2
host_binding_claim_record
```

`acceptance_event_leaf_v1` and `acceptance_event_leaf_v4` may reference
dependency event leaves only. The event currently being validated may never
appear in its own
`source_binding_digests`, even when its path and raw digest are otherwise
valid.

For an RPA V2 subject, the exact closure is reconstructed from the immutable
V1 acceptance-event leaf for every referenced V1 theorem/application
dependency, that leaf's exact source closure, the application/member evidence,
candidate universe, model, reviewer roster, and required B1/B2 sources. The
mutable V1 authority aggregate does not bind itself or enter this event
closure.

For a Context V3 subject, the closure additionally contains the immutable,
content-addressed RPA V2 V4 acceptance-event leaf for every divergent member
and that dependency event's exact source closure. The selected RPA event leaf
is defined by currentness at the time of Context V3 acceptance: resolve the
unique current `rpar.v2` record for the member's semantic `rpa.v2` ID, then
bind exactly that record's `ReviewEventRefV4` leaf. Zero or ambiguous current
RPA records block Context V3 acceptance. A later accepted revision of the same
`rpa.v2` semantic application does not rewrite this historical event closure;
live V3 eligibility still reevaluates the current RPA graph. The mutable RPA
aggregate, Context V3 aggregate, and HostBinding provenance remain
container-level sources, not event sources.

Every ContextApplicationV3 record subject first binds the immutable V1
acceptance-event leaf and exact source closure of its referenced `ContextProofV1`
`cpr.v1` record. The linked current RPA V2 event dependency is additional to
that theorem dependency. A missing or ambiguous `cpr.v1` acceptance dependency
blocks Context V3 acceptance.

For V4 supersession subjects, endpoint provenance is explicit:

```text
relation_application_v2_supersession_record:
    source_evidence_refs
    + exact ReviewEventRefV4 leaf of superseded rpar.v2
    + exact immutable source closure of that event
    + when replacement_record_id != null:
        exact ReviewEventRefV4 leaf of replacement rpar.v2
        + exact immutable source closure of that event

context_application_v3_supersession_record:
    source_evidence_refs
    + exact ReviewEventRefV4 leaf of superseded cpar.v3
    + exact immutable source closure of that event
    + when replacement_record_id != null:
        exact ReviewEventRefV4 leaf of replacement cpar.v3
        + exact immutable source closure of that event
```

For `authority_revocation`, the replacement endpoint is null and only the
superseded endpoint closure is required. No supersession event binds a mutable
RPA or Context aggregate.

Extras and omissions fail closed. Source bindings are canonically ordered by
complete CBOR encoding and duplicate-free.

## Persisted V2/V3 record wire shapes

The four new accepted-record families are closed JSON objects. Their identity
fields are rendered as namespace-qualified strings, while their CBOR identity
preimages use the exact digest bytes defined above.

```json
RelationApplicationV2Record = {
  "record_id": "rpar.v2/<64 lowercase hex>",
  "application_id": "rpa.v2/<64 lowercase hex>",
  "theorem_record_id": "rpr.v1/<64 lowercase hex>",
  "terminal_disposition": "<closed disposition>",
  "members": [],
  "acceptance": {
    "decision": "human_accepted",
    "review_event_ref": {}
  }
}
```

```json
RelationApplicationV2SupersessionRecord = {
  "record_id": "rpsr.v2/<64 lowercase hex>",
  "supersession_id": "rps.v2/<64 lowercase hex>",
  "superseded_record_id": "rpar.v2/<64 lowercase hex>",
  "replacement_record_id": "rpar.v2/<64 lowercase hex>" | null,
  "superseded_record_kind": "relation_application_v2_record",
  "replacement_record_kind": "relation_application_v2_record" | null,
  "reason_code": "<closed supersession reason>",
  "source_evidence_refs": [],
  "acceptance": {
    "decision": "human_accepted",
    "review_event_ref": {}
  }
}
```

```json
ContextApplicationV3Record = {
  "record_id": "cpar.v3/<64 lowercase hex>",
  "application_id": "cpa.v3/<64 lowercase hex>",
  "theorem_record_id": "cpr.v1/<64 lowercase hex>",
  "members": [],
  "acceptance": {
    "decision": "human_accepted",
    "review_event_ref": {}
  }
}
```

```json
ContextApplicationV3SupersessionRecord = {
  "record_id": "cpsr.v3/<64 lowercase hex>",
  "supersession_id": "cps.v3/<64 lowercase hex>",
  "superseded_record_id": "cpar.v3/<64 lowercase hex>",
  "replacement_record_id": "cpar.v3/<64 lowercase hex>" | null,
  "superseded_record_kind": "context_application_v3_record",
  "replacement_record_kind": "context_application_v3_record" | null,
  "reason_code": "<closed supersession reason>",
  "source_evidence_refs": [],
  "acceptance": {
    "decision": "human_accepted",
    "review_event_ref": {}
  }
}
```

Each object is closed; arrays use the canonical order specified by its
corresponding CBOR contract; record, application, and supersession IDs are
recomputed and prefix/kind checked before admission. `review_event_ref` is
the exact V4 reference shape above.

## Stable failure categories

```text
ROLE_BRIDGE_SHAPE_INVALID
ROLE_BRIDGE_NONCANONICAL
ROLE_BRIDGE_POSITION_MISMATCH
ROLE_BRIDGE_PARTICIPANT_MISMATCH
ROLE_BRIDGE_HISTORICAL_ROLE_MISMATCH
ROLE_BRIDGE_REVIEWED_ROLE_MISMATCH
ROLE_BRIDGE_MISSING
ROLE_BRIDGE_DUPLICATE

RELATION_APPLICATION_V2_IDENTITY_MISMATCH
RELATION_APPLICATION_V2_THEOREM_MISMATCH
RELATION_APPLICATION_V2_SOURCE_MISMATCH
RELATION_APPLICATION_V2_CURRENTNESS_FAILED
RELATION_APPLICATION_V2_CURRENTNESS_AMBIGUOUS
RELATION_APPLICATION_V2_SOURCE_CLOSURE_MISMATCH
RELATION_APPLICATION_V2_NOT_DIVERGENT
SUPERSEDED_AUTHORITY_USED

CONTEXT_AUTHORITY_VERSION_ELIGIBILITY_MISMATCH
CONTEXT_AUTHORITY_VERSION_AMBIGUOUS

CONTEXT_APPLICATION_V3_INPUT_INVALID
CONTEXT_APPLICATION_V3_IDENTITY_MISMATCH
CONTEXT_APPLICATION_V3_SOURCE_MISMATCH
CONTEXT_APPLICATION_V3_RPA_REQUIRED
CONTEXT_APPLICATION_V3_RPA_NOT_CURRENT
CONTEXT_APPLICATION_V3_MEMBER_MISMATCH
CONTEXT_APPLICATION_V3_ROLE_MISMATCH
CONTEXT_APPLICATION_V3_SOURCE_CLOSURE_MISMATCH
```

Existing V1 errors retain their existing meanings. No diagnostic is selected
by parsing exception text, and no fallback uses position, host, or capability
names.

## Required tests

Positive cases:

- existing V1 exact-role fixture remains valid;
- divergent-role RPA V2 bridge;
- exact-role member remains exclusively V1;
- Candidate 4 `ordered_participant -> source/affected` bridge;
- unary `trigger_source` mapping;
- higher-order mapping with every position explicit;
- exact `cpa.v3` identity and canonical ordering;
- exact V3 member binding to current `rpa.v2`;
- all six V3 precondition kinds use the closed V3 resolver;
- V3 class projection is sourced from the linked RPA V2 member proof;
- V3 participant preconditions never compare historical and reviewed roles
  directly;
- V4 review evidence is non-empty `AcceptanceEvidenceRefV1[]`;
- V4 raw REV3 roles are exactly the four frozen registry rows;
- V1 dependency source bindings project exactly into V4 roles;
- superseded `rpr.v1`/`cpr.v1` theorem dependencies block live eligibility;
- complete closed RPA V2 and Context V3 member wire objects reject unknown
  fields;
- rejected V3 admission when a divergent member has no current RPA V2;
- rejected exact-role V3 member;
- exact `ApplicationHostBindingV3` reuse of current HBC claims;
- separate V2/V3 context authority eligibility;
- Rust/Python/schema identity parity;
- deterministic repeatability of all preimages.

Negative cases:

- mutate SourceInstance historical roles;
- mutate bridge historical or reviewed role;
- swap source and affected;
- swap positions;
- substitute participant kind or semantic reference;
- omit, duplicate, or add bridge entries;
- infer role from array position or host;
- use V1 for a divergent member;
- use stale, revoked, or ambiguous RPA V2;
- omit an RPA binding for a divergent V3 member;
- add an RPA binding for an exact-role V3 member;
- admit an exact-role member in RPA V2;
- attach `ApplicationHostBindingV2` to a `cpa.v3` application;
- attach `ApplicationHostBindingV3` to a `cpa.v2` application;
- omit V3 HostBinding authority while retaining a current or historical V3 link;
- provide V3 HostBinding authority without an applicable V3 link;
- alter only the V3/RPA base-authority or candidate-universe digest;
- rebase one shared B1/B2/REV3 snapshot while retaining the other;
- omit an immutable event closure dependency from an aggregate closure;
- omit a superseded or replacement endpoint event from a V4 supersession
  closure;
- use the wrong V3 member or candidate-universe digest;
- alter `cpa.v2` bytes while retaining its old identity;
- attempt to make an invalid cpa.v2 valid through a wrapper;
- use HostBinding as role evidence;
- inject RPA sources into `ae.v3` closure;
- omit or add a V4 source-binding role;
- bind the V4 event leaf or V3 aggregate into its own event closure;
- create cross-version supersession.

## Migration strategy for the five-candidate pilot

1. Bind the exact pilot candidate-universe and REV3 bytes.
2. Preserve every existing V1 SourceInstance and application byte.
3. Resolve each exact RelationProofV1 record.
4. Classify each member by exact historical/reviewed role equality.
5. Keep exact-role members exclusively on the V1 compatibility path.
6. Create RPA V2 only for divergent members.
7. Keep exact-role context applications exclusively on the frozen V2 path;
   if the exact V2 contract is not satisfied, the member remains `BLOCKED`.
8. Create `cpa.v3`/`cpar.v3` for role-divergent reviewed context members.
9. Bind the exact current `rpa.v2` ID in each divergent V3 member.
10. Recompute RPA V2 and Context V3 currentness independently.
11. Run HostBinding separately.
12. Keep the pilot outside C admission until complete authority closure passes.

No V1-to-V2 cross-version supersession or in-place migration is permitted.

## Unchanged contracts

```text
SourceInstanceV1
SourceContextV1
CandidateIdentityV1
RelationProofV1
RelationApplicationV1
ContextProofV1
ContextApplicationV2
HostBinding V1/V2
DomainProofV1
DomainApplicationV1
ClassProjectionV1
EngineState and runtime rules semantics
Decision protocol, RNG, checkpoint, replay, state hashing
PlayerObservation, PlayerInformationState, ML trajectories
```

## Alternatives rejected

| Alternative | Decision |
|---|---|
| Rewrite SourceInstanceV1 roles | Reject: destroys historical provenance |
| Loosen V1 role equality | Reject: changes an existing contract in place |
| Add an alternate cpa.v2 validator profile | Reject: reinterprets cpa.v2 |
| Add a CRQ wrapper behind an invalid cpa.v2 | Reject: the wrapper is unreachable for Candidate 4 |
| Put role-divergent reviewed context into cpa.v2 | Reject: use versioned cpa.v3 instead |
| Infer roles from position, left/right, or host | Reject: not auditable |
| Put relation roles into HostBinding | Reject: host membership is not causality |
| Put the bridge into RelationProofV1 | Reject: source-instance-specific data would pollute theorem identity |
| Create a second standalone role theorem family | Reject for now: duplicates RelationProof authority |
| Duplicate the source-binding DTO per authority | Reject: mechanical contract drift |

## Implementation decomposition for a later task

1. Freeze the shared source-binding primitive and separate role registries.
2. Add RPA V2 bridge DTOs and canonical identity vectors.
3. Add the complete RelationApplicationAuthorityV2 container and source
   closure.
4. Add RPA V2 source resolution, validation, and currentness.
5. Add the frozen V4 event/reference contracts and review checklist validation.
6. Add ContextApplicationV3 DTOs, identities, source validation, and negative
   fixtures.
7. Extend the V4 event subject contract to Context V3 records and
   supersession records.
8. Add the ContextApplicationAuthorityV3 host-authority binding and V3 HBC
   composition.
9. Add independent HostBinding composition regression tests.
10. Migrate only the exact five-pilot fixtures.
11. Run Rust/Python/schema/conformance and reproducibility gates.
12. Obtain independent architecture/rules review before any production
    record.

## Final status

```text
ARCHITECTURE_DIRECTION              = PASS
BLOCKER                              = 0
MAJOR                                = 0
MINOR                                = 0
ARCHITECTURAL_REDIRECTION_REQUIRED  = NO
ADR_READY                            = PASS
ADR_ACCEPTANCE                       = ACCEPTED
PERMANENT_ADR_PROMOTION              = COMPLETE
IMPLEMENTATION                       = NOT_AUTHORIZED
PRODUCTION_AUTHORITY_RECORDS         = NOT_AUTHORIZED
DOMAIN_AUDIT_55_55                   = NOT_RUN
C_CHANGE                             = STOP
M3                                   = NOT_AUTHORIZED
```

ADR 0045 is accepted as permanent architecture. Acceptance does not authorize
implementation, production authority records, C changes, or M3.
