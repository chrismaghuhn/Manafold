# Pre-M3 Remediation Batch H: Conformance Evidence Implementation Plan

**Status:** proposed for independent plan review; production implementation remains unauthorized

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task with review checkpoints.

**Goal:** Close EVD-001 through EVD-014 and FND-016B with executable, non-vacuous conformance evidence while preserving the current M2 contracts and leaving FND-026B/FND-028 explicitly classified.

**Architecture:** Keep Rust state, rules, environment, replay, RNG, and player projection authoritative. Strengthen only the existing persistence tests, RNG tests, feature-gated fixture support, and conformance harness. Use a typed complete-state witness relation, a pre-state-derived rejection oracle, revision-bound fingerprints, and bounded independent legal-space/reference checks. No player delivery API, semantic action key, replay/checkpoint version, schema, digest domain, or Magic capability is added.

**Tech Stack:** Rust 1.85.1, Cargo with the committed lockfile, the feature-gated mtgml-rules conformance fixture support, Python 3.13 from .venv, canonical CBOR/JSON fixtures, and the existing direct verification profiles.

---

## 0. Plan gate and exact source identities

**Files:**

- Read: docs/superpowers/specs/2026-09-15-pre-m3-remediation-batch-h-conformance-evidence-design.md
- Read: docs/DECISION_PROTOCOL.md
- Read: docs/INFORMATION_MODEL.md
- Read: docs/ML_ENVIRONMENT.md
- Read: docs/REPLAY_AND_DETERMINISM.md
- Read: docs/STATE_HASHING.md
- Read: docs/testing/CONFORMANCE_AUTHORING.md
- Read: docs/testing/NONINTERFERENCE_TESTING.md
- Read: docs/contracts/ACCEPTANCE_GATES.md
- Create: docs/superpowers/plans/2026-09-15-pre-m3-remediation-batch-h-conformance-evidence.md
- Modify: docs/normative-document-register.v1.json

- [ ] **Step 1: Verify the plan input identity and clean scope.**

Run:

~~~powershell
git rev-parse HEAD
git rev-parse origin/master
git status --porcelain=v1
git diff --name-only ff37f0896cdbb8e2faea424859faf128155b4579..HEAD
~~~

Expected:

~~~text
HEAD = d70f3b743cb27b4669610ce44f1e033cdd536e59
origin/master = ff37f0896cdbb8e2faea424859faf128155b4579
status = clean
changed files = only the Batch-H design and its register entry
~~~

The design was independently approved at exact head
e6fb5d8d164bc0a4d3a01c4a52dfd0840e5123c7. The design gate is:

~~~text
BATCH_H_DESIGN_REVIEW = APPROVE
H_SPLIT_DECISION = NO
~~~

- [ ] **Step 2: Register the plan without changing executable contracts.**

Add one document entry with:

~~~json
{
  "change_process": "process-pr",
  "owner_role": "maintainer",
  "path": "docs/superpowers/plans/2026-09-15-pre-m3-remediation-batch-h-conformance-evidence.md",
  "role": "process",
  "stability": "provisional"
}
~~~

Run:

~~~powershell
.venv\Scripts\python.exe scripts/check_documentation.py
git diff --check
~~~

Expected: both commands exit 0, and the plan path exists in the register.

- [ ] **Step 3: Commit the reviewed implementation plan.**

Run:

~~~powershell
git add -- docs/superpowers/plans/2026-09-15-pre-m3-remediation-batch-h-conformance-evidence.md docs/normative-document-register.v1.json
git commit -m "docs: add Batch-H conformance implementation plan"
git rev-parse HEAD
git status --porcelain=v1
~~~

Record the resulting plan commit as PLAN_HEAD. Do not edit source/test/tooling
files before BATCH_H_PLAN_REVIEW = APPROVE and
PRODUCTION_IMPLEMENTATION_AUTHORIZED = YES are recorded by independent review.

## File ownership map

The code/test changes below are the only expected executable owners:

| File | Responsibility |
|---|---|
| crates/mtgml-persistence/src/tests.rs | Rust positive persistence corpus and exact CBOR/envelope boundaries |
| python/tests/test_persistence_codec.py | Python boundary parity over the same persistence contract |
| crates/mtgml-random/src/sampling.rs | Direct production rejection-sampling KAT; remove disconnected stub |
| crates/mtgml-conformance/src/legal_space/mod.rs | Shared bounded legal-space budget |
| crates/mtgml-conformance/src/legal_space/comparator.rs | Empty-path and exact trace/request comparison |
| crates/mtgml-conformance/src/legal_space/explorer.rs | Finite complement probes, typed budgets, rejected-probe nonmutation |
| crates/mtgml-conformance/src/legal_space/oracle.rs | Validated independent reference automaton and bounded enumeration |
| crates/mtgml-conformance/src/legal_space/gate_evidence.rs | Live legal-space/complement/invariance evidence |
| crates/mtgml-conformance/src/isolation/state_relation.rs | Complete typed authorized-difference normalization, if extraction is used |
| crates/mtgml-conformance/src/isolation/witnesses.rs | Witness API and non-vacuity integration |
| crates/mtgml-conformance/src/isolation/mod.rs | New relation module and closed harness error variants |
| crates/mtgml-conformance/src/isolation/fingerprint.rs | Revision-bound complete fingerprints and non-secret manifest identity |
| crates/mtgml-conformance/src/isolation/paired.rs | Accepted-entry helper and exact progression assertions |
| crates/mtgml-conformance/src/isolation/paired_matrix.rs | Accepted witness usage and explicit post-transition proof |
| crates/mtgml-conformance/src/isolation/checkpoint_parity.rs | Independent checkpoint restore/fork expectations and source nonmutation |
| crates/mtgml-conformance/src/isolation/fork_parity.rs | Exact accepted progress and fork isolation |
| crates/mtgml-conformance/src/isolation/endpoint_pair.rs | Twin accepted-transition proof |
| crates/mtgml-conformance/src/isolation/rejection.rs | Independent pre-state-derived complete rejected PlayerStep matrix |
| crates/mtgml-conformance/src/isolation/mutants.rs | Structural-vs-controlled mutant evidence and prerequisite assertions |
| crates/mtgml-environment/src/replay_parity_tests.rs | Eventful endpoint submit and replay reprojection |
| crates/mtgml-rules/src/fixture_support.rs | Transactional feature-gated fixture operations and explicit RNG stream |
| crates/mtgml-conformance/src/lifecycle.rs | Physical/opaque lifecycle chain selection evidence |
| docs/superpowers/specs/2026-09-15-pre-m3-remediation-batch-h-dispositions-and-evidence.md | Final evidence matrix, after all source verification |

No other affected file may be changed without stopping and recording
DISCOVERED_AFFECTED_FILE, WHY_REQUIRED, PRODUCTION_OR_TEST_ONLY, and
SEMANTIC_SCOPE_CHANGE for independent scope approval.

## Task 1: Close EVD-001 persistence corpus and exact boundaries

**Files:**

- Modify: crates/mtgml-persistence/src/tests.rs
- Modify: python/tests/test_persistence_codec.py

- [ ] **Step 1: Add the Rust positive-manifest consumer.**

Add a test named
persisted_positive_fixture_manifest_matches_rust_bytes_and_meaning. It must:

~~~rust
#[test]
fn persisted_positive_fixture_manifest_matches_rust_bytes_and_meaning() {
    // Read persistence/golden/manifest.json and every listed raw file.
    // Decode through the Rust owner, re-encode, and compare exact bytes.
    // For canonical-array.cbor expect ["input.v1", 7].
    // For digest-envelope-test.cbor expect the envelope reference,
    // payload ["input.v1", 7], the manifest SHA-256, and exact re-encoding.
}
~~~

The real implementation must parse the checked-in manifest rather than
hard-code a list of files. A match arm for an unknown committed fixture must
fail the test. The expected decoded values are independent semantic
expectations; the actual decoder result is never used to build the expected
value.

Run:

~~~powershell
cargo test -p mtgml-persistence --locked persisted_positive_fixture_manifest_matches_rust_bytes_and_meaning
~~~

Expected characterization result: PASS after the new test is added; this is
evidence-only work over the current valid bytes and has no production RED.

- [ ] **Step 2: Add exact Rust CBOR boundary helpers and assertions.**

Add the test named
cbor_resource_boundaries_are_exact_at_each_declared_boundary. It must assert
boundary-1, boundary, and boundary+1 behavior for:

~~~rust
MAX_PAYLOAD_BYTES
MAX_TEXT_BYTES
MAX_ARRAY_ELEMENTS
MAX_DEPTH
MAX_ITEMS
MAX_BYTE_STRING_BYTES
envelope::MAX_IDENTIFIER_BYTES
~~~

Use direct canonical byte builders with checked lengths. For MAX_ITEMS use a
root array containing four child arrays whose leaf counts are
1_048_576, 1_048_576, 1_048_576, and 1_048_571 at the exact boundary; vary only
the last count for boundary-1 and boundary+1. For MAX_BYTE_STRING_BYTES,
exercise the private Decoder boundary inside the mtgml-persistence unit-test
module and separately document that the public top-level payload limit
dominates a standalone 64 MiB byte string. Do not allocate a value from an
untrusted header before the checked bound.

Assert the exact closed errors:

~~~text
payload: max-1 OK, max OK, max+1 PayloadTooLarge
text: max-1 OK, max OK, max+1 StringTooLarge
array: max-1 OK, max OK, max+1 ArrayTooLarge
depth: MAX_DEPTH-1 OK, MAX_DEPTH OK, MAX_DEPTH+1 DepthExceeded
items: max-1 OK, max OK, max+1 ItemLimitExceeded
byte string: max-1 OK, max OK, max+1 PayloadTooLarge
identifier: max-1 OK, max OK, max+1 EnvelopeIdentity
~~~

Run:

~~~powershell
cargo test -p mtgml-persistence --locked cbor_resource_boundaries_are_exact_at_each_declared_boundary
~~~

Expected: PASS with at least one test executed. Existing Batch-G precedence
tests remain unchanged; do not duplicate their compound-defect fixtures.

- [ ] **Step 3: Add Python boundary parity over the same constants.**

Add
test_persistence_resource_boundaries_match_rust_contract to
python/tests/test_persistence_codec.py. It must use the pinned Python
constants, build the same valid/over-limit forms, and assert the same error
codes. Keep the existing golden manifest loop and do not add a second Python
codec.

Run:

~~~powershell
.venv\Scripts\python.exe -B -m pytest python/tests/test_persistence_codec.py -q
~~~

Expected: PASS with the complete persistence test module executed.

- [ ] **Step 4: Commit the EVD-001 evidence.**

~~~powershell
git add -- crates/mtgml-persistence/src/tests.rs python/tests/test_persistence_codec.py
git commit -m "test: close Batch-H persistence corpus evidence"
~~~

Valid persistence bytes must remain byte-identical.

## Task 2: Close EVD-002 with the production rejection sampler

**Files:**

- Modify: crates/mtgml-random/src/sampling.rs

- [ ] **Step 1: Add a direct production-path rejection KAT.**

Add the test named
production_sampler_consumes_rejected_words_and_advances_the_cursor. It must
call the existing production function uniform_below_u64 with:

~~~rust
let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
let key = global_key();
let cursor = RandomStreamCursorV1::default();
let bound = (1u64 << 63) + 1;
let (value, consumed, next) =
    uniform_below_u64(&seed, &key, &cursor, bound).unwrap();
~~~

Use the existing raw-word KAT values to establish independently that raw words
0 through 3 are below the threshold and raw word 4 is accepted. Assert:

~~~text
consumed = 5
next.next_raw_u64 = 5
value = raw_word_4 % bound
~~~

This test must not call a stub, duplicate the rejection loop, or use a second
RNG algorithm.

Run:

~~~powershell
cargo test -p mtgml-random --locked production_sampler_consumes_rejected_words_and_advances_the_cursor
~~~

Expected: PASS on the current production sampler; the old
forced_rejection_stub is still present only until the next step.

- [ ] **Step 2: Remove the disconnected stub and its test.**

Delete forced_rejection_stub and uniform_below_u64_stub. Keep all valid
sampling KATs and shuffle tests unchanged.

Run:

~~~powershell
cargo test -p mtgml-random --locked
~~~

Expected: PASS with the production rejection KAT executed and no stub
remaining.

- [ ] **Step 3: Commit the RNG evidence.**

~~~powershell
git add -- crates/mtgml-random/src/sampling.rs
git commit -m "test: prove production RNG rejection sampling"
~~~

RNG algorithm, threshold, cursor semantics, HMAC, stream derivation, shuffle,
and existing KAT outputs must remain unchanged.

## Task 3: Harden legal-space completeness, complements, and oracle bounds

**Files:**

- Modify: crates/mtgml-conformance/src/legal_space/mod.rs
- Modify: crates/mtgml-conformance/src/legal_space/comparator.rs
- Modify: crates/mtgml-conformance/src/legal_space/explorer.rs
- Modify: crates/mtgml-conformance/src/legal_space/oracle.rs
- Modify: crates/mtgml-conformance/src/legal_space/gate_evidence.rs

- [ ] **Step 1: Add the empty-path RED regression for EVD-007.**

Add test empty_production_path_is_missing_choice in the completeness test
module. It must take one existing reference choice, insert it into
production.complete_paths with Vec::new(), call completeness_defects, and
require MissingChoice for that choice.

Run:

~~~powershell
cargo test -p mtgml-conformance --locked empty_production_path_is_missing_choice
~~~

Expected RED reason on the unmodified base: completeness_defects returns no
defect for Some(empty_vec). This is the named empty-production-path evidence
failure, not a compile error or unrelated validator.

- [ ] **Step 2: Make empty paths fail closed.**

Change the comparator branch to:

~~~rust
match production.complete_paths.get(choice) {
    None => missing_choice(choice),
    Some(paths) if paths.is_empty() => missing_choice(choice),
    Some(paths) if paths.len() > 1 => duplicate_path(choice, paths.len()),
    Some(_) => {}
}
~~~

Use the existing typed SpaceDefect::MissingChoice variant. Run the RED command
again and then:

~~~powershell
cargo test -p mtgml-conformance --locked completeness
~~~

Expected: the new test and all existing completeness tests PASS.

- [ ] **Step 3: Move ExplorerBudget to the shared conformance budget owner.**

Define one LegalSpaceBudget in legal_space/mod.rs with exactly:

~~~rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegalSpaceBudget {
    pub max_candidates_per_request: u32,
    pub max_numeric_span: u64,
    pub max_depth: u32,
    pub max_total_nodes: u32,
    pub max_generated_answers: u64,
}
~~~

Implement the existing default values once. Re-export it from explorer.rs as
ExplorerBudget so existing callers remain readable. The reference automaton
and production explorer must receive the same values, while their errors
remain separate typed enums.

- [ ] **Step 4: Add finite complement probes and prove their advertised status.**

Extend is_advertised so it checks domain, candidate membership, uniqueness,
and canonical SelectMany order. Extend generate_probes with these finite
complements:

~~~text
ChooseOne: unknown candidate and wrong SelectMany variant
ChooseMany: every bounded subset, duplicate, unknown, overlong, reversed
            two-member answer when available, and wrong ChooseNumber variant
ChooseNumber: every in-range value, both checked outside-range sentinels,
              and wrong SelectOne variant
Order: every permutation for every length 0..=candidate_count+1,
       including below-minimum and above-maximum lengths, using at most one
       fresh unknown ID, plus one duplicate and one wrong-number variant
~~~

Count the complete finite set before allocation. If the set exceeds
max_generated_answers, return GeneratedAnswersExceeded. If an unknown ID
cannot be represented because the request already uses u32::MAX, omit only
that syntactically impossible extension and keep the typed bounded result.

Add test generate_probes_contains_the_bounded_invalid_complement. On the base,
use a small ChooseMany and Order request and assert the new unknown,
duplicate, reversed, wrong-variant, below-minimum, and above-maximum shapes.
This is a source/test characterization if the first assertions need to be
split; every RED command must fail because the named complement is absent.

Run:

~~~powershell
cargo test -p mtgml-conformance --locked generate_probes_contains_the_bounded_invalid_complement
~~~

Implement the minimal probe grammar and rerun the same command. Expected:
PASS with a nonzero complement count.

- [ ] **Step 5: Add rejected-probe branch nonmutation.**

In walk, capture branch.checkpoint() immediately before each submit. For a
Rejected step, capture branch.checkpoint() immediately after submit and return
a new typed ExplorationFailure::RejectedMutation if the checkpoints differ.
Do not use the source controller as the mutable branch and do not expose
trusted details in the error.

Add test live_complement_probes_are_rejected_without_branch_mutation. It must
run explore over the real endpoint and assert:

~~~text
advertised_rejected = []
out_of_contract_accepted = 0
out_of_contract_rejected > 0
~~~

Run:

~~~powershell
cargo test -p mtgml-conformance --locked live_complement_probes_are_rejected_without_branch_mutation
~~~

Expected: PASS and at least one invalid probe actually executes.

- [ ] **Step 6: Validate reference specs and transitions.**

Add typed ReferenceSpecError and ReferenceTransitionError. Make
ReferenceAssemblySpec validation reject duplicate or unsupported piece atoms.
Change ReferenceAutomaton::new and initial to return Result. Change advance to
return Result and reject any choice not in reference_choices instead of
silently retaining the old state. Make declaration order canonical in
expected_request by sorting the atom vector before returning it.

The RED characterization must be a test assertion, not a compile-error RED:
add reference_source_rejects_silent_invalid_advance that checks the
pre-change source signature/implementation boundary, then add the behavioral
test reference_advance_rejects_invalid_choice after the typed API exists.

Run the source RED before the implementation:

~~~powershell
cargo test -p mtgml-conformance --locked reference_source_rejects_silent_invalid_advance
~~~

Expected RED reason: the current oracle has a silent advance branch. After
the implementation, run:

~~~powershell
cargo test -p mtgml-conformance --locked reference_advance_rejects_invalid_choice
~~~

Expected: PASS with a typed ReferenceTransitionError and no state change.

- [ ] **Step 7: Bound reference enumeration and trace comparison.**

Change reference enumeration to consume LegalSpaceBudget:

~~~text
max_depth = 4
max_total_nodes = 64
max_generated_answers = 256
~~~

Count every explored reference node and generated choice with checked
arithmetic. Return typed bound errors. Require the complete reference path to
have exactly four stages: Anchor, Number, Members, Order.

Extend SpaceDefect with a safe TraceLengthMismatch and
ReferenceTransitionRejected defect. Make request_sequence_defects reject:

~~~text
observed_requests.len() != stages.len()
stages.len() != 4 for a complete path
any expected request after ReferenceAssemblyState::Complete
any invalid reference advance
~~~

Add:

~~~rust
#[test]
fn request_trace_rejects_extra_observed_request() { /* exact length defect */ }

#[test]
fn reversed_reference_declaration_keeps_expected_request_canonical() {
    /* [2, 1, 0] and [0, 1, 2] produce equal expected requests */
}
~~~

Run:

~~~powershell
cargo test -p mtgml-conformance --locked request_trace_rejects_extra_observed_request
cargo test -p mtgml-conformance --locked reversed_reference_declaration_keeps_expected_request_canonical
cargo test -p mtgml-conformance --locked legal_space
~~~

Expected: all commands PASS with tests executed and no silently truncated
reference/production result.

- [ ] **Step 8: Run the complete legal-space package and commit.**

~~~powershell
cargo test -p mtgml-conformance --all-features --locked
git add -- crates/mtgml-conformance/src/legal_space
git commit -m "test: harden Batch-H legal-space evidence"
~~~

Do not add new Magic semantics, an action-key contract, or a production import
of the conformance oracle.

## Task 4: Make fixture support transactional and lifecycle chain-correct

**Files:**

- Modify: crates/mtgml-rules/src/fixture_support.rs
- Modify: crates/mtgml-conformance/src/lifecycle.rs
- Modify: crates/mtgml-conformance/src/isolation/checkpoint_parity.rs

- [ ] **Step 1: Add lifecycle reidentification RED evidence.**

Extend the existing test
reidentification_of_a_randomized_card_uses_fresh_opaque_and_keeps_old_retired
before changing the helper. Capture the set of physical cards stored in P1's
retired records immediately before scenario_reidentification and assert that
the returned physical card belongs to that set.

Run:

~~~powershell
cargo test -p mtgml-conformance --all-features --locked reidentification_of_a_randomized_card_uses_fresh_opaque_and_keeps_old_retired
~~~

Expected RED reason on the current base: the helper selects the first hidden
object by GameObjectId and can choose the initial P2 hidden card, whose
physical card is not in the retired P1 chain.

- [ ] **Step 2: Select the target through the retired physical chain.**

In scenario_reidentification:

~~~rust
let (retired_opaque, retired_record) = before.knowledge.players[&P1]
    .retired
    .iter()
    .find(|(_, record)| record.physical_card.is_some())
    .ok_or_else(|| contract("retired physical chain is empty"))?;
let expected_physical = retired_record.physical_card;
let target = before.zones.objects.iter()
    .find(|(_, object)| {
        object.physical_card == expected_physical
            && before.zones.locations.get(object.0) == Some(&hidden_hand(P2))
    })
    .map(|(object, _)| *object)
    .ok_or_else(|| contract("retired physical chain is not in the hidden set"))?;
~~~

Keep the retired opaque ID in the fixture assertion path, assert the target's
old incarnation is hidden, and bind the new acquisition to the target's actual
new incarnation. Never copy the chosen target's physical identity into the
expected set after selection.

Run the RED command again. Expected: PASS and explicit old opaque/physical/new
incarnation evidence.

- [ ] **Step 3: Add transactional RED tests inside FixtureTransition.**

Add unit tests under the feature-gated fixture_support module:

~~~rust
#[test]
fn move_object_rolls_back_when_event_binding_fails() { /* RuleEventId::MAX */ }

#[test]
fn occurrence_rolls_back_when_event_binding_fails() { /* valid occurrence */ }

#[test]
fn random_sample_rolls_back_cursor_when_event_binding_fails() { /* MAX id */ }

#[test]
fn random_sample_requires_the_declared_global_stream() { /* only player stream */ }
~~~

Each test snapshots workspace, events, and offset after start, triggers the
late failure, and asserts exact equality after Err. The random test asserts
that a player-scoped stream is not silently selected.

Run:

~~~powershell
cargo test -p mtgml-rules --all-features --locked fixture_support
~~~

Expected RED reason on the current base: late event binding leaves workspace
or RNG mutation behind, and a player stream can be selected by keys().next().

- [ ] **Step 4: Implement clone/commit-or-rollback wrappers.**

Wrap move_object_incarnation, apply_occurrence, and
record_hidden_random_sample in one private transaction helper:

~~~rust
fn transaction<T>(
    &mut self,
    action: impl FnOnce(&mut Self) -> Result<T, KernelExecutionError>,
) -> Result<T, KernelExecutionError> {
    let workspace = self.workspace.clone();
    let events = self.events.clone();
    let offset = self.offset;
    match action(self) {
        Ok(value) => Ok(value),
        Err(error) => {
            self.workspace = workspace;
            self.events = events;
            self.offset = offset;
            Err(error)
        }
    }
}
~~~

Put each former body in an inner operation so no public helper has a second
partial-mutation path. Resolve exactly
RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1), use lookup_stream,
and map the checked random error into KernelExecutionError. Remove the
checkpoint helper's panic conversion; it returns HarnessError::FixtureTransitionRejected.

Run:

~~~powershell
cargo test -p mtgml-rules --all-features --locked fixture_support
cargo test -p mtgml-conformance --all-features --locked lifecycle
~~~

Expected: all transactional and lifecycle tests PASS.

- [ ] **Step 5: Commit the fixture seam before dependent evidence.**

~~~powershell
git add -- crates/mtgml-rules/src/fixture_support.rs crates/mtgml-conformance/src/lifecycle.rs crates/mtgml-conformance/src/isolation/checkpoint_parity.rs
git commit -m "fix: make Batch-H fixture helpers transactional"
~~~

This is a feature-gated TESTKIT_API_CHANGE only. Record it truthfully later;
it is not a public player or replay API change.

## Task 5: Enforce the complete EVD-004 authorized-difference relation

**Files:**

- Create: crates/mtgml-conformance/src/isolation/state_relation.rs
- Modify: crates/mtgml-conformance/src/isolation/witnesses.rs
- Modify: crates/mtgml-conformance/src/isolation/mod.rs

- [ ] **Step 1: Add the contaminated-pair RED test.**

Add contaminated_object_rename_plus_life_change_is_rejected. Build the existing
valid ObjectRenaming pair, then change one core player life value on side B
without changing the declared object bijection. Keep both states valid and
assert:

~~~rust
assert!(matches!(
    assert_witness(&state_a, &state_b, &witness),
    Err(WitnessViolation::UnauthorizedStateDifference)
));
~~~

Run:

~~~powershell
cargo test -p mtgml-conformance --locked contaminated_object_rename_plus_life_change_is_rejected
~~~

Expected RED reason on the base: assert_witness checks selected perspective
relations but accepts the unrelated core difference.

- [ ] **Step 2: Implement typed normalized state comparison.**

Create state_relation.rs with:

~~~rust
pub(crate) fn assert_only_authorized_difference(
    perspective: PlayerId,
    a: &EngineState,
    b: &EngineState,
    predicate: NonVacuityPredicate,
    bijection: Option<&TrustedRenamingBijection>,
) -> Result<(), StateRelationViolation>;
~~~

Clone both states, normalize only the declared fields, and require exact
EngineState equality after normalization. The normalizer must implement these
allowlists:

~~~text
OpponentHiddenDefinition:
  face-down object card_definition and matching foreign knowledge marker
HiddenConcealedOrdering:
  P2 face-down-library ordered vector, matching ZonePosition values, and
  matching foreign known-location positions
ForeignPrivateLook:
  one foreign active-record key only
FaceDownIdentity:
  face-down physical assignments and matching foreign retained physical fields
RootSeedPreAuth:
  RandomState.root_seed only
HiddenRngCursor:
  global SyntheticM1 cursor only
ObjectRenaming:
  declared object map in zones, ordered zones, stack/pending trusted refs,
  forward identity targets, reverse identity keys, and next_object_id;
  opaque keys stay identical and no identity is added/dropped
AbilityRenaming:
  declared ability map in stack/pending ActivateAbility refs, forward identity
  targets, reverse identity keys, and next_ability_id; opaque keys stay equal
GlobalAllocatorHistory:
  global IdentityAllocatorState only
ForeignKnowledgeHistory:
  one foreign active record's known_location and historical_locations only
~~~

For object/ability renaming, normalize B through the inverse declared
bijection. Reject collisions instead of overwriting map entries. Update
pending trusted EngineCandidateBinding references for CastSpell,
SelectObject, and ActivateAbility as applicable. Keep all unlisted fields
untouched; an equality failure becomes the closed
UnauthorizedStateDifference variant without rendering state values.

- [ ] **Step 3: Make non-vacuity predicates exact.**

Require:

~~~text
definition axis = exactly one changed face-down object plus its mapped foreign record
ordering axis = same member multiset and a changed permitted vector/position set
private-look axis = exactly one foreign active-key membership difference
physical axis = changed face-down assignment with equal physical multiset
history axis = exactly one foreign record's allowed history fields differ
renaming axis = at least one declared mapping target differs
seed/cursor/allocator axes = the named trusted field differs
~~~

Call the new relation from assert_witness after existing scoped
knowledge/decision/bijection checks and before the non-vacuity result. Keep
NonVacuityPredicate::Required compatibility for existing generic unit tests,
but all ten paired matrix axes must use the explicit axis predicates.

Run:

~~~powershell
cargo test -p mtgml-conformance --all-features --locked witnesses
cargo test -p mtgml-conformance --all-features --locked paired_matrix
~~~

Expected: the contamination RED turns green, all ten valid axes still pass,
and the existing vacuity/renaming/bijection controls still execute.

- [ ] **Step 4: Commit the witness relation.**

~~~powershell
git add -- crates/mtgml-conformance/src/isolation/state_relation.rs crates/mtgml-conformance/src/isolation/witnesses.rs crates/mtgml-conformance/src/isolation/mod.rs
git commit -m "test: enforce exact Batch-H witness differences"
~~~

The new relation remains conformance-only and cannot become semantic
authority.

## Task 6: Make EVD-012 fingerprints revision-bound and manifest-complete

**Files:**

- Modify: crates/mtgml-conformance/src/isolation/fingerprint.rs
- Modify: crates/mtgml-conformance/src/isolation/mod.rs

- [ ] **Step 1: Add identity-surface assertions before changing capture.**

Add:

~~~rust
#[test]
fn manifest_identity_is_part_of_the_environment_fingerprint() { /* mutate
    engine_build or schema identity and require EnvironmentGroupMismatch */ }

#[test]
fn digest_reference_surfaces_preserve_the_declared_domains() { /* assert
    full-state and checkpoint envelope/schema/domain/codec fields */ }
~~~

These tests may be characterization assertions over the new types. They must
not print digest bytes, seeds, stream keys, or authoritative state.

- [ ] **Step 2: Add complete non-secret manifest and digest-reference fields.**

Extend TrustedEnvironmentIdentitySurface with:

~~~text
ReplayManifestV3.schema_version
engine_build
kernel.implementation_id, semantic_version, build_profile
rules_snapshot
format_policy_snapshot
oracle_snapshot
card_bundle
randomness.contract_id
all ReplaySchemaVersionsV1 fields
all DeckIdentityV1 player/deck_id/digest values
ReplayManifestV3.initial_identity
current FullStateDigestV3 DigestReferenceV1 tuple
current CheckpointDigestV3 DigestReferenceV1 tuple
~~~

Build the checkpoint digest reference with
environment-checkpoint-digest-input.v3 and the existing
mtgml.digest-envelope.v1/sha-256/mtgml.canonical-cbor.v1 identities. Do not
store or render randomness.root_seed_hex in this default diagnostic surface.

- [ ] **Step 3: Enforce revision-bound capture.**

In capture_snapshot, require observation, information_state, and visible
decision (when present) to have the same perspective and revision. In
capture_complete:

~~~text
checkpoint_before = controller.checkpoint()
replay = controller.export_replay()
capture P1 and P2 snapshots
require every endpoint product revision == checkpoint_before.state.revision
require replay.final_identity == checkpoint_before current identity
checkpoint_after = controller.checkpoint()
require checkpoint_after == checkpoint_before
~~~

Return a closed IncoherentFingerprintCapture/HarnessError on any drift.
FingerprintComparison::All compares the captured segment anchor. The
ExcludeReplayRecorder policy retains the anchor and compares the immutable
manifest/digest identity, but omits the anchor's mutable
revision/digest/status/counter values because fork/restore intentionally
rebase a segment. Existing assert_segment_anchor calls remain the explicit
anchor proof.

Run:

~~~powershell
cargo test -p mtgml-conformance --all-features --locked fingerprint
~~~

Expected: all fingerprint tests PASS and existing fork/restore comparisons
remain valid under the clarified policy.

- [ ] **Step 4: Commit fingerprint evidence.**

~~~powershell
git add -- crates/mtgml-conformance/src/isolation/fingerprint.rs crates/mtgml-conformance/src/isolation/mod.rs
git commit -m "test: bind Batch-H fingerprints to one trusted instant"
~~~

Confirm with a source scan that no new default fingerprint field contains a
root seed, raw RNG key/cursor, physical card ID, GameObjectId, or checkpoint
payload.

## Task 7: Add independent accepted-transition/checkpoint/fork proof

**Files:**

- Modify: crates/mtgml-conformance/src/isolation/paired.rs
- Modify: crates/mtgml-conformance/src/isolation/paired_matrix.rs
- Modify: crates/mtgml-conformance/src/isolation/checkpoint_parity.rs
- Modify: crates/mtgml-conformance/src/isolation/fork_parity.rs
- Modify: crates/mtgml-conformance/src/isolation/endpoint_pair.rs

- [ ] **Step 1: Strengthen the shared accepted-entry helper.**

Make accepted_entry_submission reject any returned non-Accepted step with a
closed HarnessError::AcceptedTransitionRequired. Capture the pre-request
information state and require the returned step revision to be exactly
pre_revision + 1. Do not derive expected acceptance from the returned step.

- [ ] **Step 2: Add exact entry progress assertions.**

Add test-support helper
assert_accepted_entry_progression(before, after, step). It must assert:

~~~text
step.submission = Accepted
after.revision = before.revision + 1
decisions_submitted_after = before + 1
accepted_transitions_after = before + 1
rule_events_emitted_after = before + 5
resource/wall counters unchanged
actor life 40 -> 38
global SyntheticM1 cursor advanced
pending decision changed ChooseOne -> ChooseNumber { 0, 3 } with no candidates
ChooseCount continuation exists with the same continuation identity
step information/status/next_decision equals the independent after expectation
~~~

Use this helper from paired_matrix's accepted transition evidence and
endpoint_pair::accepted_determinism_twins. Run:

~~~powershell
cargo test -p mtgml-conformance --all-features --locked accepted_determinism_twins
cargo test -p mtgml-conformance --all-features --locked axis_07a_object_renaming_byte_equality
~~~

Expected: PASS with acceptance and exact semantic mutation established before
byte parity.

- [ ] **Step 3: Add exact count-stage checkpoint/fork progress.**

In restore_decision_rich and fork_decision_rich, for the selected count 2
assert:

~~~text
revision +1
decisions_submitted +1
accepted_transitions +1
rule_events_emitted +2
resource/wall counters unchanged
continuation payload stage ChooseMembers, selected_count = 2
next decision is ChooseMany { minimum: 2, maximum: 2 } with candidate atoms 0,1
returned step is Accepted and carries the same revision/decision
~~~

Keep exact source checkpoint copies and assert source checkpoint and replay
are unchanged after every fork-side accepted, rejected, and restore mutation.
Use direct checkpoint equality in addition to the complete fingerprint.

Run:

~~~powershell
cargo test -p mtgml-conformance --all-features --locked checkpoint_parity
cargo test -p mtgml-conformance --all-features --locked fork_parity
~~~

Expected: PASS, with source nonmutation and fork independence explicit.

- [ ] **Step 4: Commit EVD-003/EVD-006 proof.**

~~~powershell
git add -- crates/mtgml-conformance/src/isolation/paired.rs crates/mtgml-conformance/src/isolation/paired_matrix.rs crates/mtgml-conformance/src/isolation/checkpoint_parity.rs crates/mtgml-conformance/src/isolation/fork_parity.rs crates/mtgml-conformance/src/isolation/endpoint_pair.rs
git commit -m "test: prove Batch-H accepted progress and fork isolation"
~~~

No parity test may use equal outputs as its only acceptance or correctness
predicate.

## Task 8: Close EVD-005 and FND-016B with complete rejected products

**Files:**

- Modify: crates/mtgml-conformance/src/isolation/rejection.rs

- [ ] **Step 1: Add the independent pre-state product builder.**

Add a test-only function:

~~~rust
fn expected_rejected_step(
    pre_information: PlayerInformationStateV2,
    pre_decision: Option<PlayerDecisionRequestV2>,
    pre_status: EpisodeStatus,
    expected_code: PlayerSubmissionCodeV1,
) -> PlayerStepV2 {
    let next_decision = match expected_code {
        PlayerSubmissionCodeV1::StaleDecision
        | PlayerSubmissionCodeV1::InvalidAnswer
        | PlayerSubmissionCodeV1::InvalidCandidate
        | PlayerSubmissionCodeV1::DuplicateAssignment
        | PlayerSubmissionCodeV1::InvalidCardinality
        | PlayerSubmissionCodeV1::InvalidNumber
        | PlayerSubmissionCodeV1::InvalidOrder => pre_decision,
        PlayerSubmissionCodeV1::UnavailableDecision
        | PlayerSubmissionCodeV1::EpisodeClosed => None,
    };
    PlayerStepV2 {
        schema_version: PLAYER_STEP_SCHEMA_V2.into(),
        information_state: pre_information,
        observed_events: Vec::new(),
        next_decision,
        status: pre_status,
        submission: PlayerStepSubmissionV1::Rejected { code: expected_code },
    }
}
~~~

The code enum comes from the independent SEMANTIC_CASES expected_code table,
not from the actual returned step. The information state and visible decision
are captured before submit.

- [ ] **Step 2: Replace code-only rejection assertions with full parity.**

Add test
semantic_matrix_returns_the_independent_complete_rejected_product. For every
SEMANTIC_CASES row:

~~~text
capture pre checkpoint
capture pre information state and pre visible decision
derive expected code from the declared row
construct expected PlayerStepV2 from pre-state
submit the row response through the real endpoint
assert actual PlayerStepV2 == expected PlayerStepV2
capture post checkpoint
assert revision, random, allocators/IDs, knowledge, perspective identities,
execution/history, status, counters, and replay are unchanged
assert observed_events is empty and every meaningful field is covered
~~~

Keep semantic_matrix_fingerprint_stable as a broad regression, but it is not
the sole FND-016B evidence.

Run:

~~~powershell
cargo test -p mtgml-conformance --all-features --locked semantic_matrix_returns_the_independent_complete_rejected_product
cargo test -p mtgml-conformance --all-features --locked semantic_matrix_fingerprint_stable
~~~

Expected: all 15 current rows execute and pass. If one row cannot derive a
complete expected product, stop with FND_016B = SPLIT_REQUIRED; do not weaken
the assertion.

- [ ] **Step 3: Commit rejection-product evidence.**

~~~powershell
git add -- crates/mtgml-conformance/src/isolation/rejection.rs
git commit -m "test: close Batch-H rejected PlayerStep products"
~~~

No PlayerStep schema or endpoint behavior changes are authorized.

## Task 9: Route EVD-010 through the real eventful endpoint and replay

**Files:**

- Modify: crates/mtgml-environment/src/replay_parity_tests.rs

- [ ] **Step 1: Use PlayerEndpoint::submit for the live eventful product.**

In eventful_replay_reprojects_both_perspectives_byte_exactly, bind P1 and P2,
read the live P1 request, and submit a response through P1's real endpoint.
Store the returned live_step. Do not call execute_trusted_response for the
live product and do not hand-build the live PlayerStep.

Assert before replay:

~~~text
live_replay.steps.len() = 1
live_step.submission = Accepted
live_step.observed_events.len() > 0
live_after identity matches the live replay final identity
~~~

- [ ] **Step 2: Reproject the replay trace through production code.**

Execute live_replay from cp0 with execute_replay_from_checkpoint. Assert:

~~~text
trace.transition.events.len() > 0
replay P1 observed-event batch is nonempty
replay P2 observed-event batch is nonempty
replay P1 batch == live_step.observed_events
replayed PlayerStepV2 built by the existing production step assembler plus
the replay-projected P1 batch is byte-identical to live_step
P1/P2 event batches remain distinct and contain no forbidden trusted fields
live controller checkpoint and replay are unchanged by replay execution
~~~

The P2 batch is projection evidence only; do not create a non-actor PlayerStep
or delivery mechanism.

Run:

~~~powershell
cargo test -p mtgml-environment --all-features --locked eventful_replay_reprojects_both_perspectives_byte_exactly
~~~

Expected: PASS with nonempty authoritative and observed event evidence.

- [ ] **Step 3: Commit eventful replay evidence.**

~~~powershell
git add -- crates/mtgml-environment/src/replay_parity_tests.rs
git commit -m "test: prove eventful endpoint and replay products"
~~~

FND-026B remains BLOCKED_CONTRACT_AMBIGUITY and FND-026C remains DEFERRED_P2.

## Task 10: Make EVD-013 mutant evidence honest and sensitive

**Files:**

- Modify: crates/mtgml-conformance/src/isolation/mutants.rs

- [ ] **Step 1: Separate structural negatives from controlled mutants.**

Rename tests and comments so the M1/M2 literal-channel tests are explicitly
structural guard tests:

~~~text
structural_guard_rejects_trusted_order_mutation_channel
structural_guard_rejects_binding_derived_candidate_ids
fallback_mutant_detects_trusted_difference_in_valid_carrier
~~~

Do not call a structural validator negative projector/generator sensitivity.
Retain the existing validity gate: both mutated products must canonical-decode
and validate before byte inequality counts.

- [ ] **Step 2: Assert clean transition prerequisites.**

Change detect_step to accept an independently declared expected clean
submission outcome and assert it before invoking the mutant:

~~~rust
assert_eq!(step_a.submission, expected_clean_submission);
assert_eq!(step_b.submission, expected_clean_submission);
~~~

For the M4 stale-response case, expected_clean_submission is
Rejected { code: StaleDecision }. For accepted carriers, require Accepted and
the exact entry-progress helper where the caller has the checkpoint.

Run:

~~~powershell
cargo test -p mtgml-conformance --all-features --locked mutants
~~~

Expected: PASS with a named prerequisite proving the mutant reached the
intended product component.

- [ ] **Step 3: Commit mutant evidence.**

~~~powershell
git add -- crates/mtgml-conformance/src/isolation/mutants.rs
git commit -m "test: separate Batch-H structural and mutant evidence"
~~~

No production callback or hidden mutation seam is added.

## Task 11: Cross-layer focused verification before evidence recording

**Files:**

- Read/verify all files changed in Tasks 1-10
- Modify only if a directly named test/tooling defect is found and already in scope

- [ ] **Step 1: Run exact affected package suites.**

Run the packages because the owner map uses them:

~~~powershell
cargo test -p mtgml-model --all-features --locked
cargo test -p mtgml-decision --all-features --locked
cargo test -p mtgml-state --all-features --locked
cargo test -p mtgml-random --all-features --locked
cargo test -p mtgml-persistence --all-features --locked
cargo test -p mtgml-replay --all-features --locked
cargo test -p mtgml-rules --all-features --locked
cargo test -p mtgml-observation --all-features --locked
cargo test -p mtgml-environment --all-features --locked
cargo test -p mtgml-conformance --all-features --locked
~~~

Expected: every command exits 0 and reports at least one test for every
package with tests. mtgml-card-ir and mtgml-wire are not edited by this plan;
they are covered by workspace and schema gates.

- [ ] **Step 2: Run the cross-language persistence and player evidence.**

~~~powershell
.venv\Scripts\python.exe -B -m pytest python/tests/test_persistence_codec.py python/tests/test_batch_g.py -q
.venv\Scripts\python.exe scripts/generate_contracts.py --check
.venv\Scripts\python.exe scripts/validate_schemas.py
.venv\Scripts\python.exe scripts/verify_repository.py
.venv\Scripts\python.exe scripts/check_rust_source_structure.py
.venv\Scripts\python.exe scripts/check_documentation.py
.venv\Scripts\python.exe scripts/validate_maintainer_artifacts.py
.venv\Scripts\python.exe scripts/verify_python_toolchain.py
~~~

Expected: all direct commands exit 0. The Python test result must separately
retain the three known M2.H adapter scenarios as NOT_RUN when
MTGML_M2_ADAPTER_BIN is absent.

- [ ] **Step 3: Inspect scope and safety before workspace gates.**

Run:

~~~powershell
git diff --stat ff37f0896cdbb8e2faea424859faf128155b4579..HEAD
git diff --name-status ff37f0896cdbb8e2faea424859faf128155b4579..HEAD
rg -n "root_seed|raw_words|physical_card|GameObjectId|AbilityInstanceId|checkpoint_digest|keys\(\)\.next|values\(\)\.next|panic!\(" crates/mtgml-conformance/src crates/mtgml-rules/src/fixture_support.rs
git diff --check
~~~

Review every hit. New default diagnostic paths must not print protected
values. A remaining first-map-stream or panic conversion in the Batch-H
fixture owner is a failure, not a deferred nit.

- [ ] **Step 4: Run full direct Rust and Python gates.**

~~~powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
.venv\Scripts\python.exe scripts/run_python_tests.py --profile full
.venv\Scripts\python.exe scripts/run_checks.py fast
.venv\Scripts\python.exe scripts/run_checks.py integration
.venv\Scripts\python.exe scripts/run_checks.py certification
~~~

Record exact counts and exit codes. No command may be labeled PASS from a
previous run.

- [ ] **Step 5: Attempt the repository wrappers separately.**

~~~powershell
just check-fast
just check
just check-all
just archive-check
~~~

If WSL still cannot start /bin/bash, record:

~~~text
LOCAL_CHECK_FAST = BLOCKED
LOCAL_CHECK = BLOCKED
LOCAL_CHECK_ALL = BLOCKED
LOCAL_ARCHIVE_CHECK = BLOCKED
~~~

Direct profile PASS results do not upgrade wrapper BLOCKED results.

- [ ] **Step 6: Run direct archive verification only after all source changes.**

~~~powershell
.venv\Scripts\python.exe scripts/verify_archive_reproducibility.py
~~~

Record the exact archive hash and source HEAD. Do not change source after this
gate without rerunning it.

## Task 12: Record dispositions and final evidence

**Files:**

- Create: docs/superpowers/specs/2026-09-15-pre-m3-remediation-batch-h-dispositions-and-evidence.md
- Modify: docs/normative-document-register.v1.json

- [ ] **Step 1: Capture the exact evidence input head.**

Run before creating the evidence document:

~~~powershell
git rev-parse HEAD
git status --porcelain=v1
~~~

Record this SHA as EVIDENCE_INPUT_HEAD. Record
CODE_VERIFICATION_HEAD as the last source/test/tooling commit on which the
complete applicable gates in Task 11 passed. Do not put a future final evidence
commit SHA inside the evidence document.

- [ ] **Step 2: Write the required final matrix.**

The evidence document must end with the requested fields:

~~~text
TASK = PRE_M3_REMEDIATION_BATCH_H
BASE = ff37f0896cdbb8e2faea424859faf128155b4579
HEAD = FINAL_EVIDENCE_HEAD recorded externally after the evidence commit
BRANCH = chris/pre-m3-remediation-batch-h-conformance-evidence-closure
PR = pending until the single branch is pushed
EVD_001 through EVD_014 = each actual disposition and exact evidence result
FND_016B = CLOSED
FND_026B_CURRENT_STATUS = BLOCKED_CONTRACT_AMBIGUITY
FND_026B_FREEZE_READINESS = MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = NO
FND_028_CURRENT_STATUS = BLOCKED_CONTRACT_AMBIGUITY
FND_028_FREEZE_READINESS = MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = YES; FREEZE_BLOCKER
M3_STARTED = NO
M3_AUTHORIZED = NO
FOUNDATION_READY_FOR_M3 = NO
MERGE_PERFORMED = NO
~~~

For each EVD include disposition, RED/characterization/GREEN evidence, exact
test names, and whether compatibility or artifact meaning changed. Include
the bounded 30-row integration matrix from the request, package reasons,
direct/wrapper gate results, adapter NOT_RUN status, archive result,
information-safety result, and remaining FND-028 blocker.

- [ ] **Step 3: Register and validate the evidence document.**

Add the required process/provisional/maintainer/process-pr register entry.
Run:

~~~powershell
.venv\Scripts\python.exe scripts/check_documentation.py
.venv\Scripts\python.exe scripts/validate_maintainer_artifacts.py
git diff --check
~~~

Expected: PASS with the evidence path registered and links valid.

- [ ] **Step 4: Commit the evidence record.**

~~~powershell
git add -- docs/superpowers/specs/2026-09-15-pre-m3-remediation-batch-h-dispositions-and-evidence.md docs/normative-document-register.v1.json
git commit -m "docs: record Batch-H dispositions and evidence"
git rev-parse HEAD
git status --porcelain=v1
~~~

The resulting SHA is FINAL_EVIDENCE_HEAD and is recorded externally in the
delivery report, never inserted into the just-committed evidence document.

- [ ] **Step 5: Rerun change-aware documentation/archive gates on the final evidence head.**

~~~powershell
.venv\Scripts\python.exe scripts/check_documentation.py
.venv\Scripts\python.exe scripts/validate_maintainer_artifacts.py
.venv\Scripts\python.exe scripts/verify_archive_reproducibility.py
~~~

Expected: all three commands exit 0. If the evidence document changed after
archive verification, the archive gate is NOT_RUN until repeated.

## Task 13: Push one branch, open one PR, and verify hosted CI

**Files:**

- No source changes authorized after Task 12 except corrections required by a
  failing exact-head gate, which trigger the complete rerun policy.

- [ ] **Step 1: Verify the final local identity and push the single branch.**

~~~powershell
git status --porcelain=v1
git rev-parse HEAD
git ls-remote origin refs/heads/chris/pre-m3-remediation-batch-h-conformance-evidence-closure
git push --set-upstream origin chris/pre-m3-remediation-batch-h-conformance-evidence-closure
git rev-parse HEAD
git ls-remote origin refs/heads/chris/pre-m3-remediation-batch-h-conformance-evidence-closure
~~~

If the remote branch already exists at a different SHA, stop and report the
exact conflict; do not force-push. Record REMOTE_HEAD_EQUALS_LOCAL only after
the final command confirms equality.

- [ ] **Step 2: Open one PR only after local evidence is complete.**

Use:

~~~powershell
gh pr create --base master --head chris/pre-m3-remediation-batch-h-conformance-evidence-closure --title "Pre-M3 remediation Batch H: conformance evidence and proof closure" --body-file .batch-h-pr-body.md
~~~

The temporary PR body must repeat every disposition, exact BASE/HEAD,
compatibility field, focused/workspace/direct/wrapper result, blocked adapter
status, FND-028 blocker, and the explicit statements:

~~~text
MERGE_PERFORMED = NO
FOUNDATION_READY_FOR_M3 = NO
M3_AUTHORIZED = NO
~~~

Remove the temporary body file after PR creation and confirm it is not in the
branch. Do not merge.

- [ ] **Step 3: Wait for and inspect exact-head Hosted CI.**

Verify every configured required check on the final PR head, at minimum:

~~~text
manafold-pr-gate
PR Fast
PR Integration
Windows Setup Smoke
CodeQL
Analyze (rust)
Analyze (python)
Analyze (actions)
~~~

Record HOSTED_CI = PASS only if every required check on the exact final head
is successful. Otherwise record FAIL or NOT_RUN with the exact check and URL.

## Final rerun and completion policy

- [ ] If independent review changes Rust, Python semantic code, TestKit,
  tests, fixtures, schemas, or verification tooling, set
  CODE_VERIFICATION_HEAD to the new exact source head and rerun the complete
  applicable Task 11 matrix. Re-run Task 12 evidence/archive after the final
  evidence change.
- [ ] If only documentation changes after code verification, use the
  change-aware documentation and archive rerun, but repeat archive verification
  on the final evidence head.
- [ ] Never claim EVD closure from a green test whose preconditions are
  unproven, an empty legal space, copied product, twin equality, an unrelated
  mutant, a contaminated witness, a partial rejection code, or a fixture
  helper that left state changed after Err.
- [ ] Keep FND-026B = BLOCKED_CONTRACT_AMBIGUITY with freeze readiness NO and
  FND-028 = BLOCKED_CONTRACT_AMBIGUITY with freeze readiness FREEZE_BLOCKER.
- [ ] Never create Batch I, begin M3, add Commander/capability/card scope,
  add a delivery mechanism, or merge the PR.

The plan gate is satisfied only when an independent reviewer records:

~~~text
BATCH_H_PLAN_REVIEW = APPROVE
PRODUCTION_IMPLEMENTATION_AUTHORIZED = YES
~~~
