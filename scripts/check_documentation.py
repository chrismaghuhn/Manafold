#!/usr/bin/env python3
from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from urllib.parse import unquote

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
LINK_RE = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")
ADR_RE = re.compile(r"^(\d{4})-")


def main() -> None:
    errors: list[str] = []
    register_path = ROOT / "docs" / "normative-document-register.v1.json"
    register = json.loads(register_path.read_text(encoding="utf-8"))
    paths: set[str] = set()
    for item in register.get("documents", []):
        path = item.get("path")
        if not isinstance(path, str):
            errors.append("normative register contains non-string path")
            continue
        if path in paths:
            errors.append(f"duplicate normative register path: {path}")
        paths.add(path)
        if not (ROOT / path).is_file():
            errors.append(f"registered document missing: {path}")

    # Every project/process document is classified. ADR files are governed by the
    # ADR index and numbering rules rather than duplicated in this register.
    classified_roots = {
        ROOT / "README.md",
        ROOT / "PROJECT_CHARTER.md",
        ROOT / "CONTRIBUTING.md",
        ROOT / "GOVERNANCE.md",
        ROOT / "MAINTAINERS.md",
        ROOT / "SECURITY.md",
        ROOT / "SUPPORT.md",
        ROOT / "CODE_OF_CONDUCT.md",
    }
    classified_docs = {
        path
        for path in (ROOT / "docs").rglob("*.md")
        if "adr" not in path.relative_to(ROOT / "docs").parts
        and path.relative_to(ROOT / "docs").parts[:2] != ("rules", "capabilities")
    }
    expected_classified = classified_roots | classified_docs
    registered_paths = {ROOT / path for path in paths}
    unregistered = sorted(path.relative_to(ROOT) for path in expected_classified - registered_paths)
    unexpected = sorted(
        path.relative_to(ROOT) for path in registered_paths if path not in expected_classified
    )
    if unregistered:
        errors.append(f"unregistered project/process documents: {unregistered}")
    if unexpected:
        errors.append(f"register contains out-of-scope paths: {unexpected}")

    for item in register.get("documents", []):
        path = item.get("path")
        if not isinstance(path, str):
            continue
        document_path = ROOT / path
        if document_path.suffix == ".md" and item.get("role") in {
            "normative",
            "process",
        }:
            header = document_path.read_text(encoding="utf-8")[:800]
            if "**Status:**" not in header and not path.endswith("CODE_OF_CONDUCT.md"):
                errors.append(
                    f"registered {item.get('role')} document lacks explicit status: {path}"
                )

    for document in sorted(ROOT.rglob("*.md")):
        text = document.read_text(encoding="utf-8")
        for match in LINK_RE.finditer(text):
            target = match.group(1).strip()
            if not target or target.startswith(("http://", "https://", "mailto:", "#", "sandbox:")):
                continue
            target = target.split("#", 1)[0]
            if not target:
                continue
            resolved = (document.parent / unquote(target)).resolve()
            try:
                resolved.relative_to(ROOT.resolve())
            except ValueError:
                errors.append(f"link escapes repository: {document.relative_to(ROOT)} -> {target}")
                continue
            if not resolved.exists():
                errors.append(f"broken local link: {document.relative_to(ROOT)} -> {target}")

    numbers: list[int] = []
    for path in sorted((ROOT / "docs" / "adr").glob("[0-9][0-9][0-9][0-9]-*.md")):
        match = ADR_RE.match(path.name)
        if match:
            numbers.append(int(match.group(1)))
    if numbers and numbers != list(range(min(numbers), max(numbers) + 1)):
        errors.append(f"ADR numbering gap or duplicate: {numbers}")

    required_index_tokens = [
        "M0_2_SPECIFICATION.md",
        "V0_2_1_CONTRACT_CLOSURE.md",
        "V0_2_2_EXECUTABLE_FREEZE_AND_MAINTAINER_ERGONOMICS.md",
        "NORMATIVE_HIERARCHY.md",
        "DOMAIN_MODEL.md",
        "ADDING_CARDS.md",
        "ADDING_RULES_AND_MECHANICS.md",
        "FREEZE_LEVELS.md",
    ]
    index = (ROOT / "docs" / "README.md").read_text(encoding="utf-8")
    for token in required_index_tokens:
        if token not in index:
            errors.append(f"documentation index omits {token}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        raise SystemExit(1)
    print(f"PASS: documentation register, {len(numbers)} ADRs, and local links verified")


if __name__ == "__main__":
    main()
