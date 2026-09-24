from __future__ import annotations

import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class CurrentStatusEntryPointTests(unittest.TestCase):
    def test_root_readme_exposes_current_milestone_boundaries(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")

        self.assertRegex(
            readme,
            r"\*\*Foundation closure/freeze:\*\* `COMPLETE`",
        )
        self.assertIn(
            "**Current status:** M3.S1 complete / covered / not certified; "
            "M3.S2 complete / implemented / not covered / not certified; PR #208 "
            "is merged and S2 exact-head verification passed",
            readme,
        )
        self.assertNotIn(
            "S1 implementation remains not authorized",
            readme,
        )
        self.assertNotIn(
            "repository freeze/status sync is the pending governance step",
            readme,
        )
        self.assertNotIn(
            "T0 implementation remains blocked pending that separate review",
            readme,
        )
        self.assertIn("**Core modularization:** Issue #162 `COMPLETE`", readme)
        self.assertIn(
            "**M3 authorization:** `AUTHORIZED` at `ea668c47ef1361b3d989fd32b8f3cfd4751b1e79`",
            readme,
        )
        self.assertIn("**M3 milestone execution:** `STARTED`", readme)
        self.assertIn(
            "**P0:** `COMPLETE / FROZEN` "
            "(reviewed head `a7e641a7e6145610c9533187cf6340712f460e44`, "
            "merge commit `20dac927027776ef5f0a5b389a27d4a05eefb180`)",
            readme,
        )
        self.assertIn("**M3.T0:** `COMPLETE / FROZEN`", readme)
        self.assertIn("`b403edefcabf7b304c0fa5f6816d22ac8aca477b`", readme)
        self.assertIn("`b9c5f2be97b8fc1f31d648d58f890de78f0a035c`", readme)
        self.assertNotIn("COMPLETE-CANDIDATE / FREEZE-ELIGIBLE", readme)
        self.assertNotIn("freeze not yet executed or tracked", readme)
        self.assertIn("**M3.S1:** `COMPLETE / COVERED / NOT CERTIFIED`", readme)
        self.assertIn("`587016574e4e8f9f797a713877f8caf1c5143cfb`", readme)
        self.assertNotIn("SELECTED / AUTHORIZATION-ELIGIBLE / NOT_AUTHORIZED", readme)
        self.assertIn(
            "**M3 semantic implementation:** M3.S1 is `COVERED`; bounded M3.S2 "
            "zone-incarnation is `IMPLEMENTED` and not covered",
            readme,
        )
        self.assertIn(
            "**M3.S2:** `COMPLETE / IMPLEMENTED / NOT COVERED / NOT CERTIFIED`",
            readme,
        )
        self.assertIn("**PR #208:** `MERGED`; `S2_EXACT_HEAD_VERIFICATION = PASS`", readme)
        self.assertIn(
            "**S2 authoritative replay:** Draw × S2 has a Block 3 Replay V6 candidate witness; "
            "overall S2 replay coverage remains `DEFERRED_REQUIRED / BLOCKED_FOR_COVERED`",
            readme,
        )
        self.assertIn("**M3 Pre-T0 hardening:** `COMPLETE / ACCEPTED`", readme)
        self.assertIn("ADR 0054 = ACCEPTED", readme)
        self.assertIn("FOUNDATION_V2 = ACCEPTED_HARDENED_M3_SCOPE", readme)
        self.assertIn("**M3 plan status:** `ACCEPTED`", readme)
        self.assertIn("**Task 14:** `COMPLETE` — `S1_EXACT_HEAD_VERIFICATION = PASS`", readme)
        self.assertIn("**PR #213:** `MERGED` at `60b6ee7957032a36371ceac89c3a1e4f886d200c`", readme)
        self.assertIn("**S3.P0:** `COMPLETE / FROZEN`", readme)
        self.assertIn("**S3.0:** `COMPLETE / FROZEN`", readme)
        self.assertIn(
            "**Magic rules-flow inventory:** `REVIEWED / FROZEN_PLANNING_INPUT`",
            readme,
        )
        self.assertIn("**M3 Block 1:** bounded S3.A is accepted / merged", readme)
        self.assertIn(
            "**M3 Block 2:** Basic Priority + Reference response integration is accepted / merged",
            readme,
        )
        self.assertIn(
            "**M3 Block 3:** Draw + S2 replay/interaction implementation candidate complete; "
            "exact-head review pending",
            readme,
        )
        self.assertIn("Combat and later blocks have not started", readme)
        self.assertNotIn("M3_S1_AUTHORIZATION_DECISION", readme)
        self.assertNotIn("M3_T0_CLOSURE_STATUS_SYNC_EXACT_HEAD_REVIEW", readme)
        self.assertIn(
            "**M3 hardening acceptance:** PR #184 merged and accepted ADR 0054/Foundation V2",
            readme,
        )
        self.assertIn(
            "T0 was reauthorized under Issue #178, implemented by merged PRs #189/#190/#191",
            readme,
        )
        self.assertIn("finalized as COMPLETE / FROZEN", readme)
        self.assertIn(
            "9 Foundation capabilities are `specified`, 1 is `implemented`, "
            "1 is `covered`, and 0 are `certified`",
            readme,
        )
        self.assertIn(
            "**Current boundary:** S2 remains `IMPLEMENTED / NOT COVERED`; Block 3 supplies a "
            "Draw × S2 Replay V6 candidate witness but does not close all S2 coverage gates",
            readme,
        )
        self.assertIn(
            "**Real Magic semantics:** S1 is covered; S2 is implemented / not covered; "
            "bounded S3.A and S3.B implementations are accepted / merged under distinct "
            "production identities",
            readme,
        )
        self.assertNotIn("S2 covered", readme)
        self.assertNotIn("S2 certified", readme)
        self.assertIn("**Playable engine:** no", readme)
        self.assertIn("**Real card support:** none", readme)
        self.assertIn("**Current resumable execution contract:** V6.", readme)
        self.assertIn("`EnvironmentCheckpointV6`", readme)
        self.assertIn("`CheckpointDigestV6`", readme)
        self.assertRegex(readme, r"V4/V5 artifacts retain their historical\s+meanings")
        self.assertNotIn("M3 Pre-T0 plan hardening under Issue #178", readme)
        self.assertNotIn("HARDENED_PLAN_MERGE_AND_EXACT_MASTER_REAUTHORIZATION", readme)
        self.assertNotIn(
            "Current active work area:** pre-M3 foundation reconciliation and "
            "adversarial audit preparation under Issue #105",
            readme,
        )
        self.assertNotIn("maintainer hardening under Issue #130", readme)
        self.assertNotIn("Batch A5/C1", readme)
        self.assertRegex(readme, r"M2\.5[^\n]*NOT_CLAIMED")
        self.assertRegex(readme, r"M3\.S1[^\n]*COVERED / NOT CERTIFIED")
        self.assertNotIn("NOT_AUTHORIZED", readme)
        self.assertNotIn("M2 is not complete", readme)

    def test_current_entry_points_reference_status_owners_and_required_gates(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        docs_readme = (ROOT / "docs" / "README.md").read_text(encoding="utf-8")
        profile = (ROOT / "docs" / "maintenance" / "MAINTAINER_PROFILES.md").read_text(
            encoding="utf-8"
        )

        self.assertIn("docs/ROADMAP.md", readme)
        self.assertIn("docs/maintenance/MAINTAINER_PROFILES.md", readme)
        self.assertIn("README.md", docs_readme)
        self.assertIn("ROADMAP.md", docs_readme)
        self.assertIn("Current status is owned by the repository root", docs_readme)
        self.assertIn("intentionally does not restate current milestone", docs_readme)
        self.assertNotIn("engine M3 as `NOT_AUTHORIZED`", docs_readme)
        for gate in ("PR Fast", "manafold-pr-gate"):
            with self.subTest(gate=gate):
                self.assertIn(gate, profile)
        self.assertRegex(profile, r"PR\s+Integration")

    def test_roadmap_preserves_external_scope_and_engine_m3_boundary(self) -> None:
        roadmap = (ROOT / "docs" / "ROADMAP.md").read_text(encoding="utf-8")

        self.assertRegex(roadmap, r"## M2 [^\n]+\n\n\*\*Status:\*\* `COMPLETE`")
        self.assertIn("`M2.5 = NOT_CLAIMED / NOT_FROZEN`", roadmap)
        self.assertIn("FINAL_FOUNDATION_CLOSURE = PASS", roadmap)
        self.assertIn("PRE_M3_REMEDIATION_FREEZE = PASS", roadmap)
        self.assertIn("CORE_MODULARIZATION = COMPLETE", roadmap)
        self.assertIn("GOVERNANCE_CLEANUP = COMPLETE", roadmap)
        self.assertIn("M3_STARTED = YES", roadmap)
        self.assertIn("M3_AUTHORIZED = YES", roadmap)
        self.assertIn("M3_PRE_T0_HARDENING = COMPLETE / ACCEPTED", roadmap)
        self.assertIn("M3_PLAN_STATUS = ACCEPTED", roadmap)
        self.assertIn("ADR_0054 = ACCEPTED", roadmap)
        self.assertIn("FOUNDATION_V2 = ACCEPTED", roadmap)
        self.assertIn("M3_S1 = rules/turn-structure@0.1.0", roadmap)
        self.assertEqual(
            roadmap.count("M3_S1 = rules/turn-structure@0.1.0"),
            1,
        )
        self.assertIn("M3_S1_AUTHORIZATION_REVIEW = APPROVE", roadmap)
        self.assertIn(
            "S1_AUTHORIZATION_HEAD =\n587016574e4e8f9f797a713877f8caf1c5143cfb",
            roadmap,
        )
        self.assertIn(
            "AUTHORIZATION_HEAD = ea668c47ef1361b3d989fd32b8f3cfd4751b1e79",
            roadmap,
        )
        self.assertIn("S1_AUTHORIZATION_ELIGIBLE = YES", roadmap)
        self.assertIn("S1_AUTHORIZED = YES", roadmap)
        self.assertIn("S1_IMPLEMENTATION_AUTHORIZED = YES", roadmap)
        self.assertIn("S1_STARTED = YES", roadmap)
        self.assertIn("S1_IMPLEMENTATION_STARTED = YES", roadmap)
        self.assertIn("S1_TURN_STRUCTURE_LIFECYCLE = covered", roadmap)
        self.assertIn("S1_COVERAGE_STATUS = covered / certification not claimed", roadmap)
        self.assertIn("S1_SPECIFIED_CAPABILITY_COUNT = 10", roadmap)
        self.assertIn("S1_IMPLEMENTED_CAPABILITY_COUNT = 0", roadmap)
        self.assertIn("S1_COVERED_CAPABILITY_COUNT = 1", roadmap)
        self.assertIn("S1_CERTIFIED_CAPABILITY_COUNT = 0", roadmap)
        self.assertIn("CURRENT_RESUMABLE_EXECUTION_CONTRACT = V6", roadmap)
        self.assertIn(
            "V4_V5_RESUMABLE_CONTRACT_STATUS = HISTORICAL_ONLY / V5_TO_V6_MIGRATION_NONE",
            roadmap,
        )
        self.assertIn("TASK_13_DOCUMENTATION_STATUS_CLOSURE = COMPLETE", roadmap)
        self.assertIn("TASK_14_EXACT_HEAD_VERIFICATION = COMPLETE", roadmap)
        self.assertIn("S1_EXACT_HEAD_VERIFICATION = PASS", roadmap)
        self.assertIn("M3_S1_STATUS = COMPLETE / COVERED / NOT CERTIFIED", roadmap)
        self.assertIn("M3_S2 = rules/zone-incarnation@0.1.0", roadmap)
        self.assertIn("M3_S2_AUTHORIZED = YES", roadmap)
        self.assertIn("S2_IMPLEMENTATION_AUTHORIZED = YES", roadmap)
        self.assertIn("S2_STARTED = YES", roadmap)
        self.assertIn("S2_IMPLEMENTATION_STARTED = YES", roadmap)
        self.assertIn("S2_ZONE_INCARNATION_LIFECYCLE = implemented", roadmap)
        self.assertIn("S2_COVERED = NO", roadmap)
        self.assertIn("S2_CERTIFIED = NO", roadmap)
        self.assertIn(
            "S2_COVERAGE_STATUS = implemented / covered blocked by authoritative replay",
            roadmap,
        )
        self.assertIn("S2_SPECIFIED_CAPABILITY_COUNT = 9", roadmap)
        self.assertIn("S2_IMPLEMENTED_CAPABILITY_COUNT = 1", roadmap)
        self.assertIn("S2_COVERED_CAPABILITY_COUNT = 1", roadmap)
        self.assertIn("S2_CERTIFIED_CAPABILITY_COUNT = 0", roadmap)
        self.assertIn("S2_AUTHORITATIVE_REPLAY = DEFERRED_REQUIRED / BLOCKED_FOR_COVERED", roadmap)
        self.assertIn("S2_STATE_BASED_ACTIONS_INTERACTION = UNSATISFIED", roadmap)
        self.assertIn("S2_DRAW_CARD_INTERACTION = UNSATISFIED", roadmap)
        self.assertIn("TASK_7_LIFECYCLE_PROMOTION = COMPLETE", roadmap)
        self.assertIn("PR_208 = MERGED", roadmap)
        self.assertIn("S2_EXACT_HEAD_VERIFICATION = PASS", roadmap)
        self.assertIn(
            "M3_S2_STATUS = COMPLETE / IMPLEMENTED / NOT COVERED / NOT CERTIFIED", roadmap
        )
        self.assertIn("TASK_8_EXACT_HEAD_VERIFICATION = COMPLETE", roadmap)
        self.assertIn("S1_AUTHORIZED_TASK_AT_S1_HEAD = M3.S1", roadmap)
        self.assertIn("S1_REVIEW_MINORS = 3 CARRIED", roadmap)
        self.assertIn(
            "S1_SUPPORT_PREDICATE_REQUIRES_EXACTLY_TWO_PLAYERS = YES",
            roadmap,
        )
        self.assertIn("S1_ZONE_LOCATION_MUTATION_AUTHORIZED = NO", roadmap)
        self.assertIn("S1_OBJECT_INCARNATION_CHANGE_AUTHORIZED = NO", roadmap)
        self.assertIn("S1_PHYSICAL_CARD_IDENTITY_CHANGE_AUTHORIZED = NO", roadmap)
        self.assertNotIn("S1_IMPLEMENTATION = NOT_AUTHORIZED", roadmap)
        self.assertNotIn("NEXT_GATE = M3_S1_AUTHORIZATION_DECISION", roadmap)
        self.assertIn("S3_P0_IMPLEMENTATION = COMPLETE", roadmap)
        self.assertIn("S3_P0_COMPLETE = YES", roadmap)
        self.assertIn("S3_P0_FROZEN = YES", roadmap)
        self.assertIn("S3_P0_PR = #210", roadmap)
        self.assertIn("S3_P0_MERGE_COMMIT = ffc433985f41e6e2980df23a103b5e2358527ea3", roadmap)
        self.assertIn("S3_0_AUTHORIZED = YES", roadmap)
        self.assertIn("S3_0_STARTED = YES", roadmap)
        self.assertIn("S3_0_COMPLETE = YES", roadmap)
        self.assertIn("S3_0_FROZEN = YES", roadmap)
        self.assertIn("S3_0_PR = #211", roadmap)
        self.assertIn("S3_0_MERGE_COMMIT = 66f3b713787cad89674257f6e0b6448b9fd568f9", roadmap)
        self.assertIn("S3_A_AUTHORIZED = YES", roadmap)
        self.assertIn("S3_A_STARTED = YES", roadmap)
        self.assertIn("S3_A_TASK_5 = COMPLETE / REVIEWED", roadmap)
        self.assertIn("S3_A_TASK_6 = COMPLETE / REVIEWED", roadmap)
        self.assertIn("S3_A_TASK_6_REVIEW_HEAD = 5711886772e431a80f81bcd099cb5546d327a362", roadmap)
        self.assertIn("S3_A_TASK_7 = COMPLETE / REVIEWED", roadmap)
        self.assertIn("S3_A_TASK_7_REVIEW_HEAD = eb9c9a92d60bf82bccc0ceeb174ad58e1f8c1d48", roadmap)
        self.assertIn("S3_A_TASK_8 = COMPLETE / REVIEWED", roadmap)
        self.assertIn("S3_A_TASK_8_REVIEW_HEAD = 8a531c33ced9e532bd90b3870ac088b06924c348", roadmap)
        self.assertIn("S3_A_TASK_9A = COMPLETE / REVIEWED", roadmap)
        self.assertIn(
            "S3_A_TASK_9A_REVIEW_HEAD = 28ffe32b8acb72c0ec3cca98bcfbd90499827f5b", roadmap
        )
        self.assertIn("S3_A_TASK_9B0 = COMPLETE / REVIEWED", roadmap)
        self.assertIn("S3_A_TASK_9 = COMPLETE / REVIEWED", roadmap)
        self.assertIn("S3_A_BLOCK_1 = COMPLETE / REVIEWED", roadmap)
        self.assertIn("S3_A_IMPLEMENTATION = COMPLETE / REVIEWED", roadmap)
        self.assertIn("S3_A_EXACT_HEAD_REVIEW = PASS", roadmap)
        self.assertIn("S3_A_PRODUCTION_SEMANTIC_CONTRACT_REQUIRED = YES", roadmap)
        self.assertIn("S3_A_PRODUCTION_SEMANTIC_CONTRACT_ALLOCATED = YES", roadmap)
        self.assertIn(
            "NONTERMINAL_FINAL_ORDER_REFERENCE_PATH = PASS",
            roadmap,
        )
        self.assertIn("MAGIC_RULES_FLOW_INVENTORY_V1 = REVIEWED / FROZEN_PLANNING_INPUT", roadmap)
        self.assertIn("M3_MAJOR_SEMANTIC_BLOCKS = 8", roadmap)
        self.assertIn("M3_BLOCK_1 = COMPLETE / REVIEWED", roadmap)
        self.assertIn("M3_BLOCK_1_REVIEW_HEAD = 6fc3ff9694aa9d61975929a2d4a1c8006df28014", roadmap)
        self.assertIn("M3_BLOCK_2 = COMPLETE / FINAL ACCEPTANCE PASS / MERGED", roadmap)
        self.assertIn("M3_BLOCK_3 = COMPLETE_CANDIDATE", roadmap)
        self.assertIn(
            "CURRENT_M3_BLOCK = DRAW_AND_S2_REPLAY_INTERACTION", roadmap
        )
        self.assertIn("BLOCK_3_STARTED = YES", roadmap)
        self.assertIn("S3_C_AUTHORIZED = YES", roadmap)
        self.assertIn("S3_C_IMPLEMENTATION_AUTHORIZED = YES", roadmap)
        self.assertIn("NEXT_GATE = M3_BLOCK_3_EXACT_HEAD_REVIEW", roadmap)
        self.assertIn("S3_A_IMPLEMENTATION_AUTHORIZED = YES", roadmap)
        self.assertIn("S3_B_AUTHORIZED = YES", roadmap)
        self.assertIn("S3_B_IMPLEMENTATION_AUTHORIZED = YES", roadmap)
        self.assertNotIn("M3_S2_AUTHORIZED = NO", roadmap)
        self.assertNotIn("S2_IMPLEMENTED = NO", roadmap)
        self.assertNotIn("M3.S2 has not been selected or", roadmap)
        self.assertNotIn("eligible for a separate authorization decision", roadmap)
        self.assertNotIn("S1 implementation remains NOT_AUTHORIZED", roadmap)
        self.assertIn(
            "AUTHORIZED_TASK_AT_AUTHORIZATION_HEAD = M3.P0_STATE_IDENTITY_CUT",
            roadmap,
        )
        self.assertIn(
            "P0_REVIEW_HEAD = a7e641a7e6145610c9533187cf6340712f460e44",
            roadmap,
        )
        self.assertIn(
            "P0_MERGE_COMMIT = 20dac927027776ef5f0a5b389a27d4a05eefb180",
            roadmap,
        )
        self.assertIn("P0_EXACT_HEAD_REVIEW = APPROVE", roadmap)
        self.assertIn("P0_COMPLETE = YES", roadmap)
        self.assertIn("P0_FROZEN = YES", roadmap)
        self.assertIn("T0_AUTHORIZED = YES", roadmap)
        self.assertIn("T0_STARTED = YES", roadmap)
        self.assertIn("T0_01 = COMPLETE / MERGED (PR #189", roadmap)
        self.assertIn("T0_02A = COMPLETE / MERGED (PR #190", roadmap)
        self.assertIn("T0_02B = COMPLETE / MERGED (PR #191", roadmap)
        self.assertIn(
            "T0_CLOSURE_REVIEW_HEAD = b403edefcabf7b304c0fa5f6816d22ac8aca477b",
            roadmap,
        )
        self.assertIn("T0_COMPLETE = YES", roadmap)
        self.assertIn("T0_FROZEN = YES", roadmap)
        self.assertIn("T0_FREEZE_EXECUTED = YES", roadmap)
        self.assertIn(
            "T0_STATUS_SYNC_MERGE =\nb9c5f2be97b8fc1f31d648d58f890de78f0a035c",
            roadmap,
        )
        self.assertNotIn("T0_COMPLETE_CANDIDATE = YES", roadmap)
        self.assertNotIn("T0_FREEZE_ELIGIBLE = YES", roadmap)
        self.assertNotIn("T0_FREEZE_EXECUTED = NO", roadmap)
        self.assertNotIn("M3_T0_CLOSURE_STATUS_SYNC_EXACT_HEAD_REVIEW", roadmap)
        self.assertNotIn("tracker finalization remain as the pending", roadmap)
        self.assertNotIn("M3_PRE_T0_HARDENING = CANDIDATE", roadmap)
        self.assertNotIn("M3_ENTRY_DECISION = ACCEPTED_BUT_UNDER_PRE_T0_HARDENING", roadmap)
        self.assertNotIn("HARDENED_PLAN_MERGE_AND_EXACT_MASTER_REAUTHORIZATION", roadmap)
        self.assertIn(
            "M3.P0 semantic-neutral state/persistence identity cut\n"
            "→ M3.T0 thin private conformance facade\n"
            "→ M3.S1 rules/turn-structure@0.1.0 covered\n"
            "→ Task 13 documentation/status/generated-contract closure complete\n"
            "→ Task 14 exact-head verification COMPLETE\n"
            "→ M3.S2 rules/zone-incarnation@0.1.0 implemented / not covered\n"
            "→ Task 7 lifecycle promotion COMPLETE\n"
            "→ Task 8 exact-head verification PASS (PR #208 merged)\n"
            "→ M3.S3 selection and design",
            roadmap,
        )
        self.assertIn("M3.P0 semantic-neutral state/persistence identity cut", roadmap)
        self.assertIn("PR #184", roadmap)
        self.assertIn(
            "M3 has started through semantic-neutral P0 infrastructure, T0",
            roadmap,
        )
        self.assertIn(
            "T0 was\nreauthorized under Issue #178 and implemented by merged PRs #189 (T0-01),",
            roadmap,
        )
        self.assertIn("Issue #178 finalized T0 as", roadmap)
        self.assertNotIn("freeze-eligible, pending repository freeze/status sync", roadmap)
        self.assertNotIn("freeze-eligible). Repository freeze/status", roadmap)
        self.assertNotIn("T0 remains separately", roadmap)
        self.assertNotIn("T0 reauthorization remains separate", roadmap)
        self.assertNotIn(
            "M3 has started only in the sense that the semantic-neutral P0",
            roadmap,
        )
        self.assertNotIn("T0_AUTHORIZED = NO", roadmap)
        self.assertNotIn("T0_STARTED = NO", roadmap)
        self.assertNotIn("M3.T0 is `NOT_STARTED / NOT_AUTHORIZED`", roadmap)
        self.assertIn("P0 is complete and frozen", roadmap)
        self.assertIn("Issue #105", roadmap)
        self.assertNotIn("current active maintainer work area is Issue\n#130", roadmap)
        self.assertIn("Census-driven scope", roadmap)
        self.assertRegex(roadmap, r"outside\s+this authoritative\s+engine repository")

    def test_current_status_does_not_duplicate_a_second_status_file(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        self.assertNotRegex(
            readme,
            r"(?:CURRENT_STATUS|PROJECT_STATE|status\.json)",
        )

    def test_v6_cut_keeps_old_checkpoint_identities_out_of_current_claims(self) -> None:
        # Capability/milestone lifecycle pins stay as reviewed while the
        # resumable state/checkpoint/replay identity family advances.
        for path in (
            ROOT / "README.md",
            ROOT / "docs" / "ROADMAP.md",
        ):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("EnvironmentCheckpointV4", text)
            self.assertNotIn("CheckpointDigestV4", text)
            self.assertNotIn("calculate_checkpoint_digest_v4", text)

    def test_foundation_registry_tracks_s1_coverage_and_bounded_s2_implementation(self) -> None:
        registry = json.loads(
            (ROOT / "cards" / "capabilities" / "registry.json").read_text(encoding="utf-8")
        )
        expected = {
            "rules/basic-priority",
            "rules/cleanup-reset",
            "rules/combat-damage",
            "rules/combat-phase",
            "rules/declare-attackers",
            "rules/declare-blockers",
            "rules/damage-and-life",
            "rules/draw-card",
            "rules/state-based-actions-combat",
            "rules/turn-structure",
            "rules/zone-incarnation",
        }
        entries = registry["entries"]
        self.assertEqual({entry["key"] for entry in entries}, expected)
        self.assertEqual(len(entries), 11)
        turn_structure = next(entry for entry in entries if entry["key"] == "rules/turn-structure")
        zone_incarnation = next(
            entry for entry in entries if entry["key"] == "rules/zone-incarnation"
        )
        draw_card = next(entry for entry in entries if entry["key"] == "rules/draw-card")
        other_entries = [
            entry
            for entry in entries
            if entry not in (turn_structure, zone_incarnation, draw_card)
        ]

        self.assertEqual(turn_structure["version"], "0.1.0")
        self.assertEqual(turn_structure["lifecycle"], "covered")
        self.assertTrue(turn_structure["implementation_paths"])
        self.assertEqual(
            turn_structure["conformance_cases"],
            [
                "s1.temporal.successor-table",
                "s1.untap.to-upkeep",
                "s1.cleanup.next-turn",
                "s1.deterministic-rerun",
                "s1.untap.one-active-tapped",
                "s1.untap.multiple-active-tapped",
                "s1.untap.already-untapped",
                "s1.untap.nonactive-control",
                "s1.untap.narrow-mutation",
                "s1.admission.not-two-players",
                "s1.admission.priority-held",
                "s1.admission.unsupported-profile",
                "s1.downstream.upkeep-priority",
                "s1.downstream.draw",
                "s1.downstream.combat",
                "s1.cleanup.reset-required",
                "s1.turn-number-overflow",
                "s1.invalid-temporal-state",
                "s1.fabricated-response",
                "s1.catalog.exact-closure",
                "s1.catalog.wrong-closure",
                "s1.catalog.program-pairing",
                "s1.observation.temporal-fields",
                "s1.observation.public-untap",
            ],
        )
        self.assertEqual(turn_structure["benchmark_scenarios"], [])
        self.assertEqual(
            turn_structure["spec_path"],
            "docs/superpowers/specs/2026-09-21-m3-s1-turn-structure-design.md",
        )

        self.assertEqual(zone_incarnation["version"], "0.1.0")
        self.assertEqual(zone_incarnation["lifecycle"], "implemented")
        self.assertEqual(
            zone_incarnation["spec_path"],
            "docs/superpowers/specs/2026-09-23-m3-s2-zone-incarnation-design.md",
        )
        self.assertEqual(
            zone_incarnation["implementation_paths"],
            [
                "crates/mtgml-rules/src/zone_incarnation.rs",
                "crates/mtgml-rules/src/contract.rs",
                "crates/mtgml-rules/src/semantic_cursor.rs",
                "crates/mtgml-state/src/identity.rs",
            ],
        )
        self.assertEqual(
            zone_incarnation["conformance_cases"],
            [
                "s2.zone.battlefield_graveyard",
                "s2.zone.library_hand_top",
                "s2.zone.graveyard_order",
                "s2.zone.delta_reapplication",
                "s2.identity.public_remap",
                "s2.identity.owner_hand_private",
                "s2.identity.old_reference_closure",
                "s2.identity.foundation_source_cessation",
                "s2.identity.destination_canonical_state",
                "s2.observation.lifecycle_matrix",
                "s2.observation.library_noninterference",
                "s2.rejection.source_absent",
                "s2.rejection.source_location",
                "s2.rejection.unadmitted_family",
                "s2.rejection.wrong_owner_destination",
                "s2.rejection.unsupported_profile",
                "s2.rejection.library_not_top",
                "s2.rejection.stale_old_incarnation",
                "s2.state.object_id_exhaustion",
                "s2.state.invalid_before_state",
                "s2.mutant.object_id_reuse",
                "s2.mutant.physical_continuity",
                "s2.mutant.snapshot_pairing",
                "s2.mutant.lifecycle_pairing",
                "s2.mutant.delta_pairing",
                "s2.mutant.graveyard_order",
                "s2.mutant.old_reference",
                "s2.mutant.stale_old_lifecycle_occurrence",
                "s2.replay.checkpoint_restore",
                "s2.replay.fork",
                "s2.replay.rerun",
            ],
        )
        self.assertEqual(zone_incarnation["benchmark_scenarios"], [])
        self.assertEqual(zone_incarnation["dependencies"], [])
        self.assertEqual(zone_incarnation["information_risk"], "high")
        self.assertEqual(zone_incarnation["owners"], ["zones_identity"])
        self.assertNotIn("s2.replay.authoritative", zone_incarnation["conformance_cases"])
        self.assertIn("fail closed", zone_incarnation["notes"])
        self.assertIn("DEFERRED_REQUIRED", zone_incarnation["notes"])

        self.assertEqual(draw_card["lifecycle"], "specified")
        self.assertEqual(
            draw_card["conformance_cases"],
            [
                "m3.draw.upkeep-pass-s2-priority-replay",
                "m3.draw.hidden-world-noninterference",
                "m3.draw.rejection-atomicity",
            ],
        )
        self.assertEqual(
            draw_card["implementation_paths"],
            [
                "crates/mtgml-rules/src/magic.rs",
                "crates/mtgml-rules/src/contract.rs",
                "crates/mtgml-rules/src/product.rs",
                "crates/mtgml-environment/src/reference.rs",
                "crates/mtgml-environment/src/tests/s3_a_production.rs",
            ],
        )
        self.assertIn("lifecycle remains specified", draw_card["notes"])

        for entry in other_entries:
            with self.subTest(capability=entry["key"]):
                self.assertEqual(entry["version"], "0.1.0")
                self.assertEqual(entry["lifecycle"], "specified")
                self.assertEqual(entry["implementation_paths"], [])
                self.assertEqual(entry["conformance_cases"], [])
                self.assertEqual(entry["benchmark_scenarios"], [])

        self.assertEqual(sum(entry["lifecycle"] == "specified" for entry in entries), 9)
        self.assertEqual(sum(entry["lifecycle"] == "implemented" for entry in entries), 1)
        self.assertEqual(sum(entry["lifecycle"] == "covered" for entry in entries), 1)
        self.assertEqual(sum(entry["lifecycle"] == "certified" for entry in entries), 0)
        self.assertEqual(
            sum(len(entry.get("dependencies", [])) for entry in entries),
            13,
        )

    def test_plan_acceptance_is_separate_from_execution_authorization(self) -> None:
        adr_index = (ROOT / "docs" / "adr" / "README.md").read_text(encoding="utf-8")
        adr = (ROOT / "docs" / "adr" / "0054-m3-pre-t0-hardening.md").read_text(encoding="utf-8")
        foundation = (ROOT / "docs" / "rules" / "M3_INITIAL_SEMANTIC_FOUNDATION_V2.md").read_text(
            encoding="utf-8"
        )
        register = json.loads(
            (ROOT / "docs" / "normative-document-register.v1.json").read_text(encoding="utf-8")
        )
        foundation_registration = next(
            entry
            for entry in register["documents"]
            if entry["path"] == "docs/rules/M3_INITIAL_SEMANTIC_FOUNDATION_V2.md"
        )

        self.assertIn("accepted base sequence currently runs through ADR 0055", adr_index)
        self.assertIn("ADR_0054 = ACCEPTED\nFOUNDATION_V2 = ACCEPTED_HARDENED_M3_SCOPE", adr_index)
        self.assertNotIn("current numbered hardening acceptance candidate", adr_index)
        self.assertEqual(foundation_registration["role"], "normative")
        self.assertEqual(foundation_registration["stability"], "accepted")
        self.assertIn(
            "- **Status:** accepted by merge of PR #184; candidate until that merge",
            adr,
        )
        self.assertIn(
            "**Status:** accepted on merge of PR #184; candidate until that merge",
            foundation,
        )
        self.assertIn("ADR_0054 = ACCEPTED_ON_MERGE_OF_PR_184", adr)
        self.assertIn("FOUNDATION_V2 = ACCEPTED_HARDENED_M3_SCOPE_ON_MERGE_OF_PR_184", adr)
        self.assertIn("ADR_0054 = ACCEPTED\nFOUNDATION_V2 = ACCEPTED_HARDENED_M3_SCOPE", adr)
        self.assertIn("M3_AUTHORIZED = NO", adr)
        self.assertIn("AUTHORIZATION_HEAD = NOT_SET", adr)
        self.assertIn(
            "PRIOR_AUTHORIZATION_HEAD =\n0f13b43680ea7d0b043c5baee59eb2ed3c364ecc",
            adr,
        )
        self.assertIn("AUTHORIZED_NEXT_TASK = M3.P0_STATE_IDENTITY_CUT", adr)
        self.assertIn("T0_START_REQUIRES = M3 reauthorized AND P0 complete", adr)
        self.assertIn("M3_AUTHORIZED = NO", foundation)
        self.assertIn("AUTHORIZATION_HEAD = NOT_SET", foundation)
        self.assertIn(
            "PRIOR_AUTHORIZATION_HEAD = 0f13b43680ea7d0b043c5baee59eb2ed3c364ecc",
            foundation,
        )
        self.assertIn("M3_PRE_T0_HARDENING_REVIEW = APPROVE", foundation)
        self.assertNotIn("AUTHORIZED_NEXT_TASK = M3.T0", foundation)

    def test_project_source_state_points_to_the_single_current_entry_point(self) -> None:
        source_state = (ROOT / "project-sources" / "33_CURRENT_PROJECT_STATE.md").read_text(
            encoding="utf-8"
        )

        self.assertIn("V0.2.2 Foundation Snapshot", source_state)
        self.assertIn("historical", source_state.lower())
        self.assertIn("../README.md", source_state)
        self.assertIn("../docs/ROADMAP.md", source_state)
        self.assertNotIn("M2.5", source_state)
        self.assertNotIn("Issue #130", source_state)
        self.assertNotIn("Engine M3", source_state)
        self.assertNotIn("## Current blockers", source_state)
        self.assertNotIn("## Current boundary", source_state)


if __name__ == "__main__":
    unittest.main()
