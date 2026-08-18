#!/usr/bin/env python3
"""Conservatively check Rust delimiter balance without claiming compilation."""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class Position:
    path: Path
    line: int
    column: int


def raw_string_start(text: str, index: int) -> tuple[int, str] | None:
    for prefix in ("br", "rb", "r"):
        if not text.startswith(prefix, index):
            continue
        cursor = index + len(prefix)
        hashes = 0
        while cursor < len(text) and text[cursor] == "#":
            hashes += 1
            cursor += 1
        if cursor < len(text) and text[cursor] == '"':
            return cursor + 1, '"' + ("#" * hashes)
    return None


def likely_char_literal(text: str, index: int) -> int | None:
    # Lifetimes such as 'a must not be consumed as character literals.
    cursor = index + 1
    if cursor >= len(text) or text[cursor] in "\r\n":
        return None
    if text[cursor] == "\\":
        cursor += 1
        if cursor >= len(text):
            return None
        if text[cursor] == "u" and cursor + 1 < len(text) and text[cursor + 1] == "{":
            close = text.find("}", cursor + 2)
            if close == -1:
                return None
            cursor = close + 1
        else:
            cursor += 1
    else:
        cursor += 1
    return cursor + 1 if cursor < len(text) and text[cursor] == "'" else None


def validate(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    stack: list[tuple[str, Position]] = []
    pairs = {")": "(", "]": "[", "}": "{"}
    openers = set(pairs.values())
    errors: list[str] = []
    index = 0
    line = 1
    column = 1
    block_depth = 0

    def advance(fragment: str) -> None:
        nonlocal line, column
        newline_count = fragment.count("\n")
        if newline_count:
            line += newline_count
            column = len(fragment.rsplit("\n", 1)[-1]) + 1
        else:
            column += len(fragment)

    while index < len(text):
        if block_depth:
            if text.startswith("/*", index):
                block_depth += 1
                advance("/*")
                index += 2
            elif text.startswith("*/", index):
                block_depth -= 1
                advance("*/")
                index += 2
            else:
                advance(text[index])
                index += 1
            continue

        if text.startswith("//", index):
            end = text.find("\n", index)
            end = len(text) if end == -1 else end
            advance(text[index:end])
            index = end
            continue
        if text.startswith("/*", index):
            block_depth = 1
            advance("/*")
            index += 2
            continue

        raw = raw_string_start(text, index)
        if raw is not None:
            content_start, terminator = raw
            end = text.find(terminator, content_start)
            if end == -1:
                errors.append(f"{path}:{line}:{column}: unterminated raw string")
                return errors
            fragment = text[index : end + len(terminator)]
            advance(fragment)
            index = end + len(terminator)
            continue

        if text[index] == '"':
            start = Position(path, line, column)
            cursor = index + 1
            escaped = False
            while cursor < len(text):
                char = text[cursor]
                if escaped:
                    escaped = False
                elif char == "\\":
                    escaped = True
                elif char == '"':
                    cursor += 1
                    break
                cursor += 1
            else:
                errors.append(f"{start.path}:{start.line}:{start.column}: unterminated string")
                return errors
            fragment = text[index:cursor]
            advance(fragment)
            index = cursor
            continue

        if text[index] == "'":
            char_end = likely_char_literal(text, index)
            if char_end is not None:
                fragment = text[index:char_end]
                advance(fragment)
                index = char_end
                continue

        char = text[index]
        if char in openers:
            stack.append((char, Position(path, line, column)))
        elif char in pairs:
            if not stack or stack[-1][0] != pairs[char]:
                errors.append(f"{path}:{line}:{column}: unmatched {char}")
                return errors
            stack.pop()
        advance(char)
        index += 1

    if block_depth:
        errors.append(f"{path}: unterminated block comment")
    for opener, position in reversed(stack):
        errors.append(
            f"{position.path}:{position.line}:{position.column}: unclosed {opener}"
        )
    return errors


def main() -> None:
    paths = sorted((ROOT / "crates").rglob("*.rs"))
    errors = [error for path in paths for error in validate(path)]
    if errors:
        raise SystemExit("FAIL:\n" + "\n".join(errors))
    print(f"PASS: Rust lexical structure ({len(paths)} files)")


if __name__ == "__main__":
    main()
