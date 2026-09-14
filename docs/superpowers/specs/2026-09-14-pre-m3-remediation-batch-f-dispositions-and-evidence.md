# Pre-M3 Remediation Batch F dispositions and evidence

**Status:** implementation evidence recorded; hosted CI and PR delivery remain
pending
**Date:** 2026-09-14
**Base:** `b24bba153f2aa74bd59e8d6a872a0612ff7f76aa`
**Source head before this evidence commit:** `4de61d126a67b15b780848a3ca5e7c40ec7386d9`
**Branch:** `chris/pre-m3-remediation-batch-f-decision-observation-identity`
**PR:** pending final push

## Scope

Batch F owns FND-009, FND-013, FND-014, FND-015, FND-016A, FND-027, and
FND-028 from Issue #164. FND-016 remains split: FND-016A is the local
actor-bound DTO closure and FND-016B is deferred to EVD-005. FND-026B remains
`BLOCKED_CONTRACT_AMBIGUITY`; FND-026C remains `DEFERRED_P2`. FND-017-FND-019
and FND-029-FND-032 remain for Batch G. No EVD item is globally closed by
this batch.

No M3 work, new Magic semantics, cards, Card IR, replay/checkpoint version,
new endpoint, schema, digest, or RNG contract was added.

## Disposition matrix

```text
TASK = PRE_M3_REMEDIATION_BATCH_F
BASE = b24bba153f2aa74bd59e8d6a872a0612ff7f76aa
SOURCE_HEAD_BEFORE_EVIDENCE = 4de61d126a67b15b780848a3ca5e7c40ec7386d9
BRANCH = chris/pre-m3-remediation-batch-f-decision-observation-identity
PR = pending final push

F1_RULES_DECISION_CLOSURE = PASS
FND_009 = CONFIRMED
FND_013 = RESOLVED_ON_BASE
FND_014 = CONFIRMED

F2_OBSERVATION_PLAYER_STEP_CLOSURE = PASS
FND_015 = CONFIRMED
FND_016 = SPLIT_REQUIRED
FND_016A = CONFIRMED
FND_016B = DEFER_TO_EVD_005

F3_IDENTITY_CLOSURE = PARTIAL
FND_027 = RESOLVED_ON_BASE
FND_028 = BLOCKED_CONTRACT_AMBIGUITY

F4_CROSS_LAYER_INTEGRATION = PARTIAL

FND_009_NOOP_POLICY = LifeChanged and ObjectTapped are MUTATION_MUST_CHANGE and require from != to; ZoneTransition already requires distinct incarnations; occurrence-only event families retain their existing semantics
FND_013_BINDING_OWNER = AuthoritativeDecisionRequestV2::validate is local structural validation; validate_pending_authoritative_request owns exact scalar and resolver-backed binding; project_player_request projects an already authoritative-valid request and is not a second resolver authority
FND_014_CAPACITY_POLICY = one widened u64 rule accepts candidate_count <= 2^32 and returns typed CandidateCapacityExceeded above it before enumeration; checked u32 ordinal conversion remains the defensive boundary
FND_015_OBJECT_MOVED_POLICY = V2 ObjectMoved rejects only old_object=None and new_object=None; old-only, new-only, and both-present remain valid; V1 and wire/schema shape are unchanged
FND_016_REJECTION_MATRIX = EpisodeClosed -> non-Running/None/empty; UnavailableDecision -> Running/None/empty; StaleDecision and Invalid* -> Running/Some(current actor request)/empty; local validator does not claim pre-call parity
FND_027_FINAL_IDENTITY_POLICY = existing Rules SemanticValidationCursor owns lifecycle identity replay and complete after-state parity, excluding only next_player_decision_id; projector owns redaction and visible-sequence projection without a duplicate identity authority
FND_028_PLAYER_ZERO_POLICY = contract-blocked; executable behavior unchanged pending an accepted numbered ADR; current surfaces are recorded in the unnumbered architecture-owned candidate

ADR_CANDIDATES = docs/superpowers/specs/2026-09-14-player-id-zero-policy-adr-candidate.md
ADR_ACCEPTED = NONE

CONFIRMED = FND-009, FND-014, FND-015, FND-016A
REJECTED = NONE
RESOLVED_ON_BASE = FND-013, FND-027
BLOCKED_CONTRACT_AMBIGUITY = FND-028, inherited FND-026B
SPLIT_REQUIRED = FND-016
DEFERRED_P2 = FND-026C
```

## RED and focused evidence

The characterization commit is `99ad216b24d1f46166c1c55611873726e6c5aa54`.
On that pre-fix commit the following semantic REDs were observed:

- `tests::fnd_009_noop_life_change_is_rejected_by_the_transition_contract` and
  `tests::fnd_009_noop_object_tap_is_rejected_by_the_transition_contract`
  returned `Ok(())` instead of the expected `LifeChange`/`TapChange` errors.
- `batch_f::fnd_014_dense_candidate_paths_have_checked_u32_boundaries` was a
  `SOURCE_RED / STATIC_CHARACTERIZATION`: the source still contained the
  `expect("candidate ordering is bounded by u32")` and `index as u32` paths.
  It is not presented as a runtime panic reproduction.
- `tests::fnd_015_object_moved_requires_at_least_one_visible_identity` accepted
  the both-absent V2 event.
- `tests::fnd_016a_rejection_matrix_requires_the_correct_decision_presence`
  accepted missing decisions for actor-bound rejection codes.

Base controls already demonstrated the two resolved-on-base findings:

- `tests::fnd_013_authoritative_state_rejects_scalar_binding_mismatches`,
  `tests::fnd_013_authoritative_state_rejects_ability_binding_mismatch`, and
  `tests::fnd_013_authoritative_state_accepts_exact_binding_controls` exercise
  scalar, object, ability, and exact valid authoritative binding.
- `tests::fnd_027_final_identity_mismatch_is_rejected_by_the_rules_cursor`
  first proves `validate_engine_state(after) = PASS` for the tampered
  `next_opaque_object_id`, then proves
  `validate_transition_contract(before, result) =
  Err(TransitionViolation::OccurrencePairing)`. Supplementary extra/missing
  mapping, retired-ID, ability-mapping, and perspective-player-set cases also
  fail closed through the same transition boundary.
- `tests::fnd_028_current_player_zero_surfaces_are_characterized_without_a_policy`
  records the currently contradictory zero behavior without changing it.

The logical fix commits are:

```text
78b9a27f181d1a4df924de72f4a52572b6dbb0ca  fix: reject no-op mutation events
d61547c9ae5c46cae0d17187bd62e5ce10b24aee  fix: make candidate capacity fail closed
16c9b9e669310e79c3a3b5dd3b711fac9530bb17  fix: require visible identity in V2 object moves
02f83e740a944e473c6cff98c658ce217c9d7629  fix: close actor-bound PlayerStep rejection fields
722ec6f8359591868ea47c274e089b4072bc3055  docs: record Batch-F binding and PlayerId policy boundaries
4c4af86c84df660edc0a29c4588d2a8bd1d43785  docs: normalize Batch-F ADR candidate
e6cbab4a34ca836a0cb78058278320cbfecc80c9  test: satisfy Batch-F observation lint
3b9b5a78168b0e2122743496d8d856b6df73626e  style: align Batch-F Python lint directives
4de61d126a67b15b780848a3ca5e7c40ec7386d9  fix: keep Python PlayerStep matrix lint-clean
```

## F4 integration matrix

| Row | Required case | Executable evidence | Result |
| ---: | --- | --- | --- |
| 1 | valid decision request projection | `mtgml-environment` `synthetic_endpoint_returns_v2_surface`; conformance paired runtime acceptance | PASS |
| 2 | scalar payload-mismatched trusted binding | `mtgml-state` `fnd_013_authoritative_state_rejects_scalar_binding_mismatches` | PASS |
| 3 | resolver-backed object/ability mismatch | `mtgml-state` `fnd_013_authoritative_state_rejects_ability_binding_mismatch` and object case in scalar matrix | PASS |
| 4 | candidate capacity rejection | Rust `candidate_capacity_uses_the_full_u32_id_domain_without_allocation`; Python `_validate_candidate_capacity` test | PASS |
| 5 | valid dense assignment | Rust `dense_assignment_and_public_validation_remain_exact_for_small_inputs`; Python small dense request | PASS |
| 6 | no-op `LifeChanged` | Rules `fnd_009_noop_life_change_is_rejected_by_the_transition_contract` | PASS |
| 7 | no-op `ObjectTapped` | Rules `fnd_009_noop_object_tap_is_rejected_by_the_transition_contract` | PASS |
| 8 | valid `ObjectMoved` old-only | Observation `fnd_015_object_moved_requires_at_least_one_visible_identity` control | PASS |
| 9 | valid `ObjectMoved` new-only | Observation `fnd_015_object_moved_requires_at_least_one_visible_identity` control and lifecycle projection tests | PASS |
| 10 | invalid `ObjectMoved` neither-visible | Rust/Python shared negative fixture `observed-event-v2-object-moved-no-identity.json` | PASS |
| 11 | every actor-bound rejection code | Rust/Python `fnd_016a_rejection_matrix_requires_the_correct_decision_presence`; conformance M4 stale current-actor control | PASS for FND-016A; FND-016B deferred |
| 12 | lifecycle final identity mismatch | Rules canonical `fnd_027_final_identity_mismatch_is_rejected_by_the_rules_cursor` | PASS before projection boundary |
| 13 | PlayerId zero at characterized boundaries | Environment `fnd_028_current_player_zero_surfaces_are_characterized_without_a_policy`; ADR candidate | BLOCKED_CONTRACT_AMBIGUITY |
| 14 | accepted controls byte-compatible | Rust wire golden/constructive suite; Python wire contract suite | PASS |
| 15 | typed rejection remains nonmutating | Rules rejection matrix, FND-009 read-only assertions, conformance paired rejection parity | PASS |

## Verification

The affected package suites passed on the source head before this evidence
commit: `mtgml-model` 8, `mtgml-decision` 12, `mtgml-state` 96,
`mtgml-rules` 41, `mtgml-observation` 12, `mtgml-environment` 59,
`mtgml-wire` 10, and `mtgml-conformance` 125 tests.

The native Rust gates passed: `cargo fmt --all -- --check`, workspace
`cargo check --workspace --all-targets --all-features --locked`, workspace
Clippy with `-D warnings`, and `cargo test --workspace --all-features --locked`
(447 unit/doc test executions, 0 failures).

The pinned Python authority is `.venv\Scripts\python.exe` (Python 3.13.15):

```text
PYTHON_FULL = PASS: scripts/run_python_tests.py, 391 tests, 3 expected M2.H adapter skips
FOCUSED_PYTEST = PASS: Batch-F, wire contracts, and player API tests
SCHEMA_GATES = PASS: 38 wire fixtures and 9 maintainer artifacts
WIRE_GATES = PASS: 38 golden fixtures and 46 negative fixtures
MAINTAINER_GATES = PASS: repository, Rust structure, documentation, schemas, maintainer artifacts, toolchain, golden path, and Ruff checks
DIRECT_FAST_PROFILE = PASS: scripts/run_checks.py fast
DIRECT_INTEGRATION_PROFILE = PASS: scripts/run_checks.py integration
DIRECT_CERTIFICATION_PROFILE = PASS: scripts/run_checks.py certification
M2_D = PASS
M2_E = PASS
M2_G = PASS
M2_H = PASS
```

The direct pre-evidence archive gate passed at source head
`4de61d126a67b15b780848a3ca5e7c40ec7386d9`:

```text
sha256=d52a7920cb29aca631029b4323d51490ec86b42670ce82d9721a27598f4e49ee
```

`just archive-check`, `just check-fast`, `just check`, and `just check-all`
were executed separately and are `BLOCKED` on this Windows host because WSL
cannot start `/bin/bash`. Direct constituents are reported independently and
are not upgraded from those wrapper results.

## Compatibility and safety

```text
PUBLIC_API_CHANGE = NO (internal/experimental typed Rust error extension only)
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO

INFORMATION_SAFETY = PASS: player DTOs retain only opaque IDs and player-safe fields; no trusted candidate binding, internal IDs, RNG provenance, checkpoint identity, or hidden mapping was exposed
DETERMINISM = PASS: dense IDs use the shared semantic order and checked boundary; lifecycle validation/projection remains deterministic and read-only
```

Task 9 was performed as an inline exact-head self-review because this run was
explicitly requested inline; no subagent reviewer was started. Independent
exact-head review remains the next action. Hosted CI has not run until the
single PR is pushed.

```text
NEW_MAGIC_SEMANTICS = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
MERGE_PERFORMED = NO
```
