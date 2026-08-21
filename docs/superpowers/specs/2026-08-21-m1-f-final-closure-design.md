# M1.F Final Milestone Closure Design

**Status:** accepted for implementation
**Stability:** provisional process artifact
**Starting `origin/master`:** `bbbf0ee06b64889c318a7f0fd4d9b608d7181ed7`

## Goal

Produce reproducible, machine-readable evidence for the ten M1 kernel-shell
acceptance gates on one exact source commit. This work owns evidence
orchestration only; it does not add gameplay behavior, Magic semantics, cards,
or M2 functionality.

## Existing ownership boundary

`scripts/run_verification.py` remains the V0.2.2 Foundation reporter. Its
`v0.2.2-status.json`, `FOUNDATION_VERIFICATION.md`, and
`FOUNDATION_BLOCKERS.md` retain their existing meaning and are not imported,
renamed, or reinterpreted as M1 evidence.

The repository has no authoritative tracked M1 status source. The
`project-sources/33_CURRENT_PROJECT_STATE.md` file is a ChatGPT project-source
export, and the accepted M1.7 process explicitly excludes it from closure
status updates. M1 completion is therefore represented by the generated
external M1 report and the exact-head PR evidence, not by editing that export.

## Reporter architecture

Add `scripts/run_m1_closure.py` as a narrow orchestration script. It owns:

1. exact `HEAD` and source-tree identity capture;
2. pinned Rust/Python toolchain identity capture;
3. one subprocess invocation per named gate-owning test;
4. explicit `PASS`, `FAIL`, `NOT_RUN`, and `BLOCKED` evidence;
5. generated JSON and Markdown reports from one in-memory result set.

Each Rust invocation uses the exact test-binary filter:

```text
cargo test --package <crate> --all-features --locked --lib -- <module::test> --exact
```

The runner requires the output to report exactly one executed and passed test;
an unmatched filter is a failure rather than an accidental green command. It
continues after a failed command so every M1 gate receives an independent
status. The runner never executes a wrapper shell or writes to the source
tree.

Reports are written only below `dist/verification/m1/`, protected by a
dedicated ownership marker, with one log per exact test command:

```text
dist/verification/m1/
├── .mtgml-m1-closure-output
├── logs/
├── m1-verification-results.json
├── M1_VERIFICATION.md
└── M1_BLOCKERS.md
```

`overall = COMPLETE` is derived only when all ten gate statuses, the source
identity check, and both pinned toolchain checks are `PASS`. Otherwise the
report is `INCOMPLETE` and its M2 status is `BLOCKED`. The claims remain
`playable_engine = false`, `real_magic_rules = false`, and
`real_card_support = false` in every outcome.

## Gate-to-test evidence inventory

The reporter executes these exact current tests. The owning crate is part of
each command record, so the report remains auditable if a test moves later.

| M1 gate | Owning crate(s) and exact tests |
|---|---|
| `ENGINE_STATE_CONSTRUCTION_AND_INVARIANTS` | `mtgml-state`: `tests::synthetic_constructor_builds_a_nontrivial_valid_state`, `tests::synthetic_reset_is_exactly_deterministic_for_identical_inputs`, `tests::valid_empty_shell_passes_cross_component_validation`, `tests::pending_decision_must_match_state_revision`, `tests::unordered_object_must_not_appear_in_ordered_zones`, `tests::ordered_object_must_appear_exactly_once`, `tests::ordered_object_must_not_be_missing`, `tests::duplicate_live_physical_card_incarnation_rejected`, `tests::pending_candidate_binding_must_match_authoritative_binding`, `tests::opaque_allocator_must_reference_declared_player`, `tests::commander_designation_must_reference_owned_live_physical_card`, `tests::valid_commander_structural_references_are_accepted`, `tests::commander_ledger_must_reference_a_designated_physical_card`, `tests::commander_damage_ledger_must_reference_a_declared_player` |
| `ACCEPTED_TRANSITION_EXACT_PRODUCT` | `mtgml-rules`: `tests::synthetic_m1_acceptance_returns_exact_transition_product`; `mtgml-environment`: `tests::accepted_environment_transaction_commits_the_exact_m1_product` |
| `REJECTED_RESPONSE_COMPLETE_NONMUTATION` | `mtgml-rules`: `tests::synthetic_rejection_matrix_preserves_complete_nonmutation`; `mtgml-environment`: `tests::rejected_environment_submission_preserves_complete_outer_nonmutation`, `tests::semantic_replay_executes_rejected_diagnostic_without_live_recording`; `mtgml-replay`: `tests::replay_recorder_rejects_invalid_append_without_mutation`, `tests::replay_recorder_keeps_rejected_diagnostic_identity`, `tests::rejected_replay_step_must_preserve_the_full_state_digest` |
| `STATE_DELTA_FULL_REAPPLICATION` | `mtgml-state`: `tests::exact_delta_reapplies_every_component`; `mtgml-rules`: `tests::synthetic_m1_acceptance_returns_exact_transition_product` |
| `SEQUENTIAL_EVENT_DELTA_PARITY` | `mtgml-rules`: `tests::two_life_changes_in_one_atomic_transition_are_compositional`, `tests::decision_clear_then_create_composes_to_the_next_decision`, `tests::repeated_tap_changes_in_one_atomic_transition_are_compositional`, `tests::consecutive_zone_incarnations_in_one_transition_are_compositional`, `tests::second_life_event_must_use_cursor_life_after_first_event`, `tests::reversed_dependent_life_events_fail`, `tests::incomplete_life_trace_fails_final_projection`, `tests::event_and_delta_audit_disagreement_fails`, `tests::random_sample_event_is_required_for_final_cursor_progression`, `tests::random_sample_event_and_delta_audit_must_agree` |
| `CHECKPOINT_RESTORE_COMPLETE_IDENTITY` | `mtgml-environment`: `tests::checkpoint_closes_state_status_and_limit_counters`, `tests::checkpoint_digest_covers_status_and_limit_counters`, `tests::checkpoint_captures_m1_5_continuation_state`, `tests::synthetic_backend_checkpoint_captures_the_complete_initial_product`, `tests::synthetic_backend_restore_rejects_state_tampering_without_mutation`, `tests::synthetic_backend_restore_rejects_counter_tampering_without_mutation`, `tests::synthetic_backend_restore_rejects_unsupported_codec_without_mutation`, `tests::checkpoint_restore_repeats_exact_transition_and_replay_segment`, `tests::accepted_state_restore_preserves_identity_and_rebases_empty_replay` |
| `FORK_PARITY` | `mtgml-environment`: `tests::forks_from_a_checkpoint_begin_with_exact_identity_and_empty_segments`, `tests::forks_with_the_same_input_have_exact_continuation_parity`, `tests::forks_diverge_only_on_explicit_accepted_or_rejected_input` |
| `REPLAY_PARITY` | `mtgml-environment`: `tests::semantic_replay_reproduces_the_live_accepted_transition_exactly`, `tests::semantic_replay_executes_rejected_diagnostic_without_live_recording`, `tests::semantic_replay_rejects_counter_divergence_at_first_step`, `tests::semantic_replay_rejects_wrong_initial_digest_before_execution`, `tests::semantic_replay_rejects_wrong_root_seed_before_execution`, `tests::semantic_replay_rejects_tampered_accepted_after_digest_at_first_step`, `tests::semantic_replay_rejects_tampered_accepted_flag_at_first_step`, `tests::semantic_replay_rejects_stale_response_at_first_step`; `mtgml-replay`: `tests::replay_recorder_starts_empty_segment_at_manifest_identity`, `tests::replay_recorder_appends_exact_accepted_step`, `tests::replay_recorder_rejects_invalid_append_without_mutation`, `tests::replay_recorder_keeps_rejected_diagnostic_identity` |
| `DETERMINISTIC_RNG_AND_ALLOCATORS` | `mtgml-random`: `hmac_counter::tests::raw_words_0_to_7_kat`, `hmac_counter::tests::stream_isolation`, `sampling::tests::bound_ten_normative_kat`, `sampling::tests::shuffle_normative_kat`; `mtgml-state`: `tests::effect_allocator_returns_current_id_and_checked_advances`, `tests::effect_allocator_exhaustion_does_not_mutate_allocator`; `mtgml-rules`: `tests::deterministic_services_repeat_exact_transition_result`, `tests::deterministic_services_isolate_named_stream_and_allocator_cursors`, `tests::rng_exhaustion_is_a_typed_internal_failure_without_input_mutation`, `tests::effect_allocator_exhaustion_is_a_typed_internal_failure_before_rng`, `tests::random_sample_event_is_validated_against_authoritative_sampler` |
| `MULTI_PLAYER_ENDPOINT_BINDING` | `mtgml-environment`: `tests::two_player_endpoints_can_remain_alive_simultaneously`, `tests::synthetic_backend_player_surface_projects_two_bound_players`, `tests::wrong_perspective_submission_is_nonmutating_and_shared_p1_submission_advances_both`, `tests::wrong_perspective_submission_is_nonmutating_when_p2_owns_decision`, `tests::non_default_player_ids_remain_bound_through_visibility_rejection_and_step`, `tests::endpoint_submission_matches_trusted_authoritative_checkpoint_and_replay`, `tests::unknown_player_binding_remains_rejected_without_exposing_backend_details`, `tests::player_api_errors_do_not_render_trusted_or_hidden_values`, `tests::successful_player_values_and_errors_do_not_render_trusted_provenance` |

The full workspace command remains a separate regression requirement and is
run in the closure workflow:

```text
cargo test --workspace --all-features --locked
```

It does not replace the focused exact-test evidence above.

## Two-pass and status boundary

The closure branch does not edit a tracked M1 status file because the
repository has no established authoritative one. Therefore no source status
edit invalidates a completed verification pass. The final workflow still runs
the M1 reporter on the committed closure head after all source changes, and
the PR body records only the generated exact-head result. If a maintainer later
chooses to add a tracked status representation, that change must happen only
after Pass A and must trigger a complete Pass B on the new exact head before
`M1 = COMPLETE` is claimed.

## Non-goals

No new `EngineState` fields, decisions, events, RNG semantics, checkpoint or
replay versions, player capabilities, Magic rules/cards, M2 information-state
behavior, Python rules engine, or M2/M3 status claim is introduced.
