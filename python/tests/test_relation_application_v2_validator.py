from __future__ import annotations

import sys
import unittest
from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from mtgml.authority import (
    AuthorityContractError,
    ParticipantRoleBridgeEntryV1,
    ParticipantRoleBridgeV1,
)
from relation_application_v2_validator import (
    RelationApplicationV2SemanticValidationError,
    validate_relation_application_v2_semantics,
)
from test_relation_application_v2_contract import bridge, member


class FakeSourceResolver:
    def __init__(self, *, roles: tuple[str, str] = ("ordered_participant", "ordered_participant")):
        self.roles = roles

    def resolve_candidate_source_instance(self, *_args: object) -> object:
        participants = [
            {
                "role": role,
                "participant_ref": {
                    "participant_kind": "requirement_family",
                    "semantic_ref": ref,
                },
            }
            for role, ref in zip(
                self.roles,
                ("cap.mass_destruction", "cap.death_trigger"),
                strict=True,
            )
        ]
        return SimpleNamespace(
            candidate=SimpleNamespace(
                candidate_record={
                    "scope": "cross_deck",
                    "relation": "directional_binary",
                    "participant_refs": [
                        {
                            "participant_kind": "requirement_family",
                            "semantic_ref": "cap.mass_destruction",
                        },
                        {
                            "participant_kind": "requirement_family",
                            "semantic_ref": "cap.death_trigger",
                        },
                    ],
                }
            ),
            source_instance_record={"participant_bindings": participants, "source_context": {}},
        )


def theorem_record(
    *,
    roles: tuple[str, str] = ("source", "affected"),
    preconditions: list[object] | None = None,
) -> dict[str, object]:
    return {
        "proof_kind": "positive_interaction",
        "subject": {
            "arity": "binary",
            "relation": "directional_binary",
            "directionality": "directed",
            "participant_roles": [
                {
                    "position": 0,
                    "role": roles[0],
                    "participant_kind": "requirement_family",
                    "semantic_ref": "cap.mass_destruction",
                },
                {
                    "position": 1,
                    "role": roles[1],
                    "participant_kind": "requirement_family",
                    "semantic_ref": "cap.death_trigger",
                },
            ],
            "host_relationship": "cross_host",
        },
        "proof_payload": {
            "kind": "positive_interaction",
            "class_projection_template": None,
        },
        "preconditions": [] if preconditions is None else preconditions,
    }


class RelationApplicationV2ValidatorTests(unittest.TestCase):
    def test_divergent_bridge_is_accepted_without_role_inference(self) -> None:
        result = validate_relation_application_v2_semantics(
            member(), FakeSourceResolver(), theorem_record=theorem_record()
        )
        self.assertTrue(result.valid)
        self.assertEqual(result.divergent_positions, (0, 1))

    def test_exact_role_member_is_rejected_from_v2(self) -> None:
        exact_bridge = ParticipantRoleBridgeV1(
            (
                ParticipantRoleBridgeEntryV1(
                    0,
                    "requirement_family",
                    "cap.mass_destruction",
                    "ordered_participant",
                    "ordered_participant",
                ),
                ParticipantRoleBridgeEntryV1(
                    1, "requirement_family", "cap.death_trigger", "ordered_participant", "affected"
                ),
            )
        )
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                replace(
                    member(),
                    participant_role_bridge_v1=exact_bridge,
                    reviewed_relation_binding_v1=[
                        "cross_deck",
                        "directional_binary",
                        "directed",
                        "cross_host",
                        [
                            [
                                0,
                                "ordered_participant",
                                "requirement_family",
                                "cap.mass_destruction",
                            ],
                            [1, "affected", "requirement_family", "cap.death_trigger"],
                        ],
                    ],
                ),
                FakeSourceResolver(),
                theorem_record=theorem_record(roles=("ordered_participant", "affected")),
            )
        self.assertEqual(raised.exception.code, "RELATION_APPLICATION_V2_NOT_DIVERGENT")

    def test_mixed_exact_and_divergent_roles_are_rejected(self) -> None:
        mixed = ParticipantRoleBridgeV1(
            (
                ParticipantRoleBridgeEntryV1(
                    0, "requirement_family", "cap.mass_destruction", "ordered_participant", "source"
                ),
                ParticipantRoleBridgeEntryV1(
                    1,
                    "requirement_family",
                    "cap.death_trigger",
                    "ordered_participant",
                    "ordered_participant",
                ),
            )
        )
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                replace(
                    member(),
                    participant_role_bridge_v1=mixed,
                    reviewed_relation_binding_v1=[
                        "cross_deck",
                        "directional_binary",
                        "directed",
                        "cross_host",
                        [
                            [0, "source", "requirement_family", "cap.mass_destruction"],
                            [1, "ordered_participant", "requirement_family", "cap.death_trigger"],
                        ],
                    ],
                ),
                FakeSourceResolver(),
                theorem_record=theorem_record(roles=("source", "ordered_participant")),
            )
        self.assertEqual(raised.exception.code, "RELATION_APPLICATION_V2_NOT_DIVERGENT")

    def test_historical_role_substitution_fails_closed(self) -> None:
        altered = ParticipantRoleBridgeV1(
            (
                ParticipantRoleBridgeEntryV1(
                    0, "requirement_family", "cap.mass_destruction", "source", "source"
                ),
                bridge().entries[1],
            )
        )
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                replace(member(), participant_role_bridge_v1=altered),
                FakeSourceResolver(),
                theorem_record=theorem_record(),
            )
        self.assertEqual(raised.exception.code, "ROLE_BRIDGE_HISTORICAL_ROLE_MISMATCH")

    def test_reviewed_role_substitution_fails_closed(self) -> None:
        altered_binding = [
            "cross_deck",
            "directional_binary",
            "directed",
            "cross_host",
            [
                [0, "affected", "requirement_family", "cap.mass_destruction"],
                [1, "affected", "requirement_family", "cap.death_trigger"],
            ],
        ]
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                replace(member(), reviewed_relation_binding_v1=altered_binding),
                FakeSourceResolver(),
                theorem_record=theorem_record(),
            )
        self.assertEqual(raised.exception.code, "ROLE_BRIDGE_REVIEWED_ROLE_MISMATCH")

    def test_participant_kind_or_reference_substitution_fails_closed(self) -> None:
        altered = ParticipantRoleBridgeV1(
            (
                ParticipantRoleBridgeEntryV1(
                    0, "card", "cap.mass_destruction", "ordered_participant", "source"
                ),
                bridge().entries[1],
            )
        )
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                replace(member(), participant_role_bridge_v1=altered),
                FakeSourceResolver(),
                theorem_record=theorem_record(),
            )
        self.assertEqual(raised.exception.code, "ROLE_BRIDGE_PARTICIPANT_MISMATCH")

    def test_missing_bridge_is_rejected(self) -> None:
        with self.assertRaises(
            (AuthorityContractError, RelationApplicationV2SemanticValidationError)
        ):
            validate_relation_application_v2_semantics(
                replace(member(), participant_role_bridge_v1=None),
                FakeSourceResolver(),
                theorem_record=theorem_record(),
            )

    def test_participant_binding_keeps_historical_source_role(self) -> None:
        precondition = [
            "participant-fact",
            [0, "ordered_participant", "requirement_family", "cap.mass_destruction"],
            [
                [
                    "model",
                    "sources/model.json",
                    ["whole_artifact", None],
                    b"e" * 32,
                ]
            ],
            "historical fact",
        ]
        observed = replace(member(), precondition_attestations_v1=[precondition])
        theorem = theorem_record(
            preconditions=[
                {
                    "precondition_id": "participant-fact",
                    "precondition_kind": "participant_binding",
                    "payload": [
                        0,
                        "ordered_participant",
                        "requirement_family",
                        "cap.mass_destruction",
                    ],
                }
            ]
        )
        result = validate_relation_application_v2_semantics(
            observed, FakeSourceResolver(), theorem_record=theorem
        )
        self.assertTrue(result.valid)

        mutated = list(precondition)
        mutated[1] = [0, "source", "requirement_family", "cap.mass_destruction"]
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                replace(member(), precondition_attestations_v1=[mutated]),
                FakeSourceResolver(),
                theorem_record=theorem,
            )
        self.assertEqual(raised.exception.code, "PRECONDITION_MISMATCH")


if __name__ == "__main__":
    unittest.main()
