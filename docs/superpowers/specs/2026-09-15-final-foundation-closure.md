# Final Foundation Closure

**Status:** evidence record

**Date:** 2026-09-15

**BASE:** 47457cfd66ff5240a2c65d8e1bc699222add39d7

**CODE_VERIFICATION_HEAD:** f271dcc5be82910e7edf5203b9fe1f2bf91f8358

**Issue:** https://github.com/chrismaghuhn/Manafold/issues/164

**Task:** FINAL_FOUNDATION_CLOSURE

**EVIDENCE_INPUT_HEAD:** f271dcc5be82910e7edf5203b9fe1f2bf91f8358

**FINAL_EVIDENCE_HEAD:** recorded externally after the report commit; it is not embedded because this report is part of the reproducible source tree.

## Closure result

~~~text
CANONICAL_FINDINGS_TOTAL = 53
CANONICAL_FINDINGS_DISPOSITIONED = 53
FND_TOTAL = 32
FND_DISPOSITIONED = 32
EVD_TOTAL = 15
EVD_DISPOSITIONED = 15
HRD_TOTAL = 6
HRD_DISPOSITIONED = 6
OPEN_P0_BLOCKERS = 0
OPEN_P1_BLOCKERS = 0
REQUIRED_P2_BLOCKERS = 0
DEFERRED_NONBLOCKING_ITEMS = FND-026C, HRD-006
BLOCKED_NONBLOCKING_ITEMS = FND-026B
FINAL_FOUNDATION_CLOSURE = PASS
~~~

All 53 canonical IDs appear exactly once in the matrix below. Split findings are retained inside their canonical parent row so the Issue #164 inventory remains exactly 32 FND rows, 15 EVD rows, and 6 HRD rows. FND-026B is a reviewed non-actor-product ambiguity with MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = NO; FND-026C is the associated deferred delivery item. Neither is treated as a required foundation blocker.

## Canonical closure matrix

| ID | ORIGINAL_CLASS | FINAL_DISPOSITION | ROOT_CAUSE_OR_SCOPE | CURRENT_OWNER | IMPLEMENTATION_OR_DECISION | EVIDENCE | PR_OR_COMMIT | FREEZE_IMPACT | REMAINING_LIMITATION |
|---|---|---|---|---|---|---|---|---|---|
| FND-001 | P0 | CLOSED | CommanderState.designations was set-like but unordered vectors were digest-significant. | mtgml-state format validation | Batch A validates canonical designation order before digest conversion. | tests::commander_designation_membership_must_be_canonical; current state suite 96/96. | PR #165; merge 47873bdd9b22ec1757959cf98ca71d41263280f7; fix 3fc7c92. | No open blocker. | Commander state remains structural only; no Commander rules or support claim. |
| FND-002 | P0 | CLOSED | Retained knowledge acquisition, history, current/last-known location, and invalidation lacked one temporal chain. | mtgml-state m2_shape knowledge validation | ADR 0049 defines and Batch D implements one shared read-only chronology validator for active and retired records. | fnd_002_* chronology matrix; invalid chronology cannot obtain a V3 digest; current state suite 96/96. | PR #168; merge 9996cfd0fcd4ef67d98cb0422611cebabd20e46b; ADR 0049. | Closed by accepted contract and current evidence. | Chronology is the current synthetic state contract, not future Magic lifecycle semantics. |
| FND-003 | P0 | CLOSED | Detached retained locations could reference undeclared players. | mtgml-state retained knowledge validation | Batch A applies declared-player closure to active, historical, retired, and last-known locations. | every_retained_location_fact_must_reference_a_declared_player; current state suite 96/96. | PR #165; merge 47873bdd9b22ec1757959cf98ca71d41263280f7. | No open blocker. | No broader hidden-information semantics are claimed. |
| FND-004 | P0 | CLOSED | SelectPlayer candidates were checked for visible/trusted equality but not declared-player membership. | mtgml-state decision validation | Batch A rejects undeclared SelectPlayer targets at authoritative state validation. | pending_select_player_must_reference_a_declared_player; current state suite 96/96. | PR #165; merge 47873bdd9b22ec1757959cf98ca71d41263280f7. | No open blocker. | Future player-universe semantics remain outside current M2 scope. |
| FND-005 | P0 | CLOSED | Retired knowledge could exist without its corresponding retired opaque identity marker. | mtgml-state knowledge and perspective identity validation | Batch A closes the required forward join; the reviewed reverse asymmetric case remains allowed. | retired_knowledge_requires_a_matching_retired_identity; retired_opaque_identity_must_not_stay_active; current state suite 96/96. | PR #165; merge 47873bdd9b22ec1757959cf98ca71d41263280f7. | No open blocker. | The accepted contract intentionally does not require the reverse join in every state. |
| FND-006 | P0 | CLOSED | ZoneKey player references and ordered-zone vector/ZonePosition semantics were split across representations; includes 006A and 006B. | mtgml-state zone and retained-location validation | Batch A closes ZoneKey player membership; ADR 0049 and Batch D make ordered_zones authoritative, require Top ordinal offsets, and reject empty keys and Bottom/Index current spellings. | fnd_006b_noncanonical_position_cannot_obtain_a_v3_digest; all fnd_006b_* cases; empty_ordered_zone_keys_must_reference_declared_players. | PR #165 merge 47873bdd9b22ec1757959cf98ca71d41263280f7; PR #168 merge 9996cfd0fcd4ef67d98cb0422611cebabd20e46b7; ADR 0049. | Closed; invalid noncanonical states fail closed. | Bottom and Index remain type variants for compatibility but are invalid current canonical state spellings; no Magic zone legality is added. |
| FND-007 | P0 | RESOLVED_ON_BASE | The lifecycle helper stages local perspective state before complete EngineState validation; the remaining question was ownership of that intermediate seam. | mtgml-state lifecycle plus rules/environment atomic commit boundary | Local provenance/player/orphan closure was fixed in Batch A/B; Batch E confirms the staging seam is intentional and complete state validity remains at atomic transition/checkpoint ownership. | fnd_007_lifecycle_seam_can_stage_before_physical_state_completion; lifecycle_public_seam_must_not_return_ok_with_invalid_full_state; current state/rules/environment suites pass. | PR #165 merge 47873bdd9b22ec1757959cf98ca71d41263280f7; PR #166 merge 85999fb4e8e1cba9dfadba6dc66276899d44bd03; PR #169 merge b24bba153f2aa74bd59e8d6a872a0612ff7f76aa. | No open blocker; only committable candidates are fully validated. | Intermediate staged values are not independently complete EngineState values by design. |
| FND-008 | P0 | RESOLVED_ON_BASE | Transition proof initially covered only selected authoritative mutation families. | mtgml-rules semantic cursor and transition contract | Batch B closes the reachable turn/core unexplained mutation; Batch E maps reachable life/object/tap/decision/RNG/lifecycle families and rejects unsupported unexplained core mutations. | fnd_008_* mutation ownership tests; unexplained_turn_mutation_must_not_pass_transition_contract; current rules suite 45/45. | PR #166 merge 85999fb4e8e1cba9dfadba6dc66276899d44bd03; PR #169 merge b24bba153f2aa74bd59e8d6a872a0612ff7f76aa. | No current-foundation blocker. | Future stack/effect/trigger/priority/format semantics remain unsupported or unreachable. |
| FND-009 | P0 | CLOSED | LifeChanged and ObjectTapped could claim mutation when from equaled to. | mtgml-rules transition contract | Batch F makes these mutation events require distinct before/after values; ZoneTransition retains its existing distinct-incarnation rule. | fnd_009_noop_life_change_is_rejected_by_the_transition_contract; fnd_009_noop_object_tap_is_rejected_by_the_transition_contract; valid mutation controls. | PR #170; merge 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c. | Closed with current transition evidence. | Occurrence-only event families retain their existing semantics. |
| FND-010 | P0 | CLOSED | Revision, global/local allocator, trusted/player decision, continuation, and perspective-local identity progression were not uniformly exact; includes 010A-F. | mtgml-rules semantic cursor and mtgml-state allocators | Batch B requires exact revision plus one, monotonic allocators, fresh decision/continuation identities, and correct perspective ownership. | accepted_revision_must_advance_exactly_once; global_allocator_rewind_must_not_pass_transition_contract; trusted_decision_identity_reuse_must_not_pass_transition_contract; perspective_decision_identity_reuse_must_not_pass_transition_contract; continuation_identity_must_persist_across_staged_decisions; cross_perspective_decision_cursor_inheritance_must_not_pass. | PR #166; merge 85999fb4e8e1cba9dfadba6dc66276899d44bd03. | Closed for reachable M2 identities. | Future identity families are outside this finding and milestone. |
| FND-011 | P0 | CLOSED | Occurrence validation could look ahead to a later physical ZoneTransition. | mtgml-rules occurrence pairing | Batch B makes occurrence pairing sequential and causal rather than satisfied by a later event. | occurrence_must_not_bind_to_a_future_zone_transition; current rules/conformance suites pass. | PR #166; merge 85999fb4e8e1cba9dfadba6dc66276899d44bd03. | Closed. | No general Magic trigger ordering is claimed. |
| FND-012 | P0 | CLOSED | Visible random outcomes needed authoritative sampled-event provenance; AnnouncedOutcome was incorrectly treated as the same subfinding; includes 012A and 012B. | mtgml-rules event/occurrence validation | 012A is closed by immediate RandomValueSampled pairing; 012B is rejected under the accepted policy that AnnouncedOutcome is a separate nonempty presentation occurrence. | visible_random_outcome_requires_authoritative_rng_provenance; announced_outcome_is_its_separate_presentation_occurrence; fnd_012b_empty_announced_outcome_fails_closed. | PR #166 merge 85999fb4e8e1cba9dfadba6dc66276899d44bd03; PR #169 merge b24bba153f2aa74bd59e8d6a872a0612ff7f76aa. | All split subclaims dispositioned; no blocker. | No universal announcement-to-RNG identity is inferred. |
| FND-013 | P0 | RESOLVED_ON_BASE | Direct projection could be mistaken for exact resolver-backed authoritative binding. | mtgml-state authoritative decision validation | Batch F confirms structural request validation and resolver-backed pending-request validation have distinct owners; projection consumes an already authoritative-valid request. | fnd_013_authoritative_state_accepts_exact_binding_controls; fnd_013_authoritative_state_rejects_scalar_binding_mismatches; fnd_013_authoritative_state_rejects_ability_binding_mismatch. | PR #170; merge 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c. | Existing owner boundary is closed; no duplicate resolver authority. | Projection is not a second resolver or binding authority. |
| FND-014 | P0 | CLOSED | Dense CandidateIdV1 assignment had unchecked capacity/ordinal conversion paths. | mtgml-decision dense assignment and public validation | Batch F adds the widened u64 logical bound and typed CandidateCapacityExceeded before enumeration, with checked u32 conversion. | candidate_capacity_uses_the_full_u32_id_domain_without_allocation; dense_assignment_and_public_validation_remain_exact_for_small_inputs; Python matching capacity tests. | PR #170; merge 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c. | Closed fail-closed capacity boundary. | No large allocation or new candidate semantics is introduced. |
| FND-015 | P0 | CLOSED | V2 ObjectMoved allowed both visible identities to be absent. | mtgml-observation V2 event validation | Batch F rejects only old_object=None and new_object=None; partial and dual identities remain valid and V1/wire shape is unchanged. | fnd_015_object_moved_requires_at_least_one_visible_identity; shared negative fixture; Python ObjectMoved parity test. | PR #170; merge 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c. | Closed without schema change. | No new event variant or object identity surface. |
| FND-016 | P0 | CLOSED | PlayerStep rejection/request/status combinations were underconstrained; includes 016A and 016B. | mtgml-observation PlayerStep validator and conformance pre-state oracle | Batch F closes the actor-bound local rejection matrix; Batch H closes the complete independent returned-product proof under EVD-005. | fnd_016a_rejection_matrix_requires_the_correct_decision_presence; semantic_matrix_returns_the_independent_complete_rejected_product; 15-row rejection evidence. | PR #170 merge 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c; PR #172 merge 73b09ee1e78cf411bab39eca5d38658da237f352. | Closed for actor-bound products. | Non-actor product meaning remains separately blocked under FND-026B. |
| FND-017 | P0 | CLOSED | V3 checkpoint digest calculator accepted malformed reference/counter input at its direct boundary; includes 017A and 017B. | mtgml-persistence calculator plus environment/replay checkpoint owners | Batch G closes detached calculator input validation; existing high-level checkpoint/replay owners retain 017B closure. | fnd_017a_rejects_non_v3_full_state_reference_identity; fnd_017a_rejects_impossible_checkpoint_counters; fnd_017b_closed_status_with_pending_decision_is_rejected_at_checkpoint_owner. | PR #171; merge ff37f0896cdbb8e2faea424859faf128155b4579. | Closed; no duplicate checkpoint authority. | Direct helper remains distinct from complete backend checkpoint validation. |
| FND-018 | P0 | CLOSED | Runtime EnvironmentCheckpointV3 exposed an unintended top-level raw Serde persistence surface. | mtgml-environment checkpoint boundary | Batch G removes only the top-level raw Serde surface; nested internal Serde remains unchanged. | fnd_018_environment_checkpoint_is_not_a_raw_serde_surface; source guard and checkpoint/replay suites. | PR #171; merge ff37f0896cdbb8e2faea424859faf128155b4579. | Closed without checkpoint schema/version change. | Nested runtime representation remains internal implementation detail. |
| FND-019 | P0 | CLOSED | Rust/Python persistence precedence differed for compound CBOR resource/depth defects. | mtgml-persistence and Python persistence codec | Batch G aligns both implementations with ADR 0040 precedence. | fnd_019_array_limit_precedes_depth_limit; Python test_fnd_019_array_limit_precedes_depth_limit; current persistence suites pass. | PR #171; merge ff37f0896cdbb8e2faea424859faf128155b4579. | Closed cross-language parity. | Only accepted mechanical persistence categories are covered. |
| FND-020 | P0 | CLOSED | Replay producer identity fields could claim schema/RNG identities not emitted by the current producer. | mtgml-environment synthetic replay producer | Batch E rejects false configured current identities while retaining detached-reader compatibility where required. | fnd_020_current_producer_rejects_a_false_observation_schema_identity; fnd_020_detached_manifest_reader_does_not_require_current_observation_id. | PR #169; merge b24bba153f2aa74bd59e8d6a872a0612ff7f76aa. | Closed producer/reader boundary. | Detached structural readers remain less restrictive by accepted policy. |
| FND-021 | P0 | CLOSED | Replay actor, PlayerDecisionId, and lower-layer identity binding were not uniformly enforced; includes 021A-C. | mtgml-replay detached validation and mtgml-environment backend replay | Batch C binds replay responses to the pending authoritative request; Batch E confirms the detached/verified boundary and existing lower-layer owner. | replay_rejects_wrong_player_decision_id_before_trusted_execution; Replay V3 identity-chain and wrong-actor controls; current replay/environment suites pass. | PR #167 merge 481415542687ac2ca88187184acb52ba7a044ac0; PR #169 merge b24bba153f2aa74bd59e8d6a872a0612ff7f76aa. | Closed. | Detached validation never substitutes for backend verification. |
| FND-022 | P0 | CLOSED | Replay/checkpoint counter, status, terminal, and external resource progression closure was incomplete; includes 022A-E. | mtgml-environment checkpoint/replay and mtgml-replay V3 | Batch C closes initial/closed/deterministic portions; Batch E binds terminal player universes and explicitly applies recorded external counters through the replay-owned checkpoint boundary. | fnd_022b_manifest_requires_the_exact_deck_player_universe_for_closed_status; closed_status_trusted_execution_is_rejected_without_mutation; fnd_022e_replay_applies_recorded_external_counter_progression; fnd_024_external_counter_cannot_invent_a_truncated_status. | PR #167 merge 481415542687ac2ca88187184acb52ba7a044ac0; PR #169 merge b24bba153f2aa74bd59e8d6a872a0612ff7f76aa. | Closed for current V3 policy. | No host-clock reconstruction; unsupported threshold-driven external status changes fail closed. |
| FND-023 | P0 | RESOLVED_ON_BASE | Detached replay validation could be read as proving reconstructed authoritative/player identity. | mtgml-replay validate versus environment execute_replay_from_checkpoint | Batch E documents and tests structural detached validation separately from backend/checkpoint-verified execution. | fnd_023_structural_replay_validation_does_not_verify_backend_state; replay execution identity tests. | PR #169; merge b24bba153f2aa74bd59e8d6a872a0612ff7f76aa. | No open blocker; proof boundary is explicit. | Detached validation does not reconstruct state or prove player projection parity. |
| FND-024 | P0 | RESOLVED_ON_BASE | Live Layer A/B rejection recording and Replay V3 accepted=false identity semantics were inconsistent. | environment endpoint, rules kernel, and replay recorder | Batch E retains the accepted layered policy: wire/player rejections are not authoritative replay steps, trusted recorded rejection preserves complete identity, committed truncation is replayed, aborted service failure is not. | fnd_024_trusted_rejection_is_not_recorded_in_live_replay; fnd_024_external_counter_cannot_invent_a_truncated_status; diagnostic rejected identity tests. | PR #169; merge b24bba153f2aa74bd59e8d6a872a0612ff7f76aa. | No open blocker under accepted layered policy. | No rejected PlayerStep is silently promoted to an authoritative replay step. |
| FND-025 | P0 | CLOSED | Semantically keyed V3 arrays lacked one canonical acceptance policy. | V3 checkpoint/replay boundaries and persistence digest helper | Batch E rejects noncanonical status/deck order at authoritative boundaries while retaining defensive sorting inside checkpoint digest encoding. | fnd_025_checkpoint_rejects_noncanonical_status_order; fnd_025_manifest_rejects_noncanonical_deck_and_status_order; defensive outcome-sort control. | PR #169; merge b24bba153f2aa74bd59e8d6a872a0612ff7f76aa. | Closed. | Shared EpisodeStatus validation remains unchanged; no global ordering semantics added. |
| FND-026 | P0 | BLOCKED_CONTRACT_AMBIGUITY | Multi-perspective delivery contains closed 026A and 026D slices, closed envelope validation, non-actor product ambiguity 026B, and deferred delivery 026C. | mtgml-environment endpoint/product boundary and replay projector | 026A envelope validation and 026D eventful replay reprojection are closed; 026B remains blocked with nonblocking provisional policy; 026C remains DEFERRED_P2. No neutral non-actor PlayerStep, queue, mailbox, poll API, callback, or hidden controller state was added. | multi_perspective_occurrence_batches_are_constructed_and_reprojectable; non_actor_projected_envelope_is_validated_before_commit_boundary; eventful_replay_reprojects_both_perspectives_byte_exactly; Batch E/H classification evidence. | PR #167 merge 481415542687ac2ca88187184acb52ba7a044ac0; PR #169 merge b24bba153f2aa74bd59e8d6a872a0612ff7f76aa; PR #172 merge 73b09ee1e78cf411bab39eca5d38658da237f352. | Nonblocking by reviewed policy: 026B MUST_RESOLVE_BEFORE_FOUNDATION_FREEZE = NO; 026C deferred P2. | A future non-actor product/delivery contract must be separately decided before implementation; this parent row is not a green claim. |
| FND-027 | P0 | RESOLVED_ON_BASE | Lifecycle projection did not itself compare the final identity snapshot, risking duplicate identity authority. | mtgml-rules SemanticValidationCursor and environment lifecycle projector | Batch F confirms the existing semantic cursor owns lifecycle identity replay and complete after-state parity; projector owns redaction/visible-sequence projection only. | fnd_027_final_identity_mismatch_is_rejected_by_the_rules_cursor; fnd_027_identity_snapshot_variants_fail_closed_at_the_existing_owner. | PR #170; merge 04a4831f4fd6e35aa5b6ac315e641b7af238fe9c. | Existing owner boundary is closed. | next_player_decision_id remains the documented excluded cursor field. |
| FND-028 | P0 | CLOSED | PlayerId(0) policy differed across generic identity, declared state, and detached Replay V3 actor validation. | ADR 0050 plus mtgml-replay Rust/Python V3 validators and environment backend | ADR 0050 Option A accepts zero as an ordinary declared identity; PR #174 removes only contradictory numeric-zero guards and preserves declaration/pending-actor checks. | fnd_028_replay_v3_accepts_declared_zero_actor_structurally; fnd_028_declared_zero_player_is_produced_checkpointed_forked_and_replayed; undeclared/wrong-actor/wrong-decision/stale-revision zero controls; Rust/Python wire parity. | PR #173 merge 8018b61416aafbd032df58b7f5cda68dca4f07cb2; PR #174 merge 47457cfd66ff5240a2c65d8e1bc699222add39d7; implementation 5f8d2d5458f734dcf75e129de308528065543b72. | Closed; previous freeze blocker resolved. | Older readers may reject newly admitted zero-actor instances; no historical valid artifact meaning, migration, schema, or replay-version change. |
| FND-029 | P0 | CLOSED | Public raw RNG lane access could panic or lack a typed fail-closed boundary. | mtgml-random checked internal lane extraction | Batch G returns InvalidRawLane without panic or cursor mutation; valid output/KATs are unchanged. | fnd_029_invalid_raw_lane_does_not_panic; production_sampler_consumes_rejected_words_and_advances_the_cursor; current random suite 43/43. | PR #171; merge ff37f0896cdbb8e2faea424859faf128155b4579. | Closed. | Production derives valid lanes through existing bounded logic. |
| FND-030 | P0 | CLOSED | Generated contract bytes depended on host line endings. | scripts/generate_contracts.py and generated-artifact tests | Batch G writes exact UTF-8/LF bytes and compares raw bytes before accepting generated output. | test_raw_byte_check_accepts_exact_lf_target; test_raw_byte_check_rejects_crlf_target; test_write_generated_emits_exact_lf_utf8_bytes; generator check. | PR #171; merge ff37f0896cdbb8e2faea424859faf128155b4579. | Closed determinism/reproducibility issue. | No catalog vocabulary change was introduced by this closure. |
| FND-031 | P0 | CLOSED | Default conformance diagnostics rendered sensitive authoritative values through generic Debug. | mtgml-conformance diagnostics | Batch G uses bounded safe summaries with semantic path, mismatch kind, and presence shape, never trusted values. | fnd_031_default_difference_does_not_render_debug_values; fnd_031_sequence_summaries_preserve_presence_shape_without_values; current conformance suite 143/143. | PR #171; merge ff37f0896cdbb8e2faea424859faf128155b4579. | Closed information-safety issue. | Safe diagnostics do not reveal seeds, trusted IDs, RNG cursors, hidden mappings, or private values. |
| FND-032 | P0 | CLOSED | Commander tax helper inferred designation membership from cast-count ledger presence. | mtgml-commander helper | Batch G checks designation membership first and treats absent ledger count as zero. | fnd_032_designated_without_ledger_entry_has_zero_additional_cost; fnd_032_ledger_entry_without_designation_is_not_designated; current commander suite 4/4. | PR #171; merge ff37f0896cdbb8e2faea424859faf128155b4579. | Closed latent structural defect. | No Commander capability, card, or format support claim. |
| EVD-001 | P1 | CLOSED | Rust did not consume the committed positive persistence corpus and full declared boundary matrix. | mtgml-persistence tests and Python persistence parity | Batch H adds positive manifest consumption and exact CBOR/envelope boundary evidence. | persisted_positive_fixture_manifest_matches_rust_bytes_and_meaning; cbor_resource_boundaries_are_exact_at_each_declared_boundary; Python persistence boundary parity. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | No P1 blocker. | Mechanical persistence evidence does not certify semantic checkpoint content by itself. |
| EVD-002 | P1 | CLOSED | Prior rejection-sampling test used a disconnected loop rather than production uniform_below_u64. | mtgml-random production sampler test | Batch H drives a known rejected raw-word prefix through the production sampler and checks cursor advancement. | production_sampler_consumes_rejected_words_and_advances_the_cursor; five raw words consumed and accepted word used. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | No P1 blocker. | RNG algorithm and KAT meaning remain unchanged. |
| EVD-003 | P1 | CLOSED | Checkpoint/restore/fork evidence could pass on twin equality without independent correctness and source nonmutation. | conformance checkpoint/fork parity harness | Batch H adds exact accepted progression, source fingerprint, restore, fork, and independence assertions. | restore_decision_rich; restore_information_rich; fork_decision_rich; fork_information_rich; cross_mutation_isolation_matrix; accepted_determinism_twins. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | No P1 blocker. | Comparison remains scoped to declared current foundation products. |
| EVD-004 | P1 | CLOSED | Noninterference witnesses allowed cross-axis contamination through existential predicates and marker checks. | conformance isolation state relation | Batch H adds the complete authorized ten-axis relation and rejects contaminated pairs. | paired_rejection_parity_hidden_axes; contaminated_object_rename_plus_life_change_is_rejected; public_order_change_is_rejected_by_hidden_concealed_witness. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | Information-safety proof is executable and nonblocking. | Hidden values remain compared only inside trusted test code; safe output paths omit them. |
| EVD-005 | P1 | CLOSED | Rejection evidence checked codes/fingerprints but not the complete returned PlayerStep independently from pre-state. | conformance rejection oracle and environment endpoint product | Batch H builds the expected rejected PlayerStep from pre-information, pre-decision, pre-status, and row code and compares all 15 actual rows. | semantic_matrix_returns_the_independent_complete_rejected_product; semantic_matrix_fingerprint_stable; 15-row matrix. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | Closes the deferred FND-016B proof slice. | Non-actor submissions remain outside this actor-bound matrix. |
| EVD-006 | P1 | CLOSED | Accepted-transition helper could return a step without requiring accepted status or exact progress before parity comparison. | conformance accepted-entry/count helpers | Batch H requires Accepted and exact revision/counter/semantic progress before parity evidence is used. | accepted-transition proof rows; accepted-entry helper tests; accepted_determinism_twins. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | No P1 blocker. | No new production transition hook is added. |
| EVD-007 | P1 | CLOSED | Some(Vec::new()) could be counted as a represented legal path. | conformance legal-space completeness comparator | Batch H classifies an empty production path as MissingChoice. | empty_production_path_is_missing_choice; live_matrix_exactly_once. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | Legal-space completeness proof is nonvacuous. | Only the bounded synthetic decision space is claimed. |
| EVD-008 | P1 | CLOSED | Invalid complement probes omitted unknown IDs, duplicates, reversed order, wrong variants, and cardinality complements. | conformance legal-space explorer | Batch H adds bounded family-specific invalid probes through real branches and asserts no out-of-contract acceptance. | generate_probes_contains_the_bounded_invalid_complement; live_complement_probes_are_rejected_without_branch_mutation. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | Soundness evidence remains bounded and executable. | No unbounded fuzzing or broad Magic legal-space claim is made. |
| EVD-009 | P1 | CLOSED | Reference transitions could silently ignore invalid choices; budgets and trace drift were incomplete. | conformance legal-space oracle/explorer/comparator | Batch H adds typed budgets, checked reference transitions, independent canonical atoms, bounded Order ranges, and trace-length checks. | reference_source_rejects_silent_invalid_advance; reference_advance_rejects_invalid_choice; violations_fail_closed; reversed_reference_declaration_keeps_expected_request_canonical; request_trace_rejects_extra_observed_request. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | No P1 blocker. | Oracle remains test-only and cannot become production rules authority. |
| EVD-010 | P1 | CLOSED | Eventful endpoint/replay evidence was absent or vacuous for nonempty observed products. | environment eventful test and production replay projector | Batch H uses a real endpoint transaction and production replay reprojection with nonempty P1/P2 batches. | eventful_replay_reprojects_both_perspectives_byte_exactly; real endpoint returns nonempty observed events and replay P1/P2 products are nonempty. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | Closes the P1 eventful evidence block without inventing delivery semantics. | One bounded eventful situation does not claim all future event families. |
| EVD-011 | P1 | CLOSED | Lifecycle fixtures could reidentify the first hidden object rather than a physical card in the retired chain under test. | conformance lifecycle fixture | Batch H binds reidentification to P1's retired physical-card set and preserves old opaque retirement. | reidentification_of_a_randomized_card_uses_fresh_opaque_and_keeps_old_retired; full lifecycle group 10/10. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | No P1 blocker. | Fixture proves the bounded synthetic lifecycle only. |
| EVD-012 | P1 | CLOSED | Fingerprint capture did not prove one atomic trusted instant or retain complete non-secret manifest identity. | conformance fingerprint harness | Batch H binds snapshots to revision and retains manifest, schema, deck, engine/kernel, and digest-reference identities. | snapshot_capture_uses_real_endpoints_and_recomputes_digest; manifest_identity_is_part_of_the_environment_fingerprint; digest_reference_surfaces_preserve_the_declared_domains. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | Determinism/evidence identity proof closed. | Root seed and sensitive internal values remain excluded from diagnostics. |
| EVD-013 | P1 | CLOSED | Mutation guards did not always exercise the named fault rather than only structural negatives. | conformance controlled mutants plus structural guards | Batch H separates structural source guards from controlled mutation outcomes and preserves pinned legacy gate names. | full mutants group 15/15; controlled clean-outcome and source-guard evidence; pinned M2.G names. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | No P1 blocker. | No production mutation/instrumentation seam is introduced. |
| EVD-014 | P1 | CLOSED | Fixture helpers could retain partial workspace/events/RNG mutations after late failure or silently choose a stream. | feature-gated conformance FixtureTransition | Batch H makes movement, occurrence, and RNG fixture operations transactional and requires explicit global SyntheticM1 stream identity. | move_object_rolls_back_when_event_binding_fails; occurrence_rolls_back_when_event_binding_fails; random_sample_rolls_back_cursor_when_event_binding_fails; random_sample_requires_the_declared_global_stream. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | Test-only proof support closed. | Feature-gated fixture support is not a player or replay API. |
| EVD-015 | P1 | CLOSED | Retained-provenance oracle used partial contains assertions and omitted complete fields. | environment EVD-015 test owner and projection evidence | Batch H compares an independently declared complete retained-provenance vector at live, restore, and fork boundaries. | evd_015_retained_provenance_is_complete_and_stable_through_restore_and_fork; exact record/fact/provenance/reason vector. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | Closes final P1 conformance gap. | Test-only environment owner extension; semantic scope change is NO. |
| HRD-001 | P2 | CLOSED | m2_shape header still claimed detached/unreachable status after its types became embedded in current EngineState validation. | mtgml-state module documentation | Current closure updates only the module ownership comment to identify embedded M2 components and central validation composition. | current imports from EngineState and validation::validate_engine_state; cargo fmt/check/test; documentation check. | f271dcc5be82910e7edf5203b9fe1f2bf91f8358. | Required documentation hardening complete. | No state behavior or ownership boundary was changed. |
| HRD-002 | P2 | CLOSED | Rust V2 combined empty-label/range guard returned RandomOutcome for an empty label although V1 and the typed error vocabulary use EmptyEventText. | mtgml-observation V2 validator; Python outer WireError contract | Current closure separates empty-label classification from range classification; Python retains generic semantic.observed_event outer code and no new wire code. | RED: observed_event_v2_random_empty_label_uses_empty_text_error received Err(RandomOutcome); GREEN: same test receives Err(EmptyEventText); V1 control remains green; Python representative rejection code remains semantic.observed_event. | f271dcc5be82910e7edf5203b9fe1f2bf91f8358. | Proof-only typed diagnostic correction; no wire/schema or state semantic change. | Python does not expose Rust internal enum granularity; its accepted generic outer error contract remains unchanged. |
| HRD-003 | P2 | CLOSED | Shared Rust ReplayStepIdentity display text said replay-step.v2 even when V3 validation raised the same enum. | mtgml-replay validation error vocabulary | Current closure changes only the shared display to version-neutral replay-step schema identity is invalid. | RED: V3 mismatch test rendered replay-step.v2; GREEN: replay_step_identity_diagnostic_is_version_neutral passes with neutral text; Python V3 wording was already correct. | f271dcc5be82910e7edf5203b9fe1f2bf91f8358. | Proof-only diagnostic correction; V2 and V3 remain semantically unchanged. | Error enum remains shared by V2/V3 and intentionally does not encode a version-specific message. |
| HRD-004 | P2 | RESOLVED_ON_BASE | Earlier harness diagnostics were too coarse and risked unsafe Debug rendering. | mtgml-conformance diagnostics and evidence harness | Batch G/H already retain safe semantic paths, typed mismatch kinds, first indices, lengths, and presence shape without trusted values. | event_difference_reports_the_first_nonzero_index; event_missing_at_end_is_distinct_from_an_extra_event; first_difference_preserves_declared_precedence; fnd_031_sequence_summaries_preserve_presence_shape_without_values; current conformance suite 143/143. | PR #171 merge ff37f0896cdbb8e2faea424859faf128155b4579; PR #172 merge 73b09ee1e78cf411bab39eca5d38658da237f352. | No required hardening remains. | Diagnostics deliberately do not reveal trusted state, IDs, RNG, seeds, or private knowledge. |
| HRD-005 | P2 | RESOLVED_ON_BASE | Evidence labels/order assertions were weaker than row-for-row declarations. | conformance legal-space and Batch-H evidence tables | Current comparison paths use exact ordered sequences, exact-one path counts, and explicit trace-length checks rather than loose membership. | live_matrix_exactly_once; reversed_reference_declaration_keeps_expected_request_canonical; request_trace_rejects_extra_observed_request; Batch-H bounded 31-row matrix. | PR #172; merge 73b09ee1e78cf411bab39eca5d38658da237f352. | No required hardening remains. | The comparison is bounded to declared synthetic evidence rows. |
| HRD-006 | P2 | DEFERRED_P2 | Adapter-specific completeness gaps remain useful but do not falsify the trusted core gates. | temporary M2.H adapter unit/integration evidence | Existing Rust adapter unit evidence covers uniform unknown-token envelopes, foreign-actor exact-once routing, stale typed submissions, and panic classification; Python fake-child units cover token mismatch, crash, raw bytes, and teardown. The three binary-dependent M2.H scenario modules remain NOT_RUN because MTGML_M2_ADAPTER_BIN is unavailable. | Rust m2-semantic-adapter 22/22; Python full profile 398/398 with 3 M2.H skips; test_token_mismatch_answers_unknown_token_without_sending; test_child_crash_mid_request_raises_transport_closed; test_request_timeout_terminates_child_and_fails_closed; test_raw_byte_level_corruption_classes; current Rust M2.G multi-endpoint and core information gates PASS. | No production change; deferred from the original Issue #164 inventory. | Explicitly nonblocking P2; no required foundation gate is falsified. | Complete adapter binary integration and both-endpoint adapter spying remain follow-up evidence, not a reason to invent architecture or promote this item to a foundation blocker. |

## P2 hardening disposition

~~~text
HRD_001 = CLOSED
HRD_002 = CLOSED
HRD_003 = CLOSED
HRD_004 = RESOLVED_ON_BASE
HRD_005 = RESOLVED_ON_BASE
HRD_006 = DEFERRED_P2
~~~

HRD-006 is not promoted: the unresolved adapter cases are optional evidence completeness work, while the trusted Rust core information-safety, replay, noninterference, rejection, and endpoint gates pass. No new API, transport, queue, mailbox, callback, controller-global state, schema, or semantic redesign was introduced.

## Change-aware review

This was a bounded change-aware review, not a restart of the aborted broad Issue #105 audit.

~~~text
AUDIT_BASE = 219471a1306083d9f8bf22094ecbae5cec3cda06
REVIEW_HEAD = f271dcc5be82910e7edf5203b9fe1f2bf91f8358
FIRST_PARENT_SEQUENCE = PR #161 audit reconciliation -> PR #165 Batch A -> PR #166 Batch B -> PR #167 Batch C -> PR #168 Batch D -> PR #169 Batch E -> PR #170 Batch F -> PR #171 Batch G -> PR #172 Batch H -> PR #173 ADR 0050 -> PR #174 FND-028 -> closure hardening
CHANGED_FILES_AUDIT_BASE_TO_REVIEW_HEAD = 132
~~~

The review checked the remediation diff and current executable owners across state, decisions, events, observations, information state, knowledge, identity, RNG, checkpoint, replay, wire, schemas, Rust/Python parity, conformance, and maintainer tooling. The current diff contains no schema-file change, no Golden-wire fixture change, no Cargo lock change, no card/capability change, and no added production queue/mailbox/poll/callback/controller-global delivery state. The only wire-directory changes in the remediation history are three negative evidence fixtures. No newly added production-source line introduced the forbidden delivery/M3 vocabulary.

The current exact-head gate set below is the executable corroboration for this bounded review. No new concrete correctness, privacy, determinism, replay, decision, or information-boundary defect was found. FND-026B remains explicitly nonblocking by accepted policy; it is not silently reported as closed.

## Exact-head verification

The following results were executed on CODE_VERIFICATION_HEAD f271dcc5be82910e7edf5203b9fe1f2bf91f8358 with a clean worktree.

### Native Rust and Python gates

~~~text
cargo fmt --all -- --check = PASS
cargo check --workspace --all-targets --all-features --locked = PASS
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings = PASS
cargo test --workspace --all-features --locked = PASS; all package result blocks reported zero failures
C:\Python313\python.exe scripts/run_python_tests.py --profile full = PASS; 398 tests OK, skipped=3
.venv\Scripts\python.exe -m ruff format --check python scripts = PASS; 82 files already formatted
.venv\Scripts\python.exe -m ruff check python scripts = PASS
.venv\Scripts\python.exe -m mypy --config-file python/pyproject.toml = PASS; 17 source files
~~~

### Repository, contract, schema, maintainer, and golden gates

~~~text
scripts/generate_contracts.py --check = PASS
scripts/verify_repository.py = PASS; 688 files, 38 golden, 46 negative fixtures
scripts/check_rust_source_structure.py = PASS; 138 files
scripts/check_documentation.py = PASS; 45 ADRs and local links
scripts/validate_schemas.py = PASS; 38 wire fixtures and 9 maintainer artifacts
scripts/validate_maintainer_artifacts.py = PASS; 7 artifacts
scripts/validate_golden_path.py = PASS
scripts/verify_python_toolchain.py = PASS; Python 3.13.15, Rust 1.85.1, 6 direct pins
scripts/run_checks.py fast = PASS
scripts/run_checks.py integration = PASS
scripts/run_checks.py certification = PASS
scripts/run_m2_g_gates.py --expect-commit f271dcc5be82910e7edf5203b9fe1f2bf91f8358 = PASS; all 6 M2.G gates and source identity
scripts/run_m2_final_closure.py --expect-commit f271dcc5be82910e7edf5203b9fe1f2bf91f8358 = PASS; all 20 M2 gates, M1 regression, scope guard, certification, and source identity
scripts/run_verification.py = PASS; 20/20 gates, freeze PASS, source tree unchanged
scripts/verify_archive_reproducibility.py = PASS; 688 safe files, sha256=984ddf27296ef4ae116213fe88a7cb2ce92ae032cc9d3cf6d97c47a508f7e75f
git diff --check = PASS
~~~

Optional M2.H adapter scenario modules are the three expected skips from the full Python profile and remain NOT_RUN because MTGML_M2_ADAPTER_BIN is unavailable. The direct certification profile still passes under the repository's accepted optional-adapter policy; this does not upgrade the skips to PASS.

### Documented just-wrapper results

~~~text
just doctor = BLOCKED; WSL cannot execute /bin/bash
just check-fast = BLOCKED; WSL cannot execute /bin/bash
just check = BLOCKED; WSL cannot execute /bin/bash
just check-all = BLOCKED; WSL cannot execute /bin/bash
just release-candidate = BLOCKED; WSL cannot execute /bin/bash
just archive-check = BLOCKED; WSL cannot execute /bin/bash
~~~

These wrapper results are environmental and are not upgraded to PASS. Their direct native equivalents are recorded separately above and pass.

## Compatibility and scope

~~~text
FROZEN_PUBLIC_API_CHANGE = NO
WIRE_CHANGE = NO
SCHEMA_CHANGE = NO
CHECKPOINT_SCHEMA_VERSION_CHANGE = NO
REPLAY_VERSION_CHANGE = NO
DIGEST_DOMAIN_CHANGE = NO
RNG_ALGORITHM_CHANGE = NO
HISTORICAL_REPLAY_MEANING_CHANGE = NO
HISTORICAL_CHECKPOINT_MEANING_CHANGE = NO
MIGRATION_REQUIRED = NO
CAPABILITY_REGISTRY_CHANGE = NO
COMMANDER_SUPPORT_CLAIM = NO
NEW_MAGIC_SEMANTICS = NO
CARD_IR_CHANGE = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
FOUNDATION_READY_FOR_M3 = NO
PRE_M3_REMEDIATION_FREEZE = NOT_YET_PERFORMED
MERGE_PERFORMED = NO
~~~

The final closure hardening changes are limited to current M2 module documentation, one typed V2 observation diagnostic split, one version-neutral replay diagnostic, their focused Rust regressions, the closure plan, the documentation register, and this evidence record. Valid wire bytes and historical artifacts were not rewritten.

## Final boundary

~~~text
FINAL_FOUNDATION_CLOSURE = PASS
PRE_M3_REMEDIATION_FREEZE = NOT_YET_PERFORMED
FOUNDATION_READY_FOR_M3 = NO
M3_STARTED = NO
M3_AUTHORIZED = NO
ISSUE_164 = OPEN
HOSTED_CI = PENDING
FINAL_FOUNDATION_CLOSURE_REVIEW = PENDING
~~~

This report does not close Issue #164, perform the Pre-M3 remediation freeze, authorize M3, or merge any branch. The final report commit SHA, hosted CI, and independent exact-head review remain external follow-up records.
