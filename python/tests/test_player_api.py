from __future__ import annotations

import inspect
import sys
import unittest
from pathlib import Path
from typing import get_type_hints

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python" / "src"))

from mtgml.decision import DecisionResponseV3
from mtgml.decision_v4 import PlayerDecisionRequestV4
from mtgml.observation import (
    MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1,
    OBSERVATION_SCHEMA_V2,
    InformationStateDigestInput,
    ObservationEnvelope,
    PlayerInformationState,
    PlayerKnownObjectV1,
    PlayerStepV4,
    compute_information_state_digest,
)
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
        self.assertIs(hints["return"], PlayerStepV4)
        self.assertEqual(
            get_type_hints(PlayerClient.visible_decision)["return"],
            PlayerDecisionRequestV4 | None,
        )
        self.assertIs(get_type_hints(PlayerClient.observation)["return"], ObservationEnvelope)
        self.assertIs(
            get_type_hints(PlayerClient.information_state)["return"], PlayerInformationState
        )
        self.assertEqual(get_type_hints(PlayerClient.submit)["response"], DecisionResponseV3)


class PlayerStepSubmissionContractTests(unittest.TestCase):
    """The submission outcome is a closed variant union: invalid domain
    objects must never serialize as plausible wire data (WIRE_CONTRACT)."""

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


OBSERVATION_DIGEST_OF_EMPTY_OBJECT = (
    "90845308617867fd703c6c4f37ede7908da24420053821f89190ad36236dfca3"
)

# Active and retired records carrying all four observed causes.
RICH_RETAINED_KNOWLEDGE: list[dict] = [
    {
        "acquisition": {
            "cause": "private_look",
            "channel": "private",
            "kind": "observed",
            "sequence": "1",
        },
        "current_known_location_fact": {
            "location": {"player": "2", "zone": "exile"},
            "provenance": {
                "cause": "explicit_reveal",
                "channel": "public",
                "kind": "observed",
                "sequence": "4",
            },
        },
        "historical_locations": [
            {
                "location": {"player": None, "zone": "hand"},
                "provenance": {
                    "cause": "own_private_identity",
                    "channel": "private",
                    "kind": "observed",
                    "sequence": "3",
                },
            }
        ],
        "kind": "active",
        "known_definition": "42",
        "opaque_object_id": "3",
    },
    {
        "acquisition": {
            "cause": "public_event",
            "channel": "public",
            "kind": "observed",
            "sequence": "2",
        },
        "historical_locations": [],
        "invalidation": {
            "provenance": {
                "cause": "explicit_reveal",
                "channel": "public",
                "kind": "observed",
                "sequence": "4",
            },
            "reason": "shuffle",
        },
        "kind": "retired",
        "known_definition": None,
        "last_known_location_fact": {
            "location": {"player": None, "zone": "battlefield"},
            "provenance": {"kind": "initial_configuration"},
        },
        "opaque_object_id": "7",
    },
]


def _information_v3(next_visible_sequence: int, records: list[dict]) -> PlayerInformationState:
    """A V3 information state with a correct digest. Computing the digest
    validates the retained knowledge, so invalid knowledge raises here."""
    observation = ObservationEnvelope(
        OBSERVATION_SCHEMA_V2,
        1,
        next_visible_sequence,
        MAGIC_SHARED_EXECUTION_OBSERVATION_SCHEMA_V1,
        "e30=",
        OBSERVATION_DIGEST_OF_EMPTY_OBJECT,
    )
    retained = tuple(PlayerKnownObjectV1.from_wire(record) for record in records)
    input_value = InformationStateDigestInput(
        "information-state-digest-input.v3", 1, observation, next_visible_sequence, retained
    )
    _, digest = compute_information_state_digest(input_value)
    return PlayerInformationState(
        "information-state-envelope.v3", 1, observation, next_visible_sequence, retained, digest
    )


def _active(acquisition: dict) -> dict:
    return {
        "kind": "active",
        "opaque_object_id": "1",
        "known_definition": None,
        "current_known_location_fact": None,
        "historical_locations": [],
        "acquisition": acquisition,
    }


def _observed(channel: str, sequence: int, cause: str) -> dict:
    return {"kind": "observed", "channel": channel, "sequence": str(sequence), "cause": cause}


class InformationProvenanceParityTests(unittest.TestCase):
    def test_all_four_observed_causes_survive_the_public_roundtrip(self) -> None:
        from mtgml.canonical import canonical_json_bytes
        from mtgml.wire import decode_canonical

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

        expected = causes(RICH_RETAINED_KNOWLEDGE)
        self.assertEqual(
            expected,
            {"public_event", "private_look", "explicit_reveal", "own_private_identity"},
        )
        information = _information_v3(5, RICH_RETAINED_KNOWLEDGE)
        decoded = decode_canonical(
            "information-state-envelope.v3", canonical_json_bytes(information.to_wire())
        )
        assert isinstance(decoded, PlayerInformationState)
        self.assertEqual(decoded, information)
        self.assertEqual(causes(decoded.to_wire()), expected)
        self.assertEqual(
            [record.opaque_object_id for record in decoded.retained_knowledge],
            [3, 7],
        )

    def test_future_provenance_sequence_is_rejected(self) -> None:
        from mtgml.errors import WireError

        # The current location was observed at sequence 4, so a cursor of 4
        # makes that observation lie in the future.
        with self.assertRaises(WireError) as caught:
            _information_v3(4, RICH_RETAINED_KNOWLEDGE[:1])
        self.assertEqual(caught.exception.code, "semantic.information_state")

    def test_invalid_cause_channel_combination_is_rejected(self) -> None:
        from mtgml.errors import WireError

        with self.assertRaises(WireError) as caught:
            _information_v3(5, [_active(_observed("public", 1, "private_look"))])
        self.assertEqual(caught.exception.code, "semantic.information_state")

    def test_invalidation_cannot_come_from_the_initial_configuration(self) -> None:
        from mtgml.errors import WireError

        retired = dict(RICH_RETAINED_KNOWLEDGE[1])
        retired["invalidation"] = {
            "provenance": {"kind": "initial_configuration"},
            "reason": "shuffle",
        }
        with self.assertRaises(WireError) as caught:
            _information_v3(5, [retired])
        self.assertEqual(caught.exception.code, "semantic.information_state")


class InitialConfigurationCursorParityTests(unittest.TestCase):
    """Rust and Python must agree: initial_configuration owns no visible
    sequence and is valid even at cursor zero, while observed facts are bound
    by the cursor."""

    def test_initial_configuration_is_valid_at_cursor_zero(self) -> None:
        _information_v3(0, [_active({"kind": "initial_configuration"})]).validate()

    def test_observed_sequence_zero_is_invalid_at_cursor_zero(self) -> None:
        from mtgml.errors import WireError

        with self.assertRaises(WireError) as caught:
            _information_v3(0, [_active(_observed("public", 0, "public_event"))])
        self.assertEqual(caught.exception.code, "semantic.information_state")


if __name__ == "__main__":
    unittest.main()
