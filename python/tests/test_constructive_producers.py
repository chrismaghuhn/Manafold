from __future__ import annotations

import sys
import unittest
from pathlib import Path
from typing import ClassVar

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
from mtgml.observation import (
    INFORMATION_STATE_SCHEMA_V2,
    OBSERVATION_SCHEMA,
    OBSERVED_EVENT_SCHEMA_V2,
    PLAYER_STEP_SCHEMA_V2,
    InformationStateDigestInputV2,
    ObservationEnvelope,
    ObservedEventEnvelopeV2,
    ObservedEventV2,
    PlayerInformationStateV2,
    PlayerKnowledgeInvalidationV1,
    PlayerKnowledgeProvenanceV1,
    PlayerKnownLocationFactV1,
    PlayerKnownLocationV1,
    PlayerKnownObjectV1,
    PlayerStepSubmissionV1,
    PlayerStepV2,
)
from mtgml.wire import compute_information_state_digest_v2, encode_canonical

GOLDEN = ROOT / "wire" / "golden"
ZERO_DIGEST = "0" * 64


def golden_bytes(name: str) -> bytes:
    return (GOLDEN / name).read_bytes()


def observed(channel: str, sequence: int, cause: str) -> PlayerKnowledgeProvenanceV1:
    return PlayerKnowledgeProvenanceV1("observed", channel, sequence, cause)


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


class ConstructiveObservedEventEnvelopeV2Tests(unittest.TestCase):
    """All seven closed event kinds are anchored to their checked-in kind
    goldens (object_tapped rides on observed-event-envelope.v2.json)."""

    CASES: ClassVar[list[tuple[str, int, str, tuple[tuple[str, object], ...]]]] = [
        ("observed-event-envelope.v2.json", 0, "object_tapped", (("object", 7), ("tapped", True))),
        (
            "observed-event-v2-object-moved.json",
            1,
            "object_moved",
            (("from", "hand"), ("new_object", 11), ("old_object", 3), ("to", "battlefield")),
        ),
        ("observed-event-v2-object-ceased.json", 2, "object_ceased_to_exist", (("object", 13),)),
        (
            "observed-event-v2-life-changed.json",
            3,
            "life_changed",
            (("from", 20), ("player", 2), ("to", 17)),
        ),
        ("observed-event-v2-decision-available.json", 4, "decision_available", (("actor", 2),)),
        (
            "observed-event-v2-random-outcome.json",
            5,
            "random_outcome_visible",
            (("exclusive_upper_bound", 6), ("label", "coin_flip"), ("value", 3)),
        ),
        (
            "observed-event-v2-public-outcome.json",
            6,
            "public_outcome",
            (("code", "mulligan_complete"),),
        ),
    ]

    def test_every_kind_matches_its_golden_bytes(self) -> None:
        for name, sequence, kind, payload in self.CASES:
            with self.subTest(fixture=name):
                envelope = ObservedEventEnvelopeV2(
                    OBSERVED_EVENT_SCHEMA_V2,
                    sequence,
                    0,
                    ObservedEventV2(kind, payload),
                )
                self.assertEqual(encode_canonical(envelope), golden_bytes(name))


def choose_one_request() -> PlayerDecisionRequestV2:
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


def information_state_v2() -> PlayerInformationStateV2:
    observation = ObservationEnvelope(
        schema_version=OBSERVATION_SCHEMA,
        perspective=1,
        state_revision=0,
        payload_codec="synthetic-m2-observation.v1",
        payload_base64="e30=",
        digest=ZERO_DIGEST,
    )
    retained_knowledge = (
        PlayerKnownObjectV1(
            kind="active",
            opaque_object_id=3,
            known_definition=42,
            current_known_location_fact=PlayerKnownLocationFactV1(
                location=PlayerKnownLocationV1("exile", 2),
                provenance=observed("public", 4, "explicit_reveal"),
            ),
            historical_locations=(
                PlayerKnownLocationFactV1(
                    location=PlayerKnownLocationV1("hand", None),
                    provenance=observed("private", 3, "own_private_identity"),
                ),
            ),
            acquisition=observed("private", 1, "private_look"),
        ),
        PlayerKnownObjectV1(
            kind="retired",
            opaque_object_id=7,
            known_definition=None,
            last_known_location_fact=PlayerKnownLocationFactV1(
                location=PlayerKnownLocationV1("battlefield", None),
                provenance=PlayerKnowledgeProvenanceV1("initial_configuration"),
            ),
            historical_locations=(),
            acquisition=observed("public", 2, "public_event"),
            invalidation=PlayerKnowledgeInvalidationV1(
                provenance=observed("public", 4, "explicit_reveal"),
                reason="shuffle",
            ),
        ),
    )
    input_value = InformationStateDigestInputV2(
        schema_version="information-state-digest-input.v2",
        perspective=1,
        state_revision=0,
        current_observation=observation,
        next_visible_sequence=5,
        retained_knowledge=retained_knowledge,
    )
    _, digest = compute_information_state_digest_v2(input_value)
    return PlayerInformationStateV2(
        schema_version=INFORMATION_STATE_SCHEMA_V2,
        perspective=1,
        state_revision=0,
        current_observation=observation,
        next_visible_sequence=5,
        retained_knowledge=retained_knowledge,
        digest=digest,
    )


class ConstructivePlayerStepV2Tests(unittest.TestCase):
    def test_running_step_with_next_decision_matches_golden_bytes(self) -> None:
        step = PlayerStepV2(
            schema_version=PLAYER_STEP_SCHEMA_V2,
            information_state=information_state_v2(),
            observed_events=(),
            next_decision=choose_one_request(),
            status=EpisodeStatus("running"),
            submission=PlayerStepSubmissionV1("accepted"),
        )
        self.assertEqual(encode_canonical(step), golden_bytes("player-step.v2.json"))

    def test_terminal_events_step_matches_golden_bytes(self) -> None:
        events = (
            ObservedEventEnvelopeV2(
                OBSERVED_EVENT_SCHEMA_V2,
                0,
                0,
                ObservedEventV2("life_changed", (("from", 20), ("player", 2), ("to", 17))),
            ),
            ObservedEventEnvelopeV2(
                OBSERVED_EVENT_SCHEMA_V2,
                2,
                0,
                ObservedEventV2("public_outcome", (("code", "draw_step"),)),
            ),
        )
        step = PlayerStepV2(
            schema_version=PLAYER_STEP_SCHEMA_V2,
            information_state=information_state_v2(),
            observed_events=events,
            next_decision=None,
            status=EpisodeStatus(
                "terminal",
                TerminalReason.CONCESSION,
                (PlayerOutcome(1, PlayerResult.WIN), PlayerOutcome(2, PlayerResult.LOSS)),
            ),
            submission=PlayerStepSubmissionV1("accepted"),
        )
        self.assertEqual(
            encode_canonical(step), golden_bytes("player-step.v2-terminal-events.json")
        )

    def test_episode_closed_rejected_step_matches_golden_bytes(self) -> None:
        step = PlayerStepV2(
            schema_version=PLAYER_STEP_SCHEMA_V2,
            information_state=information_state_v2(),
            observed_events=(),
            next_decision=None,
            status=EpisodeStatus(
                "truncated",
                TruncationReason.EXTERNAL_STOP,
                (PlayerOutcome(1, PlayerResult.UNRESOLVED),),
            ),
            submission=PlayerStepSubmissionV1("rejected", "episode_closed"),
        )
        self.assertEqual(
            encode_canonical(step),
            golden_bytes("player-step.v2-episode-closed-rejected.json"),
        )


if __name__ == "__main__":
    unittest.main()
