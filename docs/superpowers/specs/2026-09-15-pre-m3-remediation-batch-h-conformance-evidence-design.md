# Pre-M3 Remediation Batch H: Conformance Evidence and Proof Completeness

**Status:** proposed for independent design review

**Date:** 2026-09-15

**Repository:** chrismaghuhn/Manafold

**BASE:** ff37f0896cdbb8e2faea424859faf128155b4579

**Pre-design branch:** chris/pre-m3-remediation-batch-h-conformance-evidence-closure

**Issue:** #164

**Independent design review:** REQUEST_CHANGES on review 1; amendments below require re-review before implementation

## 1. Purpose and boundary

Batch H closes or characterizes the fourteen P1 evidence findings EVD-001
through EVD-014 and the deferred FND-016B product-parity slice. Its purpose is
to make a passing conformance result mean that the claimed invariant was
actually exercised and independently checked.

The batch does not start M3, add Magic rules or cards, change Card IR,
introduce a capability, add a delivery mechanism, create a new replay or
checkpoint version, alter digest domains, change the RNG algorithm, or decide
the PlayerId(0) policy. FND-026C remains deferred and no queue, mailbox,
polling API, callback, or controller-global delivery state is in scope.

The verified base is the merged Batch-G head. The clean exact-head startup
checks were:

    LOCAL_MASTER = ORIGIN_MASTER
    WORKTREE_CLEAN = YES
    BASE = ff37f0896cdbb8e2faea424859faf128155b4579

The unchanged Batch-G baseline executed
`cargo test --workspace --all-features --locked` successfully. The current
workspace run included 127 mtgml-conformance tests and exited with zero
failures. This is baseline evidence only; it does not close any H finding.

## 2. Design decision

The recommended design keeps semantic authority in the existing owners and
adds evidence at the narrowest boundary that can prove each claim:

1. Persistence and RNG evidence consumes the committed corpus and the real
   rejection sampler.
2. The isolation harness captures revision-bound fingerprints, enforces an
   exact authorized state-difference relation, derives rejected products from
   pre-state, and proves accepted progress before using parity as evidence.
3. The legal-space harness applies finite complement probes, validates every
   reference transition, bounds both sides of exploration, canonicalizes
   independently of declaration order, and rejects trace drift.
4. Lifecycle fixture support becomes transactional and uses explicit stream
   identity; lifecycle reidentification is bound to a retired physical chain.
5. The eventful replay test uses the real player endpoint for the live product
   and the existing production projector for replay reprojection.

No proposed change creates a second rules engine. Reference and mutant code
remain conformance-only and cannot be imported by production rules or
environment code.

### H1/H2 split decision

Characterization found two implementation families:

    H1 = EVD-001, EVD-002, EVD-003, EVD-004, EVD-005, EVD-006,
         EVD-010, EVD-012, EVD-013
    H2 = EVD-007, EVD-008, EVD-009, EVD-011, EVD-014

The families are separately reviewable at the evidence level, but they share
the feature-gated FixtureTransition seam used by the checkpoint, eventful
replay, lifecycle, and fixture tests. Splitting that seam would make the
first PR's evidence depend on an unreviewed second PR. The reviewed design
therefore keeps one bounded implementation plan and one PR:

    H_SPLIT_REQUIRED = NO

The plan will sequence the shared fixture seam before the evidence consumers
and keep the legal-space/oracle changes in a clearly named task group. Only
the already-created single Batch-H branch exists.

## 3. Authority and evidence map

The table records the exact claimed invariant, normative owner, current
evidence owner, production/test owner, proof strategy, vacuity or common-mode
risk, post-Batch-G behavior, and proposed disposition. No production change
means the gap is in evidence or test support, not in the trusted semantic
engine.

| Finding | Claimed invariant and normative owner | Current evidence owner | Production/test owner | Current proof and weakness | Post-A-G characterization | Disposition |
|---|---|---|---|---|---|---|
| EVD-001 | Every committed positive persistence fixture is consumed and CBOR/envelope resource boundaries and ADR-0040 precedence are exercised. Owners: docs/STATE_HASHING.md and ADR 0040. | mtgml-persistence tests and python/tests/test_persistence_codec.py. | No semantic production change; Rust positive-corpus and boundary tests in mtgml-persistence. | Rust consumes the negative manifest but not persistence/golden/manifest.json. Existing tests cover selected upper bounds, not every boundary-1/boundary/boundary+1. Hand-built tests can drift from committed bytes. | Batch G aligned the nested array/depth precedence and recorded negative parity, but did not add a Rust positive-corpus consumer or complete boundary matrix. | CONFIRMED |
| EVD-002 | The production uniform-below sampler consumes rejected raw words, advances the cursor for each word, then returns the first accepted word. Owner: RNG contract and docs/REPLAY_AND_DETERMINISM.md. | mtgml-random sampling tests. | No semantic production change; direct production-path KAT in mtgml-random. | forced_rejection_stub exercises a separate loop and never calls uniform_below_u64. A direct KAT using a bound with a known rejected prefix is absent. Common-mode risk is hidden by the disconnected stub. | Batch G did not change sampling or add direct rejection evidence. | CONFIRMED |
| EVD-003 | Checkpoint/restore/fork parity proves correctness and nonmutation, not just equality of two executions. Owners: docs/REPLAY_AND_DETERMINISM.md and docs/TESTING_AND_CONFORMANCE.md. | mtgml-conformance isolation checkpoint_parity, fork_parity, and endpoint_pair. | No production change; conformance assertions/tests. | Several tests compare twins or a restored value without an independent exact revision, status, counter, decision, or semantic-mutation expectation. Source nonmutation is sometimes only implicit in a group comparison. | Batch G changed persistence identity only; current parity tests still contain twin-equality-only subclaims. | CONFIRMED |
| EVD-004 | A noninterference pair differs on exactly the authorized hidden axis and matches on every other relevant state component. Owners: docs/INFORMATION_MODEL.md, docs/testing/NONINTERFERENCE_TESTING.md, and docs/THREAT_MODEL.md. | mtgml-conformance isolation/witnesses.rs and paired_matrix.rs. | No production change; conformance witness relation. | Current axis predicates are existential and the relation checks only the witness perspective's knowledge, decision, and selected identity maps. Contaminated changes in unrelated authoritative fields can survive. Marker checks do not establish the full state relation. | Batch G did not touch paired witnesses. Current ten-axis tests pass, but their construction discipline is not enforced by a full authorized-difference relation. | CONFIRMED |
| EVD-005 | Every typed rejection preserves backend state and returns a complete PlayerStepV2 independently derived from the pre-state. Owners: docs/ML_ENVIRONMENT.md, docs/DECISION_PROTOCOL.md, and docs/contracts/WIRE_CONTRACT.md. | mtgml-conformance isolation/rejection.rs plus environment endpoint product code. | No public API change; conformance pre-state product oracle and matrix tests. | semantic_matrix_fingerprint_stable checks a rejection code and a broad before/after fingerprint but does not compare every returned field to a pre-state-derived expected step. It can miss a wrong observation, status, next decision, or protocol field if the code is right. | Batch F closed only FND-016A and explicitly deferred unchanged-product parity to EVD-005. Batch G did not change this boundary. | CONFIRMED |
| EVD-006 | An accepted-transition witness proves accepted=true, exactly one revision/counter progression, and the expected semantic/decision mutation before parity is used. Owners: docs/TESTING_AND_CONFORMANCE.md and docs/contracts/ACCEPTANCE_GATES.md. | mtgml-conformance paired transition helpers and checkpoint/fork tests. | No production change; conformance accepted-progress helper. | accepted_entry_submission returns a step without asserting acceptance. Some parity helpers can compare equal rejected products or equal no-op products. | Existing individual tests often assert Accepted, but the shared helper used by the axis matrix does not make acceptance a precondition and does not assert exact progress. | CONFIRMED |
| EVD-007 | A nonempty independent legal set with an empty production path is MissingChoice, never a completeness pass. Owner: docs/DECISION_PROTOCOL.md and docs/TESTING_AND_CONFORMANCE.md. | mtgml-conformance legal_space/comparator.rs and gate_evidence.rs. | No production semantic change; comparator and regression test. | completeness_defects treats Some(empty_vec) as a represented path. The live producer normally records nonempty paths, so the existing tests do not exercise this false-positive shape. | Batch G did not change legal-space comparison. | CONFIRMED |
| EVD-008 | Representative bounded inputs outside the advertised decision surface are rejected and preserve state. Owner: docs/DECISION_PROTOCOL.md and acceptance legal-space gates. | mtgml-conformance legal_space/explorer.rs and gate_evidence.rs. | No production change; finite probe grammar and gate evidence. | Probe generation enumerates advertised subsets/permutations and number sentinels, but omits unknown IDs, duplicate/order/cardinality complements, and wrong variants for several families. Soundness is therefore tested only on the advertised side. | Batch G did not change the explorer. | CONFIRMED |
| EVD-009 | Reference and production exploration have typed uniform bounds, validated transitions, canonical request/trace shape, and fail closed on exhaustion. Owners: docs/TESTING_AND_CONFORMANCE.md and docs/contracts/ACCEPTANCE_GATES.md. | mtgml-conformance legal_space/oracle.rs, explorer.rs, comparator.rs. | No production semantic change; conformance oracle/explorer. | ReferenceAutomaton::advance silently ignores invalid choices; reference enumeration is unbudgeted; reversed declaration order changes expected request atoms; request_sequence_defects ignores extra observed requests. | Batch G did not change the legal-space oracle. | CONFIRMED |
| EVD-010 | An eventful real transaction produces nonempty authoritative events, nonempty player observed events, a complete PlayerStep, and exact replay reprojection. Owners: docs/REPLAY_AND_DETERMINISM.md and docs/ML_ENVIRONMENT.md. | mtgml-environment/src/replay_parity_tests.rs and existing eventful test fixture. | No production change; test routes through PlayerEndpoint::submit and existing projector. | The eventful test uses execute_trusted_response and hand-assembles the live PlayerStep with player_step_from_state. Nonempty projection is proven, but live endpoint product ownership is not exercised. | Batch E closed FND-026D's projector evidence and retained FND-026B as a contract ambiguity; EVD-010 remains open. | CONFIRMED |
| EVD-011 | Lifecycle randomization retires the claimed physical/opaque chain, and reidentification reveals a member of that retired chain. Owners: docs/INFORMATION_MODEL.md and lifecycle conformance cases. | mtgml-conformance lifecycle.rs tests. | No semantic production change; fixture selection and chain assertions. | scenario_reidentification selects the first hidden object and copies its physical identity into the expectation. The test can pass even when the selected card was never part of the retired P1 chain. | Batch G did not change lifecycle fixtures. | CONFIRMED |
| EVD-012 | One complete fingerprint belongs to one coherent trusted instant and retains required immutable protocol/manifest identity even when recorder history is excluded. Owners: docs/REPLAY_AND_DETERMINISM.md, docs/STATE_HASHING.md, and information-safety policy. | mtgml-conformance isolation/fingerprint.rs. | No production change; revision-bound conformance capture. | capture_complete performs separate checkpoint, replay, P1, and P2 reads without checking one revision. The trusted surface retains only a subset of manifest identity, so excluding recorder history can also exclude execution context identity. | Batch G did not change fingerprint capture. | CONFIRMED |
| EVD-013 | Each mutation guard reaches the named faulty projector/generator after valid prerequisites; structural negatives are labeled separately. Owners: docs/testing/NONINTERFERENCE_TESTING.md and conformance gate policy. | mtgml-conformance isolation/mutants.rs. | No production change; conformance mutant harness/tests. | M1/M2 literal channels are structurally impossible and use fallback carriers, but tests are named as if they detect the original channel. Step mutant evidence does not uniformly assert the clean product reached the intended class before mutation. | Batch G hardened diagnostic safety only; mutant reachability remains mixed. | CONFIRMED |
| EVD-014 | Every fallible fixture helper is transactional, fail-closed, explicit about RNG stream choice, bounded to its declared capability, and does not panic where a typed error is promised. Owners: docs/testing/CONFORMANCE_AUTHORING.md and fixture support policy. | mtgml-rules fixture_support.rs and mtgml-conformance lifecycle/paired helpers. | Feature-gated testkit only: mtgml-rules::fixture_support plus conformance wrappers. | move_object_incarnation/apply_occurrence/record_hidden_random_sample can mutate the fixture workspace before a later bind error; record_hidden_random_sample uses keys().next(); one wrapper panics while converting a typed fixture error. | Batch G did not change fixture transactionality or stream selection. | CONFIRMED |

The evidence owner, not a new engine layer, owns every confirmed gap. If a
future implementation discovers a directly affected file outside this table,
the scope-extension rule applies before that file is changed.

### EVD-004 exact authorized-difference relation

The witness relation is a typed clone-normalization relation over the complete
EngineState, not a list of fields that the comparator happens to ignore. Both
states must first pass the authoritative state validator. The conformance
relation then normalizes only the permitted differences below and requires the
normalized states to be exactly equal. A separate non-vacuity check requires
the declared difference to be present in the stated shape.

| Axis | Only permitted difference after normalization | Non-vacuity requirement |
|---|---|---|
| OpponentHiddenDefinition | CardDefinitionId on face-down zone objects and the matching non-witness retained-knowledge definition markers | Exactly one face-down object and its mapped foreign knowledge record change definition; all other object and record fields remain equal |
| HiddenConcealedOrdering | The P2 face-down library ordered member vector, matching object ZonePosition values, and matching foreign known-location ZonePosition values | The member multiset is equal, the ordered vector differs, and every changed position is one of those members |
| ForeignPrivateLook | Active-record membership for one non-witness perspective | Exactly one foreign active opaque key is added or removed; all common records, retired records, cursors, and other state fields are equal |
| FaceDownIdentity | PhysicalCardId assignments among the same face-down objects and their matching foreign retained records | At least two assignments differ but the face-down physical-card multiset is equal |
| RootSeedPreAuth | RandomStateV1.root_seed only | Seeds differ; revisions, streams, and every non-random state component are equal |
| HiddenRngCursor | The explicitly named global SyntheticM1 stream cursor only | That cursor differs; stream keys, other cursors, and every non-random state component are equal |
| ObjectRenaming | The declared object bijection in zone map keys, object IDs, ordered-zone references, stack/pending trusted references, and every perspective identity mapping target/key, plus the corresponding global next-object allocator head | At least one declared object mapping changes; no undeclared object key/reference changes |
| AbilityRenaming | The declared ability bijection in stack references, pending decision trusted ActivateAbility bindings, and perspective identity mapping target/key, plus the corresponding global next-ability allocator head. Opaque ability keys remain identical; reverse-map keys are remapped only by the same declared bijection. | At least one declared ability mapping changes; no undeclared ability key/reference changes |
| GlobalAllocatorHistory | The global IdentityAllocatorState only | Global allocator values differ while the witness perspective's complete identity record remains equal |
| ForeignKnowledgeHistory | known-location and historical-location fields of one foreign active record | Exactly one foreign record changes only in those two history fields; all other foreign knowledge and all other state fields are equal |

The implementation will keep this relation in a focused conformance module if
that avoids enlarging witnesses.rs. It will add a contaminated-pair negative
that performs a valid declared object rename plus an unrelated life change and
proves the witness rejects it. TransformReport metadata is not treated as
proof; the actual normalized state comparison is the proof.

## 4. FND-016B and freeze-readiness decisions

### FND-016B

FND-016A is already closed by Batch F. FND-016B is CONFIRMED and owned by
the EVD-005 rejection matrix. The expected rejected product is constructed
before submission from:

- the pre-submission endpoint information state;
- the pre-submission visible decision, when the rejection class requires one;
- the pre-submission checkpoint status;
- the matrix's independently declared rejection code;
- an empty observed-event vector.

The actual returned PlayerStepV2 is compared field-for-field with that
expected value, including schema version, information state, observation,
observed events, rejection, status, next decision, actor, decision revision,
and opaque identity effects as represented by player-safe products. The
complete pre/post checkpoint and endpoint fingerprint is also compared. A
row that cannot derive a complete expected product remains
SPLIT_REQUIRED; it cannot be counted as closed by code-only equality.

The intended final status is CLOSED only after all rows in the current
accepted rejection matrix pass this independent product comparison.

### FND-026B

Current status remains BLOCKED_CONTRACT_AMBIGUITY. The current accepted
ML/environment and wire contracts define an actor-submission-bound
PlayerStepV2. They explicitly do not define a neutral submission value for a
non-acting perspective and do not promise live non-actor delivery. The
environment commit owns validation of every projected occurrence envelope and
the replay projector can derive non-actor observed-event batches, but neither
surface claims a complete non-actor PlayerStepV2.

    MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = NO
    FND_026B_FREEZE_READINESS = NONBLOCKING_PROVISIONAL_POLICY

This is not a semantic resolution of the ambiguity. No non-actor product,
delivery mechanism, or version is introduced by H.

### FND-028

Current status remains BLOCKED_CONTRACT_AMBIGUITY. The accepted executable
surfaces still have contradictory zero behavior: generic/model/state and
declared environment/player identities can contain PlayerId(0), while Replay
V3 rejects a zero step actor. The unnumbered PlayerId-zero ADR candidate is
non-authoritative and no option has been accepted.

Because this is a cross-surface identity policy affecting model, state,
environment reset/binding, player projection, wire, and replay actor
semantics, the foundation cannot make a coherent frozen identity claim while
the policy is undecided.

    MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = YES
    FND_028_FREEZE_READINESS = FREEZE_BLOCKER

H records this blocker and does not implement any zero policy. An accepted
ADR, normative update, compatibility decision, and then implementation are
required in a later authorized slice.

## 5. Proof strategy and common-mode controls

The following controls apply to every H implementation:

- A passing parity comparison is never the only correctness expectation.
  Checkpoint/fork tests assert exact revision/counter/status/decision
  progress, and source nonmutation is explicit.
- A reference legal set is independently declared and is never read from
  production candidate generation. Reference transitions return a typed
  failure for invalid choices.
- The completeness comparator treats a missing map entry and an empty path
  list as the same missing-choice defect.
- Every rejection row constructs its expected player product from pre-state
  data before invoking submit. The expected code and class matrix are not
  read from the returned step.
- Noninterference witnesses compare a complete normalized authoritative state
  against an axis-specific authorized-difference set. The relation first
  proves both states are valid, then proves the intended difference exists,
  then compares player-safe products.
- Mutation tests distinguish structural validator negatives from valid
  controlled mutants. A structural impossibility claim is not counted as
  projector/generator sensitivity.
- Eventful replay tests assert nonempty authoritative events and nonempty
  live/replayed event batches before byte parity.
- Fixture helper failure tests snapshot the fixture workspace, call a helper
  that fails after a possible mutation point, and assert exact workspace,
  event-vector, offset, and RNG equality.

No default diagnostic or fixture failure output will render root seeds, raw
stream keys, RNG cursors, physical cards, authoritative object/ability IDs,
trusted bindings, private knowledge, checkpoints, or full authoritative
state. New assertions compare typed values internally and report closed
failure categories or safe labels.

### EVD-003 and EVD-006 accepted-progress contract

Every accepted witness used by checkpoint, fork, or paired-state evidence
must perform these assertions before comparing the two sides:

1. the returned PlayerStep submission is Accepted;
2. the before and after revisions differ by exactly one;
3. decisions_submitted and accepted_transitions each increase by exactly one;
4. resource and wall-clock counters remain unchanged unless the scenario
   explicitly names a trusted external progression;
5. the expected semantic mutation is present, independently derived from the
   fixture contract: entry changes the actor life from 40 to 38, consumes the
   named synthetic RNG stream, creates the ChooseCount continuation, and
   exposes the frozen 0..3 number request; the count witness changes that
   request to the exact ChooseMany request for the selected count;
6. the returned step's revision/status/next decision matches the independently
   expected after product, and the replay trace carries the expected event and
   delta products;
7. rule_events_emitted_after equals
   rule_events_emitted_before plus exactly the number of authoritative events
   in the accepted transition. H's accepted semantic witnesses have
   resource_units_consumed_after equal to before and
   wall_clock_elapsed_millis_after equal to before. A test that applies
   recorded external progression must name the exact recorded delta and
   compare it to that value; no external-counter delta is left open-ended.

Checkpoint tests also retain an immutable copy of the source checkpoint and
compare it after every fork-side accepted/rejected/restore operation. Fork
tests compare the parent checkpoint and replay recorder directly, in addition
to the complete fingerprint. Equal outputs are never the acceptance
predicate.

### EVD-008 and EVD-009 bounded probe contract

The shared conformance budget is:

    max_candidates_per_request = 8
    max_numeric_span = 16
    max_depth = 4
    max_total_nodes = 64
    max_generated_answers = 256

The reference and production explorers use the same budget values while
retaining separate typed error enums. Each visible request receives a finite
complement:

- ChooseOne: every advertised candidate, one unknown candidate, and one
  wrong-union SelectMany answer;
- ChooseMany: every bounded subset, one duplicate answer, one unknown-member
  answer, one overlong answer, one reversed two-member answer when available,
  and one wrong-union number answer;
- ChooseNumber: every value within the bounded interval, both representable
  outside-range sentinels, and one wrong-union SelectOne answer;
- Order: every bounded permutation for lengths minimum through maximum, every
  bounded below-minimum permutation in the finite complement, one duplicate
  answer, one unknown member, one overlong answer, and one wrong-union number
  answer. Above-maximum lengths are represented by the smallest bounded
  extension that is syntactically constructible; if the extension would
  exceed the shared budget, the typed budget result is recorded instead of
  silently truncating.

Advertised status includes membership, uniqueness, and the canonical
SelectMany ordering rules; a representative complement is never mislabeled
as advertised. Every rejected complement probe is checked against a
before/after complete fingerprint of its branch. An accepted complement is a
typed OutOfContractAccepted defect. Budget exhaustion is a typed error and
never a truncated completeness result.

ReferenceAssemblySpec validation rejects duplicate or unsupported declared
atoms. ReferenceAutomaton construction and advance return typed errors.
Reference enumeration consumes the shared node/depth/generated-answer
budget. The frozen reference path length is exactly four stages:
Anchor, Number, Members, Order. Request comparison requires equal expected
path and observed-request lengths, rejects any request after the reference
automaton reaches Complete, and reports an invalid reference transition
instead of silently retaining the old state. Declaration iteration order is
normalized before an expected request is emitted.

### EVD-012 revision-bound fingerprint contract

TrustedEnvironmentIdentitySurface retains the complete non-secret immutable
execution context that is currently present in ReplayManifestV3. The
protected randomness.root_seed_hex is intentionally not rendered or stored in
the default diagnostic fingerprint; it remains trusted checkpoint/replay
input and is not replaced by a player-visible surrogate:

    engine_build
    kernel.implementation_id / semantic_version / build_profile
    rules_snapshot
    format_policy_snapshot
    oracle_snapshot
    card_bundle
    randomness.contract_id
    all ReplaySchemaVersionsV1 fields
    every DeckIdentityV1 player/deck_id/digest
    the ReplayManifestV3.initial_identity segment anchor, including its
    revision, status, environment counters, codec identity, full-state
    digest, and checkpoint digest
    the exact DigestReferenceV1 tuple for the current FullStateDigestV3
    (envelope_version, algorithm_id, semantic_domain, payload_codec_id,
    input_schema_id, digest_bytes)
    the exact DigestReferenceV1 tuple for the current CheckpointDigestV3
    using environment-checkpoint-digest-input.v3 and its digest bytes

The environment group continues to retain the current checkpoint schema,
codec identity, current status/counters, FullStateDigestV3, and
CheckpointDigestV3. It never stores root_seed_hex, raw stream keys, or raw
cursor values in this public/debug fingerprint surface.

capture_complete captures one checkpoint first, reads the replay and both
endpoint products, and then verifies the replay final identity equals that
checkpoint's revision, digest, status, counters, codec, and checkpoint
digest. It copies the complete non-secret manifest context and the
manifest.initial_identity anchor into the fingerprint, and derives both
digest-reference tuples from the captured typed digests rather than from
unrelated constants. capture_snapshot verifies that observation,
information state, and visible decision (when present) all carry the same
revision and perspective. The final checkpoint read must equal the first
checkpoint. Any revision or identity drift returns a closed
incoherent-capture error. This is an explicit revision-bound capture; it
introduces no mutable cache and does not claim a multi-thread lock over the
entire controller.

FingerprintComparison::All compares the captured segment anchor as well as
the current identity. FingerprintComparison::ExcludeReplayRecorder retains
the anchor for diagnostics and separate assert_segment_anchor checks, but
does not compare the anchor's mutable segment revision/digest/status/counter
values because fork and restore intentionally rebase a fresh replay segment.
It still compares the complete immutable manifest context and the digest
reference schema/domain/codec identities. Thus excluding recorder history
cannot drop execution-context identity and cannot make a fork look equal by
discarding the manifest.

### EVD-014 transactional fixture contract

Each fallible FixtureTransition helper runs its operation against a cloned
workspace, event vector, and offset. The clone is committed only after all
state mutation, event binding, and counter arithmetic succeeds. An Err
restores the exact workspace/event/offset snapshot. The rule applies to
move_object_incarnation, apply_occurrence, and record_hidden_random_sample.
The random helper resolves exactly
RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1), obtains cursors
through the checked lookup API, and fails if that stream is absent; it never
selects the first map entry. Conformance error adapters return typed closed
errors and do not panic during normal helper failure.

## 6. Expected files and ownership

The expected implementation/evidence set is deliberately bounded:

    docs/superpowers/specs/2026-09-15-pre-m3-remediation-batch-h-conformance-evidence-design.md
    docs/superpowers/plans/2026-09-15-pre-m3-remediation-batch-h-conformance-evidence.md
    docs/superpowers/specs/2026-09-15-pre-m3-remediation-batch-h-dispositions-and-evidence.md
    docs/normative-document-register.v1.json
    crates/mtgml-persistence/src/tests.rs
    crates/mtgml-random/src/sampling.rs
    crates/mtgml-conformance/src/legal_space/comparator.rs
    crates/mtgml-conformance/src/legal_space/oracle.rs
    crates/mtgml-conformance/src/legal_space/explorer.rs
    crates/mtgml-conformance/src/legal_space/gate_evidence.rs
    crates/mtgml-conformance/src/isolation/witnesses.rs or a focused extracted relation module
    crates/mtgml-conformance/src/isolation/fingerprint.rs
    crates/mtgml-conformance/src/isolation/checkpoint_parity.rs
    crates/mtgml-conformance/src/isolation/fork_parity.rs
    crates/mtgml-conformance/src/isolation/endpoint_pair.rs
    crates/mtgml-conformance/src/isolation/rejection.rs
    crates/mtgml-conformance/src/isolation/mutants.rs
    crates/mtgml-conformance/src/isolation/paired_matrix.rs
    crates/mtgml-conformance/src/isolation/mod.rs
    crates/mtgml-environment/src/replay_parity_tests.rs
    crates/mtgml-rules/src/fixture_support.rs
    crates/mtgml-conformance/src/lifecycle.rs

The final plan may remove files from this list when characterization proves
that an owner needs only an existing test, but it may not add an unreviewed
affected owner. No wire fixture, schema, replay artifact, checkpoint
artifact, generated contract vocabulary, or Python semantic engine is
expected.

## 7. Compatibility, determinism, and artifact impact

Expected defaults are:

    FROZEN_PUBLIC_API_CHANGE = NO
    WIRE_CHANGE = NO
    SCHEMA_CHANGE = NO
    CHECKPOINT_SCHEMA_VERSION_CHANGE = NO
    REPLAY_VERSION_CHANGE = NO
    DIGEST_DOMAIN_CHANGE = NO
    RNG_ALGORITHM_CHANGE = NO
    HISTORICAL_REPLAY_CHANGE = NO
    HISTORICAL_CHECKPOINT_MEANING_CHANGE = NO
    NEW_MAGIC_SEMANTICS = NO
    CAPABILITY_REGISTRY_CHANGE = NO
    COMMANDER_SUPPORT_CLAIM = NO

The feature-gated FixtureTransition transaction wrapper is an internal
testkit change. The final evidence record must state
RUST_API_CHANGE, PYTHON_API_CHANGE, and TESTKIT_API_CHANGE from the
actual diff. No public player or replay API is intended to change.

Determinism is preserved by using exact committed bytes, a known production
HMAC sampler path, explicit stream keys, typed budgets, BTreeSet/BTreeMap
canonicalization, revision-bound captures, and existing Rust-owned replay and
projection paths. No historical Replay V3 or Checkpoint V3 byte/meaning
change is authorized. If a proof gap appears to require a version change, the
slice stops with VERSIONING_REQUIRED = YES and no V4 artifact is created.

## 8. Design gate

Before a plan or production change is started, the independent review must
verify:

- every EVD row has a complete owner/disposition/proof strategy;
- FND-016B is explicitly owned by the complete rejection-product oracle;
- FND-026B is not silently resolved and its freeze-readiness is justified by
  the current actor-bound contract;
- FND-028 remains an explicit freeze blocker without an invented policy;
- the authorized-difference relation is exact enough to reject contaminated
  witnesses;
- legal-space complements and reference failures are bounded and fail closed;
- no test claims correctness from empty paths, twin equality, copied
  expectations, unrelated mutants, or manual products;
- the expected file set and H1/H2 boundary are reviewable;
- information safety and historical artifact compatibility are preserved.

Required gate values before production implementation:

    BATCH_H_DESIGN_REVIEW = APPROVE
    H_SPLIT_DECISION = NO
    PRODUCTION_IMPLEMENTATION_AUTHORIZED = YES

Until those values are produced by independent review and recorded, this
document authorizes characterization/design work only. M3 remains
unauthorized.

    M3_STARTED = NO
    M3_AUTHORIZED = NO
    FOUNDATION_READY_FOR_M3 = NO
    MERGE_PERFORMED = NO
