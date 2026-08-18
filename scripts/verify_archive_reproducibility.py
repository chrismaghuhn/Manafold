#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import stat
import sys
import tempfile
import zipfile
from pathlib import Path, PurePosixPath

sys.dont_write_bytecode = True

from build_source_archive import archive, reproducibility_config


def verify_safe(path: Path) -> int:
    expected_prefix = f"{reproducibility_config()['archive_prefix']}/"
    with zipfile.ZipFile(path) as handle:
        names = handle.namelist()
        if len(names) != len(set(names)):
            raise ValueError("duplicate archive member")
        for name in names:
            pure = PurePosixPath(name)
            if pure.is_absolute() or ".." in pure.parts or "" in pure.parts:
                raise ValueError(f"unsafe archive member: {name}")
            info = handle.getinfo(name)
            if info.is_dir():
                raise ValueError(f"unexpected directory entry: {name}")
            mode = (info.external_attr >> 16) & 0o170000
            if mode not in {0, stat.S_IFREG}:
                raise ValueError(f"non-regular archive member: {name}")
            if not name.startswith(expected_prefix):
                raise ValueError(f"unexpected archive root: {name}")
            handle.read(name)
        return len(names)


def main() -> None:
    with tempfile.TemporaryDirectory() as directory:
        first = Path(directory) / "a.zip"
        second = Path(directory) / "b.zip"
        digest_a = archive(first)
        digest_b = archive(second)
        if first.read_bytes() != second.read_bytes() or digest_a != digest_b:
            raise SystemExit("deterministic archive mismatch")
        count = verify_safe(first)
        actual = hashlib.sha256(first.read_bytes()).hexdigest()
        if actual != digest_a:
            raise SystemExit("archive digest mismatch")
        print(f"PASS: deterministic source archive ({count} safe files, sha256={actual})")


if __name__ == "__main__":
    main()
