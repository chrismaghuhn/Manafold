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
    ContextBridgeRelationV2,
    ParticipantRoleBridgeEntryV1,
    ParticipantRoleBridgeV1,
)
from test_context_application_v3_contract import ContextApplicationV3InputV1, member
from test_relation_application_v2_contract import member as rpa_member
from test_relation_application_v2_review_admission import valid_record


class FakeV1Validator:
    def __init__(self) -> None:
        self.last_member: object | None = None
        self.last_theorem: object | None = None

    def _validate_context_member_source_contract_v1(
        self, member: object, theorem: object, _resolved: object, _label: str
    ) -> None:
        self.last_member = member
        self.last_theorem = theorem
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
        theorem_preconditions: list[dict[str, object]] | None = None,
        theorem_context_dimensions: tuple[str, ...] | None = None,
    ) -> None:
        self._rpa_member_resolver = FakeRpaMemberResolver(rpa_member)
        self._roles = roles
        self._theorem_preconditions = theorem_preconditions or []
        self._theorem_context_dimensions = theorem_context_dimensions or ("not_applicable",) * 10
        self.v1_validator = FakeV1Validator()

    def _validated_base(self) -> tuple[FakeV1Validator, dict[str, object]]:
        return self.v1_validator, {}

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
            "preconditions": self._theorem_preconditions,
            "context_dimensions": list(self._theorem_context_dimensions),
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

    def test_real_context_theorem_wire_precondition_is_accepted(self) -> None:
        theorem_preconditions = [
            {
                "precondition_id": "source_context",
                "precondition_kind": "source_context",
                "payload": ["zone", "battlefield"],
            }
        ]
        application_member = replace(
            member(),
            precondition_attestations_v1=[
                [
                    "source_context",
                    ["zone", "battlefield"],
                    [member().member_evidence_refs[0].to_cbor()],
                    "fixture rationale",
                ]
            ],
        )
        result = ContextApplicationV3SemanticValidator(
            FakeV3Resolver(rpa_member(), theorem_preconditions=theorem_preconditions)
        ).validate(ContextApplicationV3InputV1(b"t" * 32, (application_member,)))
        self.assertTrue(result.valid)

    def test_all_v1_precondition_kinds_use_theorem_wire_and_v1_projection(self) -> None:
        cases = (
            (
                "candidate_relation_shape",
                ["cross_deck", "directional_binary", "directed", "cross_host"],
            ),
            (
                "participant_binding",
                [0, "ordered_participant", "requirement_family", "cap.mass_destruction"],
            ),
            ("b2_boundary", ["family.a", "active", "classification", "definition"]),
            ("source_context", ["zone", "not_applicable"]),
            ("temporal_semantic", ["trigger_order", "not_applicable"]),
            (
                "class_projection",
                [
                    "binary",
                    "directed",
                    [],
                    ["not_applicable"] * 10,
                    ["not_applicable"] * 4,
                    [],
                    [],
                    [],
                    [],
                ],
            ),
        )
        for kind, payload in cases:
            with self.subTest(kind=kind):
                precondition_id = f"{kind}-precondition"
                theorem_preconditions = [
                    {
                        "precondition_id": precondition_id,
                        "precondition_kind": kind,
                        "payload": payload,
                    }
                ]
                application_member = replace(
                    member(),
                    precondition_attestations_v1=[
                        [
                            precondition_id,
                            payload,
                            [member().member_evidence_refs[0].to_cbor()],
                            f"{kind} rationale",
                        ]
                    ],
                )
                resolver = FakeV3Resolver(rpa_member(), theorem_preconditions=theorem_preconditions)
                result = ContextApplicationV3SemanticValidator(resolver).validate(
                    ContextApplicationV3InputV1(b"t" * 32, (application_member,))
                )
                self.assertTrue(result.valid)
                assert isinstance(resolver.v1_validator.last_member, dict)
                projected_roles = resolver.v1_validator.last_member["context_binding"][
                    "participant_roles"
                ]
                self.assertEqual(projected_roles[0]["role"], "ordered_participant")
                self.assertEqual(
                    resolver.v1_validator.last_member["member_proof_attestation"],
                    rpa_member().to_wire()["member_proof_attestation"],
                )

    def test_source_context_divergence_stays_in_the_reviewed_bridge(self) -> None:
        application_member = member()
        bridge = application_member.context_member_bridge_attestation_v2
        first_slot = replace(
            bridge.context[0],
            reviewed_value="battlefield",
            relation=ContextBridgeRelationV2.REVIEWED_DIVERGENCE,
        )
        divergent_bridge = replace(bridge, context=(first_slot, *bridge.context[1:]))
        application_member = replace(
            application_member,
            context_member_bridge_attestation_v2=divergent_bridge,
        )
        resolver = FakeV3Resolver(
            rpa_member(),
            theorem_context_dimensions=("battlefield",) + ("not_applicable",) * 9,
        )
        result = ContextApplicationV3SemanticValidator(resolver).validate(
            ContextApplicationV3InputV1(b"t" * 32, (application_member,))
        )
        self.assertTrue(result.valid)
        resolved = resolver.resolve_member_source_instance(None)
        assert isinstance(resolved.source_instance_record, dict)
        self.assertEqual(
            resolved.source_instance_record["source_context"]["zone"],
            "not_applicable",
        )


if __name__ == "__main__":
    unittest.main()
