# M2.5.C — Declared Interaction Model Closure Specification

Status: corrected design specification; implementation not authorized pending
independent review.

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
   candidate source and the upstream review-additions authority, with
   explicitly accounted additions and removals;
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
  C_DESIGN_SPEC.md
  declared_interaction_model.v1.json
  interaction_review_additions.v1.json
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

`C_DESIGN_SPEC.md` is an exact-inventory, non-semantic C artifact. It is
inside the later C master-drift boundary so that the reviewed contract travels
with the C snapshot, but it is not a semantic C input and is never included in
`interaction_closure.v1.json`'s bound input set.

The C source authority graph is:

```text
declared_interaction_model
        |
        v
interaction_review_additions
        |
        v
interaction_candidate_universe
        |
        v
interaction_semantic_classes
        |
        v
interaction_classifications
        |
        v
interaction_closure
        |
        v
report / verification evidence
```

`interaction_review_additions.v1.json` is an upstream, source-grounded
authority for explicitly reviewed candidate proposals. It is allowed to be
empty for a V1 snapshot, but a `TARGETED_HIGHER_ORDER_REVIEW` candidate is
invalid unless its review record exists in this artifact. The candidate
universe owns the resulting source-instance ledger; it does not own or hide
the review records themselves.

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
- `interaction_review_additions.v1.json`;
- `interaction_candidate_universe.v1.json`;
- `interaction_semantic_classes.v1.json`; and
- `interaction_classifications.v1.json`.

The closure also records the exact identities of its external B1.Final, B2,
and REV3 prerequisites. It does not bind its own bytes, the report, the
negative-test matrix, or the verification summary. The report may reproduce
closure results and the closure digest; the summary may record all digests.
Neither is therefore a closure input.

## 7. File contracts

All JSON artifacts MUST be UTF-8, emitted with deterministic key and array
ordering, and validated as closed objects. Unknown top-level keys are rejected.
The JSON representation is a wire/document representation only. No persisted
semantic identity may hash JSON, Serde output, a language-native object, or an
implementation-defined serialization.

Unless a field is explicitly nullable below, it is required and non-null.

### 7.0 Persisted identity and raw-byte contracts

The only new C semantic identity digests are `CandidateIdentityV1` and
`InteractionClassIdentityV1`. Their digest envelopes and preimages are fixed
here. An implementation MUST not choose alternate field order, omit optional
slots, hash a JSON projection, or derive a preimage from a schema library.

The persisted JSON form of each identity is the repository-accepted closed
digest-reference object:

```text
{
  "envelope_id": "mtgml.digest-envelope.v1",
  "algorithm_id": "sha-256",
  "semantic_domain": <exact domain below>,
  "payload_codec_id": "mtgml.canonical-cbor.v1",
  "input_schema_id": <exact schema ID below>,
  "digest_hex": <64 lowercase hexadecimal characters>
}
```

The digest bytes are computed exactly as required by the existing ADR-0038
envelope:

```text
envelope_bytes =
  ASCII("mtgml.digest-envelope.v1")
  || 0x00
  || frame(ASCII("sha-256"))
  || frame(ASCII(semantic_domain))
  || frame(ASCII("mtgml.canonical-cbor.v1"))
  || frame(ASCII(input_schema_id))
  || frame(canonical_payload)

frame(x) = u64_be(byte_length(x)) || x
digest_bytes = SHA256(envelope_bytes)
digest_hex = lowercase hexadecimal(digest_bytes)
```

The identity payload is canonical CBOR V1. It contains only the allowed
deterministic-CBOR forms from `docs/STATE_HASHING.md`: fixed-position arrays,
unsigned integers, byte strings, exact UTF-8 text, `false`, `true`, and
`null`. It contains no CBOR maps, floats, tags, bignums, indefinite-length
values, or trailing values. Optional semantic positions are always present as
`null`. Every enum slot is a fixed array `[variant_id, payload]`; a unit enum
variant is exactly `[exact_variant_id, null]`. The variant identifier is the
closed ASCII identifier declared by the model, with no case folding,
normalization, or synonym lookup. Unordered reference sets are arrays sorted
by the unsigned lexicographic bytes of each entry's canonical CBOR key.

The JSON artifacts may expose enum values by their exact ASCII variant
identifiers for readability. Before hashing, the checker converts every enum
slot to the fixed CBOR pair above. A bare string is never an accepted enum
preimage value.

The following nested V1 values are fixed-position arrays:

```text
ParticipantRefV1 = [participant_kind_enum, semantic_ref]
ParticipantRoleV1 = [position, role_enum, participant_kind_enum, semantic_ref]
ParticipantSourceRefV1 = [source_kind_enum, source_locator]
ContextDimensionsV1 = [
  zone_enum,
  visibility_enum,
  timing_enum,
  temporal_order_enum,
  source_affected_relation_enum,
  control_ownership_relation_enum,
  replacement_layer_relation_enum,
  trigger_lki_relation_enum,
  information_relation_enum,
  decision_actor_relation_enum
]
TemporalSemanticsV1 = [trigger_order_enum, dependency_order_enum, duration_enum, replacement_order_enum]
B2FamilyRefV1 = [family_id, lifecycle_enum, assignment_role_enum]
B2BoundaryRefV1 = [family_id, precise_semantic_definition]
B1FinalCitationRefV1 = [authority_id, citation_id]
EvidenceRefV1 = [authority_kind_enum, path, locator, raw_sha256]
```

`B1FinalCitationRefV1` entries are sorted by the unsigned lexicographic bytes
of their canonical CBOR two-element array. The JSON representation uses the
closed object `{ "authority_id": ..., "citation_id": ... }`, and the checker
converts it to the exact two-position array before ordering or hashing. Each
pair must resolve to the corresponding accepted B1.Final authority and
citation node.

All enum vocabularies and the permitted field-to-vocabulary grammar are
declared in `declared_interaction_model.v1.json` below. A semantic value not
present in that closed V1 vocabulary blocks the snapshot and requires a
versioned spec amendment; it may not be represented as a new free-text
synonym. The literal `NOT_APPLICABLE` is an exact unit variant, encoded as
`["NOT_APPLICABLE", null]`, when a declared dimension is irrelevant. The
source JSON must still carry the same field explicitly; omission is not
equivalent to `NOT_APPLICABLE`.

#### 7.0.1 `CandidateIdentityV1`

The exact identity metadata is:

```text
semantic_domain = "manafold.m2.5.c.candidate-identity.v1"
input_schema_id = "manafold.m2.5.c.candidate-identity-input.v1"
payload_codec_id = "mtgml.canonical-cbor.v1"
algorithm_id = "sha-256"
envelope_id = "mtgml.digest-envelope.v1"
```

The canonical payload is this fixed-position `CandidateIdentityInputV1`:

```text
[
  source_origin_enum,
  scope_enum,
  relation_enum,
  participant_refs_array,
  supporting_requirement_ids_sorted_array,
  source_binding_union
]
```

The positions are numbered zero through five in the order shown. The source
origin, scope, and relation slots are each `EnumV1 = [exact_variant_id, null]`
for their declared unit variant. They use the exact ASCII identifiers declared
by the C JSON contract, with no case folding. The source-origin values are the
exact uppercase values in §7.3. `participant_refs_array` preserves semantic
participant order. `supporting_requirement_ids_sorted_array` is sorted by
canonical CBOR bytes and rejects duplicates. `source_binding_union` is the
fixed discriminated union specified in §7.3. The candidate's terminal
disposition, review rationale, class ID, and reconciliation status are not in
this identity; changing a review decision must not silently change source
candidate identity.

For an inherited REV3 candidate, the source binding includes the original
candidate ID and every exact source-row value. For a new candidate, the source
binding includes the exact B2 or targeted-review evidence that gives the
candidate its identity. The original REV3 candidate ID is preserved as data;
it is not rewritten to the digest-derived ID.

New candidate IDs are deterministic and namespaced:

```text
B2_DERIVED:
  c.v1/b2-derived/<CandidateIdentityV1.digest_hex>

TARGETED_HIGHER_ORDER_REVIEW:
  c.v1/targeted-higher-order-review/<CandidateIdentityV1.digest_hex>
```

The complete 64-character digest is used without truncation. The prefix and
variant spelling are exact, lowercase ASCII, and use `/` separators. Two new
candidates with the same ID must have byte-identical `CandidateIdentityV1`
payloads and identical source-binding evidence; otherwise the checker fails
with a candidate-identity collision. A duplicate ID with the same payload is
also invalid unless it is represented as an explicit lineage merge of one
candidate record, not as two current candidates. A new candidate ID must not
collide with any preserved REV3 ID or with a different source-origin namespace.

#### 7.0.2 `InteractionClassIdentityV1`

The exact identity metadata is:

```text
semantic_domain = "manafold.m2.5.c.interaction-class-identity.v1"
input_schema_id = "manafold.m2.5.c.interaction-class-identity-input.v1"
payload_codec_id = "mtgml.canonical-cbor.v1"
algorithm_id = "sha-256"
envelope_id = "mtgml.digest-envelope.v1"
```

The canonical payload is this fixed-position `InteractionClassIdentityInputV1`:

```text
[
  arity_enum,
  directionality_enum,
  participant_roles_array,
  host_relationship_enum,
  context_dimensions_v1,
  temporal_semantics_v1,
  b2_family_refs_sorted_array,
  b2_boundary_refs_sorted_array,
  b1_final_citation_refs_sorted_array
]
```

The positions are numbered zero through eight in the order shown. Rationale
prose and evidence-reference arrays are required class-record fields, but are
deliberately not identity fields: they are protected by the raw class-artifact
binding and the closure's semantic-input bindings. Thus class equality and
the identity preimage describe the same reusable semantic meaning, not a
particular editorial wording or evidence list. The class ID is the full
lowercase `digest_hex` namespaced as:

```text
ic.v1/<InteractionClassIdentityV1.digest_hex>
```

Class identity includes exactly the nine semantic positions listed above. It
never includes rationale prose, evidence-reference arrays, candidate IDs, or
source-instance IDs. Concrete instances therefore reuse one class without
copying its definition or changing its identity.

These two identity contracts are the only new C semantic digest preimages.
Artifact bindings use raw SHA-256 of exact file bytes, and source-tree
fingerprints use the accepted B2 algorithm in §13.
Any additional semantic identity field, closure identity, or ad-hoc
source-tree identity may be persisted only after this specification is amended
with its complete envelope and fixed preimage.

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
participant_kind_vocabulary
participant_role_vocabulary
context_value_vocabulary
temporal_value_vocabulary
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

The following closed V1 vocabularies are normative. The JSON model stores the
exact uppercase ASCII identifiers; every identifier is a unit enum and is
encoded as `[identifier, null]` in a semantic preimage. The checker rejects
unknown identifiers and rejects a value from the wrong vocabulary for a field.

```text
participant_kind_vocabulary =
  ABILITY, CARD, COPIABLE_VALUE, DECK, EFFECT, EVENT, OBJECT, PERMANENT,
  PLAYER, SOURCE_INSTANCE, SPELL, TOKEN, ZONE

participant_role_vocabulary =
  AFFECTED, CONTROLLER, COPIED_SOURCE, COPY_RESULT, DECISION_ACTOR,
  DESTINATION_ZONE, ORIGIN_ZONE, ORDERED_PARTICIPANT, OWNER,
  REPLACEMENT_ACTOR, SOURCE, TARGET, TRIGGER_SOURCE

context_value_vocabulary =
  zone =
    BATTLEFIELD, COMMAND_ZONE, EXILE, GRAVEYARD, HAND, LIBRARY, OUTSIDE_GAME,
    STACK, ZONE_AGNOSTIC, NOT_APPLICABLE
  visibility =
    CONTROLLER_ONLY, HIDDEN_TO_ACTOR, IDENTITY_HIDDEN, NOT_APPLICABLE, OWNER_ONLY,
    PRIVATE, PUBLIC
  timing =
    ACTIVATION_TIME, CAST_TIME, COMBAT_TIME, CONTINUOUS_EFFECT, NOT_APPLICABLE,
    RESOLUTION_TIME, STATE_BASED_CHECK, TRIGGER_TIME, TURN_BOUNDARY,
    ZONE_CHANGE_TIME
  temporal_order =
    AFTER, BEFORE, DURING, NOT_APPLICABLE, SEQUENTIAL, SIMULTANEOUS, UNTIL,
    WHILE
  source_affected_relation =
    BOTH_AFFECTED, NO_EFFECT_RELATION, NOT_APPLICABLE, SOURCE_AFFECTED,
    SOURCE_AFFECTS_OTHER
  control_ownership_relation =
    CONTROL_CHANGES, CROSS_CONTROLLER, CROSS_OWNER, NOT_APPLICABLE,
    OWNERSHIP_CHANGES, SAME_CONTROLLER, SAME_OWNER
  replacement_layer_relation =
    COPY_LAYER, CONTROL_LAYER, LAYER_DEPENDENCY, NO_REPLACEMENT_OR_LAYER,
    NOT_APPLICABLE, PT_LAYER, REPLACEMENT_EFFECT, TYPE_LAYER,
    ZONE_CHANGE_REPLACEMENT
  trigger_lki_relation =
    INTERVENING_IF, LAST_KNOWN_INFORMATION, NO_TRIGGER_LKI, NOT_APPLICABLE,
    TRIGGER_CONDITION, TRIGGERED_EVENT
  information_relation =
    HIDDEN_IDENTITY, KNOWN_TO_CONTROLLER, KNOWN_TO_OWNER, NO_INFORMATION_DEPENDENCY,
    NOT_APPLICABLE, PRIVATE_LOOK, PUBLIC_IDENTITY, RANDOM_UNKNOWN
  decision_actor_relation =
    ACTIVE_PLAYER, CONTROLLER, NO_DECISION, NOT_APPLICABLE, OPPONENT, OWNER,
    RULES_FORCED, TARGET_PLAYER

temporal_value_vocabulary =
  dependency_order =
    DEPENDENCY_ORDERED, NO_TEMPORAL_DEPENDENCY, NOT_APPLICABLE
  duration =
    DURATION_LIMITED, INDEFINITE, NOT_APPLICABLE, UNTIL_EVENT
  replacement_order =
    AFTER_EFFECT, BEFORE_EFFECT, NO_TEMPORAL_DEPENDENCY, NOT_APPLICABLE,
    SAME_EVENT
  trigger_order =
    DEFERRED, IMMEDIATE, NO_TEMPORAL_DEPENDENCY, NOT_APPLICABLE

additional enum vocabularies used by the fixed records:
  arity = UNARY, BINARY, HIGHER_ORDER
  directionality = DIRECTED, NONE, SYMMETRIC
  host_relationship = CROSS_HOST, NOT_APPLICABLE, SAME_HOST
  authority_kind = B1_FINAL, B2, C_REVIEW, REV3
  assignment_role = PRIMARY, SUPPORTING
  lifecycle = ACTIVE, ACTIVE_UNASSIGNED
  source_origin = B2_DERIVED, REV3, TARGETED_HIGHER_ORDER_REVIEW
  scope = CROSS_DECK, INTRA_DECK, UNARY_OR_HIGHER_ORDER
  relation = DECLARED_CARD_TRIGGER, DIRECTIONAL_BINARY, UNORDERED_BINARY
  review_kind = TARGETED_HIGHER_ORDER_REVIEW
  source_kind = B2_ASSIGNMENT, B2_CLASSIFICATION, REV3_ROW
```

The field grammar is closed as well: `participant_kind` uses
`participant_kind_vocabulary`; `role` uses `participant_role_vocabulary`;
the ten `context_dimensions` slots use the corresponding `context_value`
vocabulary; and the four `temporal_semantics` slots use the corresponding
`temporal_value` vocabulary. `semantic_ref` is not a semantic label: it is an
exact resolvable source identifier from the pinned ledgers. If a needed value
is absent, the snapshot is `BLOCKED` until this V1 model is amended; no free
text or synonym is admitted.

### 7.2 `interaction_review_additions.v1.json`

This file is the upstream authority for C-reviewed candidate proposals that
do not have a REV3 source row. It is created before candidate generation and
is never generated from the candidate universe, classifications, closure,
report, or verification summary. It contains exactly:

```text
schema
model_id
input_bindings
review_record_count
review_records
```

`review_records[]` contains exactly:

```text
review_record_id
review_kind
participant_source_refs
review_evidence_refs
review_rationale
```

`review_kind` is exactly `TARGETED_HIGHER_ORDER_REVIEW`. The record ID is a
unique stable authority key within this artifact; it is preserved verbatim in
the targeted source binding and is not a digest or a substitute for source
evidence. `participant_source_refs` is an ordered, finite list of exact
REV3/B2 source locators, and `review_evidence_refs` is a duplicate-free array
of `EvidenceRefV1` references sorted by canonical CBOR bytes. The participant
references must resolve to the pinned source authorities and establish the
finite participant set for the proposal. `review_rationale` is mandatory
source-grounded review prose; it is not a semantic identity preimage. Review
evidence may resolve only to pinned REV3, B2, or B1.Final inputs; it may not
refer to this review-additions file, another C review record, or downstream C
artifacts.

`input_bindings` is a closed object containing exactly:

```text
declared_model_path = "sources/m2_5/closures/C/declared_interaction_model.v1.json"
declared_model_raw_sha256
source_evidence_refs_sorted_array
```

The source-evidence array contains the exact external REV3/B2 source
identities used to review the additions, sorted by the canonical CBOR bytes of
`EvidenceRefV1`. It must not contain any candidate, class, classification,
closure, report, or verification-summary digest. The artifact's raw SHA-256 is
bound by the candidate universe and closure, so no self-reference is
introduced. `review_record_count` is recomputed from the array. An empty
`review_records` array is valid for a V1 snapshot and proves that no targeted
higher-order candidate authority was silently introduced.

`review_record_id` has the closed form
`ira.v1/<lowercase-ascii-stable-key>`, where the key matches
`[a-z0-9][a-z0-9._-]*`; IDs are unique in bytewise order. It is an upstream
reviewer's stable authority key, not a digest and not a generated candidate
ID. Each `participant_source_refs` entry is the JSON projection of
`ParticipantSourceRefV1`; the source kind and locator must resolve to one of
the pinned REV3 rows or accepted B2 records.

Every `TARGETED_HIGHER_ORDER_REVIEW` candidate must reference one and only one
record in this artifact. Unknown, duplicate, orphan, or path-substituted review
records are rejected. A review record cannot create a rules authority or
replace a missing B1.Final citation; it can only propose a source-grounded C
candidate within the declared model.

### 7.3 `interaction_candidate_universe.v1.json`

This file is the mechanically complete candidate ledger. It contains:

```text
schema
model_id
input_bindings
candidate_count
candidate_reconciliation_counts
source_instance_count
candidates
source_instances
```

`input_bindings` records the exact raw identities of the declared model,
`interaction_review_additions.v1.json`, the pinned REV3 source, and the
accepted B2/B1.Final inputs used to generate the ledger. The review-additions
binding is mandatory even when its `review_records` array is empty. It is a
raw artifact binding, not a semantic digest preimage. `candidate_count` and
`source_instance_count` are recomputed from their respective arrays, and
`candidate_reconciliation_counts` is recomputed from candidate lineage.

Each `candidates[]` object contains exactly:

```text
candidate_id
candidate_identity
source_origin
scope
relation
participant_refs
supporting_requirement_ids
source_binding
reconciliation_status
reconciliation_reason
```

`candidate_id`, `scope`, `relation`, `participant_refs`, and source fields are
preserved from REV3 when the candidate is inherited. `candidate_identity` is
the exact `CandidateIdentityV1` reference from §7.0.1 and is not a replacement
for the original REV3 ID.

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

`source_binding` is a closed discriminated union. The JSON form has a required
`kind` field and only the fields for that kind; the canonical identity form is
the fixed-position enum `[kind, payload]`. It has these variants:

```text
REV3 {
  archive_member,
  archive_member_sha256,
  row_ordinal,
  source_columns,
  source_values
}

B2_DERIVED {
  classification_path,
  classification_raw_sha256,
  classification_identity,
  oracle_semantic_identity,
  assignment_refs,
  catalog_path,
  catalog_raw_sha256
}

TARGETED_HIGHER_ORDER_REVIEW {
  additions_path,
  additions_raw_sha256,
  review_record_id,
  review_kind,
  participant_source_refs,
  review_evidence_refs
}
```

Their canonical `SourceBindingV1` payloads are fixed arrays in exactly this
order:

```text
[
  "REV3",
  [archive_member, archive_member_sha256, row_ordinal,
   source_columns_array, source_values_array]
]

[
  "B2_DERIVED",
  [classification_path, classification_raw_sha256,
   classification_identity_digest_reference_v1, oracle_semantic_identity,
   assignment_refs_sorted_array, catalog_path, catalog_raw_sha256]
]

[
  "TARGETED_HIGHER_ORDER_REVIEW",
  [additions_path, additions_raw_sha256, review_record_id, review_kind_enum,
   participant_source_refs_ordered_array, review_evidence_refs_sorted_array]
]
```

`classification_identity_digest_reference_v1` is the existing B2
`DigestReferenceJsonV1` projection. The checker validates its exact persisted
fields (`envelope_id`, `algorithm_id`, `semantic_domain`, `payload_codec_id`,
`input_schema_id`, and `digest_hex`) against the accepted B2 identity and then
converts those exact fields to the normative six-position `DigestReferenceV1`
CBOR array from `docs/STATE_HASHING.md`; the sixth slot is the 32-byte digest
obtained by decoding the validated lowercase `digest_hex`, not the hex text.
It does not hash the JSON projection or invent/rederive a different identity.
Each `assignment_refs_sorted_array` entry is the fixed array
`[family_id, assignment_ordinal, precise_semantic_definition]`, sorted by its
canonical CBOR bytes. Each participant source reference is an exact pinned
locator, and evidence references use `EvidenceRefV1`. These arrays are the
only permitted `source_binding_union` preimage; JSON field order has no effect.

For `REV3`, `archive_member` is exactly
`derived/Pair_Interaction_Census_REV3.csv`, `archive_member_sha256` is the
raw SHA-256 of that pinned member, `row_ordinal` is the zero-based data-row
ordinal after the single header row, `source_columns` is exactly:

```text
[
  "candidate_id", "model_id", "scope", "pair_id", "left_family_id",
  "right_family_id", "relation", "disposition", "disposition_reason",
  "supporting_requirement_ids"
]
```

`source_values` preserves every cell as exact source text in that order. The
supporting-requirement cell is preserved before parsing it into
`supporting_requirement_ids`. There is no nullable REV3 row field on a
non-REV3 candidate.

For `B2_DERIVED`, every classification and catalog path, raw file digest,
accepted B2 classification identity, OSI, assignment reference, family, and
boundary binding is required. `classification_path` is the exact repository
path `sources/m2_5/closures/B2/card_semantic_classifications.v1.json` and
`catalog_path` is the exact repository path
`sources/m2_5/closures/B2/requirement_family_catalog.v1.json`.
`assignment_ordinal` is the zero-based position in the exact B2
`requirement_assignments` array for the referenced classification record.
`classification_identity` uses the existing B2 digest-reference contract; C
does not rederive or rename it.

For `TARGETED_HIGHER_ORDER_REVIEW`, `additions_path` is exactly
`sources/m2_5/closures/C/interaction_review_additions.v1.json`,
`additions_raw_sha256` is its exact raw file digest, and `review_record_id`
must resolve to exactly one record in that artifact. `review_kind` is the
unit enum `TARGETED_HIGHER_ORDER_REVIEW`; the ordered participant references
and sorted evidence references must equal the resolved review record. This
variant is the complete source binding for the candidate and does not pretend
to have a REV3 row.

The source union is mutually exclusive: a record with `kind = REV3` cannot
carry B2-derived or targeted-review fields, and a new variant cannot carry
REV3 fields. Unknown kinds and unknown variant fields are rejected.

The candidate universe also owns the authoritative source-instance ledger:

```text
source_instances[]
  source_instance_id
  candidate_id
  source_binding
  participant_bindings
  source_context
```

Every source instance belongs to exactly one candidate. The ledger is
source-grounded: its `source_binding` is one of the candidate union variants,
its ordered `participant_bindings` resolve to the candidate's participant
references, and its `source_context` contains every context dimension required
by the declared model, using `NOT_APPLICABLE` explicitly where appropriate.
There is no source-instance authority outside this ledger.

The canonical forms used for the ledger's deterministic ordering are
`ParticipantBindingV1 = [role, participant_ref]` and
`SourceContextV1 = ContextDimensionsV1`. The JSON ledger may expose named
fields, but the checker converts them to these fixed arrays before comparing or
ordering them.

`source_instance_id` is a deterministic ledger key, not a new semantic digest.
For the zero-based `instance_index` assigned after sorting a candidate's
source-instance tuples by the canonical CBOR bytes of the fixed tuple
`[source_binding, participant_bindings, source_context]`, it is:

```text
si.v1/<base64url-no-padding(UTF8(candidate_id))>/<instance_index-decimal>
```

The Base64 encoding is RFC 4648 URL-safe encoding without `=` padding and the
index is an unpadded base-ten integer. The full candidate ID and index are
included; no truncation or normalization is allowed. The verifier recomputes
these keys, rejects duplicates, rejects an instance whose candidate is absent,
and rejects any classification reference not present in this ledger.

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

### 7.4 `interaction_semantic_classes.v1.json`

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

`input_bindings` contains the raw SHA-256 of the exact candidate-universe
bytes and the accepted external B2/B1.Final identities used to define the
classes. It does not bind classifications, closure, report, or verification
evidence. This makes the authority edge
`candidate_universe -> semantic_classes -> classifications` explicit;
`interaction_classifications.v1.json` separately binds
`semantic_classes_raw_sha256`.

Allowed `arity` values are `UNARY`, `BINARY`, and `HIGHER_ORDER`. A
`HIGHER_ORDER` class MUST state the exact finite participant count by the
normative derived rule `participant_count := len(participant_roles)` and must
provide ordered participant roles. The count is not a second free field in the
class identity; the checker recomputes it from the closed role array and
exposes it in class metrics and the report. The arity/count relation is closed:
`UNARY` requires count `1`, `BINARY` requires count `2`, and `HIGHER_ORDER`
requires count greater than `2`. A higher-order class is not an unbounded
N-way claim.

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

Roles MUST use one of the closed `participant_role_vocabulary` identifiers and
must state the participant's semantic role explicitly; a role may not be
implied only by its position. If the evidence requires a role outside that
vocabulary, C is blocked pending a versioned model amendment.

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
and their exact `precise_semantic_definition` strings. Each required family
reference states its lifecycle and assignment role. A card-derived class may
reference only a valid terminal assignment to an `ACTIVE` family. An
`ACTIVE_UNASSIGNED` family is rejected in that position.

`b1_final_citation_refs[]` identify nodes in the accepted B1.Final citation
graph. Every normative rule claim supporting a required class MUST resolve to
one of these nodes. C cannot create a citation node or replace a missing
official domain with a URL, prose, or live search result.

`class_identity` is computed from exactly the canonical class meaning listed
in the nine-position `InteractionClassIdentityV1` preimage in §7.0.2:
arity, directionality, roles, host relationship, context, temporal semantics,
B2 boundary references, and B1.Final citation references. The required
`semantic_rationale` and `source_evidence_refs` fields remain class-record
provenance, but are not class equality or identity inputs. The digest is not a
digest of the JSON object. Source instances are not copied into the class
definition; they are bound by candidate classification records.

### 7.5 `interaction_classifications.v1.json`

This file contains one record for every candidate in the candidate universe.
It contains:

```text
schema
model_id
candidate_universe_raw_sha256
semantic_classes_raw_sha256
classification_count
candidate_classifications
```

The two input bindings above are raw SHA-256 values of the exact UTF-8 JSON
bytes of the named C files. They are artifact bindings, not semantic identity
digests; no JSON serialization is used as a semantic preimage.

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

The verifier resolves every `source_instance_id` against the candidate
universe ledger, requires the ledger's `candidate_id` to match, and compares
the mapping's participant bindings and context binding with the authoritative
ledger values. Unknown, duplicate, or orphan instances are rejected before a
terminal disposition is accepted.

`reconciliation` records the candidate-universe status, original REV3 ID when
applicable, and any merged/new/stale/removal linkage. `review_rationale` is a
specific source-grounded explanation, not a keyword or co-occurrence claim.
`evidence_refs` resolve only to pinned REV3, B2, B1.Final, or C review records.

The file MUST NOT contain copied class definitions. It may contain only the
class ID and the concrete binding to that class.

### 7.6 `interaction_closure.v1.json`

This is the sole semantic C closure artifact. It contains:

```text
schema
model_id
bound_semantic_inputs
external_prerequisite_identities
candidate_reconciliation
semantic_class_metrics
terminal_disposition_metrics
source_instance_metrics
gate_status
flags
```

`bound_semantic_inputs` contains exactly these five entries and no others:

```text
declared_interaction_model.v1.json
interaction_review_additions.v1.json
interaction_candidate_universe.v1.json
interaction_semantic_classes.v1.json
interaction_classifications.v1.json
```

Each entry records path, schema, raw SHA-256 of the exact file bytes, and
record count. `C_DESIGN_SPEC.md`, the report, the negative matrix, and the
verification summary are not closure inputs. The closure does not include a
self-digest or a new closure identity.

`external_prerequisite_identities` records the exact REV3 archive/member,
B2 catalog/classification/boundary/assignment closure, and B1.Final authority
citation graph identities used to validate the five C inputs. These are
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

`gate_status` MUST preserve the existing M2.5 vocabulary exactly. On a C PASS,
the required values are:

```text
CLASSIFICATION_REFERENCE_CLOSURE        = PASS
OFFICIAL_RULE_CITATION_CLOSURE          = PASS
DECLARED_INTERACTION_MODEL_CLOSURE      = PASS
REV2_REUSE_RATIO_REPRODUCIBLE           = BLOCKED
RANKING_UNCERTAINTY_PROPAGATION         = BLOCKED
```

The closure MUST also carry these exact flags:

```text
DECK_PAIR_LOCKED                    = false
AUTHORITATIVE_RANKING_AVAILABLE     = false
M3_STARTED                          = false
```

There is no parallel C gate or flag vocabulary. C owns only the transition of
`DECLARED_INTERACTION_MODEL_CLOSURE`; the other values are inherited or remain
blocked as shown above.

### 7.7 `INTERACTION_MODEL_REPORT.md`

The report is a human-readable projection of the C artifacts. It MUST include:

- the exact source/master/archive identities;
- the authority graph and acyclic digest policy;
- candidate totals and all reconciliation deltas;
- semantic class count and class-shape totals;
- terminal disposition totals and unresolved count;
- high-risk review-set coverage;
- B2 and B1.Final binding summary;
- closure status and its raw artifact SHA-256, when available;
- the exact `gate_status` values and boolean flags from §7.6; and
- exact commands and their actual statuses.

The report MUST NOT be a closure input. If it repeats closure results or
digests, the checker treats it as derived documentation.

### 7.8 `c_negative_test_matrix.v1.json`

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

### 7.9 `c_verification_summary.v1.json`

This file is an evidence record and remains fully outside the closure. It may
be provisional at H_exec with commands marked `NOT_RUN`; it becomes the
post-execution summary only in Phase C.

The final summary MUST record:

```text
schema
execution_commit
source_tree_before_fingerprint
source_tree_after_fingerprint
prerequisite_results
c_result
negative_test_result
repository_gate_results
artifact_digests
checker_identities
evidence_protocol
evidence_export
```

`execution_commit` is H_exec. `artifact_digests` records raw SHA-256 values of
the exact bytes of every non-summary C inventory file: `C_DESIGN_SPEC.md`,
the declared model, review additions, candidate universe, semantic classes,
classifications, closure, report, and negative matrix. The summary does not
record or bind its own digest. The final summary must never claim `PASS` for
an unexecuted command.

`checker_identities` is mandatory evidence of which verifiers produced the
recorded result:

```text
checker_identities:
  c_checker:
    path = "scripts/check_m2_5_c_interactions.py"
    raw_sha256 = SHA256(exact checker bytes at H_exec)
  master_drift_checker:
    path = "scripts/check_m2_5_master_drift.py"
    raw_sha256 = SHA256(exact checker bytes at H_exec)
```

The paths are exact repository paths. The raw digests bind the executed
checker bytes, not a version string, generated output, or an unpinned tool
installation.

`evidence_protocol` has the fixed fields:

```text
H_exec
modified_path
H_evidence_relation = "direct_child_summary_only"
```

It does not embed a self-referential H_evidence commit digest; the verifier
derives the unique evidence commit from ancestry and proves the relation in
§13.2.

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
5. the exact B2 `precise_semantic_definition` boundary value; and
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
- B2 family references, including lifecycle and assignment role;
- B2 boundary references; and
- B1.Final citation references.

This list is exactly the semantic equality relation and exactly the nine
positions of `InteractionClassIdentityV1`. `semantic_rationale` and
`source_evidence_refs` are intentionally absent from both; they remain
mandatory class-record provenance protected by raw artifact and closure
bindings. A wording change or evidence-list reordering therefore cannot
change a class ID when the declared interaction semantics are unchanged.

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
   and exact ten-file C inventory, including `C_DESIGN_SPEC.md` and the
   upstream review-additions artifact.
3. Execute or consume the current B1, B1.Final, B2, and master-drift gate
   results, rejecting any prerequisite that is not `PASS`.
4. Verify the exact REV3 archive and candidate source digests.
5. Verify the declared model scope and its finite higher-order boundary.
6. Recompute the complete candidate universe and reject a missing, duplicate,
   renamed, or extra inherited candidate; validate the closed source-binding
   union for every candidate.
7. Verify every current candidate has exactly one classification record.
8. Verify candidate IDs, class IDs, and source-instance IDs are unique where
   required; verify digest-derived candidate-ID namespaces and reject identity
   collisions.
9. Verify every classification's reconciliation lineage and source binding.
10. Verify the complete candidate-universe source-instance ledger and every
    classification mapping against its owning candidate.
11. Reject an unknown OSI, family, assignment, or citation reference.
12. Reject a card-derived use of `ACTIVE_UNASSIGNED`.
13. Verify arity, participant count, role names, direction, edge orientation,
    host relationship, zones, timing, information, ordering, and temporal
    semantics.
14. Reject orphan source instances and duplicate or unbound mappings.
15. Verify every required class resolves to B1.Final citation graph nodes.
16. Recompute all candidate, class, reconciliation, and terminal counts.
17. Recompute `CandidateIdentityV1` and `InteractionClassIdentityV1` from their
    prescribed fixed-position CBOR payloads and exact envelope metadata.
18. Verify raw file bindings and exact bound-input identities; reject any
    JSON/Serde-derived semantic digest or unbound identity.
19. Verify the upstream review-additions authority and its raw binding, and
    verify that the report, negative matrix, verification summary, and design
    spec are not closure inputs.
20. Verify the exact existing gate/flag vocabulary, downstream blocked states,
    and absence of any later-gate promotion.
21. Verify the negative-test matrix inventory and expected reason codes.
22. Validate both evidence-creation and historical-descendant evidence modes
    against the H_exec/H_evidence protocol.

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
allowlist only with these exact paths:

```text
scripts/check_m2_5_c_interactions.py
sources/m2_5/closures/C/C_DESIGN_SPEC.md
sources/m2_5/closures/C/declared_interaction_model.v1.json
sources/m2_5/closures/C/interaction_review_additions.v1.json
sources/m2_5/closures/C/interaction_candidate_universe.v1.json
sources/m2_5/closures/C/interaction_semantic_classes.v1.json
sources/m2_5/closures/C/interaction_classifications.v1.json
sources/m2_5/closures/C/interaction_closure.v1.json
sources/m2_5/closures/C/INTERACTION_MODEL_REPORT.md
sources/m2_5/closures/C/verification/c_negative_test_matrix.v1.json
sources/m2_5/closures/C/verification/c_verification_summary.v1.json
```

It must not allow the broad `scripts/` directory, a broad `sources/m2_5/`
prefix, or a C-directory prefix as a substitute for this exact list. Near-miss
paths such as `scripts/check_m2_5_c_interactions.py.backup`,
`sources/m2_5/closures/C/C_DESIGN_SPEC.md.backup`, and an unlisted file under
the C directory must be rejected. The existing master-drift negative
self-test MUST retain an exact near-miss checker-path case. This is a
master-drift integration self-test outside the 32 semantic C mutations and
does not change the exact count of `c_negative_test_matrix.v1.json`.

C source is additive and must not modify historical B1/B2/REV3 artifacts.

## 13. H_exec and H_evidence protocol

### 13.1 Raw artifact bindings and tracked-source fingerprint

Every `*_raw_sha256` field in C is a raw file binding:

```text
SHA256(exact bytes read from the named repository or archive member)
```

It is not a semantic identity and is never substituted for
`CandidateIdentityV1` or `InteractionClassIdentityV1`.

The summary fields `source_tree_before_fingerprint` and
`source_tree_after_fingerprint` reuse the accepted B2 tracked-source
fingerprint algorithm exactly. The implementation MUST NOT introduce a new
tree-hash interpretation. For a commit fingerprint, the algorithm is:

```text
paths = the exact NUL-delimited output order of:
        git ls-tree -r -z --name-only <commit>

fingerprint_bytes = concatenation, for each non-empty path_bytes in paths:
  u64_be(byte_length(path_bytes)) || path_bytes
  || u64_be(byte_length(payload_bytes)) || payload_bytes

payload_bytes = exact bytes returned by:
               git show <commit>:<UTF-8-decoded path>

tracked_source_fingerprint = lowercase_hex(SHA256(fingerprint_bytes))
```

For the working-tree form used only while creating H_exec, the accepted B2
algorithm uses the exact NUL-delimited output order of `git ls-files -z` and
reads each tracked path's exact working-tree bytes. Empty path entries are
skipped. Path bytes are the exact UTF-8 bytes returned by Git; no path
normalization, case folding, sorting, JSON encoding, or line-ending rewrite is
permitted. Both before and after execution fingerprints in the final summary
MUST equal the commit fingerprint of H_exec, exactly as in the accepted B2
evidence contract.

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
the execution gates passed. It records the exact raw digest of
`C_DESIGN_SPEC.md` in the non-semantic artifact inventory but does not bind the
spec into the closure.

The creation invariant is:

```text
H_evidence^ == H_exec
diff(H_exec, H_evidence)
  == {sources/m2_5/closures/C/verification/c_verification_summary.v1.json}
```

The diff comparison is exact: no other path, file mode, rename, or source
content change is permitted.

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

That change records H_exec as `execution_commit`, the exact source-tree
fingerprints, all command results, checker identities, and raw digests for all
nine non-summary C files. The eight semantic/derived artifact digests are:

```text
model
review additions
candidate universe
semantic classes
classifications
closure
report
negative matrix
```

The ninth digest is the raw SHA-256 of `C_DESIGN_SPEC.md`. It is retained for
inventory/evidence integrity but is not a closure input. The summary itself is
outside both the closure and this digest list.

The Phase C commit MUST be a direct child of H_exec. Its diff MUST contain
exactly the verification summary. Any other diff is a failed evidence cycle,
not a minor warning.

Raw logs, the independent semantic-review export, and other generated
verification output remain outside the reproducible source archive.

### 13.2 Historical descendant validation

The verifier has a separate historical-descendant mode for a later branch or
master. In this mode, the current `HEAD` may be any descendant of H_evidence;
it is not required to equal H_evidence, and its parent is not required to be
H_exec.

The verifier MUST:

1. read the summary from the unique reachable evidence commit that records
   `execution_commit = H_exec` and the H_evidence summary-only relation;
2. prove `H_evidence` is an ancestor of the current `HEAD`;
3. prove H_evidence has exactly one parent and that
   `parent(H_evidence) = H_exec`;
4. recompute `git diff --name-status H_exec..H_evidence` and require exactly
   `sources/m2_5/closures/C/verification/c_verification_summary.v1.json`,
   including its mode and content, with no other path or mode change;
5. read the recorded artifact digests from that historical summary and compare
   them with the current exact bytes of every non-summary C inventory file,
   including the non-semantic design spec, report, and negative matrix;
6. require the current summary bytes to equal the bytes returned by
   `git show H_evidence:<summary path>`; this protects the summary without
   inventing a self-digest;
7. recompute the five semantic C input bindings and both V1 semantic identity
   preimages from the current artifacts; and
8. require the current C semantic closure and all recorded prerequisite
   identities to remain equal to the historical evidence snapshot.

If more than one reachable candidate satisfies the evidence relation, if none
does, if the direct-parent proof fails, or if any recorded current artifact
digest differs, the result is `FAIL` or `BLOCKED` according to whether the
contradiction is in source data or unavailable history. The verifier may not
silently select the newest candidate.

This historical mode is the post-merge contract. It must not require
`HEAD == H_evidence` or `HEAD^ == H_exec`; only the ancestry and historical
summary-only proofs above are normative.

The evidence-bearing C change must be integrated with ancestry preserved so
that H_evidence remains an ancestor of the resulting master. A fast-forward
or ancestry-preserving merge commit is allowed. Squash merges and rebase
merges are forbidden for this change; if the hosting workflow cannot preserve
the H_evidence ancestor, the acceptance result is `BLOCKED`.

## 14. Independent review export

After H_exec and before implementation acceptance, the agent MUST provide an
independent review export outside the repository. It must contain:

- the exact H_exec SHA and source tree identity;
- the model summary and its digest;
- the upstream review-additions records and raw digest;
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
11. The closure binds only its five semantic C inputs and is acyclic with
    respect to the design spec, report, negative matrix, and verification
    evidence.
12. The 32 negative tests pass with their exact reason codes.
13. H_exec and H_evidence satisfy the direct-child summary-only rule.
14. Required repository and language gates execute successfully.
15. The exact M2.5 gate/flag values in §7.6 remain preserved; no later gate is
    promoted.
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
