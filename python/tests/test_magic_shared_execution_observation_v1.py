from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from mtgml.errors import WireError
from mtgml.magic_shared_execution_observation_v1 import (
    MagicSharedExecutionObservationV1,
    PublicTemporaryEffectV1,
)
from mtgml.wire import decode_canonical, encode_canonical

ROOT = Path(__file__).resolve().parents[2]


def shared_fixture() -> dict[str, object]:
    return json.loads(
        (ROOT / "schemas/examples/magic-shared-execution-observation-v1.json").read_text(
            encoding="utf-8"
        )
    )


class MagicSharedExecutionObservationV1Tests(unittest.TestCase):
    def test_positive_fixture_round_trips_all_stack_and_effect_variants(self) -> None:
        raw = shared_fixture()
        payload = MagicSharedExecutionObservationV1.from_wire(raw)
        self.assertEqual(payload.to_wire(), raw)
        self.assertEqual(
            decode_canonical("magic-shared-execution-observation.v1", encode_canonical(payload)),
            payload,
        )
        self.assertEqual(len(payload.stack), 3)
        self.assertEqual(len(payload.temporary_effects), 3)

    def test_rejects_trusted_ids_unbound_sources_and_noncanonical_effect_order(self) -> None:
        raw = shared_fixture()
        with self.assertRaises(WireError):
            MagicSharedExecutionObservationV1.from_wire(raw | {"state_revision": "8"})

        trusted = copy.deepcopy(raw)
        trusted["stack"][0]["stack_object_id"] = "88"
        with self.assertRaises(WireError):
            MagicSharedExecutionObservationV1.from_wire(trusted)

        unbound = copy.deepcopy(raw)
        unbound["stack"][1]["source_object"] = None
        with self.assertRaises(WireError):
            MagicSharedExecutionObservationV1.from_wire(unbound)

        reversed_effects = copy.deepcopy(raw)
        reversed_effects["temporary_effects"].reverse()
        with self.assertRaises(WireError):
            MagicSharedExecutionObservationV1.from_wire(reversed_effects)

    def test_duplicate_effect_views_preserve_multiplicity_and_sorted_operations(self) -> None:
        raw = shared_fixture()
        duplicate = copy.deepcopy(raw["temporary_effects"][1])
        effects = [PublicTemporaryEffectV1.from_wire(item) for item in raw["temporary_effects"]]
        effects.insert(2, PublicTemporaryEffectV1.from_wire(duplicate))
        raw["temporary_effects"] = [item.to_wire() for item in effects]
        payload = MagicSharedExecutionObservationV1.from_wire(raw)
        self.assertEqual(len(payload.temporary_effects), 4)


if __name__ == "__main__":
    unittest.main()
