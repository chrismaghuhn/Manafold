#!/usr/bin/env python3
"""Run maintainer checks at development, integration, or certification depth."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]

FAST = [
    [sys.executable, "scripts/generate_contracts.py", "--check"],
    [sys.executable, "scripts/verify_repository.py"],
    [sys.executable, "scripts/check_rust_source_structure.py"],
    [sys.executable, "scripts/check_documentation.py"],
    [sys.executable, "scripts/validate_schemas.py"],
    [sys.executable, "scripts/validate_golden_path.py"],
    [sys.executable, "scripts/run_python_tests.py"],
]
INTEGRATION_EXTRA = [
    ["ruff", "format", "--check", "python", "scripts"],
    ["ruff", "check", "python", "scripts"],
    ["mypy", "--config-file", "python/pyproject.toml"],
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


def run(commands: list[list[str]], *, allow_missing: bool) -> int:
    for command in commands:
        if not command_available(command):
            print(f"MISSING TOOL: {command[0]}")
            if allow_missing:
                continue
            return 2
        print("+", " ".join(command), flush=True)
        result = subprocess.run(command, cwd=ROOT)
        if result.returncode != 0:
            return result.returncode
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
    commands = list(FAST)
    if args.profile in {"integration", "certification"}:
        commands += INTEGRATION_EXTRA
    if args.profile == "certification":
        commands += CERTIFICATION_EXTRA
    return run(commands, allow_missing=args.allow_missing_tools)


if __name__ == "__main__":
    raise SystemExit(main())
