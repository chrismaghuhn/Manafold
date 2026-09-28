from __future__ import annotations

import json
import unittest
from pathlib import Path

from mtgml.canonical import canonical_json_bytes
from mtgml.errors import WireError
from mtgml.wire import decode_canonical, encode_canonical

ROOT = Path(__file__).resolve().parents[2]
GOLDEN = ROOT / "wire" / "golden"


class PlayerProductV4Tests(unittest.TestCase):
    def test_every_v4_observed_event_golden_round_trips_exactly(self) -> None:
        paths = sorted(GOLDEN.glob("observed-event-v4-*.json"))
        self.assertGreaterEqual(len(paths), 16)
        for path in paths:
            payload = path.read_bytes()
            event = decode_canonical("observed-event-envelope.v4", payload)
            self.assertEqual(encode_canonical(event), payload, path.name)

    def test_shared_execution_observation_and_player_step_have_no_global_revision(self) -> None:
        for contract, filename in (
            (
                "magic-shared-execution-observation.v1",
                "magic-shared-execution-observation.v1.json",
            ),
            ("player-step.v4", "player-step-v4.json"),
        ):
            payload = (GOLDEN / filename).read_bytes()
            product = decode_canonical(contract, payload)
            self.assertEqual(encode_canonical(product), payload)
            self.assertNotIn(b"state_revision", payload)

    def test_player_step_v4_rejects_global_revision_and_invalid_public_cursor(self) -> None:
        value = json.loads((GOLDEN / "player-step-v4.json").read_text(encoding="utf-8"))
        with_global_revision = dict(value, state_revision="1")
        with self.assertRaises(WireError):
            decode_canonical(
                "player-step.v4",
                canonical_json_bytes(with_global_revision),
            )

        future_event = json.loads(json.dumps(value))
        future_event["observed_events"][0]["sequence"] = "5"
        with self.assertRaises(WireError):
            decode_canonical(
                "player-step.v4",
                canonical_json_bytes(future_event),
            )


if __name__ == "__main__":
    unittest.main()
