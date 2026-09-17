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
            "**Current active work area:** M3 T0 closure status sync; "
            "T0 implementation (slices T0-01/02A/02B, PRs #189/#190/#191) is merged",
            readme,
        )
        self.assertIn(
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
        self.assertIn("**M3.T0:** `COMPLETE-CANDIDATE / FREEZE-ELIGIBLE`", readme)
        self.assertIn("`b403edefcabf7b304c0fa5f6816d22ac8aca477b`", readme)
        self.assertIn("**M3.S1:** `SELECTED / NOT_AUTHORIZED`", readme)
        self.assertIn("**M3 semantic implementation:** `NOT_STARTED`", readme)
        self.assertIn("**M3 Pre-T0 hardening:** `COMPLETE / ACCEPTED`", readme)
        self.assertIn("ADR 0054 = ACCEPTED", readme)
        self.assertIn("FOUNDATION_V2 = ACCEPTED_HARDENED_M3_SCOPE", readme)
        self.assertIn("**M3 plan status:** `ACCEPTED`", readme)
        self.assertIn("**Next gate:** `M3_T0_CLOSURE_STATUS_SYNC_EXACT_HEAD_REVIEW`", readme)
        self.assertIn(
            "**M3 hardening acceptance:** PR #184 merged and accepted ADR 0054/Foundation V2",
            readme,
        )
        self.assertIn(
            "T0 was reauthorized under Issue #178 and implemented by merged PRs #189/#190/#191",
            readme,
        )
        self.assertIn("11 Foundation capabilities are `specified` only", readme)
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
        self.assertRegex(readme, r"M3[^\n]*NOT_AUTHORIZED")
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
        self.assertIn("M3_S1 = rules/turn-structure@0.1.0 SELECTED", roadmap)
        self.assertIn(
            "AUTHORIZATION_HEAD = ea668c47ef1361b3d989fd32b8f3cfd4751b1e79",
            roadmap,
        )
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
        self.assertIn("T0_COMPLETE_CANDIDATE = YES", roadmap)
        self.assertIn("T0_FREEZE_ELIGIBLE = YES", roadmap)
        self.assertIn("T0_FREEZE_EXECUTED = NO", roadmap)
        self.assertIn("S1_IMPLEMENTATION = NOT_AUTHORIZED", roadmap)
        self.assertIn("S1_AUTHORIZATION_ELIGIBLE = YES", roadmap)
        self.assertIn("NEXT_GATE = M3_T0_CLOSURE_STATUS_SYNC_EXACT_HEAD_REVIEW", roadmap)
        self.assertNotIn("M3_PRE_T0_HARDENING = CANDIDATE", roadmap)
        self.assertNotIn("M3_ENTRY_DECISION = ACCEPTED_BUT_UNDER_PRE_T0_HARDENING", roadmap)
        self.assertNotIn("HARDENED_PLAN_MERGE_AND_EXACT_MASTER_REAUTHORIZATION", roadmap)
        self.assertIn(
            "M3.P0 semantic-neutral state/persistence identity cut\n"
            "→ M3.T0 thin private conformance facade\n"
            "→ M3.S1 rules/turn-structure@0.1.0",
            roadmap,
        )
        self.assertIn("M3.P0 semantic-neutral state/persistence identity cut", roadmap)
        self.assertIn("PR #184", roadmap)
        self.assertIn(
            "M3 has started only in the sense that the semantic-neutral P0",
            roadmap,
        )
        self.assertIn("P0 is complete and frozen", roadmap)
        self.assertIn("Issue #105", roadmap)
        self.assertNotIn("current active maintainer work area is Issue\n#130", roadmap)
        self.assertIn("Census-driven scope", roadmap)
        self.assertIn("outside this authoritative engine repository", roadmap)

    def test_current_status_does_not_duplicate_a_second_status_file(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        self.assertNotRegex(
            readme,
            r"(?:CURRENT_STATUS|PROJECT_STATE|status\.json)",
        )

    def test_foundation_registry_is_specified_only(self) -> None:
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
        for entry in entries:
            with self.subTest(capability=entry["key"]):
                self.assertEqual(entry["version"], "0.1.0")
                self.assertEqual(entry["lifecycle"], "specified")
                self.assertEqual(entry["implementation_paths"], [])
                self.assertEqual(entry["conformance_cases"], [])
                self.assertEqual(entry["benchmark_scenarios"], [])

        self.assertEqual(sum(entry["lifecycle"] == "implemented" for entry in entries), 0)
        self.assertEqual(sum(entry["lifecycle"] == "covered" for entry in entries), 0)
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

        self.assertIn("accepted base sequence currently runs through ADR 0054", adr_index)
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
