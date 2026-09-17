# FND-028 PlayerId(0) Implementation Evidence Candidate

**Status:** complete candidate; implementation review and merge are pending

**Date:** 2026-09-15

**Base:** 8018b61416aafbd032df58b7f5cda68dca4f07cb

**Accepted authority:** docs/adr/0050-player-id-zero-policy.md, Option A

**Issue:** https://github.com/chrismaghuhn/Manafold/issues/164

This is a committed implementation-evidence candidate. It records source,
test, compatibility, and local verification evidence only. Review, Hosted CI,
merge, and merged-master status are intentionally not part of this record.
FND-028 remains open until those external lifecycle steps complete.

## Candidate status

~~~
ADR_0050 = ACCEPTED
SELECTED_POLICY = OPTION_A

RUST_ZERO_REPLAY_VALIDATION = PASS
PYTHON_ZERO_REPLAY_VALIDATION = PASS
DECLARED_ZERO_STATE = PASS
DECLARED_ZERO_ENDPOINT = PASS
DECLARED_ZERO_LIVE_TRANSITION = PASS
DECLARED_ZERO_REPLAY_EXPORT = PASS
DECLARED_ZERO_REPLAY_EXECUTION = PASS

UNDECLARED_ZERO_REJECTION = PASS
WRONG_ACTOR_ZERO_REJECTION = PASS
WRONG_DECISION_ID_ZERO_REJECTION = PASS
STALE_REVISION_ZERO_REJECTION = PASS

CHECKPOINT_PARITY = PASS
RESTORE_PARITY = PASS
FORK_PARITY = PASS
RUST_PYTHON_PARITY = PASS

WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
REPLAY_VERSION_CHANGE = NO
CHECKPOINT_VERSION_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
HISTORICAL_REPLAY_MEANING_CHANGE = NO
API_CHANGE = NO
MIGRATION_REQUIRED = NO

FND_028_IMPLEMENTATION_EVIDENCE = COMPLETE_CANDIDATE
FND_028_IMPLEMENTATION_REVIEW = PENDING
FND_028 = OPEN_PENDING_REVIEW_AND_MERGE
PRE_M3_FREEZE_BLOCKER = YES
FOUNDATION_READY_FOR_M3 = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
~~~

## Implementation scope

The only production changes are:

- crates/mtgml-replay/src/v3.rs: removed the numeric step.actor.0 == 0
  disjunct from detached Replay V3 identity-chain validation.
- python/src/mtgml/replay.py: removed the matching numeric step.actor == 0
  disjunct from the Python detached Replay V3 validator.

The existing backend execution owner remains unchanged. It still verifies the
manifest against the starting checkpoint, the pending actor, the
player-decision identity, revisions, transition product, counters, checkpoint
digests, and final identity before accepting replay execution.

No PlayerId representation, declaration rule, endpoint binding rule, schema,
wire codec, checkpoint format, digest domain, RNG, decision schema, observation
schema, or historical fixture was changed.

## RED and GREEN evidence

### Rust detached Replay V3 validator

The new test constructs a coherent rejected Replay V3 step whose manifest
declares PlayerId(0), whose actor is PlayerId(0), and whose complete rejected
identity equals the initial identity.

Before the production edit:

~~~
cargo test -p mtgml-replay --all-features --locked fnd_028_replay_v3_accepts_declared_zero_actor_structurally
~~~

Result: RED. The assertion received Err(RevisionDiscontinuity) solely from
the numeric zero guard; the test fixture compiled and all other identity
fields were coherent.

After the production edit: PASS, 1 test passed and 14 were filtered.

### Python detached Replay V3 validator

The equivalent Python test derives the identity from the current empty V3
golden, changes only the declared deck player and step actor to canonical
zero, and asserts canonical encode/decode parity.

Before the production edit:

~~~
python -m unittest discover -s python/tests -p test_batch_f.py -k declared_zero_actor_is_structurally_valid_and_canonical
~~~

Result: RED. The validator raised semantic.replay with replay identity
discontinuous from the numeric-zero branch.

After the production edit: PASS, 1 test ran and passed.

### Declared-zero producer path

The Batch-F characterization was retained under
fnd_028_declared_zero_state_endpoint_and_manifest_remain_valid. It continues
to prove that reset with [PlayerId(0), PlayerId(1)], EngineState validation,
bind_player(PlayerId(0)), information-state perspective zero, and manifest
validation are valid.

The new
fnd_028_declared_zero_player_is_produced_checkpointed_forked_and_replayed
test uses the authoritative environment with [PlayerId(0), PlayerId(1)] and
submits the actual pending PlayerId(0) response. It proves:

- the accepted live transition is produced by the zero endpoint;
- checkpoint state and revision advance as expected;
- an equal-input fork produces the same step and checkpoint;
- the exported Replay V3 actor is PlayerId(0);
- canonical wire encode/decode reproduces the replay;
- backend-verified replay from the initial checkpoint succeeds;
- restore followed by the same zero submission reproduces the live step,
  checkpoint, and replay.

The focused command passed with both declared-zero tests; the complete
environment package passed 66 tests.

## Fail-closed controls

The four Batch-F controls pass independently:

- fnd_028_undeclared_zero_actor: detached validation accepts the structural
  shape, while replay execution returns ActorUnavailable before trusted
  execution and the live checkpoint/replay remain equal.
- fnd_028_declared_zero_actor_must_match_pending_actor: a declared zero that
  is not the pending actor returns ActorUnavailable with no live mutation.
- fnd_028_zero_actor_wrong_player_decision_id: the existing backend decision
  identity owner returns PlayerDecisionIdentityMismatch with no live mutation.
- fnd_028_zero_actor_wrong_revision: detached validation returns
  RevisionDiscontinuity and execution is not attempted; the source
  checkpoint/replay remain equal.

These controls preserve state, RNG, allocators, knowledge, events, episode
status, counters, checkpoint identity, and replay recorder state through the
existing complete checkpoint and replay comparisons.

## Rust/Python and wire evidence

The Rust and Python detached tests accept the same V3 shape:
canonical deck player "0", canonical step actor "0", rejected-step identity
equal to the initial identity, and decision-response.v2.

The Rust environment producer emits actor "0" and round-trips through
mtgml-wire. The Python detached value emits actor "0" and round-trips through
the Python canonical codec. Existing wire fixtures remain unchanged.

Executed parity checks:

- cargo test -p mtgml-replay --all-features --locked: 15 passed;
- cargo test -p mtgml-wire --all-features --locked: 10 passed;
- Python schema parity: 12 tests passed;
- Python wire contract tests: 2 tests passed;
- generate_contracts.py --check: PASS;
- validate_schemas.py: PASS, 38 wire fixtures and 9 artifacts.

## Compatibility and information safety

This is the ADR-0050 reader-compatible validator broadening. The accepted
meaning of previously valid Replay V3 artifacts is unchanged. Inputs rejected
only by the removed numeric-zero guards had no accepted historical Replay V3
meaning. No historical V1/V2 or V3 fixture was rewritten, rehashed, relabeled,
or migrated.

PlayerId(0) is a public identity scalar under the accepted declaration
contract. Accepting it in detached structural validation exposes no hidden
state, trusted identifiers, root seed, RNG cursor, checkpoint internals, or
other-player private knowledge. The authoritative backend remains responsible
for exact declaration and pending-actor binding.

## Local verification results

The following direct checks were executed on a clean verification tree with
the implementation and test contents:

- cargo fmt --all -- --check: PASS;
- cargo check --workspace --all-targets --all-features --locked: PASS;
- cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings: PASS;
- cargo test --workspace --all-features --locked: PASS;
- Python full profile: 398 tests, OK, skipped=3 because the optional M2.H
  adapter binary was not configured;
- Ruff format/check: PASS;
- Mypy: PASS, no issues in 17 source files;
- direct fast profile: PASS;
- direct integration profile: PASS;
- direct certification profile: PASS;
- run_verification.py: PASS, freeze PASS, all 20 gates PASS, source tree
  unchanged;
- verify_archive_reproducibility.py: PASS, 682 safe files and deterministic
  archive verification;
- verify_repository.py: PASS, 682 files, 38 golden, 46 negative fixtures;
- check_rust_source_structure.py: PASS, 138 files;
- check_documentation.py: PASS, 45 ADRs and local links;
- validate_maintainer_artifacts.py: PASS, 7 artifacts;
- validate_golden_path.py: PASS;
- verify_python_toolchain.py: PASS, Python 3.13.15 and Rust 1.85.1.

The repository just wrappers were executed separately. Each of just
check-fast, check, check-all, release-candidate, and archive-check was
BLOCKED before its recipe could start because this host cannot start
/bin/bash through WSL. Direct profile results above are the supported native
fallback evidence. git diff --check: PASS.

## Lifecycle boundary

This candidate does not close FND-028. The implementation branch remains
OPEN_PENDING_REVIEW_AND_MERGE with PRE_M3_FREEZE_BLOCKER = YES. The external
review/Hosted-CI/merge sequence must record approval and merged-master
verification before Issue 164 can record FND_028 = CLOSED and
PRE_M3_FREEZE_BLOCKER = NO.

Final Foundation Closure, HRD-001..006 disposition, Pre-M3 freeze, and M3
entry remain out of scope.
