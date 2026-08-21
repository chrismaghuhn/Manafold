# M1.7 Two Player Endpoints Implementation Plan

**Status:** provisional

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement and evidence two permanently perspective-bound player endpoints over one synthetic M1 environment without starting M1.F or M2.

**Architecture:** Reuse `PlayerEndpointHandle` and its shared `Arc<Mutex<Box<dyn EnvironmentBackend>>>`. Add only the synthetic backend's player-safe observation, information-state, visible-decision, and submission methods; route accepted submissions through the existing `execute_response` transaction. Use an explicit provisional synthetic payload/digest input and keep observed events empty.

**Tech Stack:** Rust 1.85.1, Cargo locked workspace, `mtgml-environment`, `mtgml-observation`, `mtgml-decision`, `mtgml-state`, `mtgml-replay`, `mtgml-wire`, base64, and repository Python/Just verification scripts.

---

## File map

- Modify `crates/mtgml-environment/src/synthetic.rs`: own the player-safe synthetic projection, coarse player error mapping, and the bound-perspective submission path.
- Modify `crates/mtgml-environment/Cargo.toml`: add the already pinned workspace `base64` dependency for exact payload encoding.
- Modify `crates/mtgml-environment/src/tests.rs`: replace the M1.6 fail-closed placeholder with RED/GREEN endpoint, rejection, parity, serialization-safety, and non-default-ID tests.
- Create `docs/superpowers/specs/2026-08-21-m1-7-two-player-endpoints-design.md`: record the accepted provisional design.
- Create `docs/superpowers/plans/2026-08-21-m1-7-two-player-endpoints.md`: record this executable plan.
- Modify `docs/normative-document-register.v1.json`: register only the two provisional process artifacts.
- Create a local PR-body artifact outside the tracked source set when preparing the Draft PR; it records exact executed statuses and is not a project-status update.

## Task 1: Record and validate the M1.7 process artifacts

- [x] **Step 1: Save the design and plan.**

  The design states the shared-handle architecture, explicit synthetic payload
  boundary, error ordering, nonmutation evidence, and M1/M2 non-goals. This plan
  names every changed source file and every verification command.

- [x] **Step 2: Register both process paths.**

  Add entries with `owner_role = maintainer`, `role = process`,
  `stability = provisional`, and `change_process = process-pr`.

- [ ] **Step 3: Run the documentation checks.**

  Run:

  ```text
  python scripts/check_documentation.py
  python scripts/verify_repository.py
  ```

  Expected result: both commands exit 0 and report the new registered paths as
  existing local documents.

- [ ] **Step 4: Commit only process artifacts.**

  ```text
  git add -- docs/normative-document-register.v1.json docs/superpowers/specs/2026-08-21-m1-7-two-player-endpoints-design.md docs/superpowers/plans/2026-08-21-m1-7-two-player-endpoints.md
  git commit -m "docs: define M1.7 player endpoint semantics"
  ```

## Task 2: Add the failing endpoint evidence

**Files:** `crates/mtgml-environment/src/tests.rs`

- [ ] **Step 1: Add a canonical player-surface snapshot helper.**

  Capture canonical bytes for P1/P2 observation, information state, and visible
  decision, then capture the complete checkpoint, its typed digests/status/
  counters/state revision/pending decision/RNG cursor/allocators, the accepted
  replay, its step count, and canonical bytes. The helper must use
  `mtgml_wire::encode_canonical` for public DTOs and replay, not internal state
  serialization.

- [ ] **Step 2: Replace the old fail-closed placeholder test.**

  Replace `synthetic_backend_player_submission_surface_fails_closed_before_m1_7`
  with a test that requires:

  ```rust
  let p1 = controller.bind_player(PlayerId(1)).unwrap();
  let p2 = controller.bind_player(PlayerId(2)).unwrap();
  assert_eq!(p1.perspective(), PlayerId(1));
  assert_eq!(p2.perspective(), PlayerId(2));
  assert!(p1.visible_decision().unwrap().is_some());
  assert_eq!(p2.visible_decision().unwrap(), None);
  ```

  Require both initial observations at revision 0, valid initial information
  states, and a valid actor-free response.

- [ ] **Step 3: Add the RED shared-binding test.**

  Submit the exact valid P1 response through P2 and assert
  `Err(PlayerApiError::NoVisibleDecision)`. Compare the complete snapshot before
  and after. Submit the same response through the still-live P1 handle and
  require a valid `PlayerStep`; then require both existing handles to observe
  revision 1 and no visible decision.

- [ ] **Step 4: Add RED rejection and stale tests.**

  On fresh controllers, submit `candidate_id = "unknown_candidate"` and both
  stale variants (wrong `decision_id`, wrong `state_revision`) through P1.
  Require `InvalidSelection` or `StaleResponse` respectively and exact
  snapshot equality after each rejection.

- [ ] **Step 5: Add RED projection, ID, parity, and leakage tests.**

  Require after-state `PlayerStep::validate()`, empty observed events, `None`
  next decision, actual bound IDs `[PlayerId(7), PlayerId(9)]`, exact
  trusted-versus-endpoint checkpoint/replay equality, and serialized/rendered
  player values without seed/RNG/authoritative identity/checkpoint/replay or
  trusted error text.

- [ ] **Step 6: Run the focused tests to prove RED.**

  ```text
  cargo test -p mtgml-environment --locked
  ```

  Expected result: the new endpoint tests fail because the synthetic backend
  still returns `PlayerApiError::Unavailable`; the pre-existing M1.6 tests
  continue to pass.

## Task 3: Implement the minimum synthetic player projection

**Files:** `crates/mtgml-environment/src/synthetic.rs`, `crates/mtgml-environment/Cargo.toml`

- [ ] **Step 1: Add explicit payload/digest constants and imports.**

  Use `base64::engine::general_purpose::STANDARD` and add
  `base64.workspace = true`. Define the provisional codec identifiers:

  ```rust
  const SYNTHETIC_M1_OBSERVATION_CODEC: &str = "synthetic-m1-observation.v1";
  const SYNTHETIC_M1_INFORMATION_DIGEST_INPUT: &str = "synthetic-m1-information-state.v1";
  ```

- [ ] **Step 2: Add `require_player` and `player_observation`.**

  Require membership in `state.core.players`, then build the exact UTF-8
  payload `synthetic-m1-observation.v1|perspective=<u64>|state-revision=<u64>`.
  Set `payload_base64 = STANDARD.encode(&payload)` and
  `digest = ObservationDigest::from_canonical_bytes(&payload)`. Validate the
  envelope before returning it. Do not read zones, root seed, RNG, allocator,
  event, replay, or checkpoint data.

- [ ] **Step 3: Add `player_information_state`.**

  Reuse `player_observation`, read only the bound player's
  `public_history_length` and `private_history_length`, and hash the explicit
  UTF-8 input
  `synthetic-m1-information-state.v1|perspective=<u64>|state-revision=<u64>|public-history-length=<u64>|private-history-length=<u64>|observation-payload=<base64>`.
  Validate the resulting `InformationStateEnvelope` before returning it.

- [ ] **Step 4: Add `player_visible_decision`.**

  Return `Ok(None)` for no pending decision or a pending decision owned by
  another player. For the bound actor, clone and validate only
  `pending.request`; never return `candidate_bindings` or authoritative IDs.

## Task 4: Implement bound submission and the after-state PlayerStep

**File:** `crates/mtgml-environment/src/synthetic.rs`

- [ ] **Step 1: Implement fail-closed classification before execution.**

  Use this exact order: unknown player -> `Unavailable`; non-running status ->
  `EpisodeComplete`; no pending request or other actor -> `NoVisibleDecision`;
  invalid response shape -> `InvalidSelection`; mismatched decision ID or
  state revision -> `StaleResponse`.

- [ ] **Step 2: Reuse `execute_response`.**

  Call `self.execute_response(perspective, response)` exactly once for the
  authorized candidate. Map every `ControllerError` to `Unavailable`. If the
  transition is not accepted, return `InvalidSelection`; do not append replay
  or modify any environment field in this method.

- [ ] **Step 3: Project and validate the accepted step.**

  Build:

  ```rust
  PlayerStep {
      schema_version: PLAYER_STEP_SCHEMA.into(),
      information_state: self.player_information_state(perspective)?,
      observed_events: vec![],
      next_decision: None,
      status: transition.status,
  }
  ```

  Map projection or validation failure to `Unavailable` and require
  `step.validate()` before returning the step.

- [ ] **Step 4: Run the focused environment tests to prove GREEN.**

  ```text
  cargo test -p mtgml-environment --locked
  ```

  Expected result: all pre-existing M1.6 tests and all M1.7 endpoint tests pass.

## Task 5: Refactor only if the focused suite remains green

- [ ] **Step 1: Inspect the diff and API surface.**

  Confirm no `DecisionResponse.actor`, mutable endpoint perspective, duplicate
  binding registry, endpoint semantic state, observed-event history, public
  checkpoint/replay method, trusted diagnostic, or second transaction path was
  added.

- [ ] **Step 2: Run formatting and focused regressions.**

  ```text
  cargo fmt --all -- --check
  cargo test -p mtgml-decision --locked
  cargo test -p mtgml-observation --locked
  cargo test -p mtgml-rules --locked
  cargo test -p mtgml-state --locked
  cargo test -p mtgml-replay --locked
  cargo test -p mtgml-environment --locked
  ```

## Task 6: Run the required repository verification

- [ ] **Step 1: Run locked Rust verification.**

  ```text
  cargo test --workspace --all-features --locked
  cargo check --workspace --all-targets --all-features --locked
  cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 2: Run Python, documentation, and repository checks.**

  ```text
  python scripts/run_checks.py fast
  python scripts/check_documentation.py
  python scripts/verify_repository.py
  ```

- [ ] **Step 3: Run maintainer profiles and inspect generated evidence.**

  ```text
  just check-fast
  just check
  ```

  If the documented Windows/WSL lock wrapper is unavailable, record the exact
  command as `BLOCKED` and run native fallbacks without relabeling the blocked
  gate as `PASS`.

- [ ] **Step 4: Inspect source cleanliness and status boundary.**

  ```text
  git diff --check origin/master...HEAD
  git status --short
  ```

  Do not edit `project-sources/33_CURRENT_PROJECT_STATE.md` to claim M1 closure;
  the branch records only the M1.7 endpoint gate in the PR evidence.

## Task 7: Commit, publish, and open the Draft PR

- [ ] **Step 1: Review and stage only confirmed files.**

  ```text
  git diff --stat origin/master...HEAD
  git diff -- docs/normative-document-register.v1.json docs/superpowers/specs/2026-08-21-m1-7-two-player-endpoints-design.md docs/superpowers/plans/2026-08-21-m1-7-two-player-endpoints.md crates/mtgml-environment/Cargo.toml crates/mtgml-environment/src/synthetic.rs crates/mtgml-environment/src/tests.rs
  git add -- docs/normative-document-register.v1.json docs/superpowers/specs/2026-08-21-m1-7-two-player-endpoints-design.md docs/superpowers/plans/2026-08-21-m1-7-two-player-endpoints.md crates/mtgml-environment/Cargo.toml crates/mtgml-environment/src/synthetic.rs crates/mtgml-environment/src/tests.rs Cargo.lock
  git commit -m "feat: bind two synthetic player endpoints"
  ```

- [ ] **Step 2: Verify the final commit identity.**

  ```text
  git rev-parse HEAD
  git show --stat --oneline --decorate HEAD
  ```

- [ ] **Step 3: Push and create one Draft PR without merging.**

  ```text
  git push --set-upstream origin chris/m1-7-two-player-endpoints
  gh pr create --draft --base master --head chris/m1-7-two-player-endpoints --title "M1.7: bind two synthetic player endpoints" --body-file m1-7-pr-body.md
  ```

  Use `Closes #26` only when the executed focused evidence supports
  `MULTI_PLAYER_ENDPOINT_BINDING = PASS`. The body must include starting SHA,
  final SHA, changed files, endpoint behavior, nonmutation/parity evidence,
  exact commands, and a PASS/FAIL/BLOCKED/NOT_RUN table. State explicitly that
  observed events remain empty for M1.7 and that M1.F/M1 closure remain
  `NOT_RUN`. Do not merge.

## Plan self-review

- The design covers every required endpoint method, error mapping, shared-state
  proof, non-default IDs, stale/invalid nonmutation, PlayerStep validation,
  capability separation, and the M1/M2 boundary.
- No public wire schema, decision response actor, checkpoint/replay contract,
  generated vocabulary, authoritative event projection, or project closure
  status is changed.
- The only production dependency is the already pinned workspace `base64`
  crate, and the only authoritative execution path remains `execute_response`.
