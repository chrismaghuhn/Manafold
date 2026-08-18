#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

from maintainer_common import (
    CAPABILITY_LIFECYCLE_THRESHOLDS,
    CARD_LIFECYCLE_THRESHOLDS,
    ROOT,
    MaintainerArtifactError,
    capability_census,
    load_json,
)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compute recursive card/bundle capability closure."
    )
    parser.add_argument("--bundle", required=True, type=Path)
    parser.add_argument("--registry", type=Path)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument(
        "--minimum-capability-lifecycle",
        choices=CAPABILITY_LIFECYCLE_THRESHOLDS,
        default="specified",
    )
    parser.add_argument(
        "--minimum-card-lifecycle",
        choices=CARD_LIFECYCLE_THRESHOLDS,
        default="draft",
    )
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    registry_path = args.registry or args.root / "cards" / "capabilities" / "registry.json"
    try:
        result = capability_census(
            load_json(args.bundle),
            load_json(registry_path),
            root=args.root,
            required_capability_lifecycle=args.minimum_capability_lifecycle,
            required_card_lifecycle=args.minimum_card_lifecycle,
        )
    except MaintainerArtifactError as exc:
        parser.error(str(exc))
    payload = result.to_dict()
    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        for key, values in payload.items():
            print(f"{key}: {len(values)}")
            for value in values:
                print(f"  - {value}")
    blocking_keys = (
        "missing",
        "cycles",
        "below_required_lifecycle",
        "missing_definitions",
        "card_lifecycle_blockers",
        "native_executors",
        "undeclared_native_executors",
        "stale_native_executor_declarations",
    )
    blocking = any(payload[key] for key in blocking_keys)
    raise SystemExit(2 if blocking else 0)


if __name__ == "__main__":
    main()
