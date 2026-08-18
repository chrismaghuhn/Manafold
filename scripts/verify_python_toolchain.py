#!/usr/bin/env python3
from __future__ import annotations

import platform
import re
import sys
import tomllib
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
PIN_RE = re.compile(r"^[A-Za-z0-9_.-]+==[^=<>!~;\s]+$")


def main() -> None:
    errors: list[str] = []
    expected_python = (ROOT / ".python-version").read_text(encoding="utf-8").strip()
    actual_python = platform.python_version()
    if actual_python != expected_python:
        errors.append(f"Python mismatch: expected {expected_python}, got {actual_python}")

    toolchain = tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))
    rust_channel = toolchain.get("toolchain", {}).get("channel")
    if rust_channel != "1.85.1":
        errors.append(f"unexpected Rust channel: {rust_channel!r}")

    lock_path = ROOT / "python" / "requirements-dev.lock"
    pins: dict[str, str] = {}
    for number, raw in enumerate(lock_path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if PIN_RE.fullmatch(line) is None:
            errors.append(
                f"non-exact direct tool pin at {lock_path.relative_to(ROOT)}:{number}: {line}"
            )
            continue
        name, version = line.split("==", 1)
        normalized = name.lower().replace("_", "-")
        if normalized in pins:
            errors.append(f"duplicate direct tool pin: {name}")
        pins[normalized] = version

    required = {"jsonschema", "mypy", "pytest", "pyyaml", "referencing", "ruff"}
    missing = sorted(required - pins.keys())
    if missing:
        errors.append(f"missing direct tool pins: {missing}")
    if (ROOT / "requirements-dev.lock").exists():
        errors.append(
            "duplicate root requirements-dev.lock is forbidden; Python lock has one owner"
        )

    pyproject = tomllib.loads((ROOT / "python" / "pyproject.toml").read_text(encoding="utf-8"))
    declared = pyproject.get("project", {}).get("requires-python")
    if declared != ">=3.11,<3.14":
        errors.append(f"unexpected Python compatibility range: {declared!r}")
    mypy_target = pyproject.get("tool", {}).get("mypy", {}).get("python_version")
    if mypy_target != "3.11":
        errors.append(f"Mypy must target the minimum supported Python version, got {mypy_target!r}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        raise SystemExit(1)
    print(
        f"PASS: reference Python {actual_python}, Rust channel {rust_channel}, "
        f"and {len(pins)} exact direct development-tool pins"
    )


if __name__ == "__main__":
    main()
