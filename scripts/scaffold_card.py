#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.dont_write_bytecode = True

from maintainer_common import (
    ROOT,
    definition_directory,
    digest_json,
    validate_card_manifest,
    write_json,
)


def main() -> None:
    parser = argparse.ArgumentParser(description="Scaffold a reviewed card-definition work item.")
    parser.add_argument("definition_id")
    parser.add_argument("display_name")
    parser.add_argument("--source-snapshot", default="TBD-source-snapshot")
    parser.add_argument("--source-record-id", default="TBD-source-record")
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--force", action="store_true")
    args = parser.parse_args()

    directory = definition_directory(args.definition_id, args.root)
    manifest_path = directory / "manifest.json"
    if directory.exists() and any(directory.iterdir()) and not args.force:
        raise SystemExit(f"refusing to overwrite nonempty directory: {directory}")
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "cases").mkdir(exist_ok=True)
    (directory / "cases" / ".gitkeep").touch()
    manifest = {
        "schema_version": "card-definition-manifest.v1",
        "definition_id": args.definition_id,
        "display_name": args.display_name,
        "source": {
            "snapshot": args.source_snapshot,
            "record_id": args.source_record_id,
            "normalized_digest": "0" * 64,
        },
        "faces": ["front"],
        "generated_objects": [],
        "required_capabilities": [],
        "decision_surface": [],
        "information_risks": ["UNREVIEWED"],
        "conformance_cases": [],
        "native_executor": None,
        "lifecycle": "draft",
        "implementation_path": None,
        "notes": "Replace placeholders; this scaffold is not a support claim.",
    }
    validate_card_manifest(manifest, root=args.root)
    write_json(manifest_path, manifest)
    readme = f"""# Card Work Item — `{args.definition_id}`\n\n**Display name:** {args.display_name}  \n**Lifecycle:** draft  \n**Manifest digest at scaffold:** `{digest_json(manifest)}`\n\nFollow `docs/cards/ADDING_CARDS.md`. Do not promote lifecycle until provenance, capability closure, decisions, information behavior, and conformance evidence are reviewed.\n"""
    (directory / "README.md").write_text(readme, encoding="utf-8")
    print(manifest_path.relative_to(args.root))


if __name__ == "__main__":
    main()
