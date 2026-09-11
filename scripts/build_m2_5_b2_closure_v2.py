#!/usr/bin/env python3
"""Build the deterministic, non-current B2 closure-v2 artifact."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from b2_closure_v2_support import B2_CLOSURE_V2_PATH, build_closure_payload


def build_closure_v2(repo_root: Path) -> dict[str, object]:
    return build_closure_payload(repo_root)


def render_closure_v2(value: dict[str, object]) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_closure_v2(repo_root: Path, value: dict[str, object]) -> Path:
    path = repo_root / B2_CLOSURE_V2_PATH
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(render_closure_v2(value))
    return path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument(
        "--check", action="store_true", help="verify the existing artifact without rewriting it"
    )
    args = parser.parse_args()
    value = build_closure_v2(args.repo_root)
    if args.check:
        from check_m2_5_b2_closure_v2 import verify_closure_v2

        verify_closure_v2(args.repo_root, args.repo_root / B2_CLOSURE_V2_PATH)
        print("B2_CLOSURE_V2_CHECK = PASS")
    else:
        path = write_closure_v2(args.repo_root, value)
        print(f"B2_CLOSURE_V2_WRITTEN = {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
