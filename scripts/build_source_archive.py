#!/usr/bin/env python3
"""Build a deterministic source ZIP and SHA-256 sidecar."""

from __future__ import annotations

import argparse
import hashlib
import os
import stat
import sys
import tomllib
import zipfile
from collections.abc import Iterable
from datetime import UTC, datetime
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
REPRODUCIBILITY_CONFIG = ROOT / "config" / "reproducibility.toml"
EXCLUDED_PARTS = {
    ".git",
    ".venv",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    "target",
    "dist",
}
EXCLUDED_SUFFIXES = {".pyc", ".pyo", ".zip"}


def source_files() -> Iterable[Path]:
    for path in sorted(ROOT.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(ROOT)
        if any(part in EXCLUDED_PARTS for part in relative.parts):
            continue
        if path.suffix.lower() in EXCLUDED_SUFFIXES:
            continue
        if path.is_symlink():
            raise ValueError(f"refusing to archive symbolic link: {relative}")
        yield path


def reproducibility_config() -> dict[str, object]:
    with REPRODUCIBILITY_CONFIG.open("rb") as handle:
        config = tomllib.load(handle)
    if config.get("manifest_version") != "reproducibility.v1":
        raise ValueError("unsupported reproducibility manifest")
    if config.get("zip_compression") != "deflate-9":
        raise ValueError("unsupported ZIP compression policy")
    return config


def zip_timestamp() -> tuple[int, int, int, int, int, int]:
    config = reproducibility_config()
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", str(config["source_date_epoch"])))
    moment = datetime.fromtimestamp(epoch, tz=UTC)
    if moment.year < 1980:
        moment = datetime(1980, 1, 1, tzinfo=UTC)
    return (
        moment.year,
        moment.month,
        moment.day,
        moment.hour,
        moment.minute,
        moment.second - (moment.second % 2),
    )


def archive(output: Path) -> str:
    output.parent.mkdir(parents=True, exist_ok=True)
    timestamp = zip_timestamp()
    prefix = str(reproducibility_config()["archive_prefix"])

    with zipfile.ZipFile(
        output,
        mode="w",
        compression=zipfile.ZIP_DEFLATED,
        compresslevel=9,
    ) as handle:
        for path in source_files():
            relative = path.relative_to(ROOT).as_posix()
            info = zipfile.ZipInfo(f"{prefix}/{relative}", date_time=timestamp)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            mode = 0o755 if path.parent.name == "scripts" and path.suffix == ".py" else 0o644
            info.external_attr = (stat.S_IFREG | mode) << 16
            handle.writestr(info, path.read_bytes(), compress_type=zipfile.ZIP_DEFLATED)

    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    return digest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=ROOT / "dist",
        help="directory for ZIP and checksum",
    )
    parser.add_argument(
        "--filename",
        default=f"{reproducibility_config()['archive_prefix']}.zip",
        help="archive filename",
    )
    args = parser.parse_args()

    output = args.output_dir.resolve() / args.filename
    checksum = output.with_suffix(output.suffix + ".sha256")
    digest = archive(output)
    checksum.write_text(f"{digest}  {output.name}\n", encoding="utf-8")
    print(output)
    print(checksum)
    print(digest)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
