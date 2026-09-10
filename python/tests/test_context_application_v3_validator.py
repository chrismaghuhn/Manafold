from __future__ import annotations

import sys
import unittest
from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from context_application_v3_resolver import ResolvedRpaV2Member
from context_application_v3_validator import (
    ContextApplicationV3SemanticValidationError,
    ContextApplicationV3SemanticValidator,
)
from mtgml.authority import (
    ParticipantRoleBridgeEntryV1,
    ParticipantRoleBridgeV1,
)
from test_context_application_v3_contract import ContextApplicationV3InputV1, member
from test_relation_application_v2_contract import member as rpa_member
from test_relation_application_v2_review_admission import valid_record


class FakeV1Validator:
    def _validate_context_member_source_contract_v1(
        self, _member: object, _theorem: object, _resolved: object, _label: str
    ) -> None:
        return None


class FakeRpaMemberResolver:
    def __init__(self, rpa_member: object) -> None:
        record, _ = valid_record()
        self.result = ResolvedRpaV2Member(record, rpa_member)

    def resolve_current_rpa_member(self, *_args: object) -> ResolvedRpaV2Member:
        return self.result


class FakeV3Resolver:
    def __init__(
        self,
        rpa_member: object,
        roles: tuple[str, str] = ("ordered_participant", "ordered_participant"),
    ) -> None:
        self._rpa_member_resolver = FakeRpaMemberResolver(rpa_member)
        self._roles = roles

    def _validated_base(self) -> tuple[FakeV1Validator, dict[str, object]]:
        return FakeV1Validator(), {}

    def resolve_current_context_theorem(self, _theorem_id: object) -> dict[str, object]:
        return {
            "subject_shape": {
                "arity": "binary",
                "directionality": "directed",
                "participant_roles": [
                    {
                        "position": 0,
                        "role": "source",
                        "participant_kind": "requirement_family",
                        "semantic_ref": "cap.mass_destruction",
                    },
                    {
                        "position": 1,
                        "role": "affected",
                        "participant_kind": "requirement_family",
                        "semantic_ref": "cap.death_trigger",
                    },
                ],
                "host_relationship": "cross_host",
            },
            "preconditions": [],
            "context_dimensions": ["not_applicable"] * 10,
            "temporal_semantics": ["not_applicable"] * 4,
        }

    def resolve_member_source_instance(self, _member: object) -> object:
        return SimpleNamespace(
            source_instance_record={
                "participant_bindings": [
                    {
                        "role": self._roles[0],
                        "participant_ref": {
                            "participant_kind": "requirement_family",
                            "semantic_ref": "cap.mass_destruction",
                        },
                    },
                    {
                        "role": self._roles[1],
                        "participant_ref": {
                            "participant_kind": "requirement_family",
                            "semantic_ref": "cap.death_trigger",
                        },
                    },
                ],
                "source_context": {
                    name: "not_applicable"
                    for name in (
                        "zone",
                        "visibility",
                        "timing",
                        "temporal_order",
                        "source_affected_relation",
                        "control_ownership_relation",
                        "replacement_layer_relation",
                        "trigger_lki_relation",
                        "information_relation",
                        "decision_actor_relation",
                    )
                },
            }
        )


class ContextApplicationV3ValidatorTests(unittest.TestCase):
    def test_role_divergent_member_is_accepted(self) -> None:
        result = ContextApplicationV3SemanticValidator(FakeV3Resolver(rpa_member())).validate(
            ContextApplicationV3InputV1(b"t" * 32, (member(),))
        )
        self.assertTrue(result.valid)

    def test_exact_role_member_is_rejected(self) -> None:
        exact_bridge = ParticipantRoleBridgeV1(
            (
                ParticipantRoleBridgeEntryV1(
                    0, "requirement_family", "cap.mass_destruction", "source", "source"
                ),
                ParticipantRoleBridgeEntryV1(
                    1, "requirement_family", "cap.death_trigger", "affected", "affected"
                ),
            )
        )
        with self.assertRaises(ContextApplicationV3SemanticValidationError) as raised:
            ContextApplicationV3SemanticValidator(
                FakeV3Resolver(
                    replace(rpa_member(), participant_role_bridge_v1=exact_bridge),
                    roles=("source", "affected"),
                )
            ).validate(ContextApplicationV3InputV1(b"t" * 32, (member(),)))
        self.assertEqual(raised.exception.code, "CONTEXT_APPLICATION_V3_NOT_DIVERGENT")

    def test_wrong_reviewed_role_is_rejected(self) -> None:
        wrong = replace(
            rpa_member(),
            reviewed_relation_binding_v1=[
                "cross_deck",
                "directional_binary",
                "directed",
                "cross_host",
                [
                    [0, "affected", "requirement_family", "cap.mass_destruction"],
                    [1, "source", "requirement_family", "cap.death_trigger"],
                ],
            ],
        )
        with self.assertRaises(ContextApplicationV3SemanticValidationError) as raised:
            ContextApplicationV3SemanticValidator(FakeV3Resolver(wrong)).validate(
                ContextApplicationV3InputV1(b"t" * 32, (member(),))
            )
        self.assertEqual(raised.exception.code, "CONTEXT_APPLICATION_V3_ROLE_MISMATCH")


if __name__ == "__main__":
    unittest.main()
