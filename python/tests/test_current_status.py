from __future__ import annotations

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class CurrentStatusEntryPointTests(unittest.TestCase):
    def test_root_readme_exposes_current_milestone_boundaries(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")

        self.assertRegex(
            readme,
            r"\*\*Current foundation milestone:\*\* M2 .*COMPLETE",
        )
        self.assertIn(
            "**Current active work area:** maintainer hardening under Issue #130",
            readme,
        )
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
        self.assertIn("`M3 = NOT_AUTHORIZED`", roadmap)
        self.assertIn("Census-driven scope", roadmap)
        self.assertIn("outside this authoritative engine repository", roadmap)

    def test_current_status_does_not_duplicate_a_second_status_file(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        self.assertNotRegex(
            readme,
            r"(?:CURRENT_STATUS|PROJECT_STATE|status\.json)",
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
