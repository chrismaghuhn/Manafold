#!/usr/bin/env python3
"""Extract exact Oracle records from the pinned Scryfall Oracle Cards archive.

Each record carries the SHA-256 of its exact JSONL line bytes, including the
terminating LF, as stored in the archive: the source-record digest that
content provenance pins (see cards/definitions/basic-land-v1/README.md and
cards/definitions/basic-land-and-vanilla-creature-v1/README.md). The archive
itself is checked against its pinned SHA-256 before any record is read.
Records are looked up by exact card name and by Oracle id. Output is a JSON
array: the names first, then the Oracle ids, each in the order given.

With --vanilla-creature every record must also be a vanilla creature in the
sense of the vanilla-creature profile; otherwise it is refused.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True

PINNED_ARCHIVE_SHA256 = "c607300fe03ce0d9f59181b1bb33d8001e2fa339b8c68eefd70a6501fa757623"
FIELDS = (
    "oracle_id",
    "name",
    "layout",
    "mana_cost",
    "type_line",
    "oracle_text",
    "keywords",
    "power",
    "toughness",
)
CREATURE_TYPE_LINE_PREFIX = "Creature — "
INTEGER = re.compile(r"-?[0-9]+")


def vanilla_creature_failures(record: dict[str, object]) -> list[str]:
    """Why a raw Oracle record is not a vanilla creature (empty if it is one)."""
    failures: list[str] = []
    if record.get("layout") != "normal":
        failures.append(f"layout is {record.get('layout')!a}, not 'normal'")
    if record.get("card_faces"):
        failures.append("it has card_faces")
    if record.get("oracle_text") != "":
        failures.append("it has oracle text")
    if record.get("keywords") != []:
        failures.append(f"keywords are {record.get('keywords')!a}, not []")
    type_line = record.get("type_line")
    if not isinstance(type_line, str) or not type_line.startswith(CREATURE_TYPE_LINE_PREFIX):
        failures.append(f"type line {type_line!a} is not 'Creature - <subtypes>'")
    power, toughness = record.get("power"), record.get("toughness")
    if not isinstance(power, str) or not INTEGER.fullmatch(power):
        failures.append(f"power {power!a} is not an integer")
    elif int(power) < 0:
        failures.append(f"power {power} is below 0")
    if not isinstance(toughness, str) or not INTEGER.fullmatch(toughness):
        failures.append(f"toughness {toughness!a} is not an integer")
    elif int(toughness) < 1:
        failures.append(f"toughness {toughness} is below 1")
    return failures


def extract(
    archive: Path,
    archive_sha256: str,
    names: list[str],
    oracle_ids: list[str] | None = None,
    vanilla_creature: bool = False,
) -> list[dict[str, object]]:
    oracle_ids = oracle_ids or []
    raw = archive.read_bytes()
    actual = hashlib.sha256(raw).hexdigest()
    if actual != archive_sha256:
        raise SystemExit(f"archive SHA-256 {actual} does not match the pinned {archive_sha256}")
    Match = tuple[dict[str, object], list[str]]
    by_name: dict[str, list[Match]] = {name: [] for name in names}
    by_id: dict[str, list[Match]] = {oracle_id: [] for oracle_id in oracle_ids}
    for line in gzip.decompress(raw).splitlines(keepends=True):
        record = json.loads(line)
        name, oracle_id = record.get("name"), record.get("oracle_id")
        if name not in by_name and oracle_id not in by_id:
            continue
        entry = {field: record.get(field) for field in FIELDS}
        entry["record_sha256"] = hashlib.sha256(line).hexdigest()
        match = (entry, vanilla_creature_failures(record))
        if name in by_name:
            by_name[name].append(match)
        if oracle_id in by_id:
            by_id[oracle_id].append(match)
    found: list[dict[str, object]] = []
    for kind, matches, keys in (
        ("named", by_name, names),
        ("with Oracle id", by_id, oracle_ids),
    ):
        for key in keys:
            if len(matches[key]) != 1:
                count = len(matches[key])
                raise SystemExit(f"expected exactly one record {kind} {key!a}, found {count}")
            entry, failures = matches[key][0]
            if vanilla_creature and failures:
                reason = "; ".join(failures)
                raise SystemExit(f"{entry['name']!a} is not a vanilla creature: {reason}")
            found.append(entry)
    return found


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("archive", type=Path, help="oracle-cards-<snapshot>.jsonl.gz")
    parser.add_argument("names", nargs="*", help="exact card names")
    parser.add_argument(
        "--oracle-id", action="append", default=[], help="Oracle id (repeat for several)"
    )
    parser.add_argument(
        "--vanilla-creature",
        action="store_true",
        help="refuse any record that is not a vanilla creature",
    )
    parser.add_argument("--archive-sha256", default=PINNED_ARCHIVE_SHA256)
    args = parser.parse_args()
    if not args.names and not args.oracle_id:
        parser.error("give at least one card name or --oracle-id")
    records = extract(
        args.archive, args.archive_sha256, args.names, args.oracle_id, args.vanilla_creature
    )
    json.dump(records, sys.stdout, ensure_ascii=False, indent=2)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
