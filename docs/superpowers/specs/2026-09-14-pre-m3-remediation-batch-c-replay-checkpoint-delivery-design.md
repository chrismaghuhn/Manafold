# Pre-M3 Remediation Batch C: Replay, Checkpoint, and Delivery Closure

**Status:** evidence-backed Batch-C disposition; M3 remains unauthorized
**Base:** `85999fb4e8e1cba9dfadba6dc66276899d44bd03`
**Code evidence head before this disposition record:** `f68a926`
**Scope:** FND-021, FND-022, FND-026 only

## Ownership and disposition

### FND-021 — replay actor/request identity

- FND-021A actor binding: `RESOLVED_ON_BASE`. `execute_replay` already compares the recorded actor with the authoritative pending request before trusted execution. The existing tampered-actor test observes `ActorUnavailable`, and the source controller remains unchanged.
- FND-021B player-decision binding: `CONFIRMED` and fixed. A detached/resealed diagnostic step with the correct actor but a wrong `response.player_decision_id` previously executed as a rejected step. The replay execution boundary now compares the response identity with the reconstructed pending request and returns `PlayerDecisionIdentityMismatch` before trusted execution.
- FND-021C lower-layer identity ownership: `RESOLVED_ON_BASE`. `DecisionResponseV2::validate_for` remains the owner of response schema, state revision, answer-domain, candidate, uniqueness, canonicality, cardinality, and numeric validation. Replay adds only the missing reconstructed-request binding.

### FND-022 — replay/checkpoint status and counters

- FND-022A initial counter closure: `CONFIRMED` and fixed. `EnvironmentLimitCounters::validate` now owns the structural relation `accepted_transitions <= decisions_submitted`; both `EnvironmentCheckpointV3` and `InitialEnvironmentIdentityV3` call it. No intuitive relation was invented for event/resource/wall-clock counters.
- FND-022B status/player universe: `BLOCKED_CONTRACT_AMBIGUITY`. Duplicate outcomes are already rejected by local `EpisodeStatus::validate`; missing and foreign outcomes are accepted at the checkpoint boundary. The current normative text says “per-player outcome” and requires duplicate-free semantic keys, but does not explicitly state that a closed status must contain exactly one outcome for every `EngineState.core.players` member. No cross-component policy was guessed.
- FND-022C closed-episode trusted execution: `RESOLVED_ON_BASE` for the current synthetic production backend. Terminal and truncated checkpoints have no pending request; trusted execution returns a rejected transition and leaves checkpoint, replay, and counters unchanged. The player endpoint already returns its closed submission result.
- FND-022D deterministic counters: `RESOLVED_ON_BASE`. Accepted execution recomputes decisions, accepted transitions, and emitted rule events exactly; replay compares those values against execution. Focused and full package tests pass.
- FND-022E external/resource/wall-clock trace: `CONFIRMED` plus `SPLIT_REQUIRED`. The replay contract already requires recorded external/resource progression to be explicitly replayed/validated; the current executor carries the runtime values forward and rejects a differing recorded value as an after-identity mismatch. The open question is the authoritative application owner, ordering relative to accepted/rejected execution and technical limits, and API mechanism. No setter, hidden state, host-clock read, or schema field was added.

### FND-026 — multi-perspective products

- FND-026A current behavior: `CONFIRMED`. Production occurrence projection constructs independent valid batches for P1 and P2. The accepted endpoint returns only the actor batch; the non-actor batch is not retained or exposed by any current public endpoint.
- FND-026B complete product validation: `SPLIT_REQUIRED`. The closed slice is now fixed: every projected `ObservedEventEnvelopeV2`, including a non-actor envelope, is validated at the shared production projection boundary before the environment commit can proceed. The complete non-actor `PlayerStepV2` product is still not validated by the commit path; no durable delivery product is introduced while that open slice is being clarified.
- FND-026C non-actor delivery: `BLOCKED_CONTRACT_AMBIGUITY`. The current endpoint contract defines `submit(response) -> PlayerStep` for the submitting perspective and defines no durable or asynchronous non-actor delivery mechanism. Adding a queue, mailbox, polling method, callback, cross-player return value, or controller-global buffer would create new checkpoint, fork, replay, ordering, API, and information-safety contracts.
- FND-026D replay reprojection: `SPLIT_REQUIRED`. The focused two-perspective fixture proves only that the production projection from before/events/after is byte-stable and perspective-separated. Existing replay parity continues to pass, but no test yet executes authoritative replay and compares a reconstructed non-actor observed-event batch from its trace. This does not close EVD-010.

## Required disposition matrix

```text
TASK = PRE_M3_REMEDIATION_BATCH_C

BASE = 85999fb4e8e1cba9dfadba6dc66276899d44bd03
HEAD = 1e2a149cf4c8fa700fb540a4778fae2987c63356
BRANCH = chris/pre-m3-remediation-batch-c-replay-checkpoint-delivery
PR = https://github.com/chrismaghuhn/Manafold/pull/167

FND_021 = CONFIRMED
FND_021A_ACTOR_BINDING = RESOLVED_ON_BASE
FND_021B_PLAYER_DECISION_BINDING = CONFIRMED
FND_021C_LOWER_LAYER_IDENTITY_OWNERSHIP = RESOLVED_ON_BASE

FND_022 = SPLIT_REQUIRED
FND_022A_INITIAL_COUNTER_CLOSURE = CONFIRMED
FND_022B_STATUS_PLAYER_UNIVERSE = BLOCKED_CONTRACT_AMBIGUITY
FND_022C_CLOSED_EPISODE_EXECUTION = RESOLVED_ON_BASE
FND_022D_DETERMINISTIC_COUNTERS = RESOLVED_ON_BASE
FND_022E_EXTERNAL_COUNTER_TRACE = SPLIT_REQUIRED

FND_026 = SPLIT_REQUIRED
FND_026A_CURRENT_MULTI_PERSPECTIVE_BEHAVIOR = CONFIRMED
FND_026B_COMPLETE_PRODUCT_VALIDATION = SPLIT_REQUIRED
FND_026C_NON_ACTOR_DELIVERY = BLOCKED_CONTRACT_AMBIGUITY
FND_026D_REPLAY_REPROJECTION = SPLIT_REQUIRED

CONFIRMED = FND-021B, FND-022A, FND-026A, FND-026B envelope-validation slice, FND-022E current executor gap
REJECTED = NONE
RESOLVED_ON_BASE = FND-021A, FND-021C, FND-022C, FND-022D
BLOCKED_CONTRACT_AMBIGUITY = FND-022B, FND-022E application mechanism, FND-026C
SPLIT_REQUIRED = FND-022, FND-022E, FND-026B, FND-026D, FND-026

RED_TESTS = PASS evidence after RED: wrong request identity, impossible initial counters, malformed non-actor envelope
FOCUSED_TESTS = PASS
WORKSPACE_TESTS = PASS
FMT = PASS
CHECK = PASS
CLIPPY = PASS
LOCAL_CHECK_FAST = BLOCKED: just/WSL cannot execute /bin/bash
LOCAL_CHECK = BLOCKED: just/WSL cannot execute /bin/bash
INTEGRATION_GATES = PASS for cargo test -p mtgml-conformance --locked; wrapper integration remains BLOCKED
HOSTED_CI = PASS

PUBLIC_API_CHANGE = YES (internal/experimental Rust error and counter-validation API; no player-facing API)
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO

WORKTREE_CLEAN = PASS
REMOTE_HEAD_EQUALS_LOCAL = YES

M3_AUTHORIZED = NO
```
## Exact blocked questions

```text
FND-022B:
Does the accepted current status contract require a closed Terminal or
Truncated value to contain exactly one PlayerOutcome for every player in the
authoritative EngineState player universe, with no foreign player IDs? If yes,
the cross-component checkpoint/environment owner must enforce that relation;
if no, the contract must state the weaker local duplicate-free policy.

FND-022E:
The replay contract already requires trusted recorded
resource_units_consumed and wall_clock_elapsed_millis progression to be
explicitly replayed/validated. Which authoritative environment owner applies
that progression, in what order relative to accepted/rejected execution and
technical limit checks, and through which API mechanism? The current API does
not define that mechanism, so Batch C does not add one.

FND-026B:
The current contract requires complete per-perspective observed-event/
PlayerStep validation before commit. The envelope slice is fixed, but should
the environment construct and validate a complete non-actor PlayerStepV2
product transiently before commit, and what exact product fields are required
without creating a delivery API or durable controller state?

FND-026D:
The current evidence proves projection purity from before/events/after, but
does not execute a replay trace with a non-actor observed-event batch. A
future focused replay test must define the fixture and compare that exact
batch without closing EVD-010.

FND-026C:
What public or trusted delivery contract, if any, lets a non-acting
perspective receive a transient observed-event batch produced by an accepted
transition? The current PlayerEndpoint contract defines no such mechanism;
implementing one would be a separate checkpoint/fork/replay/API design.
```

## RED evidence

- `cargo test -p mtgml-replay --locked initial_identity_rejects_impossible_counters -- --nocapture` on the unmodified base failed with `left: Ok(())` and `right: Err(CounterProgression)`.
- `cargo test -p mtgml-environment --locked replay_rejects_wrong_player_decision_id_before_trusted_execution -- --nocapture` on the unmodified base failed because the structurally valid wrong-identity diagnostic step was accepted instead of rejected.
- `cargo test -p mtgml-environment --locked non_actor_projected_envelope_is_validated_before_commit_boundary -- --nocapture` on the unmodified base failed because the malformed non-actor envelope was returned without validation.

## Scope accounting

```text
NEW_MAGIC_SEMANTICS = NO
M3_STARTED = NO
CARD_IR_CHANGE = NO
REAL_CARD_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
HISTORICAL_REPLAY_CHANGE = NO
PUBLIC_API_CHANGE = YES (internal/experimental Rust API only)
M3_AUTHORIZED = NO
```
