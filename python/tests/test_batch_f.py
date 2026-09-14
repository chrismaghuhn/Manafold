from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.decision import (  # noqa: E402
    PLAYER_DECISION_REQUEST_V2_SCHEMA,
    CandidateIntent,
    DecisionSpec,
    PlayerDecisionRequestV2,
    VisibleCandidateV2,
    _validate_candidate_capacity,
)
from mtgml.errors import WireError  # noqa: E402
from mtgml.observation import ObservedEventEnvelopeV2  # noqa: E402


class CandidateCapacityTests(unittest.TestCase):
    def test_candidate_capacity_uses_the_full_u32_domain_without_allocation(self) -> None:
        _validate_candidate_capacity(2**32)
        with self.assertRaises(WireError) as caught:
            _validate_candidate_capacity(2**32 + 1)
        self.assertEqual(caught.exception.code, "semantic.decision")

    def test_small_dense_request_remains_valid(self) -> None:
        request = PlayerDecisionRequestV2(
            schema_version=PLAYER_DECISION_REQUEST_V2_SCHEMA,
            player_decision_id=1,
            state_revision=0,
            actor=1,
            visibility="public",
            decision=DecisionSpec("choose_one"),
            candidates=(
                VisibleCandidateV2(0, CandidateIntent("pass_priority")),
                VisibleCandidateV2(1, CandidateIntent("confirm")),
            ),
        )
        request.validate()


class ObservedEventIdentityTests(unittest.TestCase):
    def test_object_moved_requires_one_visible_identity(self) -> None:
        def envelope(old_object: object, new_object: object) -> dict[str, object]:
            return {
                "schema_version": "observed-event-envelope.v2",
                "sequence": "1",
                "state_revision": "0",
                "event": {
                    "kind": "object_moved",
                    "old_object": old_object,
                    "new_object": new_object,
                    "from": "hand",
                    "to": "battlefield",
                },
            }

        with self.assertRaises(WireError) as caught:
            ObservedEventEnvelopeV2.from_wire(envelope(None, None))
        self.assertEqual(caught.exception.code, "semantic.observed_event")
        ObservedEventEnvelopeV2.from_wire(envelope("3", None))
        ObservedEventEnvelopeV2.from_wire(envelope(None, "11"))
        ObservedEventEnvelopeV2.from_wire(envelope("3", "11"))


if __name__ == "__main__":
    unittest.main()
