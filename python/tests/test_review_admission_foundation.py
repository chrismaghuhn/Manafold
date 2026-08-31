from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import cast

import jsonschema

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from authority_source_resolver import (
    AuthoritySourceResolver,
    ResolutionError,
    ResolutionStatus,
)
from mtgml.authority import (
    REVIEWER_ROSTER_SCHEMA_V1,
    ReviewerRosterRefV1,
)

ROSTER_DIGEST = "6238d8ff880460adddacc8f1c79ae972d0db150ae19b5ea636431d3f4e90cd36"
ROSTER_PATH = "sources/m2_5/authorities/reviewer_rosters/v1/" + ROSTER_DIGEST + ".json"
CHECKLIST_PATH = "docs/maintenance/INTERACTION_AUTHORITY_REVIEW_CHECKLIST.md"
EXPECTED_ROLES = (
    "architecture_maintainer",
    "conformance_maintainer",
    "information_safety_reviewer",
    "project_owner",
    "rules_authority_maintainer",
)


class ReviewAdmissionFoundationTests(unittest.TestCase):
    def _roster_ref(self) -> ReviewerRosterRefV1:
        return ReviewerRosterRefV1(
            path=ROSTER_PATH,
            schema=REVIEWER_ROSTER_SCHEMA_V1,
            raw_sha256=bytes.fromhex(ROSTER_DIGEST),
        )

    def test_production_roster_resolves_with_exact_content_addressing(self) -> None:
        path = ROOT / Path(*ROSTER_PATH.split("/"))
        raw = path.read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(), ROSTER_DIGEST)

        artifact = AuthoritySourceResolver(ROOT).resolve_reviewer_roster_leaf(self._roster_ref())
        self.assertEqual(artifact.raw_bytes, raw)
        value = cast(dict[str, object], json.loads(raw.decode("utf-8")))
        schema = json.loads(
            (ROOT / "schemas" / "reviewer-roster.v1.schema.json").read_text(encoding="utf-8")
        )
        jsonschema.Draft202012Validator(schema).validate(value)
        self.assertEqual(value["schema"], REVIEWER_ROSTER_SCHEMA_V1)
        reviewers = cast(list[dict[str, object]], value["reviewers"])
        self.assertEqual(
            reviewers,
            [{"reviewer_id": "chrismaghuhn", "roles": list(EXPECTED_ROLES)}],
        )

    def test_production_roster_is_consumed_by_authority_validator(self) -> None:
        from authority_validator import AuthorityValidator

        roster = AuthorityValidator(AuthoritySourceResolver(ROOT))._parse_roster(self._roster_ref())
        self.assertEqual(len(roster.reviewers), 1)
        self.assertEqual(roster.reviewers[0].reviewer_id, "chrismaghuhn")
        self.assertEqual(roster.reviewers[0].roles, EXPECTED_ROLES)

    def test_tampered_production_roster_bytes_fail_closed(self) -> None:
        source = ROOT / Path(*ROSTER_PATH.split("/"))
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            target = repository / Path(*ROSTER_PATH.split("/"))
            target.parent.mkdir(parents=True)
            target.write_bytes(source.read_bytes() + b"tampered")
            with self.assertRaises(ResolutionError) as context:
                AuthoritySourceResolver(repository).resolve_reviewer_roster_leaf(self._roster_ref())
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "SOURCE_DIGEST_MISMATCH")

    def test_unknown_production_roster_identity_fails_closed(self) -> None:
        unknown_digest = "11" * 32
        reference = ReviewerRosterRefV1(
            path=("sources/m2_5/authorities/reviewer_rosters/v1/" + unknown_digest + ".json"),
            schema=REVIEWER_ROSTER_SCHEMA_V1,
            raw_sha256=bytes.fromhex(unknown_digest),
        )
        with self.assertRaises(ResolutionError) as context:
            AuthoritySourceResolver(ROOT).resolve_reviewer_roster_leaf(reference)
        self.assertEqual(context.exception.status, ResolutionStatus.FAIL)
        self.assertEqual(context.exception.code, "REPOSITORY_SOURCE_MISSING")

    def test_checklist_is_registered_and_contains_review_contract(self) -> None:
        checklist = (ROOT / Path(*CHECKLIST_PATH.split("/"))).read_text(encoding="utf-8")
        lowered = checklist.lower()
        for phrase in (
            "interaction-authority-review-checklist.v1",
            "source and locator verification",
            "theorem/application binding",
            "per-member preconditions",
            "b2",
            "b1.final",
            "information-safety",
            "lexical",
            "co-occurrence",
            "absence-of-evidence",
            "source-binding closure",
            "supersession",
        ):
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, lowered)

        register = json.loads(
            (ROOT / "docs" / "normative-document-register.v1.json").read_text(encoding="utf-8")
        )
        registered_paths = [item["path"] for item in register["documents"]]
        self.assertIn(CHECKLIST_PATH, registered_paths)

    def test_checklist_defines_solo_separate_self_review(self) -> None:
        checklist = (ROOT / Path(*CHECKLIST_PATH.split("/"))).read_text(encoding="utf-8")
        lowered = checklist.lower()
        self.assertIn("solo separate self-review", lowered)
        for phrase in (
            "separate review pass after proposal/artifact generation",
            "frozen exact bytes/identities/source bindings",
            "no semantic edits during the acceptance pass",
            "requires a fresh pass",
            "complete checklist again",
            "portable review evidence",
            "solo mode does not weaken semantic or information-safety review",
        ):
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, lowered)

    def test_roster_source_has_no_runtime_identity_dependency(self) -> None:
        reference = self._roster_ref()
        with self.assertRaises(AttributeError):
            reference.path = "mutated"  # type: ignore[misc]
        self.assertTrue(ROSTER_PATH.startswith("sources/"))
        self.assertNotIn("\\", ROSTER_PATH)
        self.assertFalse(Path(ROSTER_PATH).is_absolute())
        resolver_source = (ROOT / "scripts" / "authority_source_resolver.py").read_text(
            encoding="utf-8"
        )
        for forbidden in ("github.com", "subprocess", "urllib", "requests"):
            with self.subTest(forbidden=forbidden):
                self.assertNotIn(forbidden, resolver_source)


if __name__ == "__main__":
    unittest.main()
