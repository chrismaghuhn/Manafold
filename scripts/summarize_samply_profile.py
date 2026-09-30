#!/usr/bin/env python3
"""Summarize a samply profile without the Firefox profiler UI.

Reads a profile recorded with
`samply record --save-only --unstable-presymbolicate -o DIR/profile.json.gz`
and prints the functions with the most samples, inclusive and self. Extra
arguments break the profile down further:

  NAME     direct callees of the first function whose name contains NAME
  ^NAME    nearest callers of the function whose name contains NAME

See docs/agents/profiling.md.
"""

from __future__ import annotations

import argparse
import bisect
import collections
import gzip
import json
import re
import sys
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True


def load(directory: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    profile = json.loads(gzip.decompress((directory / "profile.json.gz").read_bytes()))
    symbols = json.loads((directory / "profile.json.syms.json").read_text(encoding="utf-8"))
    return profile, symbols


class Symbolizer:
    def __init__(self, profile: dict[str, Any], symbols: dict[str, Any], thread: dict[str, Any]):
        self.libs = profile["libs"]
        self.strings = symbols["string_table"]
        self.tables: dict[str, tuple[list[int], list[dict[str, int]]]] = {}
        for lib in symbols["data"]:
            table = sorted(lib["symbol_table"], key=lambda entry: entry["rva"])
            self.tables[lib["debug_name"]] = ([entry["rva"] for entry in table], table)
        self.thread = thread
        self.frames: dict[int, str] = {}
        self.stacks: dict[int, list[str]] = {}

    def frame(self, frame: int) -> str:
        if frame in self.frames:
            return self.frames[frame]
        frames = self.thread["frameTable"]
        func = frames["func"][frame]
        resource = self.thread["funcTable"]["resource"][func]
        address = frames["address"][frame]
        name = f"?{address}"
        if resource is not None and resource >= 0:
            lib = self.libs[self.thread["resourceTable"]["lib"][resource]]["debugName"]
            name = f"{lib}+{address}"
            if lib in self.tables and address is not None and address >= 0:
                rvas, table = self.tables[lib]
                index = bisect.bisect_right(rvas, address) - 1
                if index >= 0 and address < table[index]["rva"] + max(table[index]["size"], 1):
                    name = re.sub(r"::h[0-9a-f]{16}$", "", self.strings[table[index]["symbol"]])
        self.frames[frame] = name
        return name

    def stack(self, stack: int) -> list[str]:
        """Function names of a stack, leaf first."""
        if stack in self.stacks:
            return self.stacks[stack]
        names = []
        current: int | None = stack
        while current is not None:
            names.append(self.frame(self.thread["stackTable"]["frame"][current]))
            current = self.thread["stackTable"]["prefix"][current]
        self.stacks[stack] = names
        return names


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("directory", type=Path, help="directory holding profile.json.gz")
    parser.add_argument("queries", nargs="*", help="NAME for callees, ^NAME for callers")
    parser.add_argument("--top", type=int, default=40, help="rows per table")
    parser.add_argument(
        "--match", default="mtgml", help="substring for functions shown in the inclusive table"
    )
    args = parser.parse_args()

    profile, symbols = load(args.directory.expanduser())
    thread = max(profile["threads"], key=lambda thread: thread["samples"]["length"])
    symbolizer = Symbolizer(profile, symbols, thread)
    callees_of = [query for query in args.queries if not query.startswith("^")]
    callers_of = [query[1:] for query in args.queries if query.startswith("^")]

    samples = thread["samples"]
    weights = samples.get("weight") or [1] * samples["length"]
    total = 0
    inclusive: collections.Counter[str] = collections.Counter()
    self_time: collections.Counter[str] = collections.Counter()
    callees = {query: collections.Counter[str]() for query in callees_of}
    callers = {query: collections.Counter[str]() for query in callers_of}
    for stack, weight in zip(samples["stack"], weights, strict=True):
        if stack is None:
            continue
        weight = weight or 1
        total += weight
        names = symbolizer.stack(stack)
        self_time[names[0]] += weight
        for name in set(names):
            inclusive[name] += weight
        root_first = names[::-1]
        for query in callees_of:
            index = next((i for i, name in enumerate(root_first) if query in name), None)
            if index is not None:
                callee = next(
                    (name for name in root_first[index + 1 :] if args.match in name), "(self)"
                )
                callees[query][callee] += weight
        for query in callers_of:
            index = next((i for i, name in enumerate(names) if query in name), None)
            if index is not None:
                caller = next(
                    (
                        name
                        for name in names[index + 1 :]
                        if args.match in name and query not in name
                    ),
                    "(none)",
                )
                callers[query][caller] += weight

    def table(title: str, counter: collections.Counter[str], base: int, only: str = "") -> None:
        print(f"\n== {title} ==")
        rows = [(name, count) for name, count in counter.most_common() if only in name]
        for name, count in rows[: args.top]:
            print(f"{100 * count / max(base, 1):5.1f}%  {name[:160]}")

    print(f"thread {thread.get('name')}: {total} samples")
    table(f"inclusive, functions matching '{args.match}'", inclusive, total, args.match)
    table("self", self_time, total)
    for query, counter in callees.items():
        base = sum(counter.values())
        table(f"direct callees of '{query}' (% of its {base} samples)", counter, base)
    for query, counter in callers.items():
        base = sum(counter.values())
        table(f"nearest callers of '{query}' (% of its {base} samples)", counter, base)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
