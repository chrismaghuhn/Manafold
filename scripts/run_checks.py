#!/usr/bin/env python3
"""Run maintainer checks at development, integration, or certification depth."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
MAX_SINGLE_GATE_SUBPROCESS_RUNTIME_SECONDS = 600


def required_python_version() -> str:
    return (ROOT / ".python-version").read_text(encoding="utf-8").strip()


def current_python_version() -> str:
    return ".".join(map(str, sys.version_info[:3]))


def reference_python_matches() -> bool:
    try:
        return current_python_version() == required_python_version()
    except OSError:
        return False


FAST = [
    [sys.executable, "scripts/generate_contracts.py", "--check"],
    [sys.executable, "scripts/verify_repository.py"],
    [sys.executable, "scripts/check_rust_source_structure.py"],
    [sys.executable, "scripts/check_documentation.py"],
    [sys.executable, "scripts/validate_schemas.py"],
    [sys.executable, "scripts/validate_golden_path.py"],
    [sys.executable, "scripts/run_python_tests.py", "--profile", "smoke"],
]
INTEGRATION_EXTRA = [
    [sys.executable, "scripts/run_python_tests.py", "--profile", "full"],
    [sys.executable, "-m", "ruff", "format", "--check", "python", "scripts"],
    [sys.executable, "-m", "ruff", "check", "python", "scripts"],
    [sys.executable, "-m", "mypy", "--config-file", "python/pyproject.toml"],
    ["cargo", "fmt", "--all", "--", "--check"],
    ["cargo", "check", "--workspace", "--all-targets", "--all-features", "--locked"],
    [
        "cargo",
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ],
    ["cargo", "test", "--workspace", "--all-features", "--locked"],
    [sys.executable, "scripts/validate_maintainer_artifacts.py"],
]
CERTIFICATION_EXTRA = [
    [sys.executable, "scripts/verify_python_toolchain.py"],
    [sys.executable, "scripts/verify_archive_reproducibility.py"],
]


def command_available(command: list[str]) -> bool:
    return command[0] == sys.executable or shutil.which(command[0]) is not None


def command_text(command: list[str]) -> str:
    return subprocess.list2cmdline(command)


def duration_text(seconds: float) -> str:
    return f"{seconds:.3f}s"


def run(commands: list[list[str]], *, allow_missing: bool) -> int:
    for command in commands:
        text = command_text(command)
        if not command_available(command):
            print(f"MISSING TOOL: {command[0]}")
            print(f"RERUN: {text}")
            if allow_missing:
                continue
            return 2

        print(f"RUN {text}", flush=True)
        started = time.perf_counter()
        try:
            result = subprocess.run(
                command,
                cwd=ROOT,
                timeout=MAX_SINGLE_GATE_SUBPROCESS_RUNTIME_SECONDS,
            )
        except subprocess.TimeoutExpired:
            duration = duration_text(time.perf_counter() - started)
            print(
                f"TIMEOUT {duration} {text} (limit={MAX_SINGLE_GATE_SUBPROCESS_RUNTIME_SECONDS}s)"
            )
            print(f"RERUN: {text}")
            return 124
        duration = duration_text(time.perf_counter() - started)
        if result.returncode != 0:
            print(f"FAIL {duration} {text}")
            print(f"RERUN: {text}")
            return result.returncode
        print(f"PASS {duration} {text}", flush=True)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("profile", choices=["fast", "integration", "certification"])
    parser.add_argument(
        "--allow-missing-tools",
        action="store_true",
        help="development-only convenience; never valid freeze evidence",
    )
    args = parser.parse_args()
    if not reference_python_matches():
        try:
            required = required_python_version()
        except OSError:
            required = "the repository-pinned version"
        actual = current_python_version()
        print(f"FAIL: Python {required} required; running {actual} ({sys.executable})")
        print(f"RERUN: <project-python> scripts/run_checks.py {args.profile}")
        return 2
    if args.allow_missing_tools and args.profile != "fast":
        parser.error("--allow-missing-tools is only valid for the fast profile")
    commands = list(FAST)
    if args.profile in {"integration", "certification"}:
        commands += INTEGRATION_EXTRA
    if args.profile == "certification":
        commands += CERTIFICATION_EXTRA
    return run(commands, allow_missing=args.allow_missing_tools)


if __name__ == "__main__":
    raise SystemExit(main())
