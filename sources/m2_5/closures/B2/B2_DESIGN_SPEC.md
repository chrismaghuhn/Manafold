# M2.5.B2 Terminal Card Classification Closure

**Status:** proposed implementation specification

**Base:** master at f59c462874a29e0f74194b203c6fa8cca69ef3c0

**Pinned input:** Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip

**Pinned input SHA-256:** 99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90

## 1. Decision and boundaries

M2.5.B2 creates an additive, source-grounded terminal classification snapshot under
sources/m2_5/closures/B2/. It never rewrites REV3 and adds no runtime Magic
semantics.

The authority direction is:

    pinned REV3 card evidence
            ↓
    reviewed card semantics
            ↓
    terminal requirement assignments
            ↓
    family-catalog consistency checks
            ↓
    future interaction and ranking work

The family catalog cannot authorize a card assignment. A family is assigned only when
pinned card evidence supports the assignment and the review records that evidence.

B2 owns CLASSIFICATION_REFERENCE_CLOSURE only. It leaves these statuses outside its
ownership:

    OFFICIAL_RULE_CITATION_CLOSURE       = PENDING_B1_FINAL
    DECLARED_INTERACTION_MODEL_CLOSURE   = BLOCKED
    REV2_REUSE_RATIO_REPRODUCIBLE        = BLOCKED
    RANKING_UNCERTAINTY_PROPAGATION      = BLOCKED
    DECK_PAIR_LOCKED                     = NO
    AUTHORITATIVE_RANKING_AVAILABLE      = NO
    M3_STARTED                           = NO

B2 does not select a deck pair, regenerate interaction or ranking outputs, modify
PR #79, implement cards or rules, or alter Rust, Python, wire, schema, replay,
checkpoint, information, decision, or RNG semantics.

## 2. Versioned B2 surface

The implementation creates these additive artifacts:

    sources/m2_5/closures/B2/
      B2_DESIGN_SPEC.md
      card_semantic_classifications.v1.json
      deck_row_classification_refs.v1.csv
      requirement_family_catalog.v1.json
      classification_closure.v1.json
      CLASSIFICATION_REPORT.md
      verification/
        b2_negative_test_matrix.v1.json
        b2_verification_summary.v1.json

Stable schema identifiers are:

    manafold.m2.5.b2.card-semantic-classifications.v1
    manafold.m2.5.b2.deck-row-classification-refs.v1
    manafold.m2.5.b2.requirement-family-catalog.v1
    manafold.m2.5.b2.classification-closure.v1
    manafold.m2.5.b2.negative-test-matrix.v1
    manafold.m2.5.b2.verification-summary.v1

The design spec is inside the B2 surface so the master-drift extension remains
limited to the additive B2 surface and its checker.

## 3. Pinned source inputs

The verifier resolves the archive only through:

    MANAFOLD_SOURCE_ARCHIVE/m2_5/
      Manafold_M2_5_Pre_Research_ALL_ARTIFACTS_REV3.zip

It requires the exact package SHA-256 above before reading excluded payload. It reads
and manifest-verifies these archive members:

    inputs/deck_row_source_resolution_REV3.csv
    inputs/deck_row_classification_refs_REV3.csv
    inputs/oracle_semantic_evidence_REV3.json
    inputs/card_semantic_classification_REV3.json
    inputs/requirement_family_catalog_REV3.json
    source/raw/oracle_cards_selected_REV3.jsonl
    derived/Card_Requirement_Map_REV3.csv
    derived/Pair_Requirement_Aggregates_REV3.json
    Manafold_M2_5_Package_Manifest_REV3.json

The source package is a point-in-time input. Repository REV3 files remain historical
evidence and are never replaced with terminal values.

For each selected raw Oracle record, the verifier hashes the exact JSONL line bytes,
including its trailing newline. That digest must match the pinned REV3 evidence and
the B2 source binding. The normalized REV3 digest is retained as a second identity
check; B2 does not silently change its normalization policy.

## 4. Fixed universe and shared identity

    OracleSemanticIdentity values = 402
    DeckRowIdentity rows           = 441
    Reused OracleSemanticIdentity  = 23
    Total deck quantity             = 600
    Historical family IDs           = 216

OracleSemanticIdentity is the shared semantic key. Deck row, deck name, quantity,
printing, and commander context are not semantic authority keys.

The verifier proves:

1. Every input OSI has exactly one B2 classification.
2. Every B2 classification binds to exactly one known pinned source record.
3. Every deck row binds to exactly one known OSI.
4. Every repeated OSI resolves to one byte-identical classification identity.
5. A deck row cannot add or replace a shared semantic assignment.

classification_identity is the SHA-256 of canonical UTF-8 JSON for the complete
classification record. Canonical JSON uses sorted object keys, compact separators,
and recursively sorted ID arrays. It is never deck-specific.

## 5. Requirement-family catalog

### 5.1 Historical preservation

The B2 catalog contains all 216 historical REV3 family IDs exactly once, plus any
genuinely new generic B2 family IDs. Historical IDs are never renamed, reused for a
different concept, or deleted.

Every historical entry carries an immutable historical_rev3 block containing the exact
REV3 family object, its canonical digest, its historical member OSIs and card names,
and the original assignment-record digests for those members.

Each historical entry also has a historical_definition object. It contains the exact
REV3 description, classification criteria, family name, and assignment context that
REV3 actually used for the ID, together with a digest of that object. The field is a
historical record, not a B2 rewrite. Because REV3 descriptions are provisional, the
member assignments and source-backed usage context are also retained. Preservation
does not promote them to terminal truth.

### 5.2 Lifecycle

Every catalog entry has exactly one status:

    ACTIVE
      The reviewed definition is semantically equivalent to the historical concept and
      may receive terminal assignments.

    ACTIVE_UNASSIGNED
      The reviewed definition remains valid, but no B2 classification assigns it.

    SUPERSEDED
      The historical concept is preserved but is not the correct terminal authority.
      One or more replacement families are named.

    RETIRED
      The historical family is preserved and intentionally has no terminal successor.

terminal_assignable is explicit and must agree with status:

    ACTIVE              -> true
    ACTIVE_UNASSIGNED   -> false
    SUPERSEDED          -> false
    RETIRED             -> false

Terminal assignments may reference ACTIVE entries only. ACTIVE_UNASSIGNED is valid
vocabulary but cannot be assigned in this snapshot.

### 5.3 Catalog record

Each record contains:

    family_id
    canonical_name
    historical_rev3 { record, record_sha256, member_osi, assignment_record_digests }
    historical_definition { rev3_description, rev3_criteria, assignment_context, definition_sha256 }
    precise_semantic_definition
    definition_relation
    evidence_basis_allowed[]
    status
    terminal_assignable
    superseded_by[]
    supersession_reason (required for SUPERSEDED)
    review_provenance { review_status, review_basis, evidence_locators[] }

precise_semantic_definition documents the reviewed concept. For ACTIVE and
ACTIVE_UNASSIGNED, definition_relation must be
SEMANTICALLY_EQUIVALENT_CLARIFICATION. For SUPERSEDED and RETIRED, it records the
historical concept without making it assignable.

The allowed definition_relation values are:

    SEMANTICALLY_EQUIVALENT_CLARIFICATION
    MATERIALLY_CHANGED_SUPERSESSION
    HISTORICAL_PRESERVATION_NO_SUCCESSOR

The catalog top level is:

    schema = manafold.m2.5.b2.requirement-family-catalog.v1
    source_package_sha256
    rev3_catalog_sha256
    legacy_family_count = 216
    new_family_count
    terminal_family_count = legacy_family_count + new_family_count
    families[]

The catalog verifier requires exactly one record for each historical ID, unique new
IDs, canonical family ordering by family_id, and a complete historical block on
every legacy record.

### 5.4 Semantic equivalence

For an ACTIVE or ACTIVE_UNASSIGNED historical family, semantic equivalence means
that the reviewed definition preserves all of these dimensions:

    objects and object classes
    action or event
    timing and phase/step boundary
    zone and visibility boundary
    eligibility, condition, and duration
    targets and choices
    ownership and control
    numeric scaling and counters
    information or identity effect
    required rule-derived dependency

The wording may clarify an implicit boundary or replace a vague label with precise
prose. It may not change a dimension, materially broaden or narrow the family, or turn
one action into another.

These are material changes and require SUPERSEDED, not an active clarification:

    untap semantics -> attachment semantics
    one object -> a different object class
    optional action -> mandatory action
    public information -> hidden information
    one timing window -> another timing window
    one target/choice contract -> a different target/choice contract

Equivalence is a reviewed, evidence-backed relation; it is not claimed to be proven
by string comparison. The verifier enforces the declaration, historical digest,
status, evidence, and regression anchors. The report records the review basis.

## 6. Lifecycle transitions

B2 materializes one terminal snapshot and never mutates a previous B2 version. The
allowed v1 transitions are:

    REV3 INHERITED_REV2_CANDIDATE -> ACTIVE
    REV3 INHERITED_REV2_CANDIDATE -> ACTIVE_UNASSIGNED
    REV3 INHERITED_REV2_CANDIDATE -> SUPERSEDED
    REV3 INHERITED_REV2_CANDIDATE -> RETIRED

    NEW_B2_FAMILY_PROPOSAL -> ACTIVE
    NEW_B2_FAMILY_PROPOSAL -> ACTIVE_UNASSIGNED

An unresolved review has no terminal status and blocks closure. Later lifecycle
changes require a new additive catalog/classification version.

Status invariants are:

    ACTIVE              may have terminal assignments
    ACTIVE_UNASSIGNED   must have zero terminal assignments
    SUPERSEDED          must have zero terminal assignments and nonempty superseded_by
    RETIRED             must have zero terminal assignments and empty superseded_by

Every superseded_by value is a list. Each target must be a different catalog ID, must
exist exactly once, and must be ACTIVE or ACTIVE_UNASSIGNED. Self-targets, cycles,
unknown targets, and nonterminal targets are rejected.

## 7. New families, splits, and merges

New families are generic requirement concepts, not card-specific labels. New IDs use
the req.b2. namespace, lowercase snake-case slugs, and stable semantic names. The
legacy cap.* namespace is preserved only because those IDs are historical REV3
identities; neither namespace represents an implemented Manafold capability.

A reviewer may request a family but cannot finalize its ID or definition. Central
integration deduplicates requests and records the evidence for the canonical result.

For a split, the old family is SUPERSEDED, its superseded_by list contains all
replacement IDs, and each replacement assignment has independent evidence. For a
merge, every preserved old family is SUPERSEDED and points to the same target.
Neither operation deletes an old ID or rewrites its historical definition.

A valid family with zero B2 usage becomes ACTIVE_UNASSIGNED, never RETIRED solely
because its usage count is zero.

## 8. Terminal classifications

There is exactly one classification record per OSI. Each record contains:

    oracle_semantic_identity
source_identity { oracle_source_record_id, layout, raw_sha256, normalized_sha256 }
source_evidence_digest
    review_status = REVIEWED_CONFIRMED | REVIEWED_CORRECTED
    previous_rev3_classification_identity
    requirement_assignments[]
    classification_delta
    review_basis
    provenance

The classification artifact top level is:

    schema = manafold.m2.5.b2.card-semantic-classifications.v1
    source_package_sha256
    input_oracle_identity_count = 402
    classifications[]

classifications[] is sorted by oracle_semantic_identity and contains no duplicate
identity. The artifact does not contain deck-row-specific classification records.

Each assignment contains:

    requirement_family_id
    evidence_basis
    evidence_locators[]
    review_rationale

Assignments are sorted, unique, and resolve to ACTIVE catalog entries. Every
assignment has at least one valid locator and a nonempty rationale.

source_evidence_digest is the SHA-256 of canonical UTF-8 JSON containing the complete
source_identity object. classification_identity includes the complete classification
record and therefore changes whenever its source binding, assignments, deltas, or
provenance changes.

previous_rev3_classification_identity is recomputed from the exact provisional REV3
record, not trusted from a copied field. Working statuses such as
AMBIGUOUS_REQUIRES_REVIEW may exist in private batches but never in a terminal
artifact.

## 9. Delta model

The four arrays are sorted, duplicate-free, and disjoint:

    retained_family_ids
      Existing REV3 assignments that remain valid and terminally assigned.

    added_family_ids
      Terminal assignments absent from REV3, including new-family assignments.

    removed_family_ids
      REV3 assignments removed without a terminal replacement.

    superseded_family_ids
      REV3 assignments removed because their historical family is SUPERSEDED.

The final assignment set must equal:

    (REV3 requirement IDs - removed - superseded) + added

Every changed record has a per-change rationale and evidence. REVIEWED_CONFIRMED
requires an empty delta; REVIEWED_CORRECTED requires a nonempty correction delta.

## 10. Evidence locators

Evidence is typed and bound to exact pinned bytes. A v1 locator contains:

    locator_version = manafold.m2.5.b2.evidence-locator.v1
    archive_artifact
    record_id
    field
    face
    line_sha256
    field_value_sha256

Allowed evidence bases are ORACLE_TEXT, TYPE_LINE, CARD_FACE,
STRUCTURAL_CARD_PROPERTY, FORMAT_POLICY, and RULE_DERIVED.

For raw Oracle records, field identifies an exact raw field such as oracle_text,
type_line, layout, keywords, mana_cost, power, toughness, or colors. CARD_FACE is
valid only when the pinned source contains that face. Policy and rule locators
identify the exact pinned authority artifact and byte/offset location.

The verifier checks the record ID, full-line digest, field-value digest, and archive
member digest. A valid-looking locator for another record or archive revision fails.

## 11. 402-to-441 projection

deck_row_classification_refs.v1.csv is a projection, not an independent semantic
source. Its columns are:

    deck_row_id
    deck_id
    oracle_semantic_identity
    terminal_classification_identity
    classification_status
    terminal_requirement_ids

terminal_requirement_ids is a canonical compact JSON array. The projection is:

    for each pinned REV3 deck row:
      classification = B2 classification indexed by row.oracle_semantic_identity
      identity       = classification.classification_identity
      status         = classification.review_status
      requirement_ids = sorted(classification.assignment IDs)

The verifier regenerates this CSV from the 402 classifications and the pinned 441
rows. It rejects missing/unknown rows, unknown OSIs, changed arrays, and all
deck-specific semantic forks. The CSV contains no family definition, rationale,
evidence, or semantic override.

## 12. Closure artifact and criteria

classification_closure.v1.json records the pinned package digest, measured input
counts, family lifecycle counts, classification counts, correction metrics, bound
artifact digests, and the exact downstream gate statuses from Section 1.

Its required top-level fields are:

    schema
    source_package_sha256
    input_universe
    family_counts
    classification_counts
    correction_metrics
    bound_artifacts[]
    gate_status
    status

The verifier recomputes every count and every bound artifact digest. It rejects a
closure that contains a manually promoted gate status or a count that is not equal
to the validated artifact contents.

CLASSIFICATION_REFERENCE_CLOSURE = PASS requires:

    402 input OSIs and 402 terminal classifications
    441 input rows and 441 valid projections
    216 historical family IDs preserved exactly once
    zero unknown, missing, duplicate, or forked shared identities
    zero source digest or locator failures
    zero unknown or non-ACTIVE terminal family assignments
    zero silent classification changes
    every correction has explicit delta, rationale, and evidence
    historical family digests and definitions remain inspectable
    all schema IDs and canonical order checks pass

The closure also reports:

    REV3_FAMILY_COUNT
    REV3_FAMILIES_PRESERVED
    TERMINAL_FAMILY_COUNT = 216 + new family count
    ACTIVE_ASSIGNED_COUNT
    terminal_assignment_count
    ACTIVE_UNASSIGNED_COUNT
    SUPERSEDED_COUNT
    RETIRED_COUNT
    confirmed_authorities
    corrected_authorities
    families_added
    families_removed
    legacy_families_with_zero_terminal_usage
    new_terminal_families

ACTIVE_ASSIGNED_COUNT is the number of catalog family IDs with at least one terminal
assignment. terminal_assignment_count is the total number of assignment edges across
the 402 classifications. ACTIVE_UNASSIGNED_COUNT counts valid catalog IDs with zero
terminal assignments, regardless of whether they are historical or newly introduced.

## 13. Semantic review procedure

The 402 OSIs are reviewed from the exact pinned archive, never from live Oracle data
or the unmerged B1 mapping. Review batches sort OSIs by canonical UUID and use fixed
contiguous ranges. Each batch includes source identity, raw binding, type line,
structural properties, Oracle text, and provisional IDs.

The central integration step owns vocabulary, deduplicates new-family requests,
checks split/merge proposals, computes deltas, and rejects unresolved records. A
substring hit or family-name match is not semantic evidence.

The report explicitly audits:

    cap.conditional_hexproof
    cap.life_drain
    cap.delayed_sacrifice
    cap.countered_setup
    cap.copy_token_batch
    cap.crew_alternative
    cap.mass_untap
    cap.tribal_permission
    cap.state_based_actions

Each regression entry gives source card(s), historical family, terminal disposition,
exact locator(s), and reason. These are audit anchors, not hard-coded conclusions.

Any unresolved OSI blocks closure and produces:

    CLASSIFICATION_REFERENCE_CLOSURE = BLOCKED
    M2_5_B2_BLOCKED_BY_CLASSIFICATION

## 14. Verifier and negative-test contract

The checker is scripts/check_m2_5_b2_classifications.py:

    python scripts/check_m2_5_b2_classifications.py
    python scripts/check_m2_5_b2_classifications.py --negative-self-test

Positive mode validates the archive, schemas, cross-bindings, evidence, projection,
lifecycle, metrics, and gate statuses. It fails closed when the archive variable is
unset, the ZIP is absent, or a required member is missing or mismatched.

Negative mode first proves the unmutated positive fixture passes. It then mutates an
isolated copy and asserts the exact error code. Mutations are re-bound unless the
mutation itself tests digest binding.

Required error codes are:

    MISSING_CLASSIFICATION_REJECTED
    DUPLICATE_ORACLE_IDENTITY_REJECTED
    UNKNOWN_ORACLE_IDENTITY_REJECTED
    NONTERMINAL_CLASSIFICATION_REJECTED
    MISSING_DECK_ROW_REFERENCE_REJECTED
    UNKNOWN_DECK_ROW_REFERENCE_REJECTED
    REUSED_ORACLE_IDENTITY_FORK_REJECTED
    SOURCE_DIGEST_MISMATCH_REJECTED
    SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED
    UNKNOWN_REQUIREMENT_FAMILY_REJECTED
    SUPERSEDED_FAMILY_ASSIGNED_REJECTED
    SILENT_CLASSIFICATION_CHANGE_REJECTED
    CORRECTION_WITHOUT_RATIONALE_REJECTED
    CORRECTION_WITHOUT_EVIDENCE_REJECTED
    LEGACY_FAMILY_REINTERPRETATION_REJECTED
    WRONG_CLASSIFICATION_SCHEMA_REJECTED
    WRONG_CLOSURE_SCHEMA_REJECTED
    EVIDENCE_DIGEST_TAMPER_REJECTED
    OTHER_GATE_PROMOTION_REJECTED
    DECK_LOCK_PROMOTION_REJECTED
    M3_PROMOTION_REJECTED

The legacy reinterpretation test changes a historical definition's meaning without
superseding it and must reject the mutation. Other gate-promotion tests prove that
B2 cannot promote interaction, ranking, deck-lock, or M3 status.

The negative-test matrix is:

| Error code | Mutation and expected rejection |
| --- | --- |
| MISSING_CLASSIFICATION_REJECTED | Remove one OSI classification. |
| DUPLICATE_ORACLE_IDENTITY_REJECTED | Duplicate one classification identity. |
| UNKNOWN_ORACLE_IDENTITY_REJECTED | Replace an OSI with an ID absent from pinned evidence. |
| NONTERMINAL_CLASSIFICATION_REJECTED | Set one terminal record to a working review status. |
| MISSING_DECK_ROW_REFERENCE_REJECTED | Remove one of the 441 projected rows. |
| UNKNOWN_DECK_ROW_REFERENCE_REJECTED | Add a row ID absent from pinned deck resolution. |
| REUSED_ORACLE_IDENTITY_FORK_REJECTED | Change the assignment set for one repeated OSI row. |
| SOURCE_DIGEST_MISMATCH_REJECTED | Change a source binding without changing the pinned source. |
| SOURCE_EVIDENCE_LOCATOR_INVALID_REJECTED | Point an assignment locator at a wrong field or record. |
| UNKNOWN_REQUIREMENT_FAMILY_REJECTED | Add an ID absent from the terminal catalog. |
| SUPERSEDED_FAMILY_ASSIGNED_REJECTED | Assign a catalog family with status SUPERSEDED. |
| SILENT_CLASSIFICATION_CHANGE_REJECTED | Change an assignment while leaving all delta arrays empty. |
| CORRECTION_WITHOUT_RATIONALE_REJECTED | Remove the rationale for an added, removed, or superseded ID. |
| CORRECTION_WITHOUT_EVIDENCE_REJECTED | Remove the locator for a changed assignment. |
| LEGACY_FAMILY_REINTERPRETATION_REJECTED | Change a historical family meaning while keeping it ACTIVE. |
| WRONG_CLASSIFICATION_SCHEMA_REJECTED | Replace the classification schema identifier with another version. |
| WRONG_CLOSURE_SCHEMA_REJECTED | Replace the closure schema identifier with another version. |
| EVIDENCE_DIGEST_TAMPER_REJECTED | Change a raw or normalized source digest. |
| OTHER_GATE_PROMOTION_REJECTED | Set interaction or citation status to PASS in B2. |
| DECK_LOCK_PROMOTION_REJECTED | Set DECK_PAIR_LOCKED to YES. |
| M3_PROMOTION_REJECTED | Set M3_STARTED to YES. |

The positive fixture passes before these mutations run. Each mutation is isolated,
and the checker asserts the exact code in the first failing validation layer.

## 15. Master-drift boundary

The only allowlist extension is:

    sources/m2_5/closures/B2/
    scripts/check_m2_5_b2_classifications.py

The existing checker must continue rejecting changes under crates/, python/,
schemas/, wire/, docs/contracts/, docs/adr/, and cards/. If B2 requires a change
outside this boundary, stop with M2_5_B2_BLOCKED_BY_MASTER_DRIFT.

## 16. Verification sequence and done condition

The implementation order is:

1. Run master-drift and archive preconditions.
2. Write and run failing B2 checker/negative tests.
3. Implement the smallest checker surface that makes those tests pass.
4. Build deterministic review batches from the verified ZIP.
5. Complete the 402 source-grounded reviews and central vocabulary integration.
6. Emit terminal artifacts and stable verification records.
7. Run positive and negative B2 checks.
8. Run repository integration, Rust format, and Rust check commands.
9. Re-run master drift and archive preflight against the final exact head.
10. Confirm the tracked tree is clean before and after verification.

Required commands are:

    python scripts/check_m2_5_master_drift.py
    python scripts/check_m2_5_master_drift.py --verify-archive
    python scripts/check_m2_5_b2_classifications.py
    python scripts/check_m2_5_b2_classifications.py --negative-self-test
    python scripts/run_checks.py integration
    cargo +1.85.1 fmt --all -- --check
    cargo +1.85.1 check --workspace --all-targets --all-features --locked

B2 is ready for external review only when all closure criteria pass, the required
commands actually pass, MASTER_DRIFT = PASS, and all downstream statuses remain
blocked or pending as specified. The final status is exactly one of:

    M2_5_B2_READY_FOR_EXTERNAL_REVIEW
    M2_5_B2_BLOCKED_BY_CLASSIFICATION
    M2_5_B2_BLOCKED_BY_MASTER_DRIFT

Classification closure is not card implementation, support, certification, ranking,
deck selection, or permission to begin M3.
