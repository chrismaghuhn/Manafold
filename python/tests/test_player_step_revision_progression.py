from __future__ import annotations

import dataclasses
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from mtgml.episode import EpisodeStatus
from mtgml.errors import WireError
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
    PlayerStepSubmissionV1,
    PlayerStepV2,
)
from mtgml.wire import compute_information_state_digest_v2


class PlayerStepRevisionProgressionTests(unittest.TestCase):
    def test_atomic_response_keeps_earlier_rules_product_event_revisions(self) -> None:
        observation = ObservationEnvelope(
            schema_version=OBSERVATION_SCHEMA,
            perspective=1,
            state_revision=2,
            payload_codec="synthetic-m2-observation.v1",
            payload_base64="e30=",
            digest="90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3",
        )
        digest_input = InformationStateDigestInputV2(
            schema_version="information-state-digest-input.v2",
            perspective=1,
            state_revision=2,
            current_observation=observation,
            next_visible_sequence=3,
            retained_knowledge=(),
        )
        _, digest = compute_information_state_digest_v2(digest_input)
        information_state = PlayerInformationStateV2(
            schema_version=INFORMATION_STATE_SCHEMA_V2,
            perspective=1,
            state_revision=2,
            current_observation=observation,
            next_visible_sequence=3,
            retained_knowledge=(),
            digest=digest,
        )
        event_one = ObservedEventEnvelopeV2(
            schema_version=OBSERVED_EVENT_SCHEMA_V2,
            sequence=1,
            state_revision=1,
            event=ObservedEventV2("life_changed", (("from", 20), ("player", 1), ("to", 19))),
        )
        event_two = ObservedEventEnvelopeV2(
            schema_version=OBSERVED_EVENT_SCHEMA_V2,
            sequence=2,
            state_revision=2,
            event=ObservedEventV2("life_changed", (("from", 19), ("player", 1), ("to", 18))),
        )
        step = PlayerStepV2(
            schema_version=PLAYER_STEP_SCHEMA_V2,
            information_state=information_state,
            observed_events=(event_one, event_two),
            next_decision=None,
            status=EpisodeStatus("running"),
            submission=PlayerStepSubmissionV1("accepted"),
        )
        step.validate()

        future_event = dataclasses.replace(event_two, state_revision=3)
        with self.assertRaisesRegex(WireError, "future revision"):
            dataclasses.replace(step, observed_events=(event_one, future_event)).validate()

        nonmonotonic_event = dataclasses.replace(event_two, state_revision=0)
        with self.assertRaisesRegex(WireError, "future revision"):
            dataclasses.replace(step, observed_events=(event_one, nonmonotonic_event)).validate()
