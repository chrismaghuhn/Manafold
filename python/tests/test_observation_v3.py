from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from mtgml._events_v3 import ObservedEventEnvelopeV3
from mtgml._magic_basic_land_observation_v1 import MagicBasicLandObservationV1
from mtgml._player_step_v3 import PlayerStepV3
from mtgml.canonical import canonical_json_bytes
from mtgml.errors import WireError
from mtgml.wire import decode_canonical

ROOT = Path(__file__).resolve().parents[2]


def fixture(path: str) -> dict[str, object]:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


class M4Phase6ObservationTests(unittest.TestCase):
    def test_v3_event_examples_round_trip_as_exact_named_contracts(self) -> None:
        for name in (
            "observed-event-v3-entry-back-tapped.json",
            "observed-event-v3-mana-pool-changed.json",
            "observed-event-v3-counters-changed.json",
            "observed-event-v3-attachment-changed.json",
            "observed-event-v3-face-changed.json",
            "observed-event-v3-object-moved.json",
        ):
            value = fixture(f"schemas/examples/{name}")
            wire = canonical_json_bytes(ObservedEventEnvelopeV3.from_wire(value).to_wire())
            self.assertEqual(
                decode_canonical("observed-event-envelope.v3", wire).to_wire(), json.loads(wire)
            )

    def test_object_moved_requires_all_six_fields_and_one_opaque_identity(self) -> None:
        value = fixture("schemas/examples/observed-event-v3-object-moved.json")
        for field in ("old_object", "new_object", "from", "to", "entering_face", "tapped"):
            malformed = copy.deepcopy(value)
            del malformed["event"][field]
            with self.assertRaises(WireError):
                ObservedEventEnvelopeV3.from_wire(malformed)
        value["event"]["old_object"] = None
        value["event"]["new_object"] = None
        with self.assertRaises(WireError):
            ObservedEventEnvelopeV3.from_wire(value)

    def test_closed_observation_payload_rejects_candidate_authority(self) -> None:
        value = fixture("schemas/examples/magic-basic-land-observation-v1-ordered.json")
        parsed = MagicBasicLandObservationV1.from_wire(value)
        self.assertEqual(parsed.to_wire(), value)
        value["candidates"] = []
        with self.assertRaises(WireError):
            MagicBasicLandObservationV1.from_wire(value)
        with self.assertRaises(WireError):
            decode_canonical("magic-basic-land-observation.v1", canonical_json_bytes(value))

    def test_payload_canonical_order_and_ranges_are_closed(self) -> None:
        value = fixture("schemas/examples/magic-basic-land-observation-v1-ordered.json")
        bad = copy.deepcopy(value)
        bad["mana_pools"].append(copy.deepcopy(bad["mana_pools"][0]))
        with self.assertRaises(WireError):
            MagicBasicLandObservationV1.from_wire(bad)
        bad = copy.deepcopy(value)
        bad["mana_pools"][0]["unrestricted"][0] = True
        with self.assertRaises(WireError):
            MagicBasicLandObservationV1.from_wire(bad)

    def test_player_step_v3_keeps_decision_outside_observation_and_rejection_empty(self) -> None:
        accepted = fixture("schemas/examples/player-step-v3-event-next-decision.json")
        accepted["next_decision"]["state_revision"] = "0"
        accepted["observed_events"][0]["sequence"] = "4"
        accepted["observed_events"][0]["state_revision"] = "0"
        step = PlayerStepV3.from_wire(accepted)
        self.assertEqual(step.next_decision.actor, 1)
        self.assertEqual(len(step.next_decision.candidates), 2)
        observation_payload = fixture("schemas/examples/magic-basic-land-observation-v1.json")
        self.assertNotIn("candidates", observation_payload)

        rejected = fixture("schemas/examples/player-step-v3-rejected-no-events.json")
        rejected["next_decision"] = accepted["next_decision"]
        self.assertEqual(PlayerStepV3.from_wire(rejected).observed_events, ())
        rejected["observed_events"] = accepted["observed_events"]
        with self.assertRaises(WireError):
            PlayerStepV3.from_wire(rejected)

    def test_player_step_v3_accepts_omitted_optional_next_decision(self) -> None:
        value = fixture("schemas/examples/player-step-v3-no-next-decision.json")
        step = PlayerStepV3.from_wire(value)
        self.assertIsNone(step.next_decision)


if __name__ == "__main__":
    unittest.main()
