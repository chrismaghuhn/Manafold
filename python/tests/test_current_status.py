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
            "**Current active work area:** M3 Pre-T0 plan hardening under Issue #178; "
            "implementation remains blocked pending a merged hardened plan and exact-master reauthorization",
            readme,
        )
        self.assertIn("**Core modularization:** Issue #162 `COMPLETE`", readme)
        self.assertIn("**M3 semantic implementation:** `NOT_STARTED`", readme)
        self.assertIn("**M3 authorization:** `NOT_AUTHORIZED`", readme)
        self.assertIn("**M3 pre-T0 hardening:** `IN_PROGRESS`", readme)
        self.assertIn("11 Foundation capabilities are `specified` only", readme)
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
        self.assertIn("M3_STARTED = NO", roadmap)
        self.assertIn("M3_AUTHORIZED = NO", roadmap)
        self.assertIn("M3_PRE_T0_HARDENING = IN_PROGRESS", roadmap)
        self.assertIn("M3_ENTRY_DECISION = ACCEPTED_BUT_UNDER_PRE_T0_HARDENING", roadmap)
        self.assertIn("NEXT_GATE = HARDENED_PLAN_MERGE_AND_EXACT_MASTER_REAUTHORIZATION", roadmap)
        self.assertIn("M3.P0 semantic-neutral state/persistence identity cut", roadmap)
        self.assertIn("`M3 = NOT_STARTED / NOT_AUTHORIZED`", roadmap)
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
            (ROOT / "cards" / "capabilities" / "registry.json").read_text(
                encoding="utf-8"
            )
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

        self.assertEqual(
            sum(entry["lifecycle"] == "implemented" for entry in entries), 0
        )
        self.assertEqual(
            sum(entry["lifecycle"] == "covered" for entry in entries), 0
        )
        self.assertEqual(
            sum(entry["lifecycle"] == "certified" for entry in entries), 0
        )

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
