# M2.5.C — Declared Interaction Model Closure Specification

Status: V3 classification-layout amendment; implementation not authorized pending
independent review.

Date: 2026-08-28

Branch: `chris/m2-5-c-v3-classification-sharding`

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
4. assign every current candidate exactly one review state, with a terminal
   disposition only when that state is `resolved`;
5. bind every required class and instance to exact B2 semantic boundaries and
   the B1.Final official-citation graph;
6. validate the resulting closure with a dedicated fail-closed checker;
7. provide the fixed C-001 through C-042 negative-test matrix plus supplemental
   publication-layout mutations, all with stable failure reasons; and
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

C V3 is a publication-layout amendment to the corrected C V2 semantic
snapshot. It does not change the declared model, the upstream review-additions
authority, the candidate-universe semantics, the semantic-class definitions,
the CandidateIdentityV1 contract, the InteractionClassIdentityV1 contract, or
any candidate review state. It changes only the publishable representation of
the candidate classifications and the downstream artifact contracts that bind
that representation.

The four upstream semantic artifacts remain their exact V2 contracts and
paths:

- `declared_interaction_model.v2.json`;
- `interaction_review_additions.v2.json`;
- `interaction_candidate_universe.v2.json`; and
- `interaction_semantic_classes.v2.json`.

The V2 monolithic classification artifact is not carried into V3. The path
`interaction_classifications.v2.json` is forbidden in a V3 source
snapshot, is not a closure input, and must not be present in the reachable
history of a publishable V3 implementation branch. Deleting that file after
branching from a V2 attempt is insufficient; the V3 branch must start from a
verified base whose reachable history does not contain the monolithic V2
classification blob.

C owns exactly the following 26 files under `sources/m2_5/closures/C/`:

```text
sources/m2_5/closures/C/
  C_DESIGN_SPEC.md
  declared_interaction_model.v2.json
  interaction_review_additions.v2.json
  interaction_candidate_universe.v2.json
  interaction_semantic_classes.v2.json
  interaction_classifications.v3.json
  classification_shards/
  classification_shards/interaction_classifications.0000.v3.json
  classification_shards/interaction_classifications.0001.v3.json
  classification_shards/interaction_classifications.0002.v3.json
  classification_shards/interaction_classifications.0003.v3.json
  classification_shards/interaction_classifications.0004.v3.json
  classification_shards/interaction_classifications.0005.v3.json
  classification_shards/interaction_classifications.0006.v3.json
  classification_shards/interaction_classifications.0007.v3.json
  classification_shards/interaction_classifications.0008.v3.json
  classification_shards/interaction_classifications.0009.v3.json
  classification_shards/interaction_classifications.0010.v3.json
  classification_shards/interaction_classifications.0011.v3.json
  classification_shards/interaction_classifications.0012.v3.json
  classification_shards/interaction_classifications.0013.v3.json
  classification_shards/interaction_classifications.0014.v3.json
  classification_shards/interaction_classifications.0015.v3.json
  interaction_closure.v3.json
  INTERACTION_MODEL_REPORT.md
  verification/
    c_negative_test_matrix.v3.json
    c_verification_summary.v3.json
```

The list above is literal and closed. The 16 shard-name entries are the
complete set of shard paths, not a permission for a directory prefix or an
arbitrary number of shards. No additional generator, review database, raw
archive copy, temporary export, helper artifact, V2 monolithic classification
file, or unlisted shard is part of C authority.

The dedicated checker is:

```text
scripts/check_m2_5_c_interactions.py
```

`C_DESIGN_SPEC.md` is an exact-inventory, non-semantic C artifact. It is
inside the later C master-drift boundary so that the reviewed contract travels
with the C snapshot, but it is not a semantic C input and is never included in
`interaction_closure.v3.json`'s bound input set.

The V3 classification authority is the fixed root-and-shard bundle:
`interaction_classifications.v3.json` is the sole classification root and
the exact 16 listed shard files are its candidate-record members. The shards
are the candidate-level record authority; the root is the completeness,
ordering, boundary, and raw-binding authority. Neither a root without all
listed shards nor an unlisted shard is a valid classification authority.

The C source authority graph is:

```text
declared_interaction_model.v2
        |
        v
interaction_review_additions.v2
        |
        v
interaction_candidate_universe.v2
        |
        v
interaction_semantic_classes.v2
        |
        v
ordered classification shards (0000..0015)
        |
        v
interaction_classifications.v3 root
        |
        v
interaction_closure.v3
        |
        v
report / verification evidence
```

The root and every shard also carry the exact candidate-universe and
semantic-class raw bindings required by §7.5. The graph expresses authority
construction order; it does not change the five closure inputs or create a
second classification authority.

The digest graph is deliberately acyclic:

```text
model.v2
review additions.v2
candidate universe.v2
semantic classes.v2
        |
        +------> 16 ordered classification shards
                         |
                         v
                 classification root.v3
                         |
                         v
                 interaction_closure.v3
                         |
                         v
                 report / verification evidence
```

The root is created only after all shard bytes and shard raw digests exist.
No shard contains a root raw digest, and no closure/report/summary digest is
used to construct the root or a shard. The construction order is therefore:
upstream V2 inputs -> shard bytes -> shard raw digests -> root manifest/root
bytes -> root raw digest -> closure bytes. The report, negative matrix,
verification summary, and design spec remain outside the closure.

`interaction_closure.v3.json` binds exactly these five semantic C inputs:

- `declared_interaction_model.v2.json`;
- `interaction_review_additions.v2.json`;
- `interaction_candidate_universe.v2.json`;
- `interaction_semantic_classes.v2.json`; and
- `interaction_classifications.v3.json`, the classification root.

The root's `shards[]` inventory is the sole closure-level classification
binding and contains the raw bindings for all 16 shards. The closure does not
bind its own bytes, any individual shard in addition to the root, the report,
the negative test matrix, the verification summary, or the design spec. The
report may reproduce closure results and digests; the summary may record all
raw inventory digests. Neither is therefore a closure input.

## 7. File contracts

All JSON artifacts MUST be UTF-8, emitted with deterministic key and array
ordering, and validated as closed objects. Unknown top-level keys are rejected.
The JSON representation is a wire/document representation only. No persisted
semantic identity may hash JSON, Serde output, a language-native object, or an
implementation-defined serialization.

Unless a field is explicitly nullable below, it is required and non-null.

The exact C JSON schema registry is closed and fixed before implementation:

```text
declared_interaction_model.v2.json
  = manafold.m2.5.c.declared-interaction-model.v2
interaction_review_additions.v2.json
  = manafold.m2.5.c.interaction-review-additions.v2
interaction_candidate_universe.v2.json
  = manafold.m2.5.c.interaction-candidate-universe.v2
interaction_semantic_classes.v2.json
  = manafold.m2.5.c.interaction-semantic-classes.v2

interaction_classifications.v3.json
  = manafold.m2.5.c.interaction-classifications.v3
classification_shards/interaction_classifications.0000.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0001.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0002.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0003.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0004.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0005.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0006.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0007.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0008.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0009.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0010.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0011.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0012.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0013.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0014.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
classification_shards/interaction_classifications.0015.v3.json
  = manafold.m2.5.c.interaction-classifications-shard.v3
interaction_closure.v3.json
  = manafold.m2.5.c.interaction-closure.v3
verification/c_negative_test_matrix.v3.json
  = manafold.m2.5.c.negative-test-matrix.v3
verification/c_verification_summary.v3.json
  = manafold.m2.5.c.verification-summary.v3
```

The checker requires exactly these schema values and exactly the 16 shard
paths; it rejects a missing or additional registry entry and never generates a
schema identifier at runtime. The four upstream V2 artifacts retain their V2
artifact/document contracts and raw paths. The five V3 artifact roles are the
classification root, the shard family, the closure, the negative-test matrix,
and the verification summary. Every V3 artifact retains
`model_id = "declared-interaction-model.v2"` where a model ID is
required. The V3 version is a publication/downstream document version only;
it does not revise the V2 semantic model or either V1 identity preimage.


Where an artifact has a declared-model path or raw binding, it must point to
`sources/m2_5/closures/C/declared_interaction_model.v2.json`. No V2
classification artifact is an alternative reader input, and no V3 producer
may emit both the forbidden V2 monolith and the V3 bundle.
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

Every SHA-256 scalar that occurs inside a semantic CBOR preimage is a
`Sha256BytesV1` value. Its JSON projection is a string matching exactly
`[0-9a-f]{64}`: lowercase ASCII, exactly 64 hexadecimal characters, with no
whitespace, prefix, or alternate spelling. Before semantic encoding, the
checker decodes that validated JSON string into exactly 32 bytes and encodes
those bytes as one definite CBOR byte string (`0x58 0x20` followed by the 32
digest bytes). It never encodes the 64-character text as a CBOR text string.
This rule applies to `archive_member_sha256`, `additions_raw_sha256`, and
`raw_sha256`, including the `raw_sha256` slot of `EvidenceRefV1`, and to a
persisted `digest_hex` when a `DigestReferenceJsonV1` is embedded in a
semantic preimage. A malformed, uppercase, non-64-character, or non-hex value
is rejected before CBOR conversion. Raw artifact JSON retains the readable
lowercase hexadecimal projection; that projection is not an alternative
semantic preimage.

`B1FinalCitationRefV1` entries are sorted by the unsigned lexicographic bytes
of their canonical CBOR two-element array. The JSON representation uses the
closed object `{ "authority_id": ..., "citation_id": ... }`, and the checker
converts it to the exact two-position array before ordering or hashing. Each
pair must resolve to the corresponding accepted B1.Final authority and
citation node.

All enum vocabularies and the permitted field-to-vocabulary grammar are
declared in `declared_interaction_model.v2.json` below. A semantic value not
present in that closed V2 model vocabulary blocks the snapshot and requires a
versioned spec amendment; it may not be represented as a new free-text
synonym. The literal `not_applicable` is an exact unit variant, encoded as
`["not_applicable", null]`, when a declared dimension is irrelevant. The
source JSON must still carry the same field explicitly; omission is not
equivalent to `not_applicable`.

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
exact lowercase values in §7.3. `participant_refs_array` uses the exact
relation-shape rules in §8.1: unordered REV3 pairs are canonically ordered,
directional REV3 pairs preserve the source left-to-right order, and
card-trigger rows have their pinned unary card reference.
`supporting_requirement_ids_sorted_array` is sorted by canonical CBOR bytes
and rejects duplicates. `source_binding_union` is the
fixed discriminated union specified in §7.3. The candidate's terminal
disposition, review state, unresolved reason, review-domain assessments,
review rationale, class ID, and reconciliation status are not in this
identity; changing a review decision must not silently change source candidate
identity.

For an inherited REV3 candidate, the normalized source-origin value is `rev3`,
while the source binding includes the original candidate ID and every exact
source-row value. For a new candidate, the source binding includes the exact
targeted-review evidence that gives the candidate its identity. The original
REV3 candidate ID and all historical REV3 source values are preserved as data;
they are not rewritten or case-normalized.

New candidate IDs are deterministic and namespaced:

```text
targeted_higher_order_review:
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

CandidateIdentityV1 deliberately does not define a mechanical B2-terminal-data-to-interaction
derivation. B2 terminal classifications and assignments are card-side
capability evidence, not an interaction census, and there is no normative
one-to-one mapping from a B2 record to a new interaction candidate. Therefore
`b2_derived` is forbidden as a CandidateIdentityV1 `source_origin` and
`source_binding` kind;
`new_b2_derived` MUST equal zero. B2 data may support an inherited REV3
candidate or an explicitly reviewed targeted higher-order proposal, but it may
not create a candidate by family co-occurrence, capability name, lexical rule,
or any other inferred generator. A future need for B2-derived candidates
requires a versioned C spec amendment defining the complete deterministic
derivation, required set, and collision policy before implementation.

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
never includes rationale prose, evidence-reference arrays, candidate IDs,
source-instance IDs, review states, unresolved reasons, or review-domain
assessments. Concrete instances therefore reuse one class without copying its
definition or changing its identity.

These two identity contracts are the only new C semantic digest preimages.
Artifact bindings use raw SHA-256 of exact file bytes, and source-tree
fingerprints use the accepted B2 algorithm in §13.
Any additional semantic identity field, closure identity, or ad-hoc
source-tree identity may be persisted only after this specification is amended
with its complete envelope and fixed preimage.

### 7.1 `declared_interaction_model.v2.json`

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
review_states
unresolved_reasons
review_domain_vocabulary
review_domain_applicability_vocabulary
context_dimensions
authority_policy
participant_kind_vocabulary
participant_role_vocabulary
context_value_vocabulary
temporal_value_vocabulary
```

Required values include:

```text
schema = "manafold.m2.5.c.declared-interaction-model.v2"
model_id = "declared-interaction-model.v2"
coverage_scope = "pairwise_plus_review_outliers"
terminal_dispositions includes:
  required_interaction
  not_an_interaction_with_proof
  out_of_declared_scope_with_reason
review_states = resolved, unresolved
unresolved_reasons includes:
  insufficient_pair_relation_authority
  insufficient_boundary_relation_evidence
  missing_required_review_evidence
review_domain_vocabulary includes exactly:
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
review_domain_applicability_vocabulary = applicable, not_applicable, unresolved
```

`out_of_declared_scope_with_reason` is permitted only where the candidate is
provably outside the declared model boundary and the record states the exact
boundary and evidence. It is not a substitute for unresolved review. Every
current candidate still requires exactly one `review_state`; only a candidate
with `review_state = resolved` receives a terminal disposition.

The following lowercase values are forbidden as terminal dispositions or
review-state values:

```text
ambiguous_requires_review
requires_review
unknown
provisional
pending
```

`unresolved` is reserved for the closed `review_state` vocabulary and the
`review_state_metrics.unresolved` count. It MUST NOT appear in
`terminal_dispositions`, `reconciliation_status`, or `gate_status`, and it is
not itself a terminal disposition.

The model MUST explicitly preserve directionality, participant role, host
relationship, zone/visibility, timing and temporal ordering, source versus
affected object, controller versus owner, replacement/layer dependency,
trigger/LKI context, information dependency, decision actor, and higher-order
arity.

The following closed V2 model vocabularies are normative. The JSON model stores the
exact lowercase ASCII identifiers; every identifier is a unit enum and is
encoded as `[identifier, null]` in a semantic preimage. The checker rejects
unknown identifiers and rejects a value from the wrong vocabulary for a field.
Uppercase labels used elsewhere for inherited REV3/B2 source values or for the
accepted M2.5 gate vocabulary are external/raw contracts; they are not
normalized C semantic enum values.

```text
participant_kind_vocabulary =
  ability, card, copiable_value, deck, effect, event, object, permanent,
  player, requirement_family, source_instance, spell, token, zone

participant_role_vocabulary =
  affected, controller, copied_source, copy_result, decision_actor,
  destination_zone, origin_zone, ordered_participant, owner,
  replacement_actor, source, target, trigger_source

context_value_vocabulary =
  zone =
    battlefield, command_zone, exile, graveyard, hand, library, outside_game,
    stack, zone_agnostic, not_applicable
  visibility =
    controller_only, hidden_to_actor, identity_hidden, not_applicable, owner_only,
    private, public
  timing =
    activation_time, cast_time, combat_time, continuous_effect, not_applicable,
    resolution_time, state_based_check, trigger_time, turn_boundary,
    zone_change_time
  temporal_order =
    after, before, during, not_applicable, sequential, simultaneous, until,
    while
  source_affected_relation =
    both_affected, no_effect_relation, not_applicable, source_affected,
    source_affects_other
  control_ownership_relation =
    control_changes, cross_controller, cross_owner, not_applicable,
    ownership_changes, same_controller, same_owner
  replacement_layer_relation =
    copy_layer, control_layer, layer_dependency, no_replacement_or_layer,
    not_applicable, pt_layer, replacement_effect, type_layer,
    zone_change_replacement
  trigger_lki_relation =
    intervening_if, last_known_information, no_trigger_lki, not_applicable,
    trigger_condition, triggered_event
  information_relation =
    hidden_identity, known_to_controller, known_to_owner, no_information_dependency,
    not_applicable, private_look, public_identity, random_unknown
  decision_actor_relation =
    active_player, controller, no_decision, not_applicable, opponent, owner,
    rules_forced, target_player

temporal_value_vocabulary =
  dependency_order =
    dependency_ordered, no_temporal_dependency, not_applicable
  duration =
    duration_limited, indefinite, not_applicable, until_event
  replacement_order =
    after_effect, before_effect, no_temporal_dependency, not_applicable,
    same_event
  trigger_order =
    deferred, immediate, no_temporal_dependency, not_applicable

additional enum vocabularies used by the fixed records:
  coverage_scope = pairwise_plus_review_outliers
  arity = unary, binary, higher_order
  directionality = directed, none, symmetric
  host_relationship = cross_host, not_applicable, same_host
  authority_kind = b1_final, b2, c_review, rev3
  assignment_role = primary, supporting
  lifecycle = active, active_unassigned
  source_origin = rev3, targeted_higher_order_review
  scope = cross_deck, intra_deck, unary_or_higher_order
  relation = declared_card_trigger, directional_binary, reviewed_higher_order,
             unordered_binary
  review_kind = targeted_higher_order_review
  source_kind = b2_assignment, b2_classification, rev3_row
  terminal_disposition = required_interaction, not_an_interaction_with_proof,
                         out_of_declared_scope_with_reason
  review_state = resolved, unresolved
  unresolved_reason = insufficient_pair_relation_authority,
                      insufficient_boundary_relation_evidence,
                      missing_required_review_evidence
  review_domain = triggers_and_lki, replacement_layers_and_dependency,
                  copy_and_token_creation,
                  target_legality_protection_and_identity,
                  control_and_ownership, commander_and_format,
                  hidden_information_and_visibility,
                  ordering_and_temporal_dependencies,
                  source_versus_affected_identity,
                  controller_owner_and_decision_actor,
                  higher_order_interactions
  review_domain_applicability_vocabulary = applicable, not_applicable, unresolved
  reconciliation_status = unchanged, stale_rev3_candidate,
                          removed_not_interaction, merged_semantic_duplicate,
                          new_targeted_higher_order_candidate
```

The field grammar is closed as well: `participant_kind` uses
`participant_kind_vocabulary`; `role` uses `participant_role_vocabulary`;
the ten `context_dimensions` slots use the corresponding `context_value`
vocabulary; and the four `temporal_semantics` slots use the corresponding
`temporal_value` vocabulary. `coverage_scope`, terminal dispositions, review
states, unresolved reasons, review domains, review-domain applicability,
reconciliation statuses, and source binding kinds use their exact closed
entries above. `semantic_ref` is not a semantic label:
it is an exact resolvable source identifier from the pinned ledgers. If a
needed value is absent, the snapshot is `BLOCKED` until this V2 model is
amended; no free text or synonym is admitted.

### 7.2 `interaction_review_additions.v2.json`

This file is the upstream authority for C-reviewed candidate proposals that
do not have a REV3 source row. It is created before candidate generation and
is never generated from the candidate universe, classifications, closure,
report, or verification summary. It contains exactly:

```text
schema = "manafold.m2.5.c.interaction-review-additions.v2"
```

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

`review_kind` is exactly `targeted_higher_order_review`. The record ID is a
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
declared_model_path = "sources/m2_5/closures/C/declared_interaction_model.v2.json"
declared_model_raw_sha256
source_evidence_refs_sorted_array
```

The source-evidence array contains the exact external REV3/B2/B1.Final source
identities used to review the additions, sorted by the canonical CBOR bytes of
`EvidenceRefV1`. It must not contain any candidate, class, classification,
closure, report, or verification-summary digest. The artifact's raw SHA-256 is
bound by the candidate universe and closure, so no self-reference is
introduced. `review_record_count` is recomputed from the array. An empty
`review_records` array is valid for a corrected C V3 snapshot and proves that
no targeted higher-order candidate authority was silently introduced.

`review_record_id` has the closed form
`ira.v1/<lowercase-ascii-stable-key>`, where the key matches
`[a-z0-9][a-z0-9._-]*`; IDs are unique in bytewise order. It is an upstream
reviewer's stable authority key, not a digest and not a generated candidate
ID. Each `participant_source_refs` entry is the JSON projection of
`ParticipantSourceRefV1`; the source kind and locator must resolve to one of
the pinned REV3 rows or accepted B2 records.

Every `targeted_higher_order_review` candidate must reference one and only one
record in this artifact. Unknown, duplicate, orphan, or path-substituted review
records are rejected. A review record cannot create a rules authority or
replace a missing B1.Final citation; it can only propose a source-grounded C
candidate within the declared model.

### 7.3 `interaction_candidate_universe.v2.json`

This file is the mechanically complete candidate ledger. It contains:

```text
schema = "manafold.m2.5.c.interaction-candidate-universe.v2"
```

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
`interaction_review_additions.v2.json`, the pinned REV3 source, and the
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
rev3
targeted_higher_order_review
```

Every inherited REV3 candidate MUST appear exactly once with its original
`candidate_id`. A newly derived candidate MUST have a deterministic new ID,
the `targeted_higher_order_review` source origin, and exact review-additions
evidence. A candidate MUST NOT disappear because it is hard to review.

`source_binding` is a closed discriminated union. The JSON form has a required
`kind` field and only the fields for that kind; the canonical identity form is
the fixed-position enum `[kind, payload]`. It has these variants:

```text
rev3 {
  archive_member,
  archive_member_sha256,
  row_ordinal,
  source_columns,
  source_values
}

targeted_higher_order_review {
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
  "rev3",
  [archive_member, archive_member_sha256, row_ordinal,
   source_columns_array, source_values_array]
]

[
  "targeted_higher_order_review",
  [additions_path, additions_raw_sha256, review_record_id, review_kind_enum,
   participant_source_refs_ordered_array, review_evidence_refs_sorted_array]
]
```

Where C records an accepted B2 classification identity, it uses the existing
B2 `DigestReferenceJsonV1` projection. The checker validates its exact
persisted fields (`envelope_id`, `algorithm_id`, `semantic_domain`,
`payload_codec_id`, `input_schema_id`, and `digest_hex`) against the accepted
B2 identity and then converts those exact fields to the normative six-position
`DigestReferenceV1` CBOR array from `docs/STATE_HASHING.md`; the sixth slot is
the 32-byte digest obtained by decoding the validated lowercase `digest_hex`,
not the hex text. It does not hash the JSON projection or invent/rederive a
different identity. Each B2 assignment reference is the fixed array
`[family_id, assignment_ordinal, precise_semantic_definition]`, sorted by its
canonical CBOR bytes. Each participant source reference is an exact pinned
locator, and evidence references use `EvidenceRefV1`. JSON field order has no
effect on any of these conversions.

For the normalized `rev3` binding, `archive_member` is exactly
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
`supporting_requirement_ids`. There is no nullable `rev3` row field on a
non-`rev3` candidate.

C V3 has no `b2_derived` source binding. B2 classification, assignment, and
boundary records remain accepted evidence references for inherited or
targeted-review candidates, but no B2 terminal record mechanically creates a
new candidate. Any attempted `b2_derived` record is rejected with
`B2_DERIVED_FORBIDDEN_V1`; a nonzero `new_b2_derived` count is invalid.

For `targeted_higher_order_review`, `additions_path` is exactly
`sources/m2_5/closures/C/interaction_review_additions.v2.json`,
`additions_raw_sha256` is its exact raw file digest, and `review_record_id`
must resolve to exactly one record in that artifact. `review_kind` is the
unit enum `targeted_higher_order_review`; the ordered participant references
and sorted evidence references must equal the resolved review record. This
variant is the complete source binding for the candidate and does not pretend
to have a REV3 row.

The source union is mutually exclusive: a record with `kind = rev3` cannot
carry targeted-review fields, and a targeted variant cannot carry REV3 fields.
Unknown kinds and unknown variant fields are rejected. Historical uppercase
REV3 values are permitted only inside the exact `source_values` array and are
never used as normalized C enum identifiers.

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
by the declared model, using `not_applicable` explicitly where appropriate.
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
these keys. Before assigning an index, it rejects duplicate canonical source
instance tuples within a candidate; identical tuples may not become distinct
instances merely because they receive indexes `0` and `1`. It also rejects
duplicate IDs, an instance whose candidate is absent, and any classification
reference not present in this ledger.

`reconciliation_status` is an accounting field, not a terminal semantic
disposition. The allowed values are:

```text
unchanged
stale_rev3_candidate
removed_not_interaction
merged_semantic_duplicate
new_targeted_higher_order_candidate
```

The status is accompanied by a source-grounded reason. A stale, removed, or
merged row still receives a record in `interaction_classifications` and an
explicit `review_state`; accounting status never permits silent omission. A
stale or removed row may receive `not_an_interaction_with_proof` only when
positive source-grounded evidence disproves the declared relation. If that
proof is unavailable, the record remains `unresolved` with its closed reason;
"removed" is never a shortcut to a terminal non-interaction disposition.

The candidate universe MUST preserve these normalized relation shapes for
inherited REV3 rows:

```text
unordered_binary
directional_binary
declared_card_trigger
```

It MUST also represent explicitly reviewed higher-order candidates with the
closed combination `scope = unary_or_higher_order`,
`relation = reviewed_higher_order`, and an ordered participant list whose
derived finite count is greater than two. It MUST reject using
`declared_card_trigger`, `directional_binary`, or `unordered_binary` for such a
candidate unless the actual reviewed semantics independently satisfy that
relation.

### 7.4 `interaction_semantic_classes.v2.json`

This file contains one canonical definition for each reusable terminal
semantic class. It contains:

```text
schema = "manafold.m2.5.c.interaction-semantic-classes.v2"
```

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
`interaction_classifications.v3.json` separately binds
`semantic_classes_raw_sha256`.

Allowed `arity` values are `unary`, `binary`, and `higher_order`. A
`higher_order` class MUST state the exact finite participant count by the
normative derived rule `participant_count := len(participant_roles)` and must
provide ordered participant roles. The count is not a second free field in the
class identity; the checker recomputes it from the closed role array and
exposes it in class metrics and the report. The arity/count relation is closed:
`unary` requires count `1`, `binary` requires count `2`, and `higher_order`
requires count greater than `2`. A higher-order class is not an unbounded
N-way claim.

Allowed `directionality` values are:

```text
none
symmetric
directed
```

`directed` classes MUST identify source and affected roles and preserve the
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
same_host
cross_host
not_applicable
```

The value is semantic: it is not a display label derived from deck names.
Context dimensions MUST state the relevant zone, visibility, timing/phase or
event order, information identity, control/ownership, replacement/layer, and
decision context. If a dimension is not relevant, the class says
`not_applicable`; it must not omit the field.

`b2_family_refs[]` and `b2_boundary_refs[]` identify exact current B2 families
and their exact `precise_semantic_definition` strings. Each required family
reference states its lifecycle and assignment role. A card-derived class may
reference only a valid terminal assignment to an `active` family. An
`active_unassigned` family is rejected in that position.

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

### 7.5 V3 classification bundle: `interaction_classifications.v3.json` and fixed shards

The V3 classification authority is a root manifest plus exactly 16 bounded
shard files. The root contains no candidate classification records. It is the
sole root for completeness, ordering, shard boundaries, and raw shard
bindings. The root contains exactly:

`schema = "manafold.m2.5.c.interaction-classifications.v3"`

```text
schema
model_id
candidate_universe_raw_sha256
semantic_classes_raw_sha256
partition_scheme
classification_count
shard_count
shards
```

The root has `model_id = "declared-interaction-model.v2"` and
`partition_scheme = "candidate-order-fixed-chunk-1000-v1"`. This V3
publication-layout version does not create a new semantic model version.
`classification_count` equals the recomputed
`interaction_candidate_universe.v2.json.candidate_count`, which MUST be
15,679 for the pinned C snapshot. `shard_count` is exactly `16`.
`shards[]` contains exactly 16 entries, ordered by `shard_index`, and
every entry contains exactly:

```text
shard_index
path
ordinal_start
ordinal_end_exclusive
record_count
first_candidate_id
last_candidate_id
raw_sha256
```

`path` is one of the exact paths listed in §6, with
`shard_index = 0` using
`classification_shards/interaction_classifications.0000.v3.json` and
`shard_index = 15` using
`classification_shards/interaction_classifications.0015.v3.json`.
`raw_sha256` is the lowercase raw SHA-256 of that shard's exact UTF-8 JSON
bytes. `first_candidate_id` and `last_candidate_id` are the exact
candidate IDs at the inclusive boundaries of that shard's ordered record
sequence. The root manifest is written only after all shard bytes and raw
digests exist. The root has no `candidate_classifications`,
`classification_root_raw_sha256`, or other unlisted field.

The publication order is deterministic and does not depend on the order in
which a producer happened to enumerate an in-memory object. Let
`candidate_ids` be the unique candidate IDs in the authoritative
`candidates[]` array. Sort them by unsigned lexicographic comparison of the
complete canonical-CBOR encoding of each candidate ID as a UTF-8 text
string. The zero-based position in that sorted sequence is the
`classification_ordinal`. This ordering is a V3 layout order only; it does
not change `candidate_id`, CandidateIdentityV1, source-instance identity,
reconciliation, or semantic review state.

For shard index `i` in the closed range `0..15`, let
`N = 15,679`. The exact fixed range is:

```text
ordinal_start(i)         = 1000 * i
ordinal_end_exclusive(i) = min(1000 * (i + 1), N)
record_count(i)          = ordinal_end_exclusive(i) - ordinal_start(i)
```

Therefore shards `0000` through `0014` contain exactly 1,000 records
each, and shard `0015` contains exactly 679 records. These are fixed
record-count chunks, not a byte-size-dependent partition. If the recomputed
candidate count is not 15,679, or if any of the 16 required ranges/counts
differs, V3 is `BLOCKED` pending a versioned layout amendment; the producer
may not silently reduce, increase, rename, or repartition the shard set.

Each shard file is a closed object with exactly:

`schema = "manafold.m2.5.c.interaction-classifications-shard.v3"`

```text
schema
model_id
candidate_universe_raw_sha256
semantic_classes_raw_sha256
partition_scheme
shard_index
shard_count
ordinal_start
ordinal_end_exclusive
record_count
first_candidate_id
last_candidate_id
candidate_classifications
```

Each shard has `model_id = "declared-interaction-model.v2"`,
`partition_scheme = "candidate-order-fixed-chunk-1000-v1"`, the same
candidate-universe and semantic-class raw bindings as the root, and the exact
shard metadata from the root `shards[]` entry. A shard does not contain
`classification_root_raw_sha256`: the root binds shards, so adding the
root digest to shards would create an unnecessary cycle.

Each `candidate_classifications[]` object has exactly the unchanged V2
candidate-level grammar:

```text
candidate_id
review_state
review_domain_assessments
terminal_disposition
interaction_class_id
unresolved_reason
source_instance_context_mappings
reconciliation
review_rationale
evidence_refs
```

All state, domain-assessment, source-instance, reconciliation, rationale,
evidence, class-reference, and identity rules in this specification apply
unchanged to these records. A class definition may not be copied into a
classification record. V3 moves records between files; it does not split,
merge, reorder semantically, or otherwise reinterpret them.

The checker MUST recompute the sorted candidate sequence, every ordinal range,
every shard record count, the first/last candidate boundaries, and the complete
candidate-ID coverage. Each candidate ID must occur exactly once across the
16 shards, and the union must equal the candidate universe exactly. The root's
`classification_count` must equal the sum of all shard `record_count`
values and the candidate count. A root entry must match the actual shard
index, exact path, range, boundary IDs, count, and raw bytes. Missing, extra,
duplicated, out-of-range, misordered, non-contiguous, or cross-boundary
records are invalid.

Each shard's exact UTF-8 JSON bytes MUST be no larger than
`50 * 1024 * 1024 = 52,428,800` bytes. This fixed upper bound keeps every
publishable shard well below the hosting hard-file limit. If a shard exceeds
the bound, C is `BLOCKED`; the implementation may not add a seventeenth
shard, choose a byte-size-dependent partition, rename a shard, or retain the
V2 monolith as a workaround. A future change to the bound or shard count
requires a versioned layout amendment.

The root and all shards use the existing C JSON wire profile: UTF-8, no
unknown keys, deterministic key/array ordering, and exact lowercase raw digest
bindings. Raw artifact SHA-256 values are file bindings only; no JSON bytes or
language-native serialization are used as a CandidateIdentityV1 or
InteractionClassIdentityV1 preimage.

### 7.6 `interaction_closure.v3.json`

This is the sole semantic C closure artifact. It contains:

```text
schema = "manafold.m2.5.c.interaction-closure.v3"
```

```text
schema
model_id
bound_semantic_inputs
external_prerequisite_identities
candidate_reconciliation
semantic_class_metrics
review_state_metrics
source_instance_metrics
gate_status
flags
```

`bound_semantic_inputs` contains exactly these five entries and no others:

```text
declared_interaction_model.v2.json
interaction_review_additions.v2.json
interaction_candidate_universe.v2.json
interaction_semantic_classes.v2.json
interaction_classifications.v3.json
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

`new_b2_derived` is required to equal `0` in every C V3 closure because
`b2_derived` is forbidden by the V1 source-origin contract. The checker
recomputes this value from the candidate ledger and rejects every attempted
B2-derived candidate as invalid under this V3 publication contract. The required
B2-derived set is empty
in this version, so no missing required B2-derived candidate can exist; a
future non-empty set requires the versioned amendment described in §7.0.1.

`review_state_metrics` MUST report exactly these four recomputed counts:

```text
resolved_required_interaction
resolved_not_an_interaction_with_proof
resolved_out_of_declared_scope_with_reason
unresolved
```

The first three fields count only classifications with
`review_state = resolved` and the corresponding terminal disposition.
`unresolved` counts classifications with `review_state = unresolved`; it is
not a terminal-disposition count. The checker MUST recompute all four counts
from the candidate classifications and require:

```text
candidate_count
  = resolved_required_interaction
  + resolved_not_an_interaction_with_proof
  + resolved_out_of_declared_scope_with_reason
  + unresolved
```

Hand-entered aggregates cannot make either closure mode pass validation.

There are two distinct valid result modes:

1. A C `PASS` closure requires `unresolved = 0`, every candidate
   classification to have `review_state = resolved`, and the three resolved
   disposition counts to partition the complete candidate universe. Its
   `gate_status` is the exact C-PASS form below.
2. A structurally valid blocked C snapshot may have `unresolved > 0`. When
   the B1, B1.Final, B2, and master-drift prerequisites are `PASS` and the
   unresolved review state is the sole C blocker, its exact gate form is:

```text
CLASSIFICATION_REFERENCE_CLOSURE        = PASS
OFFICIAL_RULE_CITATION_CLOSURE          = PASS
DECLARED_INTERACTION_MODEL_CLOSURE      = BLOCKED
REV2_REUSE_RATIO_REPRODUCIBLE           = BLOCKED
RANKING_UNCERTAINTY_PROPAGATION         = BLOCKED
```

The blocked form MUST retain the exact false flags below and MUST expose every
unresolved candidate with one closed `unresolved_reason`. It is a
representable review snapshot, not a successful C closure or an acceptance
result. If another prerequisite is unavailable or non-terminal, the checker
reports that prerequisite as `BLOCKED` and does not upgrade the snapshot to a
C acceptance.

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
blocked as shown above. An unresolved candidate is legal only in the blocked
form and MUST be rejected if `DECLARED_INTERACTION_MODEL_CLOSURE = PASS`.

### 7.7 `INTERACTION_MODEL_REPORT.md`

The report is a human-readable projection of the C artifacts. It MUST include:

- the exact source/master/archive identities;
- the authority graph and acyclic digest policy;
- candidate totals and all reconciliation deltas;
- semantic class count and class-shape totals;
- review-state metrics, resolved disposition totals, and unresolved reasons/count;
- high-risk review-set coverage;
- B2 and B1.Final binding summary;
- V2→V3 migration-parity result and source/target record counts;
- Git-history publishability-preflight result, verified base, and violation
  inventory;
- closure status and its raw artifact SHA-256, when available;
- the exact `gate_status` values and boolean flags from §7.6; and
- exact commands and their actual statuses.

The report MUST NOT be a closure input. If it repeats closure results or
digests, the checker treats it as derived documentation.

### 7.8 `c_negative_test_matrix.v3.json`

This file is a fixed verification contract, not a semantic input. It contains
the exact mandatory C-001 through C-042 matrix plus the supplemental
publication-layout coverage listed in §11.2, each with:

```text
schema = "manafold.m2.5.c.negative-test-matrix.v3"
```

```text
case_id
mutation
expected_status
expected_reason_code
target_artifact
```

It is checked for exact inventory and stable target reason codes. It is not
bound into the closure digest.

### 7.9 `c_verification_summary.v3.json`

This file is an evidence record and remains fully outside the closure. It may
be provisional at H_exec with commands marked `NOT_RUN`; it becomes the
post-execution summary only in Phase C.

```text
schema = "manafold.m2.5.c.verification-summary.v3"
```

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
migration_parity
publishability_preflight
checker_identities
evidence_protocol
evidence_export
```

`execution_commit` is H_exec. `artifact_digests` records raw SHA-256 values of
the exact bytes of every non-summary C inventory file: `C_DESIGN_SPEC.md`,
the four V2 upstream artifacts, the V3 classification root, each of its 16
exact shard files, the V3 closure, report, and negative matrix. The summary
itself is excluded from this digest list. The summary does not
record or bind its own digest. The final summary must never claim `PASS` for
an unexecuted command. For a structurally valid blocked snapshot, the summary
may record `c_result = BLOCKED` and the actual blocked prerequisite/domain
details, but it MUST record the recomputed review-state metrics and MUST NOT
describe the C closure as accepted. A later C `PASS` requires a new summary
for the corrected source snapshot and a fresh evidence lineage when source
bytes or checker bytes changed.

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

`migration_parity` and `publishability_preflight` are evidence records
described in §7.10. They are outside the closure and are not semantic inputs.

`evidence_protocol` has the fixed fields:

```text
H_exec
modified_path
H_evidence_relation = "direct_child_summary_only"
```

It does not embed a self-referential H_evidence commit digest; the verifier
derives the unique evidence commit from ancestry and proves the relation in
§13.2.

### 7.10 V2→V3 migration parity and publishing preflight

The V3 statement that classification storage changed only layout is a
creation-time migration assertion. It MUST be proven before a publishable V3
`H_exec` is accepted. The exact migration source is the archived V2
`H_exec`:

```text
source_execution_commit =
  43bf3ccc6ff639c900914947ee0883b4731b8409
source_classifications_path =
  sources/m2_5/closures/C/interaction_classifications.v2.json
```

The local-only archival ref
`refs/heads/archive/m2-5-c-v2-delivery-blocked-0886a177` may be used to
reach this historical V2 lineage during creation. The ref is not pushed and
is not a V3 authority. The V2 monolithic classification file is read only as
migration evidence; it is not a V3 inventory file, closure input, root input,
or replacement authority, and it MUST NOT occur in the reachable history of a
publishable V3 branch.

The migration parity procedure is exact:

1. Read the four V2 upstream files from
   `git show 43bf3ccc6ff639c900914947ee0883b4731b8409:<path>` and require
   the V3 files at the same four paths to be byte-identical. The four paths
   are `declared_interaction_model.v2.json`,
   `interaction_review_additions.v2.json`,
   `interaction_candidate_universe.v2.json`, and
   `interaction_semantic_classes.v2.json`.
2. Read and validate the exact V2 monolith bytes at
   `source_classifications_path`. Extract its
   `candidate_classifications[]` array without changing any field, scalar,
   object member, or nested-array value.
3. Concatenate the `candidate_classifications[]` arrays from V3 shards
   `0000` through `0015` in shard-index order. Require exactly 15,679
   records, unique candidate IDs, and one-to-one candidate-ID coverage with
   the V2 monolith.
4. Compare each V2 and V3 record by the complete candidate ID and by the
   existing deterministic canonical-JSON bytes of the complete record. This
   comparison is field-, scalar-, object-, and array-value exact; no
   semantic normalization is permitted.
5. Reorder the V2 records only by the already specified
   `candidate-order-fixed-chunk-1000-v1` candidate order and require the
   resulting sequence to equal the concatenated V3 sequence record-for-record.
   The only permitted transformation is this ordering plus fixed chunk
   partitioning into 16 files. Review state, domain assessments, terminal
   fields, source mappings, rationale, evidence references, and every other
   classification field MUST remain unchanged.

The final V3 verification summary MUST record this result in a closed
`migration_parity` object with exactly:

```text
migration_parity
  source_execution_commit
  source_classifications_path
  source_classifications_raw_sha256
  source_upstream_artifact_digests
  target_classification_root_path
  target_shard_paths
  source_record_count
  target_record_count
  record_equality
  ordering_transform
  result
```

`source_upstream_artifact_digests` contains the raw SHA-256 for each of the
four source paths and `target_shard_paths` is the exact ordered list of the
16 V3 shard paths. A successful parity result is
`result = PASS`, `record_equality = PASS`, and
`source_record_count = target_record_count = 15,679`. Migration parity may
pass while the semantic C result remains `BLOCKED` because unresolved review
is a separate semantic condition. If the archived source is unavailable or
any upstream byte or candidate record differs, migration parity is
`BLOCKED` or `FAIL` and the V3 source snapshot is not publishable.

Migration parity is a one-time V2→V3 creation gate. Historical descendant
validation MUST validate the recorded summary bytes and parity bindings but
MUST NOT require the local archive ref, re-read the V2 monolith, or depend on
private archive availability after the V3 evidence has been created.

The V3 publishing preflight has two time-separated executions: a creation-time
Git-history gate whose result is persisted for H_exec, and a post-commit
acceptance gate whose result is evaluated for H_evidence but is not persisted
in the summary. Both executions MUST inspect the objects that the V3
evidence-bearing branch introduces relative to the verified base, not every
object reachable from every local ref. For the creation-time execution,
resolve the exact base ref `origin/master` to its verified commit
`verified_base_commit`, require that commit to be an ancestor of H_exec, and
execute the equivalent of:

```text
git rev-list --objects H_exec --not origin/master
```

The `origin/master` name MUST resolve to the recorded
`verified_base_commit`; the archival ref and other unrelated local refs MUST
not be included in this enumeration. Resolve the unique object IDs emitted by
that command with:

```text
git cat-file --batch-check='%(objectname) %(objecttype) %(objectsize)'
```

Every resolved blob MUST be no larger than the hosting hard limit:

```text
hosting_hard_limit_bytes = 100 * 1024 * 1024 = 104857600
```

The check is on reachable Git history, not only the current tree. A blob that
was introduced and later deleted remains a rejection. Any blob above the
limit fails with `PUBLISHABLE_HISTORY_OVERSIZE_BLOB`; an unresolvable object,
an invalid base ancestry, or a scope that includes unrelated refs is a
`BLOCKED`/failure condition with a stable reason. The exact path
`sources/m2_5/closures/C/interaction_classifications.v2.json` MUST also be
absent from the full `H_exec`-reachable path set, independently of the
size scan. Prove that path-history condition by executing:

```text
git rev-list --full-history H_exec -- \
  sources/m2_5/closures/C/interaction_classifications.v2.json
```

The command MUST produce no commits. Any output means that the forbidden
monolith existed at some point in the H_exec ancestry and fails with
`LEGACY_MONOLITH_REACHABLE`; the check MUST start at H_exec and MUST NOT use
`--all` or an unrelated local ref. The same exact path-history check is
repeated for H_evidence after its creation.

The final summary MUST record the H_exec creation-time result in a closed
`publishability_preflight` object with exactly:

```text
publishability_preflight
  base_ref
  verified_base_commit
  checked_commit
  introduced_object_count
  introduced_blob_count
  hosting_hard_limit_bytes
  oversized_blobs
  forbidden_legacy_paths
  result
```

`oversized_blobs[]` records object ID, path, and size for every violation;
`forbidden_legacy_paths[]` records every forbidden V2 path occurrence. A
successful result has an empty violation set, `checked_commit = H_exec`, and
`result = PASS`. The final summary MUST NOT contain an H_evidence commit SHA
or a collection of checked commit SHAs: doing so would make the summary bytes
part of the H_evidence identity and create a self-reference.

After the direct-child H_evidence commit exists, the verifier MUST execute a
separate post-commit publishability preflight for H_evidence using the same
`verified_base_commit`. It MUST rerun the object enumeration, blob-size scan,
and exact legacy-path history check with H_evidence as the starting commit.
That post-commit result is an acceptance-gate result outside the H_evidence
summary and reproducible source inventory; it MUST NOT be written back to the
summary or any other source file. The H_evidence preflight is successful only
when that fresh scan passes, the H_exec preflight recorded in the summary
passed, and the direct-child summary-only evidence relation is valid. A
post-commit failure invalidates publishability and requires a new H_exec; it
must not be repaired by creating a third evidence commit.

The archival ref and unrelated local refs are excluded from both scans. The
preflight is evidence only, not a closure input, and it does not broaden the
master-drift allowlist.

## 8. Candidate generation and reconciliation

### 8.1 Complete inherited universe

The implementation MUST read the exact REV3 pair census and produce a
candidate record for all 15,679 source rows, unless live prerequisite identity
checking proves that the pinned input changed; in that event C is blocked.

The following source shapes are preserved:

```text
intra_deck + unordered_binary
cross_deck + directional_binary
unary_or_higher_order + declared_card_trigger
```

For inherited rows, the source-to-C normalization is a closed, exact mapping.
The left column is the required historical ASCII value in the pinned REV3
cell; the right column is the normalized lowercase C value. The checker must
compare the source cell byte-for-byte with the left column and emit only the
right column. It must not implement this as `lower()`, case folding, Unicode
normalization, synonym lookup, or any other generic transformation.

| Census column | REV3 source cell | normalized C value |
| --- | --- | --- |
| `scope` | `INTRA_DECK` | `intra_deck` |
| `scope` | `CROSS_DECK` | `cross_deck` |
| `scope` | `UNARY_OR_HIGHER_ORDER` | `unary_or_higher_order` |
| `relation` | `UNORDERED_BINARY` | `unordered_binary` |
| `relation` | `DIRECTIONAL_BINARY` | `directional_binary` |
| `relation` | `DECLARED_CARD_TRIGGER` | `declared_card_trigger` |

The exact inherited `participant_refs` derivation is also closed. The
`participant_kind` vocabulary includes `requirement_family` for the two
family columns; a family reference is therefore the fixed semantic value

```text
ParticipantRefV1(family_id) =
  [["requirement_family", null], family_id]
```

where `family_id` is the exact UTF-8 text of a member of the accepted B2
`families[].family_id` set. It is never represented as an `object`, `effect`,
`card`, or other participant kind merely because of a family name.

For each inherited row the checker requires the exact source scope/relation
pair from the table above and applies these shape rules:

1. For `INTRA_DECK` + `UNORDERED_BINARY`, read `left_family_id` and
   `right_family_id` exactly as source cells, validate both against the B2
   family set, and construct the two family references in source left/right
   order. The normalized `participant_refs` array is the same two references
   sorted by the unsigned lexicographic bytes of each complete canonical CBOR
   `ParticipantRefV1`. Equal family IDs retain both entries; multiplicity is
   part of the binary arity and is not deduplicated. The pinned REV3 census
   additionally requires `left_family_id` to be no greater than
   `right_family_id` by unsigned UTF-8 source bytes; the checker validates
   that source invariant, but does not use that text comparison as a
   substitute for the canonical-CBOR ordering of the normalized array.
2. For `CROSS_DECK` + `DIRECTIONAL_BINARY`, read the exact
   `left_family_id` and `right_family_id`, validate both against the B2 family
   set, and emit the two references in exactly that source order. Do not sort
   or swap them. The REV3 census provides an ordered left-to-right relation,
   but has no source/affected columns or equivalent authoritative direction
   labels. Consequently C preserves `left_family_id -> right_family_id` as
   the only supported directional orientation and MUST NOT reinterpret it as
   `source -> affected` (or the reverse). A directed semantic class that
   requires `source` and `affected` roles may use this candidate only when
   pinned review evidence independently establishes those roles; C may not
   infer them from the words `left` and `right`.
3. For `UNARY_OR_HIGHER_ORDER` + `DECLARED_CARD_TRIGGER`, require
   `pair_id == left_family_id == right_family_id` byte-for-byte. Let that
   value be `oracle_semantic_identity`. Select exactly one row in the pinned
   `inputs/deck_row_source_resolution_REV3.csv` whose
   `oracle_semantic_identity` equals that value. The selected row is required
   to provide the exact `deck_row_id`, `source_row_id`,
   `source_snapshot_file`, `source_line_number`, and
   `oracle_source_record_id` locator fields, and its
   `oracle_source_record_id` must join exactly once to
   `source/raw/source_record_index_REV3.csv` by
   `(oracle_semantic_identity, source_record_id)`. The participant reference
   is exactly one unary card reference:

   ```text
   participant_refs =
     [[["card", null], oracle_semantic_identity]]
   ```

   The `oracle_semantic_identity` field from that pinned resolution join is
   the card/OSI identity. It is not parsed from `candidate_id`, a card name,
   a family name, or any other text. The source-record ID and raw source
   record digest are locator/evidence fields, not a replacement for the card
   participant identity. The current pinned census has exactly 18 such rows
   and exactly one resolution row and one raw-record-index row for each
   identity; any missing, duplicate, or mismatched join blocks the snapshot.

The raw `supporting_requirement_ids` CSV cell is parsed by one relation-aware
rule. The cell is the exact UTF-8 text obtained from the pinned CSV parser;
its raw spelling remains in `source_values`. The only accepted syntax is a
top-level JSON array whose members are JSON strings. `[]` is the only valid
empty form. An empty CSV field, whitespace-only field, `null`, a non-array,
non-string members, an empty member, or a member with surrounding text that
would require trimming is rejected. JSON parsing does not authorize case
folding, Unicode normalization, or whitespace trimming of an ID. Exact
duplicate strings are rejected before ordering.

For binary rows, every parsed member must be an exact member of the accepted
B2 `families[].family_id` set; this is the 216-entry B2 family ID authority.
For the 18 declared-card-trigger rows, the cell must contain exactly one
member and that member must equal the joined `oracle_semantic_identity`
above (and therefore also `pair_id`, `left_family_id`, and `right_family_id`).
No other ID grammar is accepted. After validation and duplicate rejection,
the semantic `supporting_requirement_ids_sorted_array` is the parsed set in
ascending unsigned lexicographic order of each member's complete canonical
CBOR text-string encoding. This ordering is applied to the exact parsed
strings, not to their JSON spelling; the raw source cell is not reserialized
or hashed as the semantic value. The checker recomputes the array directly
from the pinned REV3 row and rejects any candidate whose normalized fields do
not equal this result.

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
removed/not-interaction reconciliation row (accounting only)
merged semantic duplicate
new targeted higher-order candidate
```

The categories are mutually exclusive per candidate lineage. A merged
duplicate is not deleted: its original candidate record remains, its
classification points to the canonical class, and the merge relationship is
recorded. A removed/non-interaction candidate remains in the ledger and gets
a classification; it gets `not_an_interaction_with_proof` only when the
required positive proof exists, otherwise it is `unresolved`. “Removed” is
accounting language, not silent disappearance or proof of non-interaction.

The count equation MUST hold:

```text
current_total
  = rev3_total
  + new_targeted_higher_order
```

`new_b2_derived` is always zero under C V3 and is included in the closure
metrics only as an explicit proof that the forbidden category was not emitted.
The equation is evaluated with the REV3 delta categories partitioning the
inherited rows. The checker also recomputes the partition from candidate
lineage instead of trusting the equation alone.

## 9. Semantic review protocol

### 9.1 Review state and terminal truth

Each candidate is assigned exactly one `review_state`:

```text
resolved
unresolved
```

A candidate with `review_state = resolved` is reviewed to exactly one terminal
disposition:

```text
required_interaction
not_an_interaction_with_proof
out_of_declared_scope_with_reason
```

An `unresolved` candidate is present, identity-bound, reconciled, attached to
its concrete source instances, and reviewed to the extent supported by the
available authority, but it has no defensible terminal semantic claim. It
must have `terminal_disposition = null`, `interaction_class_id = null`, and
exactly one `unresolved_reason` from the closed V2 model vocabulary. `unresolved` is
not a terminal disposition, an out-of-scope decision, or permission to omit a
candidate.

No candidate may remain ambiguous, provisional, pending, or otherwise use an
unclosed review state. A candidate may remain `unresolved` only in a closure
that explicitly reports `DECLARED_INTERACTION_MODEL_CLOSURE = BLOCKED`.
`unresolved = 0` is a hard gate for `PASS`.

`required_interaction` means the cited source and exact semantic boundaries
demonstrate a reusable interaction relation within the declared model.

`not_an_interaction_with_proof` means the reviewed source and boundaries
demonstrate that the candidate is only co-occurrence, independently composable,
or otherwise lacks the declared interaction relation. The rationale must name
the reviewed participants, the exact boundary distinction, and the positive
evidence that demonstrates the separation. A missing interaction record,
missing card-level evidence, or mere absence of proof is not sufficient.

`out_of_declared_scope_with_reason` means the candidate is explicitly outside
the finite declared model boundary. It requires a precise model-boundary
reference, positive evidence for that boundary determination, and cannot
conceal missing review.

For the current pinned inputs, the 15,661 family-pair candidates are not
automatically resolved by their REV3 census/co-occurrence rows or by the
independent B2 family boundaries. Those inputs do not, by themselves, prove
either an interaction or a boundary-separated non-interaction. Unless an
accepted relation authority supplies the missing candidate-specific proof,
such a candidate MUST remain `unresolved` with the applicable closed reason.

`unresolved_reason` is one primary reason, selected by this fixed precedence
when more than one deficiency is present:

```text
1. insufficient_pair_relation_authority
2. insufficient_boundary_relation_evidence
3. missing_required_review_evidence
```

The preconditions are mutually ordered as follows. Use
`insufficient_pair_relation_authority` when the candidate's terminal relation
determination requires a candidate-specific Pair/Relation Review Authority and
no such accepted authority covers it. Use
`insufficient_boundary_relation_evidence` when the required relation authority
exists or relation review is otherwise authorized, but the concrete boundary
facts do not support a terminal interaction, non-interaction, or scope claim.
Use `missing_required_review_evidence` when neither higher-precedence condition
applies but at least one required review-domain assessment is `unresolved` or
its required domain evidence is unavailable. The persisted reason is the
highest-precedence applicable value; the rationale and evidence references may
describe the other deficiencies without changing that primary value.

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

The implementation MUST maintain an authoritative assessment for every
candidate and every required review domain in
`review_domain_assessments[]`. The closed domain identifiers and their human
meaning are:

| `review_domain` | Required review area |
| --- | --- |
| `triggers_and_lki` | triggers and last-known-information behavior |
| `replacement_layers_and_dependency` | replacement effects and layer/dependency ordering |
| `copy_and_token_creation` | copy effects and token creation |
| `target_legality_protection_and_identity` | target legality, protection, and targeting identity |
| `control_and_ownership` | control and ownership changes |
| `commander_and_format` | Commander/format-specific behavior |
| `hidden_information_and_visibility` | hidden information and visibility boundaries |
| `ordering_and_temporal_dependencies` | ordering and temporal dependencies |
| `source_versus_affected_identity` | source versus affected object identity |
| `controller_owner_and_decision_actor` | controller/owner/decision-actor identity |
| `higher_order_interactions` | explicitly reviewed higher-order interactions |

The classification's eleven assessments are the authoritative membership and
coverage representation. The review set for a domain is mechanically derived
as the candidate IDs whose assessment has `applicability = applicable`;
reports and independent review exports are projections and are never an
alternative authority. Membership is evidence of review coverage, not an
inference that the candidate is required. The resulting terminal disposition
and rationale remain candidate-specific.

`applicable` requires source-grounded evidence that the domain is implicated
and that evidence must be recorded in the assessment. `not_applicable`
requires source-grounded evidence that the domain is not implicated; it does
not prove that the candidate is a non-interaction. `unresolved` records that
the accepted authorities cannot establish applicability or cannot provide the
required domain review evidence. It makes the candidate `unresolved` and
requires `unresolved_reason = missing_required_review_evidence`, subject to
the precedence rule in §9.1.

A required review set may be empty only when every current candidate has an
authoritative `not_applicable` assessment for that domain. If any candidate's
assessment is `unresolved`, the set is incomplete; the report MUST expose the
coverage gap and C remains `BLOCKED`. Empty-set declarations may not be used
to manufacture review completeness, and no membership may be inferred from
family names, capability names, keywords, co-occurrence, or hidden builder
logic.

### 9.4 Source evidence requirements

Every required class and candidate mapping MUST reference source-grounded
evidence from the pinned REV3 archive and exact B2/B1.Final records. The
reviewer may use an accepted repository record only through its immutable
identity. A capability name, card name, keyword, pair co-occurrence, model
label, or generated family label is not semantic proof by itself.

If a required interaction depends on an official domain absent from B1.Final,
the implementation MUST stop with `BLOCKED`. It must not invent a citation,
fetch live rules during generation, or silently downgrade the requirement.

### 9.5 Future pair/relation review authority

C V3 has no upstream Pair/Relation Review Authority for the 15,661 inherited
REV3 family-pair candidates beyond the census rows and the independently
defined B2 family boundaries. Those inputs establish candidate identity and
family-boundary facts, but do not by themselves prove an interaction or a
boundary-separated non-interaction. C MUST NOT turn that absence into
`not_an_interaction_with_proof`.

Moving one of those candidates from `unresolved` to a resolved terminal
disposition requires a separately approved, source-grounded Pair/Relation
Review Authority or an equivalent upstream authority. Such an authority must
bind the concrete candidate and participant identities, the reviewed boundary
facts, the positive relation or separation determination, and its immutable
source evidence. It must be a versioned upstream artifact with an explicit
schema and digest binding; it may not be hidden inside classifications, the
closure, the report, or the verification summary, and it may not create a
self-referential digest cycle.

Introducing that authority, or enabling any currently unresolved review domain,
requires a separately reviewed specification amendment before implementation.
Until then, unresolved candidates are the honest representation in a corrected
C V3 snapshot and keep `DECLARED_INTERACTION_MODEL_CLOSURE` `BLOCKED`.

## 10. Dedicated verifier contract

`python scripts/check_m2_5_c_interactions.py` is the authoritative C structural
and semantic-boundary verifier. It MUST be deterministic and fail closed.

The default invocation MUST verify the current C source and prerequisite
identities. `--negative-self-test` MUST run every mutation in the V3 negative
matrix: the fixed C-001 through C-042 cases. Every mutation must be rejected
with its expected reason code.

The checker MUST perform all of the following checks:

1. Resolve the exact repository root, C paths, archive root, and archive
   member identities.
2. Validate the C JSON schemas, closed vocabularies, deterministic ordering,
   the exact schema registry in §7, and the exact 26-file C inventory,
   including `C_DESIGN_SPEC.md` and the upstream review-additions artifact.
3. Execute or consume the current B1, B1.Final, B2, and master-drift gate
   results, rejecting any prerequisite that is not `PASS`.
4. Verify the exact REV3 archive and candidate source digests.
5. Verify the declared model scope and its finite higher-order boundary.
6. Recompute the complete candidate universe and reject a missing, duplicate,
   renamed, or extra inherited candidate; validate the closed source-binding
   union for every candidate.
7. Verify every current candidate has exactly one classification record with
   exactly one closed `review_state`; enforce the state-dependent nullability
   and `unresolved_reason` grammar in §7.5, including exactly one assessment
   for each of the eleven required review domains.
8. Verify candidate IDs, class IDs, and source-instance IDs are unique where
   required; verify digest-derived candidate-ID namespaces and reject identity
   collisions.
9. Verify every classification's reconciliation lineage and source binding.
10. Verify the complete candidate-universe source-instance ledger and every
    classification mapping against its owning candidate, and verify every
    review-domain assessment and its evidence references against the closed
    domain vocabulary and applicability grammar.
11. Reject an unknown OSI, family, assignment, or citation reference.
12. Reject a card-derived use of `ACTIVE_UNASSIGNED`.
13. Verify arity, participant count, role names, direction, edge orientation,
    host relationship, zones, timing, information, ordering, and temporal
    semantics.
14. Reject orphan source instances and duplicate or unbound mappings.
15. Verify every required class resolves to B1.Final citation graph nodes.
16. Recompute all candidate, class, reconciliation, and review-state counts,
    including the three resolved disposition counts and `unresolved`; reject
    any count that does not partition the complete candidate universe.
17. Recompute `CandidateIdentityV1` and `InteractionClassIdentityV1` from their
    prescribed fixed-position CBOR payloads and exact envelope metadata.
18. Recompute every inherited REV3 candidate's normalized scope, relation,
    participant references, and supporting-requirement array from its exact
    pinned census row using §8.1; reject alternate casing, inferred
    participant kinds, source/affected inference, malformed trigger locators,
    and noncanonical source-cell parses.
19. Verify raw file bindings and exact bound-input identities; reject any
    JSON/Serde-derived semantic digest or unbound identity.
20. Verify the upstream review-additions authority and its raw binding, and
    verify that the report, negative matrix, verification summary, and design
    spec are not closure inputs.
21. Verify the exact existing gate/flag vocabulary, distinguish the
    structurally valid C-blocked snapshot from the C-PASS form, preserve
    downstream blocked states, and reject any later-gate promotion.
22. Verify the negative-test matrix inventory and expected reason codes.
23. Validate both evidence-creation and historical-descendant evidence modes
    against the H_exec/H_evidence protocol, selecting historical evidence from
    the current summary bytes and its recorded `execution_commit` as required
    by §13.2.
24. In V3 creation mode, execute the exact V2→V3 migration-parity procedure
    against source H_exec
    `43bf3ccc6ff639c900914947ee0883b4731b8409`; in historical-descendant
    mode, validate its recorded evidence without requiring the private archive
    or local archival ref.
25. In creation mode, persist and validate the Git-history publishability
    preflight for H_exec against the verified `origin/master` base, then
    execute the separate post-commit preflight for H_evidence against the same
    recorded base without rewriting the summary; reject any oversized reachable
    blob or forbidden legacy monolith path and validate the closed evidence
    fields and stable reason codes.

The supplemental layout, migration-parity, and publishability checks in §11.2
are mandatory execution coverage in addition to the fixed C-001 through C-042
matrix. A structurally valid C snapshot cannot claim publication readiness
unless those supplemental checks have also executed with their specified
results.

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

A structurally valid blocked C snapshot is therefore a valid fail-closed
verification outcome with exit status `2`: all candidates remain accounted for,
their classifications obey the state grammar, and the closure reports the
blocked form in §7.6. The checker MUST accept that representation as
`BLOCKED`, while rejecting the same data if the closure claims C `PASS`. The
blocked outcome does not satisfy the C acceptance criteria and cannot promote
any downstream gate.

## 11. Negative-test matrix

The fixed V3 matrix contains exactly the following independent mutations. Each mutation
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
| C-027 | Use an invalid review-state/terminal-field combination in a PASS closure | `NONTERMINAL_DISPOSITION_ON_PASS` |
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
must accurately report evidence identities. C-008 and C-027 are PASS-claim
mutations: they reject an unresolved or non-terminal classification when the
closure claims `PASS`; they do not reject a structurally valid blocked snapshot
whose gate explicitly remains `BLOCKED`.

In addition to the fixed matrix, ordinary checker/unit regression coverage
MUST exercise each state-grammar mutation directly:

```text
resolved + terminal_disposition = null
resolved + unresolved_reason != null
unresolved + terminal_disposition != null
unresolved + interaction_class_id != null
unresolved + unknown unresolved_reason
```

Each mutation must be rejected with a stable validation reason, and the tests
must also prove that a fully bound unresolved candidate is accepted as
`BLOCKED` when the closure uses the blocked gate form. These regression tests
use `REVIEW_STATE_FIELD_MISMATCH` for the first four mutations and
`UNRESOLVED_REASON_UNKNOWN` for the fifth. They are supplemental execution
coverage; they do not change the fixed C-001 to C-042 matrix or its case count.

The supplemental checker/unit coverage MUST also exercise the review-domain
authority directly: a missing, duplicate, extra, unknown, or out-of-order
domain assessment; an `applicable` or `not_applicable` assessment without its
required evidence; and an unresolved assessment paired with a resolved
candidate. These use stable reasons `REVIEW_DOMAIN_ASSESSMENT_SET_INVALID`,
`REVIEW_DOMAIN_EVIDENCE_MISSING`, and `REVIEW_DOMAIN_STATE_MISMATCH`,
respectively. A fully assessed candidate with all eleven domains
`not_applicable` must be accepted as a complete empty membership for those
domains, while any domain assessment of `unresolved` must keep the snapshot
`BLOCKED`. This coverage is supplemental and does not change the fixed matrix.

### 11.1 Supplemental C-specific mutations

The C-001 through C-042 case IDs and stable intent are preserved from the
corrected V2 matrix, including the explicitly specified C-027 review-state
semantics. The V3 matrix contains exactly those 42 cases:

| Case | Mutation | Target artifact | Expected status | Expected reason code |
| --- | --- | --- | --- | --- |
| C-033 | Replace a normalized semantic enum with an uppercase/noncanonical variant | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `FAIL` | `NONCANONICAL_ENUM_VARIANT` |
| C-034 | Add an unknown participant/context/temporal vocabulary variant | `sources/m2_5/closures/C/declared_interaction_model.v2.json` | `FAIL` | `VOCABULARY_VARIANT_UNKNOWN` |
| C-035 | Remove the targeted review record named by a candidate | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `FAIL` | `TARGETED_REVIEW_RECORD_MISSING` |
| C-036 | Reference an unknown targeted review record | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `FAIL` | `TARGETED_REVIEW_RECORD_UNKNOWN` |
| C-037 | Tamper with the review-additions raw binding | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `FAIL` | `REVIEW_ADDITIONS_DIGEST_MISMATCH` |
| C-038 | Inject a `b2_derived` candidate into a valid C V3 snapshot | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `FAIL` | `B2_DERIVED_FORBIDDEN_V1` |
| C-039 | Tamper with a `CandidateIdentityV1` preimage or digest | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `FAIL` | `CANDIDATE_IDENTITY_MISMATCH` |
| C-040 | Tamper with a recorded C or master-drift checker identity | `sources/m2_5/closures/C/verification/c_verification_summary.v3.json` | `FAIL` | `CHECKER_IDENTITY_MISMATCH` |
| C-041 | Duplicate a canonical source-instance tuple within one candidate | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `FAIL` | `DUPLICATE_SOURCE_INSTANCE_TUPLE` |
| C-042 | Replace `archive_member_sha256` with a 63-character lowercase hexadecimal value | `sources/m2_5/closures/C/interaction_candidate_universe.v2.json` | `FAIL` | `SHA256_SCALAR_ENCODING_INVALID` |

Because the C V3 artifact contract retains the V1 source-origin rule that
forbids `b2_derived`, its required generated set is empty and
`new_b2_derived` must be zero. C-038 therefore proves that an extra B2-derived
candidate is rejected; no separate “missing required B2-derived candidate”
case exists in this version. If B2-derived generation is later proposed, the
spec amendment must define both the complete derived set and its missing/extra
mutations before enabling the source origin.

### 11.2 V3 publication-layout supplemental coverage

The fixed C-001 through C-042 matrix remains exactly 42 cases. The following
layout mutations are mandatory supplemental checker/unit coverage; they do not
receive new C case IDs and do not change the fixed matrix count:

| Mutation | Expected reason code |
| --- | --- |
| omit one of the 16 root `shards[]` entries | `CLASSIFICATION_SHARD_MISSING` |
| add an extra root shard entry or an unlisted shard file | `UNLISTED_C_ARTIFACT` |
| bind a shard inventory entry to the wrong raw SHA-256 | `CLASSIFICATION_SHARD_DIGEST_MISMATCH` |
| add, remove, duplicate, or cross-place a candidate classification across shards | `CLASSIFICATION_SHARD_COVERAGE_MISMATCH` |
| alter a shard index, exact path, first/last boundary, ordinal range, or record count | `CLASSIFICATION_SHARD_RANGE_MISMATCH` |
| alter root `classification_count` or `shard_count` | `CLASSIFICATION_ROOT_COUNT_MISMATCH` |
| serialize any shard above 50 MiB (52,428,800 bytes) | `CLASSIFICATION_SHARD_SIZE_LIMIT` |
| add the forbidden V2 monolith or an unlisted shard path | `UNLISTED_C_ARTIFACT` |
| add candidate records to the root or a root digest to a shard | `CLASSIFICATION_LAYOUT_INVALID` |
| reorder shard records or make the concatenated sequence non-contiguous | `CLASSIFICATION_LAYOUT_INVALID` |

The supplemental coverage MUST also prove that a root with an incomplete
`shards[]` inventory cannot pass, that a shard alone cannot constitute a
valid V3 classification bundle, that concatenating shards in index order
reconstructs the exact authoritative classification sequence, and that all
16 shard raw digests are recorded in the root and in the non-summary evidence
inventory. These are layout-integrity checks, not new semantic dispositions.

The following migration and publishability mutations are also mandatory
supplemental checker/unit coverage; they do not change the fixed C-001 through
C-042 matrix:

| Mutation | Expected reason code |
| --- | --- |
| change one of the four V2 upstream files relative to source H_exec | `V2_V3_UPSTREAM_PARITY_MISMATCH` |
| change, omit, duplicate, or add a V2/V3 candidate classification record | `V2_V3_CLASSIFICATION_PARITY_MISMATCH` |
| retain the forbidden V2 monolith in H_exec-reachable history | `LEGACY_MONOLITH_REACHABLE` |
| create a temporary history with a blob larger than 104,857,600 bytes, delete it in a later commit, and preflight the later commit | `PUBLISHABLE_HISTORY_OVERSIZE_BLOB` |
| enumerate unrelated local refs, including the archival ref, as the publishing delta | `PUBLISHABILITY_SCOPE_INVALID` |

The oversized-blob mutation MUST use a throwaway repository or temporary
history outside the reproducible Manafold source tree and MUST prove that
current-tree deletion does not erase the historical object rejection. The
archival ref itself is not a reason for failure because it is excluded from
the H_exec-relative-to-verified-base object enumeration.

The actual V3 evidence execution MUST also run the complete publishability
preflight once for H_exec and once for the already-created H_evidence. The
second run must not edit the summary or create another evidence commit; the
post-commit result is acceptance evidence outside the source snapshot.

## 12. Master-drift allowlist integration

The existing master-drift verifier MUST remain narrow. C V3 may extend its
exact allowlist only with these exact paths:

```text
scripts/check_m2_5_c_interactions.py
sources/m2_5/closures/C/C_DESIGN_SPEC.md
sources/m2_5/closures/C/declared_interaction_model.v2.json
sources/m2_5/closures/C/interaction_review_additions.v2.json
sources/m2_5/closures/C/interaction_candidate_universe.v2.json
sources/m2_5/closures/C/interaction_semantic_classes.v2.json
sources/m2_5/closures/C/interaction_classifications.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0000.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0001.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0002.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0003.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0004.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0005.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0006.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0007.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0008.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0009.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0010.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0011.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0012.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0013.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0014.v3.json
sources/m2_5/closures/C/classification_shards/interaction_classifications.0015.v3.json
sources/m2_5/closures/C/interaction_closure.v3.json
sources/m2_5/closures/C/INTERACTION_MODEL_REPORT.md
sources/m2_5/closures/C/verification/c_negative_test_matrix.v3.json
sources/m2_5/closures/C/verification/c_verification_summary.v3.json
```

This is an exact file set. It must not allow the broad `scripts/`
directory, a broad `sources/m2_5/` prefix, a C-directory prefix, or the
`classification_shards/` directory as a substitute for the listed paths.
The forbidden
`sources/m2_5/closures/C/interaction_classifications.v2.json` path,
`sources/m2_5/closures/C/classification_shards/interaction_classifications.0016.v3.json`,
a shard backup suffix, and every other unlisted C path must be rejected. Near-miss
paths such as
`scripts/check_m2_5_c_interactions.py.backup`,
`sources/m2_5/closures/C/C_DESIGN_SPEC.md.backup`, and
`sources/m2_5/closures/C/classification_shards/interaction_classifications.0000.v3.json.backup`
are not allowed.

The existing master-drift negative self-test MUST retain an exact near-miss
checker-path case and add exact near-miss coverage for a shard path. This
master-drift integration self-test is outside the fixed 42 semantic C
mutations. C source is additive and must not modify historical B1/B2/REV3
artifacts.

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

### Blocked snapshot handling

A source snapshot may persist a structurally valid incomplete semantic review.
Its classifications retain every candidate and all available identity,
instance, reconciliation, and evidence bindings, while unresolved candidates
carry `terminal_disposition = null` and a closed `unresolved_reason`. Its
closure and summary MUST report
`DECLARED_INTERACTION_MODEL_CLOSURE = BLOCKED` and the recomputed non-zero
`review_state_metrics.unresolved` count. This is useful reviewable attempt
evidence, but it is not a C acceptance result and it does not authorize
downstream work.

An evidence lineage that records such a blocked attempt MUST preserve that
actual blocked status. It MUST NOT be reinterpreted as a successful C closure
after the fact. If the source or checker is corrected, the corrected snapshot
requires a new H_exec and a new evidence lineage under the ordinary rules
below; historical attempt commits remain immutable evidence of the earlier
state.

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

Before H_exec is accepted as the V3 source snapshot, creation mode MUST
execute and pass the migration-parity gate and the publishability preflight
against that exact H_exec. Their outputs are recorded in the final
summary/evidence record; a provisional summary with execution fields marked
`NOT_RUN` MUST not claim those results were already recorded as final
acceptance evidence. A failure requires a new source snapshot or leaves the
V3 publication result `BLOCKED`.

The creation invariant is:

```text
H_evidence^ == H_exec
diff(H_exec, H_evidence)
  == {sources/m2_5/closures/C/verification/c_verification_summary.v3.json}
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
sources/m2_5/closures/C/verification/c_verification_summary.v3.json
```

That change records H_exec as `execution_commit`, the exact source-tree
fingerprints, all command results, checker identities, and raw digests for all
25 non-summary C inventory files. The digest groups are:

```text
C_DESIGN_SPEC.md
model
review additions
candidate universe
semantic classes
classification root
classification shard 0000 through classification shard 0015 (each exact path)
closure
report
negative matrix
```

Each shard has its own raw SHA-256 entry keyed by its exact path. The root
manifest repeats those shard digests and is the classification-level closure
binding. `C_DESIGN_SPEC.md` is retained for inventory/evidence integrity but
is not a closure input. The summary itself is outside both the closure and
this digest list.

The summary records the creation-only V2→V3 `migration_parity` evidence and
the H_exec-only Git-history `publishability_preflight` evidence separately
from the non-summary artifact digests. The archived V2 monolith's digest may
appear only inside `migration_parity`; the monolith is never a V3 inventory or
closure input. Neither evidence object is a closure input or a self-digest,
and the summary does not record an H_evidence commit digest.

The Phase C commit MUST be a direct child of H_exec. Its diff MUST contain
exactly the verification summary. Any other diff is a failed evidence cycle,
not a minor warning.

Immediately after the Phase C commit creates H_evidence, the post-commit
publishability preflight from §7.10 MUST be executed against that exact
H_evidence and the recorded `verified_base_commit`. This execution is an
acceptance check outside the summary; its result MUST NOT cause a second
summary edit or a third evidence commit. A failure means that this evidence
lineage is not publishable and requires a new H_exec.

Raw logs, the independent semantic-review export, and other generated
verification output remain outside the reproducible source archive.

The pre-correction C attempt lineage `H_exec =
0c2b71cfc2343917047ac5cd5b030ea05a376c57` and `H_evidence =
2e7c8a2e5e7910dd5d03963084cd08c246e852cf` is superseded blocked attempt
evidence. It is not evidence that the corrected review-state contract passed,
and its commits or summary must not be rewritten to retrofit that meaning.

The V2 blocked, delivery-blocked monolithic-classification attempt lineage
`H_exec = 43bf3ccc6ff639c900914947ee0883b4731b8409` and
`H_evidence = 0886a1777d47f07b93a2c29f44ce1b9acbaa231b` remains reachable
only as historical attempt evidence. It is superseded by this V3
classification-layout contract, must not be rewritten, and must not be used as
the base of a publishable V3 branch. Its monolithic classification bytes are
not part of the V3 inventory or V3 evidence.

### 13.2 Historical descendant validation

The verifier has a separate historical-descendant mode for a later branch or
master. In this mode, the current `HEAD` may be any descendant of H_evidence;
it is not required to equal H_evidence, and its parent is not required to be
H_exec.

The verifier MUST:

1. read and validate the current summary bytes at
   `HEAD:sources/m2_5/closures/C/verification/c_verification_summary.v3.json`;
   this current summary is the selector for the evidence lineage, not a
   candidate discovered by recency;
2. obtain the recorded non-null `execution_commit = H_exec` from those current
   summary bytes;
3. search the current `HEAD` ancestry for evidence commits `E` such that the
   bytes at `E:sources/m2_5/closures/C/verification/c_verification_summary.v3.json`
   equal the current summary bytes, `E` has exactly one parent, and
   `parent(E) = H_exec`;
4. require exactly one such evidence commit and call it `H_evidence`; prove
   `H_evidence` is an ancestor of the current `HEAD`;
5. recompute `git diff --name-status H_exec..H_evidence` and require exactly
   `sources/m2_5/closures/C/verification/c_verification_summary.v3.json`,
   including its mode and content, with no other path or mode change;
6. read the recorded artifact digests from that historical summary and compare
   them with the current exact bytes of every non-summary C inventory file,
   including the non-semantic design spec, report, and negative matrix;
7. require the current summary bytes to equal the bytes returned by
   `git show H_evidence:<summary path>`; this protects the summary without
   inventing a self-digest;
8. recompute the five semantic C input bindings and both V1 semantic identity
   preimages from the current artifacts; and
9. require the current C semantic closure and all recorded prerequisite
   identities to remain equal to the selected historical evidence snapshot;
10. execute the post-commit Git-history publishability preflight for the
    selected H_evidence against the recorded `verified_base_commit`, including
    the exact legacy-path history check, and require it to pass without
    changing the current summary.

The exact summary-byte and recorded-`H_exec` match deliberately excludes a
superseded evidence lineage whose summary or execution commit differs, even
when that older lineage remains reachable. If more than one reachable commit
satisfies the exact relation, if none does, if the direct-parent proof fails, or
if any recorded current artifact digest differs, the result is `FAIL` or
`BLOCKED` according to whether the contradiction is in source data or
unavailable history. The verifier may not silently select the newest or any
other candidate by recency.

Historical-mode regression coverage MUST include a reachable superseded
H_exec/H_evidence pair and a current pair with different summary bytes. It
must prove that the current `HEAD` summary selects only the evidence commit
whose summary bytes and recorded `H_exec` exactly match it, and that changing
the current summary to the superseded bytes, creating two exact matching
evidence commits, or removing the matching parent proof is rejected. This is
lineage-selection coverage, not permission to discard preserved historical
attempt evidence.

This historical mode is the post-merge contract. It must not require
`HEAD == H_evidence` or `HEAD^ == H_exec`; only the ancestry and historical
summary-only proofs above are normative.

Historical descendant validation does not re-run the one-time V2→V3 migration
against the private archive and does not enumerate the local archival ref.
It validates the selected current summary's recorded migration-parity and
H_exec publishability objects, their exact summary bytes, the recorded
H_exec/H_evidence lineage, and the current V3 inventory bindings. The
creation-time publishability result is tied to its recorded verified base
commit and `checked_commit = H_exec`; the post-commit H_evidence preflight is
executed afresh against that same immutable base, not silently recomputed
against a moving or unrelated local ref and not written into the summary.

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
- the V3 classification root manifest and every one of its 16 shard files;
- every candidate classification, exactly one `review_state`, any terminal
  disposition or `unresolved_reason`, and its concrete source-instance/context
  mapping and eleven `review_domain_assessments`;
- the high-risk review-set memberships derived from those assessments;
- B2 family/boundary and B1.Final citation bindings;
- the recomputed review-state metrics and gate status, including any blocked
  review domains;
- the migration-parity source commit, raw bindings, record-equality result,
  and exact ordering transformation;
- the persisted H_exec publishability-preflight verified base, checked commit,
  introduced object counts, and oversized-blob/legacy-path violation inventory;
- the separate post-commit H_evidence publishability result and the exact
  evidence/topology conditions under which it was executed;
- the report path and digest; and
- explicit missing/blocked evidence.

The export is for review and is not a new authority. It must not be committed
to the repository or fed back into the closure digest.

## 15. Acceptance criteria

C is accepted only when all of the following are true:

1. The live exact-head and archive preflight are verified.
2. All B1, B1.Final, B2, and master-drift prerequisites are `PASS`.
3. The REV3 candidate universe is complete and every lineage is reconciled.
4. The model scope is finite and matches `pairwise_plus_review_outliers`.
5. Semantic classes are separately authoritative and deduplicated.
6. Candidate classifications contain no copied class definitions.
7. Every candidate has exactly one classification and exactly one
   `review_state`; a `resolved` candidate has exactly one terminal disposition,
   while an `unresolved` candidate has null terminal/class IDs and one closed
   `unresolved_reason`.
8. `unresolved = 0`.
9. Every required class has exact B2 boundary and B1.Final citation bindings.
10. All required context, direction, role, host, timing, ordering, and arity
    information is explicit.
11. The closure binds only its five semantic C inputs and is acyclic with
    respect to the design spec, report, negative matrix, and verification
    evidence.
12. All 42 fixed V3 negative tests pass with their exact reason codes,
     including all fixed C-001 through C-042 cases; the supplemental V3
     layout, migration-parity, and publishability coverage also passes.
13. H_exec and H_evidence satisfy the direct-child V3 summary-only rule.
14. Required repository and language gates execute successfully.
15. The exact M2.5 gate/flag values in §7.6 remain preserved; no later gate is
    promoted.
16. The independent review export is available for human review.
17. The implementation branch and PR, if later requested, remain unmerged
    until the semantic C spec and resulting evidence have been independently
    reviewed.
18. The one-time V2→V3 migration-parity gate is `PASS` for the exact source
    H_exec and proves byte-identical four-file upstream inputs plus
    record-for-record classification parity after only the specified ordering
    and sharding transformation.
19. The persisted Git-history publishability preflight is `PASS` for H_exec,
     and the separate post-commit preflight is `PASS` for H_evidence relative
     to the same verified base, with no reachable blob above 104857600 bytes
     and no reachable V2 monolith path.

If any criterion is unknown, unexecuted, contradictory, or unavailable, the
result is `BLOCKED` or `NOT_RUN` as appropriate. It is not a C PASS.

## 16. Spec-review boundary

This commit defines the V3 classification publication layout, artifact
contracts, authority graph, review protocol, migration-parity and
publishability evidence contracts, negative cases, verifier obligations, and
evidence protocol only. It does not create the C JSON artifacts, checker,
review classifications, or implementation logic. Those changes require a
separate explicit implementation authorization after independent review of
this specification.
