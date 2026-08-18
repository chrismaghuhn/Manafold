#!/usr/bin/env python3
"""Create the next ADR from the repository template."""

from __future__ import annotations

import argparse
import re
import sys
from datetime import date
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[1]
ADR_DIR = ROOT / "docs" / "adr"
TEMPLATE = ADR_DIR / "0000-template.md"
ADR_PATTERN = re.compile(r"^(\d{4})-[a-z0-9][a-z0-9-]*\.md$")


def slugify(title: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")
    if not slug:
        raise ValueError("title does not contain a usable slug")
    return slug


def next_number() -> int:
    numbers = [
        int(match.group(1))
        for path in ADR_DIR.glob("*.md")
        if (match := ADR_PATTERN.match(path.name))
    ]
    return max(numbers, default=0) + 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("title", help="human-readable ADR title")
    args = parser.parse_args()

    number = next_number()
    slug = slugify(args.title)
    destination = ADR_DIR / f"{number:04d}-{slug}.md"
    if destination.exists():
        raise FileExistsError(destination)

    content = TEMPLATE.read_text(encoding="utf-8")
    content = content.replace("# ADR 0000: Title", f"# ADR {number:04d}: {args.title}")
    content = content.replace("YYYY-MM-DD", date.today().isoformat())
    destination.write_text(content, encoding="utf-8")
    print(destination.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
