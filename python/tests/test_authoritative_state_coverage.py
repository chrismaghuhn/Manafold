"""Small source-shape guards for authoritative EngineState integration paths."""

from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ENGINE_RS = ROOT / "crates/mtgml-state/src/engine.rs"
DIGEST_RS = ROOT / "crates/mtgml-state/src/digest_v5.rs"
MUTATION_RS = ROOT / "crates/mtgml-rules/src/contract.rs"


def _block(source: str, start: str, end: str) -> str:
    try:
        return source.split(start, 1)[1].split(end, 1)[0]
    except IndexError as error:
        raise AssertionError(f"source block not found: {start!r} .. {end!r}") from error


def _engine_state_fields(source: str) -> set[str]:
    body = _block(source, "pub struct EngineState {", "\n}")
    return set(re.findall(r"^\s*pub\s+(\w+)\s*:", body, re.MULTILINE))


def _reviewed_fields(source: str, end: str) -> set[str]:
    body = _block(source, "let EngineState {", end)
    return set(re.findall(r"^\s*(\w+)\s*[:,]", body, re.MULTILINE))


def _digest_output_fields(source: str) -> set[str]:
    body = _block(source, "Ok(FullStateDigestInputV5 {", "\n    })")
    return set(re.findall(r"^\s*(\w+)\s*:", body, re.MULTILINE))


def _digest_input_fields(source: str) -> set[str]:
    body = _block(source, "pub(crate) struct FullStateDigestInputV5 {", "\n}")
    return set(re.findall(r"^\s*pub\s+(\w+)\s*:", body, re.MULTILINE))


def _mutation_classifications(source: str) -> dict[str, str]:
    body = _block(source, "let EngineState {", "} = before;")
    return dict(re.findall(r"^\s*(\w+)\s*:\s*_,\s*//\s*(.+?)\s*$", body, re.MULTILINE))


def _assert_exact_coverage(authoritative: set[str], reviewed: set[str], path: str) -> None:
    if authoritative != reviewed:
        raise AssertionError(
            f"{path} EngineState coverage drift: "
            f"missing={sorted(authoritative - reviewed)}, "
            f"stale={sorted(reviewed - authoritative)}"
        )


class AuthoritativeStateIntegrationCoverageTests(unittest.TestCase):
    def test_current_engine_state_is_exhaustively_reviewed_by_digest(self) -> None:
        fields = _engine_state_fields(ENGINE_RS.read_text(encoding="utf-8"))
        digest_source = DIGEST_RS.read_text(encoding="utf-8")
        digest_fields = _reviewed_fields(digest_source, "} = state;")
        mapped_fields = _digest_output_fields(digest_source)
        input_fields = _digest_input_fields(digest_source)
        self.assertTrue(fields)
        _assert_exact_coverage(fields, digest_fields, "FullStateDigestV5")
        _assert_exact_coverage(fields, input_fields, "FullStateDigestInputV5")
        _assert_exact_coverage(fields, mapped_fields, "FullStateDigestV5 mapping")

    def test_current_engine_state_is_classified_for_mutation_ownership(self) -> None:
        fields = _engine_state_fields(ENGINE_RS.read_text(encoding="utf-8"))
        mutation_source = MUTATION_RS.read_text(encoding="utf-8")
        mutation_fields = _reviewed_fields(mutation_source, "} = before;")
        classifications = _mutation_classifications(mutation_source)
        self.assertTrue(fields)
        _assert_exact_coverage(fields, mutation_fields, "accepted-transition ownership")
        _assert_exact_coverage(fields, set(classifications), "mutation ownership classification")
        self.assertTrue(all(value.strip() for value in classifications.values()))

    def test_deliberately_unaccounted_authoritative_field_fails_both_guards(self) -> None:
        fields = _engine_state_fields(ENGINE_RS.read_text(encoding="utf-8"))
        digest_source = DIGEST_RS.read_text(encoding="utf-8")
        mapped_fields = _digest_output_fields(digest_source)
        mutation_fields = _reviewed_fields(MUTATION_RS.read_text(encoding="utf-8"), "} = before;")
        synthetic_authoritative_fields = fields | {"new_authoritative_family"}

        with self.assertRaisesRegex(AssertionError, "new_authoritative_family"):
            _assert_exact_coverage(
                synthetic_authoritative_fields, mapped_fields, "FullStateDigestV5 mapping"
            )
        with self.assertRaisesRegex(AssertionError, "new_authoritative_family"):
            _assert_exact_coverage(
                synthetic_authoritative_fields,
                mutation_fields,
                "accepted-transition ownership",
            )


if __name__ == "__main__":
    unittest.main()
