#!/usr/bin/env python3
"""Run the explicit, read-only Rust and Python dependency audits."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MAX_SINGLE_AUDIT_SUBPROCESS_RUNTIME_SECONDS = 600
AUDIT_PASS = "PASS"
AUDIT_FAIL = "FAIL"
AUDIT_BLOCKED = "BLOCKED"


@dataclass(frozen=True)
class CommandResult:
    returncode: int | None
    stdout: str = ""
    stderr: str = ""
    blocked: bool = False
    timed_out: bool = False

    @property
    def output(self) -> str:
        return self.stdout.strip() or self.stderr.strip()


@dataclass(frozen=True)
class EcosystemResult:
    ecosystem: str
    status: str
    tool_version: str | None = None
    detail: str = ""


def command_text(command: list[str]) -> str:
    return subprocess.list2cmdline(command)


def executable_available(executable: str) -> bool:
    return Path(executable).is_file() or shutil.which(executable) is not None


def run_bounded(
    command: list[str],
    *,
    timeout: int = MAX_SINGLE_AUDIT_SUBPROCESS_RUNTIME_SECONDS,
) -> CommandResult:
    identity = command_text(command)
    if not command or not executable_available(command[0]):
        print(f"BLOCKED: audit tool unavailable: {identity}", file=sys.stderr)
        print(f"RERUN: {identity}", file=sys.stderr)
        return CommandResult(returncode=None, blocked=True)

    try:
        completed = subprocess.run(
            command,
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        print(f"TIMEOUT {identity} (limit={timeout}s)", file=sys.stderr)
        print(f"RERUN: {identity}", file=sys.stderr)
        return CommandResult(returncode=None, blocked=True, timed_out=True)
    except OSError as error:
        print(f"BLOCKED: cannot execute {identity}: {error}", file=sys.stderr)
        print(f"RERUN: {identity}", file=sys.stderr)
        return CommandResult(returncode=None, blocked=True)

    return CommandResult(
        returncode=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )


def payload_contains_vulnerabilities(ecosystem: str, output: str) -> bool:
    try:
        payload = json.loads(output)
    except (json.JSONDecodeError, TypeError):
        return False

    if ecosystem == "python" and isinstance(payload, dict):
        dependencies = payload.get("dependencies")
        return isinstance(dependencies, list) and any(
            isinstance(dependency, dict) and bool(dependency.get("vulns"))
            for dependency in dependencies
        )

    if ecosystem == "rust" and isinstance(payload, dict):
        vulnerabilities = payload.get("vulnerabilities")
        if not isinstance(vulnerabilities, dict):
            return False
        listed = vulnerabilities.get("list")
        found = vulnerabilities.get("found")
        return (isinstance(listed, list) and bool(listed)) or (isinstance(found, int) and found > 0)

    return False


def classify_audit_result(result: CommandResult, ecosystem: str) -> str:
    if result.blocked or result.returncode is None:
        return AUDIT_BLOCKED
    if result.returncode == 0:
        return AUDIT_PASS
    if result.returncode == 1 and payload_contains_vulnerabilities(ecosystem, result.output):
        return AUDIT_FAIL
    return AUDIT_BLOCKED


def aggregate_statuses(statuses: list[str]) -> str:
    if not statuses:
        return AUDIT_BLOCKED
    if AUDIT_FAIL in statuses:
        return AUDIT_FAIL
    if AUDIT_BLOCKED in statuses:
        return AUDIT_BLOCKED
    if all(status == AUDIT_PASS for status in statuses):
        return AUDIT_PASS
    return AUDIT_BLOCKED


def audit_ecosystem(
    ecosystem: str,
    *,
    version_command: list[str],
    audit_command: list[str],
) -> EcosystemResult:
    version_result = run_bounded(version_command)
    if version_result.blocked or version_result.returncode != 0:
        return EcosystemResult(
            ecosystem=ecosystem,
            status=AUDIT_BLOCKED,
            detail="audit tool version probe was unavailable",
        )

    tool_version = version_result.output.splitlines()[0] if version_result.output else "unknown"
    audit_result = run_bounded(audit_command)
    status = classify_audit_result(audit_result, ecosystem)
    detail = audit_result.output
    return EcosystemResult(
        ecosystem=ecosystem,
        status=status,
        tool_version=tool_version,
        detail=detail,
    )


def run_audits(
    *,
    rust_audit_executable: str = "cargo-audit",
    python_audit_python: Path | None = None,
) -> tuple[list[EcosystemResult], str]:
    python_executable = str(python_audit_python or Path(sys.executable))
    requirements = str(ROOT / "python/requirements-dev.lock")
    rust_result = audit_ecosystem(
        "rust",
        version_command=[rust_audit_executable, "--version"],
        audit_command=[rust_audit_executable, "audit", "--json"],
    )
    python_result = audit_ecosystem(
        "python",
        version_command=[python_executable, "-m", "pip_audit", "--version"],
        audit_command=[
            python_executable,
            "-m",
            "pip_audit",
            "--requirement",
            requirements,
            "--format",
            "json",
            "--progress-spinner",
            "off",
        ],
    )
    results = [rust_result, python_result]
    return results, aggregate_statuses([result.status for result in results])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-audit-executable", default="cargo-audit")
    parser.add_argument("--python-audit-python", type=Path, default=Path(sys.executable))
    args = parser.parse_args()

    results, overall = run_audits(
        rust_audit_executable=args.rust_audit_executable,
        python_audit_python=args.python_audit_python,
    )
    for result in results:
        print(f"{result.ecosystem.upper()}_AUDIT_TOOL_VERSION = {result.tool_version or 'unknown'}")
        print(f"{result.ecosystem.upper()}_AUDIT = {result.status}")
        if result.detail and result.status != AUDIT_PASS:
            print(f"{result.ecosystem.upper()}_AUDIT_DETAIL = {result.detail}")
    print(f"AUDIT_OVERALL = {overall}")
    return {AUDIT_PASS: 0, AUDIT_FAIL: 1, AUDIT_BLOCKED: 2}[overall]


if __name__ == "__main__":
    raise SystemExit(main())
