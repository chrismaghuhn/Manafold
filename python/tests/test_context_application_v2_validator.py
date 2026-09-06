from __future__ import annotations

import base64
import hashlib
import json
import sys
import unittest
from copy import copy, deepcopy
from dataclasses import replace
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[2]
MODEL_PATH = "sources/m2_5/closures/C/declared_interaction_model.v2.json"
MODEL_SCHEMA = "manafold.m2.5.c.declared-interaction-model.v2"
BASE_PATH = "sources/m2_5/authorities/interaction_review_authority.v1.json"
BASE_SCHEMA = "manafold.m2.5.c.interaction-review-authority.v1"
CANDIDATE_PATH = "sources/m2_5/closures/C/interaction_candidate_universe.v2.json"
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "python" / "tests"))

from authority_source_resolver import ResolutionError
from context_application_v2_validator import (
    ContextApplicationV2InformationSensitivityInventory,
    ContextApplicationV2SemanticInput,
    ContextApplicationV2SemanticValidationError,
    ContextApplicationV2SemanticValidator,
    ContextPreconditionValueV1,
    collect_information_sensitivity_facts,
    validate_context_application_v2_semantics,
)
from mtgml.authority import (
    ACCEPTANCE_EVENT_SCHEMA_V1,
    AUTHORITY_SCHEMA_V1,
    REVIEWER_ROSTER_SCHEMA_V1,
    AcceptanceEvidenceRefV1,
    AcceptanceSubjectKind,
    AcceptanceSubjectPayloadV1,
    AuthorityIdentityKind,
    AuthorityIdentityV1,
    ContextApplicationMemberV2,
    ContextApplicationV2InputV1,
    ContextApplicationV2Record,
    ContextAuthoritySourceBindingV2,
    ContextBridgeRelationV2,
    ContextMemberBridgeAttestationV2,
    ContextSlotBridgeAttestationV2,
    DigestReferenceV1,
    EvidenceRefV1,
    ReviewAcceptanceEventInputV1,
    ReviewAcceptanceEventLeafV1,
    ReviewerRoleBindingV1,
    ReviewerRosterRefV1,
    ReviewEventRefV1,
    ReviewEventRefV3,
    ReviewMode,
    SourceBindingDigestV1,
    TemporalSlotAttestationV2,
    compute_authority_identity,
)
from mtgml.persistence import PersistenceValue, encode_canonical


class ContextApplicationV2SemanticCoreTests(unittest.TestCase):
    def test_information_sensitivity_inventory_uses_only_typed_values(self) -> None:
        base = self._case("exact_match")
        class_projection = [
            "binary",
            "directed",
            [],
            "cross_host",
            ["not_applicable", "private"] + ["not_applicable"] * 8,
            ["not_applicable"] * 4,
            [],
            [],
            [],
        ]
        typed = replace(
            base,
            historical_source_values=(
                "not_applicable",
                "private",
                *base.historical_source_values[2:],
            ),
            bridge_source_values=(
                "not_applicable",
                "private",
                *base.bridge_source_values[2:],
            ),
            bridge_reviewed_values=(
                "not_applicable",
                "private",
                *base.bridge_reviewed_values[2:],
            ),
            theorem_context_values=(
                "not_applicable",
                "private",
                *base.theorem_context_values[2:],
            ),
            theorem_preconditions=(
                ContextPreconditionValueV1(
                    "free-label",
                    ["visibility", "private"],
                ),
                ContextPreconditionValueV1("another-label", class_projection),
            ),
        )
        inventory = collect_information_sensitivity_facts(typed)
        self.assertIsInstance(inventory, ContextApplicationV2InformationSensitivityInventory)
        self.assertIn("historical_source.visibility", inventory.fact_paths)
        self.assertIn("bridge.reviewed.visibility", inventory.fact_paths)
        self.assertIn("theorem.context.visibility", inventory.fact_paths)
        self.assertIn("precondition[free-label].source_context.visibility", inventory.fact_paths)
        self.assertIn(
            "precondition[another-label].class_projection.visibility",
            inventory.fact_paths,
        )

        relabeled = replace(
            typed,
            theorem_preconditions=(
                ContextPreconditionValueV1("card-name", ["visibility", "private"]),
                ContextPreconditionValueV1("rationale-text", class_projection),
            ),
        )
        relabeled_inventory = collect_information_sensitivity_facts(relabeled)
        self.assertEqual(len(relabeled_inventory.fact_paths), len(inventory.fact_paths))

    def test_exact_match_control_is_accepted(self) -> None:
        result = validate_context_application_v2_semantics(self._case("exact_match"))
        self.assertTrue(result.valid)
        self.assertIsNone(result.error_code)

    def test_reviewed_divergence_control_is_accepted_without_v1_source_equality(self) -> None:
        result = validate_context_application_v2_semantics(self._case("reviewed_divergence"))
        self.assertTrue(result.valid)
        self.assertIsNone(result.error_code)

    def test_every_negative_matrix_case_returns_its_declared_error_code(self) -> None:
        for raw in cast(list[dict[str, object]], self._matrix()["cases"]):
            expected = cast(dict[str, object], raw["expected"])
            if expected["valid"]:
                continue
            with self.subTest(case_id=raw["case_id"]):
                with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
                    validate_context_application_v2_semantics(self._case(cast(str, raw["case_id"])))
                self.assertEqual(error.exception.code, expected["error_code"])

    def _matrix(self) -> dict[str, object]:
        return cast(
            dict[str, object],
            json.loads(
                (
                    ROOT
                    / "conformance/fixtures/authority/"
                    / "context_application_v2_semantic_golden_matrix.v1.json"
                ).read_text(encoding="utf-8")
            ),
        )

    def _case(self, case_id: str) -> ContextApplicationV2SemanticInput:
        cases = cast(list[dict[str, object]], self._matrix()["cases"])
        raw = next(case for case in cases if case["case_id"] == case_id)
        return ContextApplicationV2SemanticInput(
            theorem_subject_shape=cast(PersistenceValue, raw["theorem_subject_shape"]),
            member_context_binding=cast(PersistenceValue, raw["member_context_binding"]),
            historical_source_values=tuple(cast(list[str], raw["historical_source_values"])),
            bridge_source_values=tuple(cast(list[str], raw["bridge_source_values"])),
            theorem_context_values=tuple(cast(list[str], raw["theorem_context_values"])),
            bridge_reviewed_values=tuple(cast(list[str], raw["bridge_reviewed_values"])),
            bridge_relations=tuple(
                ContextBridgeRelationV2(value) for value in cast(list[str], raw["bridge_relations"])
            ),
            theorem_temporal_values=tuple(cast(list[str], raw["theorem_temporal_values"])),
            bridge_temporal_values=tuple(cast(list[str], raw["bridge_temporal_values"])),
            theorem_preconditions=tuple(
                ContextPreconditionValueV1(
                    value["precondition_id"],
                    cast(PersistenceValue, value["payload"]),
                )
                for value in cast(list[dict[str, object]], raw["theorem_preconditions"])
            ),
            member_preconditions=tuple(
                ContextPreconditionValueV1(
                    value["precondition_id"],
                    cast(PersistenceValue, value["observed_value"]),
                )
                for value in cast(list[dict[str, object]], raw["member_preconditions"])
            ),
        )


class ContextApplicationV2IntegrationTests(unittest.TestCase):
    def test_exact_match_record_passes(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        validator = ContextApplicationV2SemanticValidator(
            case["source_resolver"],
            base_authority_binding=case["base_binding"],
        )
        result = validator.validate(case["record"])
        self.assertTrue(result.valid)
        self.assertEqual(result.member_count, 1)

    def test_reviewed_divergence_keeps_v1_source_context_precondition_historical(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="activation_time",
            precondition_value="not_applicable",
        )
        validator = ContextApplicationV2SemanticValidator(
            case["source_resolver"],
            base_authority_binding=case["base_binding"],
        )
        result = validator.validate(case["record"])
        self.assertTrue(result.valid)
        self.assertEqual(result.member_count, 1)

    def test_temporal_semantic_precondition_passes_without_source_temporal_fact(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
            precondition_kind="temporal_semantic",
            precondition_id="temporal-source",
            precondition_payload=["trigger_order", "not_applicable"],
        )
        result = self._validator_for(case).validate(case["record"])
        self.assertTrue(result.valid)

    def test_class_projection_precondition_keeps_v2_member_fail_closed_without_proof(self) -> None:
        projection = [
            "binary",
            "directed",
            [
                [0, "ordered_participant", "requirement_family", "family.a"],
                [1, "ordered_participant", "requirement_family", "family.b"],
            ],
            "cross_host",
            ["not_applicable"] * 10,
            ["not_applicable"] * 4,
            [],
            [],
            [],
        ]
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
            precondition_kind="class_projection",
            precondition_id="class-projection",
            precondition_payload=projection,
        )
        with self.assertRaises(ResolutionError) as error:
            self._validator_for(case).validate(case["record"])
        self.assertEqual(error.exception.code, "CLASS_PROJECTION_PRECONDITION_PROOF_MISSING")

    def test_input_entrypoint_validates_without_v3_record_wrapper(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        record = case["record"]
        input_value = ContextApplicationV2InputV1(
            theorem_record_id_bytes=record.theorem_record_id.digest_bytes,
            members=record.members,
        )
        validator = ContextApplicationV2SemanticValidator(
            case["source_resolver"],
            base_authority_binding=case["base_binding"],
        )
        result = validator.validate(input_value)
        self.assertTrue(result.valid)
        self.assertEqual(result.member_count, 1)

    def test_application_identity_is_recomputed_from_members(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        record = case["record"]
        wrong_application_id = AuthorityIdentityV1(
            AuthorityIdentityKind.CONTEXT_APPLICATION_V2,
            bytes.fromhex("99" * 32),
        )
        tampered = ContextApplicationV2Record.from_parts(
            application_id=wrong_application_id,
            theorem_record_id=record.theorem_record_id,
            members=record.members,
            review_event_ref_v3=record.review_event_ref_v3,
        )
        validator = ContextApplicationV2SemanticValidator(
            case["source_resolver"],
            base_authority_binding=case["base_binding"],
        )
        with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
            validator.validate(tampered)
        self.assertEqual(error.exception.code, "APPLICATION_IDENTITY_MISMATCH")

    def test_record_identity_is_recomputed_from_v3_reference_structure(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        tampered = copy(case["record"])
        object.__setattr__(
            tampered,
            "record_id",
            AuthorityIdentityV1(
                AuthorityIdentityKind.CONTEXT_APPLICATION_RECORD_V2,
                bytes.fromhex("88" * 32),
            ),
        )
        validator = ContextApplicationV2SemanticValidator(
            case["source_resolver"],
            base_authority_binding=case["base_binding"],
        )
        with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
            validator.validate(tampered)
        self.assertEqual(error.exception.code, "RECORD_IDENTITY_MISMATCH")

    def test_nested_source_instance_evidence_uses_parent_ownership(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        fixture = case["fixture"]
        raw = (fixture.repo / Path(*CANDIDATE_PATH.split("/"))).read_bytes()
        evidence = EvidenceRefV1(
            "c_candidate",
            CANDIDATE_PATH,
            ("json_pointer", "/source_instances/0/source_context/timing"),
            hashlib.sha256(raw).digest(),
        )
        member = replace(case["member"], member_evidence_refs=(evidence,))
        application_id = ContextApplicationV2InputV1(
            theorem_record_id_bytes=case["record"].theorem_record_id.digest_bytes,
            members=(member,),
        ).identity()
        record = ContextApplicationV2Record.from_parts(
            application_id=application_id,
            theorem_record_id=case["record"].theorem_record_id,
            members=(member,),
            review_event_ref_v3=case["record"].review_event_ref_v3,
        )
        validator = ContextApplicationV2SemanticValidator(
            case["source_resolver"],
            base_authority_binding=case["base_binding"],
        )
        result = validator.validate(record)
        self.assertTrue(result.valid)

    def test_out_of_range_source_instance_evidence_does_not_become_global(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        fixture = case["fixture"]
        raw = (fixture.repo / Path(*CANDIDATE_PATH.split("/"))).read_bytes()
        evidence = EvidenceRefV1(
            "c_candidate",
            CANDIDATE_PATH,
            ("json_pointer", "/source_instances/99/source_context/timing"),
            hashlib.sha256(raw).digest(),
        )
        member = replace(case["member"], member_evidence_refs=(evidence,))
        application_id = ContextApplicationV2InputV1(
            theorem_record_id_bytes=case["record"].theorem_record_id.digest_bytes,
            members=(member,),
        ).identity()
        record = ContextApplicationV2Record.from_parts(
            application_id=application_id,
            theorem_record_id=case["record"].theorem_record_id,
            members=(member,),
            review_event_ref_v3=case["record"].review_event_ref_v3,
        )
        validator = ContextApplicationV2SemanticValidator(
            case["source_resolver"],
            base_authority_binding=case["base_binding"],
        )
        with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
            validator.validate(record)
        self.assertEqual(error.exception.code, "EVIDENCE_RESOLUTION_FAILURE")

    def test_cross_candidate_and_source_instance_nested_evidence_is_rejected(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
            two_candidates=True,
        )
        fixture = case["fixture"]
        raw = (fixture.repo / Path(*CANDIDATE_PATH.split("/"))).read_bytes()
        validator = ContextApplicationV2SemanticValidator(
            case["source_resolver"],
            base_authority_binding=case["base_binding"],
        )
        for pointer in (
            "/candidates/1/candidate_id",
            "/source_instances/1/source_context/timing",
        ):
            with self.subTest(pointer=pointer):
                evidence = EvidenceRefV1(
                    "c_candidate",
                    CANDIDATE_PATH,
                    ("json_pointer", pointer),
                    hashlib.sha256(raw).digest(),
                )
                member = replace(case["member"], member_evidence_refs=(evidence,))
                application_id = ContextApplicationV2InputV1(
                    theorem_record_id_bytes=case["record"].theorem_record_id.digest_bytes,
                    members=(member,),
                ).identity()
                record = ContextApplicationV2Record.from_parts(
                    application_id=application_id,
                    theorem_record_id=case["record"].theorem_record_id,
                    members=(member,),
                    review_event_ref_v3=case["record"].review_event_ref_v3,
                )
                with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
                    validator.validate(record)
                self.assertEqual(error.exception.code, "EVIDENCE_SOURCE_SUBSTITUTION")

    def test_malformed_local_parent_identity_is_evidence_resolution_failure(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        validator = self._validator_for(case)
        raw = (case["fixture"].repo / Path(*CANDIDATE_PATH.split("/"))).read_bytes()
        candidate_reference = EvidenceRefV1(
            "c_candidate",
            CANDIDATE_PATH,
            ("json_pointer", "/candidates/0/candidate_id"),
            hashlib.sha256(raw).digest(),
        )
        source_reference = EvidenceRefV1(
            "c_candidate",
            CANDIDATE_PATH,
            ("json_pointer", "/source_instances/0/source_context/timing"),
            hashlib.sha256(raw).digest(),
        )
        mutations = (
            ("candidate missing identity", candidate_reference, "candidates", "candidate_identity"),
            (
                "candidate malformed identity",
                candidate_reference,
                "candidates",
                "candidate_identity",
            ),
            ("source missing identity", source_reference, "source_instances", "source_instance_id"),
            ("source malformed identity", source_reference, "source_instances", "candidate_id"),
        )
        for name, reference, root, field in mutations:
            with self.subTest(mutation=name):
                resolved = validator._v2_resolver.resolve_evidence(reference)
                universe = deepcopy(resolved.artifact.json_value)
                parent = universe[root][0]
                if name.endswith("missing identity"):
                    parent.pop(field)
                elif root == "candidates":
                    parent[field] = "malformed"
                else:
                    parent[field] = 7
                malformed_artifact = replace(resolved.artifact, json_value=universe)
                malformed_evidence = replace(resolved, artifact=malformed_artifact)
                with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
                    validator._validate_evidence_parent_owner(
                        reference,
                        malformed_evidence,
                        case["member"],
                        "members[0].evidence",
                    )
                self.assertEqual(error.exception.code, "EVIDENCE_RESOLUTION_FAILURE")

    def _record_for_member(
        self,
        case: dict[str, object],
        member: ContextApplicationMemberV2,
    ) -> ContextApplicationV2Record:
        original = case["record"]
        application_id = ContextApplicationV2InputV1(
            theorem_record_id_bytes=original.theorem_record_id.digest_bytes,
            members=(member,),
        ).identity()
        return ContextApplicationV2Record.from_parts(
            application_id=application_id,
            theorem_record_id=original.theorem_record_id,
            members=(member,),
            review_event_ref_v3=original.review_event_ref_v3,
        )

    def _validator_for(self, case: dict[str, object]) -> ContextApplicationV2SemanticValidator:
        return ContextApplicationV2SemanticValidator(
            case["source_resolver"],
            base_authority_binding=case["base_binding"],
        )

    def test_member_identity_and_bridge_negative_matrix(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        member = case["member"]
        record = case["record"]
        validator = self._validator_for(case)

        tampered_theorem = copy(record)
        object.__setattr__(
            tampered_theorem,
            "theorem_record_id",
            AuthorityIdentityV1(
                AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
                bytes.fromhex("77" * 32),
            ),
        )
        with (
            self.subTest(mutation="wrong theorem record"),
            self.assertRaises(ResolutionError) as error,
        ):
            validator.validate(tampered_theorem)
            self.assertEqual(error.exception.code, "THEOREM_REFERENCE_INVALID")

        mutations = [
            (
                "wrong candidate id",
                replace(member, candidate_id="CROSS_DECK|unknown"),
                "CANDIDATE_BINDING_MISMATCH",
            ),
            (
                "wrong candidate identity",
                replace(
                    member,
                    candidate_identity_digest_reference=replace(
                        member.candidate_identity_digest_reference,
                        digest_bytes=bytes.fromhex("66" * 32),
                    ),
                ),
                "CANDIDATE_IDENTITY_MISMATCH",
            ),
            (
                "wrong source instance",
                replace(member, source_instance_id="si.v1/unknown/0"),
                "SOURCE_INSTANCE_BINDING_MISMATCH",
            ),
            (
                "wrong candidate-universe digest",
                replace(
                    member,
                    candidate_universe_binding=[
                        member.candidate_universe_binding[0],
                        member.candidate_universe_binding[1],
                        bytes.fromhex("55" * 32),
                    ],
                ),
                "SOURCE_DIGEST_MISMATCH",
            ),
            (
                "context subject mismatch",
                replace(
                    member,
                    context_binding_v1=[
                        "binary",
                        "directed",
                        member.context_binding_v1[2],
                        "same_host",
                    ],
                ),
                "MEMBER_SUBJECT_MISMATCH",
            ),
        ]
        for name, mutated_member, expected_code in mutations:
            with self.subTest(mutation=name):
                with self.assertRaises(
                    (ContextApplicationV2SemanticValidationError, ResolutionError)
                ) as error:
                    validator.validate(self._record_for_member(case, mutated_member))
                self.assertEqual(error.exception.code, expected_code)

        bridge = member.context_member_bridge_attestation_v2
        timing_slot = bridge.context[2]
        reviewed_mismatch = replace(
            timing_slot,
            reviewed_value="activation_time",
            relation=ContextBridgeRelationV2.REVIEWED_DIVERGENCE,
        )
        reviewed_bridge = replace(
            bridge,
            context=(bridge.context[0], bridge.context[1], reviewed_mismatch, *bridge.context[3:]),
        )
        with self.subTest(mutation="reviewed context mismatch"):
            with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
                validator.validate(
                    self._record_for_member(
                        case, replace(member, context_member_bridge_attestation_v2=reviewed_bridge)
                    )
                )
            self.assertEqual(error.exception.code, "MEMBER_REVIEWED_CONTEXT_MISMATCH")

        relation_mismatch = replace(
            timing_slot, relation=ContextBridgeRelationV2.REVIEWED_DIVERGENCE
        )
        relation_bridge = replace(
            bridge,
            context=(bridge.context[0], bridge.context[1], relation_mismatch, *bridge.context[3:]),
        )
        with self.subTest(mutation="equal values with reviewed divergence"):
            with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
                validator.validate(
                    self._record_for_member(
                        case, replace(member, context_member_bridge_attestation_v2=relation_bridge)
                    )
                )
            self.assertEqual(error.exception.code, "BRIDGE_RELATION_MISMATCH")

    def test_precondition_attestation_matrix_preserves_v1_source_checks(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="activation_time",
            precondition_value="not_applicable",
        )
        member = case["member"]
        validator = self._validator_for(case)
        original_attestation = list(member.precondition_attestations_v1[0])
        mutations = [
            ("missing", replace(member, precondition_attestations_v1=[]), "PRECONDITION_COVERAGE"),
            (
                "wrong id",
                replace(
                    member,
                    precondition_attestations_v1=[["wrong-id", *original_attestation[1:]]],
                ),
                "PRECONDITION_MISMATCH",
            ),
            (
                "wrong observed value",
                replace(
                    member,
                    precondition_attestations_v1=[
                        [
                            original_attestation[0],
                            ["timing", "activation_time"],
                            *original_attestation[2:],
                        ]
                    ],
                ),
                "PRECONDITION_MISMATCH",
            ),
            (
                "extra",
                replace(
                    member,
                    precondition_attestations_v1=[
                        original_attestation,
                        [
                            "timing-source-extra",
                            ["timing", "not_applicable"],
                            [member.member_evidence_refs[0].to_cbor()],
                            "extra",
                        ],
                    ],
                ),
                "PRECONDITION_COVERAGE",
            ),
        ]
        for name, mutated_member, expected_code in mutations:
            with self.subTest(mutation=name):
                with self.assertRaises(ResolutionError) as error:
                    validator.validate(self._record_for_member(case, mutated_member))
                self.assertEqual(error.exception.code, expected_code)

        source_case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="activation_time",
            precondition_value="activation_time",
        )
        with self.assertRaises(ResolutionError) as error:
            self._validator_for(source_case).validate(source_case["record"])
        self.assertEqual(error.exception.code, "PRECONDITION_SOURCE_MISMATCH")

    def test_stale_and_unresolved_evidence_fail_closed(self) -> None:
        case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        member = case["member"]
        bad_member_evidence = replace(
            member.member_evidence_refs[0],
            raw_sha256=bytes.fromhex("12" * 32),
        )
        bad_member = replace(member, member_evidence_refs=(bad_member_evidence,))
        with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
            self._validator_for(case).validate(self._record_for_member(case, bad_member))
        self.assertEqual(error.exception.code, "EVIDENCE_RESOLUTION_FAILURE")

        bridge = member.context_member_bridge_attestation_v2
        stale_context_ref = replace(
            bridge.context[0].evidence_refs[0],
            raw_sha256=bytes.fromhex("13" * 32),
        )
        stale_context_slot = replace(bridge.context[0], evidence_refs=(stale_context_ref,))
        stale_context_bridge = replace(
            bridge,
            context=(stale_context_slot, *bridge.context[1:]),
        )
        stale_context_member = replace(
            member,
            context_member_bridge_attestation_v2=stale_context_bridge,
        )
        with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
            self._validator_for(case).validate(self._record_for_member(case, stale_context_member))
        self.assertEqual(error.exception.code, "EVIDENCE_RESOLUTION_FAILURE")

        stale_temporal_ref = replace(
            bridge.temporal[0].evidence_refs[0],
            raw_sha256=bytes.fromhex("14" * 32),
        )
        stale_temporal_slot = replace(bridge.temporal[0], evidence_refs=(stale_temporal_ref,))
        stale_temporal_bridge = replace(
            bridge,
            temporal=(stale_temporal_slot, *bridge.temporal[1:]),
        )
        stale_temporal_member = replace(
            member,
            context_member_bridge_attestation_v2=stale_temporal_bridge,
        )
        with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
            self._validator_for(case).validate(self._record_for_member(case, stale_temporal_member))
        self.assertEqual(error.exception.code, "EVIDENCE_RESOLUTION_FAILURE")

        unresolved_ref = replace(
            member.member_evidence_refs[0],
            locator=("json_pointer", "/missing"),
        )
        unresolved_member = replace(member, member_evidence_refs=(unresolved_ref,))
        with self.assertRaises(ContextApplicationV2SemanticValidationError) as error:
            self._validator_for(case).validate(self._record_for_member(case, unresolved_member))
        self.assertEqual(error.exception.code, "EVIDENCE_RESOLUTION_FAILURE")

    def test_rejected_v2_applications_preserve_inputs_and_source_files(self) -> None:
        base = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        bridge = base["member"].context_member_bridge_attestation_v2
        timing_slot = bridge.context[2]
        bridge_mismatch = replace(
            bridge,
            context=(
                bridge.context[0],
                bridge.context[1],
                replace(
                    timing_slot,
                    reviewed_value="activation_time",
                    relation=ContextBridgeRelationV2.REVIEWED_DIVERGENCE,
                ),
                *bridge.context[3:],
            ),
        )
        precondition_case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="activation_time",
            precondition_value="not_applicable",
        )
        bad_precondition = replace(
            precondition_case["member"],
            precondition_attestations_v1=[
                [
                    "timing-source",
                    ["timing", "activation_time"],
                    [precondition_case["member"].member_evidence_refs[0].to_cbor()],
                    "mutated",
                ]
            ],
        )
        evidence_case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        stale_evidence = replace(
            evidence_case["member"].member_evidence_refs[0],
            raw_sha256=bytes.fromhex("12" * 32),
        )
        substitution_case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
            two_candidates=True,
        )
        substitution_raw = (
            substitution_case["fixture"].repo / Path(*CANDIDATE_PATH.split("/"))
        ).read_bytes()
        substitution_evidence = EvidenceRefV1(
            "c_candidate",
            CANDIDATE_PATH,
            ("json_pointer", "/candidates/1/candidate_id"),
            hashlib.sha256(substitution_raw).digest(),
        )
        identity_case = self._synthetic_case(
            source_timing="not_applicable",
            reviewed_timing="not_applicable",
        )
        wrong_application_id = copy(identity_case["record"])
        object.__setattr__(
            wrong_application_id,
            "application_id",
            AuthorityIdentityV1(
                AuthorityIdentityKind.CONTEXT_APPLICATION_V2,
                bytes.fromhex("99" * 32),
            ),
        )
        rejected = (
            (
                base,
                self._record_for_member(base, replace(base["member"], candidate_id="unknown")),
                "CANDIDATE_BINDING_MISMATCH",
            ),
            (
                base,
                self._record_for_member(
                    base,
                    replace(base["member"], context_member_bridge_attestation_v2=bridge_mismatch),
                ),
                "MEMBER_REVIEWED_CONTEXT_MISMATCH",
            ),
            (
                precondition_case,
                self._record_for_member(precondition_case, bad_precondition),
                "PRECONDITION_MISMATCH",
            ),
            (
                evidence_case,
                self._record_for_member(
                    evidence_case,
                    replace(evidence_case["member"], member_evidence_refs=(stale_evidence,)),
                ),
                "EVIDENCE_RESOLUTION_FAILURE",
            ),
            (
                substitution_case,
                self._record_for_member(
                    substitution_case,
                    replace(
                        substitution_case["member"], member_evidence_refs=(substitution_evidence,)
                    ),
                ),
                "EVIDENCE_SOURCE_SUBSTITUTION",
            ),
            (identity_case, wrong_application_id, "APPLICATION_IDENTITY_MISMATCH"),
        )
        for index, (case, record, expected_code) in enumerate(rejected):
            with self.subTest(rejection=index):
                before = deepcopy(record)
                before_files = self._file_digests(case["fixture"])
                with self.assertRaises(
                    (ContextApplicationV2SemanticValidationError, ResolutionError)
                ) as error:
                    self._validator_for(case).validate(record)
                self.assertEqual(error.exception.code, expected_code)
                self.assertEqual(record, before)
                self.assertEqual(self._file_digests(case["fixture"]), before_files)

    @staticmethod
    def _file_digests(fixture: object) -> dict[str, str]:
        repo = cast(Path, fixture.repo)
        return {
            path.relative_to(repo).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in repo.rglob("*")
            if path.is_file()
        }

    def _synthetic_case(
        self,
        *,
        source_timing: str,
        reviewed_timing: str,
        source_visibility: str = "not_applicable",
        reviewed_visibility: str | None = None,
        precondition_value: str | None = None,
        precondition_kind: str = "source_context",
        precondition_id: str = "timing-source",
        precondition_payload: list[object] | None = None,
        precondition_observed_value: list[object] | None = None,
        two_candidates: bool = False,
    ) -> dict[str, object]:
        from test_authority_source_resolver import (
            REV3_CENSUS_MEMBER,
            REV3_SOURCE_COLUMNS,
            AuthoritySourceResolverTests,
            digest,
            json_bytes,
        )

        fixture = AuthoritySourceResolverTests()
        fixture.setUp()
        self.addCleanup(fixture.tearDown)
        source_context = {
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
        }
        source_context["timing"] = source_timing
        source_context["visibility"] = source_visibility
        rows: list[list[str]] | None = None
        extra_candidates: list[dict[str, object]] = []
        extra_instances: list[dict[str, object]] = []
        if two_candidates:
            second_candidate_id = "CROSS_DECK|P2|family.a|family.b|DIRECTIONAL_BINARY"
            rows = [
                fixture._source_values(),
                fixture._source_values(second_candidate_id),
            ]
            census_raw = fixture._census_bytes(rows)
            second_source_binding = {
                "kind": "rev3",
                "archive_member": REV3_CENSUS_MEMBER,
                "archive_member_sha256": digest(census_raw),
                "row_ordinal": 1,
                "source_columns": list(REV3_SOURCE_COLUMNS),
                "source_values": rows[1],
            }
            extra_candidates = [
                fixture._candidate_record(
                    candidate_id=second_candidate_id,
                    source_binding=second_source_binding,
                )
            ]
            extra_instances = [
                fixture._source_instance_record(
                    candidate_id=second_candidate_id,
                    source_instance_id=(
                        "si.v1/"
                        + base64.urlsafe_b64encode(second_candidate_id.encode("utf-8"))
                        .decode("ascii")
                        .rstrip("=")
                        + "/0"
                    ),
                    source_binding=second_source_binding,
                )
            ]
        source_resolver, _old_candidate_binding, candidate, instance = (
            fixture._synthetic_binding_fixture(
                rows=rows,
                extra_candidates=extra_candidates,
                extra_instances=extra_instances,
                instance_overrides={"source_context": source_context},
            )
        )

        model_raw = (ROOT / MODEL_PATH).read_bytes()
        fixture.write_repo(MODEL_PATH, model_raw)
        universe_path = fixture.repo / Path(
            *["sources", "m2_5", "closures", "C", "interaction_candidate_universe.v2.json"]
        )
        universe = cast(dict[str, object], json.loads(universe_path.read_text(encoding="utf-8")))
        input_bindings = cast(dict[str, object], universe["input_bindings"])
        declared_model = cast(dict[str, object], input_bindings["declared_model"])
        declared_model["raw_sha256"] = digest(model_raw)
        universe_raw = json_bytes(universe)
        universe_path.write_bytes(universe_raw)
        candidate_binding = SourceBindingDigestV1(
            "candidate_universe",
            "sources/m2_5/closures/C/interaction_candidate_universe.v2.json",
            "manafold.m2.5.c.interaction-candidate-universe.v2",
            bytes.fromhex(digest(universe_raw)),
        )

        model_document = cast(dict[str, object], json.loads(model_raw.decode("utf-8")))
        model_binding = SourceBindingDigestV1(
            "declared_model",
            MODEL_PATH,
            MODEL_SCHEMA,
            bytes.fromhex(digest(model_raw)),
        )
        reviewer_roles = (
            (
                "architecture_maintainer",
                "information_safety_reviewer",
                "rules_authority_maintainer",
            )
            if source_visibility != "not_applicable"
            else (
                "architecture_maintainer",
                "rules_authority_maintainer",
            )
        )
        roster_raw = json_bytes(
            {
                "schema": REVIEWER_ROSTER_SCHEMA_V1,
                "reviewers": [
                    {
                        "reviewer_id": "alice",
                        "roles": list(reviewer_roles),
                    }
                ],
            }
        )
        roster_path_text = (
            "sources/m2_5/authorities/reviewer_rosters/v1/" + digest(roster_raw) + ".json"
        )
        fixture.write_repo(roster_path_text, roster_raw)
        roster_digest = bytes.fromhex(digest(roster_raw))
        roster_ref = ReviewerRosterRefV1(
            roster_path_text,
            REVIEWER_ROSTER_SCHEMA_V1,
            roster_digest,
        )
        roster_binding = SourceBindingDigestV1(
            "reviewer_roster_leaf",
            roster_path_text,
            REVIEWER_ROSTER_SCHEMA_V1,
            roster_digest,
        )
        review_raw = b"accepted synthetic context theorem evidence\n"
        review_path_text = "docs/review/context-application-v2-slice3.md"
        fixture.write_repo(review_path_text, review_raw)
        review_evidence = AcceptanceEvidenceRefV1(
            review_path_text,
            bytes.fromhex(digest(review_raw)),
            ("whole_artifact", None),
        )
        source_evidence = EvidenceRefV1(
            "model",
            MODEL_PATH,
            ("whole_artifact", None),
            bytes.fromhex(digest(model_raw)),
        )

        candidate_refs = cast(list[dict[str, str]], candidate["participant_refs"])
        participant_arrays = [
            [
                index,
                "ordered_participant",
                reference["participant_kind"],
                reference["semantic_ref"],
            ]
            for index, reference in enumerate(candidate_refs)
        ]
        subject_shape = ["binary", "directed", participant_arrays, "cross_host"]
        context_values = ["not_applicable"] * 10
        context_values[1] = (
            source_visibility if reviewed_visibility is None else reviewed_visibility
        )
        context_values[2] = reviewed_timing
        temporal_values = ["not_applicable"] * 4
        precondition_arrays: list[list[object]] = []
        precondition_wire: list[dict[str, object]] = []
        member_preconditions: list[list[object]] = []
        if precondition_value is not None or precondition_payload is not None:
            payload = precondition_payload or ["timing", precondition_value]
            observed_value = precondition_observed_value or payload
            precondition_arrays = [
                [
                    precondition_id,
                    [precondition_kind, payload],
                ]
            ]
            precondition_wire = [
                {
                    "precondition_id": precondition_id,
                    "precondition_kind": precondition_kind,
                    "payload": payload,
                }
            ]
            member_preconditions = [
                [
                    precondition_id,
                    observed_value,
                    [source_evidence.to_cbor()],
                    f"synthetic {precondition_kind} precondition",
                ]
            ]
        theorem_id = compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_THEOREM,
            [
                "manafold.m2.5.c.context-proof-input.v1",
                cast(str, model_document["model_id"]),
                subject_shape,
                context_values,
                temporal_values,
                precondition_arrays,
                [],
                [],
            ],
        )
        rationale = "synthetic accepted context theorem"
        acceptance_subject = AcceptanceSubjectPayloadV1(
            AcceptanceSubjectKind.CONTEXT_THEOREM_RECORD,
            [theorem_id.digest_bytes, [source_evidence.to_cbor()], rationale],
        )
        event_input = ReviewAcceptanceEventInputV1(
            subject_kind=AcceptanceSubjectKind.CONTEXT_THEOREM_RECORD,
            subject_payload_digest=acceptance_subject.identity().digest_bytes,
            reviewer_roster_ref=roster_ref,
            reviewer_role_bindings=(
                ReviewerRoleBindingV1(
                    "alice",
                    reviewer_roles,
                ),
            ),
            review_mode=ReviewMode.SOLO_SEPARATE_SELF_REVIEW,
            source_binding_digests=tuple(
                sorted(
                    (model_binding, roster_binding),
                    key=lambda item: encode_canonical(item.to_cbor()),
                )
            ),
            review_evidence_refs=(review_evidence,),
        )
        event_leaf = ReviewAcceptanceEventLeafV1.from_input(event_input)
        event_raw = json_bytes(event_leaf.to_wire())
        event_path_text = (
            "sources/m2_5/authorities/review_acceptance_events/v1/"
            + event_leaf.event_id.as_text().removeprefix("ae.v1/")
            + ".json"
        )
        fixture.write_repo(event_path_text, event_raw)
        event_ref = ReviewEventRefV1(
            event_path_text,
            bytes.fromhex(digest(event_raw)),
            event_leaf.event_id.as_text(),
        )
        event_binding = SourceBindingDigestV1(
            "acceptance_event_leaf",
            event_path_text,
            ACCEPTANCE_EVENT_SCHEMA_V1,
            bytes.fromhex(digest(event_raw)),
        )
        theorem_record_id = compute_authority_identity(
            AuthorityIdentityKind.CONTEXT_THEOREM_RECORD,
            [
                "manafold.m2.5.c.context-proof-record-input.v1",
                theorem_id.digest_bytes,
                [source_evidence.to_cbor()],
                event_ref.to_cbor(),
                rationale,
            ],
        )
        theorem = {
            "theorem_id": {
                "envelope_id": "mtgml.digest-envelope.v1",
                "algorithm_id": "sha-256",
                "semantic_domain": theorem_id.semantic_domain,
                "payload_codec_id": "mtgml.canonical-cbor.v1",
                "input_schema_id": theorem_id.input_schema_id,
                "digest_hex": theorem_id.digest_bytes.hex(),
            },
            "record_id": {
                "envelope_id": "mtgml.digest-envelope.v1",
                "algorithm_id": "sha-256",
                "semantic_domain": theorem_record_id.semantic_domain,
                "payload_codec_id": "mtgml.canonical-cbor.v1",
                "input_schema_id": theorem_record_id.input_schema_id,
                "digest_hex": theorem_record_id.digest_bytes.hex(),
            },
            "subject_shape": {
                "arity": "binary",
                "directionality": "directed",
                "participant_roles": [
                    {
                        "position": item[0],
                        "role": item[1],
                        "participant_kind": item[2],
                        "semantic_ref": item[3],
                    }
                    for item in participant_arrays
                ],
                "host_relationship": "cross_host",
            },
            "context_dimensions": context_values,
            "temporal_semantics": temporal_values,
            "preconditions": precondition_wire,
            "b2_boundary_refs": [],
            "b1_final_citation_refs": [],
            "source_evidence_refs": [source_evidence.to_wire()],
            "semantic_rationale": rationale,
            "acceptance": {
                "decision": "human_accepted",
                "review_event_ref": event_ref.to_wire(),
            },
        }
        base_document = {
            "schema": AUTHORITY_SCHEMA_V1,
            "model_binding": {
                "path": MODEL_PATH,
                "raw_sha256": digest(model_raw),
                "model_id": model_document["model_id"],
                "model_version": model_document["model_version"],
            },
            "source_bindings": [
                {
                    "authority_kind": "model",
                    "artifact_role": model_binding.artifact_role,
                    "path": model_binding.path,
                    "schema_or_null": model_binding.schema_or_null,
                    "raw_sha256": model_binding.raw_sha256.hex(),
                },
                {
                    "authority_kind": "reviewer_roster",
                    "artifact_role": roster_binding.artifact_role,
                    "path": roster_binding.path,
                    "schema_or_null": roster_binding.schema_or_null,
                    "raw_sha256": roster_binding.raw_sha256.hex(),
                },
                {
                    "authority_kind": "acceptance_event",
                    "artifact_role": event_binding.artifact_role,
                    "path": event_binding.path,
                    "schema_or_null": event_binding.schema_or_null,
                    "raw_sha256": event_binding.raw_sha256.hex(),
                },
            ],
            "relation_proofs": [],
            "relation_applications": [],
            "domain_proofs": [],
            "domain_applications": [],
            "context_proofs": [theorem],
            "context_applications": [],
            "supersession_records": [],
        }
        base_raw = json_bytes(base_document)
        fixture.write_repo(BASE_PATH, base_raw)
        base_binding = ContextAuthoritySourceBindingV2(
            "base_authority_v1",
            BASE_PATH,
            BASE_SCHEMA,
            bytes.fromhex(digest(base_raw)),
        )

        model_evidence = EvidenceRefV1(
            "model",
            MODEL_PATH,
            ("whole_artifact", None),
            bytes.fromhex(digest(model_raw)),
        )
        candidate_identity = cast(dict[str, str], candidate["candidate_identity"])
        context = tuple(
            ContextSlotBridgeAttestationV2(
                slot_name=name,
                source_value=source_context[name],
                reviewed_value=context_values[index],
                relation=(
                    ContextBridgeRelationV2.EXACT_MATCH
                    if source_context[name] == context_values[index]
                    else ContextBridgeRelationV2.REVIEWED_DIVERGENCE
                ),
                evidence_refs=(model_evidence,),
                rationale="synthetic reviewed bridge",
            )
            for index, name in enumerate(
                (
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
            )
        )
        temporal = tuple(
            TemporalSlotAttestationV2(
                slot_name=name,
                reviewed_value="not_applicable",
                evidence_refs=(model_evidence,),
                rationale="synthetic temporal bridge",
            )
            for name in (
                "trigger_order",
                "dependency_order",
                "duration",
                "replacement_order",
            )
        )
        member = ContextApplicationMemberV2(
            candidate_id=cast(str, candidate["candidate_id"]),
            candidate_identity_digest_reference=DigestReferenceV1(
                candidate_identity["envelope_id"],
                candidate_identity["algorithm_id"],
                candidate_identity["semantic_domain"],
                candidate_identity["payload_codec_id"],
                candidate_identity["input_schema_id"],
                bytes.fromhex(candidate_identity["digest_hex"]),
            ),
            source_instance_id=cast(str, instance["source_instance_id"]),
            candidate_universe_binding=[
                candidate_binding.path,
                candidate_binding.schema_or_null,
                candidate_binding.raw_sha256,
            ],
            context_binding_v1=subject_shape,
            precondition_attestations_v1=member_preconditions,
            member_evidence_refs=(model_evidence,),
            context_member_bridge_attestation_v2=ContextMemberBridgeAttestationV2(
                context=context,
                temporal=temporal,
            ),
        )
        application_id = ContextApplicationV2InputV1(
            theorem_record_id_bytes=theorem_record_id.digest_bytes,
            members=(member,),
        ).identity()
        event_ref_v3 = ReviewEventRefV3(
            "sources/m2_5/authorities/review_acceptance_events/v3/" + "00" * 32 + ".json",
            bytes(32),
            "ae.v3/" + "00" * 32,
        )
        record = ContextApplicationV2Record.from_parts(
            application_id=application_id,
            theorem_record_id=theorem_record_id,
            members=(member,),
            review_event_ref_v3=event_ref_v3,
        )
        return {
            "source_resolver": source_resolver,
            "base_binding": base_binding,
            "record": record,
            "member": member,
            "fixture": fixture,
        }


if __name__ == "__main__":
    unittest.main()
