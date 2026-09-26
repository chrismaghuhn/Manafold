from __future__ import annotations

import json
import unittest
from pathlib import Path

from mtgml.canonical import canonical_json_bytes
from mtgml.decision import DecisionAnswerV2, DecisionResponseV2
from mtgml.decision_v3 import PlayerDecisionRequestV3
from mtgml.errors import WireError
from mtgml.wire import decode_canonical

ROOT = Path(__file__).resolve().parents[2]


def fixture() -> dict[str, object]:
    return json.loads(
        (ROOT / "schemas/examples/player-decision-request-v3-ordering.json").read_text()
    )


class DecisionV3Tests(unittest.TestCase):
    def test_phase_two_ordering_vector_decodes_and_round_trips(self) -> None:
        raw = fixture()
        request = PlayerDecisionRequestV3.from_wire(raw)
        emitted = request.to_wire()
        self.assertEqual(
            [candidate["intent"]["kind"] for candidate in emitted["candidates"]],
            ["pass_priority", "play_land", "cast_spell", "activate_ability"],
        )
        self.assertEqual(
            [candidate["candidate_id"] for candidate in emitted["candidates"]],
            [0, 1, 2, 3],
        )
        self.assertNotIn("trusted_binding", emitted["candidates"][1])
        decoded = decode_canonical("player-decision-request.v3", canonical_json_bytes(emitted))
        self.assertEqual(decoded, request)

    def test_v2_response_selects_v3_candidate_without_wire_change(self) -> None:
        raw = fixture()
        raw["candidates"] = [{"candidate_id": 0, "intent": {"kind": "play_land", "object": "3"}}]
        request = PlayerDecisionRequestV3.from_wire(raw)
        response = DecisionResponseV2(
            "decision-response.v2",
            request.player_decision_id,
            request.state_revision,
            DecisionAnswerV2("select_one", candidate_id=0),
        )
        request.validate_response(response)
        self.assertEqual(response.to_wire()["schema_version"], "decision-response.v2")

    def test_choose_many_accepts_v2_select_many_for_v3_request(self) -> None:
        raw = fixture()
        raw["decision"] = {"kind": "choose_many", "minimum": 1, "maximum": 3}
        request = PlayerDecisionRequestV3.from_wire(raw)
        response = DecisionResponseV2(
            "decision-response.v2",
            request.player_decision_id,
            request.state_revision,
            DecisionAnswerV2("select_many", candidate_ids=(0, 2)),
        )

        request.validate_response(response)

        self.assertEqual(response.to_wire()["answer"]["kind"], "select_many")

    def test_choose_many_and_order_reject_the_other_response_kind(self) -> None:
        raw = fixture()
        raw["decision"] = {"kind": "choose_many", "minimum": 1, "maximum": 3}
        choose_many = PlayerDecisionRequestV3.from_wire(raw)
        order_answer = DecisionResponseV2(
            "decision-response.v2",
            choose_many.player_decision_id,
            choose_many.state_revision,
            DecisionAnswerV2("order", candidate_ids=(0, 2)),
        )
        with self.assertRaisesRegex(WireError, "response domain mismatch"):
            choose_many.validate_response(order_answer)

        raw = fixture()
        raw["decision"] = {"kind": "order", "minimum": 1, "maximum": 3}
        order = PlayerDecisionRequestV3.from_wire(raw)
        select_many_answer = DecisionResponseV2(
            "decision-response.v2",
            order.player_decision_id,
            order.state_revision,
            DecisionAnswerV2("select_many", candidate_ids=(0, 2)),
        )
        with self.assertRaisesRegex(WireError, "response domain mismatch"):
            order.validate_response(select_many_answer)

    def test_wrong_candidate_order_and_duplicate_public_key_reject(self) -> None:
        raw = fixture()
        raw["candidates"][1], raw["candidates"][2] = raw["candidates"][2], raw["candidates"][1]
        with self.assertRaises(WireError):
            PlayerDecisionRequestV3.from_wire(raw)

        raw = fixture()
        raw["candidates"].insert(
            2,
            {"candidate_id": 2, "intent": {"kind": "play_land", "object": "4"}},
        )
        raw["candidates"][3]["candidate_id"] = 3
        raw["candidates"][4]["candidate_id"] = 4
        with self.assertRaises(WireError):
            PlayerDecisionRequestV3.from_wire(raw)

    def test_wire_rejects_trusted_binding_and_wrong_version(self) -> None:
        raw = fixture()
        raw["candidates"][1]["trusted_binding"] = {"kind": "play_land", "object": "10"}
        with self.assertRaises(WireError):
            PlayerDecisionRequestV3.from_wire(raw)

    def test_phase_two_semantic_negative_fixtures_are_rejected(self) -> None:
        negatives = json.loads(
            (ROOT / "schemas/negative/m4-phase2-decision-v3-semantic-negatives.json").read_text()
        )
        self.assertEqual(len(negatives["cases"]), 3)
        for case in negatives["cases"]:
            with self.subTest(case=case["case"]), self.assertRaises(WireError):
                PlayerDecisionRequestV3.from_wire(case["request"])
        raw = fixture()
        raw["schema_version"] = "player-decision-request.v2"
        with self.assertRaises(WireError):
            PlayerDecisionRequestV3.from_wire(raw)


if __name__ == "__main__":
    unittest.main()
