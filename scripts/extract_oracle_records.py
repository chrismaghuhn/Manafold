#!/usr/bin/env python3
"""Extract exact Oracle records from the pinned Scryfall Oracle Cards archive.

Each record carries the SHA-256 of its exact JSONL line bytes, including the
terminating LF, as stored in the archive: the source-record digest that
content provenance pins (see cards/definitions/basic-land-v1/README.md). The
archive itself is checked against its pinned SHA-256 before any record is
read. Output is a JSON array in the order the names were given.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

PINNED_ARCHIVE_SHA256 = "c607300fe03ce0d9f59181b1bb33d8001e2fa339b8c68eefd70a6501fa757623"
FIELDS = ("oracle_id", "name", "mana_cost", "type_line", "oracle_text", "power", "toughness")


def extract(archive: Path, archive_sha256: str, names: list[str]) -> list[dict[str, object]]:
    raw = archive.read_bytes()
    actual = hashlib.sha256(raw).hexdigest()
    if actual != archive_sha256:
        raise SystemExit(f"archive SHA-256 {actual} does not match the pinned {archive_sha256}")
    wanted = set(names)
    found: dict[str, list[dict[str, object]]] = {name: [] for name in names}
    for line in gzip.decompress(raw).splitlines(keepends=True):
        record = json.loads(line)
        name = record.get("name")
        if name not in wanted:
            continue
        entry = {field: record.get(field) for field in FIELDS}
        entry["record_sha256"] = hashlib.sha256(line).hexdigest()
        found[name].append(entry)
    for name in names:
        if len(found[name]) != 1:
            count = len(found[name])
            raise SystemExit(f"expected exactly one record named {name!r}, found {count}")
    return [found[name][0] for name in names]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("archive", type=Path, help="oracle-cards-<snapshot>.jsonl.gz")
    parser.add_argument("names", nargs="+", help="exact card names")
    parser.add_argument("--archive-sha256", default=PINNED_ARCHIVE_SHA256)
    args = parser.parse_args()
    records = extract(args.archive, args.archive_sha256, args.names)
    json.dump(records, sys.stdout, ensure_ascii=False, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
