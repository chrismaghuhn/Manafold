from __future__ import annotations

import copy
import os
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

from validate_m2_5_exact_two_deck_scope_lock import (
    EXPECTED_SOURCE_PACKAGE_SHA256,
    ScopeLockValidationError,
    compute_deck_content_sha256,
    load_lock,
    validate_lock_document,
    validate_scope_matrix,
    verify_pinned_archive,
)

LOCK_PATH = ROOT / "sources/m2_5/scope/exact_two_deck_scope_lock.v1.json"


class ExactTwoDeckScopeLockTests(unittest.TestCase):
    def setUp(self) -> None:
        self.lock = load_lock(LOCK_PATH)

    def test_lock_contains_exact_selected_pair_and_source_binding(self) -> None:
        validate_lock_document(self.lock)
        validate_scope_matrix(self.lock)

        self.assertEqual(self.lock["deck_count"], 2)
        self.assertEqual(
            [deck["deck_name"] for deck in self.lock["decks"]],
            ["Token Triumph", "Grave Danger"],
        )
        self.assertEqual(
            [deck["commander"]["card_name"] for deck in self.lock["decks"]],
            ["Emmara, Soul of the Accord", "Gisa and Geralf"],
        )
        self.assertTrue(
            all(
                deck["source_snapshot"]["source_url"]
                == "https://magic.wizards.com/en/news/announcements/starter-commander-decks-decklists-2022-10-20"
                for deck in self.lock["decks"]
            )
        )
        self.assertEqual(self.lock["source_package"]["sha256"], EXPECTED_SOURCE_PACKAGE_SHA256)
        self.assertFalse(self.lock["authoritative_ranking_available"])
        self.assertEqual(self.lock["c_pass"], "BLOCKED")
        self.assertEqual(self.lock["m2_5_final"], "NOT_YET")
        self.assertEqual(self.lock["m3"], "NOT_AUTHORIZED")

    def test_each_deck_has_100_cards_no_sideboard_and_recomputed_digest(self) -> None:
        for deck in self.lock["decks"]:
            self.assertEqual(sum(card["quantity"] for card in deck["cards"]), 100)
            self.assertEqual(deck["sideboard"], [])
            self.assertEqual(deck["content_sha256"], compute_deck_content_sha256(deck))

    def test_commander_color_identity_must_match_source_row(self) -> None:
        mutated = copy.deepcopy(self.lock)
        mutated["decks"][0]["commander"]["color_identity"] = ["G", "R", "W"]
        mutated["decks"][0]["content_sha256"] = compute_deck_content_sha256(mutated["decks"][0])

        with self.assertRaisesRegex(
            ScopeLockValidationError, "commander color identity differs from its row"
        ):
            validate_lock_document(mutated)

    def test_pinned_archive_binding_when_configured(self) -> None:
        configured_root = os.environ.get("MANAFOLD_SOURCE_ARCHIVE")
        if not configured_root:
            self.skipTest("MANAFOLD_SOURCE_ARCHIVE is not configured")

        verify_pinned_archive(self.lock, Path(configured_root))

    def test_third_deck_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.lock)
        mutated["decks"].append(copy.deepcopy(mutated["decks"][0]))

        with self.assertRaisesRegex(ScopeLockValidationError, "exactly two decks"):
            validate_lock_document(mutated)

    def test_duplicate_source_row_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.lock)
        deck = mutated["decks"][0]
        deck["cards"].append(copy.deepcopy(deck["cards"][0]))

        with self.assertRaisesRegex(ScopeLockValidationError, "duplicate source row"):
            validate_lock_document(mutated)

    def test_missing_source_row_is_rejected(self) -> None:
        mutated = copy.deepcopy(self.lock)
        mutated["decks"][1]["cards"].pop()

        with self.assertRaisesRegex(ScopeLockValidationError, "quantity total"):
            validate_lock_document(mutated)


if __name__ == "__main__":
    unittest.main()
