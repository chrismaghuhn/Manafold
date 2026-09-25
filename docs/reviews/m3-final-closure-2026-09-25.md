# M3 Final Closure Candidate — 2026-09-25

**Status:** `CANDIDATE / PENDING INDEPENDENT REVIEW`

This exit record supplements the accepted entry authority in
`docs/rules/M3_INITIAL_SEMANTIC_FOUNDATION_V2.md`. It does not rewrite the
historical `SATISFIED_EVIDENCE = NO at entry` statements, alter runtime
semantics, or claim accepted M3 completion.

```text
BASE_HEAD = c7cafa0356508164988fb92b0fcedc24bccbbdae
FOUNDATION = M3_INITIAL_SEMANTIC_FOUNDATION_V2
FOUNDATION_VERSION = 2
RUNTIME_SEMANTIC = magic_bounded_turn_0_1_0
RULES_CONTRACT_ID = 4751297521d08ffd90e5d696b7df1dd32325a35125a78898fe2fa0ab01da0b38
SEMANTIC_CONTRACT_ID = b03634245635bb65dffe5f20a21ac47828a44a3aad5732594216b8138c7eee00
CAPABILITY_CLOSURE = exactly the eleven Foundation V2 capabilities in §4.5
T0 = COMPLETE / FROZEN; closure review head b403edefcabf7b304c0fa5f6816d22ac8aca477b
T0_EXIT = 10/10 PASS, 0 BLOCKER, 0 MAJOR (accepted status record)
CERTIFIED_CAPABILITIES = 0
CERTIFIED_CARDS = 0
CERTIFIED_DECKS = 0
CERTIFIED_BUNDLES = 0
```

## Capability evidence and lifecycle

`covered` below means only the bounded Foundation V2 scope. The evidence paths
and Rust test names are the registry evidence values. PR #213, #214, #216,
#217, #218, #219 and their accepted exact-head reviews supply the independent
slice review history; this candidate adds the exact §14 integration through
P2 Draw and the next visible priority Decision.

| Capability | Specified evidence | Implementation paths | Conformance cases | Decision evidence | Information evidence | Replay/checkpoint evidence | Interaction evidence | Fail-closed evidence | Candidate lifecycle |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `rules/turn-structure@0.1.0` | Foundation V2 §4.5; S1 design | `turn_structure.rs`, `magic.rs`, `semantic_cursor.rs`, `program_kernel.rs`, environment reference/catalog | S1 `s1.*` registry cases; `task10_turn_structure_reference_ordinary_untap_uses_real_kernel_and_projection`; `task10_turn_structure_reference_quiescent_cleanup_switches_turn_without_untapping` | `production_basic_priority_active_priority_and_after_first_pass_restore_fork_and_replay` | `production_basic_priority_pass_window_noninterference_hides_opponent_library_definition` | S1 deterministic/restore/fork/replay cases; exact integration | `turn_structure` cleanup-contract reject cases; turn overflow and invalid successor tests | `covered` |
| `rules/basic-priority@0.1.0` | Foundation V2 §4.5 | `basic_priority.rs`, `magic.rs`, `semantic_cursor.rs`, environment reference/projection, observation, replay | `basic_priority_stable_upkeep_opens_actor_only_single_pass_decision`; `basic_priority_first_pass_transfers_and_second_advances_one_step`; `basic_priority_rejects_wrong_actor_and_stale_response_without_mutation`; terminal/order and exhaustion cases | `production_basic_priority_active_priority_and_after_first_pass_restore_fork_and_replay`; exact integration records every priority response | pass-window hidden-world case; exact integration paired hidden worlds | basic-priority restore/fork/replay; exact integration Replay V6 | `basic_priority_rejects_untap_draw_declaration_damage_and_cleanup_windows`; unresolved SBA/order and invalid precondition rejection | `covered` |
| `rules/draw-card@0.1.0` | Foundation V2 §4.5; S3 Draw design | `magic.rs`, `contract.rs`, `product.rs`, environment reference and production tests | `ordinary_draw_uses_s2_and_opens_active_priority`; `ordinary_draw_rejections_are_atomic_and_fail_closed` | Draw itself is forced; active Draw priority is explicitly passed; exact integration records P2's post-draw priority | `ordinary_draw_preserves_opponent_noninterference_across_hidden_worlds`; exact integration proves own draw knowledge and opponent non-knowledge | `postdraw_sba_order_continuation_restores_forks_and_resumes`; exact integration Replay V6 | Draw × S2; cleanup/turn cycle exact integration | empty Library and unsupported draw profile reject atomically | `covered` |
| `rules/cleanup-reset@0.1.0` | Foundation V2 §4.5; accepted Block 7 | `turn_structure.rs`, `magic.rs`, events, semantic cursor, contract, environment reference | `bounded_turn_cleanup_resets_marks_and_hands_off_through_next_upkeep`; `bounded_cleanup_restore_rejects_active_hand_eight_atomically`; `bounded_cleanup_restore_rejects_sba_unstable_state_without_clearing_damage` | No cleanup Decision; exact integration proves only external End Step passes | V4 before/after cleanup in exact integration | full-turn endpoint Replay V6 and checkpoint/fork cases | marks persist after combat through End Step and clear only in Cleanup; exact integration | hand limit, ambiguous ownership, SBA instability, stale combat, priority/continuation, overflow and event exhaustion rejection cases | `covered` |
| `rules/combat-damage@0.1.0` | Foundation V2 §4.5; accepted Block 6 | `combat_damage.rs`, `magic.rs`, events, semantic cursor, contract, environment reference/projection, observation, Replay V6 | typed unblocked damage; simultaneous blocked damage; two lethal creatures; multiple/mixed attackers; zero power; blocked history after blocker removal; cursor mutants | no damage Decision; post-damage priority explicitly passed | `combat_damage_rejection_and_hidden_world_projection_are_atomic`; exact integration paired worlds | forced product restore/fork/replay exactness; exact integration | declaration × assignment × damage × life × SBA × incarnation in exact integration | two blockers, unsupported characteristics, unstable SBA, arithmetic/revision/event exhaustion reject before mutation | `covered` |
| `rules/combat-phase@0.1.0` | Foundation V2 §4.5 | `basic_priority.rs`, `magic.rs`, semantic cursor, environment reference/projection, observation | `combat_phase_reaches_explicit_attacker_choice_and_commits_public_participation`; `explicit_empty_attack_skips_blockers_and_damage_then_ends_combat` | exact integration explicitly passes every combat priority window | V4 combat projection and exact paired-world integration | end-step/cleanup Replay V6 and exact full-turn replay | declaration through damage and End of Combat | empty-combat skip and old-contract boundary tests | `covered` |
| `rules/declare-attackers@0.1.0` | Foundation V2 §4.5 | `magic.rs`, events, semantic cursor, Decision V2, environment reference/projection, observation | `three_attacker_decision_represents_each_legal_subset_once`; explicit empty attack; `nine_eligible_attackers_fail_before_decision_without_mutation` | complete bounded subset space; exact integration selects `{A,C}` externally | attacker hidden-world projection invariance and exact integration | restore/fork/replay cases at attacker boundary | control history/tapped legality plus combat phase composition | >8 attackers and source-less/unstable states reject before Decision | `covered` |
| `rules/declare-blockers@0.1.0` | Foundation V2 §4.5 | state core/delta; rules priority/events/magic/contract/cursor; environment reference/projection; observation; Replay V6 | exact one-blocker assignment set; zero-blocker deterministic empty assignment; one-blocker/one-attacker explicit choices; >1 eligible blocker rejection | exact integration explicitly chooses B→A; no hidden blocker default | blocker hidden-world projection invariance | blocker decision checkpoint/fork/Replay V6 exactness | declaration × combat damage exact integration | >1 blocker, stale candidate/domain, SBA-unstable and malformed restore states reject | `covered` |
| `rules/damage-and-life@0.1.0` | Foundation V2 §4.5 | state damage/core/delta; combat damage producer; events/cursor; environment projection; observation | unblocked player damage, creature marks, simultaneous damage, lethal player terminal, zero power, checked arithmetic and mutants | no damage assignment Decision | public life/mark projection; exact integration paired worlds | combat damage forced product restore/fork/replay | damage × SBA × Cleanup in exact integration | overflow, invalid profile and malformed assignment reject atomically | `covered` |
| `rules/state-based-actions-combat@0.1.0` | Foundation V2 §4.5; S3.A evidence | `state_based_actions.rs`, `zone_incarnation.rs`, events/cursor/magic, environment reference | selected cause applicability, simultaneous lethal creatures, player loss, combat participant pruning and post-damage SBA cases | ordering only when the selected SBA profile requires it; otherwise no fake Decision | terminal/public loss and death projections through player products | SBA continuation restore/fork/replay and full integration | combat damage → SBA fixed point → zone incarnation | effect/trigger/delayed/stack and unsupported profiles reject; unstable states cannot open priority | `covered` |
| `rules/zone-incarnation@0.1.0` | Foundation V2 §4.5; S2 design | `zone_incarnation.rs`, contract/cursor, state identity, environment reference, Rust and conformance tests | S2 Battlefield→owner Graveyard and Library-top→Hand exact cases, mutation matrix, ordering and identity continuity | no zone-choice Decision in selected producer; combat ordering choices remain external | S2 lifecycle matrix and Library noninterference; exact integration proves owner draw knowledge boundary | S2 Replay V6 through Draw integration; S2 checkpoint/restore/fork/rerun cases | SBA death→graveyard incarnation and Draw→Hand incarnation in exact integration | wrong family/owner/top/stale incarnation and late product failure reject atomically | `covered` |

## M3 interaction obligations

All sixteen rows retain their exact Foundation V2 meaning. Each cites direct
executable evidence; the final column status is a candidate disposition and
is conditional on the required gate results below.

| Obligation | Concrete executable evidence | Gate | Candidate |
| --- | --- | --- | --- |
| M3-ENTRY-001 turn progression × priority | `basic_priority_first_pass_transfers_and_second_advances_one_step`; `production_basic_priority_active_priority_and_after_first_pass_restore_fork_and_replay`; exact integration | Rust workspace + integration | satisfied |
| M3-ENTRY-002 forced progression × Draw × Cleanup | `task10_turn_structure_reference_ordinary_untap_uses_real_kernel_and_projection`; `ordinary_draw_uses_s2_and_opens_active_priority`; `bounded_turn_cleanup_resets_marks_and_hands_off_through_next_upkeep`; exact integration | Rust workspace + integration | satisfied |
| M3-ENTRY-003 Draw × incarnation × information | `ordinary_draw_preserves_opponent_noninterference_across_hidden_worlds`; `ordinary_draw_uses_s2_and_opens_active_priority`; exact integration proves P2 draw and knowledge | Rust workspace + integration | satisfied |
| M3-ENTRY-004 combat phase × priority | `combat_phase_reaches_explicit_attacker_choice_and_commits_public_participation`; `production_basic_priority_end_step_two_pass_reference_replay_closes_through_cleanup`; exact integration | Rust workspace + integration | satisfied |
| M3-ENTRY-005 attacker Decision soundness/completeness | `three_attacker_decision_represents_each_legal_subset_once`; `zero_eligible_attackers_still_require_an_explicit_empty_response`; exact integration | Rust workspace + integration | satisfied |
| M3-ENTRY-006 attack legality × control history/tapped | `admitted_mixed_attacker_fixture_exposes_exactly_continuously_controlled_untapped_creatures`; `nine_eligible_attackers_fail_before_decision_without_mutation`; S1 control-history cases | Rust workspace + integration | satisfied |
| M3-ENTRY-007 blocker Decision soundness/completeness | `one_blocker_three_attacker_choice_space_is_exactly_four_assignments`; `zero_eligible_blockers_uses_deterministic_empty_assignment_without_decision`; `more_than_one_eligible_blocker_rejects_before_decision_without_mutation` | Rust workspace + integration | satisfied |
| M3-ENTRY-008 block assignment × damage | `combat_damage_simultaneously_marks_and_performs_post_damage_sba`; `combat_damage_multiple_and_mixed_attackers_are_complete_and_simultaneous`; exact integration | Rust workspace + integration | satisfied |
| M3-ENTRY-009 damage × life/marks | `combat_damage_unblocked_assignment_is_typed_public_and_replay_exact`; `combat_damage_simultaneously_marks_and_performs_post_damage_sba`; exact integration | Rust workspace + integration | satisfied |
| M3-ENTRY-010 damage × SBA fixed point | `combat_damage_two_lethal_creatures_leave_from_one_sba_snapshot`; `zero_power_has_no_damage_event_and_lethal_player_damage_has_no_priority`; exact integration | Rust workspace + integration | satisfied |
| M3-ENTRY-011 SBA × zone/incarnation/LKI | `task9b_no_order_post_damage_application_prunes_one_blocker_and_moves_it`; S2 battlefield mutation matrix; exact integration proves fresh owner-graveyard incarnation | Rust workspace + integration | satisfied |
| M3-ENTRY-012 zone × information/identity projection | S2 lifecycle/hidden Library cases; `ordinary_draw_preserves_opponent_noninterference_across_hidden_worlds`; exact integration paired worlds | Rust workspace + integration | satisfied |
| M3-ENTRY-013 Cleanup × marks | `bounded_turn_player_endpoints_carry_combat_damage_through_cleanup_and_replay`; `bounded_cleanup_restore_rejects_sba_unstable_state_without_clearing_damage`; exact integration | Rust workspace + integration | satisfied |
| M3-ENTRY-014 combat × checkpoint/fork/replay | `combat_damage_pending_checkpoint_restore_and_fork_reproduce_forced_product`; blocker decision replay case; exact integration Replay V6 | Rust workspace + integration | satisfied |
| M3-ENTRY-015 unsupported adjacent semantics × fail closed | `ordinary_draw_rejections_are_atomic_and_fail_closed`; `more_than_one_eligible_blocker_rejects_before_decision_without_mutation`; `combat_damage_restore_rejects_unsupported_characteristics_atomically`; SBA effect/trigger/delayed/stack profile rejection tests | Rust workspace + integration | satisfied for represented Foundation state axes; out-of-model keyword identities cannot be encoded in the admitted synthetic source type |
| M3-ENTRY-016 accepted/rejected product integrity | T0 rejected-fingerprint and deterministic diagnostic tests; combat damage cursor mutants/exhaustion; draw and blocker complete atomicity; S2 complete fingerprint matrix | T0 + Rust workspace + integration | satisfied |

## Exact Foundation V2 §14 integration

Executable case: `foundation_v2_exact_turn_composition_closes_through_p2_draw`
in `crates/mtgml-environment/src/tests/magic_rules_production.rs`.

The case starts from turn 2, P1 active, `FormatState::None`, Untap. P1 has
tapped A (3/3) and C (2/2); P2 has untapped B (2/2), plus a separate tapped
permanent. It explicitly attacks with A/C and explicitly assigns B→A. The
result checks B's lethal simultaneous damage and fresh graveyard incarnation,
A's two marks through End Step and their Cleanup reset, C's two damage to P2
(20→18), P1 attackers remaining tapped, P2 Untap affecting only P2's
permanent, turn 3 P2 Upkeep and Draw, and the next visible P2 pass Decision.

The case submits 24 external DecisionResponseV2 values. Every pass and both
combat choices are endpoint-submitted and recorded with actor, domain, and
candidate count. Forced Untap, Draw, damage, SBA, Cleanup, and handoff create
no synthetic responses. Replay V6 contains exactly the 24 external responses
and re-executes to the same final checkpoint. A paired hidden world changes
only an unrevealed card that remains in P1's hand; P2's player fingerprint is
equal after every response. Draw identity is learned by the drawing player
only.

## T0 and product acceptance

T0 remains the separately accepted, frozen infrastructure at
`b403edefcabf7b304c0fa5f6816d22ac8aca477b`, recorded as 10/10 exit criteria
PASS and 0 blocker/major. The executable conformance tests include accepted
and rejected authored witnesses, full rejection fingerprints, multi-step
forced progress, player-product checks, checkpoint/fork/replay hooks, and
first-divergence diagnostics. The ignored diagnostic witness
`t0_failure_witness_capture` is a developer aid and is not counted as a pass.
`FORCED_PROGRESS_CONFORMANCE` is exercised by
`forced_progress_stabilizes_entry_without_any_response` and
`forced_progress_result_accepts_a_real_response_with_continuous_replay`.

The current semantic identities are frozen: no contract manifest, Rules ID,
Semantic ID, capability version, observation schema, Replay V6, Checkpoint V6,
or FullStateDigestV5 semantics change in this closure candidate. Historical
V1/V2/V3 observation and older contract meanings remain untouched. The only
runtime source addition is an acceptance test; no gameplay implementation is
changed.

## Scope and disposition

M3 remains synthetic, two-player, format-neutral, and bounded by Foundation
V2. The candidate claims 11/11 specified, implemented, and covered; 0/11
certified; 16/16 interaction obligations; no cards, decks, formats, Commander,
playability, arbitrary Magic, or bundle certification. M4 and performance
work have not started.

Final disposition remains `CANDIDATE / PENDING INDEPENDENT EXACT-HEAD REVIEW`,
hosted CI, and post-merge exact-master verification. This document is not
executable proof and does not itself authorize the accepted repository status
`M3 = COMPLETE`.

## Verification record

| Gate | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | PASS | Block 8 worktree |
| `cargo check --workspace --all-targets --all-features --locked` | PASS | `CARGO_BUILD_JOBS=1` |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS | `CARGO_BUILD_JOBS=1` |
| `cargo test -p mtgml-environment --locked` | PASS | 236 unit tests + checkpoint V5/V6 and P0 integration test binaries; includes exact §14 case |
| `cargo test --workspace --all-features --locked` | PASS | `CARGO_BUILD_JOBS=1`; one earlier unconstrained attempt hit Windows pagefile/thread resource exhaustion before tests; serial rerun passed |
| T0 conformance regression | PASS | `cargo test -p mtgml-conformance --all-features --locked facade::t0_01_red_contract`; 14 passed, one diagnostic helper ignored |
| Exact §14 integration | PASS | `cargo test -p mtgml-environment --locked foundation_v2_exact_turn_composition_closes_through_p2_draw` |
| Python fast | PASS | `scripts/run_checks.py fast`; 71 Python smoke tests plus contract/schema/repository gates |
| Repository/schema/maintainer/golden/contract catalog | PASS | `verify_repository.py`, `validate_schemas.py`, `validate_maintainer_artifacts.py`, `validate_golden_path.py`, semantic catalog `--check` |
| Python integration | PENDING_CLEAN_HEAD_RERUN | Initial run had 2 failures in M2.H clean-tree fingerprint tests because the candidate files were uncommitted; 3 optional adapter scenarios were skipped. Must rerun on clean commit head. |
| Certification/repository acceptance profile | NOT_RUN | Run after all source changes and commit; archive reproducibility must be the final source-tree gate. |
| Hosted CI | NOT_RUN | Draft PR required |
