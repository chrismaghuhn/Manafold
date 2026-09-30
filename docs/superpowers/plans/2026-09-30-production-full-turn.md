# Production Full Turn Implementation Plan

**Status:** ACCEPTED by the owner on 2026-09-30 with three amendments (discard to hand size, random smoke gate, text pins on the Plan B list); amended the same day during Task 3 by owner decision: native turn progression instead of a bridge to the old kernel. IMPLEMENTED on branch `feat/production-full-turn` (Tasks 1–9 plus the final-review fixes: player-step.v4 schema sync and the CR 117.3c/117.4 pass succession; PR review fixes: the CR 704.5b empty-library loss with its own capability, hand bounds checked before play, exact UntapCompleted claims, and turn requests validated against their rules).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The production V8 runtime (`BasicLandEnvironmentRuntimeV8`) plays complete turns — untap, upkeep, draw, main, an empty combat, end, cleanup with discard to hand size, next turn — for two players with basic lands, and a random-vs-random smoke test proves 30-turn games run deterministically.

**Architecture:** The production path gets a native, V3-typed turn progression for the land-only slice. A new `turn_progression` module in `mtgml-rules` handles passing priority and every turn-based action (step changes, untap, draw, the combat skeleton, cleanup with discard, turn change) directly on `EngineStatePartsV3` and emits the same legacy rule events the old kernel emits. Land plays and mana abilities keep using the existing basic-land path. The old full-turn kernel (`MagicRulesKernel`, `magic.rs`) cannot run states with lands or mana abilities — its support profile admits only synthetic creatures (`state_based_actions.rs:199-265`) — so it is not called from production; it is the differential oracle for turn structure on land-free states. (The first version of this plan bridged to the old kernel; Task 3 found that profile and the owner chose the native path. The abandoned bridge is on branch `wip/turn-bridge-blocked`.)

**Tech Stack:** Rust workspace (`mtgml-card-ir`, `mtgml-rules`, `mtgml-state`, `mtgml-environment`), Python check scripts.

**Spec:** No separate spec. Behavior oracles: the predecessor test suite (`crates/mtgml-environment/src/tests/magic_rules_production.rs`, `crates/mtgml-rules/src/tests/magic_turn_structure.rs`) and the old kernel itself, run side by side on land-free states (D11). Decisions are pinned in the *Decisions* section below.

## Global Constraints

- Priority order: correctness → determinism → information safety → decision completeness → replayability.
- Same admission + seed + responses ⇒ identical states, events, deltas, checkpoints, and player steps (direct, restored, forked, replayed).
- Player products (`PlayerStepV4`) never contain `GameObjectId`, `DecisionId`, RNG state, or hidden card identities.
- Every player choice is an explicit decision; nothing is chosen heuristically or randomly.
- Unsupported rules return a typed error and leave every byte of runtime state unchanged.
- No card-name dispatch. No project labels (`M3`, `G0j`, `Phase`, `Block`) in new identifiers.
- Formats change in place (AGENTS.md §4): no new `V<n+1>` types; goldens/KATs that change are updated in the same commit.
- The predecessor runtime, the old kernel and their tests stay green and unchanged (beyond Task 1's admission-derived permissions) — they are the oracle. The only shared addition is `SelectedZoneTransitionKind::HandToOwnerGraveyard` (Task 6). Deleting the predecessor runtime is Plan B.
- Turn rules for the production path live in `turn_progression.rs`. It may reuse state-level primitives (`turn_structure::temporal_successor`, `turn_structure::derive_ordinary_untap_affected_objects`, `zone_incarnation::apply_selected_zone_transition_in_workspace`) but never the old kernel's support-profile validators or `MagicRulesKernel`.
- From Task 9 on, every PR keeps the random-vs-random smoke test green.
- Production evidence = integration tests gated `#![cfg(not(feature = "historical-conformance-runtime"))]`, run with `cargo test -p mtgml-environment --test <name> --locked`.
- Commit before running `scripts/run_checks.py` (some gates fail on a dirty checkout). On Windows call `.venv/Scripts/python.exe scripts/run_checks.py fast|integration`; `just` points at `.venv/bin/python`.

## Decisions

- **D1 — Game rules are admitted by the game, not by cards.** `admit_executable_profile_v1` adds a fixed root set `MAGIC_GAME_RULE_ROOTS` (turn-structure, basic-priority, draw-card, combat-phase, declare-attackers, cleanup-reset; versions from `cards/capabilities/registry.json`) to the card-derived roots. Combat blockers/damage are *not* admitted yet.
- **D2 — Kernel permissions come from the admission.** For `MagicKernelProfile::ExecutableBasicLand`, `allows_draw_card`, `allows_combat_attackers`, `allows_cleanup_reset` consult `admission_has` (like `allows_turn_structure` already does). `allows_combat_blockers` / `allows_combat_damage` stay false for this variant.
- **D3 — Passing priority runs the native turn progression.** `execute_magic_response_v4` handles `PassPriority` and the attacker declaration natively; `PlayLand` / `ActivateManaAbility` go to `execute_basic_land_response_v4` unchanged. The basic-land draft's own pass handling (`priority_window_after_second_pass`) is no longer reached from V4.
- **D4 — One response = one V3 step.** The pass plus every turn-based action up to the next decision (or a failure) form one transition: `StateRevision` +1, one `StateDeltaV3`, all events stamped with the step revision (`docs/EXECUTION_MODEL.md:53`).
- **D5 — Decision identity is allocated per request** exactly like `install_basic_land_request_v4`: next `DecisionId`, next `PlayerDecisionIdV1`, a `DecisionCreated` event, a `PendingRequestChanged` operation; the answered request gets `DecisionCleared`.
- **D6 — Card-rules state follows the turn.** Whenever `core.position` changes, every non-empty mana pool empties (`ManaPoolChanged`, cause `Emptied`). Whenever `core.turn_number` changes, `turn_history` becomes exactly what `validate_turn_history_delta` (`delta_v3.rs:1031`) requires: new turn number, all player histories default, `target_occurrences` and `once_ability_used` empty.
- **D7 — Untap is covered by `UntapCompleted`.** V3 delta validation treats `UntapCompleted { affected_objects }` as covering tapped→untapped for exactly those objects. The kernel is not changed.
- **D8 — Observed events follow perspective occurrences.** In the successor projector, `PerspectiveOccurrence` events project through their per-perspective policy; pure turn-structure/priority events (`TurnPositionChanged`, `TurnNumberChanged`, `ActivePlayerChanged`, `PriorityChanged`, `UntapCompleted`, `EmptyCombatStepsSkipped`) produce no observed envelope — the observation snapshot already carries turn, position, priority and active player. A library→hand `ZoneTransition` never uses the hard-coded `reveals_new: true` path.
- **D9 — Out of scope, fails closed:** creatures in play, blockers, combat damage, SBA graveyard order, drawing from an empty library (`TurnProgressUnsupported`; the CR 704.5b loss comes with Plan B's SBAs), spells.
- **D10 — Discard to hand size is an explicit decision (CR 514.1).** At cleanup, if the *active* player's hand exceeds 7, the native progression creates `ChooseMany { minimum: n, maximum: n }` with n = hand size − 7, one `SelectObject` candidate per card in that player's hand, visible to the acting player only. V4 purpose: new `DecisionPurposeV4::HandSizeDiscard` (wire tag `hand_size_discard`, appended; existing tags unchanged). The answer moves exactly the chosen cards hand → owner graveyard through a new `SelectedZoneTransitionKind::HandToOwnerGraveyard` (graveyard is public, so discarded cards are revealed to both players; kept cards are not); then the turn ends as usual. The non-active player never discards. The old kernel keeps its fail-closed hand-size behavior.
- **D11 — The old kernel is the oracle, not a dependency.** Production code never calls `MagicRulesKernel`. A test-only harness drives it and the native progression side by side on land-free states and compares the turn skeleton after every response: position, turn number, active player, priority holder, pending decision purpose and actor, hand/library/graveyard counts.
- **D12 — Smoke policy sees only what a player sees.** The random policy chooses from the `PlayerDecisionRequestV4` returned by the player-facing `visible_decision`, never from trusted state. Its PRNG (SplitMix64, defined in the test) is seeded from the game seed and is independent of the engine RNG. Trajectory = ordered list of (actor, canonical `DecisionResponseV3` bytes, canonical `PlayerStepV4` bytes for both players, after-checkpoint digest).

- **D13 — Slice support profile.** The native progression runs only when there are exactly two players, every battlefield object is an admitted basic land (`S1QueryAuthority`), and there are no foundation sources, counters, attachments, temporary effects, triggers, delayed effects, continuations or stack objects; otherwise `TurnProgressUnsupported`. Under this profile no state-based action can apply (no creatures, no damage, life never changes), so none is evaluated; the empty-library draw fails closed (D9).
- **D14 — Turn skeleton mirrors the oracle.** Step order from `temporal_successor`; no priority in untap and cleanup (CR 502.4, 514.3); the starting player skips the turn-1 draw (CR 103.8a); at declare attackers the active player gets an `AttackerDeclaration` request with no candidates, as the oracle does — if V4 cannot represent a zero-candidate `ChooseMany`, stop and report; an empty declaration skips declare blockers and combat damage (CR 508.8).

## Review Focus

1. **Opponent learns hidden cards.** Two states differing only in the drawing player's top library card — or only in a card the discarding player *keeps* — must give the opponent byte-identical `PlayerStepV4` → Task 4, Task 6, Task 7.
2. **Mana survives a step change.** Mana made in upkeep must be gone in draw → Task 3.
3. **Land entitlement not reset.** A player who played a land on turn 1 must be offered `PlayLand` in their next main phase → Task 3, Task 7.
4. **Replay/restore/fork divergence after forced progress.** The recorded step must replay to the same checkpoint digest, a fork mid-game must continue identically, and the same seed must give the same trajectory → Task 7, Task 9.
5. **Partial commit on a rules failure or bad answer.** Empty library at draw, or a discard answer with the wrong number of cards, must be rejected with runtime state, replay length, and checkpoint unchanged → Task 3, Task 6, Task 7.

---

### Task 1: Admit game rules for the executable profile

**Files:**
- Modify: `crates/mtgml-card-ir/src/preflight.rs` (`admit_executable_profile_v1`, :261)
- Modify: `crates/mtgml-rules/src/program_kernel.rs:130-162` (`for_executable_profile` root check)
- Modify: `crates/mtgml-rules/src/magic.rs:132-150` (`allows_*`)
- Modify: every fixture that pins the basic-land closure or its derived ids — at least `persistence/golden/content-contract-basic-land-v1-kat.v1.json`, `python/tests/test_m4_phase2_successor_vectors.py:398`, `schemas/examples/replay-manifest-v8.json`, the semantic manifests built in `crates/mtgml-environment/tests/current_successor_api.rs:241` and `basic_land_runtime_v8.rs:560`.

**Interfaces:**
- Produces: `pub const MAGIC_GAME_RULE_ROOTS: &[(&str, &str)]` in `mtgml_card_ir` (key, version). `ExecutableProfileAdmissionV1::direct_requirement_roots()` = card roots ∪ game roots.

- [ ] **Step 1: Write failing tests**

```rust
// crates/mtgml-card-ir/src/preflight.rs tests
#[test]
fn executable_admission_adds_game_rule_roots_to_card_roots() {
    let admission = /* the Mountain+Plains admission used by the existing admit_executable_profile_v1 tests */;
    let roots: BTreeSet<&str> = admission.direct_requirement_roots().iter().map(|r| r.key.as_str()).collect();
    assert_eq!(roots, BTreeSet::from(["rules/basic-land-mana", "rules/land-play", "rules/mana-pool",
        "rules/turn-structure", "rules/basic-priority", "rules/draw-card",
        "rules/combat-phase", "rules/declare-attackers", "rules/cleanup-reset"]));
    let resolved: BTreeSet<&str> = admission.resolved_capabilities().iter().map(|r| r.key.as_str()).collect();
    assert!(!resolved.contains("rules/declare-blockers"));
    assert!(!resolved.contains("rules/combat-damage"));
}

// crates/mtgml-rules/src/magic.rs tests
#[test]
fn executable_profile_permissions_follow_admission() {
    let kernel = MagicRulesKernel::from_executable_admission(/* admission above */);
    assert!(kernel.profile.allows_draw_card() && kernel.profile.allows_combat_attackers() && kernel.profile.allows_cleanup_reset());
    assert!(!kernel.profile.allows_combat_blockers() && !kernel.profile.allows_combat_damage());
}
```

Also: `for_executable_profile` rejects an admission whose roots are the old three-root set (`InvalidExecutableAdmission`).

- [ ] **Step 2: Run to verify they fail** — `cargo test -p mtgml-card-ir --locked executable_admission_adds_game_rule_roots` and `cargo test -p mtgml-rules --locked --lib executable_profile_permissions_follow_admission` → FAIL (roots are the three card roots).
- [ ] **Step 3: Implement D1 + D2.** `for_executable_profile` accepts exactly card roots ∪ game roots. Regenerate/update every pinned closure and id in the same change (the existing KAT with a six-capability closure is already stale — fix it here).
- [ ] **Step 4: Run** the two tests, then `cargo test -p mtgml-card-ir -p mtgml-rules --locked` and `cargo test -p mtgml-environment --test current_successor_api --locked` → PASS.
- [ ] **Step 5: Commit** — `git commit -m "feat: admit turn, draw, combat-phase and cleanup rules for executable games"`

### Task 2: V3 delta coverage for untap

**Files:**
- Modify: `crates/mtgml-state/src/delta_v3.rs:458-480` (tap-state coverage)

**Interfaces:**
- Produces: `StateDeltaV3` validation accepts tapped→untapped changes covered by `SemanticDeltaOperation::UntapCompleted { affected_objects }` (D7).

- [ ] **Step 1: Write failing tests** in `delta_v3.rs` tests: `untap_completed_covers_exactly_its_affected_objects` (two tapped permanents of the active player untapped, one `UntapCompleted` listing both → `Ok`), and `untap_completed_does_not_cover_unlisted_objects` (a third object also untapped but not listed → `Err(DeltaApplicationV3Error::UncoveredMutation)`).
- [ ] **Step 2: Run** `cargo test -p mtgml-state --locked untap_completed` → first FAILS.
- [ ] **Step 3: Implement D7.**
- [ ] **Step 4: Run** `cargo test -p mtgml-state --locked` → PASS.
- [ ] **Step 5: Commit** — `git commit -m "feat: cover untap in V3 state deltas"`

### Task 3: Native turn progression for the land-only slice

**Files:**
- Create: `crates/mtgml-rules/src/turn_progression.rs` (module, `#[cfg(test)] mod tests`, `#[cfg(test)] mod oracle` for D11)
- Modify: `crates/mtgml-rules/src/lib.rs` (module, exports)
- Modify: `crates/mtgml-rules/src/basic_land.rs:177-192` (`BasicLandTransitionError::TurnProgressUnsupported`)

**Interfaces:**
- Consumes: Task 1 admission (keys `rules/draw-card`, `rules/declare-attackers`, `rules/cleanup-reset` gate the matching steps; a content-only admission fails closed at those boundaries), Task 2 untap coverage; `selected_basic_land_action_v4`, `execute_basic_land_response_v4`, `derive_basic_land_candidates_v4`, `validate_basic_land_pending_request_v4` (`basic_land_v4.rs`), `S1QueryAuthority::for_object`, `turn_structure::temporal_successor`, `turn_structure::derive_ordinary_untap_affected_objects`, `zone_incarnation::apply_selected_zone_transition_in_workspace`, `StateDeltaV3::between_structural_only`, `events_v3::validate_event_delta_state_v3_structural_only`.
- Produces:
  - `pub fn execute_magic_response_v4(admission: &ExecutableProfileAdmissionV1, state: &EngineStatePartsV3, actor: PlayerId, response: &DecisionResponseV3, status: &EpisodeStatus) -> Result<BasicLandTransitionProductV4, BasicLandTransitionError>` - the single V4 entry point (D3-D6, D13, D14).
  - `pub fn validate_magic_pending_request_v4(admission: &ExecutableProfileAdmissionV1, state: &EngineStatePartsV3, status: &EpisodeStatus) -> Result<(), BasicLandCandidateError>` - accepts exactly the requests the progression would create for this state (priority windows via `validate_basic_land_pending_request_v4`, the attacker declaration, and from Task 6 the discard).
  - `BasicLandTransitionError::TurnProgressUnsupported`.

Fixtures: start from `basic_land::s1_b_state_with_two_lands_fixture()` (P1 priority in precombat main, Mountain + Plains in hand, one Mountain with its mana ability on P1's battlefield) as V3 state, add face-down library cards with physical ids for both players, install P1's request. Drive real turns by responding; every accepted product must pass `validate_event_delta_state_v3_structural_only` and `apply_structural_only`.

- [ ] **Step 1: Write failing tests** (in `turn_progression.rs`):

| Test | Setup -> action | Assert |
|---|---|---|
| `first_pass_only_transfers_priority` | P1 passes in precombat main | same position; request actor P2, purpose `PriorityAction`, Pass offered, no `PlayLand` |
| `main_passes_open_beginning_of_combat_priority` | both pass in precombat main | position `BeginningOfCombat`, request actor P1 |
| `mana_pools_empty_when_the_step_changes` | P1 taps the Mountain, both pass | P1 pool empty at `BeginningOfCombat`; `ManaPoolChanged { cause: Emptied }` event |
| `beginning_of_combat_passes_reach_empty_attacker_declaration` | both pass at `BeginningOfCombat` | purpose `AttackerDeclaration`, `ChooseMany{0,0}`, no candidates, actor P1 |
| `empty_attack_declaration_ends_combat_without_damage` | answer with no objects | no `BlockersDeclared`/`CombatDamageDealt`; next request is a priority window as in `explicit_empty_attack_skips_blockers_and_damage_then_ends_combat` |
| `end_step_passes_run_cleanup_and_open_next_upkeep` | P1 taps the Mountain turn 1; both pass in end step | turn 2, active P2, priority P2 in upkeep, P1's Mountain still tapped |
| `upkeep_passes_draw_one_card_for_the_active_player` | both pass in turn-2 upkeep | P2 hand +1, library -1, position `Draw`, request actor P2 |
| `land_play_entitlement_resets_on_new_turn` | P1 plays a land turn 1; drive to turn 3 precombat main | `PlayLand` offered; P1's permanents untapped; `turn_history.turn_number == 3` |
| `draw_from_empty_library_fails_closed` | empty libraries; both pass in turn-2 upkeep | `Err(TurnProgressUnsupported)` |
| `content_only_admission_stops_at_the_draw_boundary` | same game, content-only admission (7-closure) | passing into turn 2's draw returns `Err(TurnProgressUnsupported)` |
| `unsupported_state_fails_closed` | a foundation creature on the battlefield | first pass returns `Err(TurnProgressUnsupported)` |
| `turn_skeleton_matches_old_kernel_on_land_free_states` (oracle) | land-free twin states (hands and libraries with physical cards, empty battlefield), both driven by passing for three turns | skeletons equal after every response (D11) |

- [ ] **Step 2: Run** `cargo test -p mtgml-rules --locked --lib turn_progression` -> FAIL (module missing).
- [ ] **Step 3: Implement** D3-D6, D13, D14 in `turn_progression.rs`: priority passing (active with 0 passes -> non-active with 1; non-active with 1 -> step ends), step end (mana empties, `TurnPositionChanged`), turn-based actions per step, cleanup -> next turn (`ActivePlayerChanged`, `TurnNumberChanged`, untap via `UntapCompleted`, turn-history reset), request creation per D5. Emit the legacy event kinds the oracle emits, wrapped in `AuthoritativeRuleEventKindV3::Existing`.
- [ ] **Step 4: Run** `cargo test -p mtgml-rules --locked --lib` -> PASS; `cargo test --workspace --all-features --locked` -> PASS.
- [ ] **Step 5: Commit** - `git commit -m "feat: native turn progression for the land-only slice"`

**Stop and report** if: V4 cannot represent a zero-candidate `ChooseMany`; a turn-structure event the oracle emits fails V3 event/delta validation for a reason other than untap; the oracle and the native skeleton disagree on a step order the Comprehensive Rules leave no choice about.

### Task 4: Successor observation for turn events and draws

**Files:**
- Modify: `crates/mtgml-environment/src/successor_projection.rs:554-601`
- Test: `crates/mtgml-environment/src/tests/` (new file `successor_turn_projection.rs`, registered in `src/tests.rs`)

**Interfaces:**
- Consumes: Task 3 products (event shapes).
- Produces: `project_successor_player_steps_v4` no longer returns `UnsupportedObservedEvent` for D8 turn events or kernel draws.

- [ ] **Step 1: Write failing tests:**
  - `turn_structure_events_produce_no_observed_envelope` — a Task 3 end-step→upkeep product projects for both players without error; observed events contain no turn-structure entries; each observation shows the new turn, position, active player and priority holder.
  - `drawing_player_sees_the_drawn_card` — after a Task 3 draw product, the active player's step has one `ObjectMoved { from: Library, to: Hand }` whose `new_object` is `Some` and whose object appears with its face in the observation.
  - `opponent_step_after_draw_is_identical_for_different_top_cards` — two before-states identical except the active player's top card (Mountain vs Plains): the opponent's `PlayerStepV4` canonical bytes are equal.
- [ ] **Step 2: Run** `cargo test -p mtgml-environment --locked --lib successor_turn_projection` → FAIL (`UnsupportedObservedEvent`).
- [ ] **Step 3: Implement D8.**
- [ ] **Step 4: Run** `cargo test -p mtgml-environment --locked` → PASS.
- [ ] **Step 5: Commit** — `git commit -m "feat: project turn and draw events for V4 players"`

### Task 5: `HandSizeDiscard` decision purpose (contract)

**Files:**
- Modify: `crates/mtgml-decision/src/v4.rs:66` (`DecisionPurposeV4`) and its domain/answer/visibility tables (:112, :158, :202) and the profile-dependent purpose list (:1193)
- Modify: `crates/mtgml-state/src/persisted_v7.rs:1858` (encode/decode `hand_size_discard`)
- Modify: `schemas/player-decision-request.v4.schema.json` (purpose enum/const near :926), `python/src/mtgml/decision_v4.py` (:62, :78, :1051, :1127, :1232)
- Test: `crates/mtgml-decision/src/v4.rs` tests, `python/tests/test_decision_v4.py`, `python/tests/test_schema_parity.py`; add one positive and one negative example under `schemas/examples/` following the existing `attacker_declaration` examples

**Interfaces:**
- Produces: `DecisionPurposeV4::HandSizeDiscard` — domain `ChooseMany { minimum == maximum ≥ 1 }`, candidates `SelectObject` only, visibility acting player only (D10).

- [ ] **Step 1: Write failing tests:** Rust `hand_size_discard_requires_exact_choose_many_over_objects` (accepts `ChooseMany{1,1}` with `SelectObject` candidates; rejects `minimum != maximum`, `minimum == 0`, and a `PassPriority` candidate); persisted round-trip of a V3 state whose pending request has this purpose; Python mirrors of the same accept/reject cases; schema accepts the positive and rejects the negative example.
- [ ] **Step 2: Run** `cargo test -p mtgml-decision -p mtgml-state --locked hand_size_discard` and `.venv/Scripts/python.exe -m pytest python/tests/test_decision_v4.py -k hand_size_discard` → FAIL.
- [ ] **Step 3: Implement** the purpose in Rust, persistence, schema and Python together.
- [ ] **Step 4: Run** the same commands plus `.venv/Scripts/python.exe scripts/validate_schemas.py` → PASS.
- [ ] **Step 5: Commit** — `git commit -m "feat: add hand-size discard decision purpose"`

### Task 6: Discard to hand size at cleanup

**Files:**
- Modify: `crates/mtgml-rules/src/zone_incarnation.rs:21` (`SelectedZoneTransitionKind::HandToOwnerGraveyard`, per-perspective reveal of the discarded card)
- Modify: `crates/mtgml-rules/src/turn_progression.rs` (cleanup creates the D10 request; its answer discards, then the turn ends; `validate_magic_pending_request_v4` accepts it)
- Modify: `crates/mtgml-environment/src/successor_projection.rs` only if the Task 4 projection does not already cover the discard occurrence

**Interfaces:**
- Consumes: Task 3 progression, Task 4 projection, Task 5 purpose.
- Produces: `execute_magic_response_v4` returns a `HandSizeDiscard` next decision at cleanup and accepts its answer.

- [ ] **Step 1: Write failing tests** (progression-level in `turn_progression.rs`, projection-level in `successor_turn_projection.rs`):

| Test | Setup -> action | Assert |
|---|---|---|
| `cleanup_with_eight_cards_asks_active_player_to_discard_one` | active hand 8; both pass in end step | next request purpose `HandSizeDiscard`, `ChooseMany{1,1}`, 8 candidates, actor = active player; turn not yet advanced |
| `cleanup_with_ten_cards_asks_for_three` | active hand 10 | `ChooseMany{3,3}` |
| `discard_moves_chosen_card_then_next_turn_starts` | answer with one card | that card in owner graveyard (new incarnation), hand 7, turn +1, next active player has upkeep priority |
| `discard_with_wrong_count_is_rejected_without_mutation` | answer with 0 or 2 cards | `Err(InvalidSelection)`; state bytes unchanged |
| `non_active_player_never_discards` | opponent hand 8, active hand 7 | no discard request; turn advances (same behavior as `bounded_cleanup_accepts_active_hand_seven_and_ignores_opponent_hand_eight`) |
| `opponent_sees_discarded_card_but_not_kept_cards` | two states differing only in one *kept* card | opponent `PlayerStepV4` bytes equal; the discarded card is visible to the opponent in the graveyard |

- [ ] **Step 2: Run** `cargo test -p mtgml-rules --locked --lib turn_progression` and `cargo test -p mtgml-environment --locked --lib successor_turn_projection` -> FAIL.
- [ ] **Step 3: Implement D10** in `zone_incarnation.rs` and `turn_progression.rs`.
- [ ] **Step 4: Run** `cargo test -p mtgml-rules --locked --lib` and `cargo test --workspace --all-features --locked` -> PASS.
- [ ] **Step 5: Commit** - `git commit -m "feat: discard to maximum hand size at cleanup"`

### Task 7: V8 runtime plays full turns (production path)

**Files:**
- Modify: `crates/mtgml-environment/src/basic_land_runtime_v8.rs` (`submit` :200, `execute_replay` :359, `visible_decision` / `information_state`: call `execute_magic_response_v4` / `validate_magic_pending_request_v4`)
- Modify: `crates/mtgml-environment/src/checkpoint_v8.rs:50-132` only if `new_for_basic_land_profile` rejects Task 3 requests (use `validate_magic_pending_request_v4`)
- Create: `crates/mtgml-environment/tests/common/mod.rs` (move `basic_land_admission`, `add_object`, `basic_land_state`, `checkpoint_identity`, `replay_manifest` out of `current_successor_api.rs`; add `two_player_land_game`)
- Create: `crates/mtgml-environment/tests/production_turn.rs` (`#![cfg(not(feature = "historical-conformance-runtime"))]`)

**Interfaces:**
- Consumes: Tasks 1–6.
- Produces: `BasicLandEnvironmentRuntimeV8::submit` accepts responses at every priority window, at the attacker declaration and at the hand-size discard; Task 3 errors map to the existing `ServiceUnavailable` rejection path. `tests/common/mod.rs` exports `two_player_land_game(library_size: usize, hand_size: usize, seed: u64) -> BasicLandEnvironmentRuntimeV8` (library composition Mountain/Plains derived from `seed`) for Task 9.

A scripted driver in the test file answers every decision: active player in a main phase plays the first offered `PlayLand`; the attacker declaration is answered with no objects; a discard picks the first candidates; everything else passes.

- [ ] **Step 1: Write failing tests** in `production_turn.rs`:
  - `two_players_complete_a_turn_and_reach_the_next_upkeep` — from turn 1 PrecombatMain to turn 2 Upkeep priority for P2.
  - `ten_turns_of_land_drops_are_deterministic` — `two_player_land_game(10, 3, 1)`, run to turn 11 Upkeep; each player's battlefield land count = initial count + 5 (one land in each of their five turns); `export_replay` + `execute_replay` reproduces the final checkpoint digest; a `fork` taken at turn 4 and driven with the same responses reaches the same final digest; `restore` of the turn-4 checkpoint likewise.
  - `opponent_never_learns_drawn_cards_end_to_end` — two games identical except library order: every P2 step during P1's turns is byte-identical.
  - `empty_library_draw_leaves_runtime_unchanged` — library size 0: the submit that would draw returns an error; checkpoint digest, replay length and pending decision are unchanged.
  - `hand_size_discard_through_the_player_endpoint` — a player who never plays lands reaches cleanup with 8 cards, receives a `HandSizeDiscard` request through `visible_decision`, answers it, and the game continues into the next turn.
- [ ] **Step 2: Run** `cargo test -p mtgml-environment --test production_turn --locked` → FAIL (stops at beginning of combat).
- [ ] **Step 3: Implement** the runtime wiring; move helpers into `tests/common/mod.rs` and point `current_successor_api.rs` at them.
- [ ] **Step 4: Run** `cargo test -p mtgml-environment --test production_turn --locked` and `cargo test -p mtgml-environment --test current_successor_api --locked` → PASS; `cargo test --workspace --all-features --locked` → PASS.
- [ ] **Step 5: Commit** — `git commit -m "feat: play complete turns in the production runtime"`

### Task 8: Gates, status, docs

**Files:**
- Modify: `scripts/run_checks.py:72-76` (add `cargo test -p mtgml-environment --test production_turn --locked` next to `current_successor_api`)
- Modify: `scripts/run_v8_state_identity_gate.py` (only tokens this plan changed, e.g. required call sites in `basic_land_runtime_v8.rs`)
- Modify: `README.md` status lines and, in lockstep, the pinned strings in `python/tests/test_current_status.py:143-150`: the production runtime plays complete two-player turns with basic lands, including discard to hand size; no creatures, combat damage, spells, or deck support; **Playable engine: no** stays.
- Modify: `AGENTS.md` "Current focus" → native V3 kernel and one runtime (Plan B).

- [ ] **Step 1: Commit Tasks 1–7 first, then run** `.venv/Scripts/python.exe scripts/run_checks.py fast` → record result.
- [ ] **Step 2: Update** gates/README/status test so they describe the new behavior (no weakened assertions; changed pins only). Add no new README, token, or file-list pins — these are slated for deletion in Plan B.
- [ ] **Step 3: Commit, then run** `.venv/Scripts/python.exe scripts/run_checks.py integration` → PASS (report any `NOT_RUN`/`FAIL` exactly).
- [ ] **Step 4: Commit** — `git commit -m "chore: gate and document production full turns"`

### Task 9: Random-vs-random smoke test

**Files:**
- Create: `crates/mtgml-environment/tests/random_smoke.rs` (`#![cfg(not(feature = "historical-conformance-runtime"))]`; SplitMix64 policy PRNG and the trajectory recorder live here)
- Modify: `scripts/run_checks.py` (integration profile: `cargo test -p mtgml-environment --test random_smoke --locked`, so `PR Integration` runs it)
- Modify: `AGENTS.md` §5 — add: "Every PR keeps `cargo test -p mtgml-environment --test random_smoke --locked` green."

**Interfaces:**
- Consumes: Task 7 `two_player_land_game` and the runtime's player-facing `visible_decision` / `submit` / `export_replay` / `execute_replay`.

Setup per game `i` (seed = `0x4D414E41 + i`): `two_player_land_game(20, 7, seed)`, engine root seed = seed. Policy (D12): uniform over offered candidates; for `ChooseMany{min,max}` a uniform size in `[min, max]`, then a uniform subset of that size. A game ends when turn 31 begins or the status is terminal; a cap of 5 000 decisions per game fails the test. Default N = 8 games; `MANAFOLD_SMOKE_GAMES` overrides it for longer local runs.

- [ ] **Step 1: Write the tests:**
  - `random_games_run_thirty_turns_without_errors` — every `submit` is accepted; each game reaches turn 31.
  - `same_seed_gives_identical_trajectory` — run each seed twice; trajectories (D12) are equal element by element.
  - `exported_replay_reproduces_each_random_game` — `execute_replay(export_replay())` ends at the same checkpoint digest as the direct run.
  - `different_seeds_give_different_trajectories` — seeds 0 and 1 differ (guards against a policy that ignores its seed).
- [ ] **Step 2: Run** `cargo test -p mtgml-environment --test random_smoke --locked` → PASS. Record the wall time; if the PR-gate run exceeds 60 s, lower the default N and say so in the commit — never drop an assertion.
- [ ] **Step 3: Register** the command in `scripts/run_checks.py` and the rule in `AGENTS.md`; commit; run `.venv/Scripts/python.exe scripts/run_checks.py integration` → PASS.
- [ ] **Step 4: Commit** — `git commit -m "test: random-vs-random smoke games gate every PR"`

---

## Next plans (not part of this plan)

- **Plan B - Creatures and combat natively, then one runtime.** Extend `turn_progression` with creature combat (blockers with a new V4 purpose, combat damage, damage and life), state-based actions (incl. the SBA graveyard-order continuation; the CR 704.5b empty-library loss and its game end landed with Plan A's review fixes), with creatures the V8 catalog accepts. Differential oracle: the old kernel on the creature states its profile supports, plus the predecessor combat tests ported to the production path. Then delete the old kernel's executable paths and the predecessor runtime: `controller_predecessor.rs`, `endpoint_predecessor.rs`, `controller_successor_v7.rs`, `endpoint_successor_v7.rs`, `reference.rs`, `response_transaction.rs`, `successor_runtime.rs`, `successor_transaction.rs`, the `historical-*` features and their CI gate scripts, V6/V7 formats, the content-only admission scope, and the `#[cfg(test)]` module swap in `mtgml-environment/src/lib.rs`; rename `BasicLand*` runtime types to semantic names. The random smoke test stays green throughout.
  Carried over from Plan A's final review:
  - Order the events within a step by time instead of by kind (see the `turn_progression` module docs).
  - Report skipped positions such as cleanup and untap, or document the net-event price in the observation docs.
  - Admission resolves the game closure and parses the capability registry on every call, even for content-only manifests.
  - After the per-step digest plan: run the random smoke in more than one game per PR.
  Also delete checks that pin text instead of behavior: README/status string, registry-count and implementation-path pins in `python/tests/test_current_status.py`; token gates `scripts/run_v8_state_identity_gate.py` and `scripts/run_v5_execution_identity_gate.py`; the `scripts/run_m2_*_gates.py` scripts that pin test names. Keep a golden/KAT only if a test recomputes it from the real Rust or Python code; otherwise couple it to that code or delete it (example: `persistence/golden/content-contract-basic-land-v1-kat.v1.json` was pinned against itself and went stale unnoticed). Add to `AGENTS.md` section 5: "Tests check behavior. Do not pin README text, source tokens, test names, or file lists."
  **Update 2026-09-30:** the one-runtime part came first, without the differential oracle, in `docs/superpowers/plans/2026-09-30-one-runtime-cleanup.md`: the old kernel, the predecessor and V7 runtimes, the `historical-*` features, their gates and text pins, and the content-only admission scope are deleted. V6/V7 formats and the `BasicLand*` renames follow in the format cleanup. Creatures and combat now build on the native progression alone.
- **Plan C - First spells and the stack**, built on the native progression.
