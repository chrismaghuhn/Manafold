from __future__ import annotations

import sys
import unittest
from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from authority_validator import validate_relation_member_proof_v1_against_theorem
from mtgml.authority import (
    AuthorityContractError,
    ParticipantRoleBridgeEntryV1,
    ParticipantRoleBridgeV1,
    RelationApplicationV2,
)
from relation_application_v2_resolver import RelationApplicationV2Resolver
from relation_application_v2_validator import (
    RelationApplicationV2SemanticValidationError,
    validate_relation_application_v2_semantics,
)
from test_relation_application_v2_contract import bridge, evidence, member


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
            "causal_chain": [{}],
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
                    "source",
                    "source",
                ),
                ParticipantRoleBridgeEntryV1(
                    1, "requirement_family", "cap.death_trigger", "affected", "affected"
                ),
            )
        )
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                replace(
                    member(),
                    participant_role_bridge_v1=exact_bridge,
                ),
                FakeSourceResolver(roles=("source", "affected")),
                theorem_record=theorem_record(),
            )
        self.assertEqual(raised.exception.code, "RELATION_APPLICATION_V2_NOT_DIVERGENT")

    def test_mixed_exact_and_divergent_roles_are_rejected(self) -> None:
        exact = replace(
            member(digest=b"a" * 32, source_instance_id="exact"),
            participant_role_bridge_v1=ParticipantRoleBridgeV1(
                (
                    ParticipantRoleBridgeEntryV1(
                        0, "requirement_family", "cap.mass_destruction", "source", "source"
                    ),
                    ParticipantRoleBridgeEntryV1(
                        1, "requirement_family", "cap.death_trigger", "affected", "affected"
                    ),
                )
            ),
        )

        class MixedSourceResolver(FakeSourceResolver):
            def resolve_candidate_source_instance(self, *args: object) -> object:
                source_instance_id = args[2]
                roles = ("source", "affected") if source_instance_id == "exact" else self.roles
                return FakeSourceResolver(roles=roles).resolve_candidate_source_instance(*args)

        divergent = member(digest=b"b" * 32, source_instance_id="divergent")
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                RelationApplicationV2(
                    theorem_record_id_bytes=bytes(32),
                    terminal_disposition="required_interaction",
                    members=(exact, divergent),
                ),
                MixedSourceResolver(),
                theorem_record=theorem_record(),
            )
        self.assertEqual(raised.exception.code, "RELATION_APPLICATION_V2_NOT_DIVERGENT")

    def test_one_exact_position_does_not_make_a_member_exact(self) -> None:
        partly_exact = ParticipantRoleBridgeV1(
            (
                ParticipantRoleBridgeEntryV1(
                    0, "requirement_family", "cap.mass_destruction", "source", "source"
                ),
                bridge().entries[1],
            )
        )
        result = validate_relation_application_v2_semantics(
            replace(member(), participant_role_bridge_v1=partly_exact),
            FakeSourceResolver(roles=("source", "ordered_participant")),
            theorem_record=theorem_record(),
        )
        self.assertTrue(result.valid)
        self.assertEqual(result.divergent_positions, (1,))

    def test_terminal_disposition_remains_bound_to_theorem_proof_kind(self) -> None:
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                RelationApplicationV2(
                    theorem_record_id_bytes=bytes(32),
                    terminal_disposition="not_an_interaction_with_proof",
                    members=(member(),),
                ),
                FakeSourceResolver(),
                theorem_record=theorem_record(),
            )
        self.assertEqual(raised.exception.code, "RELATION_APPLICATION_V2_THEOREM_MISMATCH")

    def test_positive_interaction_binds_the_complete_causal_chain(self) -> None:
        theorem = theorem_record()
        theorem["proof_payload"] = {
            "kind": "positive_interaction",
            "causal_chain": [{}],
            "class_projection_template": None,
        }
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                replace(
                    member(),
                    member_proof_attestation_v1=["positive_interaction", [[], None]],
                ),
                FakeSourceResolver(),
                theorem_record=theorem,
            )
        self.assertEqual(raised.exception.code, "CAUSAL_CHAIN_BINDING_MISMATCH")

    def test_positive_separation_uses_the_variable_v1_coverage_shape(self) -> None:
        channels = (
            "participant_boundary",
            "event_or_effect_causality",
            "target_or_choice",
            "zone_or_object_identity",
            "control_or_ownership",
            "replacement_or_layer",
            "trigger_or_lki",
            "information_or_visibility",
            "ordering_or_temporal",
            "decision_actor",
            "format_and_declared_scope",
        )
        coverages = [
            [
                channel,
                "separated",
                [["b2_boundary", ["family", "active", "primary", "definition"]]],
                [evidence().to_cbor()],
                [],
                "covered",
            ]
            for channel in channels
        ]
        theorem = theorem_record()
        theorem["proof_kind"] = "positive_separation"
        theorem["proof_payload"] = {
            "kind": "positive_separation",
            "separation_obligations": [
                {"channel": channel, "required_conclusion": "separated"} for channel in channels
            ],
        }
        observed = replace(
            member(),
            member_proof_attestation_v1=["positive_separation", [coverages]],
        )
        result = validate_relation_application_v2_semantics(
            observed,
            FakeSourceResolver(),
            theorem_record=theorem,
        )
        self.assertTrue(result.valid)

    def test_model_bound_scope_uses_the_v1_single_attestation_shape(self) -> None:
        model_boundary = [
            "sources/m2_5/closures/C/declared_interaction_model.v2.json",
            "manafold.m2.5.c.declared-interaction-model.v2",
            b"m" * 32,
            ["coverage_scope", None],
        ]
        candidate_shape = ["cross_deck", "directional_binary", "binary", "directed", 2]
        scope = [
            "declared-interaction-model.v2",
            "2",
            model_boundary,
            "undeclared_relation_shape",
            candidate_shape,
            [evidence().to_cbor()],
        ]
        observed = replace(
            member(),
            member_proof_attestation_v1=["model_bound_scope", [scope]],
        )
        theorem = {
            "proof_payload": {
                "kind": "model_bound_scope",
                "reason_code": "undeclared_relation_shape",
                "observed_candidate_shape": {
                    "scope": "cross_deck",
                    "relation": "directional_binary",
                    "arity": "binary",
                    "directionality": "directed",
                    "participant_count": 2,
                },
                "model_boundary_ref": {
                    "path": model_boundary[0],
                    "schema": model_boundary[1],
                    "raw_sha256": (b"m" * 32).hex(),
                    "locator": {"kind": "coverage_scope"},
                },
            }
        }
        validate_relation_member_proof_v1_against_theorem(
            observed.to_wire(),
            theorem,
            "member",
            declared_model={
                "model_id": "declared-interaction-model.v2",
                "model_version": "2",
            },
        )

    def test_b2_boundary_precondition_requires_the_existing_b2_resolver_seam(self) -> None:
        payload = ["family", "ACTIVE", "required", "definition"]
        theorem = theorem_record(
            preconditions=[
                {
                    "precondition_id": "b2",
                    "precondition_kind": "b2_boundary",
                    "payload": payload,
                }
            ]
        )
        observed = replace(
            member(),
            precondition_attestations_v1=[
                ["b2", payload, [["model", "a", ["whole_artifact", None], b"e" * 32]], "b2"]
            ],
        )
        with self.assertRaises(RelationApplicationV2SemanticValidationError) as raised:
            validate_relation_application_v2_semantics(
                observed, FakeSourceResolver(), theorem_record=theorem
            )
        self.assertEqual(raised.exception.code, "B2_PRECONDITION_RESOLUTION_REQUIRED")

    def test_b2_boundary_positive_path_uses_the_existing_resolver_adapter(self) -> None:
        payload = ["family", "ACTIVE", "required", "definition"]
        theorem = theorem_record(
            preconditions=[
                {
                    "precondition_id": "b2",
                    "precondition_kind": "b2_boundary",
                    "payload": payload,
                }
            ]
        )
        observed = replace(
            member(),
            precondition_attestations_v1=[
                ["b2", payload, [["model", "a", ["whole_artifact", None], b"e" * 32]], "b2"]
            ],
        )

        class FakeB2Resolver(FakeSourceResolver):
            def resolve_relation_application_v2_b2_boundary(self, observed_payload: object) -> None:
                if observed_payload != payload:
                    raise AssertionError("wrong B2 payload")

        result = validate_relation_application_v2_semantics(
            observed, FakeB2Resolver(), theorem_record=theorem
        )
        self.assertTrue(result.valid)

    def test_production_b2_adapter_executes_existing_source_resolver(self) -> None:
        payload = ["family", "ACTIVE", "required", "definition"]

        class ProductionB2SourceResolver(FakeSourceResolver):
            def resolve_b2_requirement_family(self, family_id: str, _bindings: object) -> object:
                self.family_id = family_id
                return SimpleNamespace(record={"status": "ACTIVE"})

            def resolve_b2_boundary(self, _family: object, boundary: object) -> None:
                if boundary.precise_semantic_definition != "definition":
                    raise AssertionError("wrong B2 boundary")

        source_resolver = ProductionB2SourceResolver()
        production = RelationApplicationV2Resolver(
            source_resolver,
            object(),
            {
                "schema": "manafold.m2.5.c.interaction-review-authority.v1",
                "source_bindings": [
                    {
                        "artifact_role": role,
                        "path": path,
                        "schema_or_null": schema,
                        "raw_sha256": "aa" * 32,
                    }
                    for role, path, schema in (
                        (
                            "b2_catalog",
                            "sources/m2_5/closures/B2/requirement_family_catalog.v1.json",
                            "manafold.m2.5.b2.requirement-family-catalog.v1",
                        ),
                        (
                            "b2_classifications",
                            "sources/m2_5/closures/B2/card_semantic_classifications.v1.json",
                            "manafold.m2.5.b2.card-semantic-classifications.v1",
                        ),
                        (
                            "b2_closure",
                            "sources/m2_5/closures/B2/classification_closure.v1.json",
                            "manafold.m2.5.b2.classification-closure.v1",
                        ),
                    )
                ],
            },
        )

        production.resolve_relation_application_v2_b2_boundary(payload)
        self.assertEqual(source_resolver.family_id, "family")

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
