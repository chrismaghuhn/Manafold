# FND-028 PlayerId(0) Runtime Reconciliation Implementation Plan

**Status:** proposed for independent plan review; production implementation remains unauthorized

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task with review checkpoints. FND-028 forbids subagents; execution must be inline after the independent plan review.

**Goal:** Remove the contradictory Replay V3 zero-actor guards and prove that a declared PlayerId(0) follows accepted ADR 0050 through live execution, checkpoint/restore, fork, replay, and Rust/Python validation.

**Architecture:** Keep PlayerId as the existing canonical unsigned identity and keep declaration closure in the current state/environment owners. Change only the two numeric-zero branches in detached Replay V3 validation; retain backend pending-actor, player-decision, revision, checkpoint, manifest, and nonmutation checks. Use the existing Rust authoritative environment for the positive end-to-end path and the existing Python DTO/wire implementation for equivalent detached validation.

**Tech Stack:** Rust 1.85.1 with the locked Cargo workspace, Python 3.13.15 from .venv, canonical UTF-8 JSON, existing V3 checkpoint/replay identities, existing mtgml-wire codec, and the repository direct fast/integration/certification profiles.

---

**Date:** 2026-09-15

**Base:** 8018b61416aafbd032df58b7f5cda68dca4f07cb

**Accepted authority:** docs/adr/0050-player-id-zero-policy.md, accepted Option A

**Issue:** https://github.com/chrismaghuhn/Manafold/issues/164

## Plan gate and scope lock

This plan is Task 2 only. The implementation branch may be created only after
an independent exact-head review records:

~~~
FND_028_PLAN_REVIEW = APPROVE
PRODUCTION_IMPLEMENTATION_AUTHORIZED = YES
~~~

This Task 2 commit changes only this plan and its required process-register
entry. It does not remove either guard, add RED tests, modify production code,
modify test semantics, modify schemas, modify wire fixtures, or begin final
foundation closure.

The implementation must preserve these invariants:

- PlayerId(0) is valid only where the owning contract admits a declared player;
- None remains the absence value for optional player partitions;
- AuthoritativeReplayV3::validate() remains detached structural validation;
- backend replay remains the owner of exact pending-actor and checkpoint
  declaration binding;
- player-decision identity and state-revision continuity remain unchanged;
- invalid input reaches no trusted transition and mutates no live state;
- canonical "0" wire rendering remains unchanged;
- V1/V2 replay meaning and historical fixture bytes remain immutable;
- no M3, Magic semantics, cards, Card IR, Commander support, capabilities,
  ML, performance, or unrelated maintenance is included.

## Current source characterization

Remote master is verified by git ls-remote and git fetch as:

~~~
REMOTE_MASTER = 8018b61416aafbd032df58b7f5cda68dca4f07cb
ADR_0050 = ACCEPTED / OPTION_A
~~~

The exact current owners are:

| Surface | Current owner | Current behavior | Plan treatment |
| --- | --- | --- | --- |
| Rust detached Replay V3 validation | crates/mtgml-replay/src/v3.rs:184-192 | Rejects every step.actor.0 == 0 as ReplayValidationError::RevisionDiscontinuity before remaining step checks | Remove only the numeric-zero disjunct after RED evidence |
| Python detached Replay V3 validation | python/src/mtgml/replay.py:893-904 | Rejects every step.actor == 0 as semantic.replay / replay identity discontinuous | Remove only the numeric-zero disjunct after RED evidence |
| Rust Replay V3 tests | crates/mtgml-replay/src/tests.rs:230-415 | Builds valid V3 identities and steps but has no positive zero-actor case | Add one isolated desired-behavior RED/green test |
| Python replay tests | python/tests/test_batch_f.py | Covers Batch-F decision/observation cases but no direct V3 zero-actor validation | Add the equivalent isolated test to the existing FND-028 test module |
| Environment replay execution | crates/mtgml-environment/src/replay.rs:119-190 | Checks manifest, pending actor, player-decision ID, revisions, digests, counters, transition, and final identity | Do not change; use for positive and negative controls |
| Batch-F characterization | crates/mtgml-environment/src/tests/batch_f.rs:1-32 | Proves state/reset/endpoint/manifest zero acceptance and manually changed nonzero replay actor rejection | Preserve positives; replace obsolete detached rejection with declared-zero evidence |
| Checkpoint/fork/replay evidence | crates/mtgml-environment/src/tests/checkpoint_replay.rs; crates/mtgml-environment/src/replay_parity_tests.rs | Existing nonzero checkpoint, restore, fork, replay, projection, and nonmutation evidence | Reuse public paths from the focused Batch-F test; do not widen these general suites |
| Rust canonical wire codec | crates/mtgml-wire/src/lib.rs:220-255 | Delegates V3 replay validation, canonicalizes, and round-trips JSON | No codec change; zero replay round-trip is asserted by environment evidence |
| Python canonical wire codec | python/src/mtgml/canonical.py; python/src/mtgml/wire.py | parse_uint and uint_wire already accept canonical zero | No codec change; assert equivalent replay round-trip |
| JSON Schema | schemas/authoritative-replay.v3.schema.json; schemas/replay-manifest.v3.schema.json | uint is defined by the zero-inclusive canonical pattern | No schema edit; run schema and parity gates |
| Current and historical fixtures | wire/golden, wire/negative, wire/historical and manifests | No tracked player/actor/perspective zero fixture; historical inventory is immutable | No fixture edit; use in-memory values derived from the current V3 golden |
| FND-028 implementation evidence | docs/superpowers/specs/2026-09-15-fnd-028-player-id-zero-implementation-evidence.md | Not present on accepted master | Create only after authorized implementation evidence exists |

The detached validator is not a declaration authority. The current V3
contract separates structural replay validation from backend verification;
nonzero undeclared actors are also not rejected by the detached layer. The
implementation therefore proves undeclared and pending-mismatched zero actors
at execute_replay()/checkpoint execution, without adding a new detached
actor-universe rule.

## Planned file map

| PATH | WHY_REQUIRED | PRODUCTION / TEST / DOC | SEMANTIC_CHANGE | PUBLIC_OR_FROZEN_SURFACE |
| --- | --- | --- | --- | --- |
| crates/mtgml-replay/src/v3.rs | Remove the contradictory step.actor.0 == 0 condition | PRODUCTION | Accepted V3 structural input set broadens; field meaning is unchanged | Replay V3 experimental/freeze-candidate |
| python/src/mtgml/replay.py | Remove the Python mirror of the contradictory zero condition | PRODUCTION | Same reader-compatible validator broadening | Replay V3 experimental/freeze-candidate |
| crates/mtgml-replay/src/tests.rs | Add isolated Rust RED/green detached validation evidence | TEST | Test expectation changes to the accepted policy; no production authority | Rust replay contract test surface |
| python/tests/test_batch_f.py | Add equivalent Python RED/green and canonical round-trip evidence | TEST | Test expectation changes to the accepted policy; no production authority | Python replay/wire contract test surface |
| crates/mtgml-environment/src/tests/batch_f.rs | Preserve characterization positives and prove real declared-zero live, checkpoint, restore, fork, replay, and negative behavior | TEST | Evidence expands; environment implementation is unchanged | Trusted environment/replay evidence |
| docs/superpowers/specs/2026-09-15-fnd-028-player-id-zero-implementation-evidence.md | Record exact implementation identity, RED/green outputs, compatibility, and closure evidence | DOC | Executed evidence documentation only | Pre-M3 process evidence |
| docs/normative-document-register.v1.json | Register the plan now and the evidence record when it exists | DOC | Process metadata only | Maintainer document register |

The following current owners are not planned for modification:

- crates/mtgml-state/src: declaration/state closure already follows Option A;
- crates/mtgml-environment/src/replay.rs: backend actor/checkpoint trust
  boundary already exists;
- crates/mtgml-environment/src/tests/checkpoint_replay.rs and
  src/replay_parity_tests.rs: general parity owners are reused;
- crates/mtgml-wire/src/lib.rs: canonical codec is unchanged;
- schemas: existing uint patterns already admit zero;
- wire: no current or historical fixture rewrite is required;
- docs/adr/0050-player-id-zero-policy.md: accepted authority is unchanged.

## Task 1: Establish the exact implementation baseline

**Files:** read-only repository and remote checks.

- [ ] Verify the implementation branch starts from the actual remote master:

~~~
git fetch --no-tags origin master
git ls-remote origin refs/heads/master
git rev-parse origin/master
git status --short --branch
git show origin/master:docs/adr/0050-player-id-zero-policy.md
~~~

Expected identity:

~~~
origin/master = 8018b61416aafbd032df58b7f5cda68dca4f07cb
ADR 0050 Status = accepted
ADR 0050 Decision = Option A
~~~

- [ ] After plan approval, create the implementation branch from that master:

~~~
git switch --create chris/pre-m3-fnd-028-runtime origin/master
~~~

- [ ] Confirm the only production zero guards are the two characterized
  branches:

~~~
git grep -n -E 'step\.actor\.0 == 0|step\.actor == 0' -- crates/mtgml-replay/src/v3.rs python/src/mtgml/replay.py
~~~

Any different result requires a fresh source characterization and stops this
plan from being expanded silently.

## Task 2: Add the Rust detached Replay V3 RED test

**Files:**

- Modify: crates/mtgml-replay/src/tests.rs

- [ ] Add this helper and test after manifest_v3() and response_v3(). The
  manifest declares player zero, the rejected step preserves every identity
  field, and the desired assertion is valid after the production fix:

~~~rust
fn replay_v3_with_declared_zero_actor() -> AuthoritativeReplayV3 {
    let mut manifest = manifest_v3();
    manifest.decks[0].player = PlayerId(0);
    manifest.validate().unwrap();
    let initial = manifest.initial_identity.clone();
    let step = ReplayStepV3 {
        step_index: 0,
        actor: PlayerId(0),
        checkpoint_digest_before: initial.checkpoint_digest.clone(),
        state_revision_before: initial.state_revision,
        response: response_v3(),
        accepted: false,
        state_revision_after: initial.state_revision,
        full_state_digest_after: initial.full_state_digest.clone(),
        episode_status_after: initial.episode_status.clone(),
        environment_limit_counters_after: initial.environment_limit_counters.clone(),
        checkpoint_digest_after: initial.checkpoint_digest.clone(),
    };
    AuthoritativeReplayV3 {
        schema_version: REPLAY_FILE_SCHEMA_V3.into(),
        manifest,
        steps: vec![step],
        final_identity: initial,
    }
}

#[test]
fn fnd_028_replay_v3_accepts_declared_zero_actor_structurally() {
    let replay = replay_v3_with_declared_zero_actor();
    assert_eq!(replay.steps[0].actor, PlayerId(0));
    assert_eq!(replay.validate(), Ok(()));
}
~~~

- [ ] Run the test before touching production code:

~~~
cargo test -p mtgml-replay --all-features --locked fnd_028_replay_v3_accepts_declared_zero_actor_structurally
~~~

Expected current RED: FAIL with Err(ReplayValidationError::RevisionDiscontinuity).
The manifest, response, revision, rejected after-identity, and checkpoint
digest are independently valid; the numeric-zero branch is the only intended
failure.

## Task 3: Add the equivalent Python Replay V3 RED test

**Files:**

- Modify: python/tests/test_batch_f.py

- [ ] Add these imports:

~~~python
import json

from mtgml.canonical import canonical_json_bytes
from mtgml.replay import AuthoritativeReplayV3
from mtgml.wire import decode_canonical, encode_canonical
~~~

- [ ] Add this deterministic value builder and test. It derives identity
  fields from the current V3 empty-replay golden, changes only the declared
  deck player and step actor to canonical zero, and preserves the complete
  rejected identity:

~~~python
def _replay_v3_with_declared_zero_actor() -> dict[str, object]:
    value = json.loads(
        (ROOT / "wire" / "golden" / "authoritative-replay-empty.v3.json").read_text(
            encoding="utf-8"
        )
    )
    initial = value["manifest"]["initial_identity"]
    value["manifest"]["decks"][0]["player"] = "0"
    value["steps"] = [
        {
            "accepted": False,
            "actor": "0",
            "checkpoint_digest_after": initial["checkpoint_digest"],
            "checkpoint_digest_before": initial["checkpoint_digest"],
            "environment_limit_counters_after": initial["environment_limit_counters"],
            "episode_status_after": initial["episode_status"],
            "full_state_digest_after": initial["full_state_digest"],
            "response": {
                "answer": {"kind": "choose_number", "value": 0},
                "player_decision_id": "1",
                "schema_version": "decision-response.v2",
                "state_revision": "0",
            },
            "state_revision_after": initial["state_revision"],
            "state_revision_before": initial["state_revision"],
            "step_index": 0,
        }
    ]
    value["final_identity"] = dict(initial)
    return value


class ReplayV3PlayerZeroTests(unittest.TestCase):
    def test_declared_zero_actor_is_structurally_valid_and_canonical(self) -> None:
        raw = canonical_json_bytes(_replay_v3_with_declared_zero_actor())
        replay = AuthoritativeReplayV3.from_wire(json.loads(raw.decode("utf-8")))
        self.assertEqual(replay.steps[0].actor, 0)
        self.assertIn(b'"actor":"0"', raw)
        self.assertEqual(encode_canonical(replay), raw)
        self.assertEqual(decode_canonical("authoritative-replay.v3", raw), replay)
~~~

- [ ] Run the test before touching production code:

~~~
.\.venv\Scripts\python.exe -m unittest discover -s python/tests -p test_batch_f.py -k declared_zero_actor_is_structurally_valid_and_canonical
~~~

Expected current RED: FAIL with WireError code semantic.replay and message
replay identity is discontinuous from the Python numeric-zero branch.

## Task 4: Remove only the two contradictory production guards

**Files:**

- Modify: crates/mtgml-replay/src/v3.rs:185-192
- Modify: python/src/mtgml/replay.py:896-904

- [ ] After both RED results are recorded, make exactly these edits:

~~~rust
if step.step_index != index as u64
    || step.checkpoint_digest_before != previous.checkpoint_digest
    || step.state_revision_before != previous.state_revision
    || step.response.state_revision != previous.state_revision
{
~~~

~~~python
if (
    step.step_index != index
    or step.checkpoint_digest_before != previous.checkpoint_digest
    or step.state_revision_before != previous.state_revision
    or step.response.state_revision != previous.state_revision
):
~~~

- [ ] Do not change ReplayValidationError, response validation, counter
  validation, checkpoint digest recomputation, manifest validation, or backend
  execution.

- [ ] Rerun the Rust and Python RED tests. Both must now PASS.

## Task 5: Convert Batch-F into real declared-zero evidence

**Files:**

- Modify: crates/mtgml-environment/src/tests/batch_f.rs

- [ ] Rename the current characterization to
  fnd_028_declared_zero_state_endpoint_and_manifest_remain_valid.

- [ ] Preserve its current assertions for reset with
  [PlayerId(0), PlayerId(1)], EngineState validation, bind_player(0),
  information-state perspective zero, and Replay V3 manifest validation.

- [ ] Remove only its obsolete final block that mutates a [1,2] replay actor to
  zero and expects detached RevisionDiscontinuity. That negative becomes a
  backend-owner control in Task 6.

- [ ] Add this real producer-path test in the same file:

~~~rust
#[test]
fn fnd_028_declared_zero_player_is_produced_checkpointed_forked_and_replayed() {
    let zero = PlayerId(0);
    let players = [zero, PlayerId(1)];
    let controller = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap(),
    );
    let before = controller.checkpoint().unwrap();
    let fork = controller.fork().unwrap();
    let zero_endpoint = controller.bind_player(zero).unwrap();
    let fork_zero_endpoint = fork.bind_player(zero).unwrap();

    let live_step = zero_endpoint.submit(response(0, 0)).unwrap();
    let fork_step = fork_zero_endpoint.submit(response(0, 0)).unwrap();
    assert_eq!(
        live_step.submission,
        mtgml_observation::PlayerStepSubmissionV1::Accepted
    );
    live_step.validate().unwrap();
    fork_step.validate().unwrap();
    assert_eq!(live_step, fork_step);

    let after = controller.checkpoint().unwrap();
    assert_eq!(after.state.core.players[&zero].life, 38);
    assert_eq!(after.state.revision, StateRevision(1));
    assert_eq!(fork.checkpoint().unwrap(), after);

    let replay = controller.export_replay().unwrap();
    assert_eq!(replay.steps.len(), 1);
    assert_eq!(replay.steps[0].actor, zero);
    replay.validate().unwrap();
    let bytes = mtgml_wire::encode_canonical(&replay).unwrap();
    assert!(
        String::from_utf8(bytes.clone())
            .unwrap()
            .contains("\"actor\":\"0\"")
    );
    let decoded: AuthoritativeReplayV3 = mtgml_wire::decode_canonical(&bytes).unwrap();
    assert_eq!(decoded, replay);

    let report = controller
        .execute_replay_from_checkpoint(before.clone(), replay.clone())
        .unwrap();
    assert_eq!(report.traces.len(), 1);
    assert!(report.traces[0].transition.accepted);
    assert_eq!(report.traces[0].after, after);
    assert_eq!(report.final_checkpoint, after);
    assert_eq!(controller.checkpoint().unwrap(), after);
    assert_eq!(controller.export_replay().unwrap(), replay);

    let restored = TrustedEnvironmentController::new(
        SyntheticM1EnvironmentBackend::new(players, seed(), config(players)).unwrap(),
    );
    restored.execute_trusted_response(zero, response(0, 0)).unwrap();
    restored.restore(before).unwrap();
    let restored_step = restored.bind_player(zero).unwrap().submit(response(0, 0)).unwrap();
    assert_eq!(restored_step, live_step);
    assert_eq!(restored.checkpoint().unwrap(), after);
    assert_eq!(restored.export_replay().unwrap(), replay);
}
~~~

- [ ] Run:

~~~
cargo test -p mtgml-environment --all-features --locked fnd_028_declared_zero
cargo test -p mtgml-environment --all-features --locked fnd_028_declared_zero_state_endpoint_and_manifest_remain_valid
~~~

The test must use the actual pending zero actor as the producer. It must not
manufacture positive evidence by editing a PlayerId(1) replay.

## Task 6: Add fail-closed zero-actor controls

**Files:**

- Modify: crates/mtgml-environment/src/tests/batch_f.rs

- [ ] Add undeclared-zero evidence. Start backend() with [PlayerId(1),
  PlayerId(2)], capture before, perform one accepted PlayerId(1) transition,
  clone the replay, set only steps[0].actor = PlayerId(0), and assert:

~~~rust
tampered.validate().unwrap();
assert!(matches!(
    controller.execute_replay_from_checkpoint(before, tampered),
    Err(ControllerError::ReplayExecution(
        ReplayExecutionError::ActorUnavailable { step_index: 0 }
    ))
));
assert_eq!(controller.checkpoint().unwrap(), live_after);
assert_eq!(controller.export_replay().unwrap(), live_replay);
~~~

This is the actual declaration boundary. The complete checkpoint equality
covers state, RNG, allocators, knowledge, status, counters, and checkpoint
identity; replay equality covers recorder state.

- [ ] Add declared-but-wrong-actor evidence. Use players =
  [PlayerId(1), PlayerId(0)], so zero is declared but the pending actor is one.
  Perform the real PlayerId(1) entry transition, set only the replay step actor
  to zero, assert tampered.validate() = Ok(()), and assert backend execution
  returns ActorUnavailable { step_index: 0 } with unchanged live checkpoint and
  replay.

- [ ] Add wrong-player-decision-ID evidence. Produce a declared-zero replay,
  set only steps[0].response.player_decision_id =
  PlayerDecisionIdV1(999), assert detached validation passes, and assert:

~~~rust
Err(ControllerError::ReplayExecution(
    ReplayExecutionError::PlayerDecisionIdentityMismatch { step_index: 0 }
))
~~~

Compare the live checkpoint and replay with their pre-call values.

- [ ] Add stale-revision evidence. Produce a declared-zero replay, set only
  steps[0].response.state_revision = StateRevision(1), and assert:

~~~rust
tampered.validate() == Err(ReplayValidationError::RevisionDiscontinuity)
~~~

Do not call trusted execution after detached validation fails; assert the source
controller checkpoint and replay remain unchanged.

- [ ] Run the focused controls:

~~~
cargo test -p mtgml-environment --all-features --locked fnd_028_undeclared_zero_actor
cargo test -p mtgml-environment --all-features --locked fnd_028_declared_zero_actor_must_match_pending_actor
cargo test -p mtgml-environment --all-features --locked fnd_028_zero_actor_wrong_player_decision_id
cargo test -p mtgml-environment --all-features --locked fnd_028_zero_actor_wrong_revision
~~~

Each failure must be owned by detached revision validation, backend actor
binding, or backend player-decision binding. None is a normal player
illegality and none is an accepted transition.

## Task 7: Prove Rust/Python parity without schema or fixture changes

**Files:**

- Reuse: crates/mtgml-replay/src/tests.rs
- Reuse: python/tests/test_batch_f.py
- Do not modify: schemas, wire, crates/mtgml-wire/src/lib.rs

- [ ] Confirm both direct tests accept the same V3 shape: canonical deck player
  "0", canonical step actor "0", rejected-step identity equal to initial
  identity, and decision-response.v2.

- [ ] Confirm the Rust environment test emits actor:"0" and that
  mtgml_wire::decode_canonical reproduces the replay.

- [ ] Confirm the Python test emits actor:"0" and that Python
  decode_canonical reproduces the replay.

- [ ] Keep the current fixture corpus unchanged. Existing Rust/Python fixture
  runners remain the byte-parity authority for persisted fixtures; the zero
  replay is focused in-memory evidence.

- [ ] Run:

~~~
.\.venv\Scripts\python.exe scripts/generate_contracts.py --check
.\.venv\Scripts\python.exe scripts/validate_schemas.py
cargo test -p mtgml-wire --all-features --locked
.\.venv\Scripts\python.exe -m unittest discover -s python/tests -p test_schema_parity.py
.\.venv\Scripts\python.exe -m unittest discover -s python/tests -p test_wire_contracts.py
~~~

Expected result: existing schemas and fixture bytes remain unchanged and all
existing positive/negative fixture classifications stay green.

## Task 8: Record committed implementation evidence candidate

**Files:**

- Create: docs/superpowers/specs/2026-09-15-fnd-028-player-id-zero-implementation-evidence.md
- Modify: docs/normative-document-register.v1.json to register the evidence
  record after it exists

- [ ] Create the evidence record after the focused tests and local verification
  gates have executed on the implementation head, before the final push and
  Hosted CI. The committed record may contain the implementation base, changed
  files, RED and green commands/results, local verification results,
  compatibility classifications, and the code/test evidence. It must not
  contain its own final commit SHA, the final PR head, Hosted CI results,
  independent review, a merge commit, or merged-master state.

- [ ] Include this implementation-evidence candidate block after all required
  local evidence has executed successfully:

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
MIGRATION_REQUIRED = NO

FND_028_IMPLEMENTATION_EVIDENCE = COMPLETE_CANDIDATE
FND_028_IMPLEMENTATION_REVIEW = PENDING
FND_028 = OPEN_PENDING_REVIEW_AND_MERGE
PRE_M3_FREEZE_BLOCKER = YES
FOUNDATION_READY_FOR_M3 = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
~~~

- [ ] Describe the two production edits, link each focused test to its owner,
  record the RED and green outputs, and state that no historical fixture was
  rewritten.

- [ ] Do not put final PR-head, Hosted-CI, independent-review, merge-commit,
  or merged-master values in the committed evidence record. Record those
  external values in Task 10 after the corresponding events occur.

## Task 9: Execute the complete local verification set

Run these commands on the final implementation head after source changes are
complete. No source is changed after the archive gate.

### Focused and complete Rust/Python suites

~~~
cargo fmt --all -- --check
cargo test -p mtgml-replay --all-features --locked
cargo test -p mtgml-environment --all-features --locked
.\.venv\Scripts\python.exe scripts/run_python_tests.py --profile full
~~~

### Cross-layer and maintainer checks

~~~
.\.venv\Scripts\python.exe scripts/generate_contracts.py --check
.\.venv\Scripts\python.exe scripts/verify_repository.py
.\.venv\Scripts\python.exe scripts/check_rust_source_structure.py
.\.venv\Scripts\python.exe scripts/check_documentation.py
.\.venv\Scripts\python.exe scripts/validate_schemas.py
.\.venv\Scripts\python.exe scripts/validate_maintainer_artifacts.py
.\.venv\Scripts\python.exe scripts/validate_golden_path.py
.\.venv\Scripts\python.exe scripts/verify_python_toolchain.py
~~~

### Workspace gates

~~~
cargo check --workspace --all-targets --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
~~~

### Direct profiles

~~~
.\.venv\Scripts\python.exe scripts/run_checks.py fast
.\.venv\Scripts\python.exe scripts/run_checks.py integration
.\.venv\Scripts\python.exe scripts/run_checks.py certification
.\.venv\Scripts\python.exe scripts/run_verification.py
~~~

Integration includes full Python, Ruff, Mypy, Cargo, and maintainer artifacts.
Certification adds the pinned Python and archive reproducibility checks. Every
command must report its actual status; missing, blocked, timed-out, or failed
subprocesses are not PASS.

### Repository wrappers and final archive

~~~
just check-fast
just check
just check-all
just release-candidate
just archive-check
git diff --check
~~~

The just recipes use Bash/WSL and may be BLOCKED on this Windows host when
/bin/bash cannot start. Retain that wrapper status as BLOCKED and report direct
profile results separately. The final archive/reproducibility check runs after
all source-changing operations.

## Task 10: Exact-head review, merge, and tracker closure

- [ ] Commit the implementation as one standalone FND-028 implementation
  commit/PR containing only the two guard removals, focused tests, and
  FND-028 evidence/register updates.

- [ ] Push and inspect the exact implementation PR:

~~~
git push -u origin chris/pre-m3-fnd-028-runtime
$implementationPrNumber = gh pr view --repo chrismaghuhn/Manafold --json number --jq .number
gh pr checks $implementationPrNumber --repo chrismaghuhn/Manafold --watch
gh pr view $implementationPrNumber --repo chrismaghuhn/Manafold --json headRefOid,baseRefName,statusCheckRollup,url
~~~

The reported headRefOid must equal the reviewed implementation head and every
required Hosted check must conclude success.

- [ ] Obtain an independent exact-head review recording:

~~~
FND_028_IMPLEMENTATION_REVIEW = APPROVE
EXACT_HEAD = the SHA printed by git rev-parse HEAD
~~~

The reviewer must inspect the two production edits, all focused positive and
negative evidence, actor/declaration binding, checkpoint/fork/replay parity,
Rust/Python parity, and the no-schema/no-fixture compatibility claim.

- [ ] Merge only after independent approval and required Hosted checks pass.
  Do not mark FND-028 closed on the implementation branch before the merged
  head is available on master.

- [ ] After Hosted CI passes, independent exact-head review approves, the PR
  merges, and merged-master verification completes, record the following
  values externally in the PR/review/Issue 164 rather than in the committed
  evidence record:

~~~
FINAL_PR_HEAD = the actual headRefOid reported for the reviewed PR
HOSTED_CI = PASS
FND_028_IMPLEMENTATION_REVIEW = APPROVE
MERGE_COMMIT = the actual merge commit
MERGED_MASTER = the actual master SHA verified after merge
FND_028 = CLOSED
PRE_M3_FREEZE_BLOCKER = NO
~~~

- [ ] Set FND_028 = CLOSED only in that post-merge external record. The
  implementation branch remains OPEN_PENDING_REVIEW_AND_MERGE until
  independent approval, merge, and merged-master verification have completed.

- [ ] Stop after FND-028 closure. Do not run Final Foundation Closure, modify
  HRD-001..006, create the 53-item matrix, perform the Pre-M3 freeze, or start
  M3 in this implementation task.

## Plan self-review

| Requirement | Covered by |
| --- | --- |
| Rust RED with coherent declared-zero Replay V3 | Task 2 |
| Python RED with equivalent canonical value | Task 3 |
| Only two numeric guards removed | Task 4 |
| Real declared-zero producer transition | Task 5 |
| Existing Batch-F positives preserved | Task 5 |
| Undeclared zero and wrong pending actor rejection | Task 6 |
| Wrong player-decision ID and stale revision rejection | Task 6 |
| Rejection/nonmutation evidence | Task 6 |
| Rust/Python canonical parity | Task 7 |
| No schema or wire-fixture changes | Tasks 7 and 9 |
| Historical V1/V2 preservation and no migration | Tasks 7 and 8 |
| Checkpoint/restore/fork/replay evidence | Task 5 |
| Exact local and wrapper commands | Task 9 |
| Hosted exact-head review and merge gate | Task 10 |
| No Final Foundation Closure or M3 | Scope lock and Task 10 |

Run this scan before committing the plan:

~~~
$planScan = rg -n -i 'TODO|TBD|fill in details|implement later' docs/superpowers/plans/2026-09-15-fnd-028-player-id-zero-implementation.md | Where-Object { $_ -notmatch 'Run this scan|planScan|TODO\\|TBD' }
if ($planScan) { $planScan; throw 'plan contains a placeholder' }
~~~

The scan must return no matches. The plan contains no unowned implementation
step, no schema/fixture edit, and no second PlayerId authority.
