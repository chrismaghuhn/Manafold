# Blocks and Deaths Implementation Plan

**Status:** APPROVED by the owner on 2026-10-01 ("per plan weiter wie gewohnt").
- Execution: subagent-driven, with Sonnet implementers and Opus reviewers.
- This is step 3a of the approved spec. Step 3b gets its own plan: damage
  division, observation of blocks and damage, and symmetric smoke games.
- References were refreshed against `master` 6ac4259f, where step 2 is merged
  as #270.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** vanilla creatures block, deal and receive combat damage, die from
lethal damage, and go to their owner's graveyard in an order the owner
chooses. Marked damage is removed at cleanup.

**Architecture:**
- The defending player declares blocks one creature at a time. The partial
  declaration lives in a continuation that only the defender sees, and one
  `BlockersDeclared` is emitted when it is complete.
- Combat damage marks damage on creatures (`marked_damage` in the card-rules
  `PermanentState`).
- A state-based-action fixed point (704.5a, 704.5g) removes dying creatures
  from combat (506.4) and moves them battlefield → owner's graveyard. The
  existing `SbaGraveyardOrder` contract gets a producer, so an owner orders
  two or more simultaneous deaths.
- Cleanup removes marked damage (514.2).

**Tech Stack:** Rust workspace (`mtgml-state`, `-rules`, `-decision`,
`-observation`, `-environment`), Python client mirror, JSON schemas.

**Spec:** `docs/superpowers/specs/2026-10-01-vanilla-creatures-design.md`
(revision 2, approved), §3, §4 and §5 (blocks, marked damage, SBA, cleanup),
§7 step 3a.

## Global Constraints

- AGENTS.md is binding:
  - correctness → determinism → information safety → decision completeness;
  - every player choice is an explicit decision;
  - unsupported rules fail closed with a precise error;
  - formats change in place (delete replaced code in the same change);
  - tests go through the production runtime;
  - every rules question is answered from the pinned rules text
    `C:\Dev\src\Manafold\.rules\MagicCompRules 20260925.txt` (SHA-256
    `8d860e45…`; never commit it).
- **Every commit** keeps green: fmt, clippy `-D warnings`,
  `cargo test --workspace --locked`, the release `random_smoke`, the Python
  suite, ruff, mypy and the fast-gate scripts.
- **A commit that changes the smoke pins** re-pins them, and its message
  proves the land games' decisions unchanged with a decision-only scratch
  fingerprint (actor, decision id, answer).
  - The current pins in `crates/mtgml-environment/tests/random_smoke.rs` are
    `SHORT_FINGERPRINT` 2de310ef…, `LONG_FINGERPRINT` 41afc73c… and
    `ASYMMETRIC_SHORT_FINGERPRINT` a07d4d47….
  - The asymmetric smoke games never reach a block, because the land player
    has no creatures. Their pins change only with the digest.
- No card-name dispatch: power, toughness and card types come from the S1
  base characteristics.
- Requests and candidates are ordered by the **actor's opaque ids**
  (`assign_dense`), never by `GameObjectId`.
- **Multi-blocks reaching damage:** an attacker blocked by **two or more**
  creatures needs damage division, which comes in 3b. In 3a such a
  declaration is legal and can be made, but combat damage returns
  `Err(TurnProgressUnsupported)` for it. The smoke games stay asymmetric until
  3b, so they never reach this.

## Review Focus

1. **A blocker that leaves combat before damage:** none can in 3a, since there
   are no instants. The blocked attacker stays blocked (509.1h) and deals no
   damage (510.1c). Tested in Task 3:
   `an_attacker_whose_blocker_is_gone_deals_no_damage`, using a synthetic state.
2. **Both players at 0 life at once** (104.4a): fails closed. Tested in Task 4:
   `both_players_losing_at_once_fails_closed`.
3. **Restore mid-block:** a checkpoint taken during a half-done block
   declaration must restore and continue identically. Tested in Task 2:
   `a_restored_half_declared_block_continues_identically`.
4. **A half-declared block is invisible to the attacker:** the attacker's
   bytes must not change between the defender's partial answers. Tested in
   Task 2: `a_half_declared_block_is_invisible_to_the_attacker`.
5. **Damage survives the turn boundary:** marked damage must be gone after
   cleanup on both cleanup exits, with and without a discard. Tested in
   Task 6: `marked_damage_is_removed_at_cleanup_with_and_without_a_discard`.

---

### Task 1: Blocks in the combat state

**Files:**
- `crates/mtgml-state/src/core.rs`: `CombatState.blockers`,
  `CombatBlockerAssignmentV1`.
- `crates/mtgml-state/src/validation/core.rs`: `validate_combat`, including
  the one-blocker check (~:73-82).
- `crates/mtgml-state/src/digest.rs`: `combat_value` (~:1245-1275).
- `crates/mtgml-state/src/delta.rs`: `BlockersDeclared` coverage.
- `crates/mtgml-rules/src/events.rs`: `BlockersDeclared` (~:112), projection
  (~:1438).
- `crates/mtgml-rules/src/combat.rs` (from step 2): `declare_attackers` writes
  the new shape.
- `docs/STATE_HASHING.md`.
- Re-pin the digest pins and smoke pins.

**Interfaces:**
- Produces:
  - `CombatState.blockers: BTreeMap<GameObjectId /*blocker*/, GameObjectId /*attacker*/>`,
    next to the existing `blocked_attackers: BTreeSet<GameObjectId>` (509.1h).
  - `CombatBlockerAssignmentV1 { blocker: GameObjectId, attacker: GameObjectId }`;
    `BlockersDeclared { assignments }` sorted by blocker.
  - The combat digest has **one** form: `[defender, attackers, blockers, blocked_attackers, damage_step_completed]`.
    The dual 3/5-element compatibility form is deleted.
- **Validation:**
  - every blocker is a creature on the battlefield controlled by the
    defending player, and its attacker is in `attackers`;
  - `blocked_attackers ⊇ { attacker of any blocker }`;
  - no cap on blockers per attacker.
- **Ruling (spec §4):** a `Vec<blocker>` per attacker would carry a
  meaningless order, giving one rules state two digests. The map from blocker
  to attacker has none.
- **Carried from step 2's Task 7 review:** restore must reject two states the
  game never rests in:
  - a CombatDamage-step state with attackers and
    `damage_step_completed == false`;
  - blockers assigned while still in DeclareAttackers.

  Extend step 2's `validate_reachable_combat` (`combat.rs`) accordingly, with
  a test for each.

- [ ] **Step 1: Failing tests**
  - `two_blockers_on_one_attacker_validate`.
  - `a_blocker_of_the_attacking_player_is_rejected`.
  - `a_blocker_for_a_non_attacker_is_rejected`.
  - `the_combat_digest_binds_blockers_and_blocked_attackers`: a mutation for
    each field.
  - `an_empty_combat_has_one_digest_form`: an empty-combat state hashes the
    same before and after damage-step completion, except for the bool.
- [ ] **Step 2: Run** `cargo test -p mtgml-state --locked`. Expected: FAIL to
  compile.
- [ ] **Step 3: Implement.** Change in place and delete the old map and the
  dual digest form.
- [ ] **Step 4: Run** the workspace tests and the release smoke. Re-pin with
  the decision-only fingerprint evidence. The land games' combat digest
  changes with the single form.
- [ ] **Step 5: Commit** `feat: blocks are recorded blocker by blocker`.

### Task 2: The defending player declares blocks

**Files:**
- `crates/mtgml-decision/src/v4.rs`:
  - `DecisionPurposeV4::BlockerDeclaration`;
  - `CandidateIntent::DeclareBlock { blocker: OpaqueObjectId, attacker: Option<OpaqueObjectId> }`;
  - `EngineCandidateBinding::DeclareBlock { blocker: GameObjectId, attacker: Option<GameObjectId> }`;
  - ordering rank and visibility (`ActingPlayerOnly`).
- Python `decision_v4.py`; the schemas `player-decision-request.v4` and
  `player-step.v4`; examples and negatives.
- `crates/mtgml-state/src/shared_execution.rs`:
  `ContinuationPayload::BlockDeclaration { defender: PlayerId, pending_blockers: Vec<GameObjectId>, declared: BTreeMap<GameObjectId, Option<GameObjectId>> }`.
- `crates/mtgml-state/src/engine.rs`: continuation and request validation.
- `crates/mtgml-state/src/digest.rs`: the continuation arm.
- `crates/mtgml-rules/src/combat.rs`: replace `defender_could_block`'s
  fail-closed with the declaration.
- `crates/mtgml-rules/src/turn_progression.rs`: a non-active actor for
  install and validation (the pregame's `RequestShape` is the precedent).
- Test: `crates/mtgml-environment/tests/creature_combat.rs` (new).

**Interfaces:**
- At DeclareBlockers with attackers, the defender is asked **once per
  untapped creature they control**, in the defender's opaque order.
  - One `ChooseOne` per creature: one `DeclareBlock { blocker, attacker: Some(a) }`
    per attacker, plus `DeclareBlock { blocker, attacker: None }`.
  - The partial answers live in `BlockDeclaration`. Its request carries the
    `continuation_id`.
  - After the last answer: one `BlockersDeclared`, `blocked_attackers` is set,
    the continuation is removed, and the active player gets priority (509.2).
- If the defender has no untapped creature: no request, as in step 2.
- **Ruling:** every combination of answers is legal for vanilla creatures, so
  no whole-declaration check runs. Menace and block requirements will need
  one later (spec §5 "future seam").
- **Restore rule** (step 2's `validate_reachable_combat`, `combat.rs`): the
  rule "from DeclareBlockers on, the defender controls no untapped creature"
  is replaced.
  - With a pending `BlockerDeclaration`, the continuation and the request
    must match the re-derived ones.
  - Once the declaration is complete (priority in DeclareBlockers, or a later
    combat step), untapped defending creatures that did not block are legal.
  - Every recorded blocker must be a creature the defender controls.
  - Test: `a_restored_state_after_blocks_were_declared_may_hold_an_unblocking_creature`.

- [ ] **Step 1: Failing tests** (endpoints, creature deck against creature
  deck, scripted answers):
  - `the_defender_declares_blocks_one_creature_at_a_time`: two untapped
    creatures give two requests. Each request names its blocker and lists
    every attacker plus "no block". One `BlockersDeclared` follows the second
    answer.
  - `a_half_declared_block_is_invisible_to_the_attacker`: the attacker's
    information-state bytes are identical before and after the defender's
    first answer.
  - `a_restored_half_declared_block_continues_identically`.
  - `a_rejected_block_answer_changes_nothing`.
  - State tests: the continuation must name the defender, untapped defender
    creatures and real attackers; a `declared` entry may not point at a
    non-attacker.
- [ ] **Step 2: Run** them. Expected: FAIL (`TurnProgressUnsupported` at
  DeclareBlockers).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** everything. Expected: PASS. The land pins are unchanged
  (no creature, no request).
- [ ] **Step 5: Commit** `feat: the defending player declares blocks`.

### Task 3: Marked damage, and combat damage with one blocker

**Files:**
- `crates/mtgml-state/src/card_rules.rs`:
  `PermanentState { controlled_since_turn, marked_damage: u64 }`, with digest
  element 8 becoming `[[object, controlled_since_turn, marked_damage], …]`.
- `crates/mtgml-state/src/semantic_mutations.rs`, `engine.rs` (validation:
  `marked_damage` only on creatures).
- `crates/mtgml-state/src/delta.rs`: a `marked_damage` change needs
  `MarkedDamageChanged`.
- `crates/mtgml-rules/src/events.rs`: implement the `MarkedDamageChanged`
  projection (~:1309, today always false) and the object-damage branch
  (~:928-932).
- `crates/mtgml-rules/src/combat.rs`: damage assignment.
- `docs/STATE_HASHING.md`. Re-pin.

**Interfaces:**
- **Damage** (510.1, simultaneous, 510.2):
  - an unblocked attacker → the defending player;
  - an attacker with exactly one blocker → all its power to that blocker
    (510.1c);
  - an attacker whose blockers are gone → no damage (510.1c);
  - each blocker → its power to the attacker it blocks (510.1d);
  - two or more blockers on one attacker → `Err(TurnProgressUnsupported)`
    (3b);
  - power 0 or less assigns no damage (510.1a).
- **Events:** `CombatDamageDealt { assignments }` (with player and creature
  recipients), `LifeChanged` per damaged player, `MarkedDamageChanged` per
  damaged creature, then `CombatDamageStepCompleted`.
- **Delete** the rule-event kind `AuthoritativeRuleEventKind::DamageApplied`
  and the delta operation `SemanticDeltaOperation::DamageApplied`, with their
  projections and coverage arms (spec §4: one damage family).
  - The **trigger vocabulary** (`TriggerEventSnapshot::DamageApplied`,
    `TriggerEventKindV1::DamageApplied`, `SafeTriggerSubjectV1::DamageApplied`
    in mtgml-state and mtgml-decision, and its Python mirror) describes a
    trigger condition. It is not this family, and it stays.
  - If a validator ties a trigger snapshot to the deleted operation, stop and
    report NEEDS_CONTEXT.

- [ ] **Step 1: Failing tests:**
  - `a_blocked_attacker_and_its_blocker_damage_each_other`: Gray Ogre blocked
    by Savannah Lions gives Ogre 2 marked and Lions 2 marked. The defender's
    life is unchanged.
  - `an_unblocked_attacker_still_hits_the_player_while_another_is_blocked`.
  - `an_attacker_whose_blocker_is_gone_deals_no_damage` (synthetic).
  - `two_blockers_on_one_attacker_fail_closed_until_damage_can_be_divided`.
  - State and delta: marked damage without `MarkedDamageChanged` is rejected;
    marked damage on a land is rejected.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** Until Task 4 implements death, lethal damage
  fails closed:
  - after combat damage, a creature with `marked_damage ≥ toughness` gives
    `Err(TurnProgressUnsupported)`;
  - `validate_slice`'s pending-SBA rule (from step 2: life ≤ 0) extends to
    such a creature.

  The tests of this task use non-lethal damage, plus one test that lethal
  damage fails closed. Task 4 replaces the fail-closed with the SBA.
- [ ] **Step 4: Run** everything and re-pin (digest element 8 widens).
- [ ] **Step 5: Commit** `feat: combat damage marks creatures`.

### Task 4: Lethal damage, removal from combat, and death

**Files:**
- `crates/mtgml-rules/src/zone_incarnation.rs`:
  `SelectedZoneTransitionKind::BattlefieldToOwnerGraveyard`. Every
  perspective follows (Remap); the graveyard top is public.
- `crates/mtgml-rules/src/combat.rs` or a new `state_based_actions.rs`: the
  SBA fixed point.
- `crates/mtgml-rules/src/turn_progression.rs`: run the SBAs after combat
  damage, replacing the 704.5a-only check from step 2.
- `crates/mtgml-state/src/delta.rs:~1232`: `permanent_card_to_graveyard` may
  change, covered by the move.
- `semantic_mutations.rs` `record_permanent_card_to_graveyard`.

**Interfaces:**
- The SBA fixed point after combat damage:
  1. Collect the simultaneous actions: 704.5a for players at ≤ 0 life;
     704.5g for creatures with toughness > 0 and
     `marked_damage ≥ toughness` (`ObjectToOwnerGraveyard { causes: [LethalDamage] }`).
  2. Remove each dying creature from combat (506.4): drop it from `attackers`
     and `blockers`, and keep `blocked_attackers` (509.1h).
  3. Move it battlefield → owner's graveyard: a new incarnation; prune
     `permanents`; `record_permanent_card_to_graveyard`.
  4. Repeat until no action applies.
- Both players losing at once (104.4a) gives `Err(TurnProgressUnsupported)`.
- **Carried from step 2 (Task 4 ruling):** this is the first move that leaves
  the battlefield, so the `move_card` prune finally gets a removal test.

- [ ] **Step 1: Failing tests:**
  - `a_2_1_blocked_by_a_2_2_dies` (endpoints): the Lions goes to its owner's
    graveyard; both players observe the move; the Ogre survives with 2 marked.
  - `a_creature_that_dies_is_removed_from_combat_but_its_attacker_stays_blocked`.
  - `the_prune_removes_a_departed_permanent`.
  - `both_players_losing_at_once_fails_closed` (synthetic).
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** With exactly one dying creature per owner, the
  move applies directly. Two or more per owner fail closed until Task 5.
  Ledger this.
- [ ] **Step 4: Run** everything. Expected: PASS, and the land pins are
  unchanged.
- [ ] **Step 5: Commit** `feat: creatures with lethal damage die`.

### Task 5: The owner orders simultaneous deaths

**Files:**
- `crates/mtgml-rules/src/state_based_actions.rs` (or wherever Task 4 put the
  SBA fixed point): the producer for the existing
  `ContinuationPayload::MagicSbaGraveyardOrderV1` (shared_execution.rs
  ~:189), purpose `SbaGraveyardOrder` (`Order` domain), and its executor.
- `crates/mtgml-rules/src/turn_progression.rs`: the non-active actor (the
  owner) for the request.
- The existing validators: `engine.rs` (~:884, ~:1098, ~:1158, ~:1538) and
  `player_projection.rs` (~:766). Use them; do not rewrite them.

**Interfaces:**
- When two or more creatures go to one owner's graveyard at once (404.3), the
  contract runs as specified:
  - `selected_sba_actions` holds the whole simultaneous batch, `PlayerLoses`
    included;
  - owners order in APNAP order (`apnap_owners`, `next_owner_index`);
  - each owner answers one `Order` request over their dying cards, top to
    bottom (`SbaGraveyardOwnerOrderV1`);
  - the whole batch, losses included, applies after the last order.
- An owner with one dying card is not asked.

- [ ] **Step 1: Failing tests:**
  - `two_creatures_dying_together_ask_their_owner_for_the_order`:
    - Setup: P1 attacks with Gray Ogre and Hill Giant. P2 blocks the Ogre with
      Savannah Lions A and the Giant with Savannah Lions B.
    - Damage: both Lions die (2 ≥ 1 and 3 ≥ 1); the Ogre dies (2 ≥ 2); the
      Giant survives with 2 marked.
    - P1 owns one dying card and is not asked. P2 is asked once to order its
      two Lions.
    - P2's graveyard order matches the answer, and the Ogre is in P1's
      graveyard.
  - `a_player_losing_in_the_same_batch_still_lets_owners_order_first`.
  - `a_restored_graveyard_order_checkpoint_continues_identically`.
- [ ] **Step 2: Run.** Expected: FAIL (Task 4's fail-closed for two or more).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** everything. Expected: PASS.
- [ ] **Step 5: Commit** `feat: owners order creatures that die together`.

### Task 6: Cleanup removes marked damage

**Files:**
- `crates/mtgml-rules/src/turn_progression.rs`: both exits of `advance()` at
  cleanup — no discard, and after the `HandSizeDiscard` answer.

**Interfaces:**
- 514.2: after the 514.1 discard decision, all marked damage becomes 0, with
  one `MarkedDamageChanged` per creature that had damage.

- [ ] **Step 1: Failing test:**
  `marked_damage_is_removed_at_cleanup_with_and_without_a_discard`.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** everything.
- [ ] **Step 5: Commit** `feat: cleanup removes marked damage`.

### Task 7: Capabilities and documents

**Files:**
- `cards/capabilities/registry.json`: `rules/declare-blockers`,
  `rules/damage-and-life` and `rules/state-based-actions-combat` become
  `covered` with their conformance cases. `rules/combat-damage` stays
  `specified` (division is 3b).
- The capability docs; `docs/INFORMATION_MODEL.md` (blocks are invisible
  while half-declared); `README.md`.
- Regenerate the projection.

- [ ] **Step 1:** Update the registry and docs.
- [ ] **Step 2: Run** `scripts/validate_maintainer_artifacts.py`,
  `scripts/generate_card_ir_capability_projection.py --check` and
  `scripts/check_documentation.py`. Then run the full integration gate.
- [ ] **Step 3: Commit** `docs: blocks and deaths are covered`.

## Owner decisions (2026-10-01)

1. **Observation in 3a:** blocks and marked damage become visible to players
   only with 3b, as the spec phases it. The owner accepted the interim state:
   in 3a players see deaths through the public zone moves, but not the blocks
   or the marked damage.
