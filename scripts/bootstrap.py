#!/usr/bin/env python3
"""Prepare the project .venv without mutating semantic or lock contracts."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import venv
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MAX_BOOTSTRAP_SUBPROCESS_RUNTIME_SECONDS = 600
PROTECTED_FILES = (
    Path(".python-version"),
    Path("rust-toolchain.toml"),
    Path("Cargo.lock"),
    Path("python/requirements-dev.lock"),
)
PYTHON_VERSION_RE = re.compile(r"Python\s+(\d+\.\d+\.\d+)")


def command_text(command: list[str]) -> str:
    return subprocess.list2cmdline(command)


def current_python_version() -> str:
    return ".".join(map(str, sys.version_info[:3]))


def venv_python_path(venv_path: Path) -> Path:
    relative = Path("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
    return venv_path / relative


def run_command(
    command: list[str],
    *,
    timeout: int = MAX_BOOTSTRAP_SUBPROCESS_RUNTIME_SECONDS,
) -> int:
    identity = command_text(command)
    print(f"+ {identity}", flush=True)
    try:
        completed = subprocess.run(
            command,
            cwd=ROOT,
            check=False,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        print(f"TIMEOUT {identity} (limit={timeout}s)", file=sys.stderr)
        print(f"RERUN: {identity}", file=sys.stderr)
        return 124
    except OSError as error:
        print(f"FAIL {identity}: {error}", file=sys.stderr)
        print(f"RERUN: {identity}", file=sys.stderr)
        return 1

    if completed.returncode != 0:
        print(f"FAIL {identity} (exit={completed.returncode})", file=sys.stderr)
        print(f"RERUN: {identity}", file=sys.stderr)
        return completed.returncode
    return 0


def probe_python_version(python: Path) -> str | None:
    identity = command_text([str(python), "--version"])
    try:
        completed = subprocess.run(
            [str(python), "--version"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=MAX_BOOTSTRAP_SUBPROCESS_RUNTIME_SECONDS,
        )
    except subprocess.TimeoutExpired:
        print(
            f"TIMEOUT {identity} (limit={MAX_BOOTSTRAP_SUBPROCESS_RUNTIME_SECONDS}s)",
            file=sys.stderr,
        )
        print(f"RERUN: {identity}", file=sys.stderr)
        return None
    except OSError as error:
        print(f"FAIL {identity}: {error}", file=sys.stderr)
        print(f"RERUN: {identity}", file=sys.stderr)
        return None

    if completed.returncode != 0:
        print(f"FAIL {identity} (exit={completed.returncode})", file=sys.stderr)
        print(f"RERUN: {identity}", file=sys.stderr)
        return None

    match = PYTHON_VERSION_RE.search(f"{completed.stdout}\n{completed.stderr}")
    if match is None:
        print(f"FAIL {identity}: Python version was not reported", file=sys.stderr)
        print(f"RERUN: {identity}", file=sys.stderr)
        return None
    return match.group(1)


def protected_snapshot() -> dict[Path, bytes | None]:
    snapshot: dict[Path, bytes | None] = {}
    for relative in PROTECTED_FILES:
        path = ROOT / relative
        snapshot[relative] = path.read_bytes() if path.is_file() else None
    return snapshot


def protected_files_unchanged(before: dict[Path, bytes | None]) -> bool:
    changed: list[str] = []
    for relative, previous in before.items():
        path = ROOT / relative
        current = path.read_bytes() if path.is_file() else None
        if current != previous:
            changed.append(str(relative))
    if changed:
        print(
            "FAIL: bootstrap changed protected files: " + ", ".join(changed),
            file=sys.stderr,
        )
        return False
    return True


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--venv", type=Path, default=ROOT / ".venv")
    args = parser.parse_args()

    try:
        required = (ROOT / ".python-version").read_text(encoding="utf-8").strip()
    except OSError as error:
        print(f"FAIL: could not read .python-version: {error}", file=sys.stderr)
        return 1

    actual = current_python_version()
    if actual != required:
        print(f"Python {required} required; running {actual}", file=sys.stderr)
        print("RERUN: py -3.13 scripts/bootstrap.py", file=sys.stderr)
        return 2

    venv_path = args.venv if args.venv.is_absolute() else ROOT / args.venv
    try:
        before = protected_snapshot()
    except OSError as error:
        print(f"FAIL: could not snapshot protected files: {error}", file=sys.stderr)
        return 1

    result = 0
    try:
        if not venv_path.exists():
            venv.EnvBuilder(with_pip=False).create(venv_path)

        python = venv_python_path(venv_path)
        if not python.is_file():
            print(f"FAIL: project Python was not created at {python}", file=sys.stderr)
            result = 2
        else:
            venv_version = probe_python_version(python)
            if venv_version != required:
                print(
                    f"FAIL: project .venv Python {required} required; "
                    f"found {venv_version or 'unknown'} at {python}",
                    file=sys.stderr,
                )
                result = 2
            else:
                commands = [
                    [str(python), "-m", "ensurepip", "--upgrade"],
                    [
                        str(python),
                        "-m",
                        "pip",
                        "install",
                        "-r",
                        str(ROOT / "python/requirements-dev.lock"),
                    ],
                    [
                        str(python),
                        "-m",
                        "pip",
                        "install",
                        "--no-deps",
                        "-e",
                        str(ROOT / "python"),
                    ],
                ]
                for command in commands:
                    result = run_command(command)
                    if result != 0:
                        break
                if result == 0:
                    print(f"PASS: project Python environment prepared at {python}")
    except OSError as error:
        print(f"FAIL: could not prepare project Python environment: {error}", file=sys.stderr)
        result = 1
    finally:
        if not protected_files_unchanged(before) and result == 0:
            result = 1

    return result


if __name__ == "__main__":
    raise SystemExit(main())
