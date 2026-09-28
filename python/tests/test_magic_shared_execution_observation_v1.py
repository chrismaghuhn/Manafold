from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path

from mtgml.errors import WireError
from mtgml.magic_shared_execution_observation_v1 import (
    GrantKeywordV1,
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

    def test_rejects_malformed_values_and_duplicate_semantic_keys(self) -> None:
        malformed_keyword = shared_fixture()
        malformed_keyword["temporary_effects"][1]["operation"]["keyword"] = []
        with self.assertRaises(WireError):
            MagicSharedExecutionObservationV1.from_wire(malformed_keyword)

        duplicate_mode_slot = shared_fixture()
        duplicate_mode_slot["stack"][0]["modes"].append({"mode_slot": 0, "selected_mode": 2})
        with self.assertRaises(WireError):
            MagicSharedExecutionObservationV1.from_wire(duplicate_mode_slot)

        duplicate_cost_id = shared_fixture()
        duplicate_cost_id["stack"][0]["cost_facts"]["paid_additional_cost_ids"] = [2, 2]
        with self.assertRaises(WireError):
            MagicSharedExecutionObservationV1.from_wire(duplicate_cost_id)

        duplicate_affected_object = shared_fixture()
        duplicate_affected_object["temporary_effects"][0]["affected_objects"] = ["2", "2"]
        with self.assertRaises(WireError):
            MagicSharedExecutionObservationV1.from_wire(duplicate_affected_object)

        overflowing_power = shared_fixture()
        overflowing_power["temporary_effects"][0]["operation"]["power"] = 2**31
        with self.assertRaises(WireError):
            MagicSharedExecutionObservationV1.from_wire(overflowing_power)

    def test_invalid_constructed_keyword_has_stable_encoding_error(self) -> None:
        with self.assertRaises(WireError) as raised:
            GrantKeywordV1([]).to_wire()  # type: ignore[arg-type]
        self.assertEqual(raised.exception.code, "encode.serialization")


if __name__ == "__main__":
    unittest.main()
