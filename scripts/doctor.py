#!/usr/bin/env python3
"""Report exact local prerequisites and unresolved decisions."""

from __future__ import annotations

import argparse
import importlib.util
import platform
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
TOOLS = ("cargo", "rustc", "rustfmt", "clippy-driver", "just", "ruff", "mypy")
MODULES = ("pytest", "jsonschema", "referencing", "yaml")
DECISION_ROW = re.compile(r"\|\s*(OD-\d{3})\s*\|\s*([^|]+)\|")


def version(command: list[str]) -> str | None:
    try:
        result = subprocess.run(command, check=True, capture_output=True, text=True)
    except (OSError, subprocess.CalledProcessError):
        return None
    return result.stdout.strip() or result.stderr.strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args()

    print(f"repository: {ROOT}")
    print(f"platform:   {platform.platform()}")
    actual_python = platform.python_version()
    required_python = (ROOT / ".python-version").read_text().strip()
    with (ROOT / "rust-toolchain.toml").open("rb") as handle:
        required_rust = str(tomllib.load(handle)["toolchain"]["channel"])
    print(f"python:     {actual_python} ({sys.executable})")
    print(f"required:   Python {required_python}")
    print(f"required:   Rust {required_rust}")

    missing: list[str] = []
    if actual_python != required_python:
        missing.append(f"python:{required_python}")
    print("\ncommand-line tools:")
    for tool in TOOLS:
        location = shutil.which(tool)
        print(f"  {tool:14} {location or 'MISSING'}")
        if location is None:
            missing.append(tool)

    print("\nPython modules:")
    for module in MODULES:
        present = importlib.util.find_spec(module) is not None
        print(f"  {module:14} {'OK' if present else 'MISSING'}")
        if not present:
            missing.append(f"python:{module}")

    rust = version(["rustc", "--version"])
    if rust:
        print(f"\nrustc:      {rust}")
        if not rust.startswith(f"rustc {required_rust}"):
            missing.append(f"rust:{required_rust}")

    cargo_lock = ROOT / "Cargo.lock"
    print(f"Cargo.lock: {'OK' if cargo_lock.is_file() else 'MISSING'}")
    if not cargo_lock.is_file():
        missing.append("Cargo.lock")

    register = (ROOT / "docs" / "OPEN_DECISIONS.md").read_text(encoding="utf-8")
    rows = DECISION_ROW.findall(register)
    open_ids = [identifier for identifier, status in rows if status.strip() == "open"]
    partial_ids = [identifier for identifier, status in rows if status.strip() == "partial"]
    print(f"\nopen decisions:    {len(open_ids)}")
    print(f"partial decisions: {len(partial_ids)}")

    print("\nrecommended profiles:")
    print("  local iteration: just check-fast")
    print("  review-ready:    just check")
    print("  release smoke:   just check-all")
    if missing:
        print("\nMissing tools mean the associated gates were not run.")
        if args.strict:
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
