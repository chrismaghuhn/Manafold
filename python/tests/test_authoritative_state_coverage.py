"""Small source-shape guards for authoritative EngineState integration paths."""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
STATE_SRC = ROOT / "crates/mtgml-state/src"
ENCODER_RS = STATE_SRC / "digest.rs"

# The state struct and its exhaustive destructure in the single-pass
# FullStateDigest encoder.
STATE_STRUCTS = ((STATE_SRC / "engine.rs", "pub struct EngineState {", "let EngineState {"),)


def _block(source: str, start: str, end: str) -> str:
    try:
        return source.split(start, 1)[1].split(end, 1)[0]
    except IndexError as error:
        raise AssertionError(f"source block not found: {start!r} .. {end!r}") from error


def _struct_fields(source: str, header: str) -> set[str]:
    body = _block(source, header, "\n}")
    return set(re.findall(r"^\s*pub\s+(\w+)\s*:", body, re.MULTILINE))


def _destructured_fields(source: str, header: str) -> set[str]:
    body = _block(source, header, "} = ")
    return set(re.findall(r"^\s*(\w+)\s*[:,]", body, re.MULTILINE))


def _assert_exact_coverage(authoritative: set[str], reviewed: set[str], path: str) -> None:
    if authoritative != reviewed:
        raise AssertionError(
            f"{path} coverage drift: "
            f"missing={sorted(authoritative - reviewed)}, "
            f"stale={sorted(reviewed - authoritative)}"
        )


class AuthoritativeStateIntegrationCoverageTests(unittest.TestCase):
    def test_every_state_field_is_destructured_by_the_digest_encoder(self) -> None:
        encoder = ENCODER_RS.read_text(encoding="utf-8")
        for struct_file, struct_header, destructure_header in STATE_STRUCTS:
            with self.subTest(struct=struct_header):
                fields = _struct_fields(struct_file.read_text(encoding="utf-8"), struct_header)
                self.assertTrue(fields)
                _assert_exact_coverage(
                    fields,
                    _destructured_fields(encoder, destructure_header),
                    "FullStateDigest encoder",
                )

    def test_deliberately_unaccounted_authoritative_field_fails_the_digest_guard(self) -> None:
        encoder = ENCODER_RS.read_text(encoding="utf-8")
        struct_file, struct_header, destructure_header = STATE_STRUCTS[0]
        fields = _struct_fields(struct_file.read_text(encoding="utf-8"), struct_header)
        synthetic_authoritative_fields = fields | {"new_authoritative_family"}

        with self.assertRaisesRegex(AssertionError, "new_authoritative_family"):
            _assert_exact_coverage(
                synthetic_authoritative_fields,
                _destructured_fields(encoder, destructure_header),
                "FullStateDigest encoder",
            )


if __name__ == "__main__":
    unittest.main()
