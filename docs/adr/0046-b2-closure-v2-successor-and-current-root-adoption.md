# ADR 0046: B2 Closure-v2 Successor and Current-Root Adoption

- **Status:** accepted
- **Date:** 2026-09-11
- **Supersedes:** none; preserves B2 v1 historical evidence and adds its closure successor
- **Superseded by:** none
- **Depends on:** ADR 0041, ADR 0042, ADR 0043, ADR 0044, ADR 0045, and the immutable B2 v1 closure artifacts
- **Reviewed baseline:** `e81d56d5eaa44bce38c3171c51f27d29f7ec0d9f`
- **Candidate SHA-256 before promotion:** `C71E778665991A05924BFB2EA9B78B3284E92E59A0D56D4FFC5B455AE7A5DCC2`
- **Review provenance:** `APPROVE_CONSOLIDATED_ADR_CANDIDATE`; `0 BLOCKER / 0 MAJOR / 0 MINOR / 0 NIT`
- **Implementation evidence:** `NOT_RUN`

This ADR accepts architecture only. It does not authorize closure-v2
implementation, executable source-role changes, current-root adoption,
production authority, C changes, Domain Audit 55/55, ranking, deck locking,
or M3.

**Milestone:** M2.5
**Scope:** B2 closure-contract contradiction only

> Reissued reconstruction. The original temporary Remediation-02 artifact was lost. This candidate reconstructs the completed remediation from the independent review findings, the remediation task, and the recorded final status. It is not claimed to be byte-identical to the lost temporary artifact.

## Context

M2.5.B2 produced a source-grounded terminal semantic snapshot containing:

```text
402 effective card classifications
1883 terminal assignment edges
441 deck-row projections
216 historical family identities
210 ACTIVE families
6 ACTIVE_UNASSIGNED families
```

The semantic snapshot itself is not defective.

In particular, effective B2 v1 already contains terminal `REVIEWED_CORRECTED` records for:

```text
Sporemound
  cap.trigger
  cap.landfall
  cap.token_creation

Rampant Rejuvenator
  cap.death_trigger
  cap.counters
  cap.search_library
  cap.put_battlefield
  cap.mass_put_battlefield
  cap.shuffle
  cap.trigger
```

The previously observed `288 CURRENT_ORACLE_BLOCKED` records belong to historical REV3 input provenance. They are not 288 missing current B2 classifications.

However, B2 v1 contains a real contract contradiction:

- the closure section of `B2_DESIGN_SPEC.md` describes one top-level closure shape;
- `classification_closure.v1.json` persists another shape;
- the executable B2 checker validates the persisted shape;
- the design spec itself is bound by the v1 closure, so changing the prose in place would change historical closure evidence;
- B2 v1 states that previous B2 versions are not mutated.

Under Manafold's normative conflict policy, this contradiction blocks freeze/certification of the affected surface until resolved additively.

## Decision

Manafold will preserve the complete B2 v1 semantic snapshot and historical closure evidence unchanged and introduce a **closure-only B2 v2 successor**.

The successor versions the closure/currentness contract, not the B2 semantic classifications.

The following remain byte-identical:

```text
card_semantic_classifications.v1.json
requirement_family_catalog.v1.json
deck_row_classification_refs.v1.csv
all 402 classification identities
all 1883 terminal assignment edges
all historical B2 v1 records and evidence
```

A future `classification_closure.v2` will bind the exact existing B2 v1 semantic artifacts by explicit role, repository-relative path, schema identifier, and raw SHA-256, together with the pinned source-package identity and validated snapshot counts.

B2 v1 remains historical and verifiable forever. B2 v2 becomes current only through an explicit, atomic current-root admission after every migration and verification gate passes.

## 1. B2 v1 immutability

B2 v1 is immutable historical evidence.

Forbidden:

- editing `B2_DESIGN_SPEC.md` in place to repair the contradiction;
- modifying `classification_closure.v1.json`;
- rewriting B2 v1 semantic artifacts;
- recomputing historical identities to make v1 look consistent;
- treating a later ADR as permission to reinterpret existing v1 bytes.

Historical v1 verification continues to use the exact historical bytes and the existing v1 contract implementation.

## 2. Closure-only versioning

The new version applies to the **closure contract**.

It does not imply:

```text
classification v2
family catalog v2
projection v2
```

unless a future independent semantic change requires those versions.

The closure v2 contract may bind v1 semantic artifacts explicitly. This is valid because the referenced semantic snapshot is unchanged; only its current closure/root contract is superseded.

## 3. Immutable legacy and additive v2 source roles

Persisted source bindings retain their exact historical literals. The existing
role `b2_closure` is the immutable legacy B2 closure-v1 role. It is not an
alias, shorthand, normalized spelling, or dynamically version-resolved name.

`artifact_role == "b2_closure"` MUST resolve exactly to:

```text
path   = sources/m2_5/closures/B2/classification_closure.v1.json
schema = manafold.m2.5.b2.classification-closure.v1
```

The historical record/source closure supplies the exact raw SHA-256 and
source-package identity. Pairing `b2_closure` with a v2 path/schema, another
version, or another path fails closed. Existing historical source-binding and
authority identities remain unchanged.

The additive role `b2_closure_v2` is reserved for the future closure-v2
contract. It MUST resolve only to the exact v2 path, schema, raw SHA-256,
source-package binding, and validated snapshot metadata. It MUST NOT resolve
v1.

The role/version matrix is closed:

```text
v1 + b2_closure       = VALID
v1 + b2_closure_v2    = INVALID
v2 + b2_closure       = INVALID
v2 + b2_closure_v2    = VALID
```

Unknown roles, versions, and combinations fail closed. The nonexistent
`b2_closure_v1` spelling is not introduced; the existing `b2_closure` literal
already uniquely owns the historical v1 meaning. No role alias or role-name
normalization may erase this distinction.

## 4. Exact closure-v2 artifact bindings

Every semantic artifact referenced by closure v2 is bound by a closed record containing:

```text
artifact_role
repository_relative_path
schema_identifier
raw_sha256
```

The binding set is deterministic and canonically ordered by `artifact_role`.

Duplicate or unknown roles fail closed.

Closure v2 binds at minimum the exact existing B2 semantic snapshot roles:

```text
b2_classifications_v1
b2_family_catalog_v1
b2_projection_v1
```

The exact admitted path/schema pair for each role is closed by the v2 contract.

Path alone is insufficient. Schema alone is insufficient. Raw SHA alone without role/path/schema context is insufficient.

## 5. Source-package and consistency binding

Closure v2 binds the exact pinned source package:

```text
source_package_sha256 =
99b33945a3e0c7b2982734e65f770715029ce6acd500104bde48e8466eed1a90
```

It also validates these snapshot consistency values:

```text
oracle_semantic_identity_count = 402
classification_count           = 402
terminal_assignment_edge_count = 1883
deck_row_count                  = 441
projection_row_count            = 441
historical_family_count         = 216
catalog_family_count            = 216
```

These counts are consistency constraints only. They do not replace exact byte or semantic identity checks.

A closure with correct counts but incorrect referenced bytes fails.

## 6. Sole current-root admission

Currentness is represented explicitly by one authoritative closed admission record, conceptually `B2ClosureCurrentRootV1`.

The exact generated type/file name is fixed by the implementation contract, but the semantic shape is:

```text
schema
artifact_role
closure_version
repository_relative_path
closure_schema_id
closure_raw_sha256
source_package_sha256
```

There must be exactly one admitted current B2 closure root.

Role presence never determines currentness; only the explicit current-root
admission does.

Required fail-closed invariants:

```text
zero current roots             -> FAIL
multiple current roots         -> FAIL
unknown root role/version      -> FAIL
unsupported version            -> FAIL
path mismatch                  -> FAIL
schema mismatch                -> FAIL
raw digest mismatch            -> FAIL
source-package mismatch        -> FAIL
```

Currentness MUST NOT be selected by:

- latest/highest version;
- lexical version or filename order;
- directory enumeration;
- filesystem mtime;
- file presence;
- caller preference;
- "if v2 exists" logic;
- first-valid closure;
- retrying another version after failure.

## 7. Currentness lifecycle and atomic adoption

### 7.1 `V1_CURRENT`

The explicit admission identifies `(v1, b2_closure)`.

v1 is current for new construction because it is explicitly admitted, not because v2 is absent.

### 7.2 `V2_READY_NOT_ADOPTED`

A v2 closure may exist and verify successfully while the admission still identifies v1.

In this state:

```text
V2_CURRENT = NO
```

No current consumer may select v2.

### 7.3 `V2_ADOPTED`

The transition to v2 is one atomic current-root admission change after all mandatory adoption gates pass.

After adoption:

```text
current root for new construction = b2_closure_v2
v1 current eligibility            = NO
b2_closure historical verification = YES
```

A partially implemented or partially migrated v2 is never current.

## 8. Mandatory adoption gates

The v1→v2 switch is forbidden until all applicable gates are `PASS`, including:

- v2 schema/DTO closure;
- v2 positive checker;
- v2 negative matrix;
- exact B2 v1 semantic byte parity;
- exact artifact role/path/schema/SHA bindings;
- source-package binding;
- count consistency;
- historical v1 verification preservation;
- versioned source-role admission;
- AuthoritySourceResolver v2 readiness;
- AuthorityValidator v2 readiness;
- B1 current evidence-root readiness;
- C current source-root readiness;
- RPA/Context/Host/current consumer readiness where applicable;
- required local repository/Rust/Python/schema/conformance gates;
- hosted exact-head CI;
- independent review;
- explicit maintainer adoption action.

A required `FAIL`, `BLOCKED`, `NOT_RUN`, or unresolved `EXPERIMENTAL` status prevents adoption.

## 9. Fail-closed version handling

The implementation contract must preserve distinct rejection categories for at least:

```text
CURRENT_ROOT_MISSING
MULTIPLE_CURRENT_ROOTS
UNKNOWN_CLOSURE_VERSION
UNSUPPORTED_CLOSURE_VERSION
CURRENT_ROOT_ROLE_MISMATCH
CURRENT_ROOT_PATH_MISMATCH
CURRENT_ROOT_SCHEMA_MISMATCH
CURRENT_ROOT_DIGEST_MISMATCH
SOURCE_PACKAGE_MISMATCH
REFERENCED_SEMANTIC_ARTIFACT_MISMATCH
UNKNOWN_SOURCE_ROLE
DUPLICATE_SOURCE_ROLE
CURRENT_CONSUMER_VERSION_UNSUPPORTED
```

Exact names may be finalized in the generated contract vocabulary, but the semantic categories may not be removed.

Explicitly forbidden:

- silent v2→v1 fallback;
- retrying another closure version after validation failure;
- accepting whichever version validates first;
- ignoring an unknown version;
- treating an unknown future version as backward-compatible;
- accepting v2 while any referenced v1 semantic artifact bytes differ from the admitted bindings.

## 10. Historical verification and current construction are distinct operations

### Historical verification

Input is an exact historical record.

The verifier reads the exact role stored by that historical record. The
literal `b2_closure` resolves through the closed v1 path/schema table and the
verifier checks those exact historical bytes.

It does not consult the current-root admission record.

A historical record containing `b2_closure` remains valid after v2 adoption.
The verifier does not transform the role to another literal and does not
recompute historical IDs merely for migration.

### Current authority construction

The constructor resolves exactly the sole admitted current B2 closure root.

Before adoption, current construction uses `(v1, b2_closure)`. After v2
adoption, current construction that requires current B2 closure authority uses
`(v2, b2_closure_v2)`.

After adoption, `b2_closure` is historical-only for new current construction.
If v2 is unsupported or invalid, construction fails. It does not fallback to
the legacy role.

## 11. B1.Final migration ownership

The closure-root migration does not change B1 semantic citations.

Required invariants:

```text
B1 semantic payload rewrite          = NO
historical B1.Final mutation         = NO
historical B1 identities recomputed  = NO solely for closure migration
new current evidence-root binding    = YES where required
```

Future current B1/B2 dependency evidence may bind the adopted v2 closure explicitly.

That is additive evidence-root migration, not semantic B1 rematerialization.

## 12. C migration ownership

Historical C artifacts remain unchanged and retain their exact historical source roots.

Required invariants:

```text
historical C mutation             = NO
CandidateIdentity rewrite         = NO
SourceInstanceV1 rewrite          = NO
semantic class identity rewrite   = NO unless independently required
new current B2-root binding       = YES for future/current work where required
```

The closure migration does not resolve candidate semantics.

Candidate 3 remains `LEGITIMATELY_UNRESOLVED` and `cap.exile` must not be substituted with `cap.mass_exile`.

Candidate 1 and Candidate 4 require later fresh semantic review against effective B2 after the closure contradiction is resolved.

## 13. Authority migration ownership

Historical Authority records remain byte-identical and historically verifiable, including where applicable:

- RelationProof;
- RelationApplication V1/V2;
- ContextProof;
- ContextApplication V2/V3;
- HostBinding;
- Review/Acceptance events.

A change in the current B2 closure root alone does not authorize recomputing historical theorem/application/context/host/event identities.

Historical records using `b2_closure` remain historical-v1 records. Future
records after v2 adoption use the explicit `b2_closure_v2` role where the
owning contract requires current B2 closure authority.

If an existing identity preimage includes the exact closure binding, migration must use an additive successor/new record or version under the owning contract. Existing history is never mutated.

## 14. Evidence DAG

The closure-v2 evidence graph is acyclic:

```text
immutable B2-v1 semantic artifacts
        ↓
classification_closure.v2
        ↓
v2 post-verification summary/evidence
        ↓
current-root admission
```

Closure v2 must not bind:

- itself;
- its own post-verification summary;
- the current-root admission record.

The current-root admission is downstream of verified closure evidence.

## 15. Maintainer workflow

After v2 adoption there is one normal current path and one explicit historical path.

### Normal path

```text
verify current B2
    -> resolve sole current-root admission
    -> verify v2
```

### Historical forensic path

```text
verify historical B2 v1
    -> explicit historical-v1 mode / exact historical record
    -> verify v1 bytes
```

The maintainer must not manually choose between two generic competing current checkers.

Exact CLI spelling is an implementation detail, but current-vs-historical mode separation is normative.

## 16. Implementation slicing

ADR acceptance occurs before implementation and is not an implementation slice.

Implementation is divided into six separately reviewable slices.

### Slice 0 — ADR acceptance only

Accept the architecture. No executable change or current-root record is
authorized by this slice.

### Slice 1 — Contract and currentness definitions

Define the legacy `b2_closure` contract, additive `b2_closure_v2` contract,
closed role/version matrix, currentness model, fail-closed vocabulary, golden
fixtures, and negative fixtures. No v2 currentness.

### Slice 2 — v2 closure builder/verifier

Materialize and verify closure v2 over exact immutable v1 semantic artifacts.
Prove path/schema/SHA/source-package/count and semantic parity. v2 remains
NON_CURRENT.

### Slice 3 — historical/current source resolution

Preserve exact historical `b2_closure` resolution and add candidate
`b2_closure_v2` resolution. Reject aliases, role/version mismatches, and
identity mutation. v2 remains NON_CURRENT.

### Slice 4 — downstream current-consumer readiness

Add required B1/C/Authority successor-contract handling without historical
mutation. Historical contracts remain untouched. v2 remains NON_CURRENT.

### Slice 5 — adoption evidence

Run exact-head readiness gates, parity, no-alias/no-mutation evidence,
source-binding parity, integration, hosted CI, and independent review. No
current-root switch.

### Slice 6 — atomic adoption

Only after Slice 5 PASS, atomically change the sole current-root admission:

```text
(v1, b2_closure) -> (v2, b2_closure_v2)
```

Run post-adoption verification. No slice may prestage the semantic
responsibilities of a later slice, and every later slice requires separate
authorization.

## 17. Pilot status preserved

The closure-contract decision does not accept or reclassify pilot relations:

```text
Candidate 2 = REVIEW_SUPPORTS_PROPOSAL
Candidate 5 = REVIEW_SUPPORTS_PROPOSAL
Candidate 3 = LEGITIMATELY_UNRESOLVED
Candidate 1 = previous B2 blocker invalidated; fresh semantic review remains future work
Candidate 4 = previous B2 blocker invalidated; fresh semantic review remains future work
```

These statuses are review provenance only. They do not create theorem,
application, acceptance, C, or production Authority records.

## 18. Compatibility

### Historical compatibility

B2 v1 remains readable/verifiable forever using its exact historical contract.

Historical B1/C/Authority records remain historically meaningful and keep their original source bindings.

### Current compatibility

After v2 adoption, new current consumers use the sole admitted v2 root when current B2 closure authority is required.

No silent compatibility alias exists between v1 and v2 closure roles.

### Schema evolution

v2 uses a new closure schema identifier. The semantic artifact schemas remain v1 because those artifacts are unchanged.

Unknown future closure versions fail closed until explicitly supported.

## 19. Consequences

### Positive

- resolves the v1 closure-contract contradiction without semantic churn;
- preserves all historical B2 evidence;
- preserves 402 classifications and 1,883 assignments;
- creates explicit currentness rather than implicit version selection;
- makes version failure behavior deterministic and auditable;
- separates historical verification from current construction;
- keeps solo-maintainer workflow simple after adoption.

### Cost

- new closure-v2 schema/checker contract;
- additive source-role/version handling;
- explicit downstream readiness work before adoption;
- one current-root admission mechanism;
- retained historical-v1 verifier path.

These costs are accepted because they resolve an existing normative contradiction without rematerializing semantic truth.

## 20. Rejected alternatives

### Mutate B2 v1

Rejected. v1 bytes are historical closure evidence and already downstream-bound.

### Full B2 v2 semantic rematerialization

Rejected as unnecessary. No semantic classification defect requires reissuing 402 records or 1,883 assignments.

### Additive overlay over v1

Rejected. Existing B2 v1 inventory/checking does not admit an overlay, and C/Authority supersession contracts do not own B2 artifact lifecycle.

### ADR-only clarification

Rejected as insufficient. It does not remove the contradiction inside immutable v1 evidence.

## 21. Non-goals

This decision does not:

- change Magic rules or engine behavior;
- change any card classification;
- change any B2 family definition;
- change classification identities;
- change the Candidate Universe;
- resolve Candidate 3;
- accept Candidate 1/2/4/5 relation semantics;
- create production Relation/Context/Host authority;
- run Domain Audit 55/55;
- change ranking;
- lock a deck pair;
- authorize or begin M3.

## 22. Review provenance

The reviewed architecture was formed through the following evidence chain:

```text
initial B2 contradiction research
    -> closure-only v2 successor recommended
first independent ADR review
    -> REJECT: 4 MAJOR / 1 MINOR
Remediation 02
    -> independent re-review: REJECT, 1 MAJOR legacy-role gap
Remediation 03
    -> final independent re-review: APPROVE_ADR_CANDIDATE, 0 / 0 / 0 / 0
```

The temporary research files are provenance only and are not runtime or
repository dependencies.

## 23. M2.5 gate consequence

Until closure v2 is implemented, independently verified, and atomically adopted:

```text
M2_5_FREEZE          = BLOCKED
ACCEPTANCE           = NOT_AUTHORIZED
PRODUCTION_AUTHORITY = NOT_AUTHORIZED
DOMAIN_AUDIT_55_55   = NOT_RUN
C_CHANGE             = STOP
RANKING              = NOT_AUTHORIZED
DECK_PAIR_LOCK        = NOT_AUTHORIZED
M3                   = NOT_AUTHORIZED
```

After v2 adoption, these downstream gates remain separately gated. Closure-v2 adoption alone does not pass them.
