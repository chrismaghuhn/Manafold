from __future__ import annotations

import re
import subprocess
import sys
import unittest
from pathlib import Path
from unittest import mock

import yaml

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import run_dependency_audit as dependency_audit
import verify_repository


class ActionPinTests(unittest.TestCase):
    def test_full_lowercase_sha_is_accepted(self) -> None:
        self.assertIsNone(verify_repository.action_pin_error("actions/checkout@" + "a" * 40))

    def test_mutable_tags_and_branches_are_rejected(self) -> None:
        for reference in ("v7", "v2", "main", "master", "HEAD"):
            with self.subTest(reference=reference):
                self.assertIsNotNone(
                    verify_repository.action_pin_error(f"actions/checkout@{reference}")
                )

    def test_short_and_noncanonical_shas_are_rejected(self) -> None:
        for sha in ("a" * 39, "A" * 40, "a" * 39 + "G"):
            with self.subTest(sha=sha):
                self.assertIsNotNone(verify_repository.action_pin_error(f"actions/checkout@{sha}"))

    def test_local_action_is_allowed(self) -> None:
        self.assertIsNone(verify_repository.action_pin_error("./.github/actions/local"))

    def test_current_workflow_action_inventory_is_immutable(self) -> None:
        references = verify_repository.workflow_action_references()
        self.assertEqual(len(references), 18)
        self.assertEqual(verify_repository.workflow_action_pin_errors(), [])

    def test_dependabot_github_actions_ecosystem_is_preserved(self) -> None:
        dependabot = (ROOT / ".github/dependabot.yml").read_text(encoding="utf-8")
        self.assertIn("package-ecosystem: github-actions", dependabot)


class DependencyAuditTests(unittest.TestCase):
    @staticmethod
    def result(
        returncode: int | None,
        *,
        blocked: bool = False,
        timed_out: bool = False,
        stdout: str = "",
        stderr: str = "",
    ) -> dependency_audit.CommandResult:
        return dependency_audit.CommandResult(
            returncode=returncode,
            blocked=blocked,
            timed_out=timed_out,
            stdout=stdout,
            stderr=stderr,
        )

    def test_rust_clean_audit_is_pass(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="cargo-audit 0.22.2"),
                self.result(
                    0,
                    stdout='{"vulnerabilities": {"found": false, "count": 0, "list": []}}',
                ),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "rust",
                version_command=["cargo-audit", "--version"],
                audit_command=["cargo-audit", "--json"],
                expected_tool_version="cargo-audit 0.22.2",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_PASS)

    def test_rust_vulnerability_is_fail(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="cargo-audit 0.22.2"),
                self.result(
                    1,
                    stdout=(
                        '{"vulnerabilities": {"found": true, "count": 1, '
                        '"list": [{"id": "RUSTSEC-0000-0000"}]}}'
                    ),
                ),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "rust",
                version_command=["cargo-audit", "--version"],
                audit_command=["cargo-audit", "--json"],
                expected_tool_version="cargo-audit 0.22.2",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_FAIL)

    def test_rust_empty_audit_output_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="cargo-audit 0.22.2"),
                self.result(0, stdout=""),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "rust",
                version_command=["cargo-audit", "--version"],
                audit_command=["cargo-audit", "audit", "--json"],
                expected_tool_version="cargo-audit 0.22.2",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_rust_malformed_audit_output_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="cargo-audit 0.22.2"),
                self.result(0, stdout="garbage"),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "rust",
                version_command=["cargo-audit", "--version"],
                audit_command=["cargo-audit", "audit", "--json"],
                expected_tool_version="cargo-audit 0.22.2",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_rust_unexpected_audit_shape_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="cargo-audit 0.22.2"),
                self.result(0, stdout='{"unexpected": "schema"}'),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "rust",
                version_command=["cargo-audit", "--version"],
                audit_command=["cargo-audit", "audit", "--json"],
                expected_tool_version="cargo-audit 0.22.2",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_rust_version_mismatch_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="cargo-audit 0.19.0"),
                self.result(
                    0,
                    stdout='{"vulnerabilities": {"found": false, "count": 0, "list": []}}',
                ),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "rust",
                version_command=["cargo-audit", "--version"],
                audit_command=["cargo-audit", "audit", "--json"],
                expected_tool_version="cargo-audit 0.22.2",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)
        self.assertIn("expected cargo-audit 0.22.2", result.detail)

    def test_rust_advisory_database_error_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="cargo-audit 0.22.2"),
                self.result(1, stderr="error loading advisory database: unsupported CVSS version"),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "rust",
                version_command=["cargo-audit", "--version"],
                audit_command=["cargo-audit", "audit", "--json"],
                expected_tool_version="cargo-audit 0.22.2",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_rust_missing_tool_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            return_value=self.result(None, blocked=True),
        ):
            result = dependency_audit.audit_ecosystem(
                "rust",
                version_command=["cargo-audit", "--version"],
                audit_command=["cargo-audit", "--json"],
                expected_tool_version="cargo-audit 0.22.2",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_rust_timeout_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            return_value=self.result(None, blocked=True, timed_out=True),
        ):
            result = dependency_audit.audit_ecosystem(
                "rust",
                version_command=["cargo-audit", "--version"],
                audit_command=["cargo-audit", "--json"],
                expected_tool_version="cargo-audit 0.22.2",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_rust_audit_executable_receives_the_audit_subcommand(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="cargo-audit 0.22.2"),
                self.result(
                    0,
                    stdout='{"vulnerabilities": {"found": false, "count": 0, "list": []}}',
                ),
                self.result(0, stdout="pip-audit 2.10.1"),
                self.result(0, stdout='{"dependencies": [], "fixes": []}'),
            ],
        ) as run_bounded:
            dependency_audit.run_audits(
                rust_audit_executable="cargo-audit",
                python_audit_python=Path("audit-python"),
            )

        self.assertEqual(
            run_bounded.call_args_list[1].args[0],
            ["cargo-audit", "audit", "--json"],
        )

    def test_python_clean_audit_is_pass(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="pip-audit 2.10.1"),
                self.result(0, stdout='{"dependencies": [], "fixes": []}'),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "python",
                version_command=["python", "-m", "pip_audit", "--version"],
                audit_command=["python", "-m", "pip_audit", "--format", "json"],
                expected_tool_version="pip-audit 2.10.1",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_PASS)

    def test_python_vulnerability_is_fail(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="pip-audit 2.10.1"),
                self.result(
                    1,
                    stdout=(
                        '{"dependencies": [{"name": "pytest", "vulns": '
                        '[{"id": "PYSEC-0000-0000"}], "version": "8.4.1"}], '
                        '"fixes": []}'
                    ),
                ),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "python",
                version_command=["python", "-m", "pip_audit", "--version"],
                audit_command=["python", "-m", "pip_audit", "--format", "json"],
                expected_tool_version="pip-audit 2.10.1",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_FAIL)

    def test_python_empty_audit_output_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="pip-audit 2.10.1"),
                self.result(0, stdout=""),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "python",
                version_command=["python", "-m", "pip_audit", "--version"],
                audit_command=["python", "-m", "pip_audit", "--format", "json"],
                expected_tool_version="pip-audit 2.10.1",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_python_malformed_audit_output_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="pip-audit 2.10.1"),
                self.result(0, stdout="garbage"),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "python",
                version_command=["python", "-m", "pip_audit", "--version"],
                audit_command=["python", "-m", "pip_audit", "--format", "json"],
                expected_tool_version="pip-audit 2.10.1",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_python_unexpected_audit_shape_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="pip-audit 2.10.1"),
                self.result(0, stdout='{"unexpected": "schema"}'),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "python",
                version_command=["python", "-m", "pip_audit", "--version"],
                audit_command=["python", "-m", "pip_audit", "--format", "json"],
                expected_tool_version="pip-audit 2.10.1",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_python_version_mismatch_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            side_effect=[
                self.result(0, stdout="pip-audit 2.8.0"),
                self.result(0, stdout='{"dependencies": [], "fixes": []}'),
            ],
        ):
            result = dependency_audit.audit_ecosystem(
                "python",
                version_command=["python", "-m", "pip_audit", "--version"],
                audit_command=["python", "-m", "pip_audit", "--format", "json"],
                expected_tool_version="pip-audit 2.10.1",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)
        self.assertIn("expected pip-audit 2.10.1", result.detail)

    def test_python_missing_tool_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            return_value=self.result(None, blocked=True),
        ):
            result = dependency_audit.audit_ecosystem(
                "python",
                version_command=["python", "-m", "pip_audit", "--version"],
                audit_command=["python", "-m", "pip_audit", "--format", "json"],
                expected_tool_version="pip-audit 2.10.1",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_python_timeout_is_blocked(self) -> None:
        with mock.patch.object(
            dependency_audit,
            "run_bounded",
            return_value=self.result(None, blocked=True, timed_out=True),
        ):
            result = dependency_audit.audit_ecosystem(
                "python",
                version_command=["python", "-m", "pip_audit", "--version"],
                audit_command=["python", "-m", "pip_audit", "--format", "json"],
                expected_tool_version="pip-audit 2.10.1",
            )
        self.assertEqual(result.status, dependency_audit.AUDIT_BLOCKED)

    def test_fail_dominates_blocked_and_blocked_is_not_pass(self) -> None:
        self.assertEqual(
            dependency_audit.aggregate_statuses(
                [dependency_audit.AUDIT_PASS, dependency_audit.AUDIT_BLOCKED]
            ),
            dependency_audit.AUDIT_BLOCKED,
        )
        self.assertEqual(
            dependency_audit.aggregate_statuses(
                [dependency_audit.AUDIT_FAIL, dependency_audit.AUDIT_BLOCKED]
            ),
            dependency_audit.AUDIT_FAIL,
        )

    def test_audit_subprocess_budget_is_bounded(self) -> None:
        self.assertLessEqual(
            dependency_audit.MAX_SINGLE_AUDIT_SUBPROCESS_RUNTIME_SECONDS,
            600,
        )
        with (
            mock.patch.object(dependency_audit.shutil, "which", return_value="tool"),
            mock.patch.object(
                dependency_audit.subprocess,
                "run",
                side_effect=subprocess.TimeoutExpired(["tool"], 600),
            ),
        ):
            result = dependency_audit.run_bounded(["tool"])
        self.assertTrue(result.blocked)
        self.assertTrue(result.timed_out)


class ReleaseReproducibilityTests(unittest.TestCase):
    def test_reproducibility_levels_and_open_decision_boundaries_are_explicit(self) -> None:
        policy = (ROOT / "docs/maintenance/TOOLCHAIN_POLICY.md").read_text(encoding="utf-8")
        decisions = (ROOT / "docs/OPEN_DECISIONS.md").read_text(encoding="utf-8")
        lowered = policy.lower()
        for level in ("level 1", "level 2", "level 3"):
            self.assertIn(level, lowered)
        self.assertIn("exact direct development-tool pins", lowered)
        self.assertIn("od-016", lowered)
        self.assertIn("od-021", lowered)
        od_016 = re.search(r"\| OD-016 \| ([^|]+)\|", decisions)
        od_021 = re.search(r"\| OD-021 \| ([^|]+)\|", decisions)
        self.assertIsNotNone(od_016)
        self.assertIsNotNone(od_021)
        self.assertEqual(od_016.group(1).strip().lower(), "partial")
        self.assertEqual(od_021.group(1).strip().lower(), "open")
        self.assertNotIn("od-016 = resolved", lowered)
        self.assertNotIn("attestation complete", lowered)

    def test_dependency_audit_workflow_is_separate_from_the_pr_gate(self) -> None:
        workflow_path = ROOT / ".github/workflows/dependency-audit.yml"
        self.assertTrue(workflow_path.is_file())
        source = workflow_path.read_text(encoding="utf-8")
        workflow = yaml.safe_load(source)
        self.assertIn("workflow_dispatch", source)
        self.assertNotIn("manafold-pr-gate", source)
        self.assertEqual(workflow["jobs"]["dependency-audit"]["runs-on"], "ubuntu-latest")
        self.assertIn("rustup toolchain install 1.85.1", source)
        self.assertIn("rustup toolchain install 1.88.0", source)
        self.assertIn("cargo +1.88.0 install cargo-audit", source)
        self.assertIn("--version 0.22.2", source)
        self.assertIn("pip-audit==2.10.1", source)
        for step in workflow["jobs"]["dependency-audit"]["steps"]:
            if "run" in step and any(
                token in step["run"]
                for token in ("bootstrap.py", "pip-audit", "cargo-audit", "run_dependency_audit.py")
            ):
                self.assertEqual(step.get("timeout-minutes"), 10)


if __name__ == "__main__":
    unittest.main()
