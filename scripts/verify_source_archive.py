#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import stat
import sys
from pathlib import Path, PurePosixPath
from zipfile import ZipFile

sys.dont_write_bytecode = True

from build_source_archive import reproducibility_config, source_files

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("archive")
    args = parser.parse_args()
    config = reproducibility_config()
    expected_prefix = f"{config['archive_prefix']}/"
    expected_members = {
        f"{expected_prefix}{path.relative_to(ROOT).as_posix()}": path
        for path in source_files()
    }
    with ZipFile(args.archive) as archive:
        names = archive.namelist()
        if len(names) != len(set(names)):
            raise SystemExit("duplicate archive member")
        actual_members = set(names)
        if actual_members != set(expected_members):
            missing = sorted(set(expected_members) - actual_members)
            unexpected = sorted(actual_members - set(expected_members))
            raise SystemExit(
                f"archive/source member mismatch: missing={missing[:5]}, "
                f"unexpected={unexpected[:5]}"
            )
        for name in names:
            path = PurePosixPath(name)
            if "\\" in name or path.is_absolute() or ".." in path.parts or "" in path.parts:
                raise SystemExit(f"unsafe archive member: {name}")
            if not name.startswith(expected_prefix):
                raise SystemExit(f"unexpected archive root: {name}")
            if any(part in {"__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache", "target"} for part in path.parts):
                raise SystemExit(f"generated/cache member in archive: {name}")
            info = archive.getinfo(name)
            if info.is_dir():
                raise SystemExit(f"unexpected directory entry: {name}")
            mode = (info.external_attr >> 16) & 0o170000
            if mode not in {0, stat.S_IFREG}:
                raise SystemExit(f"non-regular archive member: {name}")
            payload = archive.read(name)
            if payload != expected_members[name].read_bytes():
                raise SystemExit(f"archive/source byte mismatch: {name}")
    digest = hashlib.sha256(Path(args.archive).read_bytes()).hexdigest()
    print(f"PASS: {len(names)} safe files, sha256={digest}")


if __name__ == "__main__":
    main()
