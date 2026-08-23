from __future__ import annotations

import inspect
import json
import sys
import unittest
from pathlib import Path
from typing import get_type_hints

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.observation import PlayerInformationStateV2, PlayerKnownObjectV1, PlayerStepV2
from mtgml.player_client import PlayerClient


class PlayerApiTests(unittest.TestCase):
    def test_python_protocol_contains_the_full_rust_player_surface(self) -> None:
        self.assertEqual(
            {
                name
                for name, value in inspect.getmembers(PlayerClient, inspect.isfunction)
                if not name.startswith("_")
            },
            {"observation", "information_state", "visible_decision", "submit"},
        )
        hints = get_type_hints(PlayerClient.submit)
        self.assertIs(hints["return"], PlayerStepV2)

    def test_v2_public_boundary_excludes_privileged_fields(self) -> None:
        fields = set(PlayerStepV2.__dataclass_fields__)
        self.assertEqual(
            fields,
            {
                "schema_version",
                "information_state",
                "observed_events",
                "next_decision",
                "status",
                "submission",
            },
        )
        self.assertTrue(
            fields.isdisjoint({"root_seed", "checkpoint", "fork", "authoritative_events", "replay"})
        )


class PlayerStepSubmissionContractTests(unittest.TestCase):
    """The submission outcome is a closed variant union: invalid domain
    objects must never serialize as plausible wire data (WIRE_CONTRACT)."""

    GOLDEN = ROOT / "wire" / "golden" / "player-step-v2-rejected.json"

    def _golden_step(self) -> PlayerStepV2:
        from mtgml.wire import decode_canonical

        decoded = decode_canonical("player-step.v2", self.GOLDEN.read_bytes())
        assert isinstance(decoded, PlayerStepV2)
        return decoded

    def test_unknown_kind_is_never_serialized_as_plausible_wire_data(self) -> None:
        from mtgml.errors import WireError
        from mtgml.observation import PlayerStepSubmissionV1

        with self.assertRaises(WireError):
            PlayerStepSubmissionV1("garbage", None).to_wire()
        with self.assertRaises(WireError):
            PlayerStepSubmissionV1("garbage", "stale_decision").to_wire()

    def test_accepted_must_not_carry_a_code(self) -> None:
        from mtgml.errors import WireError
        from mtgml.observation import PlayerStepSubmissionV1

        with self.assertRaises(WireError):
            PlayerStepSubmissionV1.from_wire({"kind": "accepted", "code": "stale_decision"})
        with self.assertRaises(WireError):
            PlayerStepSubmissionV1("accepted", "stale_decision").to_wire()

    def test_rejected_requires_a_closed_code(self) -> None:
        from mtgml.errors import WireError
        from mtgml.observation import PlayerStepSubmissionV1

        with self.assertRaises(WireError):
            PlayerStepSubmissionV1.from_wire({"kind": "rejected"})
        with self.assertRaises(WireError):
            PlayerStepSubmissionV1("rejected", None).to_wire()
        with self.assertRaises(WireError):
            PlayerStepSubmissionV1("rejected", "not_a_code").to_wire()

    def test_valid_variants_round_trip_exactly(self) -> None:
        from mtgml.observation import PlayerStepSubmissionV1

        self.assertEqual(
            PlayerStepSubmissionV1.from_wire({"kind": "accepted"}).to_wire(),
            {"kind": "accepted"},
        )
        self.assertEqual(
            PlayerStepSubmissionV1.from_wire(
                {"kind": "rejected", "code": "stale_decision"}
            ).to_wire(),
            {"kind": "rejected", "code": "stale_decision"},
        )

    def test_step_level_rejection_invariants(self) -> None:
        import dataclasses

        from mtgml.episode import EpisodeStatus
        from mtgml.errors import WireError
        from mtgml.observation import ObservedEventEnvelopeV2, PlayerStepSubmissionV1

        step = self._golden_step()
        step.validate()

        event = ObservedEventEnvelopeV2.from_wire(
            {
                "schema_version": "observed-event-envelope.v2",
                "sequence": "1",
                "state_revision": "0",
                "event": {"kind": "public_outcome", "code": "synthetic"},
            }
        )
        with_events = dataclasses.replace(step, observed_events=(event,))
        with self.assertRaises(WireError):
            with_events.validate()

        closed_running = dataclasses.replace(
            step,
            submission=PlayerStepSubmissionV1("rejected", "episode_closed"),
        )
        with self.assertRaises(WireError):
            closed_running.validate()

        other_code_closed_episode = dataclasses.replace(
            step,
            status=EpisodeStatus.from_wire(
                {"kind": "truncated", "reason": "external_stop", "players": []}
            ),
        )
        with self.assertRaises(WireError):
            other_code_closed_episode.validate()


def test_v2_public_boundary_excludes_privileged_fields() -> None:
    test = PlayerApiTests("test_v2_public_boundary_excludes_privileged_fields")
    test.test_v2_public_boundary_excludes_privileged_fields()


class InformationProvenanceParityTests(unittest.TestCase):
    GOLDEN = ROOT / "wire" / "golden" / "information-state-envelope.v2.json"

    def _decode(self) -> PlayerInformationStateV2:
        from mtgml.wire import decode_canonical

        payload = self.GOLDEN.read_bytes()
        decoded = decode_canonical("information-state-envelope.v2", payload)
        assert isinstance(decoded, PlayerInformationStateV2)
        return decoded

    def test_all_four_observed_causes_survive_the_public_roundtrip(self) -> None:
        information = self._decode()

        def causes(value: object) -> set[str]:
            found: set[str] = set()
            if isinstance(value, dict):
                if value.get("kind") == "observed" and "cause" in value:
                    found.add(str(value["cause"]))
                for item in value.values():
                    found |= causes(item)
            elif isinstance(value, list):
                for item in value:
                    found |= causes(item)
            return found

        wire = json.loads(self.GOLDEN.read_text(encoding="utf-8"))
        expected = causes(wire["retained_knowledge"])
        self.assertEqual(
            expected,
            {"public_event", "private_look", "explicit_reveal", "own_private_identity"},
        )
        self.assertEqual(causes(information.to_wire()), expected)
        self.assertEqual(
            [record.opaque_object_id for record in information.retained_knowledge],
            [3, 7],
        )

    def test_future_provenance_sequence_is_rejected(self) -> None:
        from mtgml.errors import WireError

        information = self._decode()
        corrupt = PlayerInformationStateV2(
            schema_version=information.schema_version,
            perspective=information.perspective,
            state_revision=information.state_revision,
            current_observation=information.current_observation,
            next_visible_sequence=information.next_visible_sequence,
            retained_knowledge=information.retained_knowledge,
            digest=information.digest,
        )
        active = corrupt.retained_knowledge[0]
        fact = active.current_known_location_fact
        assert fact is not None and fact.provenance.sequence is not None
        forged = PlayerKnownObjectV1(
            kind="active",
            opaque_object_id=active.opaque_object_id,
            known_definition=active.known_definition,
            current_known_location_fact=fact,
            historical_locations=active.historical_locations,
            acquisition=active.acquisition,
        )
        with_self_sequence = PlayerInformationStateV2(
            schema_version=corrupt.schema_version,
            perspective=corrupt.perspective,
            state_revision=corrupt.state_revision,
            current_observation=corrupt.current_observation,
            next_visible_sequence=fact.provenance.sequence,
            retained_knowledge=(forged,),
            digest=corrupt.digest,
        )
        with self.assertRaises(WireError):
            with_self_sequence.validate()


if __name__ == "__main__":
    unittest.main()


class InitialConfigurationCursorParityTests(unittest.TestCase):
    """Rust and Python must agree: initial_configuration owns no visible
    sequence and is valid even at cursor zero, while observed facts are bound
    by the cursor."""

    @staticmethod
    def _information(next_visible_sequence: int, acquisition: dict) -> PlayerInformationStateV2:
        import sys

        sys.path.insert(0, str(ROOT / "python" / "src"))
        from mtgml.observation import InformationStateDigestInputV2, ObservationEnvelope
        from mtgml.wire import compute_information_state_digest_v2

        observation = ObservationEnvelope(
            schema_version="observation-envelope.v1",
            perspective=1,
            state_revision=0,
            payload_codec="synthetic-m2-observation.v1",
            payload_base64="e30=",
            digest="0000000000000000000000000000000000000000000000000000000000000000",
        )
        record = {
            "kind": "active",
            "opaque_object_id": "1",
            "known_definition": None,
            "current_known_location_fact": None,
            "historical_locations": [],
            "acquisition": acquisition,
        }
        input_value = InformationStateDigestInputV2.from_wire(
            {
                "schema_version": "information-state-digest-input.v2",
                "perspective": "1",
                "state_revision": "0",
                "current_observation": observation.to_wire(),
                "next_visible_sequence": str(next_visible_sequence),
                "retained_knowledge": [record],
            }
        )
        _, digest = compute_information_state_digest_v2(input_value)
        return PlayerInformationStateV2(
            schema_version="information-state-envelope.v2",
            perspective=1,
            state_revision=0,
            current_observation=observation,
            next_visible_sequence=next_visible_sequence,
            retained_knowledge=(PlayerKnownObjectV1.from_wire(record),),
            digest=digest,
        )

    def test_initial_configuration_is_valid_at_cursor_zero(self) -> None:
        information = self._information(0, {"kind": "initial_configuration"})
        information.validate()

    def test_observed_sequence_zero_is_invalid_at_cursor_zero(self) -> None:
        from mtgml.errors import WireError

        information = self._information(
            0,
            {
                "kind": "observed",
                "channel": "public",
                "sequence": "0",
                "cause": "public_event",
            },
        )
        with self.assertRaises(WireError):
            information.validate()
