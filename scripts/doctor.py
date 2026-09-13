#!/usr/bin/env python3
"""Report and, in strict mode, prove the selected maintainer environment."""

from __future__ import annotations

import argparse
import importlib.metadata as metadata
import importlib.util
import os
import platform
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
DOCTOR_PROBE_TIMEOUT_SECONDS = 30
PYTHON_MODULES = ("pytest", "jsonschema", "referencing", "yaml")
PYTHON_TOOLS = {
    "ruff": "ruff",
    "mypy": "mypy",
    "pytest": "pytest",
}
NATIVE_TOOLS = ("rustup", "cargo", "rustc", "rustfmt", "clippy-driver")
DECISION_ROW = re.compile(r"\|\s*(OD-\d{3})\s*\|\s*([^|]+)\|")


def command_text(command: list[str]) -> str:
    return subprocess.list2cmdline(command)


def project_python_path() -> Path:
    relative = Path(".venv/Scripts/python.exe" if sys.platform == "win32" else ".venv/bin/python")
    return ROOT / relative


def normalized_path(path: str | Path) -> str:
    return os.path.normcase(os.path.abspath(os.fspath(path)))


def probe(command: list[str]) -> str | None:
    identity = command_text(command)
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=DOCTOR_PROBE_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired:
        print(
            f"TIMEOUT {identity} (limit={DOCTOR_PROBE_TIMEOUT_SECONDS}s)",
            file=sys.stderr,
        )
        print(f"RERUN: {identity}", file=sys.stderr)
        return None
    except OSError as error:
        print(f"FAIL {identity}: {error}", file=sys.stderr)
        print(f"RERUN: {identity}", file=sys.stderr)
        return None

    if result.returncode != 0:
        return None
    return result.stdout.strip() or result.stderr.strip()


def version(command: list[str]) -> str | None:
    return probe(command)


def locked_tool_versions() -> dict[str, str]:
    versions: dict[str, str] = {}
    lock = ROOT / "python/requirements-dev.lock"
    for raw_line in lock.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "==" not in line:
            continue
        name, value = line.split("==", 1)
        versions[name.strip().lower().replace("_", "-")] = value.strip()
    return versions


def normalized_distribution(name: str) -> str:
    return name.lower().replace("_", "-")


def add_problem(problems: list[str], problem: str) -> None:
    if problem not in problems:
        problems.append(problem)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args()

    problems: list[str] = []
    print(f"repository:       {ROOT}")
    print(f"platform:         {platform.platform()}")
    print(f"sys.platform:     {sys.platform}")
    print(
        "shell:            "
        f"COMSPEC={os.environ.get('COMSPEC', 'unset')} "
        f"SHELL={os.environ.get('SHELL', 'unset')}"
    )

    try:
        required_python = (ROOT / ".python-version").read_text(encoding="utf-8").strip()
    except OSError as error:
        required_python = "unavailable"
        add_problem(problems, f"python-reference:{error}")
    actual_python = platform.python_version()
    expected_project_python = project_python_path()
    project_environment = normalized_path(sys.executable) == normalized_path(
        expected_project_python
    )

    print(f"python:           {actual_python} ({sys.executable})")
    print(f"required Python:  {required_python}")
    print(f"project .venv: {'YES' if project_environment else 'NO'} ({expected_project_python})")
    if actual_python != required_python:
        add_problem(problems, f"python:{required_python}")
    if not project_environment:
        add_problem(problems, f"project-python:{expected_project_python}")

    try:
        with (ROOT / "rust-toolchain.toml").open("rb") as handle:
            toolchain = tomllib.load(handle)["toolchain"]
        required_rust = str(toolchain["channel"])
        components = tuple(str(component) for component in toolchain.get("components", ()))
    except (OSError, KeyError, TypeError, tomllib.TOMLDecodeError) as error:
        required_rust = "unavailable"
        components = ()
        add_problem(problems, f"rust-toolchain:{error}")
    print(f"required Rust:    {required_rust}")
    print(f"Rust components:  {', '.join(components) or 'none declared'}")

    print("\nPython modules:")
    for module in PYTHON_MODULES:
        present = importlib.util.find_spec(module) is not None
        print(f"  {module:14} {'OK' if present else 'MISSING'}")
        if not present:
            add_problem(problems, f"python:{module}")

    try:
        locked_versions = locked_tool_versions()
    except OSError as error:
        locked_versions = {}
        add_problem(problems, f"python-lock:{error}")

    print("\nselected Python tools:")
    for tool, distribution in PYTHON_TOOLS.items():
        lock_key = normalized_distribution(distribution)
        expected_version = locked_versions.get(lock_key)
        try:
            installed_version = metadata.version(distribution)
        except metadata.PackageNotFoundError:
            installed_version = None
        runnable = probe([sys.executable, "-m", tool, "--version"])
        version_ok = expected_version is not None and installed_version == expected_version
        runnable_ok = runnable is not None and (
            expected_version is None or expected_version in runnable
        )
        status = installed_version or "MISSING"
        print(
            f"  {tool:14} {status} "
            f"({'OK' if version_ok and runnable_ok else 'MISSING'}) via {sys.executable} -m"
        )
        if not version_ok:
            add_problem(problems, f"python-tool-version:{tool}")
        if not runnable_ok:
            add_problem(problems, f"python-tool-runtime:{tool}")

    print("\ncommand-line tools:")
    for tool in NATIVE_TOOLS:
        location = shutil.which(tool)
        print(f"  {tool:14} {location or 'MISSING'}")
        if location is None:
            add_problem(problems, tool)

    for tool in ("bash", "just"):
        location = shutil.which(tool)
        required = sys.platform != "win32"
        label = "required" if required else "optional"
        print(f"  {tool:14} {location or 'MISSING'} ({label})")
        if required and location is None:
            add_problem(problems, tool)

    active_toolchain = version(["rustup", "show", "active-toolchain"])
    rustc_version = version(["rustc", "--version"])
    cargo_version = version(["cargo", "--version"])
    rustfmt_version = version(["rustfmt", "--version"])
    clippy_version = version(["clippy-driver", "--version"])
    print(f"\nrustup:           {active_toolchain or 'MISSING'}")
    print(f"rustc:            {rustc_version or 'MISSING'}")
    print(f"cargo:            {cargo_version or 'MISSING'}")
    print(f"rustfmt:          {rustfmt_version or 'MISSING'}")
    print(f"clippy-driver:    {clippy_version or 'MISSING'}")
    if required_rust != "unavailable":
        if active_toolchain is None or not active_toolchain.startswith(required_rust):
            add_problem(problems, f"rustup:{required_rust}")
        if rustc_version is None or not rustc_version.startswith(f"rustc {required_rust}"):
            add_problem(problems, f"rust:{required_rust}")
    for tool, tool_version in (
        ("rustup", active_toolchain),
        ("rustc", rustc_version),
        ("cargo", cargo_version),
        ("rustfmt", rustfmt_version),
        ("clippy-driver", clippy_version),
    ):
        if tool_version is None:
            add_problem(problems, tool)

    cargo_lock = ROOT / "Cargo.lock"
    print(f"Cargo.lock:       {'OK' if cargo_lock.is_file() else 'MISSING'}")
    if not cargo_lock.is_file():
        add_problem(problems, "Cargo.lock")

    try:
        register = (ROOT / "docs" / "OPEN_DECISIONS.md").read_text(encoding="utf-8")
        rows = DECISION_ROW.findall(register)
        open_ids = [identifier for identifier, status in rows if status.strip() == "open"]
        partial_ids = [identifier for identifier, status in rows if status.strip() == "partial"]
        print(f"\nopen decisions:    {len(open_ids)}")
        print(f"partial decisions: {len(partial_ids)}")
    except OSError as error:
        add_problem(problems, f"open-decisions:{error}")
        print(f"\nopen decisions:    unavailable ({error})")

    print("\nrecommended profiles:")
    print("  local iteration: <project-python> scripts/run_checks.py fast")
    print("  review-ready:    <project-python> scripts/run_checks.py integration")
    print("  release smoke:   <project-python> scripts/run_checks.py certification")

    if problems:
        print("\nproblems:")
        for problem in problems:
            print(f"  - {problem}")
        print("\nMissing or mismatched prerequisites mean the associated gates were not run.")
        if args.strict:
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
