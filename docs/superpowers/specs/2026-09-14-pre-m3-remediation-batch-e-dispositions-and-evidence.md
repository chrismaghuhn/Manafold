# Pre-M3 Remediation Batch E dispositions and evidence

**Status:** implementation evidence in progress; final exact-head review pending
**Date:** 2026-09-14
**Base:** `9996cfd0fcd4ef67d98cb0422611cebabd20e46b`
**Code/evidence head before final evidence metadata:** `a29075f3f105cabddc472d53754d447ef9af873b`
**Branch:** `chris/pre-m3-remediation-batch-e-cross-layer-closure`
**PR:** `NOT_RUN`

## Scope

Batch E owns FND-007, FND-008, FND-012B, FND-020, FND-022B, FND-022E,
FND-023, FND-024, FND-025, and FND-026B-FND-026D from Issue #164. It does
not absorb FND-009, FND-013-FND-019, FND-027-FND-032, EVD-001-EVD-014, or
Issue #162. No M3 work or real Magic semantics is included.

## Current disposition matrix

```text
TASK = PRE_M3_REMEDIATION_BATCH_E
BASE = 9996cfd0fcd4ef67d98cb0422611cebabd20e46b
HEAD = a29075f3f105cabddc472d53754d447ef9af873b
BRANCH = chris/pre-m3-remediation-batch-e-cross-layer-closure
PR = NOT_RUN

E1_RUNTIME_CLOSURE = PASS
FND_007_REMAINDER = RESOLVED_ON_BASE
FND_008_REMAINDER = RESOLVED_ON_BASE
FND_012B = REJECTED

E2_REPLAY_CHECKPOINT_CLOSURE = PASS
FND_020 = CONFIRMED
FND_022B = CONFIRMED
FND_022E = CONFIRMED
FND_023 = RESOLVED_ON_BASE
FND_024 = RESOLVED_ON_BASE
FND_025 = CONFIRMED

E3_MULTI_PERSPECTIVE_CLOSURE = PARTIAL
FND_026B = BLOCKED_CONTRACT_AMBIGUITY
FND_026C = DEFERRED_P2
FND_026D = RESOLVED_ON_BASE
E4_CROSS_LAYER_INTEGRATION = BLOCKED: E3 complete non-actor PlayerStepV2 product semantics remain contract-blocked

FND_007_CONTRACT = lower lifecycle seam is an intentional staging primitive; complete EngineState validation remains at the atomic transition/checkpoint owner
FND_008_MUTATION_FAMILY_MATRIX = reachable life/object/tap/decision/RNG/lifecycle mutations are event/cursor owned; allocator/revision progression is explicit; has_lost/turn/active/priority/player-universe mutation is rejected; effect/trigger/stack/format semantic mutation is not reachable in the current M2 foundation
FND_012B_CAUSALITY_POLICY = AnnouncedOutcome is a separate nonempty presentation occurrence and is not equated with RandomValueSampled

FND_020_PRODUCER_IDENTITY_POLICY = current synthetic producer rejects configured schema/RNG identities that differ from the actual V2/V3 producer constants; detached V3 validation keeps its nonempty observation identity rule
FND_022B_STATUS_UNIVERSE_POLICY = V3 checkpoint status outcomes equal EngineState.core.players; V3 replay status outcomes equal the manifest deck-player set; duplicate and noncanonical order fail at the V3 boundary
FND_022E_EXTERNAL_COUNTER_OWNER = environment replay executor owns application of resource_units_consumed and wall_clock_elapsed_millis through the replay-owned complete checkpoint restore path
FND_022E_APPLICATION_ORDER = validate detached step and deterministic before identity, execute response, validate transition/status/deterministic counters, validate recorded external monotonicity, apply recorded counters, recompute checkpoint identity, compare exact after identity
FND_023_STRUCTURAL_VS_VERIFIED_BOUNDARY = AuthoritativeReplayV3.validate is detached structural validation; execute_replay_from_checkpoint and ReplayExecutionReport are backend/checkpoint-verified evidence
FND_024_REJECTION_RECORDING_POLICY = wire Layer A and player semantic Layer B rejection are not authoritative replay steps; trusted accepted=false Layer C is replayable only when explicitly recorded and preserves complete identity; committed deterministic truncation is recorded; aborted Layer D2 failure is not recorded; unsupported external-counter status crossing fails closed
FND_025_CANONICAL_KEY_ORDER_POLICY = V3 checkpoint/replay boundaries reject noncanonical status/deck keyed-array order; checkpoint_digest_v3 retains its defensive PlayerOutcome sort; shared EpisodeStatus::validate is unchanged

FND_026B_COMPLETE_PRODUCT_POLICY = no neutral non-actor PlayerStepV2 submission semantics are defined; envelope validation is closed but complete non-actor PlayerStepV2 validation remains BLOCKED_CONTRACT_AMBIGUITY
FND_026C_DELIVERY_POLICY = actor-bound endpoint only; no live non-actor delivery promise, queue, mailbox, polling API, callback, or hidden controller state in the foundation; DEFERRED_P2
FND_026D_REPLAY_REPROJECTION_EVIDENCE = eventful_replay_reprojects_both_perspectives_byte_exactly at the production lifecycle projector from replay before/events/after; P1 and P2 batches are nonempty, distinct, privacy-safe, and the source controller is unchanged

ADR_CANDIDATES = NONE
ADR_ACCEPTED = NONE

CONFIRMED = FND-020, FND-022B, FND-022E, FND-025
REJECTED = FND-012B
RESOLVED_ON_BASE = FND-007 remainder, FND-008 remainder, FND-023, FND-024, FND-026D
BLOCKED_CONTRACT_AMBIGUITY = FND-026B; external-counter-driven status transition without a threshold contract is unsupported under FND-022E
SPLIT_REQUIRED = NONE
DEFERRED_P2 = FND-026C

RED_TESTS = PASS: fnd_020_current_producer_rejects_a_false_observation_schema_identity, fnd_022b_checkpoint_requires_the_exact_authoritative_player_universe, fnd_025_checkpoint_rejects_noncanonical_status_order, fnd_025_manifest_rejects_noncanonical_deck_and_status_order, fnd_022e_replay_applies_recorded_external_counter_progression, eventful_replay_reprojection_requires_nonempty_base_batches
FOCUSED_TESTS = PASS: mtgml-state 93, mtgml-rules 35, mtgml-replay 14, mtgml-environment 58, mtgml-observation 10, mtgml-conformance 125
WORKSPACE_TESTS = PASS: cargo test --workspace --all-features --locked, 432 tests, 0 failures
FMT = PASS: cargo fmt --all -- --check
CHECK = PASS: cargo check --workspace --all-targets --all-features --locked
CLIPPY = PASS: cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
PYTHON_FULL = PASS: .venv/Scripts/python.exe scripts/run_python_tests.py --profile full, 387 tests, 3 expected M2.H skips
SCHEMA_GATES = PASS: 38 wire fixtures and 9 maintainer artifacts validated against schemas
WIRE_GATES = PASS: 38 golden fixtures and 43 negative fixtures verified; direct Python wire profile passed
MAINTAINER_GATES = PASS: direct fast/integration profiles; generated contracts, repository, Rust structure, documentation, golden path, maintainer artifacts, and toolchain checks passed
LOCAL_CHECK_FAST = BLOCKED: just cannot start WSL /bin/bash on this Windows host
LOCAL_CHECK = BLOCKED: just cannot start WSL /bin/bash on this Windows host
DIRECT_FAST_PROFILE = PASS: .venv/Scripts/python.exe scripts/run_checks.py fast
DIRECT_INTEGRATION_PROFILE = PASS: .venv/Scripts/python.exe scripts/run_checks.py integration
HOSTED_CI = NOT_RUN

PUBLIC_API_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO

INFORMATION_SAFETY = PASS for the eventful projection test's observed products; no new player surface exposes trusted IDs, RNG provenance, checkpoint identity, or hidden order

WORKTREE_CLEAN = YES at a29075f
REMOTE_HEAD_EQUALS_LOCAL = NOT_RUN

NEW_MAGIC_SEMANTICS = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
MERGE_PERFORMED = NO
```

## Evidence and implementation notes

### E1

FND-007 is resolved on the base for the remaining question. The public
lifecycle helper can intentionally stage a perspective mutation before the
physical mutation, and the resulting intermediate state is not a complete
committable state. The accepted fixture/transition owner validates the complete
candidate before commit. FND-008 has no reachable unexplained accepted mutation
in the current M2 program: existing event/cursor/progression proofs cover the
reachable families, while future stack/effect/trigger/format/core semantic
families remain unsupported or unreachable. FND-012B remains separate from RNG
causality and is rejected as a finding under the accepted presentation policy.

### E2

The producer guard rejects a false current schema identity without changing the
detached reader's nonempty observation identity rule. Closed statuses are
validated against the real authoritative player universe at checkpoints and
against manifest deck players in V3 replay. `EpisodeStatus::validate()` remains
the shared local validator and is not changed for FND-025.

The checkpoint digest helper still sorts keyed player outcomes defensively. The
authoritative V3 boundaries reject unsorted status/deck input before accepting
it as current identity. Replay applies recorded resource and wall-clock
progression on an internal fork through the existing checkpoint restore path;
the host wall clock is never sampled.

### E3 and E4

FND-026B is not implemented through an invented non-actor PlayerStep. The
unanswered contract question is the neutral meaning of PlayerStepV2.submission
for a perspective that did not submit, or the separately versioned product that
should replace it. FND-026C is deferred because the current actor-bound API does
not promise live non-actor delivery.

FND-026D is closed by the eventful replay test. The fixture only creates one
eventful situation. Live and replayed observed-event batches both use
project_occurrence_envelopes, and the replay runs through the production
execute_replay path. EVD-010 is not closed by this single test. E4 remains
partial until the final matrix and all required gates are recorded; no blocked
non-actor contract is represented as green behavior.

## PR body

The PR body will repeat this scope and every disposition. It will state the
exact policies for rejected submissions, trusted external counters, structural
versus verified replay, complete per-perspective validation, and non-actor
delivery. It will list every command and exact count from the final verification
pass, classify blocked/deferred work without vague wording, state all six
compatibility fields, and end with `M3_AUTHORIZED = NO`.

The PR will target `master`, use the title `Pre-M3 remediation Batch E:
cross-layer runtime and replay closure`, and remain unmerged. Hosted CI will be
reported only for the final pushed PR head.
