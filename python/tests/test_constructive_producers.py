from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml._generated_contract_vocab import (
    PlayerResult,
    TerminalReason,
    TruncationReason,
)
from mtgml.decision import (
    DECISION_RESPONSE_V2_SCHEMA,
    PLAYER_DECISION_REQUEST_V2_SCHEMA,
    CandidateIntent,
    DecisionAnswerV2,
    DecisionResponseV2,
    DecisionSpec,
    PlayerDecisionRequestV2,
    VisibleCandidateV2,
)
from mtgml.episode import EpisodeStatus, PlayerOutcome
from mtgml.wire import encode_canonical

GOLDEN = ROOT / "wire" / "golden"
OBSERVATION_DIGEST = "90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3"


def golden_bytes(name: str) -> bytes:
    return (GOLDEN / name).read_bytes()


class ConstructiveDecisionResponseV2Tests(unittest.TestCase):
    """Each gate-owned decision-response.v2 golden is re-produced from typed
    dataclass constructors: adding, removing, or retyping a DTO field breaks
    this node instead of silently drifting away from the shared fixture."""

    def _response(self, player_decision_id: int, answer: DecisionAnswerV2) -> DecisionResponseV2:
        return DecisionResponseV2(
            schema_version=DECISION_RESPONSE_V2_SCHEMA,
            player_decision_id=player_decision_id,
            state_revision=0,
            answer=answer,
        )

    def test_select_one_matches_golden_bytes(self) -> None:
        response = self._response(1, DecisionAnswerV2(kind="select_one", candidate_id=1))
        self.assertEqual(
            encode_canonical(response), golden_bytes("decision-response.v2-select-one.json")
        )

    def test_select_many_matches_golden_bytes(self) -> None:
        response = self._response(1, DecisionAnswerV2(kind="select_many", candidate_ids=(0, 2)))
        self.assertEqual(
            encode_canonical(response), golden_bytes("decision-response.v2-select-many.json")
        )

    def test_order_matches_golden_bytes(self) -> None:
        response = self._response(2, DecisionAnswerV2(kind="order", candidate_ids=(2, 0)))
        self.assertEqual(
            encode_canonical(response), golden_bytes("decision-response.v2-order.json")
        )

    def test_choose_number_matches_golden_bytes(self) -> None:
        response = self._response(3, DecisionAnswerV2(kind="choose_number", value=-5))
        self.assertEqual(
            encode_canonical(response), golden_bytes("decision-response.v2-choose-number.json")
        )


class ConstructivePlayerDecisionRequestV2Tests(unittest.TestCase):
    def _request(
        self,
        player_decision_id: int,
        visibility: str,
        decision: DecisionSpec,
        candidates: tuple[VisibleCandidateV2, ...],
    ) -> PlayerDecisionRequestV2:
        return PlayerDecisionRequestV2(
            schema_version=PLAYER_DECISION_REQUEST_V2_SCHEMA,
            player_decision_id=player_decision_id,
            state_revision=0,
            actor=1,
            visibility=visibility,
            decision=decision,
            candidates=candidates,
        )

    def test_choose_one_matches_golden_bytes(self) -> None:
        request = self._request(
            1,
            "public",
            DecisionSpec("choose_one"),
            (
                VisibleCandidateV2(0, CandidateIntent("choose_boolean", (("value", False),))),
                VisibleCandidateV2(1, CandidateIntent("choose_boolean", (("value", True),))),
            ),
        )
        self.assertEqual(encode_canonical(request), golden_bytes("player-decision-request.v2.json"))

    def test_choose_many_matches_golden_bytes(self) -> None:
        request = self._request(
            5,
            "public",
            DecisionSpec("choose_many", 1, 2),
            (
                VisibleCandidateV2(0, CandidateIntent("pass_priority")),
                VisibleCandidateV2(1, CandidateIntent("cast_spell", (("object", 9),))),
            ),
        )
        self.assertEqual(
            encode_canonical(request), golden_bytes("player-decision-request.v2-choose-many.json")
        )

    def test_order_matches_golden_bytes(self) -> None:
        request = self._request(
            6,
            "acting_player_only",
            DecisionSpec("order", 1, 2),
            (
                VisibleCandidateV2(0, CandidateIntent("select_player", (("player", 4),))),
                VisibleCandidateV2(1, CandidateIntent("select_mode", (("mode_index", 3),))),
            ),
        )
        self.assertEqual(
            encode_canonical(request), golden_bytes("player-decision-request.v2-order.json")
        )

    def test_choose_number_matches_golden_bytes(self) -> None:
        request = self._request(4, "public", DecisionSpec("choose_number", 0, 10), ())
        self.assertEqual(
            encode_canonical(request), golden_bytes("player-decision-request.v2-choose-number.json")
        )


class ConstructiveEpisodeStatusTests(unittest.TestCase):
    def test_running_matches_golden_bytes(self) -> None:
        status = EpisodeStatus("running")
        self.assertEqual(encode_canonical(status), golden_bytes("episode-status.v1.json"))

    def test_terminal_concession_matches_golden_bytes(self) -> None:
        status = EpisodeStatus(
            "terminal",
            TerminalReason.CONCESSION,
            (PlayerOutcome(1, PlayerResult.WIN), PlayerOutcome(2, PlayerResult.LOSS)),
        )
        self.assertEqual(
            encode_canonical(status),
            golden_bytes("episode-status-terminal-concession.v1.json"),
        )

    def test_truncated_external_stop_matches_golden_bytes(self) -> None:
        status = EpisodeStatus(
            "truncated",
            TruncationReason.EXTERNAL_STOP,
            (PlayerOutcome(1, PlayerResult.UNRESOLVED),),
        )
        self.assertEqual(
            encode_canonical(status),
            golden_bytes("episode-status-truncated-external-stop.v1.json"),
        )


if __name__ == "__main__":
    unittest.main()
