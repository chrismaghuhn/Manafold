from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from mtgml.decision import DecisionAnswerV2, DecisionResponseV3
from mtgml.decision_v4 import PlayerDecisionRequestV4
from mtgml.errors import WireError
from mtgml.wire import decode_canonical, encode_canonical

ROOT = Path(__file__).resolve().parents[2]


def request_fixture() -> dict[str, object]:
    return json.loads(
        (ROOT / "schemas/examples/player-decision-request-v4-trigger-order.json").read_text(
            encoding="utf-8"
        )
    )


class PlayerDecisionRequestV4Tests(unittest.TestCase):
    def test_all_trigger_subjects_round_trip_in_exact_canonical_order(self) -> None:
        raw = request_fixture()
        request = PlayerDecisionRequestV4.from_wire(raw)
        self.assertEqual(len(request.candidates), 11)
        self.assertEqual(request.to_wire(), raw)
        self.assertNotIn("state_revision", request.to_wire())
        self.assertEqual(
            decode_canonical("player-decision-request.v4", encode_canonical(request)), request
        )

    def test_response_binds_only_request_and_view_identity(self) -> None:
        request = PlayerDecisionRequestV4.from_wire(request_fixture())
        response = DecisionResponseV3(
            "decision-response.v3",
            request.player_decision_id,
            request.view_sequence,
            DecisionAnswerV2("order", candidate_ids=tuple(range(11))),
        )
        request.validate_response(response)
        with self.assertRaises(WireError):
            request.validate_response(
                DecisionResponseV3(
                    "decision-response.v3",
                    request.player_decision_id,
                    request.view_sequence + 1,
                    response.answer,
                )
            )

    def test_rejects_event_subject_mismatch_and_noncanonical_attack_order(self) -> None:
        raw = request_fixture()
        mismatched = copy.deepcopy(raw)
        mismatched["candidates"][0]["intent"]["trigger"]["subject"]["kind"] = "card_drawn"
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(mismatched)

        unordered = copy.deepcopy(raw)
        unordered["candidates"][6]["intent"]["trigger"]["subject"]["attackers"].reverse()
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(unordered)

    def test_rejects_duplicate_safe_trigger_descriptors(self) -> None:
        raw = request_fixture()
        duplicate = copy.deepcopy(raw)
        duplicate["candidates"][1]["intent"] = copy.deepcopy(duplicate["candidates"][0]["intent"])
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(duplicate)

    def test_rejects_conflicting_opaque_ability_source_mapping(self) -> None:
        raw = request_fixture()
        conflicting = copy.deepcopy(raw)
        descriptor = conflicting["candidates"][1]["intent"]["trigger"]
        descriptor["source_ability"] = "3"
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(conflicting)


if __name__ == "__main__":
    unittest.main()
