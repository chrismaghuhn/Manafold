from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from mtgml._magic_basic_land_observation_v1 import MagicBasicLandObservationV1
from mtgml.canonical import canonical_json_bytes
from mtgml.errors import WireError
from mtgml.observation_v3 import (
    INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3,
    InformationStateDigestInputV3,
    ObservationEnvelopeV2,
    PlayerInformationStateV3,
    compute_information_state_digest_v3,
)
from mtgml.wire import decode_canonical, encode_canonical

ROOT = Path(__file__).resolve().parents[2]


def fixture(path: str) -> dict[str, object]:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


class M4Phase6ObservationTests(unittest.TestCase):
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


class SuccessorObservationDtoTests(unittest.TestCase):
    def test_observation_envelope_v2_fixture_round_trips_without_global_revision(self) -> None:
        raw = json.loads(
            (ROOT / "schemas/examples/observation-envelope-v2.json").read_text(encoding="utf-8")
        )
        observation = ObservationEnvelopeV2.from_wire(raw)
        self.assertNotIn("state_revision", observation.to_wire())
        self.assertEqual(
            decode_canonical("observation-envelope.v2", encode_canonical(observation)),
            observation,
        )
        with self.assertRaises(WireError):
            ObservationEnvelopeV2.from_wire(raw | {"state_revision": "4"})

    def test_information_state_v3_digest_fixture_binds_cursor_and_safe_children(self) -> None:
        raw = json.loads(
            (ROOT / "schemas/examples/information-state-envelope-v3.json").read_text(
                encoding="utf-8"
            )
        )
        information = PlayerInformationStateV3.from_wire(raw)
        self.assertEqual(
            information.digest_input().schema_version, INFORMATION_STATE_DIGEST_INPUT_SCHEMA_V3
        )
        self.assertNotIn("state_revision", information.to_wire())
        self.assertEqual(
            decode_canonical("information-state-envelope.v3", encode_canonical(information)),
            information,
        )
        digest_input = InformationStateDigestInputV3.from_wire(
            json.loads(
                (ROOT / "schemas/examples/information-state-digest-input-v3.json").read_text(
                    encoding="utf-8"
                )
            )
        )
        _, digest = compute_information_state_digest_v3(digest_input)
        self.assertEqual(digest, information.digest)

    def test_rejects_cursor_mismatch_and_wrong_digest(self) -> None:
        raw = json.loads(
            (ROOT / "schemas/examples/information-state-envelope-v3.json").read_text(
                encoding="utf-8"
            )
        )
        wrong_cursor = copy.deepcopy(raw)
        wrong_cursor["next_visible_sequence"] = "6"
        with self.assertRaises(WireError):
            PlayerInformationStateV3.from_wire(wrong_cursor)
        wrong_digest = copy.deepcopy(raw)
        wrong_digest["digest"] = "0" * 64
        with self.assertRaises(WireError):
            PlayerInformationStateV3.from_wire(wrong_digest)


if __name__ == "__main__":
    unittest.main()
