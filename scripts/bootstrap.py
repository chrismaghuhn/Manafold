#!/usr/bin/env python3
"""Prepare .venv without mutating contract or lock files."""

from __future__ import annotations

import argparse
import subprocess
import sys
import venv
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--venv", type=Path, default=ROOT / ".venv")
    args = parser.parse_args()
    required = (ROOT / ".python-version").read_text(encoding="utf-8").strip()
    actual = ".".join(map(str, sys.version_info[:3]))
    if actual != required:
        print(f"Python {required} required; running {actual}", file=sys.stderr)
        return 2
    if not args.venv.exists():
        venv.EnvBuilder(with_pip=True).create(args.venv)
    py = args.venv / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")
    commands = [
        [
            str(py),
            "-m",
            "pip",
            "install",
            "-r",
            str(ROOT / "python/requirements-dev.lock"),
        ],
        [str(py), "-m", "pip", "install", "--no-deps", "-e", str(ROOT / "python")],
    ]
    for command in commands:
        print("+", " ".join(command))
        try:
            subprocess.run(command, cwd=ROOT, check=True)
        except subprocess.CalledProcessError as exc:
            print(
                "bootstrap could not install pinned tooling;"
                " package-index access may be unavailable",
                file=sys.stderr,
            )
            return exc.returncode
    print("PASS: Python environment prepared; Rust and Cargo.lock were not mutated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
