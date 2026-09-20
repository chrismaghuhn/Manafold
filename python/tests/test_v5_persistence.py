"""Task 11 contract tests: Python mirror of V5 checkpoint digest (spec §9, §19.3).

The KAT vectors live in the shared fixture
``persistence/golden/checkpoint-digest-v5-kat.v1.json`` — the single source
of truth read byte-identically by the Rust and Python suites. This suite also
proves the plan-mandated negative evidence: malformed semantic contract IDs
reject inside the mirror function, and the codec identity pair is fail-closed.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.persistence import (  # noqa: E402
    calculate_checkpoint_digest_v5,
)
from mtgml.episode import EpisodeStatus  # noqa: E402

KAT_PATH = ROOT / "persistence" / "golden" / "checkpoint-digest-v5-kat.v1.json"

DEFAULT_COUNTERS = {
    "decisions_submitted": 0,
    "accepted_transitions": 0,
    "rule_events_emitted": 0,
    "resource_units_consumed": 0,
    "wall_clock_elapsed_millis": 0,
}

CODEC_ID_V5 = "in-memory-reference"
CODEC_VERSION_V5 = "5"


def load_kat_vectors() -> list[dict[str, object]]:
    with KAT_PATH.open(encoding="utf-8") as handle:
        document = json.load(handle)
    return document["vectors"]


class V5CheckpointDigestKatTests(unittest.TestCase):
    def test_shared_kat_vectors_reproduce_frozen_digests(self) -> None:
        vectors = load_kat_vectors()
        self.assertGreaterEqual(len(vectors), 3)
        for vector in vectors:
            case = str(vector["case"])
            digest = calculate_checkpoint_digest_v5(
                full_state_digest=str(vector["full_state_digest"]),
                status=EpisodeStatus.running(),
                counters=DEFAULT_COUNTERS,
                codec_id=CODEC_ID_V5,
                semantic_version=CODEC_VERSION_V5,
                program_kind=str(vector["program_kind"]),
                semantic_contract_id=str(vector["semantic_contract_id"]),
            )
            self.assertEqual(
                digest,
                vector["expected_digest"],
                f"KAT case {case} drifted",
            )


class V5CheckpointDigestMutationTests(unittest.TestCase):
    def test_different_program_kind_changes_digest(self) -> None:
        base = calculate_checkpoint_digest_v5(
            full_state_digest="07" * 32,
            status=EpisodeStatus.running(),
            counters=DEFAULT_COUNTERS,
            codec_id=CODEC_ID_V5,
            semantic_version=CODEC_VERSION_V5,
            program_kind="synthetic_rules_compat",
            semantic_contract_id="11" * 32,
        )
        magic = calculate_checkpoint_digest_v5(
            full_state_digest="07" * 32,
            status=EpisodeStatus.running(),
            counters=DEFAULT_COUNTERS,
            codec_id=CODEC_ID_V5,
            semantic_version=CODEC_VERSION_V5,
            program_kind="magic_rules",
            semantic_contract_id="11" * 32,
        )
        self.assertNotEqual(base, magic)

    def test_different_semantic_contract_id_changes_digest(self) -> None:
        first = calculate_checkpoint_digest_v5(
            full_state_digest="07" * 32,
            status=EpisodeStatus.running(),
            counters=DEFAULT_COUNTERS,
            codec_id=CODEC_ID_V5,
            semantic_version=CODEC_VERSION_V5,
            program_kind="synthetic_rules_compat",
            semantic_contract_id="11" * 32,
        )
        second = calculate_checkpoint_digest_v5(
            full_state_digest="07" * 32,
            status=EpisodeStatus.running(),
            counters=DEFAULT_COUNTERS,
            codec_id=CODEC_ID_V5,
            semantic_version=CODEC_VERSION_V5,
            program_kind="synthetic_rules_compat",
            semantic_contract_id="22" * 32,
        )
        self.assertNotEqual(first, second)

    def test_different_full_state_changes_digest(self) -> None:
        seven = calculate_checkpoint_digest_v5(
            full_state_digest="07" * 32,
            status=EpisodeStatus.running(),
            counters=DEFAULT_COUNTERS,
            codec_id=CODEC_ID_V5,
            semantic_version=CODEC_VERSION_V5,
            program_kind="synthetic_rules_compat",
            semantic_contract_id="11" * 32,
        )
        nine = calculate_checkpoint_digest_v5(
            full_state_digest="09" * 32,
            status=EpisodeStatus.running(),
            counters=DEFAULT_COUNTERS,
            codec_id=CODEC_ID_V5,
            semantic_version=CODEC_VERSION_V5,
            program_kind="synthetic_rules_compat",
            semantic_contract_id="11" * 32,
        )
        self.assertNotEqual(seven, nine)


class V5CheckpointDigestNegativeTests(unittest.TestCase):
    def test_unknown_program_kind_rejects(self) -> None:
        from mtgml.persistence import PersistenceError

        with self.assertRaises(PersistenceError):
            calculate_checkpoint_digest_v5(
                full_state_digest="07" * 32,
                status=EpisodeStatus.running(),
                counters=DEFAULT_COUNTERS,
                codec_id=CODEC_ID_V5,
                semantic_version=CODEC_VERSION_V5,
                program_kind="unknown_program",
                semantic_contract_id="11" * 32,
            )

    def test_malformed_semantic_contract_id_rejects(self) -> None:
        from mtgml.persistence import PersistenceError

        for bad in ("too_short", "5a" * 31, "5A" * 32, "zz" * 32):
            with self.subTest(bad=bad):
                with self.assertRaises((PersistenceError, ValueError)):
                    calculate_checkpoint_digest_v5(
                        full_state_digest="07" * 32,
                        status=EpisodeStatus.running(),
                        counters=DEFAULT_COUNTERS,
                        codec_id=CODEC_ID_V5,
                        semantic_version=CODEC_VERSION_V5,
                        program_kind="synthetic_rules_compat",
                        semantic_contract_id=bad,
                    )

    def test_codec_semantic_version_4_rejected(self) -> None:
        from mtgml.persistence import PersistenceError

        with self.assertRaises(PersistenceError):
            calculate_checkpoint_digest_v5(
                full_state_digest="07" * 32,
                status=EpisodeStatus.running(),
                counters=DEFAULT_COUNTERS,
                codec_id=CODEC_ID_V5,
                semantic_version="4",
                program_kind="synthetic_rules_compat",
                semantic_contract_id="11" * 32,
            )

    def test_wrong_codec_pair_rejected(self) -> None:
        from mtgml.persistence import PersistenceError

        with self.assertRaises(PersistenceError):
            calculate_checkpoint_digest_v5(
                full_state_digest="07" * 32,
                status=EpisodeStatus.running(),
                counters=DEFAULT_COUNTERS,
                codec_id="some-other-codec",
                semantic_version=CODEC_VERSION_V5,
                program_kind="synthetic_rules_compat",
                semantic_contract_id="11" * 32,
            )


if __name__ == "__main__":
    unittest.main()
