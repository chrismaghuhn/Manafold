# M2.5.C Cross-Deck Host Binding Implementation Plan

**Status:** provisional
**Stability:** provisional

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a versioned, rules-neutral host-binding underlay that binds one exact Candidate/SourceInstance member to correlated discovery and participant-host evidence without changing any existing V1 identity, C artifact, or semantic conclusion.

**Architecture:** Keep all accepted V1 theorem/application contracts byte-for-byte unchanged. Add a separate Python host-binding contract module for the member-atomic `hbc.v1` claim, correlated `HostRealizationWitnessV1` records, semantic-application links, and the V2 authority envelope. Source verification continues through `AuthoritySourceResolver`; the new validator interprets only exact verified rows/records needed for host realization.

**Tech Stack:** Python 3.11, frozen dataclasses, existing canonical-CBOR/digest-envelope primitives, JSON Schema Draft 2020-12, `unittest`, `jsonschema`, and the existing repository/REV3 source resolver.

---

### Task 1: Freeze typed host-binding contracts and identities

**Files:**
- Create: `python/src/mtgml/host_binding.py`
- Test: `python/tests/test_host_binding_contract.py`

- [x] **Step 1: Write RED tests for exact member/witness shapes**

Use the wished-for public types and assert the following canonical shapes:

~~~python
ApplicationMemberKeyV1 =
[
    candidate_id_utf8,
    candidate_identity_digest_bytes,
    source_instance_id_utf8,
]

HostRealizationWitnessV1 =
[
    discovery_mapping_ref,
    deck_row_ref,
    osi_ref,
    b2_assignment_refs,
]

ParticipantHostRealizationV1 =
[
    member_key,
    participant_position_u32,
    participant_ref,
    host_ref,
    witnesses,
]
~~~

Test missing witnesses, duplicate participant positions, noncanonical witness order, duplicate witnesses, and a realization whose host differs from its discovery binding.

- [x] **Step 2: Run RED**

~~~powershell
python -m unittest python.tests.test_host_binding_contract -v
~~~

Expected: missing `mtgml.host_binding` types or equivalent contract failures.

- [x] **Step 3: Implement the minimal immutable contract**

Create frozen dataclasses with tuple-backed nested values and exact `to_cbor()` arrays. Reuse `encode_canonical`, `encode_envelope`, and `hash_envelope` from `mtgml.persistence`. Do not add host fields to any V1 application or CandidateIdentity type.

Define the sole new identity registry in this module:

~~~python
HostBindingIdentityKind:
    CROSS_DECK_HOST_BINDING_CLAIM
    CROSS_DECK_HOST_BINDING_CLAIM_RECORD
    CROSS_DECK_HOST_BINDING_SUPERSESSION
~~~

Use fixed `hbc.v1/`, `hbcr.v1/`, and `hbcs.v1/` prefixes and fixed semantic domains/input schemas. The claim preimage contains exactly one member, complete discovery bindings, complete correlated realizations, and `observed_host_relationship`; it never contains an application Record ID.

- [x] **Step 4: Run GREEN**

~~~powershell
python -m unittest python.tests.test_host_binding_contract -v
~~~

Expected: all contract tests pass.

### Task 2: Add the closed V2 authority schema

**Files:**
- Create: `schemas/interaction-review-authority.v2.schema.json`
- Create: `schemas/review-acceptance-event.v2.schema.json`
- Create: `conformance/fixtures/authority/interaction_review_authority.v2.json`
- Create: `conformance/fixtures/authority/review_acceptance_event.v2.json`
- Modify: `schemas/README.json`
- Modify: `scripts/run_m2_final_closure.py`
- Modify: `scripts/validate_schemas.py`
- Modify: `python/tests/test_host_binding_contract.py`

- [x] **Step 1: Write RED schema tests**

Test that the V2 schema rejects:

~~~text
a V1 root containing a host claim
a claim with application_record_id
a claim with more than one member
a witness with independent deck/OSI/B2 arrays
an unknown V2 source role
an application link with an invalid semantic application ID
~~~

- [x] **Step 2: Run RED**

~~~powershell
python -m unittest python.tests.test_host_binding_contract.HostBindingSchemaTests -v
~~~

Expected: the V2 schema is absent.

- [x] **Step 3: Implement the V2 root and closed definitions**

Define the exact root:

~~~json
{
  "schema": "manafold.m2.5.c.interaction-review-authority.v2",
  "base_authority_v1_binding": {},
  "source_bindings": [],
  "cross_deck_host_binding_claim_records": [],
  "cross_deck_host_binding_claim_supersession_records": [],
  "application_host_bindings": []
}
~~~

Use `additionalProperties: false` throughout. Define closed wire shapes for:

~~~text
ApplicationMemberKeyV1
DiscoveryHostRefV1
HostEvidenceRefV2
HostRealizationWitnessV1
ParticipantHostRealizationV1
CrossDeckHostBindingClaimV1
CrossDeckHostBindingClaimRecordV1
CrossDeckHostBindingClaimSupersessionV1
ApplicationHostBindingV1
~~~

Include explicit V2 source roles for `rev3_card_requirement_map` and `rev3_osi_source_records`, while retaining existing B2 roles where reused. Do not add any of these definitions to the V1 schema.

- [x] **Step 4: Run schema gates**

~~~powershell
python -m unittest python.tests.test_host_binding_contract -v
python scripts/validate_schemas.py
~~~

Expected: both commands exit with code 0.

### Task 3: Implement source-bound correlated Witness resolution

**Files:**
- Create: `scripts/authority_host_binding.py`
- Modify: `scripts/README.md`
- Test: `python/tests/test_authority_host_binding.py`

- [x] **Step 1: Write RED source-join tests**

Use an injected `Rev3ArchiveStore` and the existing `AuthoritySourceResolver`. Cover:

~~~text
exact discovery-map row to deck host
exact deck row to OSI
exact OSI to B2 assignment
wrong discovery host
wrong deck row
wrong OSI
wrong B2 assignment
missing mapping row
duplicate mapping witness
cross-snapshot witness
~~~

- [x] **Step 2: Run RED**

~~~powershell
python -m unittest python.tests.test_authority_host_binding -v
~~~

Expected: source-bound host resolver functions are missing.

- [x] **Step 3: Implement the minimal resolver**

The resolver must accept a resolver-verified `ResolvedSourceInstance` and obtain all raw bytes through `AuthoritySourceResolver`. It may parse verified REV3 CSV/JSONL and call existing B2 resolution methods, but it must not read files or ZIP members directly.

For every participant position:

~~~text
resolve the exact candidate-side discovery binding
require one ParticipantHostRealizationV1
require at least one correlated Witness
require witness mapping refs to equal the discovery mapping-ref set
require deck row host == realization host
require realization host == discovery host
require exact OSI and B2 assignment joins
~~~

Return only immutable source facts. Never return source/affected, causal direction, interaction, separation, domain applicability, or context conclusions.

- [x] **Step 4: Add the real canary probe**

If `MANAFOLD_SOURCE_ARCHIVE` is absent, report `BLOCKED`. If present, require the pinned REV3 archive and exact joins. Do not write a production claim or modify C artifacts.

- [x] **Step 5: Run GREEN**

~~~powershell
python -m unittest python.tests.test_authority_host_binding -v
~~~

### Task 4: Implement V2 cross-layer closure validation

**Files:**
- Create: `scripts/authority_v2_validator.py`
- Create: `docs/maintenance/CROSS_DECK_HOST_BINDING_REVIEW_CHECKLIST.md`
- Modify: `docs/normative-document-register.v1.json`
- Modify: `scripts/README.md`
- Test: `python/tests/test_authority_v2_validator.py`

- [x] **Step 1: Write RED closure tests**

Cover:

~~~text
member-atomic claim acceptance
different Relation/Domain/Context partitions with exact claim unions
missing or duplicate claim member
same member mapped to different current claims
application_record_id rejected from claim
semantic rpa.v1/dpa.v1/cpa.v1 links
observed_host_relationship mismatch with RelationProof subject
observed_host_relationship mismatch with ContextProof subject
superseded claim rejected
V2 claim in V1 root rejected
V1 acceptance event cannot accept a host claim
~~~

- [x] **Step 2: Run RED**

~~~powershell
python -m unittest python.tests.test_authority_v2_validator -v
~~~

Expected: the V2 validator module is missing.

- [x] **Step 3: Implement V2 validation**

The validator must:

~~~text
validate the closed V2 root
resolve the exact V1 authority artifact through AuthoritySourceResolver
invoke AuthorityValidator for that V1 artifact
register hbc claim records and same-kind supersessions
reject duplicate/ambiguous current claims per member
validate application_kind with rpa.v1/dpa.v1/cpa.v1
compare claim-member unions with exact V1 application members
compare observed_host_relationship with every relevant theorem subject
reject superseded or revoked claims as current closure
~~~

The validator must use semantic Application IDs and must never require an Application Record ID while constructing a claim. Rejected validation mutates nothing.

- [x] **Step 4: Run GREEN**

~~~powershell
python -m unittest python.tests.test_authority_v2_validator -v
~~~

### Task 5: Document, verify, commit, and stop

**Files:**
- Modify: `scripts/README.md`
- Modify: `docs/superpowers/plans/2026-09-01-m2-5-c-cross-deck-host-binding.md`

- [x] **Step 1: Document the ownership boundary**

Document:

~~~text
hbc.v1 is member-atomic
ApplicationHostBindingV1 targets semantic application IDs
Witnesses correlate mapping/deck/OSI/B2 evidence atomically
discovery host is not semantic ownership
observed host relation is compared with theorem expectation
no source/affected or interaction conclusion is produced
~~~

- [x] **Step 2: Run focused verification**

~~~powershell
python -m unittest python.tests.test_host_binding_contract python.tests.test_authority_host_binding python.tests.test_authority_v2_validator -v
python -m ruff format --check python scripts
python -m ruff check python scripts
python -m mypy --config-file python/pyproject.toml
git diff --check
~~~

Expected: all commands exit with code 0.

- [ ] **Step 3: Run the normal repository gates**

~~~powershell
python scripts/run_python_tests.py
python scripts/generate_contracts.py --check
python scripts/verify_repository.py
python scripts/check_rust_source_structure.py
python scripts/check_documentation.py
python scripts/validate_schemas.py
python scripts/validate_maintainer_artifacts.py
python scripts/verify_python_toolchain.py
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
python scripts/run_checks.py fast
python scripts/run_checks.py integration
python scripts/run_checks.py certification
python scripts/verify_archive_reproducibility.py
~~~

Report `PASS`, `FAIL`, `NOT_RUN`, or `BLOCKED` only from actual execution. The canary remains `UNRESOLVED`; no production Acceptance Event or C classification is created.

- [ ] **Step 4: Inspect scope and create one standalone commit**

~~~powershell
git status --short
git diff --stat origin/master...HEAD
git diff --name-only origin/master...HEAD
git add docs/superpowers/plans/2026-09-01-m2-5-c-cross-deck-host-binding.md docs/maintenance/CROSS_DECK_HOST_BINDING_REVIEW_CHECKLIST.md docs/normative-document-register.v1.json python/src/mtgml/host_binding.py scripts/authority_host_binding.py scripts/authority_v2_validator.py scripts/README.md scripts/validate_schemas.py schemas/README.json schemas/interaction-review-authority.v2.schema.json schemas/review-acceptance-event.v2.schema.json conformance/fixtures/authority/interaction_review_authority.v2.json conformance/fixtures/authority/review_acceptance_event.v2.json python/tests/test_host_binding_contract.py python/tests/test_authority_host_binding.py python/tests/test_authority_v2_validator.py
git commit -m "feat: add cross-deck host binding authority underlay"
~~~

Before committing, confirm no V1 contract, C artifact, candidate classification, production acceptance event, or Slice 3B implementation changed.

- [ ] **Step 5: Push and stop**

~~~powershell
git push -u origin chris/m2-5-c-cross-deck-host-binding
~~~

Report exact HEAD, parent, origin/master, branch, changed files, focused/full gate results, real-probe status, and worktree status. Do not merge and do not start Slice 3B.
