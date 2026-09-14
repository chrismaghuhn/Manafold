from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.decision import (
    PLAYER_DECISION_REQUEST_V2_SCHEMA,
    CandidateIntent,
    DecisionSpec,
    PlayerDecisionRequestV2,
    VisibleCandidateV2,
    _validate_candidate_capacity,
)
from mtgml.episode import EpisodeStatus, TruncationReason
from mtgml.errors import WireError
from mtgml.observation import (
    INFORMATION_STATE_SCHEMA_V2,
    OBSERVATION_SCHEMA,
    PLAYER_STEP_SCHEMA_V2,
    ObservationEnvelope,
    ObservedEventEnvelopeV2,
    PlayerInformationStateV2,
    PlayerStepSubmissionV1,
    PlayerStepV2,
)
from mtgml.wire import compute_information_state_digest_v2


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


class ActorBoundRejectionTests(unittest.TestCase):
    @staticmethod
    def _information_state() -> PlayerInformationStateV2:
        observation = ObservationEnvelope(
            schema_version=OBSERVATION_SCHEMA,
            perspective=1,
            state_revision=0,
            payload_codec="synthetic-m2-observation.v1",
            payload_base64="e30=",
            digest="90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3",
        )
        state = PlayerInformationStateV2(
            schema_version=INFORMATION_STATE_SCHEMA_V2,
            perspective=1,
            state_revision=0,
            current_observation=observation,
            next_visible_sequence=0,
            retained_knowledge=(),
            digest="0" * 64,
        )
        _, digest = compute_information_state_digest_v2(state.digest_input())
        return PlayerInformationStateV2(
            schema_version=state.schema_version,
            perspective=state.perspective,
            state_revision=state.state_revision,
            current_observation=state.current_observation,
            next_visible_sequence=state.next_visible_sequence,
            retained_knowledge=state.retained_knowledge,
            digest=digest,
        )

    @staticmethod
    def _request() -> PlayerDecisionRequestV2:
        return PlayerDecisionRequestV2(
            schema_version=PLAYER_DECISION_REQUEST_V2_SCHEMA,
            player_decision_id=1,
            state_revision=0,
            actor=1,
            visibility="public",
            decision=DecisionSpec("choose_one"),
            candidates=(
                VisibleCandidateV2(0, CandidateIntent("choose_boolean", (("value", False),))),
                VisibleCandidateV2(1, CandidateIntent("choose_boolean", (("value", True),))),
            ),
        )

    def _rejected(
        self,
        code: str,
        next_decision: PlayerDecisionRequestV2 | None,
        status: EpisodeStatus,
    ) -> PlayerStepV2:
        return PlayerStepV2(
            schema_version=PLAYER_STEP_SCHEMA_V2,
            information_state=self._information_state(),
            observed_events=(),
            next_decision=next_decision,
            status=status,
            submission=PlayerStepSubmissionV1("rejected", code),
        )

    def test_rejection_matrix_requires_actor_bound_decision_presence(self) -> None:
        request = self._request()
        for code in (
            "stale_decision",
            "invalid_answer",
            "invalid_candidate",
            "duplicate_assignment",
            "invalid_cardinality",
            "invalid_number",
            "invalid_order",
        ):
            with self.subTest(code=code):
                with self.assertRaises(WireError) as caught:
                    self._rejected(code, None, EpisodeStatus.running()).validate()
                self.assertEqual(caught.exception.code, "semantic.player_step")
                self._rejected(code, request, EpisodeStatus.running()).validate()

        with self.assertRaises(WireError):
            self._rejected("unavailable_decision", request, EpisodeStatus.running()).validate()
        self._rejected("unavailable_decision", None, EpisodeStatus.running()).validate()
        self._rejected(
            "episode_closed",
            None,
            EpisodeStatus("truncated", TruncationReason.EXTERNAL_STOP),
        ).validate()


if __name__ == "__main__":
    unittest.main()
