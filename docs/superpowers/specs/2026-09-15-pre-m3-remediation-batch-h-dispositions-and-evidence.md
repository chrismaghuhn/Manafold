# Pre-M3 Remediation Batch H: Dispositions and Evidence

**Date:** 2026-09-15  
**Repository:** `chrismaghuhn/Manafold`  
**Issue:** #164  
**Status:** evidence recorded; independent plan approval and hosted exact-head review pending  
**BASE:** `ff37f0896cdbb8e2faea424859faf128155b4579`  
**Branch:** `chris/pre-m3-remediation-batch-h-conformance-evidence-closure`

## Scope and process status

Batch H remained bounded to conformance evidence, test-only fixture support,
and the evidence owners named by the approved design. No Magic semantics,
cards, Commander capability, delivery mechanism, replay/checkpoint version, or
digest/RNG contract was added.

The design was independently approved at exact head
`e6fb5d8d164bc0a4d3a01c4a52dfd0840e5123c7` after three review rounds. The
implementation-plan review returned `REQUEST_CHANGES` at the original plan
head `1a0e4004bb53d3ecf253044d2a1f52ef0e79a078`; all four recorded corrections
were applied and committed in `19cd73f`. A second independent approval of the
corrected plan was not run after the user requested that no further subagents
be spawned. Therefore the plan-governance gate remains explicitly
`PARTIAL/NOT_RUN`; this record does not claim independent final plan approval.

## Evidence input identity

The complete applicable direct verification matrix passed at:

    CODE_VERIFICATION_HEAD = f62c03f9bd792edf16f69a699daf3960173a69a3
    EVIDENCE_INPUT_HEAD = f62c03f9bd792edf16f69a699daf3960173a69a3
    WORKTREE_CLEAN_AT_INPUT = YES

`FINAL_EVIDENCE_HEAD` is intentionally not written into this file because
that would make the evidence self-referential. It is recorded externally
after the evidence commit.

## Finding dispositions

| Finding | Disposition | Evidence result | Compatibility / artifact meaning |
|---|---|---|---|
| EVD-001 | CONFIRMED | Rust positive manifest consumer, exact CBOR/envelope boundaries, and Python boundary parity pass. | No valid persistence bytes or meaning changed. |
| EVD-002 | CONFIRMED | Direct production `uniform_below_u64` rejection KAT passes with rejected-word cursor advancement. | RNG algorithm, threshold, HMAC, stream derivation, and KAT meaning unchanged. |
| EVD-003 | CONFIRMED | Accepted-entry, checkpoint, restore, and fork tests now assert exact progression and source nonmutation. | No checkpoint or replay contract change. |
| EVD-004 | CONFIRMED | Complete normalized authoritative-state relation passes all ten axes and rejects a contaminated object-rename/life pair. | Player-facing projection and information boundary unchanged. |
| EVD-005 | CONFIRMED | Fifteen-row rejection matrix compares complete pre-state-derived `PlayerStepV2` values and unchanged fingerprints. | No player-step schema or endpoint behavior change. |
| EVD-006 | CONFIRMED | Shared accepted-entry and count-stage helpers require `Accepted`, exact revision/counter progress, and semantic mutation. | No public API or replay meaning change. |
| EVD-007 | CONFIRMED | Empty production path is classified as `MissingChoice`; completeness remains exact-one. | No new action semantics. |
| EVD-008 | CONFIRMED | Bounded family-specific complement probes execute through real branches; advertised rejects are empty and out-of-contract accepts are zero. | No production legality change. |
| EVD-009 | CONFIRMED | Shared budgets, checked reference transitions, canonical atoms, bounded Order range, and trace-length defects pass. | Oracle remains conformance-only. |
| EVD-010 | CONFIRMED | Eventful live product now comes from `PlayerEndpoint::submit`; replay reprojection is nonempty and byte-exact. | No non-actor `PlayerStep` or delivery API added. |
| EVD-011 | CONFIRMED | Reidentification selects a hidden object through P1's retired physical chain and preserves the retired opaque identity. | No lifecycle semantic expansion. |
| EVD-012 | CONFIRMED | Fingerprints are revision-bound and retain non-secret manifest/schema/deck/digest-reference identity. | Root seed is still excluded from the default diagnostic surface; no version change. |
| EVD-013 | CONFIRMED | Structural guard comments are separate from controlled mutant evidence; legacy test names are retained for the pinned M2.G gate manifest, and controlled tests retain validity gates and clean-outcome prerequisites. | No hidden callback or production mutation seam. |
| EVD-014 | CONFIRMED | Fixture operations roll back workspace/events/offset on late error and require the explicit global SyntheticM1 stream. | Feature-gated testkit only; no player/replay API change. |

FND-016B is `CLOSED` by the complete rejection-product matrix. The expected
product is built before submit from pre-information, pre-decision, pre-status,
and the independently declared row code.

FND-026B remains `BLOCKED_CONTRACT_AMBIGUITY` with
`MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = NO` and
`FND_026B_FREEZE_READINESS = NONBLOCKING_PROVISIONAL_POLICY`. H does not
invent a non-actor product or delivery mechanism.

FND-028 remains `BLOCKED_CONTRACT_AMBIGUITY` with
`MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = YES` and
`FND_028_FREEZE_READINESS = FREEZE_BLOCKER`. H does not select a PlayerId(0)
policy.

## RED and characterization record

Actual named behavioral RED reproductions executed before their corresponding
minimal fixes were:

| Finding | RED reproduction | Observed reason | GREEN |
|---|---|---|---|
| EVD-007 | `empty_production_path_is_missing_choice` | `Some(Vec::new())` was treated as represented. | `cargo test -p mtgml-conformance --all-features --locked completeness` — 5 passed. |
| EVD-008 | `generate_probes_contains_the_bounded_invalid_complement` | The old explorer omitted the bounded unknown/duplicate/reversed/wrong-variant complement. | Same named test — passed after the finite grammar was added. |
| EVD-011 | `reidentification_of_a_randomized_card_uses_fresh_opaque_and_keeps_old_retired` | The helper could choose the first hidden object whose physical card was not in P1's retired chain. | `cargo test -p mtgml-conformance --all-features --locked lifecycle` — 10 passed. |

The EVD-001 and EVD-002 changes are evidence-only additions and had no
production behavioral RED. The source-level and regression tests for EVD-003,
EVD-004, EVD-006, EVD-009, EVD-012, EVD-013, and EVD-014 are green on the
verified head; a separate pre-fix RED transcript for each was not retained in
the final work log. No such omission is counted as a PASS claim for an
unexecuted command.

## Focused evidence

The following focused tests were executed successfully on the current code
head:

| Evidence area | Executed evidence |
|---|---|
| Persistence | `persisted_positive_fixture_manifest_matches_rust_bytes_and_meaning`; `cbor_resource_boundaries_are_exact_at_each_declared_boundary`; Python `test_persistence_resource_boundaries_match_rust_contract`. |
| RNG rejection | `production_sampler_consumes_rejected_words_and_advances_the_cursor`; full `mtgml-random` suite: 43 passed. |
| Checkpoint/fork | `restore_decision_rich`, `restore_information_rich`, `corrupt_checkpoint_restores_fail_closed`, `fork_decision_rich`, `fork_information_rich`, `cross_mutation_isolation_matrix`, `accepted_determinism_twins`. |
| Noninterference | Ten axis tests in `paired_matrix`, `paired_rejection_parity_hidden_axes`, and `contaminated_object_rename_plus_life_change_is_rejected`. |
| Rejection products | `semantic_matrix_returns_the_independent_complete_rejected_product`; `semantic_matrix_fingerprint_stable`; 15 declared rows. |
| Legal space | Full `legal_space` group: 29 passed, including complement probes, empty paths, bounded Order failure, reference transition validation, canonical declaration order, and trace-length controls. |
| Eventful replay | `eventful_replay_reprojects_both_perspectives_byte_exactly`; live endpoint product and replay projection are nonempty and byte-exact. |
| Lifecycle | Full `lifecycle` group: 10 passed, including physical-chain reidentification. |
| Fingerprint | Full `fingerprint` group: 11 passed, including revision-bound capture, manifest identity, and digest-reference domains. |
| Mutation guards | Full `mutants` group: 15 passed; structural guard names are distinct from controlled mutants. |
| Fixture transactionality | Feature-gated `fixture_support` group: 4 passed, including all late-binding rollback cases and explicit global-stream failure. |

## Bounded 30-row integration matrix

| # | Obligation | Result and exact evidence |
|---:|---|---|
| 1 | Rust positive persistence corpus | PASS — Rust manifest consumer; `mtgml-persistence`: 12 passed. |
| 2 | Persistence exact boundary matrix | PASS — Rust private Decoder/envelope boundary test and Python parity test. |
| 3 | Production RNG rejection sampler | PASS — direct `uniform_below_u64` KAT. |
| 4 | Cursor advancement through rejection | PASS — five raw words consumed; cursor ends at five; accepted word is used. |
| 5 | Checkpoint correctness beyond twin equality | PASS — exact accepted-entry/count progression helpers. |
| 6 | Checkpoint source nonmutation | PASS — restore/fork source checkpoint and complete fingerprint equality. |
| 7 | Fork independence | PASS — fork-side accepted/rejected/restore mutations leave parent unchanged. |
| 8 | Authorized noninterference relation | PASS — complete state normalizer for all ten declared axes. |
| 9 | Cross-axis contamination rejection | PASS — object rename plus unrelated life change returns `UnauthorizedStateDifference`. |
| 10 | Rejection backend state preservation | PASS — all 15 matrix rows preserve complete fingerprint. |
| 11 | Independent complete rejected PlayerStep | PASS — all 15 actual steps equal pre-state-derived expectations. |
| 12 | Accepted-transition proof | PASS — entry and count helpers require `Accepted` and exact progression. |
| 13 | Empty legal path completeness | PASS — `MissingChoice` regression. |
| 14 | Every bounded reference choice represented | PASS — live matrix exact-one paths; reference space contains 10 choices. |
| 15 | Invalid-complement soundness | PASS — real explorer records no advertised rejection and zero out-of-contract accepts. |
| 16 | Explorer budget exhaustion | PASS — `OrderProbeRangeExceeded`, numeric, candidate, and generated-answer bounds are typed. |
| 17 | Oracle declaration-order canonicalization | PASS — reversed declaration expected atoms and complete spaces compare equal. |
| 18 | Trace-length drift | PASS — extra observed request is `TraceLengthMismatch`. |
| 19 | Eventful PlayerStep nonempty path | PASS — real endpoint returns nonempty observed events. |
| 20 | Eventful replay product parity | PASS — P1/P2 replay projections are nonempty; P1 step is byte-identical. |
| 21 | Lifecycle retirement chain coherence | PASS — retired opaque/physical chain is retained and validated. |
| 22 | Lifecycle reidentification chain selection | PASS — selected physical card belongs to P1 retired set. |
| 23 | One coherent fingerprint instant | PASS — snapshot revisions and replay final identity equal the checkpoint; final checkpoint is unchanged. |
| 24 | Immutable fingerprint identity | PASS — manifest, schemas, decks, engine/kernel, and both digest-reference surfaces retained. |
| 25 | Mutation guard intended reachability | PASS — clean product outcome asserted for step mutant; structural guards labeled separately. |
| 26 | Fixture rollback after late failure | PASS — workspace/events/offset exact rollback for movement, occurrence, and RNG. |
| 27 | Explicit fixture RNG stream | PASS — only global `RandomStreamKeyV1::global(SyntheticM1)` is used. |
| 28 | No silent player-choice selection | PASS — fixture helpers perform declared lifecycle/RNG operations only; no player-choice helper added. |
| 29 | FND-026B freeze-readiness | PASS as a classification — blocked ambiguity, nonblocking provisional policy, no invented product. |
| 30 | FND-028 freeze-readiness | PASS as a classification — blocked ambiguity, freeze blocker, no invented zero policy. |

## Package and workspace verification

The owner-map package suites all passed:

| Package | Result |
|---|---:|
| `mtgml-model` | 8 passed |
| `mtgml-decision` | 12 passed |
| `mtgml-state` | 96 passed |
| `mtgml-random` | 43 passed |
| `mtgml-persistence` | 12 passed |
| `mtgml-replay` | 14 passed |
| `mtgml-rules` | 45 passed |
| `mtgml-observation` | 12 passed |
| `mtgml-environment` | 61 passed |
| `mtgml-conformance` | 142 passed |

The complete workspace command passed with 481 unit tests and zero failures;
all workspace doc-test groups also exited successfully.

## Python, schema, maintainer, and archive gates

- `scripts/run_python_tests.py --profile full`: PASS — 397 tests, 3 skipped.
  The skipped tests are `m2_h.test_m2_h_core_scenarios`,
  `m2_h.test_m2_h_isolation_scenarios`, and
  `m2_h.test_m2_h_rejection_scenarios`; all require the unavailable
  `MTGML_M2_ADAPTER_BIN`.
- `scripts/generate_contracts.py --check`: PASS.
- `scripts/validate_schemas.py`: PASS — 38 wire fixtures and 9 maintainer
  artifacts.
- `scripts/verify_repository.py`: PASS — 684 files, 38 golden fixtures, 46
  negative fixtures.
- `scripts/check_rust_source_structure.py`: PASS — 138 files.
- `scripts/check_documentation.py`: PASS — register, 44 ADRs, and links.
- `scripts/validate_maintainer_artifacts.py`: PASS — 7 artifacts.
- `scripts/verify_python_toolchain.py`: PASS — Python 3.13.15, Rust 1.85.1,
  and 6 direct tool pins.
- `scripts/run_checks.py fast`: PASS.
- `scripts/run_checks.py integration`: PASS.
- `scripts/run_checks.py certification`: PASS.
- `scripts/verify_archive_reproducibility.py`: PASS — 685 safe files. The
  final archive hash is recorded externally after the final evidence commit
  because embedding it would make the archive hash self-referential.

The repository wrappers were attempted separately and all four were blocked
before their underlying Python command could start because this host's WSL
cannot launch `/bin/bash`:

    just check-fast   = BLOCKED
    just check        = BLOCKED
    just check-all    = BLOCKED
    just archive-check = BLOCKED

Direct profile PASS results do not upgrade these wrapper statuses.

## Compatibility and safety classification

The actual diff contains Rust conformance-harness signature changes:
`ReferenceAutomaton` now returns typed `Result` values and the shared budget
has a `LegalSpaceBudget` owner with the existing explorer alias. This is not a
production/public player API.

The feature-gated `FixtureTransition` testkit behavior is now transactional
and explicit about its stream. Python changes are tests only. No wire/schema,
checkpoint, replay, digest-domain, RNG-algorithm, historical-artifact, or
Magic semantic meaning changed.

Default diagnostic surfaces continue to omit root seeds, raw stream keys and
cursors, physical cards, authoritative object/ability IDs, trusted bindings,
private knowledge, checkpoints, and full authoritative state. The secret
injection functions in `mutants.rs` remain controlled conformance mutants,
not default diagnostics.

## Required final matrix

```text
TASK = PRE_M3_REMEDIATION_BATCH_H
BASE = ff37f0896cdbb8e2faea424859faf128155b4579
HEAD = FINAL_EVIDENCE_HEAD recorded externally after the evidence commit
BRANCH = chris/pre-m3-remediation-batch-h-conformance-evidence-closure
PR = pending until the single branch is pushed

EVD_001 = CONFIRMED — positive corpus and exact persistence boundaries PASS
EVD_002 = CONFIRMED — production rejection-sampling KAT PASS
EVD_003 = CONFIRMED — checkpoint/restore/fork exact progression and nonmutation PASS
EVD_004 = CONFIRMED — complete authorized-difference relation PASS
EVD_005 = CONFIRMED — independent complete rejected PlayerStep matrix PASS
EVD_006 = CONFIRMED — accepted-transition progression proof PASS
EVD_007 = CONFIRMED — empty production path is MissingChoice PASS
EVD_008 = CONFIRMED — bounded invalid complement probes PASS
EVD_009 = CONFIRMED — bounded validated canonical explorer/oracle PASS
EVD_010 = CONFIRMED — real eventful endpoint and replay reprojection PASS
EVD_011 = CONFIRMED — retired physical/opaque lifecycle chain PASS
EVD_012 = CONFIRMED — revision-bound manifest-complete fingerprint PASS
EVD_013 = CONFIRMED — structural and controlled mutation evidence separated PASS; pinned legacy names retained
EVD_014 = CONFIRMED — transactional explicit-stream fixture support PASS

FND_016B = CLOSED

FND_026B_CURRENT_STATUS = BLOCKED_CONTRACT_AMBIGUITY
FND_026B_FREEZE_READINESS = MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = NO; NONBLOCKING_PROVISIONAL_POLICY

FND_028_CURRENT_STATUS = BLOCKED_CONTRACT_AMBIGUITY
FND_028_FREEZE_READINESS = MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = YES; FREEZE_BLOCKER

CONFIRMED = EVD_001..EVD_014; FND_016B
REJECTED = NONE
RESOLVED_ON_BASE = NONE
PARTIALLY_RESOLVED_ON_BASE = NONE
SPLIT_REQUIRED = NO
BLOCKED_CONTRACT_AMBIGUITY = FND_026B, FND_028
DEFERRED_P2 = FND_026C

VACUITY_GAPS_FOUND = EVD_003 twin-only parity; EVD_006 unproven acceptance; EVD_007 empty paths; EVD_008 incomplete complement; EVD_010 empty/manual eventful product; EVD_011 first-hidden selection; EVD_012 mixed-instant capture
COMMON_MODE_GAPS_FOUND = EVD_002 disconnected sampler stub; EVD_004 partial witness relation; EVD_005 code-only rejection assertion; EVD_009 silent oracle transition and trace drift; EVD_013 unrelated structural negatives; EVD_014 partial fixture mutation
PROOF_GAPS_FIXED = EVD_001..EVD_014 evidence-owner fixes listed above

RED_TESTS = empty_production_path_is_missing_choice; generate_probes_contains_the_bounded_invalid_complement; reidentification_of_a_randomized_card_uses_fresh_opaque_and_keeps_old_retired
CHARACTERIZATION_TESTS = Batch-G baseline workspace PASS; source-level fingerprint/reference checks; complete package and cross-language matrix
FOCUSED_TESTS = conformance 142 PASS; persistence 12 PASS; random 43 PASS; rules 45 PASS; environment 61 PASS; lifecycle 10 PASS; fingerprint 11 PASS; mutants 15 PASS
WORKSPACE_TESTS = cargo test --workspace --all-features --locked PASS — 481 unit tests, zero failures

PERSISTENCE_EVIDENCE = PASS
RNG_REJECTION_EVIDENCE = PASS
CHECKPOINT_PARITY_EVIDENCE = PASS
FORK_ISOLATION_EVIDENCE = PASS
NONINTERFERENCE_EVIDENCE = PASS
REJECTION_PRODUCT_EVIDENCE = PASS — 15 rows
LEGAL_SPACE_COMPLETENESS = PASS — 10 exact-one reference choices
LEGAL_SPACE_SOUNDNESS = PASS — no advertised rejects; zero out-of-contract accepts
EXPLORER_ORACLE_EVIDENCE = PASS
EVENTFUL_REPLAY_EVIDENCE = PASS
LIFECYCLE_FIXTURE_EVIDENCE = PASS
FINGERPRINT_EVIDENCE = PASS
MUTATION_GUARD_EVIDENCE = PASS
FIXTURE_TRANSACTIONALITY = PASS

FMT = PASS
CHECK = PASS
CLIPPY = PASS

PYTHON_FULL = PASS — 397 tests, 3 adapter skips
SCHEMA_GATES = PASS
MAINTAINER_GATES = PASS

DIRECT_FAST_PROFILE = PASS
DIRECT_INTEGRATION_PROFILE = PASS
DIRECT_CERTIFICATION_PROFILE = PASS
DIRECT_ARCHIVE_CHECK = PASS — 685 safe files; final hash recorded externally after the final evidence commit

LOCAL_CHECK_FAST = BLOCKED — WSL /bin/bash unavailable
LOCAL_CHECK = BLOCKED — WSL /bin/bash unavailable
LOCAL_CHECK_ALL = BLOCKED — WSL /bin/bash unavailable
LOCAL_ARCHIVE_CHECK = BLOCKED — WSL /bin/bash unavailable

M2_H_ADAPTER_SCENARIOS = NOT_RUN — MTGML_M2_ADAPTER_BIN unavailable; not required for the H-owned proof obligations

HOSTED_CI = NOT_RUN at EVIDENCE_INPUT_HEAD; required after the single PR is opened

RUST_API_CHANGE = YES — conformance-only typed oracle/budget signatures
PYTHON_API_CHANGE = NO — tests only
TESTKIT_API_CHANGE = YES — feature-gated FixtureTransition transactionality/stream selection

FROZEN_PUBLIC_API_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
CHECKPOINT_SCHEMA_VERSION_CHANGE = NO
REPLAY_VERSION_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO
HISTORICAL_CHECKPOINT_MEANING_CHANGE = NO

INFORMATION_SAFETY = PASS
DETERMINISM = PASS
REPLAY_COMPATIBILITY = PASS
DECISION_SOUNDNESS = PASS
DECISION_COMPLETENESS = PASS

CAPABILITY_REGISTRY_CHANGE = NO
COMMANDER_SUPPORT_CLAIM = NO
NEW_MAGIC_SEMANTICS = NO

REMAINING_PRE_FREEZE_BLOCKERS = FND-028 freeze blocker; independent final plan approval is NOT_RUN after REQUEST_CHANGES corrections; independent exact-head review and Hosted CI remain pending

WORKTREE_CLEAN = YES at EVIDENCE_INPUT_HEAD
REMOTE_HEAD_EQUALS_LOCAL = NOT_RUN

M3_STARTED = NO
M3_AUTHORIZED = NO
FOUNDATION_READY_FOR_M3 = NO
MERGE_PERFORMED = NO
```

## Delivery status at evidence creation

The source/test evidence is complete at the recorded verification head, but
the governance plan gate and hosted exact-head review are still open. The
next authorized action is one exact-head review/PR flow; no merge, M3 start,
Batch I, or foundation-freeze claim is made here.
