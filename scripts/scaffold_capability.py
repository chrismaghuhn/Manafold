#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.dont_write_bytecode = True

from maintainer_common import (
    ROOT,
    ensure_capability_key,
    load_json,
    validate_capability_registry,
    write_json,
)


def category_for(key: str) -> str:
    if key.startswith("rules/"):
        return "core_rule"
    if key.startswith("mechanic/"):
        return "mechanic"
    if key.startswith("format/"):
        return "format_policy"
    if key.startswith("decision/"):
        return "decision"
    if key.startswith("visibility/"):
        return "visibility"
    return "tooling"


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Register and scaffold a capability specification."
    )
    parser.add_argument("key")
    parser.add_argument("title")
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--registry", type=Path)
    args = parser.parse_args()
    ensure_capability_key(args.key)

    registry_path = args.registry or args.root / "cards" / "capabilities" / "registry.json"
    registry = load_json(registry_path)
    validate_capability_registry(registry, root=args.root)
    entries = registry.setdefault("entries", [])
    if any(entry.get("key") == args.key for entry in entries):
        raise SystemExit(f"capability already exists: {args.key}")

    spec_rel = Path("docs") / "rules" / "capabilities" / Path(*args.key.split("/"))
    spec_rel = spec_rel.with_suffix(".md")
    spec_path = args.root / spec_rel
    spec_path.parent.mkdir(parents=True, exist_ok=True)
    if spec_path.exists():
        raise SystemExit(f"spec already exists: {spec_path}")
    spec_path.write_text(
        f"# Mechanic Specification — {args.title}\n\n"
        f"**Capability key/version:** `{args.key}@0.1.0`  \n"
        "**Lifecycle:** proposed  \n"
        "**Authority snapshots:** TBD\n\n"
        "## Supported scope\n\n## Explicit exclusions\n\n## State and identity model\n\n"
        "## Events and replacement points\n\n## Decisions and ordering\n\n"
        "## Information and opaque identities\n\n## Transition/continuation behavior\n\n"
        "## Conformance, property, replay, and performance evidence\n",
        encoding="utf-8",
    )
    entries.append(
        {
            "key": args.key,
            "version": "0.1.0",
            "category": category_for(args.key),
            "lifecycle": "proposed",
            "summary": args.title,
            "dependencies": [],
            "authority_refs": [],
            "spec_path": spec_rel.as_posix(),
            "implementation_paths": [],
            "conformance_cases": [],
            "information_risk": "unreviewed",
            "benchmark_scenarios": [],
            "owners": ["TBD-owner-role"],
            "notes": "Generated proposal; complete the specification before implementation.",
        }
    )
    entries.sort(key=lambda item: item["key"])
    validate_capability_registry(registry, root=args.root)
    write_json(registry_path, registry)
    print(spec_rel.as_posix())


if __name__ == "__main__":
    main()
