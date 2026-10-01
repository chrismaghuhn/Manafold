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

    def test_preserves_attacker_declaration_and_synthetic_assembly_domains(self) -> None:
        names = (
            "attacker-declaration",
            "synthetic-entry",
            "synthetic-count",
            "synthetic-members",
            "synthetic-order",
        )
        for name in names:
            with self.subTest(name=name):
                raw = json.loads(
                    (
                        ROOT / "schemas/examples" / f"player-decision-request-v4-{name}.json"
                    ).read_text(encoding="utf-8")
                )
                self.assertEqual(PlayerDecisionRequestV4.from_wire(raw).to_wire(), raw)

    def test_hand_size_discard_requires_exact_choose_many_over_objects(self) -> None:
        value = json.loads(
            (ROOT / "schemas/examples/player-decision-request-v4-hand-size-discard.json").read_text(
                encoding="utf-8"
            )
        )
        request = PlayerDecisionRequestV4.from_wire(value)
        self.assertEqual(request.purpose.kind, "hand_size_discard")
        for edit in (
            lambda item: item["decision_domain_v2"].update(minimum=1, maximum=2),
            lambda item: item["decision_domain_v2"].update(minimum=0, maximum=0),
            lambda item: item.update(visibility="public"),
            lambda item: item["candidates"][0].update(intent={"kind": "pass_priority"}),
        ):
            edited = copy.deepcopy(value)
            edit(edited)
            with self.assertRaises(WireError):
                PlayerDecisionRequestV4.from_wire(edited)

    def test_game_start_purposes_require_their_domain_intent_and_visibility(self) -> None:
        for name, kind, other_visibility in (
            ("starting-player", "starting_player", "acting_player_only"),
            ("mulligan-declaration", "mulligan_declaration", "acting_player_only"),
            ("mulligan-bottom", "mulligan_bottom", "public"),
        ):
            path = ROOT / f"schemas/examples/player-decision-request-v4-{name}.json"
            value = json.loads(path.read_text(encoding="utf-8"))
            request = PlayerDecisionRequestV4.from_wire(value)
            self.assertEqual(request.purpose.kind, kind)
            self.assertEqual(request.to_wire(), value)
            for edit in (
                lambda item, other=other_visibility: item.update(visibility=other),
                lambda item: item["candidates"][0].update(intent={"kind": "pass_priority"}),
                lambda item: item.update(
                    decision_domain_v2={"kind": "choose_many", "minimum": 1, "maximum": 1}
                ),
            ):
                edited = copy.deepcopy(value)
                edit(edited)
                with self.assertRaises(WireError, msg=kind):
                    PlayerDecisionRequestV4.from_wire(edited)
        path = ROOT / "schemas/examples/player-decision-request-v4-mulligan-bottom.json"
        bottom = json.loads(path.read_text(encoding="utf-8"))
        for minimum, maximum in ((0, 0), (1, 2)):
            edited = copy.deepcopy(bottom)
            edited["decision_domain_v2"].update(minimum=minimum, maximum=maximum)
            with self.assertRaises(WireError):
                PlayerDecisionRequestV4.from_wire(edited)

    def test_blocker_declaration_offers_each_attacker_and_no_block_to_its_actor_only(self) -> None:
        path = ROOT / "schemas/examples/player-decision-request-v4-blocker-declaration.json"
        value = json.loads(path.read_text(encoding="utf-8"))
        request = PlayerDecisionRequestV4.from_wire(value)
        self.assertEqual(request.purpose.kind, "blocker_declaration")
        self.assertEqual(request.to_wire(), value)
        # "No block" comes first, then each attacker in ascending order.
        self.assertEqual(
            [(item.intent.blocker_id, item.intent.attacker_id) for item in request.candidates],
            [(7, None), (7, 3), (7, 9)],
        )

        def swap_attackers(item: dict[str, object]) -> None:
            first, second = item["candidates"][1]["intent"], item["candidates"][2]["intent"]  # type: ignore[index]
            first["attacker"], second["attacker"] = second["attacker"], first["attacker"]

        for edit in (
            lambda item: item.update(visibility="public"),
            lambda item: item["candidates"][0].update(intent={"kind": "pass_priority"}),
            lambda item: item["candidates"][0]["intent"].pop("attacker"),
            lambda item: item["candidates"][0]["intent"].update(blocker=None),
            lambda item: item["candidates"][0]["intent"].update(attacker="03"),
            lambda item: item["candidates"][1]["intent"].update(attacker=None),
            lambda item: item.update(
                decision_domain_v2={"kind": "choose_many", "minimum": 1, "maximum": 1}
            ),
            swap_attackers,
        ):
            edited = copy.deepcopy(value)
            edit(edited)
            with self.assertRaises(WireError):
                PlayerDecisionRequestV4.from_wire(edited)

    def test_cost_route_request_must_be_actor_only(self) -> None:
        raw = json.loads(
            (ROOT / "schemas/examples/player-decision-request-v4-cost-route.json").read_text(
                encoding="utf-8"
            )
        )
        raw["visibility"] = "public"
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(raw)

    def test_sba_graveyard_order_uses_order_domain_and_safe_object_candidates(self) -> None:
        raw = json.loads(
            (
                ROOT / "schemas/examples/player-decision-request-v4-sba-graveyard-order.json"
            ).read_text(encoding="utf-8")
        )
        request = PlayerDecisionRequestV4.from_wire(raw)
        self.assertEqual(request.purpose.kind, "sba_graveyard_order")
        self.assertEqual(request.decision_domain_v2.kind, "order")
        self.assertEqual([item.intent.kind for item in request.candidates], ["select_object"] * 2)

    def test_sba_graveyard_order_rejects_public_visibility_and_wrong_candidate(self) -> None:
        raw = json.loads(
            (
                ROOT / "schemas/examples/player-decision-request-v4-sba-graveyard-order.json"
            ).read_text(encoding="utf-8")
        )
        public = copy.deepcopy(raw)
        public["visibility"] = "public"
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(public)

        wrong_candidate = copy.deepcopy(raw)
        wrong_candidate["candidates"][0]["intent"] = {"kind": "select_player", "player": "1"}
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(wrong_candidate)

    def test_rejects_candidate_intents_outside_attacker_and_synthetic_purposes(self) -> None:
        attacker = json.loads(
            (
                ROOT / "schemas/examples/player-decision-request-v4-attacker-declaration.json"
            ).read_text(encoding="utf-8")
        )
        attacker["candidates"][0]["intent"] = {"kind": "cast_spell", "object": "1"}
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(attacker)

        synthetic = json.loads(
            (ROOT / "schemas/examples/player-decision-request-v4-synthetic-entry.json").read_text(
                encoding="utf-8"
            )
        )
        synthetic["candidates"][0]["intent"] = {"kind": "cast_spell", "object": "1"}
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(synthetic)

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

    def test_trigger_order_requires_order_domain_and_actor_only_visibility(self) -> None:
        raw = request_fixture()
        wrong_domain = copy.deepcopy(raw)
        wrong_domain["decision_domain_v2"] = {"kind": "choose_one"}
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(wrong_domain)
        public = copy.deepcopy(raw)
        public["visibility"] = "public"
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(public)

    def test_each_purpose_rejects_a_domain_outside_its_closed_relation(self) -> None:
        cases = (
            ({"kind": "priority_action"}, "choose_many"),
            ({"kind": "attacker_declaration"}, "choose_one"),
            ({"kind": "cast_cost_route"}, "choose_many"),
            ({"kind": "mode_selection", "mode_slot": 0}, "choose_number"),
            ({"kind": "target_selection", "target_slot": 0}, "choose_number"),
            (
                {
                    "kind": "cost_operand_selection",
                    "cost_slot": 0,
                    "operation": "put_counters",
                    "counter_kind": "minus_one_minus_one",
                    "count": 2,
                },
                "choose_many",
            ),
            ({"kind": "mana_production_choice"}, "choose_many"),
            ({"kind": "mana_payment"}, "choose_many"),
            ({"kind": "optional_cost_payment", "profile_local_cost_id": 0}, "choose_many"),
            ({"kind": "ability_action"}, "choose_many"),
            ({"kind": "trigger_order"}, "choose_one"),
            ({"kind": "trigger_target", "target_slot": 0}, "choose_number"),
            (
                {"kind": "synthetic_assembly", "stage": "choose_count"},
                "choose_one",
            ),
        )
        for purpose, wrong_domain in cases:
            with self.subTest(purpose=purpose["kind"]):
                raw = {
                    "schema_version": "player-decision-request.v4",
                    "player_decision_id": "1",
                    "view_sequence": "0",
                    "actor": "0",
                    "visibility": "public",
                    "decision_domain_v2": {
                        "kind": wrong_domain,
                        "minimum": 0,
                        "maximum": 0,
                    },
                    "purpose": purpose,
                    "parent_player_decision_id": None,
                    "candidates": [],
                }
                with self.assertRaises(WireError):
                    PlayerDecisionRequestV4.from_wire(raw)

    def test_rejects_source_ability_without_visible_source_object(self) -> None:
        raw = request_fixture()
        unbound = copy.deepcopy(raw)
        descriptor = unbound["candidates"][0]["intent"]["trigger"]
        descriptor["source_object"] = None
        descriptor["source_ability"] = "99"
        with self.assertRaises(WireError):
            PlayerDecisionRequestV4.from_wire(unbound)


if __name__ == "__main__":
    unittest.main()
